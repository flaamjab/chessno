use std::path::Path;

use erupt::vk;
use smallvec::{SmallVec, ToSmallVec};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::camera::Camera;
use crate::context::Context;
use crate::g;
use crate::geometry::Geometry;
use crate::gpu_program::Shader;
use crate::logging::trace;
use crate::object::Object;
use crate::sync_pool::SyncPool;
use crate::transform::Transform;

const SHADER_VERT: &[u8] = include_bytes!("../shaders/unlit.vert.spv");
const SHADER_FRAG: &[u8] = include_bytes!("../shaders/unlit.frag.spv");

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    ctx: Context,
    sync_pool: SyncPool,
    frame: Frame,
    resources: Resources,
    resize_required: bool,
    new_size: vk::Extent2D,
}

struct Frame {
    number: usize,
}

impl Renderer {
    pub fn new(app_name: &str, window: &Window) -> Self {
        let ctx = Context::new(window, app_name, "No Engine");
        let frame = Frame { number: 0 };
        let mut sync_pool = SyncPool::new();
        let resources = Resources::new(&ctx, &mut sync_pool);

        Self {
            ctx,
            sync_pool,
            frame,
            resources,
            resize_required: false,
            new_size: vk::Extent2D::default(),
        }
    }

    pub fn draw(&mut self, objects: &[Object], camera: &Camera) {
        let copy_queue = self.ctx.queues.graphics;
        let copy_queue_family = self.ctx.physical_device.queue_families.graphics;

        let image_index = unsafe {
            g::acquire_image(
                &mut self.ctx,
                self.resources.in_flight_fences[self.frame.number],
                self.resources.image_available_semaphores[self.frame.number],
                &mut self.resize_required,
                &self.new_size,
            )
        };

        if image_index.is_none() {
            return;
        }

        let image_index = image_index.unwrap();
        let cmd_buf = self.resources.cmd_bufs[self.frame.number];

        unsafe {
            let framebuffers = self
                .ctx
                .swapchain
                .framebuffers(&self.ctx.device, self.resources.render_pass);
            g::begin_draw(
                &self.ctx.device,
                cmd_buf,
                self.resources.render_pass,
                framebuffers[image_index as usize],
                self.ctx.swapchain.image_extent(),
            );
        }

        let mut free_queue = Vec::new();
        for o in objects {
            unsafe {
                let mut geometry = Geometry::new();
                geometry.push_mesh(&o.mesh);

                let (vertex_buf, vertex_mem) = g::create_vertex_buffer(
                    &self.ctx,
                    geometry.vertices(),
                    copy_queue_family,
                    copy_queue,
                );

                let (index_buf, index_mem) = g::create_index_buffer(
                    &self.ctx,
                    geometry.indices(),
                    copy_queue_family,
                    copy_queue,
                );

                free_queue.push((vertex_buf, vertex_mem));
                free_queue.push((index_buf, index_mem));

                let transform = Transform::new(o.position, o.rotation, camera);
                g::draw_mesh(
                    &self.ctx.device,
                    self.resources.pipeline.handle,
                    self.resources.pipeline.layout,
                    cmd_buf,
                    vertex_buf,
                    index_buf,
                    geometry.indices().len(),
                    &transform,
                    self.resources.descriptor_sets[self.frame.number],
                    self.resources.uniforms[self.frame.number].1,
                );
            }
        }

        unsafe {
            // End draw
            let image_available_semaphore =
                self.resources.image_available_semaphores[self.frame.number];
            let render_finished_semaphore =
                self.resources.render_finished_semaphores[self.frame.number];
            let in_flight_fence = self.resources.in_flight_fences[self.frame.number];
            g::end_draw(
                &self.ctx.device,
                self.ctx.queues.graphics,
                cmd_buf,
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
            );

            // Present
            g::present(&self.ctx, image_index, render_finished_semaphore);

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

        self.frame.number = (self.frame.number + 1) % FRAMES_IN_FLIGHT;
    }

    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        self.resize_required = true;

        let PhysicalSize { width, height } = new_size;
        self.new_size = vk::Extent2D { width, height };
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.resources.destroy(&mut self.ctx);
        }
    }
}

struct Resources {
    shader: Shader,
    render_pass: vk::RenderPass,
    descriptor_pool: vk::DescriptorPool,
    texture: Texture,
    sampler: vk::Sampler,
    uniforms: SmallVec<[(vk::Buffer, vk::DeviceMemory); 2]>,
    descriptor_set_layouts: SmallVec<[vk::DescriptorSetLayout; 2]>,
    descriptor_sets: SmallVec<[vk::DescriptorSet; 2]>,
    cmd_pool: vk::CommandPool,
    cmd_bufs: SmallVec<[vk::CommandBuffer; FRAMES_IN_FLIGHT]>,
    pipeline: Pipeline,
    image_available_semaphores: SmallVec<[vk::Semaphore; FRAMES_IN_FLIGHT]>,
    render_finished_semaphores: SmallVec<[vk::Semaphore; FRAMES_IN_FLIGHT]>,
    in_flight_fences: SmallVec<[vk::Fence; FRAMES_IN_FLIGHT]>,
}

