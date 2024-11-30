pub mod vulkan;
pub mod compute_test;
pub mod image_test;

use compute_test::compute_test;
use image_test::image_test;
use std::sync::Arc;
use vulkan::{VulkanAllocation, VulkanToolset};

fn main() {
    // Setup Vulkan toolset
    let toolset = VulkanToolset::new();
    let device = toolset.vulkan_device;
    let queue = toolset.vulkan_queue;

    let allocator = Arc::new(VulkanAllocation::new(device.clone()));

    // Test basic shader workability
    compute_test(device.clone(), queue.clone(), allocator.clone());

    // Test basic image workability
    image_test(device.clone(), queue.clone(), allocator.clone());

    println!("Everything succeeded!");
}
