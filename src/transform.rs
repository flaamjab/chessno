use cgmath::prelude::*;
use cgmath::{Deg, Matrix4, Vector3, Vector4};

pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
}

impl Transform {
    pub fn new(position: Vector3<f32>, rotation: Vector4<f32>) -> Self {
        Self { position, rotation }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(self.position);
        let rotation = Matrix4::from_axis_angle(
            self.rotation.clone().truncate().normalize(),
            Deg(self.rotation.w),
        );

        translation * rotation
    }
}
