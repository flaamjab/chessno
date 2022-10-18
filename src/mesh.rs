use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use obj::{ObjData, ObjError};

use crate::gfx::geometry::Vertex;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Mesh {
    pub fn from_file(path: &Path) -> Result<Mesh, Error> {
        let reader = BufReader::new(File::open(path)?);
        let obj = ObjData::load_buf(reader).expect("failed to load model");

        let mut indices = Vec::with_capacity(obj.position.len());
        for o in obj.objects {
            for g in o.groups {
                for p in g.polys {
                    for ix in p.0 {
                        indices.push(ix.0 as u16);
                    }
                }
            }
        }

        let vertices = obj
            .position
            .iter()
            .zip(obj.texture)
            .map(|(p, t)| Vertex {
                pos: (p[0], p[1], p[2]).into(),
                uv: (t[0], t[1], 0.0).into(),
            })
            .collect();

        Ok(Mesh { vertices, indices })
    }

    pub fn new_plane() -> Mesh {
        let vertices = [
            Vertex {
                pos: (-0.5, -0.5, 0.0).into(),
                uv: (0.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (-0.5, 0.5, 0.0).into(),
                uv: (0.0, 1.0, 0.0).into(),
            },
            Vertex {
                pos: (0.5, 0.5, 0.0).into(),
                uv: (1.0, 1.0, 0.0).into(),
            },
            Vertex {
                pos: (0.5, -0.5, 0.0).into(),
                uv: (1.0, 0.0, 0.0).into(),
            },
        ]
        .to_vec();

        let indices = [1, 2, 0, 2, 3, 0].to_vec();

        Mesh { vertices, indices }
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
