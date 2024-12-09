use openxr as xr;
use anyhow::Result;
use wgpu;
use std::fmt;
use ash::vk::Format;

use super::math::ViewProjection;
use super::pipeline::{VRPipeline, VRUniform};
use super::vulkan::{
    create_vulkan_instance,
    get_vulkan_physical_device,
    create_vulkan_device,
};
use super::frame::{FrameManager, FrameResources};

const XR_TARGET_VERSION: xr::Version = xr::Version::new(1, 2, 0);

#[derive(Debug)]
pub enum SessionState {
    Idle,
    Ready,
    Visible,
    Focused {
        resources: FrameResources,
    },
    Running {
        resources: FrameResources,
    },
    Stopping,
    Stopped,
}

pub struct VRSystem {
    instance: xr::Instance,
    system: xr::SystemId,
    frame_manager: Option<FrameManager>,
    view_configuration: Option<xr::ViewConfigurationProperties>,
    views: Option<Vec<xr::ViewConfigurationView>>,
    swapchain_format: wgpu::TextureFormat,
    pipeline: Option<VRPipeline>,
    session_state: SessionState,
}

impl fmt::Debug for VRSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VRSystem")
            .field("system", &self.system)
            .field("swapchain_format", &self.swapchain_format)
            .field("session_state", &self.session_state)
            .finish()
    }
}

impl VRSystem {
    pub fn new() -> Result<Self> {
        // Create entry point
        let entry = xr::Entry::linked();

        // Required extensions for our application
        let mut required_extensions = xr::ExtensionSet::default();
        required_extensions.khr_vulkan_enable2 = true;  // Enable Vulkan 2 support
        required_extensions.khr_composition_layer_depth = true;  // Enable depth composition

        // Create instance (skip validation layers for now)
        let app_info = xr::ApplicationInfo {
            application_name: "WGPU 3D Viewer",
            application_version: 1,
            engine_name: "No Engine",
            engine_version: 1,
        };

        let instance = entry.create_instance(
            &app_info,
            &required_extensions,
            &[],  // No validation layers for now
        )?;

        // Get the system (HMD) with Vulkan graphics API
        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
        log::info!("Successfully created OpenXR system with HMD form factor");

        // Verify Vulkan support
        let supported_form_factor = instance.enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)?;
        log::info!("Supported environment blend modes: {:?}", supported_form_factor);
        
        if supported_form_factor.is_empty() {
            return Err(anyhow::anyhow!("System does not support PRIMARY_STEREO view configuration"));
        }

        // Get view configuration and views
        let views = match instance.enumerate_view_configuration_views(
            system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        ) {
            Ok(views) => {
                log::info!("View configuration views: {:?}", views);
                Some(views)
            },
            Err(e) => {
                log::error!("Failed to enumerate view configuration views: {:?}", e);
                return Err(e.into());
            }
        };

