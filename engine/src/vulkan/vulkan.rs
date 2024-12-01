use std::sync::Arc;
use vulkano::{
    buffer::Subbuffer, command_buffer::{allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo}, device::*, image::{view::ImageView, Image, ImageUsage}, instance::*, memory::allocator::{FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator}, pipeline::{compute::ComputePipelineCreateInfo, graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::{Viewport, ViewportState}, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, ComputePipeline, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, shader::{EntryPoint, ShaderModule}, swapchain::{Surface, Swapchain, SwapchainCreateInfo}, VulkanLibrary
};
use winit::{event_loop::EventLoop, window::{Window, WindowBuilder}};

use crate::tests::window_test::VulkanVertex;

pub struct VulkanWindow;

impl VulkanWindow {
    pub fn create_window(event_loop : &EventLoop<()>) -> Arc<Window> {
        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

        window
    }

    pub fn create_surface(instance : &Arc<Instance>, window : &Arc<Window>) -> Arc<Surface> {
        let surface = Surface::from_window(instance.clone(), window.clone())
        .expect("failed to create window surface");

        surface
    }
}

pub struct VulkanToolset {
    pub vulkan_instance : Arc<Instance>,
    pub vulkan_device : Arc<Device>,
    pub vulkan_queue : Arc<Queue>,
    pub vulkan_allocator : Arc<VulkanAllocation>,
    pub vulkan_window : Arc<Window>,
    pub viewport : Viewport,
    pub swapchain : Arc<Swapchain>,
    pub images : Vec<Arc<Image>>,
    pub vulkan_framebuffers : Vec<Arc<Framebuffer>>,
    vulkan_surface : Arc<Surface>,
    render_pass : Arc<RenderPass>,
}

impl VulkanToolset {
    pub fn new(event_loop : &EventLoop<()>) -> VulkanToolset {
        let instance = Self::create_instance(&event_loop);

        let window = VulkanWindow::create_window(&event_loop);
        let surface = VulkanWindow::create_surface(&instance, &window);

        let (device, queue) = Self::create_logical_device(&instance, &surface);

        let allocator = Arc::new(VulkanAllocation::new(device.clone()));

        // Swapchain
        let caps = device.physical_device()
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

        let dimensions = window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = device.physical_device()
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0;

        let (swap, image) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT, // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        ).unwrap();

        let pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    // Set the format the same as the swapchain.
                    format: swap.image_format(),
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

        let framebuffers = image.iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            ).unwrap()
        }).collect::<Vec<_>>();

        let view = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let toolset = VulkanToolset {
            vulkan_instance : instance,
            vulkan_device : device,
            vulkan_queue : queue,
            vulkan_window : window,
            vulkan_surface : surface,
            render_pass : pass,
            vulkan_framebuffers : framebuffers,
            vulkan_allocator : allocator,
            viewport : view,
            swapchain : swap,
            images : image
        };

        toolset
    }

    pub fn create_framebuffers(&self, images: &[Arc<Image>]) -> Vec<Arc<Framebuffer>> {
        images.iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                self.render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            ).unwrap()
        }).collect::<Vec<_>>()
    }

    pub fn create_graphics_pipeline(&self, vs : Arc<ShaderModule>, fs : Arc<ShaderModule>) -> Arc<GraphicsPipeline> {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let vertex_input_state = VulkanVertex::per_vertex()
            .definition(&vs.info().input_interface)
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            self.vulkan_device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.vulkan_device.clone())
                .unwrap(),
        ).unwrap();

        let subpass = Subpass::from(self.render_pass.clone(), 0).unwrap();

        GraphicsPipeline::new(
            self.vulkan_device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [self.viewport.clone()].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        ).unwrap()
    }

    pub fn create_command_buffers(&self, vbo : &Subbuffer<[VulkanVertex]>, pipeline : &Arc<GraphicsPipeline>, framebuffers : &Vec<Arc<Framebuffer>>) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                &self.vulkan_allocator.buffer_allocator,
                self.vulkan_queue.queue_family_index(),
                // Don't forget to write the correct buffer usage.
                CommandBufferUsage::MultipleSubmit,
            ).unwrap();

            builder.begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            ).unwrap()
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()
            .bind_vertex_buffers(0, vbo.clone())
            .unwrap()
            .draw(vbo.len() as u32, 1, 0, 0)
            .unwrap()
            .end_render_pass(SubpassEndInfo::default())
            .unwrap();

            builder.build().unwrap()
        }).collect()
    }

    fn create_instance(event_loop : &EventLoop<()>) -> Arc<Instance> {
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = Surface::required_extensions(&event_loop);
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        ).expect("failed to create instance");

        instance
    }

    fn create_logical_device(instance : &Arc<Instance>, surface : &Arc<Surface>) -> (Arc<Device>, Arc<Queue>) {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
            .iter()
            .enumerate()
            .position(|(i, q)| {
                q.queue_flags.contains(QueueFlags::GRAPHICS)
                && p.surface_support(i as u32, &surface).unwrap_or(false)
            })
            .map(|q| (p, q as u32))
        }).min_by_key(|(p, _)| match  p.properties().device_type {
            physical::PhysicalDeviceType::DiscreteGpu => 0,
            physical::PhysicalDeviceType::IntegratedGpu => 1,
            physical::PhysicalDeviceType::VirtualGpu => 2,
            physical::PhysicalDeviceType::Cpu => 3,
            _ => 4,
        }).expect("no devices available");

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions : device_extensions,
                ..Default::default()
            },
        ).expect("failed to create device");

        let queue = queues.next().unwrap();

        (device, queue)
    }
}

pub struct VulkanAllocation {
    pub general_allocator : Arc<GenericMemoryAllocator<FreeListAllocator>>,
    pub buffer_allocator : StandardCommandBufferAllocator,
}

impl VulkanAllocation {
    pub fn new(device : Arc<Device>) -> VulkanAllocation {
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        let allocator = VulkanAllocation {
            general_allocator : memory_allocator,
            buffer_allocator : command_buffer_allocator,
        };

        allocator
    }
}

pub struct ComputeShader {
    pub pipeline : Arc<ComputePipeline>,
}

impl ComputeShader {
    pub fn new(shader : EntryPoint, device : Arc<Device>) -> ComputeShader {
        let stage = PipelineShaderStageCreateInfo::new(shader);
        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        ).unwrap();

        let compute_pipeline = ComputePipeline::new(
            device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        ).expect("failed to create compute pipeline");

        let compute = ComputeShader {
            pipeline : compute_pipeline,
        };

        compute
    }
}