struct Texture {
    memory: vk::DeviceMemory,
    image: vk::Image,
    image_view: vk::ImageView,
}

struct UniformBuffer {
    memory: vk::DeviceMemory,
    buffer: vk::Buffer,
}

struct Pipeline {
    handle: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Resources {
    pub fn new(ctx: &Context, sync_pool: &mut SyncPool) -> Resources {
        let shader = Shader::new(
            &ctx.device,
            &[
                (SHADER_VERT, vk::ShaderStageFlagBits::VERTEX),
                (SHADER_FRAG, vk::ShaderStageFlagBits::FRAGMENT),
            ],
        )
        .expect("failed to create shader");
        let shader_stages = shader.stage_infos();

        unsafe {
            let render_pass = g::create_render_pass(&ctx);

            let uniforms = g::create_uniform_buffers(&ctx, FRAMES_IN_FLIGHT);
            let descriptor_pool = g::create_descriptor_pool(&ctx.device, FRAMES_IN_FLIGHT);

            let path = Path::new("./assets/textures/happy-tree.png");
            let (texture, texture_mem) = g::create_texture(
                &ctx,
                &path,
                ctx.queues.graphics,
                ctx.physical_device.queue_families.graphics,
            )
            .expect("failed to create texture");
            let texture_view = g::create_texture_view(&ctx.device, texture);
            let texture = Texture {
                memory: texture_mem,
                image: texture,
                image_view: texture_view,
            };
            let sampler = g::create_sampler(&ctx);

            let descriptor_set_layout = g::create_descriptor_set_layout(&ctx);
            let descriptor_set_layouts = [descriptor_set_layout; 2];

            let descriptor_sets = g::create_descriptor_sets(
                &ctx.device,
                descriptor_pool,
                &descriptor_set_layouts,
                &uniforms,
                (texture_view, sampler),
                FRAMES_IN_FLIGHT,
            );

            let (pipeline, pipeline_layout) =
                g::create_pipeline(&ctx, &shader_stages, render_pass, &descriptor_set_layouts);
            let pipeline = Pipeline {
                handle: pipeline,
                layout: pipeline_layout,
            };

            drop(shader_stages);

            let cmd_pool_info = vk::CommandPoolCreateInfoBuilder::new()
                .queue_family_index(ctx.physical_device.queue_families.graphics)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            let cmd_pool = ctx
                .device
                .create_command_pool(&cmd_pool_info, None)
                .unwrap();

            trace!("Creating command buffers");
            let framebuffers = ctx.swapchain.framebuffers(&ctx.device, render_pass);
            let cmd_buf_allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
                .command_pool(cmd_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(framebuffers.len() as _);
            let cmd_bufs = ctx
                .device
                .allocate_command_buffers(&cmd_buf_allocate_info)
                .unwrap();

            let image_available_semaphores: SmallVec<_> = (0..FRAMES_IN_FLIGHT)
                .map(|_| sync_pool.semaphore(&ctx.device))
                .collect();
            let render_finished_semaphores: SmallVec<_> = (0..FRAMES_IN_FLIGHT)
                .map(|_| sync_pool.semaphore(&ctx.device))
                .collect();

            let in_flight_fences: SmallVec<_> = (0..FRAMES_IN_FLIGHT)
                .map(|_| sync_pool.fence(&ctx.device, true))
                .collect();

            Self {
                descriptor_pool,
                descriptor_set_layouts: descriptor_set_layouts.to_smallvec(),
                descriptor_sets: descriptor_sets.to_smallvec(),
                cmd_pool,
                cmd_bufs: cmd_bufs.to_smallvec(),
                render_pass,
                pipeline,
                uniforms,
                shader,
                texture,
                sampler,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            }
        }
    }

    pub unsafe fn destroy(&mut self, ctx: &mut Context) {
        for semaphore in self
            .image_available_semaphores
            .iter()
            .chain(self.render_finished_semaphores.iter())
        {
            ctx.device.destroy_semaphore(*semaphore, None);
        }

        for fence in self.in_flight_fences.iter() {
            ctx.device.destroy_fence(*fence, None);
        }

        g::release_resources(
            ctx,
            &self.uniforms,
            &self.shader,
            self.descriptor_pool,
            self.cmd_pool,
            self.pipeline.handle,
            self.pipeline.layout,
            self.descriptor_set_layouts[0],
            self.render_pass,
            self.texture.image,
            self.texture.memory,
            self.texture.image_view,
            self.sampler,
        )
    }
}
