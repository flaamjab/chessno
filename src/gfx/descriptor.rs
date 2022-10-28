use std::mem::size_of;

use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;

use crate::gfx::context::Context;
use crate::gfx::texture::GpuResidentTexture;
use crate::transform::Transform;

pub unsafe fn global_textures_descriptor_set(
    device: &DeviceLoader,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    textures: &[&GpuResidentTexture],
) -> vk::DescriptorSet {
    debug_assert!(textures.len() <= 16 && !textures.is_empty());

    let layouts = [layout];
    let alloc_info = vk::DescriptorSetAllocateInfoBuilder::new()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    let set = device
        .allocate_descriptor_sets(&alloc_info)
        .expect("failed to allocate global descriptor set")[0];

    let image_infos: SmallVec<[vk::DescriptorImageInfoBuilder; 16]> = textures
        .iter()
        .map(|t| {
            vk::DescriptorImageInfoBuilder::new()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        })
        .collect();

    let combined_sampler_dw = vk::WriteDescriptorSetBuilder::new()
        .dst_set(set)
        .dst_binding(0)
        .dst_array_element(0)
        .image_info(&image_infos);

    device.update_descriptor_sets(&[combined_sampler_dw], &[]);

    set
}

pub unsafe fn create_spatial_descriptor_sets(
    device: &DeviceLoader,
    pool: vk::DescriptorPool,
    count: usize,
) {
}

pub unsafe fn create_descriptor_sets(
    device: &DeviceLoader,
    pool: vk::DescriptorPool,
    layouts: &[vk::DescriptorSetLayout],
    uniforms: &[(vk::Buffer, vk::DeviceMemory)],
    texture_view: vk::ImageView,
    sampler: vk::Sampler,
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
            .image_view(texture_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(sampler);

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

pub unsafe fn descriptor_set_layout_16_textures(
    device: &DeviceLoader,
    binding: u32,
) -> vk::DescriptorSetLayout {
    let sampler_binding = vk::DescriptorSetLayoutBindingBuilder::new()
        .binding(binding)
        .descriptor_count(16)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER);

    let bindings = [sampler_binding];
    let info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

    device
        .create_descriptor_set_layout(&info, None)
        .expect("failed to create descriptor set layout")
}

fn create_sampler_binding<'a>() -> vk::DescriptorSetLayoutBindingBuilder<'a> {
    vk::DescriptorSetLayoutBindingBuilder::new()
        .binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
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