use wgpu;
use std::ffi::c_void;
use openxr as xr;
use anyhow::Result;
use ash::vk;

pub fn create_vulkan_instance(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<*const c_void> {
    unsafe {
        // Create Vulkan entry point
        let vk_entry = ash::Entry::load().map_err(|err| {
            anyhow::anyhow!("Failed to load Vulkan entry point: {}", err)
        })?;

        // Get instance proc addr function
        let get_instance_proc_addr = {
            type Fn<T> = unsafe extern "system" fn(T, *const i8) -> Option<unsafe extern "system" fn()>;
            type AshFn = Fn<vk::Instance>;
            type OpenXrFn = Fn<*const c_void>;
            std::mem::transmute::<AshFn, OpenXrFn>(vk_entry.static_fn().get_instance_proc_addr)
        };

        // Create Vulkan instance through OpenXR
        let mut app_info = vk::ApplicationInfo::default();
        app_info.api_version = vk::make_api_version(0, 1, 2, 0);

        let mut create_info = vk::InstanceCreateInfo::default();
        create_info.p_application_info = &app_info;

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
    _vk_instance: *const c_void,
) -> Result<*const c_void> {
    unsafe {
        let vk_physical_device = xr_instance
            .vulkan_graphics_device(system, _vk_instance)
            .map_err(|err| anyhow::anyhow!("Failed to get Vulkan physical device: {}", err))?;

        Ok(vk_physical_device as *const c_void)
    }
}

pub fn create_vulkan_device(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    _vk_instance: *const c_void,
    vk_physical_device: *const c_void,
) -> Result<(*const c_void, u32, u32)> {
    unsafe {
        // Get instance proc addr function
        let vk_entry = ash::Entry::load().map_err(|err| {
            anyhow::anyhow!("Failed to load Vulkan entry point: {}", err)
        })?;

        let get_instance_proc_addr = {
            type Fn<T> = unsafe extern "system" fn(T, *const i8) -> Option<unsafe extern "system" fn()>;
            type AshFn = Fn<vk::Instance>;
            type OpenXrFn = Fn<*const c_void>;
            std::mem::transmute::<AshFn, OpenXrFn>(vk_entry.static_fn().get_instance_proc_addr)
        };

        // Create device through OpenXR
        let queue_priorities = [1.0];
        let mut queue_create_info = vk::DeviceQueueCreateInfo::default();
        queue_create_info.queue_family_index = 0;
        queue_create_info.p_queue_priorities = queue_priorities.as_ptr();
        queue_create_info.queue_count = 1;

        let mut device_create_info = vk::DeviceCreateInfo::default();
        device_create_info.queue_create_info_count = 1;
        device_create_info.p_queue_create_infos = &queue_create_info;

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

/// Convert WGPU texture format to Vulkan format
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

#[cfg(test)]
mod tests {
    use super::*;
    //use pollster::FutureExt;

    // fn create_test_device() -> Option<wgpu::Device> {
    //     let instance = wgpu::Instance::default();
        
    //     let adapter = instance
    //         .request_adapter(&wgpu::RequestAdapterOptions {
    //             power_preference: wgpu::PowerPreference::LowPower,
    //             force_fallback_adapter: true,
    //             compatible_surface: None,
    //         })
    //         .block_on()?;

    //     let (device, _) = adapter
    //         .request_device(
    //             &wgpu::DeviceDescriptor {
    //                 label: None,
    //                 required_features: wgpu::Features::empty(),
    //                 required_limits: wgpu::Limits::downlevel_defaults(),
    //                 memory_hints: Default::default(),
    //             },
    //             None,
    //         )
    //         .block_on()
    //         .ok()?;

    //     Some(device)
    // }

    // #[test]
    // fn test_vulkan_handle_extraction() {
    //     if let Some(device) = create_test_device() {
    //         // Test instance extraction
    //         let instance = get_vulkan_instance_from_wgpu(&device);
    //         assert!(instance.is_ok(), "Failed to get Vulkan instance");
    //         assert!(!instance.unwrap().is_null(), "Vulkan instance is null");

    //         // Test physical device extraction
    //         let physical_device = get_vulkan_physical_device_from_wgpu(&device);
    //         assert!(physical_device.is_ok(), "Failed to get Vulkan physical device");
    //         assert!(!physical_device.unwrap().is_null(), "Vulkan physical device is null");

    //         // Test device extraction
    //         let logical_device = get_vulkan_device_from_wgpu(&device);
    //         assert!(logical_device.is_ok(), "Failed to get Vulkan logical device");
    //         assert!(!logical_device.unwrap().is_null(), "Vulkan logical device is null");

    //         // Test queue info extraction
    //         let queue_info = get_vulkan_queue_info_from_wgpu(&device);
    //         assert!(queue_info.is_ok(), "Failed to get Vulkan queue info");
    //         let (family, index) = queue_info.unwrap();
    //         assert!(family < 16, "Queue family index out of reasonable range"); // Most GPUs have < 16 queue families
    //         assert!(index == 0, "Expected first queue in family");
    //     } else {
    //         println!("Skipping Vulkan handle extraction test - no suitable GPU adapter available");
    //     }
    // }

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