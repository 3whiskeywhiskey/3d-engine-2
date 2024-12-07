mod renderer;
#[cfg(test)]
mod tests;

pub use renderer::Renderer;
use glam::{Mat4, Vec3};
use crate::model::Model;

pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(self.position);
        let rotation = Mat4::from_euler(glam::EulerRot::XYZ, self.rotation.x, self.rotation.y, self.rotation.z);
        let scale = Mat4::from_scale(self.scale);
        translation * rotation * scale
    }
}

pub struct SceneObject {
    pub model: Model,
    pub transform: Transform,
}

pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            position: Vec3::new(0.0, 1.0, 2.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let proj = Mat4::perspective_rh(
            self.fovy.to_radians(),
            self.aspect,
            self.znear,
            self.zfar,
        );
        let flip_y = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
        flip_y * proj * view
    }
}

pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<(Model, Transform)>,
    pub ambient_light: Vec3,
    pub directional_light: Vec3,
    pub light_direction: Vec3,
}

impl Scene {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            camera: Camera::new(width, height),
            objects: Vec::new(),
            ambient_light: Vec3::splat(0.1),
            directional_light: Vec3::ONE,
            light_direction: Vec3::new(-1.0, -1.0, -1.0).normalize(),
        }
    }

    pub fn add_object(&mut self, model: Model, transform: Transform) {
        self.objects.push((model, transform));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect = width as f32 / height as f32;
    }
} 