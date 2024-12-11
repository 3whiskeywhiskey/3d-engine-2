use anyhow::Result;
use ash::vk::{self, Handle};
use openxr as xr;
use super::{VulkanContext, VrRenderer, ViewData};
use log::{info, debug};
use std::ffi::c_void;

pub struct VrSession {
    instance: xr::Instance,
    system: xr::SystemId,
    session: xr::Session<xr::Vulkan>,
    frame_waiter: xr::FrameWaiter,
    frame_stream: xr::FrameStream<xr::Vulkan>,
    stage: xr::Space,
    swapchain: xr::Swapchain<xr::Vulkan>,
    view_configs: Vec<xr::ViewConfigurationView>,
    vulkan: VulkanContext,
    renderer: VrRenderer,
}

impl VrSession {
    pub fn new() -> Result<Self> {
        // Create OpenXR instance
        info!("Creating OpenXR instance...");
        let entry = xr::Entry::linked();
        let app_info = xr::ApplicationInfo {
            application_name: "VR PoC",
            application_version: 1,
            engine_name: "No Engine",
            engine_version: 0,
        };

        // Create extension set with Vulkan support
        let mut extensions = xr::ExtensionSet::default();
        extensions.khr_vulkan_enable2 = true;
        extensions.ext_debug_utils = true;

        // Print available extensions
        let available_extensions = entry.enumerate_extensions()?;
        debug!("Available OpenXR extensions: {:?}", available_extensions);

        let instance = entry.create_instance(&app_info, &extensions, &[])?;
        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
        info!("OpenXR system created");

        // Get Vulkan requirements
        let reqs = instance.graphics_requirements::<xr::Vulkan>(system)?;
        debug!("Vulkan requirements: {:?}", reqs);

        // Get view configuration
        let view_configs = instance.enumerate_view_configuration_views(
            system,
            xr::ViewConfigurationType::PRIMARY_STEREO,
        )?;
        debug!("View configurations: {:?}", view_configs);

        // Create Vulkan instance
        info!("Creating Vulkan instance...");
        let vk_instance = unsafe {
            use std::ffi::CString;
            let app_name = CString::new("VR PoC").unwrap();
            let engine_name = CString::new("No Engine").unwrap();

            // Enable required extensions at instance level
            let instance_extensions = [
                vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr(),
                vk::KhrExternalMemoryCapabilitiesFn::name().as_ptr(),
                vk::KhrExternalFenceCapabilitiesFn::name().as_ptr(),
                vk::KhrExternalSemaphoreCapabilitiesFn::name().as_ptr(),
                vk::ExtDebugUtilsFn::name().as_ptr(),
            ];

            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(
                    &vk::ApplicationInfo::builder()
                        .application_name(&app_name)
                        .application_version(vk::make_api_version(0, 1, 0, 0))
                        .engine_name(&engine_name)
                        .engine_version(vk::make_api_version(0, 1, 0, 0))
                        .api_version(vk::API_VERSION_1_1)
                        .build()
                )
                .enabled_extension_names(&instance_extensions)
                .build();

            let vk_entry = ash::Entry::linked();
            let get_instance_proc_addr = vk_entry.static_fn().get_instance_proc_addr;
            let get_instance_proc_addr: unsafe extern "system" fn(
                instance: *const c_void,
                p_name: *const std::os::raw::c_char,
            ) -> vk::PFN_vkVoidFunction = unsafe { std::mem::transmute(get_instance_proc_addr) };

            instance.create_vulkan_instance(
                system,
                get_instance_proc_addr,
                &create_info as *const _ as *const _,
            )?.unwrap()
        };

        // Get the Vulkan physical device from OpenXR
        info!("Getting Vulkan physical device from OpenXR...");
        let physical_device = unsafe {
            instance.vulkan_graphics_device(system, vk_instance as _)?
        };
        debug!("Got physical device: {:?}", physical_device);

        // Create Vulkan device through OpenXR
        info!("Creating Vulkan device through OpenXR...");
        let device = unsafe {
            let device_extensions = [
                vk::KhrSwapchainFn::name().as_ptr(),
                vk::KhrMultiviewFn::name().as_ptr(),
                vk::KhrExternalMemoryFn::name().as_ptr(),
                vk::KhrExternalFenceFn::name().as_ptr(),
                vk::KhrExternalSemaphoreFn::name().as_ptr(),
            ];

            let mut features = vk::PhysicalDeviceFeatures::default();
            features.shader_storage_image_multisample = vk::TRUE;
            features.shader_storage_image_array_dynamic_indexing = vk::TRUE;
            features.shader_uniform_buffer_array_dynamic_indexing = vk::TRUE;

            let mut multiview_features = vk::PhysicalDeviceMultiviewFeatures::builder()
                .multiview(true)
                .build();

            let device_create_info = vk::DeviceCreateInfo::builder()
                .enabled_extension_names(&device_extensions)
                .enabled_features(&features)
                .push_next(&mut multiview_features)
                .build();

            let raw_device = instance.create_vulkan_device(
                system,
                vk_instance as _,
                physical_device as _,
                &device_create_info as *const _ as *const _,
            )?.unwrap();

            let vk_entry = ash::Entry::linked();
            let vk_instance = ash::Instance::load(
                vk_entry.static_fn(),
                vk::Instance::from_raw(vk_instance as u64),
            );

            ash::Device::load(
                vk_instance.fp_v1_0(),
                vk::Device::from_raw(raw_device as u64),
            )
        };

        // Create Vulkan context
        info!("Creating Vulkan context...");
        let vulkan = VulkanContext::new(vk_instance as *const _, physical_device as u64, device)?;

        // Create OpenXR session
        info!("Creating OpenXR session...");
        let (session, frame_waiter, frame_stream) = unsafe {
            instance.create_session::<xr::Vulkan>(
                system,
                &xr::vulkan::SessionCreateInfo {
                    instance: vk_instance as _,
                    physical_device: vulkan.physical_device.as_raw() as _,
                    device: vulkan.device.handle().as_raw() as _,
                    queue_family_index: vulkan.queue_family_index,
                    queue_index: 0,
                },
            )?
        };

        // Create stage space
        info!("Creating stage space...");
        let stage = session.create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)?;

