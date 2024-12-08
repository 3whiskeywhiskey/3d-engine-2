use anyhow::Result;
use openxr as xr;
use wgpu;
use glam::{Mat4, Vec3, Quat};
use std::ffi::c_void;
use wgpu::hal::api::Vulkan;

mod pipeline;
use pipeline::{VRPipeline, VRUniform};

fn get_vulkan_instance_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let _vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the instance handle
            // TODO: Implement proper Vulkan instance extraction
            Ok(std::ptr::null())
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan instance"))?
    }
}

fn get_vulkan_physical_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let _vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the physical device handle
            // TODO: Implement proper Vulkan physical device extraction
            Ok(std::ptr::null())
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan physical device"))?
    }
}

fn get_vulkan_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let _vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the device handle
            // TODO: Implement proper Vulkan device extraction
            Ok(std::ptr::null())
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?
    }
}

fn get_vulkan_queue_info_from_wgpu(device: &wgpu::Device) -> Result<(u32, u32)> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<(u32, u32)>>(|vulkan_device| {
            let _vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // For now, we'll use the first queue family and queue
            // TODO: Get actual queue family and index from the queue
            Ok((0, 0))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan queue info"))?
    }
}

#[derive(Debug)]
pub struct ViewProjection {
    pub view: Mat4,
    pub projection: Mat4,
    pub fov: xr::Fovf,
    pub pose: xr::Posef,
}

#[derive(Debug)]
pub struct FrameResources {
    pub frame_state: xr::FrameState,
    pub view_projections: Vec<ViewProjection>,
}

#[derive(Debug)]
pub enum SessionState {
    Idle,
    Ready,
    Running {
        resources: FrameResources,
    },
    Stopping,
    Stopped,
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
    pipeline: Option<VRPipeline>,
    session_state: SessionState,
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
            swapchain_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            pipeline: None,
            session_state: SessionState::Idle,
        })
    }

    pub fn initialize_session(&mut self, device: &wgpu::Device) -> Result<()> {
        let _requirements = self.instance.graphics_requirements::<xr::Vulkan>(self.system)?;
        
        // Get Vulkan handles from wgpu
        let vk_instance = get_vulkan_instance_from_wgpu(device)?;
        let vk_physical_device = get_vulkan_physical_device_from_wgpu(device)?;
        let vk_device = get_vulkan_device_from_wgpu(device)?;
        let (queue_family_index, queue_index) = get_vulkan_queue_info_from_wgpu(device)?;

        // Create session with proper Vulkan device info
        let vk_session_create_info = xr::vulkan::SessionCreateInfo {
            instance: vk_instance,
            physical_device: vk_physical_device,
            device: vk_device,
            queue_family_index,
            queue_index,
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

        // Create pipeline
        self.pipeline = Some(VRPipeline::new(
            device,
            self.swapchain_format,
            wgpu::TextureFormat::Depth32Float,
        ));

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

    pub fn get_pipeline(&self) -> Option<&VRPipeline> {
        self.pipeline.as_ref()
    }

    pub fn update_view_uniforms(&self, queue: &wgpu::Queue, view_proj: &ViewProjection) -> Result<()> {
        if let Some(pipeline) = &self.pipeline {
            let uniform = VRUniform {
                view_proj: view_proj.projection.mul_mat4(&view_proj.view).to_cols_array_2d(),
                view: view_proj.view.to_cols_array_2d(),
                proj: view_proj.projection.to_cols_array_2d(),
            };
            pipeline.update_uniform(queue, &uniform);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipeline not initialized"))
        }
    }

    pub fn update_session_state(&mut self) -> Result<()> {
        if let Some(session) = &self.session {
            let mut event_storage = xr::EventDataBuffer::new();
            while let Some(event) = self.instance.poll_event(&mut event_storage)? {
                match event {
                    xr::Event::SessionStateChanged(state_event) => {
                        match state_event.state() {
                            xr::SessionState::READY => {
                                session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                                self.session_state = SessionState::Ready;
                            }
                            xr::SessionState::STOPPING => {
                                session.end()?;
                                self.session_state = SessionState::Stopping;
                            }
                            xr::SessionState::SYNCHRONIZED => {
                                let frame_state = xr::FrameState {
                                    predicted_display_time: xr::Time::from_nanos(0),
                                    predicted_display_period: xr::Duration::from_nanos(0),
                                    should_render: true,
                                };
                                self.session_state = SessionState::Running {
                                    resources: FrameResources {
                                        frame_state,
                                        view_projections: Vec::new(),
                                    },
                                };
                            }
                            xr::SessionState::IDLE => {
                                self.session_state = SessionState::Idle;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub fn is_session_running(&self) -> bool {
        matches!(self.session_state, SessionState::Running { .. })
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
    use pollster::FutureExt;

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

        if let Ok(mut vr) = VRSystem::new() {
            let format = vr.get_swapchain_format();
            if format != wgpu::TextureFormat::Bgra8UnormSrgb {
                println!("Unexpected default format: {:?}", format);
                return Err("Default format mismatch".to_string());
            }
            
            vr.set_swapchain_format(wgpu::TextureFormat::Rgba8UnormSrgb);
            let updated_format = vr.get_swapchain_format();
            if updated_format != wgpu::TextureFormat::Rgba8UnormSrgb {
                println!("Format not updated correctly: {:?}", updated_format);
                return Err("Format update failed".to_string());
            }
            Ok(())
        } else {
            Err("Failed to create VR system".to_string())
        }
    }

    #[test]
    #[serial]
    fn test_vulkan_format_conversion() -> Result<(), String> {
        let format = wgpu_format_to_vulkan(wgpu::TextureFormat::Bgra8UnormSrgb);
        if format != 50 {
            println!("Incorrect Vulkan format for BGRA8_SRGB: got {}, expected 50", format);
            return Err("BGRA8_SRGB format conversion failed".to_string());
        }

        let format = wgpu_format_to_vulkan(wgpu::TextureFormat::Rgba8UnormSrgb);
        if format != 43 {
            println!("Incorrect Vulkan format for RGBA8_SRGB: got {}, expected 43", format);
            return Err("RGBA8_SRGB format conversion failed".to_string());
        }

        let format = wgpu_format_to_vulkan(wgpu::TextureFormat::R8Unorm);
        if format != 50 {
            println!("Incorrect default Vulkan format: got {}, expected 50 (BGRA8_SRGB)", format);
            return Err("Default format conversion failed".to_string());
        }
        Ok(())
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