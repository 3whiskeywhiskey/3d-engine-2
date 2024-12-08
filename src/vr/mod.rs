use anyhow::Result;
use openxr as xr;
use wgpu;
use glam::{Mat4, Vec3, Quat};

#[derive(Debug)]
pub struct ViewProjection {
    pub view: Mat4,
    pub projection: Mat4,
}

pub struct VRSystem {
    instance: xr::Instance,
    system: xr::SystemId,
    session: Option<xr::Session<xr::Vulkan>>,
    stage: Option<xr::Space>,
    view_configuration: Option<xr::ViewConfigurationProperties>,
    views: Option<Vec<xr::ViewConfigurationView>>,
}

impl VRSystem {
    pub fn new() -> Result<Self> {
        // Create OpenXR instance with Vulkan graphics API
        let entry = xr::Entry::linked();
        let app_info = xr::ApplicationInfo {
            application_name: "WGPU 3D Viewer",
            application_version: 1,
            engine_name: "No Engine",
            engine_version: 1,
        };

        // Available extensions
        let available_extensions = entry.enumerate_extensions()?;
        #[cfg(debug_assertions)]
        log::debug!("Available OpenXR extensions: {:?}", available_extensions);

        // Required extensions for our application
        let mut required_extensions = xr::ExtensionSet::default();
        required_extensions.khr_vulkan_enable2 = true;  // Enable Vulkan 2 support

        // Create instance
        let instance = entry.create_instance(&app_info, &required_extensions, &[])?;

        // Get the system (HMD) with Vulkan graphics API
        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;

        Ok(Self {
            instance,
            system,
            session: None,
            stage: None,
            view_configuration: None,
            views: None,
        })
    }

    pub fn initialize_session(&mut self, device: &wgpu::Device) -> Result<()> {
        // Get system properties for Vulkan device creation
        let requirements = self.instance.graphics_requirements::<xr::Vulkan>(self.system)?;
        
        // TODO: We need to properly get these from wgpu/Vulkan
        // For now, we'll use placeholder values that should work with most systems
        let vk_session_create_info = xr::vulkan::SessionCreateInfo {
            instance: std::ptr::null(),
            physical_device: std::ptr::null(),
            device: std::ptr::null(),
            queue_family_index: 0,
            queue_index: 0,
        };

        // Create OpenXR session
        let (session, frame_waiter, frame_stream) = unsafe {
            self.instance.create_session::<xr::Vulkan>(self.system, &vk_session_create_info)?
        };

        // Get view configuration and views
        self.view_configuration = Some(self.instance.view_configuration_properties(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?);

        self.views = Some(self.instance.enumerate_view_configuration_views(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?);

        // Begin session
        session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;

        // Create reference space
        let stage = session.create_reference_space(
            xr::ReferenceSpaceType::STAGE,
            xr::Posef::IDENTITY,
        )?;

        // Store session and stage
        self.session = Some(session);
        self.stage = Some(stage);

        Ok(())
    }

    pub fn is_hmd_available(&self) -> bool {
        // Check if we can get view configuration views (means HMD is connected and available)
        self.instance
            .enumerate_view_configurations(self.system)
            .map(|configs| configs.contains(&xr::ViewConfigurationType::PRIMARY_STEREO))
            .unwrap_or(false)
    }

    pub fn get_view_configuration(&self) -> Result<xr::ViewConfigurationProperties> {
        Ok(self.instance.view_configuration_properties(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?)
    }

    pub fn get_view_configuration_views(&self) -> Result<Vec<xr::ViewConfigurationView>> {
        Ok(self.instance.enumerate_view_configuration_views(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?)
    }

    pub fn get_views(&mut self, frame_state: &xr::FrameState) -> Result<Vec<xr::View>> {
        if let (Some(session), Some(stage)) = (&self.session, &self.stage) {
            let (_, views) = session.locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                frame_state.predicted_display_time,
                stage,
            )?;
            Ok(views)
        } else {
            Err(anyhow::anyhow!("Session or stage not initialized"))
        }
    }

    pub fn get_view_projections(&mut self, frame_state: &xr::FrameState) -> Result<Vec<ViewProjection>> {
        let views = self.get_views(frame_state)?;
        
        let mut view_projections = Vec::new();
        for view in views {
            // Convert OpenXR pose to view matrix
            let position = Vec3::new(
                view.pose.position.x,
                view.pose.position.y,
                view.pose.position.z,
            );
            
            let orientation = Quat::from_xyzw(
                view.pose.orientation.x,
                view.pose.orientation.y,
                view.pose.orientation.z,
                view.pose.orientation.w,
            );

            // Create view matrix (inverse of pose transform)
            let view_matrix = Mat4::from_rotation_translation(orientation, position).inverse();

            // Create projection matrix from OpenXR FoV
            let projection_matrix = perspective_infinite_reverse_rh(
                view.fov.angle_left,
                view.fov.angle_right,
                view.fov.angle_up,
                view.fov.angle_down,
                0.001,  // Near plane
            );

            view_projections.push(ViewProjection {
                view: view_matrix,
                projection: projection_matrix,
            });
        }

        Ok(view_projections)
    }

    pub fn get_swapchain_image_layout(&self) -> Option<(u32, u32)> {
        self.views.as_ref().map(|views| {
            let view = &views[0];  // Both eyes use the same resolution
            (
                view.recommended_image_rect_width,
                view.recommended_image_rect_height,
            )
        })
    }
}

// Helper function to create a perspective projection matrix from FoV angles
fn perspective_infinite_reverse_rh(
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_vr_system_creation() {
        let vr = VRSystem::new();
        assert!(vr.is_ok(), "Failed to create VR system: {:?}", vr.err());
    }

    #[test]
    #[serial]
    fn test_hmd_availability() {
        let vr = VRSystem::new().expect("Failed to create VR system");
        println!("HMD available: {}", vr.is_hmd_available());
        // Note: This test might fail if no HMD is connected
        // assert!(vr.is_hmd_available(), "No HMD detected");
    }

    #[test]
    #[serial]
    fn test_view_configuration() {
        let vr = VRSystem::new().expect("Failed to create VR system");
        if vr.is_hmd_available() {
            let config = vr.get_view_configuration();
            assert!(config.is_ok(), "Failed to get view configuration: {:?}", config.err());
            
            if let Ok(config) = config {
                println!("View configuration:");
                println!("  FOV mutable: {}", config.fov_mutable);
            }

            // Get recommended view configuration
            if let Ok(views) = vr.get_view_configuration_views() {
                for (i, view) in views.iter().enumerate() {
                    println!("View {}:", i);
                    println!("  Recommended width: {}", view.recommended_image_rect_width);
                    println!("  Recommended height: {}", view.recommended_image_rect_height);
                    println!("  Max swapchain samples: {}", view.max_swapchain_sample_count);
                }
            }
        } else {
            println!("Skipping view configuration test - no HMD available");
        }
    }
} 