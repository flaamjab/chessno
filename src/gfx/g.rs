use std::ffi::c_void;
use std::mem::size_of;

use erupt::{vk, vk1_0::CommandBufferResetFlags, DeviceLoader};
use memoffset::offset_of;

use crate::gfx::context::Context;
use crate::gfx::geometry::Vertex;
use crate::gfx::spatial::Spatial;
use crate::logging::trace;

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
                offset: offset_of!(Vertex, uv) as u32,
            }
            .into_builder(),
        ]
        .into()
    }
}

pub unsafe fn create_render_pass(ctx: &Context) -> vk::RenderPass {
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

pub unsafe fn create_pipeline(
    ctx: &Context,
    shader_stages: &[vk::PipelineShaderStageCreateInfoBuilder],
    render_pass: vk::RenderPass,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> (vk::Pipeline, vk::PipelineLayout) {
    let attribute_descs = Vertex::attribute_descs();
    let binding_descs = [Vertex::binding_desc()];
    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
        .vertex_attribute_descriptions(&attribute_descs)
        .vertex_binding_descriptions(&binding_descs);

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

pub unsafe fn setup_draw_state(
    device: &DeviceLoader,
    cmd_buf: vk::CommandBuffer,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    descriptor_set: vk::DescriptorSet,
    push_constant: &Spatial,
) {
    device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);

    device.cmd_bind_vertex_buffers(cmd_buf, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(cmd_buf, index_buffer, 0, vk::IndexType::UINT16);

    device.cmd_push_constants(
        cmd_buf,
        pipeline_layout,
        vk::ShaderStageFlags::VERTEX,
        0,
        size_of::<Spatial>() as _,
        push_constant as *const Spatial as *const c_void,
    );

    device.cmd_bind_descriptor_sets(
        cmd_buf,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline_layout,
        0,
        &[descriptor_set],
        &[],
    );
}

pub unsafe fn begin_once_commands(
    device: &DeviceLoader,
    copy_queue_family: u32,
) -> (vk::CommandBuffer, vk::CommandPool) {
    let cmd_pool = create_transient_command_pool(device, copy_queue_family);
    let alloc_info = vk::CommandBufferAllocateInfoBuilder::new()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(cmd_pool)
        .command_buffer_count(1);

    let cmd_buf = device
        .allocate_command_buffers(&alloc_info)
        .expect("failed to allocate command buffer")[0];

    let begin_info = vk::CommandBufferBeginInfoBuilder::new()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    device
        .begin_command_buffer(cmd_buf, &begin_info)
        .expect("failed to begin command buffer");

    (cmd_buf, cmd_pool)
}

pub unsafe fn end_once_commands(
    device: &DeviceLoader,
    cmd_pool: vk::CommandPool,
    cmd_buf: vk::CommandBuffer,
    queue: vk::Queue,
) {
    device
        .end_command_buffer(cmd_buf)
        .expect("failed to end command buffer");

    let cmd_bufs = [cmd_buf];
    let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&cmd_bufs);

    device
        .queue_submit(queue, &[submit_info], vk::Fence::null())
        .expect("failed to submit to queue");
    device
        .queue_wait_idle(queue)
        .expect("failed to await queue operations finished");

    device.free_command_buffers(cmd_pool, &[cmd_buf]);
    device.destroy_command_pool(cmd_pool, None);
}

pub unsafe fn create_transient_command_pool(
    device: &DeviceLoader,
    copy_queue_family: u32,
) -> vk::CommandPool {
    let command_pool_ci = vk::CommandPoolCreateInfoBuilder::new()
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(copy_queue_family);

    device
        .create_command_pool(&command_pool_ci, None)
        .expect("Failed to create transient command pool for staging buffer transfer")
}

pub unsafe fn acquire_image(
    ctx: &mut Context,
    in_flight_fence: vk::Fence,
    image_available_semaphore: vk::Semaphore,
    framebuffer_resized: &mut bool,
    window_size: &vk::Extent2D,
) -> Option<u32> {
    ctx.device
        .wait_for_fences(&[in_flight_fence], true, u64::MAX)
        .unwrap();

    let maybe_image = ctx.device.acquire_next_image_khr(
        ctx.swapchain.handle(),
        u64::MAX,
        image_available_semaphore,
        vk::Fence::null(),
    );

    if maybe_image.raw == vk::Result::ERROR_OUT_OF_DATE_KHR || *framebuffer_resized {
        ctx.device
            .queue_wait_idle(ctx.queues.graphics)
            .expect("failed to wait on queue");
        *framebuffer_resized = false;
        trace!("Recreating swapchain");
        ctx.swapchain
            .recreate(&ctx.device, &ctx.physical_device, ctx.surface, &window_size);
        return None;
    } else if maybe_image.raw != vk::Result::SUCCESS {
        panic!("failed to acquire image from swapchain, aborting...");
    }

    maybe_image.value
}

pub unsafe fn begin_draw(
    device: &DeviceLoader,
    cmd_buf: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    draw_area_size: &vk::Extent2D,
) {
    device
        .reset_command_buffer(cmd_buf, CommandBufferResetFlags::empty())
        .unwrap();

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
            extent: *draw_area_size,
        })
        .clear_values(&clear_values);

    device.cmd_begin_render_pass(
        cmd_buf,
        &render_pass_begin_info,
        vk::SubpassContents::INLINE,
    );

    let viewport = vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(draw_area_size.width as f32)
        .height(draw_area_size.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd_buf, 0, &[viewport]);

    let scissor = vk::Rect2D {
        extent: *draw_area_size,
        offset: vk::Offset2D { x: 0, y: 0 },
    }
    .into_builder();
    device.cmd_set_scissor(cmd_buf, 0, &[scissor]);
}

