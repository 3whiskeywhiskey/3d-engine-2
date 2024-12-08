use glam::{Mat4, Vec3};

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