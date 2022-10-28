use std::collections::HashSet;

use winit::event::VirtualKeyCode;

use crate::assets::Assets;
use crate::camera::Camera;
use crate::object::Object;

pub struct Scene {
    pub objects: Vec<Object>,
    pub cameras: Vec<Camera>,
    pub assets: Assets,
}

pub trait Scenelike {
    fn objects(&self) -> &[Object];
    fn cameras(&self) -> &[Camera];
    fn active_camera(&self) -> &Camera;
    fn assets(&self) -> &Assets;
    fn assets_mut(&mut self) -> &mut Assets;
}

pub trait DynamicScene {
    fn update(
        &mut self,
        time_delta: f32,
        pressed_keys: &HashSet<VirtualKeyCode>,
        aspect_ratio: f32,
    );
}
