use std::collections::HashSet;
use std::io;

use obj::ObjError;

use crate::assets::{generate_id, Asset, AssetId, Assets};
use crate::gfx::geometry::Vertex;

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
    pub texture_id: AssetId,
    pub start_index: usize,
    pub end_index: usize,
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
                start_index: 0,
                end_index: n_indices,
                texture_id,
            }],
        };
        assets.insert_mesh(name, mesh);

        mesh_id
    }
}

#[derive(Debug)]
pub enum Error {
    IOError(io::Error),
    ObjError(obj::ObjError),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<ObjError> for Error {
    fn from(e: ObjError) -> Self {
        Error::ObjError(e)
    }
}
