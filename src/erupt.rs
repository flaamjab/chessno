use std::mem::{size_of, size_of_val};
use std::time::Instant;
use std::{ffi::c_void, sync::Arc};

use cgmath::{Deg, Matrix4};
use erupt::{
    vk,
    vk1_0::{CommandBufferResetFlags, Extent2D},
    DeviceLoader,
};
use memoffset::offset_of;
use winit::dpi::PhysicalSize;
use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::context::Context;
use crate::geometry::{Geometry, Vertex};
use crate::logging::info;
use crate::shader::Shader;
use crate::sync_pool::SyncPool;
use crate::transform::Transform;
use crate::validation;

const TITLE: &str = "Isochess";
const FRAMES_IN_FLIGHT: usize = 2;
const SHADER_VERT: &[u8] = include_bytes!("../shaders/unlit.vert.spv");
const SHADER_FRAG: &[u8] = include_bytes!("../shaders/unlit.frag.spv");

impl Transform {
    fn layout<'a>() -> vk::DescriptorSetLayoutBindingBuilder<'a> {
        vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
    }
}

impl Vertex {
    fn binding_desc<'a>() -> vk::VertexInputBindingDescriptionBuilder<'a> {
        vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .input_rate(vk::VertexInputRate::VERTEX)
            .stride(size_of::<Vertex>() as u32)
    }

    fn attribute_descs<'a>() -> Vec<vk::VertexInputAttributeDescriptionBuilder<'a>> {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            }
            .into_builder(),
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            }
            .into_builder(),
        ]
        .into()
    }
}

