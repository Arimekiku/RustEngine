mod vulkan;
mod tests;

use tests::{compute_test::compute_test, image_test::image_test, window_test::window_test};
use vulkan::vulkan::VulkanToolset;
use winit::event_loop::EventLoop;

pub struct App;

impl App {
    pub fn run() {
        // Setup Vulkan toolset
        let event_loop = EventLoop::new();

        let toolset = VulkanToolset::new(&event_loop);
        let device = &toolset.logical_device;
        let queue = &toolset.device_queue;
        let allocator = &toolset.memory_allocator;

        // Test basic shader workability
        compute_test(&device, &queue, &allocator);

        // Test basic image workability
        image_test(&device, &queue, &allocator);

        // Vertex test
        window_test(toolset, event_loop);
    }
}