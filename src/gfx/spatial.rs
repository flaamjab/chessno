use cgmath::Matrix4;

#[repr(C)]
pub struct Spatial(pub Matrix4<f32>);
