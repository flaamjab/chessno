use std::ffi::CString;

use erupt::{utils, vk, DeviceLoader};
use smallvec::SmallVec;
use thiserror::Error as Err;

use crate::logging::debug;

pub struct Shader {
    modules: SmallVec<[(vk::ShaderModule, vk::ShaderStageFlagBits); 8]>,
    entry_point: CString,
}

impl Shader {
    pub fn new(
        device: &DeviceLoader,
        programs: &[(&[u8], vk::ShaderStageFlagBits)],
    ) -> Result<Shader, Error> {
        let mut modules = SmallVec::with_capacity(programs.len());
        for (bytes, stage) in programs {
            let vert_decoded = utils::decode_spv(bytes).unwrap();
            let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&vert_decoded);
            unsafe {
                let module = device
                    .create_shader_module(&module_info, None)
                    .map_err(|e| Error::from_vulkan_result(e))?;
                modules.push((module, *stage));
            }
        }

        Ok(Self {
            entry_point: CString::new("main").unwrap(),
            modules,
        })
    }

    pub fn stage_infos(&self) -> SmallVec<[vk::PipelineShaderStageCreateInfoBuilder; 4]> {
        self.modules
            .iter()
            .map(move |sm| {
                vk::PipelineShaderStageCreateInfoBuilder::new()
                    .stage(sm.1)
                    .module(sm.0)
                    .name(&self.entry_point)
            })
            .collect()
    }

    pub unsafe fn destroy(&self, device: &DeviceLoader) {
        for (module, _) in &self.modules {
            device.destroy_shader_module(*module, None);
        }
    }
}
#[derive(Err, Debug)]
pub enum Error {
    #[error("compilation failed")]
    CompilationError,
    #[error("unknown error")]
    Unknown,
}

impl Error {
    fn from_vulkan_result(result: vk::Result) -> Self {
        match result {
            vk::Result::ERROR_INVALID_SHADER_NV => Error::CompilationError,
            e => {
                debug!("received {} after failing to compile shaders", e);
                Error::Unknown
            }
        }
    }
}
