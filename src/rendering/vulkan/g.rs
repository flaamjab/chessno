use erupt::{vk, vk1_0::CommandBufferResetFlags, DeviceLoader};

use crate::rendering::vulkan::context::Context;

use super::swapchain::Swapchain;

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

    let clear_values = [
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.1, 0.0, 0.0, 1.0],
            },
        },
        vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        },
    ];

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

pub unsafe fn present(
    device: &DeviceLoader,
    swapchain: &Swapchain,
    image_index: u32,
    render_finished_semaphore: vk::Semaphore,
) {
    let swapchains = [swapchain.handle()];
    let image_indices = [image_index];
    let semaphores = [render_finished_semaphore];
    let present_info = vk::PresentInfoKHRBuilder::new()
        .wait_semaphores(&semaphores)
        .swapchains(&swapchains)
        .image_indices(&image_indices);

    device
        .queue_present_khr(swapchain.queue(), &present_info)
        .unwrap();
}
