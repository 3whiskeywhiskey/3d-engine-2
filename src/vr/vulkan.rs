use wgpu;
use std::ffi::c_void;
use openxr as xr;
use anyhow::Result;
use ash::vk;
use std::mem::transmute;

pub fn wgpu_format_to_vulkan(format: wgpu::TextureFormat) -> u32 {
    match format {
        wgpu::TextureFormat::Bgra8UnormSrgb => 50,  // VK_FORMAT_B8G8R8A8_SRGB
        wgpu::TextureFormat::Rgba8UnormSrgb => 43,  // VK_FORMAT_R8G8B8A8_SRGB
        wgpu::TextureFormat::R8Unorm => 9,          // VK_FORMAT_R8_UNORM
        wgpu::TextureFormat::Rgba8Unorm => 37,      // VK_FORMAT_R8G8B8A8_UNORM
        wgpu::TextureFormat::Bgra8Unorm => 44,      // VK_FORMAT_B8G8R8A8_UNORM
        _ => panic!("Unsupported texture format"),
    }
}

pub struct VulkanContext {
    pub instance: *const c_void,
    pub physical_device: *const c_void,
    pub device: *const c_void,
}

pub fn create_vulkan_instance(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
) -> Result<*const c_void> {
    unsafe {
        log::warn!("Creating Vulkan instance");

        // Create Vulkan instance through OpenXR
        let mut app_info = vk::ApplicationInfo::default();
        app_info.api_version = vk::make_api_version(0, 1, 1, 0);  // Explicitly require Vulkan 1.1

        // Enable required extensions at instance level
        let extensions = [
            vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr(),
        ];

        log::warn!("Enabling instance extensions:");
        log::warn!("  KhrGetPhysicalDeviceProperties2: {:?}", 
            std::ffi::CStr::from_ptr(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr()));

        let mut create_info = vk::InstanceCreateInfo::default();
        create_info.p_application_info = &app_info;
        create_info.enabled_extension_count = extensions.len() as u32;
        create_info.pp_enabled_extension_names = extensions.as_ptr();

        let get_instance_proc_addr = transmute::<vk::PFN_vkGetInstanceProcAddr, 
            unsafe extern "system" fn(*const c_void, *const i8) -> Option<unsafe extern "system" fn()>>(get_instance_proc_addr);

        log::warn!("Creating Vulkan instance through OpenXR");
        let vk_instance = xr_instance
            .create_vulkan_instance(
                system,
                get_instance_proc_addr,
                &create_info as *const _ as *const _,
            )
            .map_err(|err| anyhow::anyhow!("Failed to create Vulkan instance: {}", err))?
            .map_err(|raw| anyhow::anyhow!("Vulkan error: {}", vk::Result::from_raw(raw)))?;

        log::warn!("Successfully created Vulkan instance");
        Ok(vk_instance as *const c_void)
    }
}

pub fn get_vulkan_physical_device(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    vk_instance: *const c_void,
) -> Result<*const c_void> {
    unsafe {
        let vk_physical_device = xr_instance
            .vulkan_graphics_device(system, vk_instance)
            .map_err(|err| anyhow::anyhow!("Failed to get Vulkan physical device: {}", err))?;

        Ok(vk_physical_device as *const c_void)
    }
}

pub fn create_vulkan_device(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    vk_instance: *const c_void,
    vk_physical_device: *const c_void,
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
) -> Result<(*const c_void, u32, u32)> {
    unsafe {
        // Set up queue info
        let queue_priorities = [1.0];
        let queue_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: 0,
            queue_count: 1,
            p_queue_priorities: queue_priorities.as_ptr(),
        };

        // Enable Vulkan 1.1 features including multiview
        let mut vulkan11_features = vk::PhysicalDeviceVulkan11Features {
            s_type: vk::StructureType::PHYSICAL_DEVICE_VULKAN_1_1_FEATURES,
            p_next: std::ptr::null_mut(),
            multiview: vk::TRUE,
            ..Default::default()
        };

        // Create device info with Vulkan 1.1 features
        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: &vulkan11_features as *const _ as *const c_void,
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_info,
            enabled_layer_count: 0,
            enabled_extension_count: 0,
            pp_enabled_extension_names: std::ptr::null(),
            pp_enabled_layer_names: std::ptr::null(),
            p_enabled_features: std::ptr::null(),
        };

        log::warn!("Creating Vulkan device through OpenXR");
        let vk_device = xr_instance.create_vulkan_device(
            system,
            transmute(get_instance_proc_addr),
            transmute(vk_physical_device),
            &device_create_info as *const _ as *const _,
        )?;

        match vk_device {
            Ok(device) => {
                log::warn!("Successfully created Vulkan device");
                Ok((device, 0, 0))
            },
            Err(err) => {
                log::error!("Failed to create Vulkan device: {}", err);
                Err(anyhow::anyhow!("Failed to create Vulkan device: {}", err))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_format_conversion() {
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Bgra8UnormSrgb), 50);
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Rgba8UnormSrgb), 43);
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::R8Unorm), 9);
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Rgba8Unorm), 37);
        assert_eq!(wgpu_format_to_vulkan(wgpu::TextureFormat::Bgra8Unorm), 44);
    }

    #[test]
    #[should_panic(expected = "Unsupported texture format")]
    fn test_unsupported_format() {
        wgpu_format_to_vulkan(wgpu::TextureFormat::R8Snorm);
    }
} 