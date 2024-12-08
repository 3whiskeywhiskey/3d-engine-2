mod pipeline;
pub use pipeline::{VRPipeline, VRUniform};

mod math;
pub use math::{ViewProjection, create_view_matrix, perspective_infinite_reverse_rh};

mod vulkan;
pub use vulkan::{
    get_vulkan_instance_from_wgpu,
    get_vulkan_physical_device_from_wgpu,
    get_vulkan_device_from_wgpu,
    get_vulkan_queue_info_from_wgpu,
    wgpu_format_to_vulkan,
};

mod system;
pub use system::{VRSystem, SessionState, FrameResources};

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use pollster::FutureExt;
    use openxr as xr;  // Used in tests

    struct TestContext {
        device: wgpu::Device,
        #[allow(dead_code)]
        queue: wgpu::Queue,
        #[allow(dead_code)]
        adapter: wgpu::Adapter,
    }

    impl TestContext {
        fn new() -> Option<Self> {
            let instance = wgpu::Instance::default();
            
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                })
                .block_on()?;

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .block_on()
                .ok()?;

            Some(Self {
                device,
                queue,
                adapter,
            })
        }
    }

    // Helper function to check VR runtime availability
    fn is_vr_runtime_available() -> bool {
        if let Ok(vr) = VRSystem::new() {
            vr.is_hmd_available()
        } else {
            false
        }
    }

    #[test]
    #[serial]
    fn test_vr_system_creation() -> Result<(), String> {
        if !is_vr_runtime_available() {
            println!("Test skipped: VR runtime not available");
            return Ok(());
        }

        match VRSystem::new() {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Failed to create VR system: {}", e);
                Err("VR system creation failed".to_string())
            }
        }
    }

    #[test]
    #[serial]
    fn test_hmd_availability() -> Result<(), String> {
        if !is_vr_runtime_available() {
            println!("Test skipped: VR runtime not available");
            return Ok(());
        }

        if let Ok(vr) = VRSystem::new() {
            let available = vr.is_hmd_available();
            println!("HMD available: {}", available);
            if !available {
                println!("HMD not detected - check if your VR headset is connected");
                return Err("HMD not available".to_string());
            }
            Ok(())
        } else {
            Err("Failed to create VR system".to_string())
        }
    }

    #[test]
    #[serial]
    fn test_view_configuration() -> Result<(), String> {
        if !is_vr_runtime_available() {
            println!("Test skipped: VR runtime not available");
            return Ok(());
        }

        if let Ok(vr) = VRSystem::new() {
            if let Err(e) = vr.get_view_configuration() {
                println!("Failed to get view configuration: {}", e);
                return Err("View configuration retrieval failed".to_string());
            }
            Ok(())
        } else {
            Err("Failed to create VR system".to_string())
        }
    }

    #[test]
    #[serial]
    fn test_swapchain_format() -> Result<(), String> {
        if !is_vr_runtime_available() {
            println!("Test skipped: VR runtime not available");
            return Ok(());
        }

        if let Ok(vr) = VRSystem::new() {
            let format = vr.get_swapchain_format();
            println!("Swapchain format: {:?}", format);
            Ok(())
        } else {
            Err("Failed to create VR system".to_string())
        }
    }

    #[test]
    #[serial]
    fn test_view_projection_creation() -> Result<(), String> {
        if !is_vr_runtime_available() {
            println!("Test skipped: VR runtime not available");
            return Ok(());
        }
        
        // Create test device using TestContext
        let context = match TestContext::new() {
            Some(context) => context,
            None => {
                println!("Skipping test 'test_view_projection_creation' - no suitable GPU adapter available");
                return Ok(());
            }
        };

        // Create VR system
        let mut vr = match VRSystem::new() {
            Ok(vr) => vr,
            Err(e) => {
                println!("Failed to create VR system: {}", e);
                return Err("VR system creation failed".to_string());
            }
        };

        // Create mock frame state
        let frame_state = xr::FrameState {
            predicted_display_time: xr::Time::from_nanos(0),
            predicted_display_period: xr::Duration::from_nanos(0),
            should_render: true,
        };

        // Initialize VR session
        if let Err(e) = vr.initialize_session(&context.device) {
            println!("Failed to initialize VR session: {}", e);
            return Err("Session initialization failed".to_string());
        }

        // Get view projections
        let view_projections = match vr.get_view_projections(&frame_state) {
            Ok(view_projections) => view_projections,
            Err(e) => {
                println!("Failed to get view projections: {}", e);
                return Err("View projection retrieval failed".to_string());
            }
        };

        if view_projections.len() != 2 {
            println!("Unexpected number of view projections: got {}, expected 2", view_projections.len());
            return Err("Invalid view projection count".to_string());
        }

        for (i, proj) in view_projections.iter().enumerate() {
            // Check view matrix orthogonality
            let view_transpose = proj.view.transpose();
            let identity = proj.view * view_transpose;
            let expected = if i == 0 { 1.0 } else { 0.0 };

            for i in 0..4 {
                for j in 0..4 {
                    if (identity.col(i)[j] - expected).abs() >= 0.001 {
                        println!("View matrix not orthogonal at position [{}, {}]", i, j);
                        return Err("Non-orthogonal view matrix".to_string());
                    }
                }
            }

            // Check projection matrix properties
            if proj.projection.w_axis.z >= 0.0 {
                println!("Invalid projection W.z: {}", proj.projection.w_axis.z);
                return Err("Invalid projection matrix W.z".to_string());
            }

            if proj.projection.z_axis.w != -1.0 {
                println!("Invalid projection Z.w: {}", proj.projection.z_axis.w);
                return Err("Invalid projection matrix Z.w".to_string());
            }
        }

        Ok(())
    }
} 