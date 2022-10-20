use nalgebra::{Matrix4, Point3, Vector3};

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
            &-Vector3::y(),
        );
        self.projection * view
    }

    pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Matrix4<f32> {
        let fov_rad = fov_deg.to_radians();
        let focal_length = 1.0 / (fov_rad / 2.0).tan();

        let x = focal_length / aspect;
        let y = focal_length;
        let a = near / (far - near);
        let b = far * a;

        Matrix4::new(
            x, 0.0, 0.0, 0.0, //
            0.0, y, 0.0, 0.0, //
            0.0, 0.0, a, b, //
            0.0, 0.0, -1.0, 0.0, //
        )
    }
}
