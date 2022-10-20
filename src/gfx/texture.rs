use std::ffi::c_void;
use std::io;
use std::path::Path;

use erupt::{vk, DeviceLoader};
use image::io::Reader as ImageReader;
use image::EncodableLayout;

use crate::gfx::context::Context;
use crate::gfx::g;
use crate::gfx::memory;

pub unsafe fn create_sampler(ctx: &Context) -> vk::Sampler {
    let max_anisotropy = ctx.physical_device.properties.limits.max_sampler_anisotropy;
    let info = vk::SamplerCreateInfoBuilder::new()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::NEAREST)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(max_anisotropy)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);

    ctx.device
        .create_sampler(&info, None)
        .expect("failed to create a texture sampler")
}

pub unsafe fn create_texture(
    ctx: &Context,
    path: &Path,
    copy_queue: vk::Queue,
    copy_queue_family: u32,
) -> io::Result<(vk::Image, vk::DeviceMemory)> {
    let image = ImageReader::open(path)?
        .decode()
        .expect("failed to decode image at {:path}");
    let image = image.to_rgba8();
    let image_size = image.as_bytes().len();

    let (staging_buf, staging_mem) = memory::allocate_buffer(
        ctx,
        image_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    memory::copy_to_gpu(
        &ctx.device,
        image.as_bytes().as_ptr() as *const c_void,
        staging_mem,
        image_size,
    );

    let (texture, texture_mem) = memory::create_image(
        &ctx,
        image.width(),
        image.height(),
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    transition_image_layout(
        &ctx.device,
        texture,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        copy_queue_family,
        copy_queue,
    );

    memory::copy_buffer_to_image(
        &ctx.device,
        staging_buf,
        texture,
        (image.width(), image.height()),
        copy_queue,
        copy_queue_family,
    );

    transition_image_layout(
        &ctx.device,
        texture,
        vk::Format::R8G8B8A8_SNORM,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        copy_queue_family,
        copy_queue,
    );

    ctx.device.destroy_buffer(staging_buf, None);
    ctx.device.free_memory(staging_mem, None);

    Ok((texture, texture_mem))
}

pub unsafe fn create_texture_view(device: &DeviceLoader, texture: vk::Image) -> vk::ImageView {
    let image_view_info = vk::ImageViewCreateInfoBuilder::new()
        .image(texture)
        .view_type(vk::ImageViewType::_2D)
        .format(vk::Format::R8G8B8A8_SRGB)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    device
        .create_image_view(&image_view_info, None)
        .expect("failed to create texture view")
}

pub unsafe fn transition_image_layout(
    device: &DeviceLoader,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) {
    let (cmd_buf, cmd_pool) = g::begin_once_commands(device, copy_queue_family);

    let mut barrier = vk::ImageMemoryBarrierBuilder::new()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let source_stage;
    let destination_stage;
    match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
            barrier = barrier
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        }
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
            barrier = barrier
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        }
        _ => panic!("unsupported layout transition"),
    }

    device.cmd_pipeline_barrier(
        cmd_buf,
        source_stage,
        destination_stage,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
    );

    g::end_once_commands(device, cmd_pool, cmd_buf, copy_queue);
}
