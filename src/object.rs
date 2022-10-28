use crate::{assets::AssetId, gfx::mesh::Mesh, transform::Transform};

pub struct Object {
    pub mesh_id: AssetId,
    pub transform: Transform,
}
