use std::path::Path;

use nalgebra::{Point3, Vector4};
use winit::window::Window;

use crate::assets::Assets;
use crate::camera::Camera;
use crate::free_camera_control::FreeCameraControl;
use crate::input_state::InputState;
use crate::obj_loader::ObjLoader;
use crate::object::Object;
use crate::projection::Projection;
use crate::scene::DynamicScene;
use crate::scene::Scene;
use crate::transform::Transform;

pub struct PlaygroundScene {
    objects: Vec<Object>,
    camera_control: FreeCameraControl,
}

impl PlaygroundScene {
    pub fn new(assets: &mut Assets) -> Self {
        let objects = Self::setup_objects(assets);
        let camera_control = Self::setup_camera();
        Self {
            objects,
            camera_control,
        }
    }

    fn setup_objects(assets: &mut Assets) -> Vec<Object> {
        let table_path = Path::new("assets/models/table/table.obj");
        let plant_path = Path::new("assets/models/indoor_plant/indoor plant_02.obj");
        let m1887_path = Path::new("assets/models/m1887/M1887.obj");

        // let shrek_texture = Texture::from_file(Path::new("assets/textures/shrek.jpg")).unwrap();
        // let shrek_texture_id = shrek_texture.id();
        // assets.insert_texture("shrek", shrek_texture);

        // let chess_cell_id = Mesh::new_plane("chess_cell", shrek_texture_id, &mut assets);
        let mut mesh_loader = ObjLoader::new(assets);
        let mesh_id = mesh_loader.load_from_file(Path::new(m1887_path), "model");

        let mut objects = Vec::with_capacity(17);
        objects.push(Object {
            mesh_id,
            transform: Transform::new(Point3::origin(), Vector4::zeros()),
        });
        // objects.push(Object {
        //     mesh_id: plant_id,
        //     transform: Transform::new(Vector3::zeros(), Vector4::zeros()),
        // });

        // let cell = assets.get_mesh_by_id(chess_cell_id).unwrap();
        // let cell_w = cell.bbox.width;
        // let cell_l = cell.bbox.length;
        // for row in 0..8 {
        //     for col in 0..8 {
        //         let o = Object {
        //             mesh_id: cell.id(),
        //             transform: Transform {
        //                 position: Vector3::new(cell_w * row as f32, 0.0, cell_l * col as f32),
        //                 rotation: Vector4::new(1.0, 0.0, 0.0, 90.0),
        //             },
        //         };
        //         objects.push(o);
        //     }
        // }

        objects
    }

    fn setup_camera() -> FreeCameraControl {
        let camera_pos = Point3::new(0.0, -1.0, -2.0);
        let camera_dir = -camera_pos.coords;
        let projection = Projection::perspective(45.0, 0.1, 100.0);
        let camera = Camera::new(&camera_pos, &camera_dir, projection);
        FreeCameraControl::new(camera, 5.0, 60.0)
    }
}

impl Scene for PlaygroundScene {
    fn active_camera(&self) -> &Camera {
        &self.camera_control.camera()
    }

    fn active_camera_mut(&mut self) -> &mut Camera {
        self.camera_control.camera_mut()
    }

    fn objects(&self) -> &[Object] {
        &self.objects
    }
}

impl DynamicScene for PlaygroundScene {
    fn update(
        &mut self,
        window: &Window,
        input_state: &InputState,
        time_delta: f32,
        _assets: &mut Assets,
    ) {
        self.camera_control.update(window, input_state, time_delta);
    }
}