pub unsafe fn draw(
    device: &DeviceLoader,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    cmd_buf: vk::CommandBuffer,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: usize,
    mvp: &Spatial,
    descriptor_set: vk::DescriptorSet,
    uniform_mem: vk::DeviceMemory,
) {
    // memory::upload_uniform_buffers(&device, &transform, uniform_mem);
    setup_draw_state(
        &device,
        cmd_buf,
        pipeline,
        pipeline_layout,
        vertex_buffer,
        index_buffer,
        descriptor_set,
        &mvp,
    );
    device.cmd_draw_indexed(cmd_buf, index_count as u32, 1, 0, 0, 0);
}

pub unsafe fn end_draw(
    device: &DeviceLoader,
    graphics_queue: vk::Queue,
    cmd_buf: vk::CommandBuffer,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
) {
    device.cmd_end_render_pass(cmd_buf);
    device.end_command_buffer(cmd_buf).unwrap();

    let wait_semaphores = [image_available_semaphore];
    let command_buffers = [cmd_buf];
    let signal_semaphores = [render_finished_semaphore];
    let submit_info = vk::SubmitInfoBuilder::new()
        .wait_semaphores(&wait_semaphores)
        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .command_buffers(&command_buffers)
        .signal_semaphores(&signal_semaphores);

    device.reset_fences(&[in_flight_fence]).unwrap();

    device
        .queue_submit(graphics_queue, &[submit_info], in_flight_fence)
        .unwrap();
}

pub unsafe fn present(ctx: &Context, image_index: u32, render_finished_semaphore: vk::Semaphore) {
    let swapchains = vec![ctx.swapchain.handle()];
    let image_indices = vec![image_index];
    let semaphores = [render_finished_semaphore];
    let present_info = vk::PresentInfoKHRBuilder::new()
        .wait_semaphores(&semaphores)
        .swapchains(&swapchains)
        .image_indices(&image_indices);

    ctx.device
        .queue_present_khr(ctx.queues.graphics, &present_info)
        .unwrap();
}
