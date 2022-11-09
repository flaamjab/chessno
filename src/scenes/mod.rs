mod playground;

use winit::window::Window;

pub use playground::PlaygroundScene;

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
    fn update(
        &mut self,
        window: &Window,
        input_state: &InputState,
        time_delta: f32,
        assets: &mut Assets,
    );
}
