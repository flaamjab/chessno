use winit::{event::Event, event_loop::EventLoop, window::Window};

use crate::context::Context;
use crate::logging::debug;

mod context;
mod erupt;
mod geometry;
mod logging;
mod mesh;
mod physical_device;
// mod renderer;
mod shader;
mod swapchain;
mod sync_pool;
mod transform;
#[cfg(debug_assertions)]
mod validation;

pub fn desktop_main() {
    logging::init();

    unsafe { crate::erupt::init() }

    // let event_loop = EventLoop::new();
    // let window = Window::new(&event_loop).unwrap();
    // unsafe {
    //     let _context = Context::new(&window, "Main", "No Engine");
    // }
}

#[cfg_attr(target_os = "android", ndk_glue::main(logger(level = "debug")))]
pub fn android_main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    unsafe {
        event_loop.run(move |event, _, _control_flow| match event {
            Event::Resumed => {
                debug!("Resumed");
            }
            Event::Suspended => {
                debug!("Suspended")
            }
            _ => {}
        })
    }
}
