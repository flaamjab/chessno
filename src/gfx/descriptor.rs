use std::collections::HashMap;
use std::mem::size_of;

use erupt::{vk, DeviceLoader};
use smallvec::{smallvec, SmallVec};

use crate::assets::AssetId;
use crate::gfx::texture::GpuResidentTexture;
use crate::transform::Transform;

pub unsafe fn texture_descriptor_sets(
    device: &DeviceLoader,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    binding: u32,
    textures: &[&GpuResidentTexture],
    sampler: vk::Sampler,
) -> HashMap<AssetId, vk::DescriptorSet> {
    debug_assert!(textures.len() <= 8);

    let layouts: SmallVec<[vk::DescriptorSetLayout; 16]> = smallvec![layout; textures.len()];
    let alloc_info = vk::DescriptorSetAllocateInfoBuilder::new()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    let sets = device
        .allocate_descriptor_sets(&alloc_info)
        .expect("failed to allocate global descriptor set");

    let image_infos: SmallVec<[_; 8]> = textures
        .iter()
        .map(|t| {
            vk::DescriptorImageInfoBuilder::new()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(t.image_view)
                .sampler(sampler)
        })
        .map(|image_info| {
            let v: SmallVec<[vk::DescriptorImageInfoBuilder; 1]> = smallvec![image_info];
            v
        })
        .collect();

    let descriptor_writes: SmallVec<[vk::WriteDescriptorSetBuilder; 8]> = sets
        .iter()
        .zip(image_infos.iter())
        .map(|(&set, image_infos)| {
            vk::WriteDescriptorSetBuilder::new()
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_set(set)
                .dst_binding(binding)
                .dst_array_element(0)
                .image_info(&image_infos)
        })
        .collect();

    device.update_descriptor_sets(&descriptor_writes, &[]);

    textures
        .iter()
        .zip(sets.iter())
        .map(|(t, s)| (t.id(), *s))
        .collect()
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
            .descriptor_count(16),
    ];
    let pool_info = vk::DescriptorPoolCreateInfoBuilder::new()
        .pool_sizes(&pool_sizes)
        .max_sets(16);

    device
        .create_descriptor_pool(&pool_info, None)
        .expect("Failed to create a descriptor pool")
}

pub unsafe fn descriptor_set_layout_1_texture(
    device: &DeviceLoader,
    binding: u32,
) -> vk::DescriptorSetLayout {
    let sampler_binding = vk::DescriptorSetLayoutBindingBuilder::new()
        .binding(binding)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = [sampler_binding];
    let info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

    device
        .create_descriptor_set_layout(&info, None)
        .expect("failed to create a descriptor set layout")
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
