use anyhow::Result;
use ash::vk::{self, Handle};
use std::ffi::{c_void, CString};
use log::{info, debug};

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
    owns_device: bool,
}

impl VulkanContext {
    pub fn new(vk_instance: *const c_void, physical_device: u64, device: ash::Device) -> Result<Self> {
        unsafe {
            // Create Vulkan instance from OpenXR instance
            debug!("Loading Vulkan entry...");
            let entry = ash::Entry::load()?;
            debug!("Creating Vulkan instance...");
            let instance = ash::Instance::load(
                entry.static_fn(),
                vk::Instance::from_raw(vk_instance as u64),
            );

            // Convert physical device handle
            let physical_device = vk::PhysicalDevice::from_raw(physical_device);

            // Print device info
            let device_properties = instance.get_physical_device_properties(physical_device);
            info!(
                "Selected Vulkan device: {} (type: {:?}, API version: {}.{}.{})",
                std::ffi::CStr::from_ptr(device_properties.device_name.as_ptr()).to_string_lossy(),
                device_properties.device_type,
                vk::api_version_major(device_properties.api_version),
                vk::api_version_minor(device_properties.api_version),
                vk::api_version_patch(device_properties.api_version),
            );

            // Find a queue family that supports graphics
            debug!("Finding graphics queue family...");
            let queue_family_properties = instance.get_physical_device_queue_family_properties(physical_device);
            let queue_family_index = queue_family_properties
                .iter()
                .enumerate()
                .find(|(_, properties)| properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .map(|(index, _)| index as u32)
                .ok_or_else(|| anyhow::anyhow!("No graphics queue family found"))?;
            debug!("Selected queue family index: {}", queue_family_index);

            let queue = device.get_device_queue(queue_family_index, 0);
            info!("Vulkan device initialized successfully");

            Ok(Self {
                entry,
                instance,
                device,
                physical_device,
                queue,
                queue_family_index,
                owns_device: false,  // OpenXR owns the device
            })
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        info!("Cleaning up Vulkan context");
        if self.owns_device {
            unsafe {
                self.device.destroy_device(None);
            }
        }
    }
} 