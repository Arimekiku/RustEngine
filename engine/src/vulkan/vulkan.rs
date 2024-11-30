use std::sync::Arc;
use vulkano::{
    command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, device::*, image::ImageUsage, instance::*, memory::allocator::{FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator}, pipeline::{compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo, ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo}, shader::EntryPoint, swapchain::{Surface, Swapchain, SwapchainCreateInfo}, VulkanLibrary
};
use winit::{event_loop::EventLoop, window::{Window, WindowBuilder}};

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
    pub vulkan_event : EventLoop<()>,
    vulkan_window : Arc<Surface>,
}

impl VulkanToolset {
    pub fn new() -> VulkanToolset {
        let event_loop = EventLoop::new();
        let instance = Self::create_instance(&event_loop);

        let window = VulkanWindow::create_window(&event_loop);
        let surface = VulkanWindow::create_surface(&instance, &window);

        let (device, queue) = Self::create_logical_device(&instance, &surface);

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

        let (mut swapchain, images) = Swapchain::new(
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
        )
        .unwrap();

        let toolset = VulkanToolset {
            vulkan_instance : instance,
            vulkan_device : device,
            vulkan_queue : queue,
            vulkan_event : event_loop,
            vulkan_window : surface
        };

        toolset
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
        )
        .expect("failed to create instance");

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
        })
        .min_by_key(|(p, _)| match  p.properties().device_type {
            physical::PhysicalDeviceType::DiscreteGpu => 0,
            physical::PhysicalDeviceType::IntegratedGpu => 1,
            physical::PhysicalDeviceType::VirtualGpu => 2,
            physical::PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no devices available");

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
        )
        .expect("failed to create device");

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
        )
        .unwrap();

        let compute_pipeline = ComputePipeline::new(
            device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .expect("failed to create compute pipeline");

        let compute = ComputeShader {
            pipeline : compute_pipeline,
        };

        compute
    }
}