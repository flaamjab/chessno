use crate::{assets::MeshId, rendering::PrimitiveType, transform::Transform};

pub struct Object {
    pub mesh_id: MeshId,
    pub primitive_type: PrimitiveType,
    pub transform: Transform,
}