        // Create swapchain
        info!("Creating swapchain...");
        let swapchain = session.create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT | xr::SwapchainUsageFlags::SAMPLED,
            format: vk::Format::B8G8R8A8_SRGB.as_raw() as _,
            sample_count: 1,
            width: view_configs[0].recommended_image_rect_width,
            height: view_configs[0].recommended_image_rect_height,
            face_count: 1,
            array_size: 2,
            mip_count: 1,
        })?;

        // Create renderer
        info!("Creating renderer...");
        let renderer = VrRenderer::new(&vulkan, vk::Format::B8G8R8A8_SRGB, view_configs[0].recommended_image_rect_width, view_configs[0].recommended_image_rect_height)?;

        info!("VR session created successfully");
        Ok(Self {
            instance,
            system,
            session,
            frame_waiter,
            frame_stream,
            stage,
            swapchain,
            view_configs,
            vulkan,
            renderer,
        })
    }

    pub fn render_frame(&mut self) -> Result<()> {
        // Wait for frame
        debug!("Waiting for frame...");
        let frame_state = self.frame_waiter.wait()?;
        self.frame_stream.begin()?;

        if !frame_state.should_render {
            debug!("Frame should not render, submitting empty frame");
            self.frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[],
            )?;
            return Ok(());
        }

        // Get view transforms
        debug!("Getting view transforms...");
        let (_view_flags, views) = self.session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            frame_state.predicted_display_time,
            &self.stage,
        )?;

        // Convert view transforms to matrices
        debug!("Converting view transforms to matrices...");
        let mut view_data = ViewData {
            view_matrices: [[0.0; 16]; 2],
            projection_matrices: [[0.0; 16]; 2],
        };

        for (i, view) in views.iter().enumerate() {
            // Convert pose to view matrix
            let position = view.pose.position;
            let orientation = view.pose.orientation;
            
            // Convert quaternion to rotation matrix
            let x = orientation.x;
            let y = orientation.y;
            let z = orientation.z;
            let w = orientation.w;
            
            let rotation = [
                [1.0 - 2.0*y*y - 2.0*z*z, 2.0*x*y - 2.0*w*z, 2.0*x*z + 2.0*w*y],
                [2.0*x*y + 2.0*w*z, 1.0 - 2.0*x*x - 2.0*z*z, 2.0*y*z - 2.0*w*x],
                [2.0*x*z - 2.0*w*y, 2.0*y*z + 2.0*w*x, 1.0 - 2.0*x*x - 2.0*y*y],
            ];

            // Combine rotation and translation into 4x4 matrix and flatten to array
            let view_matrix_4x4 = [
                [rotation[0][0], rotation[0][1], rotation[0][2], -(rotation[0][0]*position.x + rotation[0][1]*position.y + rotation[0][2]*position.z)],
                [rotation[1][0], rotation[1][1], rotation[1][2], -(rotation[1][0]*position.x + rotation[1][1]*position.y + rotation[1][2]*position.z)],
                [rotation[2][0], rotation[2][1], rotation[2][2], -(rotation[2][0]*position.x + rotation[2][1]*position.y + rotation[2][2]*position.z)],
                [0.0, 0.0, 0.0, 1.0],
            ];
            
            // Flatten 4x4 matrix to array
            view_data.view_matrices[i] = [
                view_matrix_4x4[0][0], view_matrix_4x4[0][1], view_matrix_4x4[0][2], view_matrix_4x4[0][3],
                view_matrix_4x4[1][0], view_matrix_4x4[1][1], view_matrix_4x4[1][2], view_matrix_4x4[1][3],
                view_matrix_4x4[2][0], view_matrix_4x4[2][1], view_matrix_4x4[2][2], view_matrix_4x4[2][3],
                view_matrix_4x4[3][0], view_matrix_4x4[3][1], view_matrix_4x4[3][2], view_matrix_4x4[3][3],
            ];

            // Convert fov to projection matrix
            let fov = view.fov;
            let near = 0.05;
            let far = 100.0;
            let tan_left = f32::tan(fov.angle_left);
            let tan_right = f32::tan(fov.angle_right);
            let tan_down = f32::tan(fov.angle_down);
            let tan_up = f32::tan(fov.angle_up);
            let x_scale = 2.0 / (tan_right - tan_left);
            let y_scale = 2.0 / (tan_up - tan_down);
            let x_offset = (tan_right + tan_left) * x_scale * 0.5;
            let y_offset = (tan_up + tan_down) * y_scale * 0.5;
            
            let projection_matrix_4x4 = [
                [x_scale, 0.0, x_offset, 0.0],
                [0.0, y_scale, y_offset, 0.0],
                [0.0, 0.0, far / (near - far), (far * near) / (near - far)],
                [0.0, 0.0, -1.0, 0.0],
            ];
            
            // Flatten 4x4 matrix to array
            view_data.projection_matrices[i] = [
                projection_matrix_4x4[0][0], projection_matrix_4x4[0][1], projection_matrix_4x4[0][2], projection_matrix_4x4[0][3],
                projection_matrix_4x4[1][0], projection_matrix_4x4[1][1], projection_matrix_4x4[1][2], projection_matrix_4x4[1][3],
                projection_matrix_4x4[2][0], projection_matrix_4x4[2][1], projection_matrix_4x4[2][2], projection_matrix_4x4[2][3],
                projection_matrix_4x4[3][0], projection_matrix_4x4[3][1], projection_matrix_4x4[3][2], projection_matrix_4x4[3][3],
            ];
        }

        // Update view matrices in renderer
        debug!("Updating view matrices in renderer...");
        self.renderer.update_view_matrices(&view_data)?;

        // Acquire swapchain image
        debug!("Acquiring swapchain image...");
        let image_index = self.swapchain.acquire_image()?;
        self.swapchain.wait_image(xr::Duration::INFINITE)?;

        // Create framebuffer and render
        debug!("Creating framebuffer and rendering...");
        let framebuffer = self.create_framebuffer(image_index)?;
        self.renderer.record_command_buffer(framebuffer, 
            self.view_configs[0].recommended_image_rect_width,
            self.view_configs[0].recommended_image_rect_height,
        )?;
        self.renderer.submit_commands(self.vulkan.queue)?;

        // Submit frame
        debug!("Submitting frame...");
        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.view_configs[0].recommended_image_rect_width as i32,
                height: self.view_configs[0].recommended_image_rect_height as i32,
            },
        };

        let projection_views = [
            xr::CompositionLayerProjectionView::new()
                .pose(views[0].pose)
                .fov(views[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(0)
                        .image_rect(rect),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(views[1].pose)
                .fov(views[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(1)
                        .image_rect(rect),
                ),
        ];

        let projection_layer = xr::CompositionLayerProjection::new()
            .layer_flags(xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA)
            .space(&self.stage)
            .views(&projection_views);

        self.frame_stream.end(
            frame_state.predicted_display_time,
            xr::EnvironmentBlendMode::OPAQUE,
            &[&projection_layer],
        )?;

        // Release swapchain image
        debug!("Releasing swapchain image...");
        self.swapchain.release_image()?;

        Ok(())
    }

    fn create_framebuffer(&self, image_index: u32) -> Result<vk::Framebuffer> {
        debug!("Enumerating swapchain images...");
        let swapchain_images = self.swapchain.enumerate_images()?;
        debug!("Creating image view for image {}...", image_index);
        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(unsafe { std::mem::transmute(swapchain_images[image_index as usize]) })
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(vk::Format::B8G8R8A8_SRGB)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 2,
            })
            .build();

        let image_view = unsafe {
            self.vulkan.device.create_image_view(&image_view_info, None)?
        };

        debug!("Creating framebuffer...");
        let framebuffer_info = vk::FramebufferCreateInfo::builder()
            .render_pass(self.renderer.get_render_pass())
            .attachments(&[image_view])
            .width(self.view_configs[0].recommended_image_rect_width)
            .height(self.view_configs[0].recommended_image_rect_height)
            .layers(2)
            .build();

        let framebuffer = unsafe {
            self.vulkan.device.create_framebuffer(&framebuffer_info, None)?
        };

        unsafe {
            self.vulkan.device.destroy_image_view(image_view, None);
        }

        Ok(framebuffer)
    }
}

impl Drop for VrSession {
    fn drop(&mut self) {
        info!("Cleaning up VR session");
        // Resources will be cleaned up automatically through their Drop implementations
    }
} 