use std::ffi::CString;
use std::io::{self, Read};
use std::path::Path;

use erupt::{vk, DeviceLoader};
use spirv_compiler::{CompilerBuilder, CompilerError, ShaderKind};

use crate::assets::{Asset, AssetLocator, ShaderId};
use crate::rendering::vulkan::resource::DeviceResource;

#[derive(Clone, Copy)]
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
    pub id: ShaderId,
    raw_code: String,
    stage: ShaderStage,
}

impl Shader {
    pub fn from_asset(
        asset_locator: &AssetLocator,
        path: &Path,
        stage: ShaderStage,
    ) -> io::Result<Self> {
        let mut reader = asset_locator.open(&path)?;
        let mut code = String::with_capacity(1024);
        reader.read_to_string(&mut code)?;

        Ok(Self {
            id: 0,
            raw_code: code,
            stage,
        })
    }

    pub unsafe fn initialize(
        &self,
        device: &DeviceLoader,
    ) -> Result<InitializedShader, CompilerError> {
        let compiled_code = self.compile()?;
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&compiled_code);
        let module = device
            .create_shader_module(&module_info, None)
            .expect("failed to create shader module");

        Ok(InitializedShader {
            module,
            stage: self.stage.into(),
            entry_point: CString::new("main").unwrap(),
        })
    }

    fn compile(&self) -> Result<Vec<u32>, CompilerError> {
        let mut compiler = CompilerBuilder::new().build().unwrap();
        let kind = match self.stage {
            ShaderStage::Fragment => ShaderKind::Fragment,
            ShaderStage::Vertex => ShaderKind::Vertex,
        };
        compiler.compile_from_string(&self.raw_code, kind)
    }
}

impl Asset for Shader {
    fn id(&self) -> ShaderId {
        self.id
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::assets::Assets;

    #[test]
    fn test_shader_compiles() {
        let assets = Assets::new();
        let shader = Shader::from_asset(
            assets.asset_locator(),
            Path::new("shaders/unlit.vert"),
            ShaderStage::Vertex,
        )
        .unwrap();
        shader.compile().unwrap();
    }

    #[test]
    fn test_bad_shader_errors() {
        let assets = Assets::new();
        let shader = Shader::from_asset(
            assets.asset_locator(),
            Path::new("shaders/error.vert"),
            ShaderStage::Vertex,
        )
        .unwrap();

        let compilation_result = shader.compile();
        if let Err(e) = &compilation_result {
            eprintln!("{e}");
        }
        assert!(matches!(compilation_result, Err(CompilerError::Log(_))));
    }
}
