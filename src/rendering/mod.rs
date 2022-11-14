pub mod material;
pub mod mesh;
pub mod projection;
pub mod renderer;
pub mod shader;
mod spatial;
pub mod texture;
pub mod vertex;
mod vulkan;

pub enum PrimitiveType {
    Points,
    Lines,
    LineStrip,
    Triangles,
    TriangleStrip,
}
