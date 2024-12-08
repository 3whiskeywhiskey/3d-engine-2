use std::ffi::c_void;
use wgpu::hal::api::Vulkan;
use anyhow::Result;

/// Extract Vulkan instance handle from wgpu device
pub fn get_vulkan_instance_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
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

/// Extract Vulkan physical device handle from wgpu device
pub fn get_vulkan_physical_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
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

/// Extract Vulkan device handle from wgpu device
pub fn get_vulkan_device_from_wgpu(device: &wgpu::Device) -> Result<*const c_void> {
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

/// Get Vulkan queue family and queue index from wgpu device
pub fn get_vulkan_queue_info_from_wgpu(device: &wgpu::Device) -> Result<(u32, u32)> {
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