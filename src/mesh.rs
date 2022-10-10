use crate::geometry::Vertex;
use crate::shader::Shader;

pub struct Mesh {
    vertices: Vertex,
    indices: Vec<u16>,
    shader: Shader,
}
