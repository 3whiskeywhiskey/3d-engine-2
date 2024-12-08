use anyhow::Result;
use openxr as xr;

pub struct VRSystem {
    instance: xr::Instance,
    system: xr::SystemId,
    session: Option<xr::Session<xr::Vulkan>>,
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
        })
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