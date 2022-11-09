use std::path::Path;

use nalgebra::{Point3, Vector4};
use winit::window::Window;

use crate::{
    assets::{Asset, Assets},
    camera::{Camera, CameraControl, FreeCameraMouseControl, FreeCameraTouchControl},
    input_state::InputState,
    obj_loader::ObjLoader,
    object::Object,
    rendering::projection::Projection,
    rendering::{mesh::Mesh, texture::Texture},
    scenes::{DynamicScene, Scene},
    transform::Transform,
};

pub struct PlaygroundScene {
    objects: Vec<Object>,
    camera_control: Box<dyn CameraControl>,
}

impl PlaygroundScene {
    pub fn new(assets: &mut Assets) -> Self {
        let objects = Self::setup_objects(assets);
        let camera_control = Self::camera_control();
        Self {
            objects,
            camera_control,
        }
    }

    fn setup_objects(assets: &mut Assets) -> Vec<Object> {
        let table_path = Path::new("models/table/table.obj");
        let plant_path = Path::new("models/indoor_plant/indoor plant_02.obj");
        let m1887_path = Path::new("models/m1887/M1887.obj");

        // let mut mesh_loader = ObjLoader::new(assets);
        // let gun_mesh_id = mesh_loader.load(m1887_path, "gun");
        // let table_mesh_id = mesh_loader.load(table_path, "table");

        let mut objects = Vec::with_capacity(128);
        // objects.push(Object {
        //     mesh_id: gun_mesh_id,
        //     transform: Transform::new(Point3::origin(), Vector4::zeros(), 1.0),
        // });
        // objects.push(Object {
        //     mesh_id: table_mesh_id,
        //     transform: Transform::new(Point3::new(5.0, 0.1, 5.0), Vector4::zeros(), 0.001),
        // });

        let mut chess_chells = Self::create_chess_board(assets);
        objects.append(&mut chess_chells);

        objects
    }

    fn create_chess_board(assets: &mut Assets) -> Vec<Object> {
        let locator = assets.asset_locator();
        let shrek_texture = Texture::from_asset(locator, Path::new("textures/shrek.jpg")).unwrap();
        let shrek_texture_id = assets.insert_texture("shrek", shrek_texture);

        let chess_cell = Mesh::new_plane(shrek_texture_id);
        let chess_cell_id = assets.insert_mesh("chess_cell", chess_cell);

        let cell = assets.mesh(chess_cell_id).unwrap();
        let cell_w = cell.bbox.width;
        let cell_l = cell.bbox.length;

        let mut objects = Vec::with_capacity(16);
        for row in 0..8 {
            for col in 0..8 {
                let o = Object {
                    mesh_id: cell.id(),
                    transform: Transform::new(
                        Point3::new(cell_w * row as f32, 0.0, cell_l * col as f32),
                        Vector4::new(1.0, 0.0, 0.0, 90.0),
                        1.0,
                    ),
                };
                objects.push(o);
            }
        }

        objects
    }

    fn camera_control() -> Box<dyn CameraControl> {
        if cfg!(target_os = "android") {
            Box::new(Self::touch_camera_control())
        } else if cfg!(not(any(target_os = "iOS"))) {
            Box::new(Self::mouse_camera_control())
        } else {
            panic!("unsupported platform")
        }
    }

    fn mouse_camera_control() -> FreeCameraMouseControl {
        let camera = Self::new_camera();
        FreeCameraMouseControl::new(camera, 3.5, 60.0)
    }

    fn touch_camera_control() -> FreeCameraTouchControl {
        let camera = Self::new_camera();
        FreeCameraTouchControl::new(camera, 3.5, 120.0, 0.3)
    }

    fn new_camera() -> Camera {
        let camera_pos = Point3::new(0.0, 1.0, 2.0);
        let camera_dir = -camera_pos.coords;
        let projection = Projection::perspective(45.0, 0.1, 100.0);
        Camera::new(&camera_pos, &camera_dir, projection)
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
