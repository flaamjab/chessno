use math::Point2D;
use winit::{
    event::{DeviceEvent, ElementState, Event, TouchPhase, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use crate::{
    assets::Assets,
    input_state::{InputState, Key},
    logging::{debug, warn},
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
    let mut renderer: Option<Renderer> = None;

    let mut timer = Timer::new();
    let mut input_state = InputState::new();

    let mut scene = PlaygroundScene::new(&mut assets);
    let mut active = false;

    event_loop.run(move |event, _, control_flow| match event {
        Event::Resumed => {
            debug!("Resumed");
            active = true;
            timer.reset();
            match &mut renderer {
                Some(renderer) => {
                    debug!("Invalidating surface after resume");
                    renderer.invalidate_surface(&window);
                    renderer.resume();
                }
                None => {
                    let mut new_renderer = Renderer::new(TITLE, &window);
                    new_renderer.load_assets(&assets);
                    renderer = Some(new_renderer);
                }
            }
        }
        Event::Suspended => {
            active = false;
            if let Some(renderer) = &mut renderer {
                renderer.pause();
            }
            debug!("Suspended");
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
            }
            WindowEvent::Resized(new_size) => {
                debug!("Window resized");
                assert_eq!(new_size, window.inner_size());
                if let Some(renderer) = &mut renderer {
                    debug!("Invalidating surface");
                    renderer.invalidate_surface_size(new_size);
                }
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
            if active {
                let delta = timer.elapsed();
                timer.reset();

                scene.update(&window, &input_state, delta, &mut assets);

                if let Some(renderer) = &mut renderer {
                    renderer.draw(&mut scene, &mut assets);
                }

                input_state.end_frame();
            }
        }
        _ => {}
    })
}
