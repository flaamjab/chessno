use winit::{
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::assets::Assets;
use crate::input_state::InputState;
use crate::logging::debug;
use crate::samples::PlaygroundScene;
use crate::scene::DynamicScene;
use crate::timer::Timer;
use crate::{gfx::renderer::Renderer, input_state::Key};

mod asset_locator;
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
mod platform;
mod projection;
mod samples;
mod scene;
mod timer;
mod transform;

const TITLE: &str = "Chessno";

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "debug"))
)]
pub fn main() {
    if !cfg!(target_os = "android") {
        debug!("Calling logging init");
        logging::init();
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(TITLE)
        .build(&event_loop)
        .unwrap();

    let mut assets = Assets::new();
    let mut renderer = Renderer::new(TITLE);

    let mut timer = Timer::new();
    let mut input_state = InputState::new();

    let mut scene = PlaygroundScene::new(&mut assets);
    let mut focus = false;

    event_loop.run(move |event, _, control_flow| match event {
        Event::Resumed => {
            debug!("Resumed")
        }
        Event::Suspended => {
            debug!("Suspended")
        }
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                input_state.set_mouse_offset((delta.0 as f32, delta.1 as f32));
            }
            _ => {}
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Focused(value) => {
                focus = value;
                if !renderer.is_initialized() {
                    renderer.initialize_with_window(&window);
                    let textures: Vec<_> = assets.textures().collect();
                    let meshes: Vec<_> = assets.meshes().collect();
                    renderer.use_textures(&textures);
                    renderer.use_meshes(&meshes);
                    focus = true;
                }
            }
            WindowEvent::Resized(new_size) => {
                renderer.handle_resize(new_size);
            }
            WindowEvent::Touch(e) => {
                debug!("{:?}", e);
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
                control_flow.set_exit();
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            let delta = timer.elapsed();
            timer.reset();

            scene.update(&window, &input_state, delta, &mut assets);

            if focus {
                renderer.draw(&mut scene, &mut assets);
            }

            input_state.end_frame();
        }
        _ => {}
    })
}