pub unsafe fn init() {
    let geometry = Geometry::new_cube();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(TITLE)
        .build(&event_loop)
        .unwrap();

    let ctx = Context::new(&window, "Main", "No Engine");

    let (swapchain, swapchain_image_extent) = create_swapchain(
        &ctx,
        ctx.surface,
        window.inner_size(),
        ctx.physical_device.surface_format,
        ctx.physical_device.present_mode,
    );

    // https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Shader_modules
    let shader = Shader::new(
        &ctx.device,
        &[
            (SHADER_VERT, vk::ShaderStageFlagBits::VERTEX),
            (SHADER_FRAG, vk::ShaderStageFlagBits::FRAGMENT),
        ],
    )
    .expect("failed to create shader");
    let shader_stages = shader.stage_infos();

    let render_pass = create_render_pass(&ctx);

    let image_views =
        create_image_views(&ctx.device, swapchain, ctx.physical_device.surface_format);
    let framebuffers = create_framebuffers(
        &ctx.device,
        &image_views,
        render_pass,
        &swapchain_image_extent,
    );

    let uniforms = create_uniform_buffers(&ctx);
    let descriptor_pool = create_descriptor_pool(&ctx.device);

    let descriptor_set_layout = create_descriptor_set_layout(&ctx);
    let descriptor_set_layouts = [descriptor_set_layout; 2];

    let descriptor_sets = create_descriptor_sets(
        &ctx.device,
        descriptor_pool,
        &descriptor_set_layouts,
        &uniforms,
    );

    let (pipeline, pipeline_layout) = create_pipeline(
        &ctx,
        &swapchain_image_extent,
        &shader_stages,
        render_pass,
        &descriptor_set_layouts,
    );

    drop(shader_stages);

    let mut swapchain_state = SwapchainState {
        swapchain,
        image_views,
        framebuffers,
        image_extent: swapchain_image_extent,
    };

    let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
        &ctx,
        &geometry.vertices(),
        ctx.physical_device.queue_families.graphics,
        ctx.queues.graphics,
    );

    let (index_buffer, index_buffer_memory) = create_index_buffer(
        &ctx,
        &geometry.indices(),
        ctx.physical_device.queue_families.graphics,
        ctx.queues.graphics,
    );

    // https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Command_buffers
    let command_pool_info = vk::CommandPoolCreateInfoBuilder::new()
        .queue_family_index(ctx.physical_device.queue_families.graphics)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
    let command_pool = ctx
        .device
        .create_command_pool(&command_pool_info, None)
        .unwrap();

    let cmd_buf_allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(swapchain_state.framebuffers.len() as _);
    let cmd_bufs = ctx
        .device
        .allocate_command_buffers(&cmd_buf_allocate_info)
        .unwrap();

    // https://vulkan-tutorial.com/en/Drawing_a_triangle/Drawing/Rendering_and_presentation
    let mut sync_pool = SyncPool::new();
    let image_available_semaphores: Vec<_> = (0..FRAMES_IN_FLIGHT)
        .map(|_| sync_pool.semaphore(&ctx.device))
        .collect();
    let render_finished_semaphores: Vec<_> = (0..FRAMES_IN_FLIGHT)
        .map(|_| sync_pool.semaphore(&ctx.device))
        .collect();

    let in_flight_fences: Vec<_> = (0..FRAMES_IN_FLIGHT)
        .map(|_| sync_pool.fence(&ctx.device, true))
        .collect();

    let mut frame = 0;
    let mut framebuffer_resized = false;
    let mut prev_cur_time = Instant::now();
    let mut angle = 0.0;
    let speed = 10.0;
    let fov = 45.0;

    let PhysicalSize { width, height } = window.inner_size();
    let mut transform = Transform::new_test(45.0, width as f32 / height as f32);

    #[allow(clippy::collapsible_match, clippy::single_match)]
    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::Poll;
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized(_) => {
                framebuffer_resized = true;
                let size = window.inner_size();
                transform = transform.with_ortho(fov, aspect_ratio(size));
            }
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            _ => (),
        },
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(keycode),
                state,
                ..
            }) => match (keycode, state) {
                (VirtualKeyCode::Escape, ElementState::Released) => {
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            },
            _ => (),
        },
        Event::MainEventsCleared => {
            let cur_time = Instant::now();
            let delta = cur_time.duration_since(prev_cur_time).as_secs_f32();
            prev_cur_time = cur_time;
            angle += delta * speed;
            if angle > 360.0 {
                angle -= 360.0
            }
            transform = transform.with_model(Matrix4::from_angle_z(Deg(angle)));

            draw(
                &ctx,
                &mut swapchain_state,
                &in_flight_fences,
                &image_available_semaphores,
                &render_finished_semaphores,
                frame,
                &cmd_bufs,
                &mut framebuffer_resized,
                window.inner_size(),
                ctx.surface,
                render_pass,
                ctx.queues.graphics,
                &geometry,
                pipeline,
                pipeline_layout,
                vertex_buffer,
                index_buffer,
                &descriptor_sets,
                &transform,
                &uniforms,
            );

            frame = (frame + 1) % FRAMES_IN_FLIGHT;
        }
        Event::LoopDestroyed => {
            release_resources(
                &ctx,
                &uniforms,
                vertex_buffer,
                vertex_buffer_memory,
                index_buffer,
                index_buffer_memory,
                &mut sync_pool,
                &shader,
                descriptor_pool,
                command_pool,
                pipeline,
                pipeline_layout,
                descriptor_set_layout,
                &swapchain_state,
                render_pass,
                ctx.surface,
            );
            info!("Exited cleanly");
        }
        _ => (),
    })
}

unsafe fn cleanup_swapchain(device: &Arc<DeviceLoader>, state: &SwapchainState) {
    for fb in &state.framebuffers {
        device.destroy_framebuffer(*fb, None);
    }

    for iv in &state.image_views {
        device.destroy_image_view(*iv, None);
    }

    device.destroy_swapchain_khr(state.swapchain, None);
}

struct SwapchainState {
    swapchain: vk::SwapchainKHR,
    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>,
    image_extent: Extent2D,
}

