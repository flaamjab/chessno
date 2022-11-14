use std::fs::File;
use std::io::{BufReader, Result};
use std::{io::Read, path::Path};

#[cfg(target_os = "android")]
use ndk::asset::{Asset, AssetManager};

const ASSET_BASE_PATH: &str = "./assets";
const SHADERS_BASE_PATH: &str = "./shaders";

pub struct AssetLocator {}

impl AssetLocator {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(target_os = "android")]
    pub fn open(&self, path: &Path) -> Result<BufReader<Asset>> {
        use std::{ffi::CString, io, os::unix::prelude::OsStrExt};

        let asset_manager = ndk_glue::native_activity().asset_manager();
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        asset_manager
            .open(&path)
            .ok_or(io::ErrorKind::NotFound.into())
            .map(|a| BufReader::new(a))
    }

    #[cfg(not(target_os = "android"))]
    pub fn open(&self, path: &Path) -> Result<BufReader<File>> {
        let path = Path::new(ASSET_BASE_PATH).join(path);
        let f = File::open(path)?;
        Ok(BufReader::new(f))
    }
}
