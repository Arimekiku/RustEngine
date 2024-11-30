pub mod compute_test;
pub mod vulkan;

use compute_test::compute_test;
use vulkan::VulkanToolset;

fn main() {
    // Setup Vulkan toolset
    let toolset = VulkanToolset::new();
    let device = toolset.vulkan_device;
    let queue = toolset.vulkan_queue;

    // Test basic shader workability
    compute_test(device, queue);

    println!("Everything succeeded!");
}
