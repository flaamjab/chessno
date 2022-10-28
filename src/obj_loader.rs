use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use obj::{Obj, ObjMaterial};

use crate::assets::{generate_id, Asset, AssetId, Assets, MISSING_TEXTURE};
use crate::gfx::mesh::{BBox, Mesh, Submesh};
use crate::gfx::{geometry::Vertex, texture::Texture};
use crate::logging::debug;
use crate::path_wrangler::PathWrangler;

pub struct ObjLoader {}

impl ObjLoader {
    pub fn new() -> Self {
        Self {}
    }

    pub fn load_from_file(&self, path: &Path, name: &str, assets: &mut Assets) -> AssetId {
        let mut obj = Obj::load(path).expect("failed to load OBJ file");
        obj.load_mtls()
            .expect("failed to load one or more MTL files");
        let vertices = self.load_vertices(&obj);
        let indices = self.load_indices(&obj);
        let textures = self.load_textures(&obj, assets);
        let submeshes = self.load_groups(&obj, assets);

        let mesh_id = generate_id();
        let mesh = Mesh {
            id: mesh_id,
            vertices,
            indices,
            textures,
            submeshes,
            bbox: BBox::default(),
        };
        assets.insert_mesh(name, mesh);

        mesh_id
    }

    fn load_vertices(&self, obj: &Obj) -> Vec<Vertex> {
        let mut vertices: Vec<_> = obj
            .data
            .position
            .iter()
            .map(|p| Vertex {
                pos: [p[0], p[1], p[2]],
                uv: [0.0; 3],
            })
            .collect();

        for (v, t) in vertices.iter_mut().zip(&obj.data.texture) {
            v.uv = [t[0], t[1], 0.0];
        }

        vertices
    }

    fn load_indices(&self, obj: &Obj) -> Vec<u16> {
        let mut indices = Vec::with_capacity(obj.data.position.len());
        for o in &obj.data.objects {
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

    fn load_textures(&self, obj: &Obj, assets: &mut Assets) -> HashSet<AssetId> {
        let mut textures = HashSet::new();
        let mtls = &obj.data.material_libs;
        for ml in mtls {
            for m in &ml.materials {
                if let Some(map_kd) = &m.map_kd {
                    let path = self.texture_path(&map_kd);
                    let path = obj.path.join(path);
                    let name = self.texture_name(&path);
                    if path.exists() && !assets.is_present(&name) {
                        debug!("Loading texture at {:?}", &path);
                        eprintln!("Loading texture at {:?}", &path);
                        let t = Texture::from_file(&path)
                            .expect("failed to load texture from existing file");
                        textures.insert(t.id());
                        assets.insert_texture(&name, t);
                    }
                }
            }
        }

        textures
    }

    fn load_groups(&self, obj: &Obj, assets: &Assets) -> Vec<Submesh> {
        let mut submeshes = Vec::with_capacity(1);
        let mut submesh_start_vertex;
        let mut submesh_end_vertex: usize = 0;
        let missing_texture_id = assets.id_of(MISSING_TEXTURE).unwrap();
        for o in &obj.data.objects {
            for g in &o.groups {
                submesh_start_vertex = submesh_end_vertex;
                let texture_id = match &g.material {
                    Some(ObjMaterial::Mtl(m)) => {
                        if let Some(map_kd) = &m.map_kd {
                            let path = self.texture_path(map_kd);
                            let name = self.texture_name(&path);
                            if let Some(id) = assets.id_of(&name) {
                                id
                            } else {
                                missing_texture_id
                            }
                        } else {
                            missing_texture_id
                        }
                    }
                    _ => missing_texture_id,
                };

                for _ in &g.polys {
                    submesh_end_vertex += 6;
                }

                submeshes.push(Submesh {
                    texture_id,
                    start_index: submesh_start_vertex,
                    end_index: submesh_end_vertex,
                    bbox: BBox::default(),
                })
            }
        }

        submeshes
    }

    fn texture_path(&self, path: &str) -> PathBuf {
        PathWrangler::new(&path).with_os_convention().finish()
    }

    fn texture_name<'a, 'b>(&'a self, path: &'b Path) -> Cow<'b, str> {
        path.file_stem().unwrap().to_string_lossy()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_model_loads_successfully() {
        let path = Path::new("assets/models/indoor plant_02.obj");
        let loader = ObjLoader::new();
        let mut assets = Assets::new();
        let _mesh = loader.load_from_file(path, "plant", &mut assets);
    }
}
