use std::sync::Arc;

use crate::geometry::Vertex;
use crate::gpu_program::Shader;

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

pub fn new_cube(shader: &Arc<Shader>) -> Mesh {
    let raw_vertices: [f32; 180] = [
        -0.5, -0.5, -0.5, 0.0, 0.0, 0.5, -0.5, -0.5, 1.0, 0.0, 0.5, 0.5, -0.5, 1.0, 1.0, 0.5, 0.5,
        -0.5, 1.0, 1.0, -0.5, 0.5, -0.5, 0.0, 1.0, -0.5, -0.5, -0.5, 0.0, 0.0, -0.5, -0.5, 0.5,
        0.0, 0.0, 0.5, -0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, 0.5, 1.0, 1.0, -0.5,
        0.5, 0.5, 0.0, 1.0, -0.5, -0.5, 0.5, 0.0, 0.0, -0.5, 0.5, 0.5, 1.0, 0.0, -0.5, 0.5, -0.5,
        1.0, 1.0, -0.5, -0.5, -0.5, 0.0, 1.0, -0.5, -0.5, -0.5, 0.0, 1.0, -0.5, -0.5, 0.5, 0.0,
        0.0, -0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.5, -0.5, 1.0, 1.0, 0.5,
        -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, 0.5, 0.0, 0.0, 0.5, 0.5, 0.5,
        1.0, 0.0, -0.5, -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, -0.5, 1.0, 1.0, 0.5, -0.5, 0.5, 1.0, 0.0,
        0.5, -0.5, 0.5, 1.0, 0.0, -0.5, -0.5, 0.5, 0.0, 0.0, -0.5, -0.5, -0.5, 0.0, 1.0, -0.5, 0.5,
        -0.5, 0.0, 1.0, 0.5, 0.5, -0.5, 1.0, 1.0, 0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 0.0,
        -0.5, 0.5, 0.5, 0.0, 0.0, -0.5, 0.5, -0.5, 0.0, 1.0,
    ];

    let vertices: Vec<Vertex>;
    unsafe {
        let raw_vertices = raw_vertices.as_ptr();
        let vertex_ptr = raw_vertices as *const Vertex;
        let vertex_slice = std::slice::from_raw_parts(vertex_ptr, 36);
        vertices = vertex_slice.to_vec();
    }

    let indices = (0..(vertices.len() / 3) as u16)
        .flat_map(|ix| {
            let ix = ix * 3;
            [ix + 2, ix + 1, ix]
        })
        .collect();

    Mesh { vertices, indices }
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

    let indices = [1, 0, 2, 2, 0, 3].to_vec();

    Mesh { vertices, indices }
}
