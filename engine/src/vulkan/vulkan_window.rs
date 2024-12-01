use std::sync::Arc;

use vulkano::{device::Device, image::{view::ImageView, Image, ImageUsage}, instance::Instance, pipeline::graphics::viewport::Viewport, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass}, swapchain::{Surface, Swapchain, SwapchainCreateInfo}};
use winit::{event_loop::EventLoop, window::{Window, WindowBuilder}};

pub struct VulkanWindow {
    native_window : Arc<Window>,
    window_surface : Arc<Surface>,
    window_viewport : Viewport,
    window_swapchain : Option<Arc<Swapchain>>,
    window_images : Option<Vec<Arc<Image>>>,
    window_render_pass : Option<Arc<RenderPass>>,
}

impl VulkanWindow {
    pub fn new(vulkan_instance : &Arc<Instance>, event_loop : &EventLoop<()>) -> VulkanWindow {
        // Create native window
        let window = Arc::new(WindowBuilder::new().build(&event_loop)
        .unwrap());

        // Create window surface
        let surface = Surface::from_window(vulkan_instance.clone(), window.clone())
        .expect("failed to create window surface");

        // Define viewport
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let vulkan_window = VulkanWindow {
            native_window : window,
            window_surface : surface,
            window_viewport : viewport,
            window_swapchain : None,
            window_images : None,
            window_render_pass : None,
        };

        vulkan_window
    }

    pub fn create_swapchain(&mut self, vulkan_device : &Arc<Device>) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
        let caps = vulkan_device.physical_device()
        .surface_capabilities(&self.window_surface, Default::default())
        .expect("failed to get surface capabilities");

        let dimensions = self.native_window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = vulkan_device.physical_device()
        .surface_formats(&self.window_surface, Default::default())
        .unwrap()[0]
        .0;

        let (swapchain, images) = Swapchain::new(
            vulkan_device.clone(),
            self.window_surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT, // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        ).unwrap();

        let render_pass = vulkano::single_pass_renderpass!(
            vulkan_device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        ).unwrap();

        self.window_swapchain = Some(swapchain.clone());
        self.window_images = Some(images.clone());
        self.window_render_pass = Some(render_pass.clone());

        (self.window_swapchain.clone().unwrap(), self.window_images.clone().unwrap())
    }

    pub fn create_framebuffers(&self, images : Vec<Arc<Image>>) -> Vec<Arc<Framebuffer>> {
        images.iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                self.window_render_pass.clone().expect("Framebuffer retrieve empty render pass!"),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            ).unwrap()
        }).collect::<Vec<_>>()
    }

    pub fn get_swapchain(&self) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
        match (self.window_swapchain.clone(), self.window_images.clone()) {
            (Some(swapchain), Some(images)) => (swapchain, images),
            _ => panic!("Swapchain is empty!"),
        }
    }

    pub fn get_render_pass(&self) -> Arc<RenderPass> {
        match self.window_render_pass.clone() {
            Some(render_pass) => render_pass,
            None => panic!("Render pass is empty"),
        }
    }

    pub fn get_native_window(&self) -> Arc<Window> {
        self.native_window.clone()
    }

    pub fn get_window_surface(&self) -> Arc<Surface> {
        self.window_surface.clone()
    }

    pub fn get_window_viewport(&self) -> Viewport {
        self.window_viewport.clone()
    }
}