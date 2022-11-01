use std::collections::HashSet;

use crate::assets::{generate_id, Asset, AssetId, Assets};
use crate::gfx::geometry::Vertex;
use crate::gfx::memory::{IndexBuffer, VertexBuffer};

use super::resource::DeviceResource;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub id: AssetId,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub textures: HashSet<AssetId>,
    pub submeshes: Vec<Submesh>,
    pub bbox: BBox,
}

#[derive(Clone, Debug)]
pub struct Submesh {
    pub id: AssetId,
    pub texture_id: AssetId,
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Debug)]
pub struct GpuResidentMesh {
    pub texture_id: AssetId,
    pub vertex_buf: VertexBuffer,
    pub index_buf: IndexBuffer,
}

impl DeviceResource for GpuResidentMesh {
    fn destroy(&self, device: &erupt::DeviceLoader) {
        self.vertex_buf.destroy(device);
        self.index_buf.destroy(device);
    }
}

#[derive(Clone, Debug, Default)]
pub struct BBox {
    pub width: f32,
    pub length: f32,
    pub height: f32,
}

impl Asset for Mesh {
    fn id(&self) -> AssetId {
        self.id
    }
}

impl Mesh {
    pub fn new_plane(name: &str, texture_id: AssetId, assets: &mut Assets) -> AssetId {
        let vertices = [
            Vertex {
                pos: [-0.5, -0.5, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0; 3],
            },
            Vertex {
                pos: [-0.5, 0.5, 0.0],
                uv: [0.0, 1.0, 0.0],
                color: [0.0; 3],
            },
            Vertex {
                pos: [0.5, 0.5, 0.0],
                uv: [1.0, 1.0, 0.0],
                color: [0.0; 3],
            },
            Vertex {
                pos: [0.5, -0.5, 0.0],
                uv: [1.0, 0.0, 0.0],
                color: [0.0; 3],
            },
        ]
        .to_vec();

        let indices = [1, 2, 0, 2, 3, 0].to_vec();

        let bbox = BBox {
            width: 1.0,
            length: 1.0,
            height: 0.0,
        };

        let n_indices = indices.len();
        let mesh_id = generate_id();
        let mesh = Mesh {
            id: mesh_id,
            vertices,
            indices,
            bbox: bbox.clone(),
            textures: HashSet::new(),
            submeshes: vec![Submesh {
                id: generate_id(),
                start_index: 0,
                end_index: n_indices,
                texture_id,
            }],
        };
        assets.insert_mesh(name, mesh);

        mesh_id
    }
}