        Ok(Self {
            instance,
            system,
            frame_manager: None,
            view_configuration: None,
            views,
            swapchain_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            pipeline: None,
            session_state: SessionState::Idle,
        })
    }

    pub fn initialize_session(&mut self, _device: &wgpu::Device) -> Result<()> {
        // Get and validate graphics requirements
        let requirements = self.instance.graphics_requirements::<xr::Vulkan>(self.system)?;
        log::info!("OpenXR graphics requirements: {:?}", requirements);
        log::info!("Target Vulkan version: {}", XR_TARGET_VERSION);
        
        if requirements.min_api_version_supported > XR_TARGET_VERSION
            || requirements.max_api_version_supported.major() < XR_TARGET_VERSION.major()
        {
            return Err(anyhow::anyhow!(
                "OpenXR runtime requires Vulkan version > {}, < {}.0.0",
                requirements.min_api_version_supported,
                requirements.max_api_version_supported.major() + 1
            ));
        }

        // Create Vulkan instance through OpenXR
        log::info!("Creating Vulkan instance through OpenXR...");
        let vk_instance = create_vulkan_instance(&self.instance, self.system)?;
        log::info!("Successfully created Vulkan instance");

        // Get physical device through OpenXR
        log::info!("Getting Vulkan physical device through OpenXR...");
        let vk_physical_device = get_vulkan_physical_device(&self.instance, self.system, vk_instance)?;
        log::info!("Successfully got Vulkan physical device");

        // Create device through OpenXR
        log::info!("Creating Vulkan device through OpenXR...");
        let (vk_device, queue_family_index, queue_index) = create_vulkan_device(
            &self.instance,
            self.system,
            vk_instance,
            vk_physical_device,
        )?;
        log::info!("Successfully created Vulkan device");

        log::info!("Creating OpenXR session with Vulkan device info: queue_family={}, queue_index={}", 
            queue_family_index, queue_index);

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
            match self.instance.create_session::<xr::Vulkan>(self.system, &vk_session_create_info) {
                Ok(result) => {
                    log::info!("Successfully created OpenXR session");
                    result
                },
                Err(e) => {
                    log::error!("Failed to create OpenXR session: {:?}", e);
                    return Err(e.into());
                }
            }
        };

        // Initialize frame manager with session and wait for it to be ready
        let mut frame_manager = FrameManager::new();
        if let Some(views) = &self.views {
            frame_manager.initialize_session(session, frame_waiter, frame_stream, views.clone());
            self.frame_manager = Some(frame_manager);
        }

        // Get view configuration and views
        self.view_configuration = Some(match self.instance.view_configuration_properties(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        ) {
            Ok(config) => {
                log::info!("View configuration: {:?}", config);
                config
            },
            Err(e) => {
                log::error!("Failed to get view configuration: {:?}", e);
                return Err(e.into());
            }
        });

        // Wait for session to be ready and synchronized
        let mut session_state = xr::SessionState::UNKNOWN;
        let mut sync_attempt = 0;
        let max_sync_attempts = 50; // 5 seconds with 100ms sleep
        
        while session_state != xr::SessionState::SYNCHRONIZED && sync_attempt < max_sync_attempts {
            sync_attempt += 1;
            log::info!("Waiting for session synchronization (attempt {}/{})", sync_attempt, max_sync_attempts);
           
            let mut event_storage = xr::EventDataBuffer::new();
            while let Some(event) = self.instance.poll_event(&mut event_storage)? {
                match event {
                    xr::Event::SessionStateChanged(state_event) => {
                        session_state = state_event.state();
                        log::info!("Session state changed to: {:?}", session_state);
                        match session_state {
                            xr::SessionState::READY => {
                                log::info!("Session is ready, attempting to begin session...");
                                // Begin session when ready
                                if let Some(frame_manager) = &mut self.frame_manager {
                                    if let Some(session) = frame_manager.get_session() {
                                        session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                                        log::info!("Session begun successfully");

                                        // Create a reference space for head tracking
                                        let space = session.create_reference_space(
                                            xr::ReferenceSpaceType::VIEW,
                                            xr::Posef::IDENTITY,
                                        )?;
                                        log::info!("Successfully created reference space");

                                        // Create swapchain after beginning session
                                        if let Some(views) = &self.views {
                                            let swapchain_create_info = xr::SwapchainCreateInfo {
                                                create_flags: xr::SwapchainCreateFlags::EMPTY,
                                                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                                                    | xr::SwapchainUsageFlags::SAMPLED,
                                                format: Format::B8G8R8A8_SRGB.as_raw() as u32,
                                                sample_count: 1,
                                                width: views[0].recommended_image_rect_width,
                                                height: views[0].recommended_image_rect_height,
                                                face_count: 1,
                                                array_size: 1,
                                                mip_count: 1,
                                            };

                                            match session.create_swapchain(&swapchain_create_info) {
                                                Ok(swapchain) => {
                                                    log::info!("Successfully created swapchain");
                                                    frame_manager.initialize_resources(swapchain, space);

                                                    // Begin frame loop to establish timing
                                                    if let Ok(frame_state) = frame_manager.wait_frame() {
                                                        log::info!("Successfully waited for first frame");

                                                        // Get views and submit frame
                                                        let views = frame_manager.get_views(&frame_state)?;
                                                        frame_manager.submit_frame_with_projection(&frame_state, &views)?;
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to create swapchain: {:?}", e);
                                                    return Err(e.into());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            xr::SessionState::SYNCHRONIZED => {
                                log::info!("Session is synchronized");
                                // Only now do we consider the session fully initialized
                                sync_attempt = max_sync_attempts; // Exit the sync loop
                            }
                            xr::SessionState::VISIBLE => {
                                log::info!("Session is visible");
                            }
                            xr::SessionState::FOCUSED => {
                                log::info!("Session is now focused");
                            }
                            xr::SessionState::STOPPING => {
                                log::warn!("Session is stopping during initialization");
                            }
                            xr::SessionState::IDLE => {
                                log::info!("Session is idle");
                            }
                            xr::SessionState::EXITING => {
                                log::warn!("Session is exiting during initialization");
                            }
                            xr::SessionState::LOSS_PENDING => {
                                log::warn!("Session loss is pending during initialization");
                            }
                            other => {
                                log::info!("Unhandled session state during initialization: {:?}", other);
                            }
                        }
                    }
                    xr::Event::EventsLost(events_lost) => {
                        log::warn!("Lost {} events during synchronization", events_lost.lost_event_count());
                    }
                    xr::Event::InstanceLossPending(_) => {
                        log::error!("OpenXR instance loss pending during synchronization");
                        return Err(anyhow::anyhow!("OpenXR instance loss pending during synchronization"));
                    }
                    _ => {
                        log::info!("Received other event during synchronization");
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        if sync_attempt >= max_sync_attempts {
            log::error!("Failed to synchronize session after {} attempts", max_sync_attempts);
            return Err(anyhow::anyhow!("Session failed to synchronize after {} attempts", max_sync_attempts));
        }

        Ok(())
    }

    pub fn begin_frame(&mut self) -> Result<xr::FrameState> {
        if let Some(frame_manager) = &mut self.frame_manager {
            frame_manager.begin_frame()
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Result<u32> {
        if let Some(frame_manager) = &mut self.frame_manager {
            frame_manager.acquire_swapchain_image()
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn release_swapchain_image(&mut self) -> Result<()> {
        if let Some(frame_manager) = &mut self.frame_manager {
            frame_manager.release_swapchain_image()
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
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
        if let Some(frame_manager) = &self.frame_manager {
            frame_manager.get_views(frame_state)
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn get_view_projections(&mut self, frame_state: &xr::FrameState) -> Result<Vec<ViewProjection>> {
        if let Some(frame_manager) = &self.frame_manager {
            frame_manager.get_view_projections(frame_state)
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn get_swapchain_image_layout(&self) -> Option<(u32, u32)> {
        self.frame_manager.as_ref().and_then(|fm| fm.get_swapchain_image_layout())
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
                eye_position: [
                    view_proj.pose.position.x,
                    view_proj.pose.position.y,
                    view_proj.pose.position.z,
                ],
                _padding: 0,
            };
            pipeline.update_uniform(queue, &uniform);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipeline not initialized"))
        }
    }

    pub fn update(&mut self) -> Result<()> {
        if let Some(frame_manager) = &mut self.frame_manager {
            // Process all pending events
            let mut event_storage = xr::EventDataBuffer::new();
            while let Some(event) = self.instance.poll_event(&mut event_storage)? {
                match event {
                    xr::Event::SessionStateChanged(state_event) => {
                        let session_state = state_event.state();
                        log::info!("Session state changed to: {:?}", session_state);
                        match session_state {
                            xr::SessionState::READY => {
                                log::info!("Session is ready");
                                self.session_state = SessionState::Ready;
                            }
                            xr::SessionState::SYNCHRONIZED => {
                                log::info!("Session is synchronized");
                                // Wait for synchronization before proceeding
                                if let Ok(frame_state) = frame_manager.wait_frame() {
                                    if let Some(frame_stream) = frame_manager.get_frame_stream_mut() {
                                        frame_stream.begin().map_err(|e| anyhow::anyhow!("Failed to begin frame: {}", e))?;
                                    } else {
                                        return Err(anyhow::anyhow!("Frame stream not initialized"));
                                    }

                                    // Get initial view projections with proper timing
                                    let view_projections = frame_manager.get_view_projections(&frame_state)?;

                                    self.session_state = SessionState::Running {
                                        resources: FrameResources {
                                            frame_state,
                                            view_projections,
                                        }
                                    };
                                }
                            }
                            xr::SessionState::STOPPING => {
                                log::info!("Session is stopping");
                                // End session
                                if let Some(session) = frame_manager.get_session() {
                                    session.end()?;
                                }
                                self.session_state = SessionState::Stopping;
                            }
                            xr::SessionState::IDLE => {
                                log::info!("Session is idle");
                                self.session_state = SessionState::Idle;
                            }
                            xr::SessionState::EXITING => {
                                log::info!("Session is exiting");
                                if let Some(session) = frame_manager.get_session() {
                                    session.end()?;
                                }
                                self.session_state = SessionState::Stopped;
                            }
                            xr::SessionState::LOSS_PENDING => {
                                log::warn!("Session loss is pending");
                            }
                            xr::SessionState::VISIBLE => {
                                log::info!("Session is visible");
                                self.session_state = SessionState::Visible;
                            }
                            xr::SessionState::FOCUSED => {
                                log::info!("Session is focused");
                                // Only change application type after we're fully synchronized and focused
                                if let SessionState::Running { resources } = &self.session_state {
                                    self.session_state = SessionState::Focused {
                                        resources: resources.clone(),
                                    };
                                }
                            }
                            other => {
                                log::info!("Unhandled session state: {:?}", other);
                            }
                        }
                    }
                    xr::Event::InstanceLossPending(_) => {
                        log::warn!("OpenXR instance loss pending");
                    }
                    xr::Event::EventsLost(_) => {
                        log::warn!("Lost some OpenXR events");
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

    pub fn get_swapchain(&self) -> Result<&xr::Swapchain<xr::Vulkan>> {
        if let Some(frame_manager) = &self.frame_manager {
            frame_manager.get_swapchain()
                .ok_or_else(|| anyhow::anyhow!("Swapchain not initialized"))
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn get_swapchain_handle(&self) -> Result<xr::sys::Swapchain> {
        if let Some(frame_manager) = &self.frame_manager {
            if let Some(swapchain) = frame_manager.get_swapchain() {
                Ok(swapchain.as_raw())
            } else {
                Err(anyhow::anyhow!("Swapchain not initialized"))
            }
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn verify_swapchain(&self) -> Result<()> {
        if let Some(frame_manager) = &self.frame_manager {
            if frame_manager.get_swapchain().is_some() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Swapchain not initialized"))
            }
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }

    pub fn submit_frame(
        &mut self,
        frame_state: xr::FrameState,
        view_projections: &[ViewProjection],
        width: u32,
        height: u32,
    ) -> Result<()> {
        if let Some(frame_manager) = &mut self.frame_manager {
            frame_manager.submit_frame(frame_state, view_projections, width, height)
        } else {
            Err(anyhow::anyhow!("Frame manager not initialized"))
        }
    }
} 