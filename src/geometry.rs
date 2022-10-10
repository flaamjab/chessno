use cgmath::Vector3;

#[repr(C)]
pub struct Vertex {
    pub pos: Vector3<f32>,
    pub color: Vector3<f32>,
}

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
        let vertices = vec![
            Vertex {
                pos: (-1.0, -1.0, 1.0).into(),
                color: (1.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (1.0, -1.0, 1.0).into(),
                color: (1.0, 1.0, 1.0).into(),
            },
            Vertex {
                pos: (1.0, 1.0, 1.0).into(),
                color: (0.0, 0.0, 1.0).into(),
            },
            Vertex {
                pos: (-1.0, 1.0, 1.0).into(),
                color: (1.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (-1.0, -1.0, -1.0).into(),
                color: (0.0, 1.0, 1.0).into(),
            },
            Vertex {
                pos: (1.0, -1.0, -1.0).into(),
                color: (0.0, 0.0, 1.0).into(),
            },
            Vertex {
                pos: (1.0, 1.0, -1.0).into(),
                color: (1.0, 0.0, 0.0).into(),
            },
            Vertex {
                pos: (-1.0, 1.0, -1.0).into(),
                color: (0.0, 1.0, 0.0).into(),
            },
            Vertex {
                pos: (0.0, 0.0, 0.0).into(),
                color: (0.0, 0.0, 0.0).into(),
            },
        ];

        let indices = vec![
            0, 1, 2, 2, 3, 0, 1, 5, 6, 6, 2, 1, 7, 6, 5, 5, 4, 7, 4, 0, 3, 3,
            7, 4, 4, 5, 1, 1, 0, 4, 3, 2, 6, 6, 7, 3,
        ];

        Geometry { vertices, indices }
    }
}
