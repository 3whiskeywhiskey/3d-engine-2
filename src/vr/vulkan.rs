use std::ffi::c_void;
use wgpu::hal::api::Vulkan;
use anyhow::Result;

/// Extract Vulkan instance handle from wgpu device
pub fn get_vulkan_instance_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the raw instance handle from the device
            let raw_instance = vulkan_device
                .shared_instance()
                .raw_instance()
                .handle();

            Ok(std::mem::transmute(raw_instance))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan instance"))?
    }
}

/// Extract Vulkan physical device handle from wgpu device
pub fn get_vulkan_physical_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the raw physical device handle
            let raw_physical_device = vulkan_device
                .raw_physical_device();

            Ok(std::mem::transmute(raw_physical_device))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan physical device"))?
    }
}

/// Extract Vulkan device handle from wgpu device
pub fn get_vulkan_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<*const c_void>>(|vulkan_device| {
            let vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the raw device handle
            let raw_device = vulkan_device
                .raw_device()
                .handle();

            Ok(std::mem::transmute(raw_device))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?
    }
}

/// Get Vulkan queue family and queue index from wgpu device
pub fn get_vulkan_queue_info_from_wgpu(device: &wgpu::Device) -> Result<(u32, u32)> {
    unsafe {
        device.as_hal::<Vulkan, _, Result<(u32, u32)>>(|vulkan_device| {
            let vulkan_device = vulkan_device
                .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan device"))?;
            
            // Get the queue family index from the device
            let family_index = vulkan_device.queue_family_index();
            
            Ok((family_index, 0)) // Using first queue in family
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to get Vulkan queue info"))?
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
    use pollster::FutureExt;

    fn create_test_device() -> Option<wgpu::Device> {
        let instance = wgpu::Instance::default();
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: true,
                compatible_surface: None,
            })
            .block_on()?;

        let (device, _) = adapter
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

        Some(device)
    }

    #[test]
    fn test_vulkan_handle_extraction() {
        if let Some(device) = create_test_device() {
            // Test instance extraction
            let instance = get_vulkan_instance_from_wgpu(&device);
            assert!(instance.is_ok(), "Failed to get Vulkan instance");
            assert!(!instance.unwrap().is_null(), "Vulkan instance is null");

            // Test physical device extraction
            let physical_device = get_vulkan_physical_device_from_wgpu(&device);
            assert!(physical_device.is_ok(), "Failed to get Vulkan physical device");
            assert!(!physical_device.unwrap().is_null(), "Vulkan physical device is null");

            // Test device extraction
            let logical_device = get_vulkan_device_from_wgpu(&device);
            assert!(logical_device.is_ok(), "Failed to get Vulkan logical device");
            assert!(!logical_device.unwrap().is_null(), "Vulkan logical device is null");

            // Test queue info extraction
            let queue_info = get_vulkan_queue_info_from_wgpu(&device);
            assert!(queue_info.is_ok(), "Failed to get Vulkan queue info");
            let (family, index) = queue_info.unwrap();
            assert!(family < 16, "Queue family index out of reasonable range"); // Most GPUs have < 16 queue families
            assert!(index == 0, "Expected first queue in family");
        } else {
            println!("Skipping Vulkan handle extraction test - no suitable GPU adapter available");
        }
    }

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