use anyhow::Result;
use openxr as xr;
use wgpu::TextureFormat;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use ash::vk::Handle as VkHandle;

use super::vulkan::{
    create_vulkan_instance,
    get_vulkan_physical_device,
    create_vulkan_device,
    wgpu_format_to_vulkan,
};

use super::pipeline::VRPipeline;
use super::math::ViewProjection;

pub struct VRSystem {
    pub instance: xr::Instance,
    pub system: xr::SystemId,
    pub session: xr::Session<xr::Vulkan>,
    pub frame_wait: xr::FrameWaiter,
    pub frame_stream: xr::FrameStream<xr::Vulkan>,
    pub swapchain: xr::Swapchain<xr::Vulkan>,
    pub swapchain_resolution: (u32, u32),
    pub reference_space: xr::Space,
    pub stage_space: xr::Space,
    pub view_configuration_views: Vec<xr::ViewConfigurationView>,
    pub pipeline: Option<VRPipeline>,
    swapchain_format: TextureFormat,
}

impl VRSystem {
    pub fn new() -> Result<Self> {
        // Initialize logging if not already done
        env_logger::try_init().ok();
        log::warn!("Initializing VR System");

        // Create OpenXR instance
        let xr_entry = xr::Entry::linked();

        // Check available extensions
        let available_extensions = xr_entry.enumerate_extensions()
            .map_err(|err| anyhow::anyhow!("Failed to enumerate OpenXR extensions: {}", err))?;

        let mut enabled_extensions = xr::ExtensionSet::default();
        enabled_extensions.khr_vulkan_enable2 = available_extensions.khr_vulkan_enable2;

        if !enabled_extensions.khr_vulkan_enable2 {
            log::warn!("OpenXR Vulkan support not available");
            return Err(anyhow::anyhow!("OpenXR Vulkan support not available"));
        }

        let app_info = xr::ApplicationInfo {
            application_name: "wgpu-3d-viewer",
            application_version: 0,
            engine_name: "wgpu-3d-viewer",
            engine_version: 0,
        };

        let instance = xr_entry
            .create_instance(&app_info, &enabled_extensions, &[])
            .map_err(|err| anyhow::anyhow!("Failed to create OpenXR instance: {}", err))?;

        // Get system ID
        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|err| anyhow::anyhow!("Failed to get system ID: {}", err))?;

        // Check for opaque display support
        if !instance
            .enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .unwrap_or_default()
            .iter()
            .any(|&blend_mode| blend_mode == xr::EnvironmentBlendMode::OPAQUE)
        {
            return Err(anyhow::anyhow!("OpenXR opaque blend mode not supported"));
        }

        // Check Vulkan requirements
        let xr::vulkan::Requirements {
            max_api_version_supported,
            min_api_version_supported,
        } = instance
            .graphics_requirements::<xr::Vulkan>(system)
            .map_err(|err| anyhow::anyhow!("Failed to get Vulkan requirements: {}", err))?;

        // Get view configuration
        let view_configuration_views = instance
            .enumerate_view_configuration_views(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|err| anyhow::anyhow!("Failed to get view configuration: {}", err))?;

        // Create Vulkan instance
        let vk_entry = unsafe { ash::Entry::load() }.map_err(|err| {
            anyhow::anyhow!("Failed to load Vulkan entry point: {}", err)
        })?;

        let get_instance_proc_addr = vk_entry.static_fn().get_instance_proc_addr;

        let vk_instance = create_vulkan_instance(&instance, system, get_instance_proc_addr)?;

        // Get physical device
        let vk_physical_device = get_vulkan_physical_device(&instance, system, vk_instance)?;

        // Log device capabilities for multiview
        // unsafe {
        //     let vk_instance_handle = std::mem::transmute::<*const std::ffi::c_void, ash::vk::Instance>(vk_instance);
        //     let vk_physical_device_handle = std::mem::transmute::<*const std::ffi::c_void, ash::vk::PhysicalDevice>(vk_physical_device);
            