unsafe fn recreate_swapchain(
    ctx: &Context,
    state: &SwapchainState,
    surface: vk::SurfaceKHR,
    draw_area_size: PhysicalSize<u32>,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    render_pass: vk::RenderPass,
) -> SwapchainState {
    ctx.device.device_wait_idle().unwrap();

    cleanup_swapchain(&ctx.device, &state);

    let (swapchain, surface_extent) =
        create_swapchain(ctx, surface, draw_area_size, format, present_mode);
    let image_views = create_image_views(&ctx.device, swapchain, format);
    let framebuffers = create_framebuffers(&ctx.device, &image_views, render_pass, &surface_extent);

    let new_state = SwapchainState {
        swapchain,
        framebuffers,
        image_views,
        image_extent: surface_extent,
    };

    new_state
}

unsafe fn create_swapchain(
    ctx: &Context,
    surface: vk::SurfaceKHR,
    surface_size: PhysicalSize<u32>,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
) -> (vk::SwapchainKHR, vk::Extent2D) {
    let surface_caps = ctx
        .instance
        .get_physical_device_surface_capabilities_khr(ctx.physical_device.handle, surface)
        .unwrap();
    let mut image_count = surface_caps.min_image_count + 1;
    if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
        image_count = surface_caps.max_image_count;
    }

    // https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkSurfaceCapabilitiesKHR.html#_description
    let swapchain_image_extent = match surface_caps.current_extent {
        vk::Extent2D {
            width: u32::MAX,
            height: u32::MAX,
        } => {
            let PhysicalSize { width, height } = surface_size;
            vk::Extent2D { width, height }
        }
        normal => normal,
    };

    let swapchain_info = vk::SwapchainCreateInfoKHRBuilder::new()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(swapchain_image_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_caps.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain = ctx
        .device
        .create_swapchain_khr(&swapchain_info, None)
        .unwrap();

    (swapchain, swapchain_image_extent)
}

