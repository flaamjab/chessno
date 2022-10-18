use std::mem::size_of;
use std::path::Path;

use erupt::vk;
use smallvec::{SmallVec, ToSmallVec};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::camera::Camera;
use crate::gfx::context::Context;
use crate::gfx::g;
use crate::gfx::geometry::Geometry;
use crate::gfx::memory;
use crate::gfx::shader::Shader;
use crate::gfx::spatial::Spatial;
use crate::gfx::sync_pool::SyncPool;
use crate::gfx::texture;
use crate::logging::trace;
use crate::object::Object;
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
                resources,
                resize_required: false,
                new_size: vk::Extent2D::default(),
            }
        }
    }

    pub fn draw(&mut self, objects: &[Object], camera: &Camera) {
        let copy_queue = self.ctx.queues.graphics;
        let copy_queue_family = self.ctx.physical_device.queue_families.graphics;

        let current_frame = self.current_frame().clone();
        let image_index = unsafe {
            g::acquire_image(
                &mut self.ctx,
                current_frame.in_flight_fence,
                current_frame.image_available_semaphore,
                &mut self.resize_required,
                &self.new_size,
            )
        };

        if image_index.is_none() {
            return;
        }

        let image_index = image_index.unwrap();

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

        let mut free_queue = Vec::with_capacity(objects.len());
        for o in objects {
            unsafe {
                let mut geometry = Geometry::new();
                geometry.push_mesh(&o.mesh);

                let (vertex_buf, vertex_mem) = memory::create_vertex_buffer(
                    &self.ctx,
                    geometry.vertices(),
                    copy_queue_family,
                    copy_queue,
                );

                let (index_buf, index_mem) = memory::create_index_buffer(
                    &self.ctx,
                    geometry.indices(),
                    copy_queue_family,
                    copy_queue,
                );

                free_queue.push((vertex_buf, vertex_mem));
                free_queue.push((index_buf, index_mem));

                let mvp = Spatial(camera.matrix() * o.transform.matrix());
                g::draw(
                    &self.ctx.device,
                    self.resources.pipeline.handle,
                    self.resources.pipeline.layout,
                    current_frame.cmd_buf,
                    vertex_buf,
                    index_buf,
                    geometry.indices().len(),
                    &mvp,
                    self.resources.descriptor_sets[self.frame_number],
                    self.resources.uniforms[self.frame_number].1,
                );
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
            self.resources.destroy(&mut self.ctx);
            self.ctx.device.destroy_command_pool(self.cmd_pool, None);
            self.sync_pool.destroy_all(&self.ctx.device);
        }
    }
}

struct Resources {
    render_pass: vk::RenderPass,
    descriptor_pool: vk::DescriptorPool,
    texture: Texture,
    sampler: vk::Sampler,
    uniforms: SmallVec<[(vk::Buffer, vk::DeviceMemory); 2]>,
    descriptor_set_layouts: SmallVec<[vk::DescriptorSetLayout; 2]>,
    descriptor_sets: SmallVec<[vk::DescriptorSet; 2]>,
    pipeline: Pipeline,
}

struct Texture {
    memory: vk::DeviceMemory,
    image: vk::Image,
    image_view: vk::ImageView,
}

struct Pipeline {
    handle: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Resources {
    pub fn new(ctx: &Context) -> Resources {
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

            let uniforms =
                memory::create_uniform_buffers(&ctx, size_of::<Transform>(), FRAMES_IN_FLIGHT);
            let descriptor_pool = memory::create_descriptor_pool(&ctx.device, FRAMES_IN_FLIGHT);

            let path = Path::new("./assets/textures/happy-tree.png");
            let (texture, texture_mem) = texture::create_texture(
                &ctx,
                &path,
                ctx.queues.graphics,
                ctx.physical_device.queue_families.graphics,
            )
            .expect("failed to create texture");
            let texture_view = texture::create_texture_view(&ctx.device, texture);
            let texture = Texture {
                memory: texture_mem,
                image: texture,
                image_view: texture_view,
            };
            let sampler = texture::create_sampler(&ctx);

            let descriptor_set_layout = memory::create_descriptor_set_layout(&ctx);
            let descriptor_set_layouts = [descriptor_set_layout; 2];

            let descriptor_sets = memory::create_descriptor_sets(
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
            shader.destroy(&ctx.device);

            trace!("Creating command buffers");
            Self {
                descriptor_pool,
                descriptor_set_layouts: descriptor_set_layouts.to_smallvec(),
                descriptor_sets: descriptor_sets.to_smallvec(),
                render_pass,
                pipeline,
                uniforms,
                texture,
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
            self.descriptor_set_layouts[0],
            self.render_pass,
            self.texture.image,
            self.texture.memory,
            self.texture.image_view,
            self.sampler,
        )
    }
}
