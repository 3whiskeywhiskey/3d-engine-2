use openxr as xr;
use anyhow::Result;
use wgpu;

use super::math::ViewProjection;
use super::pipeline::{VRPipeline, VRUniform};
use super::vulkan::{
    get_vulkan_instance_from_wgpu,
    get_vulkan_physical_device_from_wgpu,
    get_vulkan_device_from_wgpu,
    get_vulkan_queue_info_from_wgpu,
    wgpu_format_to_vulkan,
};
use super::frame::{FrameManager, FrameResources};

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
    frame_manager: Option<FrameManager>,
    view_configuration: Option<xr::ViewConfigurationProperties>,
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
            frame_manager: None,
            view_configuration: None,
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

        let views = self.instance.enumerate_view_configuration_views(
            self.system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?;

        // Create reference space
        let stage = session.create_reference_space(
            xr::ReferenceSpaceType::STAGE,
            xr::Posef::IDENTITY,
        )?;

        // Create swapchain
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

        // Create pipeline
        self.pipeline = Some(VRPipeline::new(
            device,
            self.swapchain_format,
            wgpu::TextureFormat::Depth32Float,
        ));

        // Initialize frame manager
        let mut frame_manager = FrameManager::new();
        frame_manager.initialize(session, frame_waiter, frame_stream, swapchain, stage, views);
        self.frame_manager = Some(frame_manager);

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

    pub fn end_frame(&mut self, frame_state: xr::FrameState, views: &[xr::CompositionLayerProjectionView<xr::Vulkan>]) -> Result<()> {
        if let Some(frame_manager) = &mut self.frame_manager {
            frame_manager.end_frame(frame_state, views)
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

    pub fn update_session_state(&mut self) -> Result<()> {
        if let Some(frame_manager) = &self.frame_manager {
            let mut event_storage = xr::EventDataBuffer::new();
            while let Some(event) = self.instance.poll_event(&mut event_storage)? {
                match event {
                    xr::Event::SessionStateChanged(state_event) => {
                        match state_event.state() {
                            xr::SessionState::READY => {
                                // Begin session
                                if let Some(session) = frame_manager.get_session() {
                                    session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                                    self.session_state = SessionState::Ready;
                                }
                            }
                            xr::SessionState::STOPPING => {
                                // End session
                                if let Some(session) = frame_manager.get_session() {
                                    session.end()?;
                                    self.session_state = SessionState::Stopping;
                                }
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