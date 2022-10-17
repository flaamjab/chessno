use cgmath::prelude::*;
use cgmath::{Deg, Matrix4, Vector3, Vector4};

use crate::camera::{self, Camera};

#[derive(Debug)]
#[repr(C, align(16))]
pub struct Transform {
    pub mvp: Matrix4<f32>,
}

impl Transform {
    pub fn new(position: Vector3<f32>, rotation: Vector4<f32>, camera: &Camera) -> Self {
        // let pos = Matrix4::from_translation(position);
        // let rot = Matrix4::from_axis_angle(rotation.clone().truncate(), Deg(rotation.w));
        // let model = pos * rot;
        // let vp = camera.matrix();

        Self {
            mvp: camera.matrix(),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            mvp: Matrix4::identity(),
        }
    }
}
