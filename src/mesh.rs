use crate::geometry::Vertex;
use crate::shader::Shader;
use crate::transform::Transform;

pub struct Mesh {
    vertices: Vertex,
    indices: Vec<u16>,
    shader: Shader,
    transform: Transform,
}
