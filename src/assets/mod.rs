mod asset_locator;

pub use asset_locator::AssetLocator;

use std::{collections::HashMap, path::Path};

use uuid::Uuid;

use crate::rendering::{mesh::Mesh, texture::Texture};

type AssetId = u128;
pub type MeshId = AssetId;
pub type TextureId = AssetId;

fn generate_id() -> AssetId {
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
    pub fn new() -> Self {
        let asset_locator = AssetLocator::new();
        let fallback_texture_path = Path::new("textures/fallback.png");
        let fallback_texture = Texture::from_asset(&asset_locator, fallback_texture_path)
            .expect("make sure fallback texture is present in the assets folder");

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

    pub fn insert_texture(&mut self, name: &str, mut texture: Texture) -> TextureId {
        self.record_name(name, &texture);
        let texture_id = generate_id();
        texture.id = texture_id;
        self.textures.insert(texture_id, texture);

        texture_id
    }

    pub fn insert_mesh(&mut self, name: &str, mut mesh: Mesh) -> MeshId {
        for submesh in &mut mesh.submeshes {
            submesh.id = generate_id();
        }

        self.record_name(name, &mesh);
        let mesh_id = generate_id();
        mesh.id = mesh_id;
        self.meshes.insert(mesh_id, mesh);

        mesh_id
    }

    pub fn texture(&self, id: AssetId) -> Option<&Texture> {
        self.textures.get(&id)
    }

    pub fn mesh(&self, id: AssetId) -> Option<&Mesh> {
        self.meshes.get(&id)
    }

    pub fn id_of(&self, name: &str) -> Option<AssetId> {
        self.name_map.get(name).map(|id| *id)
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
