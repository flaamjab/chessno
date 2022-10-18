use cgmath::{Vector3, Vector4};

use crate::{mesh::Mesh, transform::Transform};

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
}
