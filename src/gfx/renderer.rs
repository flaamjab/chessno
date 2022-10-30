use std::collections::hash_map::Entry;
use std::mem::size_of;
use std::{collections::HashMap, ffi::c_void};

use erupt::vk;
use smallvec::SmallVec;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::assets::{Asset, AssetId};
use crate::gfx::{
    context::Context,
    descriptor as ds, g,
    geometry::Vertex,
    memory,
    shader::{Shader, ShaderStage},
    spatial::Spatial,
    sync_pool::SyncPool,
    texture::{self, GpuResidentTexture, Texture},
    vulkan_resource::DeviceResource,
};
use crate::scene::Scenelike;
use crate::transform::Transform;

const SHADER_VERT: &[u8] = include_bytes!("../../shaders/unlit.vert.spv");
const SHADER_FRAG: &[u8] = include_bytes!("../../shaders/unlit.frag.spv");

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    ctx: Context,
    sync_pool: SyncPool,
    frames_in_flight: SmallVec<[Frame; FRAMES_IN_FLIGHT]>,
    frame_number: usize,
    resources: Resources,
    resize_required: bool,
    cmd_pool: vk::CommandPool,

    textures: HashMap<AssetId, GpuResidentTexture>,
    texture_descriptor_sets: HashMap<AssetId, vk::DescriptorSet>,

    new_size: vk::Extent2D,
}

#[derive(Clone)]
struct Frame {
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    cmd_buf: vk::CommandBuffer,
}

impl Renderer {
    pub fn new(app_name: &str, window: &Window) -> Self {
        let ctx = Context::new(window, app_name, "No Engine");
        let mut sync_pool = SyncPool::new();
        unsafe {
            let cmd_pool = memory::create_command_pool(
                &ctx.device,
                ctx.physical_device.queue_families.graphics,
            );
            let cmd_bufs = memory::create_command_buffers(&ctx.device, cmd_pool, FRAMES_IN_FLIGHT);

            let frames_in_flight = (0..FRAMES_IN_FLIGHT)
                .map(|n| Frame {
                    image_available_semaphore: sync_pool.semaphore(&ctx.device),
                    render_finished_semaphore: sync_pool.semaphore(&ctx.device),
                    in_flight_fence: sync_pool.fence(&ctx.device, true),
                    cmd_buf: cmd_bufs[n],
                })
                .collect();

            let resources = Resources::new(&ctx);
            Self {
                ctx,
                sync_pool,
                cmd_pool,
                frames_in_flight,
                frame_number: 0,
                texture_descriptor_sets: HashMap::new(),
                resources,
                textures: HashMap::new(),
                resize_required: false,
                new_size: vk::Extent2D::default(),
            }
        }
    }

    pub fn use_textures(&mut self, textures: &[&Texture]) {
        unsafe {
            for t in textures {
                if !self.textures.contains_key(&t.id()) {
                    self.textures.insert(t.id(), t.load(&self.ctx));
                }
            }

            let textures: SmallVec<[&GpuResidentTexture; 16]> = self.textures.values().collect();
            self.texture_descriptor_sets = ds::texture_descriptor_sets(
                &self.ctx.device,
                self.resources.descriptor_pool,
                self.resources.material_dsl,
                1,
                &textures,
                self.resources.sampler,
            );
        }
    }