unsafe fn create_image_views(
    device: &Arc<DeviceLoader>,
    swapchain: vk::SwapchainKHR,
    format: vk::SurfaceFormatKHR,
) -> Vec<vk::ImageView> {
    // https://vulkan-tutorial.com/Drawing_a_triangle/Presentation/Swap_chain
    let swapchain_images = device.get_swapchain_images_khr(swapchain, None).unwrap();

    // https://vulkan-tutorial.com/Drawing_a_triangle/Presentation/Image_views
    swapchain_images
        .iter()
        .map(|swapchain_image| {
            let image_view_info = vk::ImageViewCreateInfoBuilder::new()
                .image(*swapchain_image)
                .view_type(vk::ImageViewType::_2D)
                .format(format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(
                    vk::ImageSubresourceRangeBuilder::new()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                );
            device.create_image_view(&image_view_info, None).unwrap()
        })
        .collect()
}

unsafe fn create_framebuffers(
    device: &Arc<DeviceLoader>,
    image_views: &[vk::ImageView],
    render_pass: vk::RenderPass,
    extent: &vk::Extent2D,
) -> Vec<vk::Framebuffer> {
    // https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Framebuffers
    image_views
        .iter()
        .map(|image_view| {
            let attachments = vec![*image_view];
            let framebuffer_info = vk::FramebufferCreateInfoBuilder::new()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            device.create_framebuffer(&framebuffer_info, None).unwrap()
        })
        .collect()
}

unsafe fn create_render_pass(ctx: &Context) -> vk::RenderPass {
    // https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Render_passes
    let attachments = vec![vk::AttachmentDescriptionBuilder::new()
        .format(ctx.physical_device.surface_format.format)
        .samples(vk::SampleCountFlagBits::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];

    let color_attachment_refs = vec![vk::AttachmentReferenceBuilder::new()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let subpasses = vec![vk::SubpassDescriptionBuilder::new()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)];
    let dependencies = vec![vk::SubpassDependencyBuilder::new()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];

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
    swapchain_image_extent: &vk::Extent2D,
    shader_stages: &[vk::PipelineShaderStageCreateInfoBuilder],
    render_pass: vk::RenderPass,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> (vk::Pipeline, vk::PipelineLayout) {
    // https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Fixed_functions
    let attribute_descs = Vertex::attribute_descs();
    let binding_descs = [Vertex::binding_desc()];
    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
        .vertex_attribute_descriptions(&attribute_descs)
        .vertex_binding_descriptions(&binding_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewports = vec![vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(swapchain_image_extent.width as f32)
        .height(swapchain_image_extent.height as f32)
        .max_depth(1.0)];
    let scissors = vec![vk::Rect2DBuilder::new()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(*swapchain_image_extent)];
    let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
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

    let pipeline_layout_info =
        vk::PipelineLayoutCreateInfoBuilder::new().set_layouts(&descriptor_set_layouts);
    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&pipeline_layout_info, None)
        .unwrap();

    let dynamic_state_info = vk::PipelineDynamicStateCreateInfoBuilder::new()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    // https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Conclusion
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
        .dynamic_state(&dynamic_state_info);

    let pipeline = ctx
        .device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        .unwrap()[0];

    (pipeline, pipeline_layout)
}

unsafe fn record_command_buffer(
    device: &Arc<DeviceLoader>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    cmd_buf: vk::CommandBuffer,
    index_count: usize,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    descriptor_set: vk::DescriptorSet,
    draw_area_extent: &vk::Extent2D,
) {
    let cmd_buf_begin_info = vk::CommandBufferBeginInfoBuilder::new();
    device
        .begin_command_buffer(cmd_buf, &cmd_buf_begin_info)
        .unwrap();

    let clear_values = vec![vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    }];
    let render_pass_begin_info = vk::RenderPassBeginInfoBuilder::new()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: *draw_area_extent,
        })
        .clear_values(&clear_values);

    device.cmd_begin_render_pass(
        cmd_buf,
        &render_pass_begin_info,
        vk::SubpassContents::INLINE,
    );

    device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);

    device.cmd_bind_vertex_buffers(cmd_buf, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(cmd_buf, index_buffer, 0, vk::IndexType::UINT16);

    device.cmd_bind_descriptor_sets(
        cmd_buf,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline_layout,
        0,
        &[descriptor_set],
        &[],
    );

    let viewport = vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(draw_area_extent.width as f32)
        .height(draw_area_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd_buf, 0, &[viewport]);

    let scissor = vk::Rect2D {
        extent: *draw_area_extent,
        offset: vk::Offset2D { x: 0, y: 0 },
    }
    .into_builder();
    device.cmd_set_scissor(cmd_buf, 0, &[scissor]);

    device.cmd_draw_indexed(cmd_buf, index_count as u32, 1, 0, 0, 0);
    device.cmd_end_render_pass(cmd_buf);

    device.end_command_buffer(cmd_buf).unwrap();
}

unsafe fn allocate_buffer(
    ctx: &Context,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_info = vk::BufferCreateInfoBuilder::new()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = ctx
        .device
        .create_buffer(&buffer_info, None)
        .expect("Failed to create buffer");

    let mem_reqs = ctx.device.get_buffer_memory_requirements(buffer);
    let mem_type = find_memory_type(ctx, mem_reqs.memory_type_bits, properties);

    let allocate_info = vk::MemoryAllocateInfoBuilder::new()
        .allocation_size(mem_reqs.size)
        .memory_type_index(mem_type);
    let buffer_memory = ctx
        .device
        .allocate_memory(&allocate_info, None)
        .expect("Failed to allocate memory for vertex buffer");

    ctx.device
        .bind_buffer_memory(buffer, buffer_memory, 0)
        .expect("Failed to bind memory");

    (buffer, buffer_memory)
}

