mod renderer;
#[cfg(test)]
mod tests;

pub use renderer::Renderer;
use glam::{Mat4, Vec3};
use crate::model::Model;
use winit::keyboard::KeyCode;
use std::time::Instant;

pub mod camera;
use camera::Camera;

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

pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<(Model, Transform)>,
    pub light_direction: Vec3,
    pub directional_light: Vec3,
    pub ambient_light: Vec3,
    last_update: Instant,
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
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();
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

    pub fn render<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>) {
        // Render each object in the scene
        for (model, _transform) in &self.objects {
            model.render(&mut render_pass);
        }
    }
} 