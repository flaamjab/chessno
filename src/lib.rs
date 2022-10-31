use std::collections::HashSet;

use assets::Assets;
use gfx::texture::Texture;
use scene::{DynamicScene, Scenelike};
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

mod assets;
mod camera;
mod frame_counter;
mod gfx;
mod logging;
mod obj_loader;
mod object;
mod path_wrangler;
mod projection;
mod samples;
mod scene;
mod timer;
mod transform;

const TITLE: &str = "Chessno";

pub fn linux_main() {
    logging::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let mut assets = Assets::new();
    let mut renderer = Renderer::new(TITLE, &window);

    let mut timer = Timer::new();
    let mut pressed_keys = HashSet::new();

    let mut scene = PlaygroundScene::new(&mut assets);
    let textures: Vec<_> = assets.textures().collect();
    renderer.use_textures(&textures);

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

            scene.update(delta, &pressed_keys, &mut assets);

            renderer.draw(&mut scene, &mut assets);
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
