use std::collections::hash_map::Entry;
use std::mem::size_of;
use std::{collections::HashMap, ffi::c_void};

use erupt::utils::surface;
use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::{
    assets::{Asset, Assets, MeshId, TextureId},
    logging::{debug, trace},
    rendering::{
        context::Context,
        descriptor as ds, g,
        memory::{self, IndexBuffer, VertexBuffer},
        mesh::{LoadedSubmesh, Mesh},
        resource::DeviceResource,
        shader::{Shader, ShaderStage},
        spatial::Spatial,
        texture::{self, LoadedTexture, Texture},
        vertex::Vertex,
    },
    scenes::Scene,
};

const SHADER_VERT: &[u8] = include_bytes!("../../shaders/unlit.vert.spv");
const SHADER_FRAG: &[u8] = include_bytes!("../../shaders/unlit.frag.spv");

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    ctx: Option<Context>,

    frames_in_flight: SmallVec<[Frame; FRAMES_IN_FLIGHT]>,
    frame_number: usize,
    has_window_target: bool,

    textures: HashMap<TextureId, LoadedTexture>,
    texture_descriptor_sets: HashMap<TextureId, vk::DescriptorSet>,
    sampler: vk::Sampler,

    meshes: HashMap<MeshId, LoadedSubmesh>,

    surface_size: vk::Extent2D,

    descriptor_pool: vk::DescriptorPool,
    texture_descriptor_set_layout: vk::DescriptorSetLayout,

    render_pass: vk::RenderPass,
    pipeline: Pipeline,

    app_name: String,
}

#[derive(Clone)]
struct Frame {
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    cmd_buf: vk::CommandBuffer,
}

