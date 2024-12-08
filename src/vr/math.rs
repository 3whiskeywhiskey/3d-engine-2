use glam::{Mat4, Vec3, Quat};
use openxr as xr;

#[derive(Debug)]
pub struct ViewProjection {
    pub view: Mat4,
    pub projection: Mat4,
    pub fov: xr::Fovf,
    pub pose: xr::Posef,
}

impl ViewProjection {
    pub fn from_xr_view(view: &xr::View, near: f32) -> Self {
        // Convert OpenXR pose to view matrix
        let view_matrix = create_view_matrix(&view.pose);

        // Create projection matrix from OpenXR FoV
        let projection_matrix = perspective_infinite_reverse_rh(
            view.fov.angle_left,
            view.fov.angle_right,
            view.fov.angle_up,
            view.fov.angle_down,
            near,
        );

        Self {
            view: view_matrix,
            projection: projection_matrix,
            fov: view.fov,
            pose: view.pose,
        }
    }
}

/// Creates a perspective projection matrix from FoV angles using reverse Z with infinite far plane
pub fn perspective_infinite_reverse_rh(
    left: f32,
    right: f32,
    up: f32,
    down: f32,
    near: f32,
) -> Mat4 {
    let left = f32::tan(left);
    let right = f32::tan(right);
    let up = f32::tan(up);
    let down = f32::tan(down);

    let width = right - left;
    let height = up - down;

    let x = 2.0 / width;
    let y = 2.0 / height;

    let a = (right + left) / width;
    let b = (up + down) / height;

    Mat4::from_cols_array(&[
        x,    0.0,  a,    0.0,
        0.0,  y,    b,    0.0,
        0.0,  0.0,  0.0,  near,
        0.0,  0.0,  -1.0, 0.0,
    ])
}

pub fn create_view_matrix(pose: &xr::Posef) -> Mat4 {
    let position = Vec3::new(
        pose.position.x,
        pose.position.y,
        pose.position.z,
    );
    
    let orientation = Quat::from_xyzw(
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    );

    Mat4::from_rotation_translation(orientation, position).inverse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perspective_matrix() {
        let fov = std::f32::consts::FRAC_PI_4; // 45 degrees
        let mat = perspective_infinite_reverse_rh(-fov, fov, fov, -fov, 0.1);
        
        // Test that the matrix preserves symmetry
        assert!((mat.col(0)[0] - mat.col(1)[1]).abs() < 1e-6);
        
        // Test that near plane is correctly set
        assert!((mat.col(2)[3] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_view_matrix() {
        let pose = xr::Posef {
            orientation: xr::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            position: xr::Vector3f {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };

        let view_mat = create_view_matrix(&pose);
        
        // Test that translation is negated in view matrix
        assert!((view_mat.col(3)[0] + 1.0).abs() < 1e-6);
        assert!((view_mat.col(3)[1] + 2.0).abs() < 1e-6);
        assert!((view_mat.col(3)[2] + 3.0).abs() < 1e-6);
    }
} 