use nalgebra::{Point3, Rotation3, Unit, Vector3};
use winit::dpi::PhysicalPosition;
use winit::window::{CursorGrabMode, Window};

use crate::input_state::{Key, MouseButton, VirtualKeyCode};
use crate::logging::{debug, warn};
use crate::{camera::Camera, input_state::InputState};

pub struct FreeCameraControl {
    sensitivity: f32,
    move_speed: f32,

    camera: Camera,
    up: Unit<Vector3<f32>>,
    camera_pos: Point3<f32>,
    camera_dir: Unit<Vector3<f32>>,
    camera_right: Unit<Vector3<f32>>,

    cursor_grabbed: bool,
}

impl FreeCameraControl {
    pub fn new(camera: Camera, move_speed: f32, sensitivity: f32) -> Self {
        let up = Vector3::y_axis();
        let camera_dir = Unit::new_normalize(camera.direction);
        let camera_right = Unit::new_normalize(camera_dir.cross(&up));
        Self {
            sensitivity,
            move_speed,
            camera_pos: camera.position,
            camera_dir,
            camera_right,
            camera,
            up,
            cursor_grabbed: false,
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn update(&mut self, window: &Window, input_state: &InputState, time_delta: f32) {
        let mut camera_velocity = Vector3::zeros();
        let mut rot_left_right = Rotation3::identity();
        let mut rot_up_down = Rotation3::identity();

        if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::W)) {
            camera_velocity += self.camera_dir.as_ref();
        }

        if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::A)) {
            camera_velocity -= self.camera_right.as_ref();
        }

        if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::S)) {
            camera_velocity -= self.camera_dir.as_ref();
        }

        if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::D)) {
            camera_velocity += self.camera_right.as_ref();
        }
        camera_velocity *= self.move_speed * time_delta;

        let mouse_offset = input_state.mouse_offset();
        let lmb = Key::MouseButton(MouseButton::Left);
        let lmb_held = input_state.is_held(lmb);

        if input_state.is_pressed(lmb) {
            self.grab_cursor(window);
        }

        if lmb_held {
            let modifier = time_delta;
            rot_left_right =
                Rotation3::from_axis_angle(&self.up, -(mouse_offset.0 * modifier).to_radians());
            rot_up_down = Rotation3::from_axis_angle(
                &self.camera_right,
                -(mouse_offset.1 * modifier).to_radians(),
            );
        }

        if input_state.is_released(lmb) {
            self.release_cursor(window);
        }

        if !lmb_held {
            let look_offset = (self.sensitivity * time_delta).to_radians();
            if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::Up)) {
                rot_up_down = Rotation3::from_axis_angle(&self.camera_right, look_offset);
            }

            if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::Down)) {
                rot_up_down = Rotation3::from_axis_angle(&self.camera_right, -look_offset);
            }

            if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::Left)) {
                rot_left_right = Rotation3::from_axis_angle(&self.up, look_offset);
            }

            if input_state.is_held(Key::KeyboardKey(VirtualKeyCode::Right)) {
                rot_left_right = Rotation3::from_axis_angle(&self.up, -look_offset);
            }
        }

        self.camera_pos = self.camera_pos + camera_velocity;
        self.camera_dir = rot_left_right * rot_up_down * self.camera_dir;
        self.camera_right = Unit::new_normalize(self.camera_dir.cross(&self.up));

        self.camera.position = self.camera_pos;
        self.camera.direction = *self.camera_dir.as_ref();
    }

    fn grab_cursor(&mut self, window: &Window) {
        debug!("Grab cursor called");
        if !self.cursor_grabbed {
            self.cursor_grabbed = true;
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .unwrap_or_else(|e| warn!("failed to set cursor grab mode to locked: {:?}", e));

            let center = self.window_center(window);
            window
                .set_cursor_position(center)
                .unwrap_or_else(|e| warn!("failed to set cursor position: {:?}", e));

            window.set_cursor_visible(false);
        }
    }

    fn release_cursor(&mut self, window: &Window) {
        if self.cursor_grabbed {
            window.set_cursor_grab(CursorGrabMode::None).expect(
                "you happen to be on a platform that does not support
            cursor grab modes",
            );
            self.cursor_grabbed = false;
            window.set_cursor_visible(true);
        }
    }

    fn window_center(&self, window: &Window) -> PhysicalPosition<u32> {
        let size = window.inner_size();
        PhysicalPosition::new(size.width / 2, size.height / 2)
    }
}
