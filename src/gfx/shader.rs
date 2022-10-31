use std::collections::hash_map::Entry;
use std::ffi::CString;
use std::{collections::HashMap, hash::Hash};

use erupt::{utils, vk, DeviceLoader};
use nalgebra::Matrix4;

use crate::gfx::vulkan_resource::DeviceResource;

pub enum ShaderStage {
    Vertex,
    Fragment,
}

impl Into<vk::ShaderStageFlagBits> for ShaderStage {
    fn into(self) -> vk::ShaderStageFlagBits {
        match self {
            ShaderStage::Vertex => vk::ShaderStageFlagBits::VERTEX,
            ShaderStage::Fragment => vk::ShaderStageFlagBits::FRAGMENT,
        }
    }
}

pub struct Shader {
    code: Vec<u8>,
    stage: ShaderStage,
}

impl Shader {
    pub fn new(code: &[u8], stage: ShaderStage) -> Self {
        Self {
            code: code.to_vec(),
            stage: stage,
        }
    }

    pub unsafe fn into_initialized(self, device: &DeviceLoader) -> InitializedShader {
        let shader_decoded = utils::decode_spv(&self.code).expect("failed to decode shader");
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&shader_decoded);
        let module = device
            .create_shader_module(&module_info, None)
            .expect("failed to create shader module");

        InitializedShader {
            module: module,
            stage: self.stage.into(),
            entry_point: CString::new("main").unwrap(),
        }
    }
}

pub struct InitializedShader {
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlagBits,
    entry_point: CString,
}

impl InitializedShader {
    pub fn stage_info(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        assert!(!self.module.is_null());
        vk::PipelineShaderStageCreateInfoBuilder::new()
            .stage(self.stage)
            .module(self.module)
            .name(&self.entry_point)
    }
}

impl DeviceResource for InitializedShader {
    fn destroy(&self, device: &DeviceLoader) {
        unsafe { device.destroy_shader_module(self.module, None) }
    }
}