    pub fn draw(&mut self, scene: &impl Scenelike) {
        let copy_queue = self.ctx.queues.graphics;
        let copy_queue_family = self.ctx.physical_device.queue_families.graphics;

        let current_frame = self.current_frame().clone();
        let image_index = match self.ctx.swapchain.acquire_image(
            &self.ctx.device,
            &self.ctx.physical_device,
            current_frame.in_flight_fence,
            current_frame.image_available_semaphore,
            &mut self.resize_required,
            &self.new_size,
        ) {
            Some(image_index) => image_index,
            None => return,
        };

        unsafe {
            let framebuffers = self
                .ctx
                .swapchain
                .framebuffers(&self.ctx.device, self.resources.render_pass);
            g::begin_draw(
                &self.ctx.device,
                current_frame.cmd_buf,
                self.resources.render_pass,
                framebuffers[image_index as usize],
                self.ctx.swapchain.image_extent(),
            );
        }

        let objects = scene.objects();
        let camera = scene.active_camera();
        let assets = scene.assets();
        let mut free_queue = Vec::with_capacity(objects.len());
        for o in objects {
            let mesh = assets
                .get_mesh_by_id(o.mesh_id)
                .expect("failed to fetch mesh that is supposed to be loaded");
            unsafe {
                let (vertex_buf, vertex_mem) = memory::create_vertex_buffer(
                    &self.ctx,
                    &mesh.vertices,
                    copy_queue_family,
                    copy_queue,
                );
                free_queue.push((vertex_buf, vertex_mem));

                for sm in &mesh.submeshes {
                    let indices = &mesh.indices[sm.start_index..sm.end_index];
                    let (index_buf, index_mem) = memory::create_index_buffer(
                        &self.ctx,
                        &indices,
                        copy_queue_family,
                        copy_queue,
                    );

                    free_queue.push((index_buf, index_mem));

                    let mvp = Spatial(camera.matrix() * o.transform.matrix());

                    let device = &self.ctx.device;
                    let cmd_buf = self.current_frame().cmd_buf;

                    // memory::upload_uniform_buffers(&device, &mvp, uniform_mem);

                    self.ctx.device.cmd_bind_pipeline(
                        cmd_buf,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipeline.handle,
                    );

                    device.cmd_bind_vertex_buffers(cmd_buf, 0, &[vertex_buf], &[0]);
                    device.cmd_bind_index_buffer(cmd_buf, index_buf, 0, vk::IndexType::UINT16);

                    device.cmd_push_constants(
                        cmd_buf,
                        self.resources.pipeline.layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        size_of::<Spatial>() as _,
                        &mvp as *const Spatial as *const c_void,
                    );

                    let texture_descriptor_set = self.texture_descriptor_sets[&sm.texture_id];
                    device.cmd_bind_descriptor_sets(
                        cmd_buf,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipeline.layout,
                        0,
                        &[texture_descriptor_set],
                        &[],
                    );
                    device.cmd_draw_indexed(cmd_buf, indices.len() as _, 1, 0, 0, 0);
                }
            }
        }

        unsafe {
            // End draw
            g::end_draw(
                &self.ctx.device,
                self.ctx.queues.graphics,
                current_frame.cmd_buf,
                current_frame.image_available_semaphore,
                current_frame.render_finished_semaphore,
                current_frame.in_flight_fence,
            );

            // Present
            g::present(
                &self.ctx,
                image_index,
                current_frame.render_finished_semaphore,
            );

            // Free buffers and memory
            self.ctx
                .device
                .queue_wait_idle(self.ctx.queues.graphics)
                .expect("failed to wait for queue");
            for (buf, mem) in free_queue.drain(..) {
                self.ctx.device.destroy_buffer(buf, None);
                self.ctx.device.free_memory(mem, None);
            }
        }

        self.advance_frame()
    }

    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        self.resize_required = true;

        let PhysicalSize { width, height } = new_size;
        self.new_size = vk::Extent2D { width, height };
    }

    fn advance_frame(&mut self) {
        self.frame_number = (self.frame_number + 1) % FRAMES_IN_FLIGHT;
    }

    fn current_frame(&self) -> &Frame {
        &self.frames_in_flight[self.frame_number]
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            for t in self.textures.values() {
                t.destroy(&self.ctx.device);
            }
            self.resources.destroy(&mut self.ctx);
            self.ctx.device.destroy_command_pool(self.cmd_pool, None);
            self.sync_pool.destroy_all(&self.ctx.device);
        }
    }
}

// unsafe fn build_pipelines(ctx: &Context) -> SmallVec<[vk::Pipeline; 8]> {
//     let (pipeline, pipeline_layout) =
//         g::create_pipeline(&ctx, &shader_stages, render_pass, &descriptor_set_layouts);
//     let pipeline = Pipeline {
//         handle: pipeline,
//         layout: pipeline_layout,
//     };

//     smallvec![pipeline]
// }

struct Resources {
    render_pass: vk::RenderPass,
    descriptor_pool: vk::DescriptorPool,
    sampler: vk::Sampler,
    uniforms: SmallVec<[(vk::Buffer, vk::DeviceMemory); 2]>,
    material_dsl: vk::DescriptorSetLayout,
    pipeline: Pipeline,
}

