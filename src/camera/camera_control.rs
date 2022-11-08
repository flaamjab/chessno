use winit::window::Window;

use crate::camera::Camera;
use crate::input_state::InputState;

pub trait CameraControl {
    fn update(&mut self, window: &Window, input_state: &InputState, time_delta: f32);
    fn camera(&self) -> &Camera;
    fn camera_mut(&mut self) -> &mut Camera;
}
