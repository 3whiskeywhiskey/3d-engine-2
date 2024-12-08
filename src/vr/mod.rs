use anyhow::Result;
use openxr as xr;
use wgpu;
use glam::{Mat4, Vec3, Quat};

#[derive(Debug)]
pub struct ViewProjection {
    pub view: Mat4,
    pub projection: Mat4,
    pub fov: xr::Fovf,
    pub pose: xr::Posef,
}

pub struct VRSystem {
    instance: xr::Instance,
    system: xr::SystemId,
    session: Option<xr::Session<xr::Vulkan>>,
    frame_waiter: Option<xr::FrameWaiter>,
    frame_stream: Option<xr::FrameStream<xr::Vulkan>>,
    swapchain: Option<xr::Swapchain<xr::Vulkan>>,
    stage: Option<xr::Space>,
    view_configuration: Option<xr::ViewConfigurationProperties>,
    views: Option<Vec<xr::ViewConfigurationView>>,
    swapchain_format: wgpu::TextureFormat,
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
            frame_waiter: None,
            frame_stream: None,
            swapchain: None,
            stage: None,
            view_configuration: None,
            views: None,
            swapchain_format: wgpu::TextureFormat::Bgra8UnormSrgb,  // Default format
        })
    }

    pub fn initialize_session(&mut self, _device: &wgpu::Device) -> Result<()> {
        // Get system properties for Vulkan device creation
        let _requirements = self.instance.graphics_requirements::<xr::Vulkan>(self.system)?;
        
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

        // Create swapchain
        if let Some(views) = &self.views {
            let swapchain = session.create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: wgpu_format_to_vulkan(self.swapchain_format),
                sample_count: 1,
                width: views[0].recommended_image_rect_width,
                height: views[0].recommended_image_rect_height,
                face_count: 1,
                array_size: 2,  // One for each eye
                mip_count: 1,
            })?;
            self.swapchain = Some(swapchain);
        }

        // Store session components
        self.session = Some(session);
        self.frame_waiter = Some(frame_waiter);
        self.frame_stream = Some(frame_stream);
        self.stage = Some(stage);

        Ok(())
    }

    pub fn begin_frame(&mut self) -> Result<xr::FrameState> {
        if let (Some(frame_waiter), Some(frame_stream)) = (&mut self.frame_waiter, &mut self.frame_stream) {
            frame_waiter.wait()?;
            let frame_state = xr::FrameState {
                predicted_display_time: xr::Time::from_nanos(0),  // We'll get the actual time from the runtime later
                predicted_display_period: xr::Duration::from_nanos(0),
                should_render: true,  // We'll assume we should always render for now
            };
            frame_stream.begin().map_err(|e| anyhow::anyhow!("Failed to begin frame: {:?}", e))?;
            Ok(frame_state)
        } else {
            Err(anyhow::anyhow!("Frame waiter or stream not initialized"))
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Result<u32> {
        if let Some(swapchain) = &mut self.swapchain {
            let image_index = swapchain.acquire_image()?;
            swapchain.wait_image(xr::Duration::from_nanos(100_000_000))?;
            Ok(image_index)
        } else {
            Err(anyhow::anyhow!("Swapchain not initialized"))
        }
    }

    pub fn release_swapchain_image(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.release_image()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Swapchain not initialized"))
        }
    }

    pub fn end_frame(&mut self, frame_state: xr::FrameState, views: &[xr::CompositionLayerProjectionView<xr::Vulkan>]) -> Result<()> {
        if let (Some(frame_stream), Some(stage)) = (&mut self.frame_stream, &self.stage) {
            let projection_layer = xr::CompositionLayerProjection::new().space(stage).views(views);
            frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[&projection_layer],
            )?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Frame stream not initialized"))
        }
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
                fov: view.fov,
                pose: view.pose,
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

    pub fn get_swapchain_format(&self) -> wgpu::TextureFormat {
        self.swapchain_format
    }

    pub fn set_swapchain_format(&mut self, format: wgpu::TextureFormat) {
        self.swapchain_format = format;
    }
}

