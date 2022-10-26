use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use obj::{ObjData, ObjError};

use crate::gfx::geometry::Vertex;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub bbox: BBox,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct BBox {
    pub width: f32,
    pub length: f32,
    pub height: f32,
}

impl Mesh {
    pub fn from_file(path: &Path) -> Result<Mesh, Error> {
        let reader = BufReader::new(File::open(path)?);
        let obj = ObjData::load_buf(reader)?;

        let mut indices = Vec::with_capacity(obj.position.len());
        for o in obj.objects {
            for g in o.groups {
                for p in g.polys {
                    if p.0.len() == 4 {
                        indices.extend(
                            [p.0[0], p.0[1], p.0[2], p.0[0], p.0[2], p.0[3]].map(|t| t.0 as u16),
                        );
                    } else {
                        panic!("unsupported OBJ face format")
                    }
                }
            }
        }

        let mut vertices: Vec<_> = obj
            .position
            .iter()
            .map(|p| Vertex {
                pos: [p[0], p[1], p[2]],
                uv: [0.0; 3],
            })
            .collect();

        for (v, t) in vertices.iter_mut().zip(obj.texture) {
            v.uv = [t[0], t[1], 0.0];
        }

        Ok(Mesh {
            vertices,
            indices,
            bbox: BBox {
                width: 0.0,
                length: 0.0,
                height: 0.0,
            },
        })
    }

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

        Mesh {
            vertices,
            indices,
            bbox: BBox {
                width: 1.0,
                length: 1.0,
                height: 0.0,
            },
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_model_loads_successfully() {
        let path = Path::new("assets/models/Clock_obj.obj");
        let mesh = Mesh::from_file(path).expect("failed to load model");

        println!("{:?}", mesh)
    }
}
