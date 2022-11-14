mod asset_locator;

use std::{collections::HashMap, path::Path};

use uuid::Uuid;

pub use crate::assets::asset_locator::AssetLocator;
use crate::rendering::material::Material;
use crate::rendering::shader::{Shader, ShaderStage};
use crate::rendering::{mesh::Mesh, texture::Texture};

type AssetId = u128;
pub type MeshId = AssetId;
pub type TextureId = AssetId;
pub type ShaderId = AssetId;
pub type MaterialId = AssetId;

fn new_uuid() -> AssetId {
    Uuid::new_v4().as_u128()
}

pub trait Asset {
    fn id(&self) -> AssetId;
}

pub const FALLBACK_TEXTURE: &str = "fallback_texture";
pub const DEFAULT_FRAG_SHADER: &str = "unlit_frag";
pub const DEFAULT_VERT_SHADER: &str = "unlit_vert";
pub const DEFAULT_MATERIAL: &str = "default_material";

pub struct Assets {
    asset_locator: AssetLocator,
    name_map: HashMap<String, AssetId>,
    textures: HashMap<TextureId, Texture>,
    meshes: HashMap<MeshId, Mesh>,
    shaders: HashMap<ShaderId, Shader>,
    materials: HashMap<MaterialId, Material>,
}

impl Assets {
    pub fn new() -> Self {
        let locator = AssetLocator::new();
        let fallback_texture_path = Path::new("textures/fallback.png");
        let mut fallback_texture = Texture::from_asset(&locator, fallback_texture_path)
            .expect("make sure fallback texture is present in the assets folder");
        fallback_texture.id = new_uuid();

        let mut unlit_vert_shader = Shader::from_asset(
            &locator,
            Path::new("shaders/unlit.vert"),
            ShaderStage::Vertex,
        )
        .unwrap();
        unlit_vert_shader.id = new_uuid();

        let mut unlit_frag_shader = Shader::from_asset(
            &locator,
            Path::new("shaders/unlit.frag"),
            ShaderStage::Fragment,
        )
        .unwrap();
        unlit_frag_shader.id = new_uuid();

        let default_material = Material {
            id: new_uuid(),
            fragment_shader_id: unlit_frag_shader.id,
            vertex_shader_id: unlit_vert_shader.id,
            texture_id: fallback_texture.id,
        };

        let name_map = HashMap::from_iter([
            (FALLBACK_TEXTURE.to_string(), fallback_texture.id),
            (DEFAULT_VERT_SHADER.to_string(), unlit_vert_shader.id),
            (DEFAULT_FRAG_SHADER.to_string(), unlit_frag_shader.id),
            (DEFAULT_MATERIAL.to_string(), default_material.id),
        ]);
        let textures = HashMap::from_iter([(fallback_texture.id, fallback_texture)]);
        let shaders = HashMap::from_iter([
            (unlit_vert_shader.id, unlit_vert_shader),
            (unlit_frag_shader.id, unlit_frag_shader),
        ]);
        let materials = HashMap::from_iter([(default_material.id, default_material)]);

        Self {
            asset_locator: locator,
            name_map,
            textures,
            meshes: HashMap::new(),
            shaders,
            materials,
        }
    }

    pub fn asset_locator(&self) -> &AssetLocator {
        &self.asset_locator
    }

    pub fn insert_texture(&mut self, name: &str, mut texture: Texture) -> TextureId {
        self.record_name(name, &texture);
        let texture_id = new_uuid();
        texture.id = texture_id;
        self.textures.insert(texture_id, texture);

        texture_id
    }

    pub fn insert_mesh(&mut self, name: &str, mut mesh: Mesh) -> MeshId {
        for submesh in &mut mesh.submeshes {
            submesh.id = new_uuid();
        }

        self.record_name(name, &mesh);
        let mesh_id = new_uuid();
        mesh.id = mesh_id;
        self.meshes.insert(mesh_id, mesh);

        mesh_id
    }

    pub fn insert_shader(&mut self, name: &str, mut shader: Shader) -> ShaderId {
        self.record_name(name, &shader);
        let shader_id = new_uuid();
        shader.id = shader_id;
        self.shaders.insert(shader_id, shader);

        shader_id
    }

    pub fn insert_material(&mut self, name: &str, mut material: Material) -> MaterialId {
        self.record_name(name, &material);
        let id = new_uuid();
        material.id = id;
        self.materials.insert(id, material);

        id
    }

    pub fn texture(&self, id: TextureId) -> Option<&Texture> {
        self.textures.get(&id)
    }

    pub fn mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(&id)
    }

    pub fn shader(&self, id: ShaderId) -> Option<&Shader> {
        self.shaders.get(&id)
    }

    pub fn material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
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

    pub fn shaders(&self) -> impl Iterator<Item = &Shader> {
        self.shaders.values()
    }

    fn record_name(&mut self, name: &str, asset: &impl Asset) {
        self.name_map.insert(name.to_string(), asset.id());
    }
}
