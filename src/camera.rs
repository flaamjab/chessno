use cgmath::{Angle, Deg, InnerSpace, Matrix, Matrix4, Rad, SquareMatrix, Vector3, Vector4, Zero};

const TOLERANCE: f32 = 1e-4;

pub struct Camera {
    position: Vector3<f32>,
    rotation: Vector4<f32>,
    projection: Matrix4<f32>,
}

impl Camera {
    pub fn new(
        position: &Vector3<f32>,
        rotation: &Vector4<f32>,
        projection: &Matrix4<f32>,
    ) -> Self {
        Self {
            position: *position,
            rotation: *rotation,
            projection: *projection,
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        // let mut result = Matrix4::identity();
        // if self.rotation.w < TOLERANCE {
        //     let axis = self.rotation.clone().truncate().normalize();
        //     let rot = Matrix4::from_axis_angle(axis, Deg(self.rotation.w));
        //     result = rot * result;
        // }

        // let translate = Matrix4::from_translation(self.position);
        // self.projection * translate * result

        let translate = Matrix4::from_translation(Vector3::new(0.0, 0.0, 2.0));
        self.projection * translate
    }
}

pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Matrix4<f32> {
    let fov: Rad<f32> = Deg(fov).into();
    let focal_length = 1.0 / (fov / 2.0).tan();

    let x = focal_length / aspect;
    let y = -focal_length;
    let a = near / (far - near);
    let b = far * a;

    Matrix4::new(
        x, 0.0, 0.0, 0.0, //
        0.0, y, 0.0, 0.0, //
        0.0, 0.0, a, b, //
        0.0, 0.0, -1.0, 0.0, //
    )
    .transpose()
}
