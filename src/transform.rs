use nalgebra::{Matrix4, Rotation3, Translation3, Unit, Vector3, Vector4};

const TOLERANCE: f32 = 1e-4;

pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
}

impl Transform {
    pub fn new(position: Vector3<f32>, rotation: Vector4<f32>) -> Self {
        Self { position, rotation }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let translation: Translation3<f32> = self.position.into();

        let rotation;
        if self.rotation.w.abs() > TOLERANCE {
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
