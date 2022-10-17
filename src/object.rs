use cgmath::{Vector3, Vector4};

use crate::mesh::Mesh;

pub struct Object {
    pub mesh: Mesh,
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
}
