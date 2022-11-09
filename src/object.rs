use crate::{assets::MeshId, transform::Transform};

pub struct Object {
    pub mesh_id: MeshId,
    pub transform: Transform,
}
