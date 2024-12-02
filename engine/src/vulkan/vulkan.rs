use std::sync::Arc;
use vulkano::{
    buffer::Subbuffer, command_buffer::{allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo}, device::*, instance::*, memory::allocator::{FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator}, pipeline::{compute::ComputePipelineCreateInfo, graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::ViewportState, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, ComputePipeline, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{Framebuffer, Subpass}, shader::{EntryPoint, ShaderModule}, swapchain::Surface, VulkanLibrary
};
use winit::event_loop::EventLoop;

use crate::tests::window_test::VulkanVertex;
use super::vulkan_window::VulkanWindow;

pub struct VulkanToolset {
    pub instance : Arc<Instance>,
    pub logical_device : Arc<Device>,
    pub device_queue : Arc<Queue>,
    pub memory_allocator : Arc<VulkanAllocation>,
    pub window : Arc<VulkanWindow>,
}

impl VulkanToolset {
    pub fn new(event_loop : &EventLoop<()>) -> VulkanToolset {
        // Create basic instances
        let vulkan_instance = Self::create_instance(event_loop);
        let mut window_instance = VulkanWindow::new(&vulkan_instance, event_loop);

        // Create logical device
        let surface = window_instance.get_window_surface();
        let (device, queue) = Self::create_logical_device(&vulkan_instance, &surface);

        // Create vulkan window
        window_instance.create_swapchain(&device);
        let vulkan_window = Arc::new(window_instance);

        // Create vulkan allocator
        let allocator = Arc::new(VulkanAllocation::new(device.clone()));

        VulkanToolset {
            instance: vulkan_instance,
            logical_device : device,
            device_queue : queue,
            memory_allocator : allocator,
            window: vulkan_window
        }
    }
  
    pub fn create_graphics_pipeline(&self, vs : &Arc<ShaderModule>, fs : &Arc<ShaderModule>) -> Arc<GraphicsPipeline> {
        let render_pass = self.window.get_render_pass();
        let viewport = self.window.get_window_viewport();

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
            self.logical_device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.logical_device.clone())
                .unwrap(),
        ).unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        GraphicsPipeline::new(
            self.logical_device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [viewport.clone()].into_iter().collect(),
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
            // Create graphics pipeline
            let mut builder = AutoCommandBufferBuilder::primary(
                &self.memory_allocator.buffer_allocator,
                self.device_queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            ).unwrap();

            // Fill pipeline with commands
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

            // Build result pipeline
            builder.build().unwrap()
        }).collect()
    }

    pub fn get_vulkan_window(&self) -> &Arc<VulkanWindow> {
        &self.window
    } 

    fn create_instance(event_loop : &EventLoop<()>) -> Arc<Instance> {
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let required_extensions = Surface::required_extensions(&event_loop);

        Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        ).expect("failed to create instance")
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

        VulkanAllocation {
            general_allocator : memory_allocator,
            buffer_allocator : command_buffer_allocator,
        }
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

        ComputeShader {
            pipeline : compute_pipeline,
        }
    }
}