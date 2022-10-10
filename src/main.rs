use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::Window,
};

use crate::context::Context;
use crate::logging::debug;

mod context;
#[cfg(debug_assertions)]
mod validation;
// mod erupt;
mod geometry;
mod logging;
mod mesh;
mod renderer;
mod shader;
mod sync_pool;
mod transform;

fn main() {
    logging::init();

    // unsafe { crate::erupt::init() }

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    unsafe {
        let _context = Context::new(&window, "Main", "No Engine");
    }

    event_loop.run(process_android_events);
}

fn process_android_events(
    event: Event<()>,
    window_target: &EventLoopWindowTarget<()>,
    control_flow: &mut ControlFlow,
) {
    match event {
        Event::Resumed => {
            debug!("Resumed");
        }
        Event::Suspended => {
            debug!("Suspended")
        }
        _ => {}
    }
}
