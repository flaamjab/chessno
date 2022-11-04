use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use log::{trace, warn};
use obj::{Group, Obj, ObjMaterial, SimplePolygon};
use smallvec::{smallvec, SmallVec};

use crate::assets::Asset;
use crate::assets::{generate_id, AssetId, Assets, MISSING_TEXTURE};
use crate::gfx::geometry::Vertex;
use crate::gfx::mesh::{BBox, Mesh, Submesh};
use crate::gfx::texture::Texture;
use crate::path_wrangler::PathWrangler;

pub struct ObjLoader<'a> {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    textures: HashSet<AssetId>,
    assets: &'a mut Assets,
}

struct IndexedVertex(usize, Vertex);

impl<'a> ObjLoader<'a> {
    pub fn new(assets: &'a mut Assets) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            textures: HashSet::new(),
            assets,
        }
    }

    pub fn load_from_file(&mut self, path: &Path, name: &str) -> AssetId {
        let mut obj = Obj::load(path).expect("failed to load OBJ file");

        let vertex_count = obj.data.position.len();
        self.vertices = vec![Vertex::zeroed(); vertex_count];
        self.indices = Vec::with_capacity(vertex_count);

        obj.load_mtls()
            .expect("failed to load one or more MTL files");

        let submeshes = self.assemble(&obj);

        let mesh_id = generate_id();
        let vertices = std::mem::replace(&mut self.vertices, Vec::new());
        let indices = std::mem::replace(&mut self.indices, Vec::new());
        let textures = std::mem::replace(&mut self.textures, HashSet::new());
        let mesh = Mesh {
            id: mesh_id,
            vertices,
            indices,
            textures,
            submeshes,
            bbox: BBox::default(),
        };

        self.assets.insert_mesh(name, mesh);

        mesh_id
    }

    fn assemble(&mut self, obj: &Obj) -> Vec<Submesh> {
        let mut submesh_start;
        let mut submesh_end = 0;
        let mut submeshes = Vec::with_capacity(1);
        let zero_vertex = Vertex::zeroed();
        for object in &obj.data.objects {
            for group in &object.groups {
                submesh_start = submesh_end;
                for poly in &group.polys {
                    let poly_indices = self.indices(&poly);
                    submesh_end += poly_indices.len();
                    let poly_vertices = self.vertices(obj, &poly);

                    // Update mesh vertices and indices
                    self.indices.extend(poly_indices.iter());
                    for iv in poly_vertices {
                        if self.vertices[iv.0] != zero_vertex {
                            trace!("Vertex with position index {} is already written", iv.0);
                        } else {
                            self.vertices[iv.0] = iv.1;
                        }
                    }
                }

                let texture_id = self.texture(obj, &group);
                self.textures.insert(texture_id);

                submeshes.push(Submesh {
                    id: generate_id(),
                    texture_id,
                    start_index: submesh_start,
                    end_index: submesh_end,
                });
            }
        }

        submeshes
    }

    fn texture(&mut self, obj: &Obj, group: &Group) -> AssetId {
        let missing_texture_id = self.assets.id_of(MISSING_TEXTURE).unwrap();
        if let Some(material) = &group.material {
            if let ObjMaterial::Mtl(material) = material {
                if let Some(diffuse) = &material.map_kd {
                    let path = self.texture_path(&obj.path, diffuse);
                    let name = self.texture_name(&path);
                    if let Some(texture_id) = self.assets.id_of(&name) {
                        texture_id
                    } else {
                        Texture::from_file(&path)
                            .map(|t| {
                                let id = t.id();
                                self.assets.insert_texture(&name, t);
                                id
                            })
                            .unwrap_or(missing_texture_id)
                    }
                } else {
                    missing_texture_id
                }
            } else {
                missing_texture_id
            }
        } else {
            missing_texture_id
        }
    }

    /// Creates vertices for a polygon, looking up actual data within `obj` by indices in `poly`.
    fn vertices(&self, obj: &Obj, poly: &SimplePolygon) -> SmallVec<[IndexedVertex; 4]> {
        let tuples = &poly.0;
        let positions = &obj.data.position;
        let uvs = &obj.data.texture;
        tuples
            .iter()
            .map(|t| {
                let uv = {
                    if let Some(uv_ix) = t.1 {
                        let uv = uvs[uv_ix];
                        [uv[0], uv[1], 0.0]
                    } else {
                        warn!("Missing UV index for vertex index {}", t.0);
                        [0.0; 3]
                    }
                };
                let pos = positions[t.0];

                let color = if tuples.len() > 3 {
                    [1.0, 0.0, 0.0]
                } else {
                    [1.0; 3]
                };

                IndexedVertex(t.0, Vertex { pos, uv, color })
            })
            .collect()
    }

    fn indices(&self, poly: &SimplePolygon) -> SmallVec<[u16; 8]> {
        let tuples = &poly.0;
        let indices: SmallVec<[usize; 8]> = match tuples.len() {
            3 => {
                let (a, b, c) = (tuples[0], tuples[1], tuples[2]);
                smallvec![a.0, b.0, c.0]
            }
            4 => {
                let (a, b, c, d) = (tuples[0], tuples[1], tuples[2], tuples[3]);
                smallvec![a.0, b.0, c.0, a.0, c.0, d.0]
            }
            _ => panic!("unsupported polygon vertex count ({})", tuples.len()),
        };

        indices.into_iter().map(|ix| ix as u16).collect()
    }

    fn texture_path(&self, base_path: &Path, name: &str) -> PathBuf {
        let path = PathWrangler::new(&name).with_os_convention().finish();
        base_path.join(path)
    }

    fn texture_name<'b, 'c>(&'b self, path: &'c Path) -> Cow<'c, str> {
        path.file_stem().unwrap().to_string_lossy()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_model_loads_successfully() {
        let path = Path::new("assets/models/indoor plant_02.obj");
        let mut assets = Assets::new();
        let mut loader = ObjLoader::new(&mut assets);
        let _mesh = loader.load_from_file(path, "plant");
    }
}
