use nalgebra::{Matrix4, Perspective3, Point3, Vector3};

pub struct Camera {
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    pub projection: Matrix4<f32>,
}

impl Camera {
    pub fn new(
        position: &Point3<f32>,
        direction: &Vector3<f32>,
        projection: &Matrix4<f32>,
    ) -> Self {
        Self {
            position: *position,
            direction: *direction,
            projection: *projection,
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(
            &self.position,
            &(self.position + self.direction),
            &Vector3::y(),
        );
        self.projection * view
    }

    pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Matrix4<f32> {
        let mut p = Perspective3::new(aspect, fov_deg.to_radians(), near, far).to_homogeneous();
        p.m22 *= -1.0;

        p
    }
}
