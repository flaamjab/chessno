use cgmath::Vector3;

use crate::mesh::Mesh;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub pos: Vector3<f32>,
    pub uv: Vector3<f32>,
}

#[derive(Debug)]
pub struct Geometry {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Geometry {
    pub fn new() -> Self {
        Geometry {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    pub fn push_mesh(&mut self, mesh: &Mesh) {
        let vertices = mesh.vertices.clone();
        self.vertices.extend(vertices);

        self.indices.extend(mesh.indices.iter());
    }
}
