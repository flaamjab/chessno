use std::collections::HashSet;

use crate::assets::{Asset, MeshId, TextureId};
use crate::rendering::vertex::Vertex;
use crate::rendering::vulkan::memory::{IndexBuffer, VertexBuffer};
use crate::rendering::vulkan::resource::DeviceResource;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub id: MeshId,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub textures: HashSet<TextureId>,
    pub submeshes: Vec<Submesh>,
    pub bbox: BBox,
}

#[derive(Clone, Debug)]
pub struct Submesh {
    pub id: MeshId,
    pub texture_id: TextureId,
    pub start_index: usize,
    pub end_index: usize,
}

/// Submesh loaded to the GPU (current implementation uses one index and vertex buffer per submesh).
#[derive(Debug)]
pub struct LoadedSubmesh {
    pub id: MeshId,
    pub texture_id: TextureId,
    pub vertex_buf: VertexBuffer,
    pub index_buf: IndexBuffer,
}

impl DeviceResource for LoadedSubmesh {
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
    fn id(&self) -> MeshId {
        self.id
    }
}

impl Mesh {
    pub fn new_plane(texture_id: TextureId) -> Mesh {
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
        Mesh {
            id: 0,
            vertices,
            indices,
            bbox: bbox.clone(),
            textures: HashSet::new(),
            submeshes: vec![Submesh {
                id: 0,
                start_index: 0,
                end_index: n_indices,
                texture_id,
            }],
        }
    }
}
