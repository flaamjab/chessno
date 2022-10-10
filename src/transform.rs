use cgmath::prelude::*;
use cgmath::{Deg, Matrix4, Point3, Rad, Vector3};

const NEAR: f32 = 0.1;
const FAR: f32 = 100.0;

#[derive(Debug)]
#[repr(C, align(16))]
pub struct Transform {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
}

impl Transform {
    pub fn new_test(fov: f32, aspect: f32) -> Self {
        let projection = perspective(fov, aspect, NEAR, FAR);
        Transform {
            model: Matrix4::identity(),
            view: Matrix4::look_at_rh(
                Point3::from_value(2.0),
                Point3::from_value(0.0),
                Vector3::unit_z(),
            ),
            projection,
        }
    }

    pub fn with_ortho(&self, fov: f32, aspect: f32) -> Self {
        let projection = perspective(fov, aspect, NEAR, FAR);
        Transform {
            projection,
            ..*self
        }
    }

    pub fn with_model(&self, model: Matrix4<f32>) -> Self {
        Transform { model, ..*self }
    }

    pub fn with_view(&self, view: Matrix4<f32>) -> Self {
        Transform { view, ..*self }
    }
}

fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Matrix4<f32> {
    let fov: Rad<f32> = Deg(fov).into();
    let focal_length = 1.0 / (fov / 2.0).tan();

    let x = focal_length / aspect;
    let y = -focal_length;
    let a = near / (far - near);
    let b = far * a;

    let projection = Matrix4::new(
        x, 0.0, 0.0, 0.0, //
        0.0, y, 0.0, 0.0, //
        0.0, 0.0, a, b, //
        0.0, 0.0, -1.0, 0.0, //
    )
    .transpose();

    projection
}
