use winit::{
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::assets::Assets;
use crate::input_state::InputState;
use crate::logging::debug;
use crate::samples::PlaygroundScene;
use crate::scene::DynamicScene;
use crate::timer::Timer;
use crate::{gfx::renderer::Renderer, input_state::Key};

mod assets;
mod camera;
mod frame_counter;
mod free_camera_control;
mod gfx;
mod input_state;
mod logging;
mod math;
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
    let mut input_state = InputState::new();

    let mut scene = PlaygroundScene::new(&mut assets);
    let textures: Vec<_> = assets.textures().collect();
    let meshes: Vec<_> = assets.meshes().collect();
    renderer.use_textures(&textures);
    renderer.use_meshes(&meshes);

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                input_state.set_mouse_offset((delta.0 as f32, delta.1 as f32));
            }
            _ => {}
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized(new_size) => {
                renderer.handle_resize(new_size);
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => input_state.set_pressed(Key::MouseButton(button)),
                ElementState::Released => input_state.set_released(Key::MouseButton(button)),
            },
            WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                Some(code) => match input.state {
                    ElementState::Pressed => {
                        input_state.set_pressed(Key::KeyboardKey(code));
                    }
                    ElementState::Released => {
                        input_state.set_released(Key::KeyboardKey(code));
                    }
                },
                None => {}
            },
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            let delta = timer.elapsed();
            timer.reset();

            scene.update(&window, &input_state, delta, &mut assets);

            renderer.draw(&mut scene, &mut assets);

            input_state.end_frame();
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
