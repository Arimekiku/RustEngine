use std::sync::Arc;

use vulkano::{device::*, instance::*, VulkanLibrary};

pub struct VulkanToolset {
    pub vulkan_instance : Arc<Instance>,
    pub vulkan_device : Arc<Device>,
    pub vulkan_queue : Arc<Queue>
}

impl VulkanToolset {
    pub fn new() -> VulkanToolset {
        // Create vulkan instances
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        // Create vulkan device
        let physical_device = instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .next()
        .expect("no devices available");

        for family in physical_device.queue_family_properties() {
            println!("Found a queue family with {:?} queue(s)", family.queue_count);
        }

        let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("couldn't find a graphical queue family") as u32;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let toolset = VulkanToolset {
            vulkan_instance : instance,
            vulkan_device : device,
            vulkan_queue : queue
        };

        toolset
    }
}