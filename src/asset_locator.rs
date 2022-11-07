use std::fs::File;
use std::io::{BufReader, Result};
use std::{io::Read, path::Path};

#[cfg(target_os = "android")]
use ndk::asset::{Asset, AssetManager};

const ASSET_BASE_PATH: &str = "./assets";

pub struct AssetLocator {}

impl AssetLocator {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(target_os = "android")]
    pub fn open(&self, path: &Path) -> Result<BufReader<Asset>> {}

    #[cfg(not(target_os = "android"))]
    pub fn open(&self, path: &Path) -> Result<BufReader<File>> {
        let path = Path::new(ASSET_BASE_PATH).join(path);
        let f = File::open(path)?;
        Ok(BufReader::new(f))
    }
}