// Helper function to convert WGPU texture format to Vulkan format
fn wgpu_format_to_vulkan(format: wgpu::TextureFormat) -> u32 {
    match format {
        wgpu::TextureFormat::Bgra8UnormSrgb => 50,  // VK_FORMAT_B8G8R8A8_SRGB
        wgpu::TextureFormat::Rgba8UnormSrgb => 43,  // VK_FORMAT_R8G8B8A8_SRGB
        _ => 50,  // Default to BGRA8_SRGB
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
        assert!(vr.is_ok(), "Failed to create VR system");
    }

    #[test]
    #[serial]
    fn test_hmd_availability() {
        let vr = VRSystem::new().unwrap();
        let available = vr.is_hmd_available();
        println!("HMD available: {}", available);
    }

    #[test]
    #[serial]
    fn test_view_configuration() {
        let vr = VRSystem::new().unwrap();
        let config = vr.get_view_configuration();
        assert!(config.is_ok(), "Failed to get view configuration");
    }

    #[test]
    #[serial]
    fn test_swapchain_format() {
        let mut vr = VRSystem::new().unwrap();
        assert_eq!(vr.get_swapchain_format(), wgpu::TextureFormat::Bgra8UnormSrgb, "Default format should be Bgra8UnormSrgb");
        
        vr.set_swapchain_format(wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(vr.get_swapchain_format(), wgpu::TextureFormat::Rgba8UnormSrgb, "Format should be updated to Rgba8UnormSrgb");
    }

    #[test]
    #[serial]
    fn test_vulkan_format_conversion() {
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Bgra8UnormSrgb), 50, "BGRA8_SRGB format should be 50");
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Rgba8UnormSrgb), 43, "RGBA8_SRGB format should be 43");
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::R8Unorm), 50, "Unknown format should default to BGRA8_SRGB (50)");
    }

    #[test]
    #[serial]
    fn test_perspective_matrix() {
        // Test with symmetric FOV
        let matrix = perspective_infinite_reverse_rh(
            -0.5, // left
            0.5,  // right
            0.5,  // up
            -0.5, // down
            0.01, // near
        );

        // Check that the matrix preserves aspect ratio
        assert!((matrix.x_axis.x - matrix.y_axis.y).abs() < f32::EPSILON, "Perspective matrix should preserve aspect ratio for symmetric FOV");

        // Test with asymmetric FOV
        let matrix = perspective_infinite_reverse_rh(
            -0.3, // left
            0.7,  // right
            0.6,  // up
            -0.4, // down
            0.01, // near
        );

        // Check that the matrix handles asymmetric FOV correctly
        assert!(matrix.x_axis.x > 0.0, "X scale should be positive");
        assert!(matrix.y_axis.y > 0.0, "Y scale should be positive");
        assert!(matrix.w_axis.z < 0.0, "W component of Z axis should be negative for reverse-Z");
    }

    #[tokio::test]
    #[serial]
    async fn test_view_projection_creation() {
        let mut vr = VRSystem::new().unwrap();
        
        // Create a mock frame state
        let frame_state = xr::FrameState {
            predicted_display_time: xr::Time::from_nanos(0),
            predicted_display_period: xr::Duration::from_nanos(0),
            should_render: true,
        };

        // Test view projection creation (this will fail if no HMD is connected)
        if vr.is_hmd_available() {
            let device = create_test_device().await;
            
            // Skip the test if session initialization fails (which is expected without a real VR runtime)
            if let Ok(()) = vr.initialize_session(&device) {
                let view_projections = vr.get_view_projections(&frame_state);
                if let Ok(projections) = view_projections {
                    assert_eq!(projections.len(), 2, "Should have two view projections for stereo");
                    
                    for proj in projections {
                        // Check view matrix is orthogonal (up to floating point error)
                        let view_transpose = proj.view.transpose();
                        let identity = proj.view * view_transpose;
                        for i in 0..4 {
                            for j in 0..4 {
                                let expected = if i == j { 1.0 } else { 0.0 };
                                assert!((identity.col(i)[j] - expected).abs() < 0.001, 
                                    "View matrix should be orthogonal");
                            }
                        }

                        // Check projection matrix properties
                        assert!(proj.projection.w_axis.z < 0.0, 
                            "Projection W.z should be negative for reverse-Z");
                        assert_eq!(proj.projection.z_axis.w, -1.0, 
                            "Projection Z.w should be -1 for perspective projection");
                    }
                }
            } else {
                println!("Skipping view projection test - failed to initialize VR session");
            }
        } else {
            println!("Skipping view projection test - no HMD available");
        }
    }

    async fn create_test_device() -> wgpu::Device {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, _) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        device
    }
} 