pub mod camera;
pub mod transform;
#[cfg(test)]
mod tests;

pub use camera::Camera;
pub use transform::Transform;

use std::sync::Arc;
use wgpu::BindGroup;
use glam::Vec3;
use winit::keyboard::KeyCode;
use std::time::Instant;
use crate::model::Model;

pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<(Model, Transform)>,
    ambient_light: Vec3,
    directional_light: Vec3,
    light_direction: Vec3,
    last_update: Instant,
    camera_bind_group: Option<Arc<BindGroup>>,
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            objects: Vec::new(),
            light_direction: Vec3::new(-1.0, -1.0, -1.0).normalize(),
            directional_light: Vec3::new(1.0, 1.0, 1.0),
            ambient_light: Vec3::new(0.1, 0.1, 0.1),
            last_update: Instant::now(),
            camera_bind_group: None,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        self.camera.update(dt);
    }

    pub fn process_keyboard(&mut self, key: KeyCode, pressed: bool) {
        self.camera.process_keyboard(key, pressed);
    }

    pub fn process_mouse(&mut self, dx: f32, dy: f32) {
        self.camera.process_mouse(dx, dy);
    }

    pub fn add_object(&mut self, model: Model, transform: Transform) {
        self.objects.push((model, transform));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect = width as f32 / height as f32;
    }

    pub fn set_ambient_light(&mut self, intensity: f32) {
        self.ambient_light = Vec3::splat(intensity.clamp(0.0, 1.0));
    }

    pub fn set_directional_light(&mut self, color: Vec3, direction: Vec3) {
        self.directional_light = color.clamp(Vec3::ZERO, Vec3::ONE);
        self.light_direction = direction.normalize();
    }
} 