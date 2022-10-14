use winit::window::Window;

use crate::context::Context;
use crate::mesh::Mesh;
use crate::swapchain::Swapchain;

pub struct Renderer {
    context: Context,
    swapchain: Swapchain,
}

impl Renderer {
    pub fn new(app_name: &str, window: &Window) -> Self {
        let context = Context::new(window, app_name, "No Engine");
        Self { context }
    }

    pub fn draw(meshes: &[Mesh]) {}
}