unsafe fn create_vertex_buffer(
    ctx: &Context,
    vertices: &[Vertex],
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let size = (size_of_val(&vertices[0]) * vertices.len()) as u64;
    let (staging_buf, staging_mem) = allocate_buffer(
        ctx,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    let data = ctx
        .device
        .map_memory(staging_mem, 0, size, vk::MemoryMapFlags::empty())
        .unwrap();
    data.copy_from_nonoverlapping(vertices.as_ptr() as *const c_void, size as usize);
    ctx.device.unmap_memory(staging_mem);

    let (vertex_buf, vertex_mem) = allocate_buffer(
        ctx,
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    copy_buffer(
        ctx,
        vertex_buf,
        staging_buf,
        size,
        copy_queue_family,
        copy_queue,
    );

    ctx.device.destroy_buffer(staging_buf, None);
    ctx.device.free_memory(staging_mem, None);

    (vertex_buf, vertex_mem)
}

unsafe fn copy_buffer(
    ctx: &Context,
    dst_buf: vk::Buffer,
    src_buf: vk::Buffer,
    size: vk::DeviceSize,
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) {
    let command_pool_ci = vk::CommandPoolCreateInfoBuilder::new()
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(copy_queue_family);

    let command_pool = ctx
        .device
        .create_command_pool(&command_pool_ci, None)
        .expect("Failed to create transient command pool for staging buffer transfer");

    let alloc_info = vk::CommandBufferAllocateInfoBuilder::new()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1)
        .command_pool(command_pool);

    let cmd_bufs = ctx.device.allocate_command_buffers(&alloc_info).unwrap();
    let cmd_buf = cmd_bufs[0];

    let begin_info = vk::CommandBufferBeginInfoBuilder::new()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    ctx.device
        .begin_command_buffer(cmd_buf, &begin_info)
        .unwrap();

    let copy_region = vk::BufferCopyBuilder::new().size(size);

    ctx.device
        .cmd_copy_buffer(cmd_buf, src_buf, dst_buf, &[copy_region]);

    ctx.device.end_command_buffer(cmd_buf).unwrap();

    let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&cmd_bufs);

    ctx.device
        .queue_submit(copy_queue, &[submit_info], vk::Fence::null())
        .unwrap();
    ctx.device.queue_wait_idle(copy_queue).unwrap();

    ctx.device.free_command_buffers(command_pool, &cmd_bufs);
    ctx.device.destroy_command_pool(command_pool, None);
}

