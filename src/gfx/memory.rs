use std::ffi::c_void;
use std::mem::{size_of, size_of_val};
use std::ptr;

use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;

use crate::gfx::context::Context;
use crate::gfx::g;
use crate::gfx::geometry::Vertex;
use crate::gfx::physical_device::PhysicalDevice;
use crate::transform::Transform;

pub struct UniformBuffer {
    memory: vk::DeviceMemory,
    handle: vk::Buffer,
}

pub unsafe fn release_resources(
    ctx: &mut Context,
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
    descriptor_pool: vk::DescriptorPool,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
    render_pass: vk::RenderPass,
    sampler: vk::Sampler,
) {
    ctx.device.device_wait_idle().unwrap();

    ctx.device.destroy_sampler(sampler, None);

    for (b, m) in uniforms {
        ctx.device.destroy_buffer(*b, None);
        ctx.device.free_memory(*m, None);
    }

    ctx.device.destroy_descriptor_pool(descriptor_pool, None);

    ctx.device.destroy_pipeline(pipeline, None);

    ctx.device.destroy_render_pass(render_pass, None);

    ctx.device.destroy_pipeline_layout(pipeline_layout, None);

    for &layout in descriptor_set_layouts {
        ctx.device.destroy_descriptor_set_layout(layout, None);
    }
}

pub unsafe fn copy_to_gpu(
    device: &DeviceLoader,
    src: *const c_void,
    dst: vk::DeviceMemory,
    size: usize,
) {
    let data = device
        .map_memory(dst, 0, size as vk::DeviceSize, vk::MemoryMapFlags::empty())
        .expect("failed to map memory");
    ptr::copy_nonoverlapping(src, data, size as usize);
    device.unmap_memory(dst);
}