        //     let instance_raw = ash::Instance::load(vk_entry.static_fn(), vk_instance_handle);
        //     let mut multiview_features = ash::vk::PhysicalDeviceMultiviewFeatures::default();
        //     let mut features2 = ash::vk::PhysicalDeviceFeatures2::default();
        //     features2.p_next = &mut multiview_features as *mut _ as *mut std::ffi::c_void;
        //     instance_raw.get_physical_device_features2(
        //         vk_physical_device_handle,
        //         &mut features2
        //     );
        //     log::warn!("VR Device Capabilities:");
        //     log::warn!("  Multiview support: {}", multiview_features.multiview != 0);
        //     log::warn!("  Multiview geometry shader: {}", multiview_features.multiview_geometry_shader != 0);
        //     log::warn!("  Multiview tessellation shader: {}", multiview_features.multiview_tessellation_shader != 0);
        // }

        // Create logical device
        let (vk_device, queue_family_index, queue_index) = create_vulkan_device(
            &instance,
            system,
            vk_instance,
            vk_physical_device,
            get_instance_proc_addr,
        )?;

        // Create session
        let (session, frame_wait, frame_stream) = unsafe {
            instance
                .create_session::<xr::Vulkan>(
                    system,
                    &xr::vulkan::SessionCreateInfo {
                        instance: vk_instance as _,
                        physical_device: vk_physical_device as _,
                        device: vk_device as _,
                        queue_family_index,
                        queue_index,
                    },
                )
                .map_err(|err| anyhow::anyhow!("Failed to create session: {}", err))?
        };

        // Create reference space
        let reference_space = session
            .create_reference_space(
                xr::ReferenceSpaceType::LOCAL,
                xr::Posef::IDENTITY,
            )
            .map_err(|err| anyhow::anyhow!("Failed to create reference space: {}", err))?;

        // Create stage space
        let stage_space = session
            .create_reference_space(
                xr::ReferenceSpaceType::STAGE,
                xr::Posef::IDENTITY,
            )
            .map_err(|err| anyhow::anyhow!("Failed to create stage space: {}", err))?;

        // Create swapchain
        let swapchain_formats = session
            .enumerate_swapchain_formats()
            .map_err(|err| anyhow::anyhow!("Failed to get swapchain formats: {}", err))?;

        let color_format = TextureFormat::Bgra8UnormSrgb;
        let color_format_vulkan = wgpu_format_to_vulkan(color_format);

        if !swapchain_formats.contains(&color_format_vulkan) {
            return Err(anyhow::anyhow!("Swapchain format not supported"));
        }

