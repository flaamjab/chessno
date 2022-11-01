use crate::assets::Assets;
use crate::camera::Camera;
use crate::input_state::InputState;
use crate::object::Object;

pub trait Scene {
    fn objects(&self) -> &[Object];
    fn active_camera(&self) -> &Camera;
    fn active_camera_mut(&mut self) -> &mut Camera;
}

pub trait DynamicScene {
    fn update(&mut self, time_delta: f32, input_state: &InputState, assets: &mut Assets);
}
