use std::{collections::HashSet, hash::Hash, time::Instant};

use camera::Camera;
use cgmath::{
    Deg, EuclideanSpace, InnerSpace, Matrix3, Point3, SquareMatrix, Vector3, Vector4, Zero,
};
use mesh::Mesh;
use transform::Transform;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::gfx::renderer::Renderer;
use crate::logging::{debug, trace};
use crate::object::Object;

mod camera;
mod gfx;
mod logging;
mod mesh;
mod object;
mod transform;

const TITLE: &str = "Isochess";

pub fn linux_main() {
    logging::init();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let mut renderer = Renderer::new(TITLE, &window);
    let plane = Mesh::new_plane();
    let up = Vector4::unit_y();
    let right = Vector4::unit_x();
    let mut objects = [
        Object {
            mesh: plane.clone(),
            transform: Transform {
                position: Vector3::new(1.0, 1.0, 0.0),
                rotation: Vector4::new(0.0, 1.0, 0.0, 30.0),
            },
        },
        Object {
            mesh: plane.clone(),
            transform: Transform {
                position: Vector3::zero(),
                rotation: right,
            },
        },
        Object {
            mesh: plane,
            transform: Transform {
                position: Vector3::new(-0.5, -1.0, -0.5),
                rotation: up,
            },
        },
    ];

    let mut prev_time = Instant::now();
    let mut delta = 0.33;

    let up = Vector3::new(0.0, -1.0, 0.0);
    let mut camera_pos = Point3::new(0.0, -1.0, -2.0);
    let mut camera_dir = -camera_pos.to_vec().normalize();
    let mut camera_right = up.cross(camera_dir).normalize();
    let move_speed = 10.0;
    let look_sensitivity = 150.0;

    let mut pressed_keys = HashSet::new();
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
            let (camera_velocity, rot_left_right, rot_up_down) = update_camera(
                &camera_dir,
                &camera_right,
                move_speed,
                look_sensitivity,
                delta,
                &pressed_keys,
            );
            camera_dir = (rot_left_right * rot_up_down * camera_dir).normalize();

            camera_pos = camera_pos + camera_velocity;
            camera_right = camera_dir.cross(up).normalize();

            let projection =
                camera::perspective(45.0, aspect_ratio(window.inner_size()), 0.1, 100.0);
            let camera = Camera::new(&camera_pos, &camera_dir, &projection);

            renderer.draw(&objects, &camera);

            let now = Instant::now();
            delta = now.duration_since(prev_time).as_secs_f32();
            prev_time = now;
        }
        _ => {}
    })
}

fn update_camera(
    camera_dir: &Vector3<f32>,
    camera_right: &Vector3<f32>,
    move_speed: f32,
    look_sensitivity: f32,
    delta: f32,
    pressed_keys: &HashSet<VirtualKeyCode>,
) -> (Vector3<f32>, Matrix3<f32>, Matrix3<f32>) {
    let mut camera_velocity = Vector3::zero();
    let mut rot_left_right = Matrix3::identity();
    let mut rot_up_down = Matrix3::identity();

    if pressed_keys.contains(&VirtualKeyCode::W) {
        camera_velocity += *camera_dir;
    }

    if pressed_keys.contains(&VirtualKeyCode::A) {
        camera_velocity -= *camera_right;
    }

    if pressed_keys.contains(&VirtualKeyCode::S) {
        camera_velocity -= *camera_dir;
    }

    if pressed_keys.contains(&VirtualKeyCode::D) {
        camera_velocity += *camera_right;
    }
    camera_velocity *= move_speed * delta;

    let look_offset = look_sensitivity * delta;
    if pressed_keys.contains(&VirtualKeyCode::Up) {
        rot_up_down = Matrix3::from_axis_angle(*camera_right, Deg(look_offset));
    }

    if pressed_keys.contains(&VirtualKeyCode::Down) {
        rot_up_down = Matrix3::from_axis_angle(*camera_right, Deg(-look_offset));
    }

    if pressed_keys.contains(&VirtualKeyCode::Left) {
        rot_left_right = Matrix3::from_angle_y(Deg(-look_offset));
    }

    if pressed_keys.contains(&VirtualKeyCode::Right) {
        rot_left_right = Matrix3::from_angle_y(Deg(look_offset));
    }

    (camera_velocity, rot_left_right, rot_up_down)
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
