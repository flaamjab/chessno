use std::time::Instant;

use camera::Camera;
use cgmath::{Array, Vector3, Vector4, Zero};
use renderer::Renderer;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::logging::{debug, trace};
use crate::object::Object;

mod camera;
mod context;
mod erupt;
mod geometry;
mod logging;
mod mesh;
mod object;
mod physical_device;
mod renderer;
mod shader;
mod swapchain;
mod sync_pool;
mod transform;
#[cfg(debug_assertions)]
mod validation;

const TITLE: &str = "Isochess";

pub fn linux_main() {
    logging::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut prev_time = Instant::now();
    let mut delta = 0.0;

    let mut renderer = Renderer::new(TITLE, &window);
    let plane = mesh::new_plane();
    let mut objects = [Object {
        mesh: plane,
        position: Vector3::zero(),
        rotation: Vector4::new(0.0, -1.0, 0.0, 0.0),
    }];

    let rotation_speed = 10.0f32;
    let mut rotation_angle = 0.0;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::Resized(new_size) => {
                trace!("Window resized, notifying renderer");
                renderer.handle_resize(new_size);
            }
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            rotation_angle += delta * rotation_speed;
            if rotation_angle > 360.0 {
                rotation_angle -= 360.0;
            }

            let cur_time = Instant::now();
            delta = cur_time.duration_since(prev_time).as_secs_f32();
            prev_time = cur_time;

            let view = Vector3::new(0.0, 0.0, 2.0);
            let PhysicalSize { width, height } = window.inner_size();
            let projection = camera::perspective(45.0, width as f32 / height as f32, 0.1, 100.0);
            let camera = Camera::new(&view, &Vector4::from_value(0.0), &projection);

            objects[0].rotation.w = rotation_angle;
            renderer.draw(&objects, &camera);
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
