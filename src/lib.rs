use std::collections::HashSet;

use scene::DynamicScene;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::gfx::renderer::Renderer;
use crate::logging::{debug, trace};
use crate::samples::PlaygroundScene;
use crate::timer::Timer;

mod camera;
mod frame_counter;
mod gfx;
mod logging;
mod mesh;
mod object;
mod samples;
mod scene;
mod timer;
mod transform;

const TITLE: &str = "Chessno";

pub fn linux_main() {
    logging::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let mut renderer = Renderer::new(TITLE, &window);

    let mut timer = Timer::new();
    let mut pressed_keys = HashSet::new();

    let mut scene = PlaygroundScene::new(aspect_ratio(window.inner_size()));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized(new_size) => {
                trace!("Window resized, notifying renderer");
                renderer.handle_resize(new_size);
            }
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                Some(code) => match input.state {
                    ElementState::Pressed => {
                        pressed_keys.insert(code);
                    }
                    ElementState::Released => {
                        pressed_keys.remove(&code);
                    }
                },
                None => {}
            },
            _ => {}
        },
        Event::MainEventsCleared => {
            let delta = timer.elapsed();
            timer.reset();

            scene.update(delta, &pressed_keys, aspect_ratio(window.inner_size()));

            renderer.draw(&scene);
        }
        _ => {}
    })
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

pub fn aspect_ratio(size: PhysicalSize<u32>) -> f32 {
    let PhysicalSize { width, height } = size;
    width as f32 / height as f32
}
