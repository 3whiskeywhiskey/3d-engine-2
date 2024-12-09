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
        // Create Vulkan instance through OpenXR
        let mut app_info = vk::ApplicationInfo::default();
        app_info.api_version = vk::make_api_version(0, 1, 2, 0);

        let mut create_info = vk::InstanceCreateInfo::default();
        create_info.p_application_info = &app_info;

        let get_instance_proc_addr = transmute::<vk::PFN_vkGetInstanceProcAddr, unsafe extern "system" fn(*const c_void, *const i8) -> Option<unsafe extern "system" fn()>>(get_instance_proc_addr);

        let vk_instance = xr_instance
            .create_vulkan_instance(
                system,
                get_instance_proc_addr,
                &create_info as *const _ as *const _,
            )
            .map_err(|err| anyhow::anyhow!("Failed to create Vulkan instance: {}", err))?
            .map_err(|raw| anyhow::anyhow!("Vulkan error: {}", vk::Result::from_raw(raw)))?;

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
        // Create device through OpenXR
        let queue_priorities = [1.0];
        let mut queue_create_info = vk::DeviceQueueCreateInfo::default();
        queue_create_info.queue_family_index = 0;
        queue_create_info.p_queue_priorities = queue_priorities.as_ptr();
        queue_create_info.queue_count = 1;

        // Enable Vulkan 1.1 features including multiview
        let mut features_v1_1 = vk::PhysicalDeviceVulkan11Features::default();
        features_v1_1.multiview = vk::TRUE;

        // Enable base Vulkan features
        let mut features = vk::PhysicalDeviceFeatures2::default();
        features.p_next = &features_v1_1 as *const _ as *mut c_void;

        // Create device info
        let mut device_create_info = vk::DeviceCreateInfo::default();
        device_create_info.queue_create_info_count = 1;
        device_create_info.p_queue_create_infos = &queue_create_info;
        device_create_info.p_next = &features as *const _ as *const c_void;

        // Enable required extensions
        let extensions = [
            vk::KhrMultiviewFn::name().as_ptr(),
            vk::KhrMaintenance1Fn::name().as_ptr(),
        ];
        device_create_info.enabled_extension_count = extensions.len() as u32;
        device_create_info.pp_enabled_extension_names = extensions.as_ptr();

        let get_instance_proc_addr = transmute::<vk::PFN_vkGetInstanceProcAddr, unsafe extern "system" fn(*const c_void, *const i8) -> Option<unsafe extern "system" fn()>>(get_instance_proc_addr);

        let vk_device = xr_instance
            .create_vulkan_device(
                system,
                get_instance_proc_addr,
                vk_physical_device,
                &device_create_info as *const _ as *const _,
            )
            .map_err(|err| anyhow::anyhow!("Failed to create Vulkan device: {}", err))?
            .map_err(|raw| anyhow::anyhow!("Vulkan error: {}", vk::Result::from_raw(raw)))?;

        Ok((vk_device as *const c_void, 0, 0))
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