use std::io::{self};
use std::path::Path;

use obj::ObjError;

use crate::gfx::geometry::Vertex;
use crate::gfx::texture::Texture;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<Submesh>,
    pub bbox: BBox,
}

#[derive(Clone, Debug)]
pub struct Submesh {
    pub texture: Texture,
    pub start_index: usize,
    pub end_index: usize,
    pub bbox: BBox,
}

#[derive(Clone, Debug, Default)]
pub struct BBox {
    pub width: f32,
    pub length: f32,
    pub height: f32,
}

impl Mesh {
    pub fn new_plane() -> Mesh {
        let vertices = [
            Vertex {
                pos: [-0.5, -0.5, 0.0],
                uv: [0.0, 0.0, 0.0],
            },
            Vertex {
                pos: [-0.5, 0.5, 0.0],
                uv: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [0.5, 0.5, 0.0],
                uv: [1.0, 1.0, 0.0],
            },
            Vertex {
                pos: [0.5, -0.5, 0.0],
                uv: [1.0, 0.0, 0.0],
            },
        ]
        .to_vec();

        let indices = [1, 2, 0, 2, 3, 0].to_vec();

        let bbox = BBox {
            width: 1.0,
            length: 1.0,
            height: 0.0,
        };

        let texture = Texture::from_file(Path::new("assets/textures/missing.png"))
            .expect("failed to load missing texture");
        let n_indices = indices.len();
        Mesh {
            vertices,
            indices,
            bbox: bbox.clone(),
            submeshes: vec![Submesh {
                bbox,
                start_index: 0,
                end_index: n_indices,
                texture,
            }],
        }
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
