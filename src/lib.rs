use math::Point2D;
use winit::{
    event::{DeviceEvent, ElementState, Event, TouchPhase, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use crate::{
    assets::Assets,
    input_state::{InputState, Key},
    logging::{debug, trace, warn},
    rendering::renderer::Renderer,
    scenes::{DynamicScene, PlaygroundScene},
    timer::Timer,
};

mod assets;
mod camera;
mod input_state;
mod logging;
mod math;
mod obj_loader;
mod object;
mod path_wrangler;
mod platform;
mod rendering;
mod scenes;
mod timer;
mod transform;

const TITLE: &str = "Chessno";

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(level = "trace"))
)]
pub fn main() {
    if !cfg!(target_os = "android") {
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
    let mut started = false;

    event_loop.run(move |event, _, control_flow| match event {
        Event::Resumed => {
            debug!("Resumed");
            if !renderer.is_initialized() {
                renderer.initialize_with_window(&window);
                renderer.load_assets(&assets);
            } else {
                renderer.invalidate_surface(&window);
            }
        }
        Event::Suspended => {
            debug!("Suspended")
        }
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                input_state.set_mouse_offset(Point2D::new(delta.0, delta.1));
            }
            _ => {}
        },
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Focused(value) => {
                debug!("Focused: {value}");
                focus = value;
            }
            WindowEvent::Resized(new_size) => {
                assert_eq!(new_size, window.inner_size());
                renderer.invalidate_surface(&window);
            }
            WindowEvent::Touch(e) => match e.phase {
                TouchPhase::Started => {
                    input_state
                        .set_touch_start_position(e.id, Point2D::new(e.location.x, e.location.y));
                }
                TouchPhase::Moved => {
                    input_state
                        .set_touch_move_position(e.id, Point2D::new(e.location.x, e.location.y));
                }
                TouchPhase::Ended => {
                    input_state
                        .set_touch_end_position(e.id, Point2D::new(e.location.x, e.location.y));
                }
                TouchPhase::Cancelled => {
                    warn!("{e:?}");
                }
            },
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

            if renderer.is_initialized() && (focus || !started) {
                renderer.draw(&mut scene, &mut assets);
            }

            input_state.end_frame();

            if !started {
                started = true;
            }
        }
        _ => {}
    })
}
