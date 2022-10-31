use nalgebra::{Matrix4, Point3, Vector3};

use crate::projection::Projection;

pub struct Camera {
    position: Point3<f32>,
    direction: Vector3<f32>,
    projection: Projection,
}

impl Camera {
    pub fn new(position: &Point3<f32>, direction: &Vector3<f32>, projection: Projection) -> Self {
        Self {
            position: *position,
            direction: *direction,
            projection,
        }
    }

    pub fn set_position(&mut self, position: &Point3<f32>) {
        self.position = *position;
    }

    pub fn set_direction(&mut self, direction: &Vector3<f32>) {
        self.direction = *direction;
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let projection = self.projection.matrix();
        let view = Matrix4::look_at_rh(
            &self.position,
            &(self.position + self.direction),
            &Vector3::y(),
        );
        projection * view
    }

    pub fn set_viewport_dimensions(&mut self, width: f32, height: f32) {
        self.projection.set_viewport_dimensions(width, height);
    }
}
