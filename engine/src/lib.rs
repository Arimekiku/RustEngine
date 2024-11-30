mod vulkan;
mod tests;

use std::sync::Arc;
use tests::{compute_test::compute_test, image_test::image_test, window_test::window_test};
use vulkan::vulkan::{VulkanAllocation, VulkanToolset};

pub struct App;

impl App {
    pub fn run() {
        // Setup Vulkan toolset
        let toolset = VulkanToolset::new();
        let device = toolset.vulkan_device;
        let queue = toolset.vulkan_queue;

        let allocator = Arc::new(VulkanAllocation::new(device.clone()));

        // Test basic shader workability
        compute_test(&device, &queue, &allocator);

        // Test basic image workability
        image_test(&device, &queue, &allocator);

        // Test basic window workability
        window_test(toolset.vulkan_event);
    }
}