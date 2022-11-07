use std::{collections::HashMap, path::Path};

use uuid::Uuid;

use crate::{
    asset_locator::AssetLocator,
    gfx::{mesh::Mesh, texture::Texture},
};

pub type AssetId = u128;

pub fn generate_id() -> AssetId {
    Uuid::new_v4().as_u128()
}

pub trait Asset {
    fn id(&self) -> AssetId;
}

pub const FALLBACK_TEXTURE: &str = "fallback_texture";

pub struct Assets {
    textures: HashMap<AssetId, Texture>,
    meshes: HashMap<AssetId, Mesh>,
    name_map: HashMap<String, AssetId>,
    asset_locator: AssetLocator,
}

impl Assets {
    pub fn new(asset_locator: AssetLocator) -> Self {
        let fallback_texture_path = Path::new("textures/missing.png");
        let mut texture_reader = asset_locator.open(&fallback_texture_path).unwrap();
        let fallback_texture =
            Texture::from_reader(&mut texture_reader).expect("failed to load missing texture");

        let name_map = HashMap::from_iter([(FALLBACK_TEXTURE.to_string(), fallback_texture.id())]);
        let textures = HashMap::from_iter([(fallback_texture.id(), fallback_texture)]);

        Self {
            meshes: HashMap::new(),
            textures,
            name_map,
            asset_locator,
        }
    }

    pub fn asset_locator(&self) -> &AssetLocator {
        &self.asset_locator
    }

    pub fn insert_texture(&mut self, name: &str, texture: Texture) {
        self.record_name(name, &texture);
        self.textures.insert(texture.id(), texture);
    }

    pub fn insert_mesh(&mut self, name: &str, mesh: Mesh) {
        self.record_name(name, &mesh);
        self.meshes.insert(mesh.id(), mesh);
    }

    pub fn get_texture_by_id(&self, id: AssetId) -> Option<&Texture> {
        self.textures.get(&id)
    }

    pub fn get_mesh_by_id(&self, id: AssetId) -> Option<&Mesh> {
        self.meshes.get(&id)
    }

    pub fn id_of(&self, name: &str) -> Option<AssetId> {
        self.name_map.get(name).map(|id| *id)
    }

    pub fn is_present(&self, name: &str) -> bool {
        self.name_map.contains_key(name)
    }

    pub fn textures(&self) -> impl Iterator<Item = &Texture> {
        self.textures.values()
    }

    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes.values()
    }

    fn record_name(&mut self, name: &str, asset: &impl Asset) {
        self.name_map.insert(name.to_string(), asset.id());
    }
}
