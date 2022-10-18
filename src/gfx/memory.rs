use std::ffi::c_void;
use std::mem::{size_of, size_of_val};
use std::ptr;

use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;

use crate::gfx::context::Context;
use crate::gfx::g;
use crate::gfx::geometry::Vertex;
use crate::gfx::gpu_program::GpuProgram;
use crate::gfx::transform::Transform;

pub unsafe fn release_resources(
    ctx: &mut Context,
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
    shader: &GpuProgram,
    descriptor_pool: vk::DescriptorPool,
    command_pool: vk::CommandPool,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    render_pass: vk::RenderPass,
    texture: vk::Image,
    texture_mem: vk::DeviceMemory,
    texture_view: vk::ImageView,
    sampler: vk::Sampler,
) {
    ctx.device.device_wait_idle().unwrap();

    ctx.device.destroy_sampler(sampler, None);
    ctx.device.destroy_image_view(texture_view, None);
    ctx.device.destroy_image(texture, None);
    ctx.device.free_memory(texture_mem, None);

    for (b, m) in uniforms {
        ctx.device.destroy_buffer(*b, None);
        ctx.device.free_memory(*m, None);
    }

    shader.destroy(&ctx.device);

    ctx.device.destroy_descriptor_pool(descriptor_pool, None);

    ctx.device.destroy_command_pool(command_pool, None);

    ctx.device.destroy_pipeline(pipeline, None);

    ctx.device.destroy_render_pass(render_pass, None);

    ctx.device.destroy_pipeline_layout(pipeline_layout, None);

    ctx.device
        .destroy_descriptor_set_layout(descriptor_set_layout, None);
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
    ctx: &Context,
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

    let device = &ctx.device;
    let image = device
        .create_image(&image_info, None)
        .expect("failed to create texture image");

    let mem_reqs = device.get_image_memory_requirements(image);
    let mem_type_index = find_memory_type(ctx, mem_reqs.memory_type_bits, properties);
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

pub unsafe fn create_descriptor_sets(
    device: &DeviceLoader,
    pool: vk::DescriptorPool,
    layouts: &[vk::DescriptorSetLayout],
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
    texture: (vk::ImageView, vk::Sampler),
    frames_in_flight: usize,
) -> Vec<vk::DescriptorSet> {
    let alloc_info = vk::DescriptorSetAllocateInfoBuilder::new()
        .descriptor_pool(pool)
        .set_layouts(layouts);

    let descriptor_sets = device
        .allocate_descriptor_sets(&alloc_info)
        .expect("failed to allocate descriptor sets")
        .to_vec();

    for ix in 0..frames_in_flight {
        let buffer_info = vk::DescriptorBufferInfoBuilder::new()
            .buffer(uniforms[ix].0)
            .offset(0)
            .range(size_of::<Transform>() as u64);

        let image_info = vk::DescriptorImageInfoBuilder::new()
            .image_view(texture.0)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(texture.1);

        let buffer_infos = [buffer_info];
        let image_infos = [image_info];

        let uniform_dw = vk::WriteDescriptorSetBuilder::new()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .dst_set(descriptor_sets[ix])
            .dst_binding(0)
            .dst_array_element(0)
            .buffer_info(&buffer_infos);

        let combined_sampler_dw = vk::WriteDescriptorSetBuilder::new()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_set(descriptor_sets[ix])
            .dst_binding(1)
            .dst_array_element(0)
            .image_info(&image_infos);

        device.update_descriptor_sets(&[uniform_dw, combined_sampler_dw], &[]);
    }

    descriptor_sets
}

pub unsafe fn create_descriptor_pool(
    device: &DeviceLoader,
    frames_in_flight: usize,
) -> vk::DescriptorPool {
    let pool_sizes = [
        vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(frames_in_flight as u32),
        vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(frames_in_flight as u32),
    ];
    let pool_info = vk::DescriptorPoolCreateInfoBuilder::new()
        .pool_sizes(&pool_sizes)
        .max_sets(frames_in_flight as u32);

    device
        .create_descriptor_pool(&pool_info, None)
        .expect("Failed to create a descriptor pool")
}

pub unsafe fn create_descriptor_set_layout(ctx: &Context) -> vk::DescriptorSetLayout {
    let transform_binding = Transform::binding();
    let sampler_binding = create_sampler_binding();
    let bindings = [transform_binding, sampler_binding];

    let layout_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

    let descriptor_set_layout = ctx
        .device
        .create_descriptor_set_layout(&layout_info, None)
        .expect("failed to create descriptor set layout");

    descriptor_set_layout
}

fn create_sampler_binding<'a>() -> vk::DescriptorSetLayoutBindingBuilder<'a> {
    vk::DescriptorSetLayoutBindingBuilder::new()
        .binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
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
                ctx,
                size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
        })
        .collect()
}

pub unsafe fn allocate_buffer(
    ctx: &Context,
    size: usize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_info = vk::BufferCreateInfoBuilder::new()
        .size(size as vk::DeviceSize)
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

pub unsafe fn create_vertex_buffer(
    ctx: &Context,
    vertices: &[Vertex],
    copy_queue_family: u32,
    copy_queue: vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let size = size_of_val(&vertices[0]) * vertices.len();
    let (staging_buf, staging_mem) = allocate_buffer(
        ctx,
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
        ctx,
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

pub unsafe fn find_memory_type(
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

    panic!("failed to find suitable memory type");
}

impl Transform {
    fn binding<'a>() -> vk::DescriptorSetLayoutBindingBuilder<'a> {
        vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
    }
}
