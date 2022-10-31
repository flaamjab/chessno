use nalgebra::{Matrix4, Perspective3};

#[derive(Debug)]
pub enum Projection {
    Orthographic(OrthographicProjection),
    Perspective(PerspectiveProjection),
}

impl Projection {
    pub fn perspective(fov_deg: f32, near: f32, far: f32) -> Self {
        Self::Perspective(PerspectiveProjection::new(fov_deg, near, far))
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        match self {
            Self::Perspective(p) => p.matrix(),
            _ => unimplemented!(),
        }
    }

    pub fn set_viewport_dimensions(&mut self, width: f32, height: f32) {
        match self {
            Self::Perspective(p) => p.set_aspect_ratio(width / height),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub struct OrthographicProjection {
    width: f32,
    height: f32,
    near: f32,
    far: f32,
}

#[derive(Debug)]
pub struct PerspectiveProjection {
    fov_deg: f32,
    aspect: f32,
    near: f32,
    far: f32,
}

impl PerspectiveProjection {
    fn new(fov_deg: f32, near: f32, far: f32) -> Self {
        Self {
            fov_deg,
            near,
            far,
            aspect: 0.0,
        }
    }

    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        let mut p = Perspective3::new(self.aspect, self.fov_deg.to_radians(), self.near, self.far)
            .to_homogeneous();
        p.m22 *= -1.0;

        p
    }
}