pub unsafe fn create_image(
    device: &DeviceLoader,
    physical_device: &PhysicalDevice,
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> (vk::Image, vk::DeviceMemory) {
    let image_info = vk::ImageCreateInfoBuilder::new()
        .image_type(vk::ImageType::_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlagBits::_1)
        .flags(vk::ImageCreateFlags::empty());

    let image = device
        .create_image(&image_info, None)
        .expect("failed to create texture image");

    let mem_reqs = device.get_image_memory_requirements(image);
    let mem_type_index = find_memory_type(
        physical_device.memory_properties,
        mem_reqs.memory_type_bits,
        properties,
    );
    let alloc_info = vk::MemoryAllocateInfoBuilder::new()
        .allocation_size(mem_reqs.size)
        .memory_type_index(mem_type_index);

    let mem = device
        .allocate_memory(&alloc_info, None)
        .expect("failed to allocate memory");
    device
        .bind_image_memory(image, mem, 0)
        .expect("failed to bind image memory");

    (image, mem)
}

pub unsafe fn upload_uniform_buffers(
    device: &DeviceLoader,
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

pub unsafe fn create_uniform_buffers(
    ctx: &Context,
    size: usize,
    count: usize,
) -> SmallVec<[(vk::Buffer, vk::DeviceMemory); 2]> {
    (0..count)
        .map(|_| {
            allocate_buffer(
                &ctx.device,
                &ctx.physical_device,
                size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
        })
        .collect()
}

pub unsafe fn allocate_buffer(
    device: &DeviceLoader,
    physical_device: &PhysicalDevice,
    size: usize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_info = vk::BufferCreateInfoBuilder::new()
        .size(size as vk::DeviceSize)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device
        .create_buffer(&buffer_info, None)
        .expect("Failed to create buffer");

    let mem_reqs = device.get_buffer_memory_requirements(buffer);
    let mem_type = find_memory_type(
        physical_device.memory_properties,
        mem_reqs.memory_type_bits,
        properties,
    );

    let allocate_info = vk::MemoryAllocateInfoBuilder::new()
        .allocation_size(mem_reqs.size)
        .memory_type_index(mem_type);
    let buffer_memory = device
        .allocate_memory(&allocate_info, None)
        .expect("Failed to allocate memory for vertex buffer");

    device
        .bind_buffer_memory(buffer, buffer_memory, 0)
        .expect("Failed to bind memory");

    (buffer, buffer_memory)
}

pub unsafe fn create_vertex_buffer(
    ctx: &Context,
    vertices: &[Vertex],
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let size = size_of_val(&vertices[0]) * vertices.len();
    let (staging_buf, staging_mem) = allocate_buffer(
        &ctx.device,
        &ctx.physical_device,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    copy_to_gpu(
        &ctx.device,
        vertices.as_ptr() as *const c_void,
        staging_mem,
        size,
    );

    let (vertex_buf, vertex_mem) = allocate_buffer(
        &ctx.device,
        &ctx.physical_device,
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

pub unsafe fn copy_buffer_to_image(
    device: &DeviceLoader,
    buffer: vk::Buffer,
    image: vk::Image,
    size: (u32, u32),
    copy_queue: vk::Queue,
    copy_queue_family: u32,
) {
    let (cmd_buf, cmd_pool) = g::begin_once_commands(device, copy_queue_family);

    let region = vk::BufferImageCopyBuilder::new()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width: size.0,
            height: size.1,
            depth: 1,
        });

    device.cmd_copy_buffer_to_image(
        cmd_buf,
        buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );

    g::end_once_commands(device, cmd_pool, cmd_buf, copy_queue)
}

pub unsafe fn copy_buffer(
    ctx: &Context,
    dst_buf: vk::Buffer,
    src_buf: vk::Buffer,
    size: usize,
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) {
    let (cmd_buf, cmd_pool) = g::begin_once_commands(&ctx.device, copy_queue_family);

    let copy_region = vk::BufferCopyBuilder::new().size(size as vk::DeviceSize);

    ctx.device
        .cmd_copy_buffer(cmd_buf, src_buf, dst_buf, &[copy_region]);

    g::end_once_commands(&ctx.device, cmd_pool, cmd_buf, copy_queue);
}

pub unsafe fn create_index_buffer(
    ctx: &Context,
    indices: &[u16],
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buf_size = size_of_val(&indices[0]) * indices.len();

    let (staging_buf, staging_mem) = allocate_buffer(
        &ctx.device,
        &ctx.physical_device,
        buf_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    );

    copy_to_gpu(
        &ctx.device,
        indices.as_ptr() as *const c_void,
        staging_mem,
        buf_size,
    );

    let (index_buf, index_mem) = allocate_buffer(
        &ctx.device,
        &ctx.physical_device,
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

pub unsafe fn find_memory_type(
    mem_properties: vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    for (ix, mem_type) in mem_properties.memory_types.iter().enumerate() {
        if type_filter & (1 << ix) != 0 && (properties & mem_type.property_flags) == properties {
            return ix as u32;
        }
    }

    panic!("failed to find suitable memory type");
}

pub unsafe fn create_command_pool(device: &DeviceLoader, queue_family: u32) -> vk::CommandPool {
    let info = vk::CommandPoolCreateInfoBuilder::new()
        .queue_family_index(queue_family)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    device
        .create_command_pool(&info, None)
        .expect("failed to create command pool")
}

pub unsafe fn create_command_buffers(
    device: &DeviceLoader,
    cmd_pool: vk::CommandPool,
    count: usize,
) -> SmallVec<[vk::CommandBuffer; 8]> {
    let cmd_buf_allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
        .command_pool(cmd_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count as _);
    device
        .allocate_command_buffers(&cmd_buf_allocate_info)
        .expect("failed to create command buffer")
}

pub unsafe fn create_depth_buffer(
    device: &DeviceLoader,
    physical_device: &PhysicalDevice,
    format: vk::Format,
    extent: &vk::Extent2D,
) -> (vk::Image, vk::DeviceMemory) {
    create_image(
        device,
        physical_device,
        extent.width,
        extent.height,
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
}

fn has_stencil_component(format: vk::Format) -> bool {
    format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D32_SFLOAT_S8_UINT
}

pub unsafe fn create_image_view(
    device: &DeviceLoader,
    image: vk::Image,
    format: vk::Format,
    aspect_flags: vk::ImageAspectFlags,
) -> vk::ImageView {
    let image_view_info = vk::ImageViewCreateInfoBuilder::new()
        .image(image)
        .view_type(vk::ImageViewType::_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: aspect_flags,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    device
        .create_image_view(&image_view_info, None)
        .expect("failed to create texture view")
}