        let resolution = view_configuration_views[0];
        let swapchain = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: color_format_vulkan,
                sample_count: 1,
                width: resolution.recommended_image_rect_width,
                height: resolution.recommended_image_rect_height,
                face_count: 1,
                array_size: 2,
                mip_count: 1,
            })
            .map_err(|err| anyhow::anyhow!("Failed to create swapchain: {}", err))?;

        Ok(Self {
            instance,
            system,
            session,
            frame_wait,
            frame_stream,
            swapchain,
            swapchain_resolution: (
                resolution.recommended_image_rect_width,
                resolution.recommended_image_rect_height,
            ),
            reference_space,
            stage_space,
            view_configuration_views: vec![resolution],
            pipeline: None,
            swapchain_format: color_format,
        })
    }

    pub fn initialize_session(&mut self, device: &wgpu::Device) -> Result<()> {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.swapchain_format,
            width: self.swapchain_resolution.0,
            height: self.swapchain_resolution.1,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.pipeline = Some(VRPipeline::new(device, &config));
        Ok(())
    }

    pub fn begin_frame(&mut self) -> Result<xr::FrameState> {
        // Check if session is ready
        let mut event_storage = xr::EventDataBuffer::new();
        while let Some(event) = self.instance.poll_event(&mut event_storage)? {
            match event {
                xr::Event::SessionStateChanged(state_event) => {
                    let session_state = state_event.state();
                    log::info!("Session state changed to: {:?}", session_state);
                    match session_state {
                        xr::SessionState::READY => {
                            self.session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                        }
                        xr::SessionState::STOPPING => {
                            self.session.end()?;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            return Err(anyhow::anyhow!("VR session is exiting"));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let frame_state = self.frame_wait
            .wait()
            .map_err(|err| anyhow::anyhow!("Failed to wait for frame: {}", err))?;

        self.frame_stream
            .begin()
            .map_err(|err| anyhow::anyhow!("Failed to begin frame: {}", err))?;

        Ok(frame_state)
    }

    pub fn acquire_swapchain_image(&mut self) -> Result<u32> {
        let image_index = self.swapchain
            .acquire_image()
            .map_err(|err| anyhow::anyhow!("Failed to acquire swapchain image: {}", err))?;
        Ok(image_index)
    }

    pub fn wait_swapchain_image(&mut self) -> Result<()> {
        self.swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|err| anyhow::anyhow!("Failed to wait for swapchain image: {}", err))?;
        Ok(())
    }

    pub fn release_swapchain_image(&mut self) -> Result<()> {
        self.swapchain
            .release_image()
            .map_err(|err| anyhow::anyhow!("Failed to release swapchain image: {}", err))?;
        Ok(())
    }

    pub fn get_view_projections(&mut self, frame_state: &xr::FrameState) -> Result<Vec<ViewProjection>> {
        let (_, views) = self.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            frame_state.predicted_display_time,
            &self.reference_space,
        ).map_err(|err| anyhow::anyhow!("Failed to locate views: {}", err))?;

        let mut view_projections = Vec::new();
        for view in views.into_iter() {
            let fov = view.fov;
            let projection = Mat4::perspective_infinite_rh(
                fov.angle_right - fov.angle_left,
                fov.angle_up - fov.angle_down,
                0.05,
            );

            // Convert OpenXR pose to view matrix
            let orientation = view.pose.orientation;
            let position = view.pose.position;
            let rotation = Mat4::from_quat(Quat::from_xyzw(
                orientation.x,
                orientation.y,
                orientation.z,
                orientation.w,
            ));
            let translation = Mat4::from_translation(Vec3::new(
                position.x,
                position.y,
                position.z,
            ));
            let view_matrix = (rotation * translation).inverse();
            
            view_projections.push(ViewProjection {
                view: view_matrix,
                projection,
                pose: view.pose,
                fov,
            });
        }

        Ok(view_projections)
    }

    pub fn get_swapchain_image_layout(&self) -> Option<(u32, u32)> {
        Some(self.swapchain_resolution)
    }

    pub fn get_pipeline(&self) -> Option<&VRPipeline> {
        self.pipeline.as_ref()
    }

    pub fn submit_frame(&mut self, frame_state: xr::FrameState, view_projections: &[ViewProjection], width: u32, height: u32) -> Result<()> {
        // Create projection views
        let projection_views = [
            xr::CompositionLayerProjectionView::new()
                .pose(view_projections[0].pose)
                .fov(view_projections[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(0)
                        .image_rect(xr::Rect2Di {
                            offset: xr::Offset2Di { x: 0, y: 0 },
                            extent: xr::Extent2Di {
                                width: width as i32,
                                height: height as i32,
                            },
                        }),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(view_projections[1].pose)
                .fov(view_projections[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(1)
                        .image_rect(xr::Rect2Di {
                            offset: xr::Offset2Di { x: 0, y: 0 },
                            extent: xr::Extent2Di {
                                width: width as i32,
                                height: height as i32,
                            },
                        }),
                ),
        ];

        // Create layer for stereo rendering
        let layer = xr::CompositionLayerProjection::new()
            .space(&self.stage_space)
            .views(&projection_views);

        // End frame with the layer
        self.frame_stream
            .end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[&layer],
            )
            .map_err(|err| anyhow::anyhow!("Failed to submit frame: {}", err))?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        let mut event_storage = xr::EventDataBuffer::new();
        while let Some(event) = self.instance.poll_event(&mut event_storage)? {
            match event {
                xr::Event::SessionStateChanged(state_event) => {
                    let session_state = state_event.state();
                    log::info!("Session state changed to: {:?}", session_state);
                    match session_state {
                        xr::SessionState::READY => {
                            self.session.begin(xr::ViewConfigurationType::PRIMARY_STEREO)?;
                        }
                        xr::SessionState::STOPPING => {
                            self.session.end()?;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
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
        self.instance
            .view_configuration_properties(self.system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|err| anyhow::anyhow!("Failed to get view configuration: {}", err))
    }

    pub fn get_swapchain_format(&self) -> TextureFormat {
        self.swapchain_format
    }
} 