struct Pipeline {
    handle: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Resources {
    pub fn new(ctx: &Context) -> Resources {
        unsafe {
            let vertex_shader =
                Shader::new(SHADER_VERT, ShaderStage::Vertex).into_initialized(&ctx.device);
            let fragment_shader =
                Shader::new(SHADER_FRAG, ShaderStage::Fragment).into_initialized(&ctx.device);
            let shader_stages = [vertex_shader.stage_info(), fragment_shader.stage_info()];

            let render_pass = create_render_pass(&ctx);

            let uniforms =
                memory::create_uniform_buffers(&ctx, size_of::<Transform>(), FRAMES_IN_FLIGHT);
            let descriptor_pool = ds::create_descriptor_pool(&ctx.device, FRAMES_IN_FLIGHT);

            let texture_descriptor_set_layout = ds::descriptor_set_layout_1_texture(&ctx.device, 1);
            let sampler = texture::create_sampler(&ctx);

            let vertex_binding_descs = [Vertex::binding_desc()];
            let vertex_attribute_descs = Vertex::attribute_descs();
            let pipeline = create_pipeline(
                ctx,
                render_pass,
                &shader_stages,
                &vertex_binding_descs,
                &vertex_attribute_descs,
                &[texture_descriptor_set_layout],
            );

            drop(shader_stages);
            vertex_shader.destroy(&ctx.device);
            fragment_shader.destroy(&ctx.device);

            Self {
                descriptor_pool,
                material_dsl: texture_descriptor_set_layout,
                render_pass,
                pipeline,
                uniforms,
                sampler,
            }
        }
    }

    pub unsafe fn destroy(&mut self, ctx: &mut Context) {
        memory::release_resources(
            ctx,
            &self.uniforms,
            self.descriptor_pool,
            self.pipeline.handle,
            self.pipeline.layout,
            &[self.material_dsl],
            self.render_pass,
            self.sampler,
        )
    }
}

unsafe fn create_render_pass(ctx: &Context) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescriptionBuilder::new()
        .format(ctx.physical_device.surface_format.format)
        .samples(vk::SampleCountFlagBits::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let depth_attachment = vk::AttachmentDescriptionBuilder::new()
        .format(ctx.physical_device.depth_format)
        .samples(vk::SampleCountFlagBits::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let attachments = [color_attachment, depth_attachment];

    let color_attachment_refs = [vk::AttachmentReferenceBuilder::new()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let depth_attachment_ref = vk::AttachmentReferenceBuilder::new()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    let subpasses = [vk::SubpassDescriptionBuilder::new()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref)];

    let dependencies = vec![vk::SubpassDependencyBuilder::new()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )];

    let render_pass_info = vk::RenderPassCreateInfoBuilder::new()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    ctx.device
        .create_render_pass(&render_pass_info, None)
        .unwrap()
}

unsafe fn create_pipeline(
    ctx: &Context,
    render_pass: vk::RenderPass,
    shader_stages: &[vk::PipelineShaderStageCreateInfoBuilder],
    vertex_binding_descs: &[vk::VertexInputBindingDescriptionBuilder],
    vertex_attribute_descs: &[vk::VertexInputAttributeDescriptionBuilder],
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> Pipeline {
    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
        .vertex_binding_descriptions(&vertex_binding_descs)
        .vertex_attribute_descriptions(&vertex_attribute_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let image_extent = ctx.swapchain.image_extent();
    let viewports = vec![vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(image_extent.width as f32)
        .height(image_extent.height as f32)
        .max_depth(1.0)];
    let scissors = vec![vk::Rect2DBuilder::new()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(*image_extent)];
    let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE);

    let multisampling = vk::PipelineMultisampleStateCreateInfoBuilder::new()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlagBits::_1);

    let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentStateBuilder::new()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)];
    let color_blending = vk::PipelineColorBlendStateCreateInfoBuilder::new()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let push_constant_range = vk::PushConstantRangeBuilder::new()
        .offset(0)
        .size(size_of::<Spatial>() as _)
        .stage_flags(vk::ShaderStageFlags::VERTEX);
    let push_constant_ranges = [push_constant_range];

    let pipeline_layout_info = vk::PipelineLayoutCreateInfoBuilder::new()
        .set_layouts(&descriptor_set_layouts)
        .push_constant_ranges(&push_constant_ranges);
    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&pipeline_layout_info, None)
        .unwrap();

    let dynamic_state_info = vk::PipelineDynamicStateCreateInfoBuilder::new()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfoBuilder::new()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)
        .stencil_test_enable(false);

    let pipeline_info = vk::GraphicsPipelineCreateInfoBuilder::new()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .depth_stencil_state(&depth_stencil_info)
        .dynamic_state(&dynamic_state_info);

    let pipeline = ctx
        .device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        .unwrap()[0];

    Pipeline {
        handle: pipeline,
        layout: pipeline_layout,
    }
}