unsafe fn create_index_buffer(
    ctx: &Context,
    indices: &[u16],
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buf_size = (size_of_val(&indices[0]) * indices.len()) as u64;

    let (staging_buf, staging_mem) = allocate_buffer(
        ctx,
        buf_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    );

    let data = ctx
        .device
        .map_memory(staging_mem, 0, buf_size, vk::MemoryMapFlags::empty())
        .unwrap();

    std::ptr::copy_nonoverlapping(indices.as_ptr() as *const c_void, data, buf_size as usize);
    ctx.device.unmap_memory(staging_mem);

    let (index_buf, index_mem) = allocate_buffer(
        ctx,
        buf_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    copy_buffer(
        ctx,
        index_buf,
        staging_buf,
        buf_size,
        copy_queue_family,
        copy_queue,
    );

    ctx.device.destroy_buffer(staging_buf, None);
    ctx.device.free_memory(staging_mem, None);

    (index_buf, index_mem)
}

unsafe fn find_memory_type(
    ctx: &Context,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    let mem_properties = ctx
        .instance
        .get_physical_device_memory_properties(ctx.physical_device.handle);

    for (ix, mem_type) in mem_properties.memory_types.iter().enumerate() {
        if type_filter & (1 << ix) != 0 && (properties & mem_type.property_flags) == properties {
            return ix as u32;
        }
    }

    panic!("Failed to find suitable memory type");
}

unsafe fn create_uniform_buffers(ctx: &Context) -> Vec<(vk::Buffer, vk::DeviceMemory)> {
    let buf_size = size_of::<Transform>() as u64;
    (0..FRAMES_IN_FLIGHT)
        .map(|_| {
            allocate_buffer(
                ctx,
                buf_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
        })
        .collect()
}

unsafe fn upload_uniform_buffers(
    device: &Arc<DeviceLoader>,
    transform: &Transform,
    uniform_mem: vk::DeviceMemory,
) {
    let size = size_of_val(transform);
    let data = device
        .map_memory(uniform_mem, 0, size as u64, vk::MemoryMapFlags::empty())
        .unwrap();

    std::ptr::copy_nonoverlapping(transform as *const Transform as *const c_void, data, size);

    device.unmap_memory(uniform_mem);
}

unsafe fn create_descriptor_set_layout(ctx: &Context) -> vk::DescriptorSetLayout {
    let bindings = [Transform::layout()];
    let layout_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

    let descriptor_set_layout = ctx
        .device
        .create_descriptor_set_layout(&layout_info, None)
        .expect("Failed to create descriptor set layout");

    descriptor_set_layout
}

unsafe fn create_descriptor_pool(device: &Arc<DeviceLoader>) -> vk::DescriptorPool {
    let pool_size = vk::DescriptorPoolSizeBuilder::new()
        ._type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(FRAMES_IN_FLIGHT as u32);

    let pool_sizes = [pool_size];
    let pool_ci = vk::DescriptorPoolCreateInfoBuilder::new()
        .pool_sizes(&pool_sizes)
        .max_sets(FRAMES_IN_FLIGHT as u32);

    device
        .create_descriptor_pool(&pool_ci, None)
        .expect("Failed to create a descriptor pool")
}

unsafe fn create_descriptor_sets(
    device: &Arc<DeviceLoader>,
    pool: vk::DescriptorPool,
    layouts: &[vk::DescriptorSetLayout],
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
) -> Vec<vk::DescriptorSet> {
    let alloc_info = vk::DescriptorSetAllocateInfoBuilder::new()
        .descriptor_pool(pool)
        .set_layouts(layouts);

    let descriptor_sets = device
        .allocate_descriptor_sets(&alloc_info)
        .expect("Faled to allocate descriptor sets")
        .to_vec();

    for ((b, _), ds) in uniforms.iter().zip(descriptor_sets.iter()) {
        let desc_buffer_info = vk::DescriptorBufferInfoBuilder::new()
            .buffer(*b)
            .offset(0)
            .range(size_of::<Transform>() as u64);
        let desc_buffer_infos = [desc_buffer_info];

        let descriptor_write = vk::WriteDescriptorSetBuilder::new()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .dst_set(*ds)
            .dst_binding(0)
            .dst_array_element(0)
            .buffer_info(&desc_buffer_infos);

        device.update_descriptor_sets(&[descriptor_write], &[]);
    }

    descriptor_sets
}

fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D32_SFLOAT_S8_UINT
}

unsafe fn find_depth_format(ctx: &Context) -> Option<vk::Format> {
    find_supported_format(
        ctx,
        &[
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ],
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    )
}

unsafe fn find_supported_format(
    ctx: &Context,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Option<vk::Format> {
    candidates
        .iter()
        .find(|&format| {
            let props = ctx
                .instance
                .get_physical_device_format_properties(ctx.physical_device.handle, *format);

            let mut format_suitable = false;
            match tiling {
                vk::ImageTiling::LINEAR => {
                    if (props.linear_tiling_features & features) == features {
                        format_suitable = true;
                    }
                }
                vk::ImageTiling::OPTIMAL => {
                    if (props.optimal_tiling_features & features) == features {
                        format_suitable = true;
                    }
                }
                _ => {}
            }

            format_suitable
        })
        .map(|&f| f)
}

unsafe fn draw(
    ctx: &Context,
    swapchain_state: &mut SwapchainState,
    in_flight_fences: &[vk::Fence],
    image_available_semaphores: &[vk::Semaphore],
    render_finished_semaphores: &[vk::Semaphore],
    frame: usize,
    cmd_bufs: &[vk::CommandBuffer],
    framebuffer_resized: &mut bool,
    window_size: PhysicalSize<u32>,
    surface: vk::SurfaceKHR,
    render_pass: vk::RenderPass,
    queue: vk::Queue,
    geometry: &Geometry,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    descriptor_sets: &[vk::DescriptorSet],
    transform: &Transform,
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
) {
    ctx.device
        .wait_for_fences(&[in_flight_fences[frame]], true, u64::MAX)
        .unwrap();

    let maybe_image = ctx.device.acquire_next_image_khr(
        swapchain_state.swapchain,
        u64::MAX,
        image_available_semaphores[frame],
        vk::Fence::null(),
    );

    if maybe_image.raw == vk::Result::ERROR_OUT_OF_DATE_KHR || *framebuffer_resized {
        *framebuffer_resized = false;
        *swapchain_state = recreate_swapchain(
            &ctx,
            &swapchain_state,
            surface,
            window_size,
            ctx.physical_device.surface_format,
            ctx.physical_device.present_mode,
            render_pass,
        );
        return;
    } else if maybe_image.raw != vk::Result::SUCCESS {
        panic!("Failed to acquire image from swapchain, aborting...");
    }
    let image_index = maybe_image.value.unwrap();

    let buf = cmd_bufs[frame];
    ctx.device
        .reset_command_buffer(buf, CommandBufferResetFlags::empty())
        .unwrap();

    upload_uniform_buffers(&ctx.device, &transform, uniforms[frame].1);

    record_command_buffer(
        &ctx.device,
        pipeline,
        pipeline_layout,
        buf,
        geometry.indices().len(),
        vertex_buffer,
        index_buffer,
        render_pass,
        swapchain_state.framebuffers[image_index as usize],
        descriptor_sets[frame],
        &swapchain_state.image_extent,
    );

    let wait_semaphores = vec![image_available_semaphores[frame]];
    let command_buffers = vec![buf];
    let signal_semaphores = vec![render_finished_semaphores[frame]];

    let submit_info = vk::SubmitInfoBuilder::new()
        .wait_semaphores(&wait_semaphores)
        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .command_buffers(&command_buffers)
        .signal_semaphores(&signal_semaphores);
    let in_flight_fence = in_flight_fences[frame];
    ctx.device.reset_fences(&[in_flight_fence]).unwrap();
    ctx.device
        .queue_submit(queue, &[submit_info], in_flight_fence)
        .unwrap();

    let swapchains = vec![swapchain_state.swapchain];
    let image_indices = vec![image_index];
    let present_info = vk::PresentInfoKHRBuilder::new()
        .wait_semaphores(&signal_semaphores)
        .swapchains(&swapchains)
        .image_indices(&image_indices);

    ctx.device.queue_present_khr(queue, &present_info).unwrap();
}

unsafe fn release_resources(
    ctx: &Context,
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    sync_pool: &mut SyncPool,
    shader: &Shader,
    descriptor_pool: vk::DescriptorPool,
    command_pool: vk::CommandPool,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    swapchain_state: &SwapchainState,
    render_pass: vk::RenderPass,
    surface: vk::SurfaceKHR,
) {
    ctx.device.device_wait_idle().unwrap();

    for (b, m) in uniforms {
        ctx.device.destroy_buffer(*b, None);
        ctx.device.free_memory(*m, None);
    }

    ctx.device.destroy_buffer(vertex_buffer, None);
    ctx.device.free_memory(vertex_buffer_memory, None);

    ctx.device.destroy_buffer(index_buffer, None);
    ctx.device.free_memory(index_buffer_memory, None);

    sync_pool.destroy_all(&ctx.device);
    shader.destroy(&ctx.device);

    ctx.device.destroy_descriptor_pool(descriptor_pool, None);

    ctx.device.destroy_command_pool(command_pool, None);

    ctx.device.destroy_pipeline(pipeline, None);

    ctx.device.destroy_render_pass(render_pass, None);

    ctx.device.destroy_pipeline_layout(pipeline_layout, None);

    ctx.device
        .destroy_descriptor_set_layout(descriptor_set_layout, None);

    cleanup_swapchain(&ctx.device, &swapchain_state);

    ctx.instance.destroy_surface_khr(surface, None);

    validation::deinit(&ctx.instance);
}

fn aspect_ratio(size: PhysicalSize<u32>) -> f32 {
    let PhysicalSize { width, height } = size;
    width as f32 / height as f32
}