impl Renderer {
    pub fn new(app_name: &str) -> Self {
        Self {
            ctx: None,
            frames_in_flight: SmallVec::new(),
            frame_number: 0,
            descriptor_pool: vk::DescriptorPool::null(),
            texture_descriptor_set_layout: vk::DescriptorSetLayout::null(),
            texture_descriptor_sets: HashMap::new(),
            meshes: HashMap::new(),
            pipeline: Pipeline::default(),
            render_pass: vk::RenderPass::null(),
            sampler: vk::Sampler::null(),
            textures: HashMap::new(),
            surface_size: vk::Extent2D::default(),
            has_window_target: false,
            app_name: app_name.to_string(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.has_window_target
    }

    pub fn initialize_with_window(&mut self, window: &Window) {
        let mut ctx = Context::new(window, &self.app_name, "No Engine");
        unsafe {
            let cmd_bufs =
                memory::create_command_buffers(&ctx.device, ctx.cmd_pool, FRAMES_IN_FLIGHT);

            self.frames_in_flight = (0..FRAMES_IN_FLIGHT)
                .map(|n| Frame {
                    image_available_semaphore: ctx.sync_pool.semaphore(&ctx.device),
                    render_finished_semaphore: ctx.sync_pool.semaphore(&ctx.device),
                    in_flight_fence: ctx.sync_pool.fence(&ctx.device, true),
                    cmd_buf: cmd_bufs[n],
                })
                .collect();

            let render_pass = create_render_pass(&ctx);
            self.render_pass = render_pass;

            self.descriptor_pool = ds::create_descriptor_pool(&ctx.device, FRAMES_IN_FLIGHT);

            let texture_descriptor_set_layout = ds::descriptor_set_layout_1_texture(&ctx.device, 1);
            self.texture_descriptor_set_layout = texture_descriptor_set_layout;
            self.sampler = texture::create_sampler(&ctx);

            let vertex_shader =
                Shader::new(SHADER_VERT, ShaderStage::Vertex).into_initialized(&ctx.device);
            let fragment_shader =
                Shader::new(SHADER_FRAG, ShaderStage::Fragment).into_initialized(&ctx.device);
            let shader_stages = [vertex_shader.stage_info(), fragment_shader.stage_info()];

            let vertex_binding_descs = [Vertex::binding_desc()];
            let vertex_attribute_descs = Vertex::attribute_descs();
            self.pipeline = create_pipeline(
                &ctx.device,
                render_pass,
                &shader_stages,
                &vertex_binding_descs,
                &vertex_attribute_descs,
                &[texture_descriptor_set_layout],
            );

            vertex_shader.destroy(&ctx.device);
            fragment_shader.destroy(&ctx.device);

            self.ctx = Some(ctx);
            self.has_window_target = true;
        }
    }

    pub fn load_assets(&mut self, assets: &Assets) {
        self.use_textures(assets.textures());
        self.use_meshes(assets.meshes());
    }

    fn use_textures<'a>(&mut self, textures: impl Iterator<Item = &'a Texture>) {
        if let Some(ctx) = &self.ctx {
            unsafe {
                for t in textures {
                    if !self.textures.contains_key(&t.id()) {
                        let gpu_texture = t.init(ctx);
                        self.textures.insert(t.id(), gpu_texture);
                    }
                }

                let textures: SmallVec<[&LoadedTexture; 16]> = self.textures.values().collect();
                self.texture_descriptor_sets = ds::texture_descriptor_sets(
                    &ctx.device,
                    self.descriptor_pool,
                    self.texture_descriptor_set_layout,
                    1,
                    &textures,
                    self.sampler,
                );
            }
        }
    }

    fn use_meshes<'a>(&mut self, meshes: impl Iterator<Item = &'a Mesh>) {
        if let Some(ctx) = &self.ctx {
            let copy_queue = ctx.graphics_queue;
            let copy_queue_family = ctx.physical_device.graphics_queue_family;
            unsafe {
                for mesh in meshes {
                    for submesh in &mesh.submeshes {
                        let (vertex_buf, vertex_mem) = memory::create_vertex_buffer(
                            &ctx,
                            &mesh.vertices,
                            copy_queue_family,
                            copy_queue,
                        );
                        let indices = &mesh.indices[submesh.start_index..submesh.end_index];
                        let (index_buf, index_mem) = memory::create_index_buffer(
                            &ctx,
                            indices,
                            copy_queue_family,
                            copy_queue,
                        );

                        let vertex_buf = VertexBuffer {
                            handle: vertex_buf,
                            memory: vertex_mem,
                        };

                        let index_buf = IndexBuffer {
                            handle: index_buf,
                            memory: index_mem,
                            index_count: indices.len(),
                        };

                        let gpu_mesh = LoadedSubmesh {
                            id: submesh.id,
                            texture_id: submesh.texture_id,
                            index_buf,
                            vertex_buf,
                        };

                        match self.meshes.entry(submesh.id) {
                            Entry::Vacant(e) => {
                                e.insert(gpu_mesh);
                            }
                            _ => {}
                        };
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, scene: &mut impl Scene, assets: &mut Assets) {
        let image = match self.render_target() {
            Some(image_index) => image_index,
            None => return,
        };

        if let Some(ctx) = &self.ctx {
            scene.active_camera_mut().set_viewport_dimensions(
                self.surface_size.width as f32,
                self.surface_size.height as f32,
            );

            if let Some(swapchain) = &ctx.swapchain {
                let current_frame = self.current_frame();
                unsafe {
                    let framebuffers = swapchain.framebuffers(&ctx.device, self.render_pass);
                    g::begin_draw(
                        &ctx.device,
                        current_frame.cmd_buf,
                        self.render_pass,
                        framebuffers[image as usize],
                        swapchain.image_dimensions(),
                    );
                }

                let objects = scene.objects();
                let camera = scene.active_camera();
                for o in objects {
                    let mesh = assets
                        .mesh(o.mesh_id)
                        .expect("failed to fetch mesh that is supposed to be loaded");
                    unsafe {
                        for sm in &mesh.submeshes {
                            let mvp = Spatial(camera.matrix() * o.transform.matrix());

                            let device = &ctx.device;
                            let cmd_buf = self.current_frame().cmd_buf;

                            ctx.device.cmd_bind_pipeline(
                                cmd_buf,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.pipeline.handle,
                            );

                            let mesh = &self.meshes[&sm.id];
                            device.cmd_bind_vertex_buffers(
                                cmd_buf,
                                0,
                                &[mesh.vertex_buf.handle],
                                &[0],
                            );
                            device.cmd_bind_index_buffer(
                                cmd_buf,
                                mesh.index_buf.handle,
                                0,
                                vk::IndexType::UINT16,
                            );

                            device.cmd_push_constants(
                                cmd_buf,
                                self.pipeline.layout,
                                vk::ShaderStageFlags::VERTEX,
                                0,
                                size_of::<Spatial>() as _,
                                &mvp as *const Spatial as *const c_void,
                            );

                            let texture_descriptor_set =
                                self.texture_descriptor_sets[&sm.texture_id];
                            device.cmd_bind_descriptor_sets(
                                cmd_buf,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.pipeline.layout,
                                0,
                                &[texture_descriptor_set],
                                &[],
                            );
                            device.cmd_draw_indexed(
                                cmd_buf,
                                mesh.index_buf.index_count as _,
                                1,
                                0,
                                0,
                                0,
                            );
                        }
                    }
                }

                unsafe {
                    g::end_draw(
                        &ctx.device,
                        ctx.graphics_queue,
                        current_frame.cmd_buf,
                        current_frame.image_available_semaphore,
                        current_frame.render_finished_semaphore,
                        current_frame.in_flight_fence,
                    );

                    g::present(
                        &ctx.device,
                        &swapchain,
                        image,
                        current_frame.render_finished_semaphore,
                    );
                }
            }

            self.advance_frame()
        }
    }

    pub fn invalidate_surface(&mut self, window: &Window) {
        unsafe {
            if let Some(ctx) = &mut self.ctx {
                if let Some(swapchain) = &mut ctx.swapchain {
                    let PhysicalSize { width, height } = window.inner_size();
                    let surface = surface::create_surface(&ctx.instance, &window, None)
                        .expect("failed to create Vulkan surface");
                    swapchain.queue_recreate(
                        surface,
                        &vk::Extent2D {
                            width: width as u32,
                            height: height as u32,
                        },
                    );
                }
            }
        }
    }

    fn render_target(&mut self) -> Option<u32> {
        if self.ctx.is_some() {
            let current_frame = self.current_frame().clone();
            let ctx = self.ctx.as_mut().unwrap();
            if let Some(swapchain) = &mut ctx.swapchain {
                match swapchain.acquire_image(
                    &ctx.device,
                    &ctx.physical_device,
                    current_frame.in_flight_fence,
                    current_frame.image_available_semaphore,
                ) {
                    None => {
                        return None;
                    }
                    image => return image,
                }
            }
        }

        None
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
            if let Some(ctx) = &self.ctx {
                debug!("Dropping renderer");

                ctx.device.device_wait_idle().unwrap();

                for m in self.meshes.values() {
                    m.destroy(&ctx.device)
                }

                for t in self.textures.values() {
                    t.destroy(&ctx.device);
                }
                ctx.device.destroy_sampler(self.sampler, None);

                ctx.device
                    .destroy_descriptor_set_layout(self.texture_descriptor_set_layout, None);
                ctx.device
                    .destroy_descriptor_pool(self.descriptor_pool, None);

                self.pipeline.destroy(&ctx.device);

                ctx.device.destroy_render_pass(self.render_pass, None);
            }
        }
    }
}

struct Pipeline {
    handle: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            handle: vk::Pipeline::null(),
            layout: vk::PipelineLayout::null(),
        }
    }
}

impl DeviceResource for Pipeline {
    fn destroy(&self, device: &erupt::DeviceLoader) {
        unsafe {
            device.destroy_pipeline(self.handle, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
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
    device: &DeviceLoader,
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

    let viewports = vec![vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(0.0)
        .height(0.0)
        .max_depth(1.0)];
    let scissors = vec![vk::Rect2DBuilder::new()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(vk::Extent2D::default())];
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
    let pipeline_layout = device
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

    let pipeline = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        .unwrap()[0];

    Pipeline {
        handle: pipeline,
        layout: pipeline_layout,
    }
}
