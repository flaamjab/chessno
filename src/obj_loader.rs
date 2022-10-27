use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use obj::{Obj, ObjData, ObjMaterial};

use crate::gfx::{geometry::Vertex, texture::Texture};
use crate::logging::debug;
use crate::mesh::{BBox, Mesh, Submesh};

pub struct ObjLoader {}

impl ObjLoader {
    pub fn new() -> Self {
        Self {}
    }

    pub fn load_from_file(&self, path: &Path) -> Mesh {
        let mut obj = Obj::load(path).expect("failed to load OBJ file");
        obj.load_mtls()
            .expect("failed to load one or more MTL files");
        let vertices = self.load_vertices(&obj.data);
        let indices = self.load_indices(&obj.data);
        let submeshes = self.load_groups(&obj);

        Mesh {
            vertices,
            indices,
            submeshes,
            bbox: BBox::default(),
        }
    }

    fn load_vertices(&self, obj: &ObjData) -> Vec<Vertex> {
        let mut vertices: Vec<_> = obj
            .position
            .iter()
            .map(|p| Vertex {
                pos: [p[0], p[1], p[2]],
                uv: [0.0; 3],
            })
            .collect();

        for (v, t) in vertices.iter_mut().zip(&obj.texture) {
            v.uv = [t[0], t[1], 0.0];
        }

        vertices
    }

    fn load_indices(&self, obj: &ObjData) -> Vec<u16> {
        let mut indices = Vec::with_capacity(obj.position.len());
        for o in &obj.objects {
            for g in &o.groups {
                for p in &g.polys {
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
        indices
    }

    fn load_groups(&self, obj: &Obj) -> Vec<Submesh> {
        let mut submeshes = Vec::new();
        let mut submesh_start_vertex;
        let mut submesh_end_vertex: usize = 0;
        let missing_texture = Texture::from_file(Path::new("assets/textures/missing.png"))
            .expect("failed to load missing texture");
        for o in &obj.data.objects {
            for g in &o.groups {
                submesh_start_vertex = submesh_end_vertex;
                let texture = match &g.material {
                    Some(ObjMaterial::Mtl(m)) => {
                        if let Some(path) = &m.map_kd {
                            let path = path.replace("\\", "/");
                            let path = obj.path.join(path);

                            if path.exists() {
                                debug!("Loading texture at {:?}", path);
                                Texture::from_file(&path)
                                    .expect("failed to load texture at existing path")
                            } else {
                                debug!("File at {:?} is missing", path);
                                missing_texture.clone()
                            }
                        } else {
                            missing_texture.clone()
                        }
                    }
                    _ => missing_texture.clone(),
                };

                for _ in &g.polys {
                    submesh_end_vertex += 6;
                }

                submeshes.push(Submesh {
                    texture,
                    start_index: submesh_start_vertex,
                    end_index: submesh_end_vertex,
                    bbox: BBox::default(),
                })
            }
        }

        submeshes
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_model_loads_successfully() {
        let path = Path::new("assets/models/indoor plant_02.obj");
        let loader = ObjLoader::new();
        let _mesh = loader.load_from_file(path);
    }
}
