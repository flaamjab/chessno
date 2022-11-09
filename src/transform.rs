use nalgebra::{Matrix4, Point3, Rotation3, Scale3, Translation3, Unit, Vector3, Vector4};

pub struct Transform {
    pub position: Point3<f32>,
    pub rotation: Vector4<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new(position: Point3<f32>, rotation: Vector4<f32>, scale: f32) -> Self {
        Self {
            position,
            rotation,
            scale: Vector3::from_element(scale),
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let scale = Scale3::from(self.scale);

        let translation: Translation3<f32> = self.position.into();

        let rotation;
        if self.rotation.w.abs() > f32::EPSILON {
            rotation = Rotation3::from_axis_angle(
                &Unit::new_normalize(self.rotation.xyz()),
                self.rotation.w.to_radians(),
            );
        } else {
            rotation = Rotation3::identity();
        }

        translation.to_homogeneous() * rotation.to_homogeneous() * scale.to_homogeneous()
    }
}
