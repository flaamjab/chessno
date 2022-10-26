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

#[derive(Clone, Default)]
pub struct ShaderParams {
    matrices4f32: HashMap<String, Matrix4<f32>>,
}

impl ShaderParams {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_matrixf32(&mut self, name: &str, value: &Matrix4<f32>) {
        self.matrices4f32.insert(name.to_string(), value.clone());
    }

    pub fn matrixf32(&self, name: &str) -> Matrix4<f32> {
        let mut maybe_matrix = self.matrices4f32.get(name);
        maybe_matrix.get_or_insert(&Matrix4::zeros()).clone()
    }
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
            params: Default::default(),
        }
    }
}

pub struct InitializedShader {
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlagBits,
    entry_point: CString,
    params: ShaderParams,
}

impl InitializedShader {
    pub fn stage_info(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        assert!(!self.module.is_null());
        vk::PipelineShaderStageCreateInfoBuilder::new()
            .stage(self.stage)
            .module(self.module)
            .name(&self.entry_point)
    }

    pub fn params(&self) -> &ShaderParams {
        &self.params
    }

    pub fn set_params(&mut self, params: &ShaderParams) {
        self.params = params.clone();
    }
}

impl DeviceResource for InitializedShader {
    fn destroy(&self, device: &DeviceLoader) {
        unsafe { device.destroy_shader_module(self.module, None) }
    }
}
