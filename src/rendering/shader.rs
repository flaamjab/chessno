use std::ffi::CString;
use std::io::{self, Read};
use std::path::Path;

use erupt::{utils::decode_spv, vk, DeviceLoader};

use crate::assets::{Asset, AssetLocator, ShaderId};
use crate::logging::debug;
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
    raw_code: Vec<u8>,
    stage: ShaderStage,
}

impl Shader {
    pub fn from_asset(
        asset_locator: &AssetLocator,
        path: &Path,
        stage: ShaderStage,
    ) -> io::Result<Self> {
        let mut extension = path.extension().unwrap().to_os_string();
        extension.push(".spv");
        let path = path.with_extension(extension);

        let mut reader = asset_locator.open(&path)?;
        let mut code = Vec::with_capacity(1024);
        reader.read_to_end(&mut code)?;
        debug!("Code length is {}", code.len());

        Ok(Self {
            id: 0,
            raw_code: code,
            stage,
        })
    }

    pub unsafe fn initialize(&self, device: &DeviceLoader) -> io::Result<InitializedShader> {
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

    fn compile(&self) -> io::Result<Vec<u32>> {
        decode_spv(&self.raw_code)
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
        assert!(matches!(compilation_result, Err(_)));
    }
}
