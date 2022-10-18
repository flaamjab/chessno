use std::time::Instant;

use camera::Camera;
use cgmath::{EuclideanSpace, Point3, Vector3, Vector4, Zero};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::gfx::mesh;
use crate::gfx::renderer::Renderer;
use crate::logging::{debug, trace};
use crate::object::Object;

mod camera;
mod gfx;
mod logging;
mod object;
const TITLE: &str = "Isochess";

pub fn linux_main() {
    logging::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut prev_time = Instant::now();
    let mut delta = 0.0;

    let mut renderer = Renderer::new(TITLE, &window);
    let plane = mesh::new_plane();
    let up = -Vector4::unit_y();
    let mut objects = [
        Object {
            mesh: plane.clone(),
            position: Vector3::new(1.0, 1.0, 0.0),
            rotation: up,
        },
        Object {
            mesh: plane.clone(),
            position: Vector3::zero(),
            rotation: up,
        },
    ];

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

            let camera_pos = Point3::new(0.0, -2.0, 1.0);
            let camera_dir = -camera_pos.to_vec();
            let projection =
                camera::perspective(45.0, aspect_ratio(window.inner_size()), 0.1, 100.0);
            let camera = Camera::new(&camera_pos, &camera_dir, &projection);

            for (n, ob) in objects.iter_mut().enumerate() {
                ob.rotation.w = rotation_angle + 10.0 * n as f32;
            }
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

pub fn aspect_ratio(size: PhysicalSize<u32>) -> f32 {
    let PhysicalSize { width, height } = size;
    width as f32 / height as f32
}
