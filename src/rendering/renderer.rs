use std::collections::hash_map::Entry;
use std::mem::size_of;
use std::{collections::HashMap, ffi::c_void};

use erupt::utils::surface;
use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::assets::{MaterialId, ShaderId};
use crate::{
    assets::{Asset, Assets, MeshId, TextureId},
    logging::{debug, error},
    rendering::{
        mesh::LoadedSubmesh,
        spatial::Spatial,
        texture::{self, LoadedTexture, Texture},
        vertex::Vertex,
        vulkan::{
            context::Context,
            descriptor as ds, g,
            memory::{self, IndexBuffer, VertexBuffer},
            resource::DeviceResource,
            swapchain::Swapchain,
        },
    },
    scenes::Scene,
};

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    ctx: Context,

    frames_in_flight: SmallVec<[Frame; FRAMES_IN_FLIGHT]>,
    frame_number: usize,

    textures: HashMap<TextureId, LoadedTexture>,
    texture_descriptor_sets: HashMap<TextureId, vk::DescriptorSet>,
    sampler: vk::Sampler,

    meshes: HashMap<MeshId, LoadedSubmesh>,

    surface_size: vk::Extent2D,
    new_surface_size: Option<vk::Extent2D>,
    new_surface: Option<vk::SurfaceKHR>,

    descriptor_pool: vk::DescriptorPool,
    texture_descriptor_set_layout: vk::DescriptorSetLayout,

    render_pass: vk::RenderPass,
    pipelines: HashMap<MaterialId, Pipeline>,
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
        let mut ctx = Context::new(window, &app_name, "No Engine");
        unsafe {
            let cmd_bufs =
                memory::create_command_buffers(&ctx.device, ctx.cmd_pool, FRAMES_IN_FLIGHT);

            let frames_in_flight = (0..FRAMES_IN_FLIGHT)
                .map(|n| Frame {
                    image_available_semaphore: ctx.sync_pool.semaphore(&ctx.device),
                    render_finished_semaphore: ctx.sync_pool.semaphore(&ctx.device),
                    in_flight_fence: ctx.sync_pool.fence(&ctx.device, true),
                    cmd_buf: cmd_bufs[n],
                })
                .collect();

            let render_pass = create_render_pass(&ctx);

            let descriptor_pool = ds::create_descriptor_pool(&ctx.device, FRAMES_IN_FLIGHT);

            let texture_descriptor_set_layout = ds::descriptor_set_layout_1_texture(&ctx.device, 1);
            let sampler = texture::create_sampler(&ctx);

            Self {
                ctx,
                frames_in_flight,
                frame_number: 0,
                descriptor_pool,
                texture_descriptor_set_layout,
                texture_descriptor_sets: HashMap::new(),
                meshes: HashMap::new(),
                pipelines: HashMap::new(),
                render_pass,
                sampler,
                textures: HashMap::new(),
                surface_size: vk::Extent2D::default(),
                new_surface: None,
                new_surface_size: None,
            }
        }
    }

    pub fn draw(&mut self, scene: &mut impl Scene, assets: &mut Assets) {
        let image_index = match self.next_swapchain_image() {
            Some(image_index) => image_index,
            None => return,
        };

        scene.active_camera_mut().set_viewport_dimensions(
            self.surface_size.width as f32,
            self.surface_size.height as f32,
        );

        if let Some(swapchain) = &self.ctx.swapchain {
            let current_frame = self.current_frame();
            unsafe {
                let framebuffers = swapchain.framebuffers(&self.ctx.device, self.render_pass);
                g::begin_draw(
                    &self.ctx.device,
                    current_frame.cmd_buf,
                    self.render_pass,
                    framebuffers[image_index as usize],
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

                        let device = &self.ctx.device;
                        let cmd_buf = self.current_frame().cmd_buf;

                        let pipeline = &self.pipelines[&sm.material_id];
                        self.ctx.device.cmd_bind_pipeline(
                            cmd_buf,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline.handle,
                        );

                        let mesh = &self.meshes[&sm.id];
                        device.cmd_bind_vertex_buffers(cmd_buf, 0, &[mesh.vertex_buf.handle], &[0]);
                        device.cmd_bind_index_buffer(
                            cmd_buf,
                            mesh.index_buf.handle,
                            0,
                            vk::IndexType::UINT16,
                        );

                        device.cmd_push_constants(
                            cmd_buf,
                            pipeline.layout,
                            vk::ShaderStageFlags::VERTEX,
                            0,
                            size_of::<Spatial>() as _,
                            &mvp as *const Spatial as *const c_void,
                        );

                        let material = assets.material(sm.material_id).unwrap();
                        let texture_descriptor_set =
                            self.texture_descriptor_sets[&material.texture_id];
                        device.cmd_bind_descriptor_sets(
                            cmd_buf,
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline.layout,
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
                    &self.ctx.device,
                    self.ctx.graphics_queue,
                    current_frame.cmd_buf,
                    current_frame.image_available_semaphore,
                    current_frame.render_finished_semaphore,
                    current_frame.in_flight_fence,
                );

                g::present(
                    &self.ctx.device,
                    &swapchain,
                    image_index,
                    current_frame.render_finished_semaphore,
                );
            }
        }

        self.finish_frame()
    }

    pub fn invalidate_surface_size(&mut self, new_size: PhysicalSize<u32>) {
        let PhysicalSize { width, height } = new_size;
        self.new_surface_size = Some(vk::Extent2D { width, height });
    }

    pub fn invalidate_surface(&mut self, window: &Window) {
        self.invalidate_surface_size(window.inner_size());
        unsafe {
            self.new_surface =
                Some(surface::create_surface(&self.ctx.instance, &window, None).unwrap())
        }
    }

    fn next_swapchain_image(&mut self) -> Option<u32> {
        let current_frame = self.current_frame().clone();
        if let Some(swapchain) = &mut self.ctx.swapchain {
            let maybe_image = swapchain.acquire_image(
                &self.ctx.device,
                current_frame.in_flight_fence,
                current_frame.image_available_semaphore,
            );

            if maybe_image.is_none() || self.new_surface_size.is_some() {
                debug!(
                    "Recreating swapchain with surface size {:?}",
                    self.surface_size
                );

                if let Some(new_size) = self.new_surface_size {
                    self.surface_size = new_size;
                }

                unsafe {
                    self.ctx
                        .device
                        .queue_wait_idle(self.ctx.graphics_queue)
                        .unwrap();
                    swapchain.recreate(
                        &self.ctx.instance,
                        &self.ctx.device,
                        &self.ctx.physical_device,
                        self.new_surface,
                        &self.surface_size,
                    );
                }

                self.new_surface = None;
                self.new_surface_size = None;

                swapchain.acquire_image(
                    &self.ctx.device,
                    current_frame.in_flight_fence,
                    current_frame.image_available_semaphore,
                )
            } else {
                maybe_image
            }
        } else {
            None
        }
    }

    fn finish_frame(&mut self) {
        self.frame_number = (self.frame_number + 1) % FRAMES_IN_FLIGHT;
    }

    fn current_frame(&self) -> &Frame {
        &self.frames_in_flight[self.frame_number]
    }

    pub fn load_assets(&mut self, assets: &Assets) {
        self.use_textures(assets.textures());
        self.use_meshes(assets);
    }

    fn use_textures<'a>(&mut self, textures: impl Iterator<Item = &'a Texture>) {
        unsafe {
            for t in textures {
                if !self.textures.contains_key(&t.id()) {
                    let gpu_texture = t.init(&self.ctx);
                    self.textures.insert(t.id(), gpu_texture);
                }
            }

            let textures: SmallVec<[&LoadedTexture; 16]> = self.textures.values().collect();
            self.texture_descriptor_sets = ds::texture_descriptor_sets(
                &self.ctx.device,
                self.descriptor_pool,
                self.texture_descriptor_set_layout,
                1,
                &textures,
                self.sampler,
            );
        }
    }

    fn use_meshes<'a>(&mut self, assets: &Assets) {
        let copy_queue = self.ctx.graphics_queue;
        let copy_queue_family = self.ctx.physical_device.graphics_queue_family;

        let vertex_binding_descs = [Vertex::binding_desc()];
        let vertex_attribute_descs = Vertex::attribute_descs();

        unsafe {
            for mesh in assets.meshes() {
                for submesh in &mesh.submeshes {
                    let (vertex_buf, vertex_mem) = memory::create_vertex_buffer(
                        &self.ctx,
                        &mesh.vertices,
                        copy_queue_family,
                        copy_queue,
                    );
                    let indices = &mesh.indices[submesh.start_index..submesh.end_index];
                    let (index_buf, index_mem) = memory::create_index_buffer(
                        &self.ctx,
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
                        material_id: submesh.material_id,
                        index_buf,
                        vertex_buf,
                    };

                    match self.meshes.entry(submesh.id) {
                        Entry::Vacant(e) => {
                            e.insert(gpu_mesh);
                        }
                        _ => {}
                    };

                    match self.pipelines.entry(submesh.material_id) {
                        Entry::Vacant(e) => {
                            let material = assets.material(submesh.material_id).unwrap();
                            let vertex_shader = assets
                                .shader(material.vertex_shader_id)
                                .unwrap()
                                .initialize(&self.ctx.device)
                                .map_err(|e| {
                                    error!("{e}");
                                })
                                .expect("fix shader compilation errors");
                            let fragment_shader = assets
                                .shader(material.fragment_shader_id)
                                .unwrap()
                                .initialize(&self.ctx.device)
                                .map_err(|e| {
                                    error!("{e}");
                                })
                                .expect("fix shader compilation errors");
                            let shader_stages =
                                [vertex_shader.stage_info(), fragment_shader.stage_info()];

                            let pipeline = create_pipeline(
                                &self.ctx.device,
                                self.render_pass,
                                &shader_stages,
                                &vertex_binding_descs,
                                &vertex_attribute_descs,
                                vk::PrimitiveTopology::TRIANGLE_LIST,
                                &[self.texture_descriptor_set_layout],
                            );

                            e.insert(pipeline);

                            vertex_shader.destroy(&self.ctx.device);
                            fragment_shader.destroy(&self.ctx.device);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn resume(&mut self) {
        debug!("Recreating swapchain after start");
        self.surface_size = self
            .new_surface_size
            .expect("new_surface_size must be set when creating a new swapchain");
        let surface = self
            .new_surface
            .expect("new_surface must be set when creating a new swapchain");

        self.ctx.swapchain = Some(Swapchain::new(
            &self.ctx.device,
            &self.ctx.physical_device,
            self.ctx.graphics_queue,
            surface,
            &self.surface_size,
        ));
    }

    pub fn pause(&mut self) {
        debug!("Destroying swapchain after pause");
        let mut swapchain = self.ctx.swapchain.take();
        if let Some(swapchain) = &mut swapchain {
            unsafe {
                swapchain.destroy(&self.ctx.device, &self.ctx.instance);
            }
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            debug!("Dropping renderer");

            self.ctx.device.device_wait_idle().unwrap();

            for m in self.meshes.values() {
                m.destroy(&self.ctx.device)
            }

            for t in self.textures.values() {
                t.destroy(&self.ctx.device);
            }
            self.ctx.device.destroy_sampler(self.sampler, None);

            self.ctx
                .device
                .destroy_descriptor_set_layout(self.texture_descriptor_set_layout, None);
            self.ctx
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            for (_, p) in &self.pipelines {
                p.destroy(&self.ctx.device);
            }

            self.ctx.device.destroy_render_pass(self.render_pass, None);
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
    primitive_topology: vk::PrimitiveTopology,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> Pipeline {
    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
        .vertex_binding_descriptions(&vertex_binding_descs)
        .vertex_attribute_descriptions(&vertex_attribute_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
        .topology(primitive_topology)
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
