use cgmath::{Vector2, Vector3};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub pos: Vector3<f32>,
    pub uv: Vector3<f32>,
}

#[derive(Debug)]
pub struct Geometry {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Geometry {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    pub fn new_cube() -> Geometry {
        let raw_vertices: [f32; 180] = [
            -0.5, -0.5, -0.5, 0.0, 0.0, 0.5, -0.5, -0.5, 1.0, 0.0, 0.5, 0.5, -0.5, 1.0, 1.0, 0.5,
            0.5, -0.5, 1.0, 1.0, -0.5, 0.5, -0.5, 0.0, 1.0, -0.5, -0.5, -0.5, 0.0, 0.0, -0.5, -0.5,
            0.5, 0.0, 0.0, 0.5, -0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, 0.5, 1.0,
            1.0, -0.5, 0.5, 0.5, 0.0, 1.0, -0.5, -0.5, 0.5, 0.0, 0.0, -0.5, 0.5, 0.5, 1.0, 0.0,
            -0.5, 0.5, -0.5, 1.0, 1.0, -0.5, -0.5, -0.5, 0.0, 1.0, -0.5, -0.5, -0.5, 0.0, 1.0,
            -0.5, -0.5, 0.5, 0.0, 0.0, -0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 0.0, 0.5, 0.5,
            -0.5, 1.0, 1.0, 0.5, -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, 0.5,
            0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 0.0, -0.5, -0.5, -0.5, 0.0, 1.0, 0.5, -0.5, -0.5, 1.0,
            1.0, 0.5, -0.5, 0.5, 1.0, 0.0, 0.5, -0.5, 0.5, 1.0, 0.0, -0.5, -0.5, 0.5, 0.0, 0.0,
            -0.5, -0.5, -0.5, 0.0, 1.0, -0.5, 0.5, -0.5, 0.0, 1.0, 0.5, 0.5, -0.5, 1.0, 1.0, 0.5,
            0.5, 0.5, 1.0, 0.0, 0.5, 0.5, 0.5, 1.0, 0.0, -0.5, 0.5, 0.5, 0.0, 0.0, -0.5, 0.5, -0.5,
            0.0, 1.0,
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

        Geometry { vertices, indices }
    }

    pub fn new_plane() -> Geometry {
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

        Geometry { vertices, indices }
    }

    pub fn new_triangle() -> Geometry {
        let vertices = vec![
            Vertex {
                pos: (-0.5, 0.0, 0.0).into(),
                uv: (0.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (0.5, 0.0, 0.0).into(),
                uv: (1.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (0.0, 0.5, 0.0).into(),
                uv: (0.0, 1.0, 0.0).into(),
            },
        ];

        let indices = vec![1, 2, 0];

        Geometry { vertices, indices }
    }
}
