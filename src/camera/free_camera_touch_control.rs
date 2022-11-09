use winit::window::Window;

use nalgebra::{Rotation3, Unit, Vector2, Vector3};

use crate::camera::camera_control::CameraControl;
use crate::camera::Camera;
use crate::input_state::{InputState, Touch};
use crate::logging::debug;
use crate::math::{Point2D, Rectangle};

pub struct FreeCameraTouchControl {
    camera: Camera,

    up: Unit<Vector3<f32>>,
    camera_right: Unit<Vector3<f32>>,

    move_speed: f32,
    sensitivity: f32,
    joystick_radius_scale: f32,
}

impl FreeCameraTouchControl {
    pub fn new(
        camera: Camera,
        move_speed: f32,
        sensitivity: f32,
        joystick_radius_scale: f32,
    ) -> Self {
        let up = Vector3::y_axis();
        let camera_right = Self::camera_right(&camera, &up);
        Self {
            camera,
            up,
            camera_right,
            move_speed,
            sensitivity,
            joystick_radius_scale,
        }
    }

    fn camera_right(camera: &Camera, up: &Unit<Vector3<f32>>) -> Unit<Vector3<f32>> {
        Unit::new_normalize(camera.direction.cross(&up))
    }

    fn movement(&self, rect: &Rectangle, touch: &Touch, time_delta: f32) -> Vector3<f32> {
        let mut camera_velocity = Vector3::zeros();

        let offset = self.offset(touch);
        let offset_len = self.rectangle_normalized(&offset, rect).norm();
        if offset_len > 0.01 {
            let norm_offset = offset.normalize();
            let x = norm_offset.x;
            let y = -norm_offset.y;

            camera_velocity += self.camera.direction * y;
            camera_velocity += self.camera_right.as_ref() * x;

            let force = self.force(offset_len);
            camera_velocity *= self.move_speed * force * time_delta;
        }

        camera_velocity
    }

    fn rotation(&self, rect: &Rectangle, touch: &Touch, time_delta: f32) -> Rotation3<f32> {
        let mut rot_vertical = Rotation3::default();
        let mut rot_horizontal = Rotation3::default();

        let offset = self.offset(touch);

        let offset_len = self.rectangle_normalized(&offset, rect).norm();
        if offset_len > 0.01 {
            let norm_offset = offset.normalize();
            let force = self.force(offset_len);
            let modifier = self.sensitivity * force * time_delta;

            rot_horizontal = Rotation3::from_axis_angle(
                &self.up,
                -(norm_offset.x as f32 * modifier).to_radians(),
            );

            rot_vertical = Rotation3::from_axis_angle(
                &self.camera_right,
                -(norm_offset.y as f32 * modifier).to_radians(),
            );
        }
        rot_horizontal * rot_vertical
    }

    fn offset(&self, touch: &Touch) -> Vector2<f32> {
        let x = touch.move_position.x - touch.start_position.x;
        let y = touch.move_position.y - touch.start_position.y;

        Vector2::new(x as f32, y as f32)
    }

    fn rectangle_normalized(&self, vec: &Vector2<f32>, rect: &Rectangle) -> Vector2<f32> {
        Vector2::new(vec.x / rect.width() as f32, vec.y / rect.height() as f32)
    }

    fn force(&self, rect_normalized_offset_len: f32) -> f32 {
        let force = (rect_normalized_offset_len / self.joystick_radius_scale).clamp(0.0, 1.0);
        force
    }

    fn window_split(&self, window: &Window) -> (Rectangle, Rectangle) {
        let size = window.inner_size();
        let hcenter = size.width as f64 / 2.0;
        let bottom = size.height as f64;
        let right = size.width as f64;

        let move_area = Rectangle::new(Point2D::new(hcenter, 0.0), Point2D::new(right, bottom));
        let rotate_area = Rectangle::new(Point2D::new(0.0, 0.0), Point2D::new(hcenter, bottom));

        (move_area, rotate_area)
    }
}

impl CameraControl for FreeCameraTouchControl {
    fn camera(&self) -> &Camera {
        &self.camera
    }

    fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    fn update(&mut self, window: &Window, input_state: &InputState, time_delta: f32) {
        let (move_area, rotate_area) = self.window_split(&window);
        input_state
            .touches()
            .find(|t| move_area.contains(t.move_position))
            .map(|t| {
                let m = self.movement(&move_area, &t, time_delta);
                self.camera.position += m;
            });

        input_state
            .touches()
            .find(|t| rotate_area.contains(t.move_position))
            .map(|t| {
                let r = self.rotation(&rotate_area, &t, time_delta);
                self.camera.direction = r * self.camera.direction;
                self.camera_right = Self::camera_right(&self.camera, &self.up);
            });
    }
}
