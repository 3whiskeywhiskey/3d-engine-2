use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // Rotation around Y axis
    pub pitch: f32, // Rotation around X axis
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    // Movement state
    pub moving_forward: bool,
    pub moving_backward: bool,
    pub moving_left: bool,
    pub moving_right: bool,
    pub moving_up: bool,
    pub moving_down: bool,
}

impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            yaw: -90.0,
            pitch: 0.0,
            fov: 45.0,
            aspect,
            near: 0.1,
            far: 100.0,
            moving_forward: false,
            moving_backward: false,
            moving_left: false,
            moving_right: false,
            moving_up: false,
            moving_down: false,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let projection = Mat4::perspective_rh_gl(
            self.fov.to_radians(),
            self.aspect,
            self.near,
            self.far,
        );
        
        let view_dir = self.get_view_direction();
        let target = self.position + view_dir;
        let view = Mat4::look_at_rh(
            self.position,
            target,
            Vec3::Y,
        );

        projection * view
    }

    pub fn get_forward(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.to_radians().sin_cos();
        Vec3::new(yaw_cos, 0.0, yaw_sin).normalize()
    }

    pub fn get_right(&self) -> Vec3 {
        let forward = self.get_forward();
        forward.cross(Vec3::Y).normalize()
    }

    fn get_view_direction(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.to_radians().sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch.to_radians().sin_cos();
        Vec3::new(
            yaw_cos * pitch_cos,
            pitch_sin,
            yaw_sin * pitch_cos,
        ).normalize()
    }

    pub fn process_mouse(&mut self, dx: f32, dy: f32) {
        const MOUSE_SENSITIVITY: f32 = 1.0;
        
        self.yaw += dx * MOUSE_SENSITIVITY;
        let new_pitch = self.pitch - dy * MOUSE_SENSITIVITY;
        self.pitch = new_pitch.clamp(-89.0, 89.0);
    }

    pub fn update(&mut self, dt: f32) {
        const SPEED: f32 = 5.0;
        let velocity = SPEED * dt;

        let forward = self.get_forward();
        let right = self.get_right();

        if self.moving_forward {
            self.position += forward * velocity;
        }
        if self.moving_backward {
            self.position -= forward * velocity;
        }
        if self.moving_right {
            self.position += right * velocity;
        }
        if self.moving_left {
            self.position -= right * velocity;
        }
        if self.moving_up {
            self.position.y += velocity;
        }
        if self.moving_down {
            self.position.y -= velocity;
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW => self.moving_forward = pressed,
            KeyCode::KeyS => self.moving_backward = pressed,
            KeyCode::KeyA => self.moving_left = pressed,
            KeyCode::KeyD => self.moving_right = pressed,
            KeyCode::Space => self.moving_up = pressed,
            KeyCode::ShiftLeft => self.moving_down = pressed,
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_camera_initialization() {
        let camera = Camera::new(Vec3::new(1.0, 2.0, 3.0), 16.0/9.0);
        assert_eq!(camera.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(camera.yaw, -90.0);
        assert_eq!(camera.pitch, 0.0);
        assert_eq!(camera.aspect, 16.0/9.0);
        assert!(!camera.moving_forward);
        assert!(!camera.moving_backward);
        assert!(!camera.moving_left);
        assert!(!camera.moving_right);
        assert!(!camera.moving_up);
        assert!(!camera.moving_down);
    }

    #[test]
    fn test_view_direction() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0);
        
        // Looking along -Z (default)
        let dir = camera.get_view_direction();
        assert_relative_eq!(dir.x, 0.0, epsilon = 0.001);
        assert_relative_eq!(dir.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(dir.z, -1.0, epsilon = 0.001);

        // Look right (+X)
        camera.yaw = 0.0;
        let dir = camera.get_view_direction();
        assert_relative_eq!(dir.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(dir.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(dir.z, 0.0, epsilon = 0.001);

        // Look up
        camera.pitch = 90.0;
        let dir = camera.get_view_direction();
        assert_relative_eq!(dir.x, 0.0, epsilon = 0.001);
        assert_relative_eq!(dir.y, 1.0, epsilon = 0.001);
        assert_relative_eq!(dir.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_movement_directions() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0);
        
        // Test forward direction (should be -Z when yaw is -90)
        let forward = camera.get_forward();
        assert_relative_eq!(forward.x, 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.z, -1.0, epsilon = 0.001);

        // Test right direction (should be +X when looking along -Z)
        let right = camera.get_right();
        assert_relative_eq!(right.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(right.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(right.z, 0.0, epsilon = 0.001);

        // Look right and test again
        camera.yaw = 0.0;
        let forward = camera.get_forward();
        assert_relative_eq!(forward.x, 1.0, epsilon = 0.001);
        assert_relative_eq!(forward.y, 0.0, epsilon = 0.001);
        assert_relative_eq!(forward.z, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_keyboard_input() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0);
        
        // Test each key individually
        let test_cases = [
            (KeyCode::KeyW, "moving_forward"),
            (KeyCode::KeyS, "moving_backward"),
            (KeyCode::KeyA, "moving_left"),
            (KeyCode::KeyD, "moving_right"),
            (KeyCode::Space, "moving_up"),
            (KeyCode::ShiftLeft, "moving_down"),
        ];

        for (key, flag_name) in test_cases {
            camera.process_keyboard(key, true);
            let flag_value = match flag_name {
                "moving_forward" => camera.moving_forward,
                "moving_backward" => camera.moving_backward,
                "moving_left" => camera.moving_left,
                "moving_right" => camera.moving_right,
                "moving_up" => camera.moving_up,
                "moving_down" => camera.moving_down,
                _ => unreachable!(),
            };
            assert!(flag_value, "Key {:?} did not set {} flag", key, flag_name);

            camera.process_keyboard(key, false);
            let flag_value = match flag_name {
                "moving_forward" => camera.moving_forward,
                "moving_backward" => camera.moving_backward,
                "moving_left" => camera.moving_left,
                "moving_right" => camera.moving_right,
                "moving_up" => camera.moving_up,
                "moving_down" => camera.moving_down,
                _ => unreachable!(),
            };
            assert!(!flag_value, "Key {:?} did not clear {} flag", key, flag_name);
        }
    }

    #[test]
    fn test_mouse_movement() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0);
        
        // Test yaw movement
        camera.process_mouse(10.0, 0.0);
        assert_relative_eq!(camera.yaw, -90.0 + 10.0, epsilon = 0.001); // 1.0 sensitivity

        // Test small pitch movements
        camera.process_mouse(0.0, -10.0);  // Move mouse up
        assert_relative_eq!(camera.pitch, 10.0, epsilon = 0.001); // Should increase pitch
        
        camera.process_mouse(0.0, 10.0);   // Move mouse down
        assert_relative_eq!(camera.pitch, 0.0, epsilon = 0.001); // Should decrease pitch
        
        // Test pitch clamping with large movements
        camera.process_mouse(0.0, -100.0); // Move mouse way up
        assert_relative_eq!(camera.pitch, 89.0, epsilon = 0.001); // Should clamp to 89
        
        camera.process_mouse(0.0, 100.0);  // Move mouse way down
        assert_relative_eq!(camera.pitch, -89.0, epsilon = 0.001); // Should clamp to -89
    }

    #[test]
    fn test_movement_update() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0);
        let dt = 1.0;
        
        // Test forward movement
        camera.moving_forward = true;
        camera.update(dt);
        assert_relative_eq!(camera.position.z, -5.0, epsilon = 0.001); // SPEED = 5.0

        // Reset and test right movement
        camera = Camera::new(Vec3::ZERO, 1.0);
        camera.moving_right = true;
        camera.update(dt);
        assert_relative_eq!(camera.position.x, 5.0, epsilon = 0.001);

        // Test vertical movement
        camera = Camera::new(Vec3::ZERO, 1.0);
        camera.moving_up = true;
        camera.update(dt);
        assert_relative_eq!(camera.position.y, 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_view_matrix_changes() {
        let mut camera = Camera::new(Vec3::new(0.0, 0.0, 5.0), 1.0);
        let initial_matrix = camera.build_view_projection_matrix();
        
        // Move camera and verify matrix changes
        camera.position = Vec3::new(1.0, 1.0, 5.0);
        let moved_matrix = camera.build_view_projection_matrix();
        assert_ne!(initial_matrix, moved_matrix);

        // Rotate camera and verify matrix changes
        camera.yaw = 0.0;
        let rotated_matrix = camera.build_view_projection_matrix();
        assert_ne!(moved_matrix, rotated_matrix);
    }
} 