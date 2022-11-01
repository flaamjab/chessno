use nalgebra::{Matrix4, Point3, Rotation3, Translation3, Unit, Vector4};

use crate::math::TOLERANCE_F32;

pub struct Transform {
    pub position: Point3<f32>,
    pub rotation: Vector4<f32>,
}

impl Transform {
    pub fn new(position: Point3<f32>, rotation: Vector4<f32>) -> Self {
        Self { position, rotation }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let translation: Translation3<f32> = self.position.into();

        let rotation;
        if self.rotation.w.abs() > TOLERANCE_F32 {
            rotation = Rotation3::from_axis_angle(
                &Unit::new_normalize(self.rotation.xyz()),
                self.rotation.w.to_radians(),
            );
        } else {
            rotation = Rotation3::identity();
        }

        translation.to_homogeneous() * rotation.to_homogeneous()
    }
}
