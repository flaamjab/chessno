use std::collections::HashSet;
use std::path::Path;

use nalgebra::{Point3, Rotation3, Unit, Vector3, Vector4};
use winit::event::VirtualKeyCode;

use crate::camera::Camera;
use crate::mesh::Mesh;
use crate::object::Object;
use crate::scene::DynamicScene;
use crate::scene::Scene;
use crate::scene::Scenelike;
use crate::transform::Transform;

pub struct PlaygroundScene {
    inner: Scene,

    up: Unit<Vector3<f32>>,

    camera_pos: Point3<f32>,
    camera_dir: Unit<Vector3<f32>>,
    camera_right: Unit<Vector3<f32>>,

    move_speed: f32,
    look_sensitivity: f32,
}

impl PlaygroundScene {
    pub fn new(aspect_ratio: f32) -> Self {
        let up = Vector3::y_axis();
        let cube = Mesh::from_file(Path::new("assets/models/cube.obj")).unwrap();
        let plant = Mesh::from_file(Path::new("assets/models/indoor plant_02.obj")).unwrap();
        let objects = vec![
            Object {
                mesh: cube.clone(),
                transform: Transform::new(
                    Vector3::new(3.0, 1.0, 3.0),
                    Vector4::new(0.0, 1.0, 0.0, 30.0),
                ),
            },
            Object {
                mesh: cube.clone(),
                transform: Transform::new(Vector3::new(0.0, 0.0, -1.0), Vector4::zeros()),
            },
            Object {
                mesh: cube.clone(),
                transform: Transform::new(Vector3::new(-5.0, 0.0, -5.0), Vector4::zeros()),
            },
            Object {
                mesh: plant,
                transform: Transform::new(Vector3::new(-2.0, 2.0, 2.0), Vector4::zeros()),
            },
        ];

        let camera_pos = Point3::new(0.0, -1.0, -2.0);
        let camera_dir = Unit::new_normalize(-camera_pos.coords);
        let camera_right = Unit::new_normalize(up.cross(&camera_dir));

        let projection = Camera::perspective(45.0, aspect_ratio, 0.1, 100.0);
        let camera = Camera::new(&camera_pos, &camera_dir, &projection);

        Self {
            inner: Scene {
                objects,
                cameras: vec![camera],
            },
            up,
            camera_pos,
            camera_dir,
            camera_right,
            move_speed: 10.0,
            look_sensitivity: 150.0,
        }
    }

    fn camera_change(
        &mut self,
        delta: f32,
        pressed_keys: &HashSet<VirtualKeyCode>,
    ) -> (Vector3<f32>, Rotation3<f32>, Rotation3<f32>) {
        let mut camera_velocity = Vector3::zeros();
        let mut rot_left_right = Rotation3::identity();
        let mut rot_up_down = Rotation3::identity();

        if pressed_keys.contains(&VirtualKeyCode::W) {
            camera_velocity += self.camera_dir.as_ref();
        }

        if pressed_keys.contains(&VirtualKeyCode::A) {
            camera_velocity -= self.camera_right.as_ref();
        }

        if pressed_keys.contains(&VirtualKeyCode::S) {
            camera_velocity -= self.camera_dir.as_ref();
        }

        if pressed_keys.contains(&VirtualKeyCode::D) {
            camera_velocity += self.camera_right.as_ref();
        }
        camera_velocity *= self.move_speed * delta;

        let look_offset = (self.look_sensitivity * delta).to_radians();
        if pressed_keys.contains(&VirtualKeyCode::Up) {
            rot_up_down = Rotation3::from_axis_angle(&self.camera_right, -look_offset);
        }

        if pressed_keys.contains(&VirtualKeyCode::Down) {
            rot_up_down = Rotation3::from_axis_angle(&self.camera_right, look_offset);
        }

        if pressed_keys.contains(&VirtualKeyCode::Left) {
            rot_left_right = Rotation3::from_axis_angle(&self.up, -look_offset);
        }

        if pressed_keys.contains(&VirtualKeyCode::Right) {
            rot_left_right = Rotation3::from_axis_angle(&self.up, look_offset);
        }

        (camera_velocity, rot_left_right, rot_up_down)
    }
}

impl Scenelike for PlaygroundScene {
    fn active_camera(&self) -> &Camera {
        &self.inner.cameras[0]
    }

    fn cameras(&self) -> &[Camera] {
        &self.inner.cameras
    }

    fn objects(&self) -> &[Object] {
        &self.inner.objects
    }
}

impl DynamicScene for PlaygroundScene {
    fn update(
        &mut self,
        time_delta: f32,
        pressed_keys: &HashSet<VirtualKeyCode>,
        aspect_ratio: f32,
    ) {
        let (camera_velocity, rot_left_right, rot_up_down) =
            self.camera_change(time_delta, &pressed_keys);

        self.camera_pos = self.camera_pos + camera_velocity;
        self.camera_dir = rot_left_right * rot_up_down * self.camera_dir;
        self.camera_right = Unit::new_normalize(self.up.cross(&self.camera_dir));

        let projection = Camera::perspective(45.0, aspect_ratio, 0.1, 100.0);
        self.inner.cameras[0] = Camera::new(&self.camera_pos, &self.camera_dir, &projection);
    }
}
