use std::sync::Arc;
use vulkano::{
    command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, 
    device::*, 
    instance::*, 
    memory::allocator::{FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator}, 
    pipeline::{compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo, ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo}, 
    shader::EntryPoint, 
    VulkanLibrary
};

pub struct VulkanToolset {
    pub vulkan_instance : Arc<Instance>,
    pub vulkan_device : Arc<Device>,
    pub vulkan_queue : Arc<Queue>,
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
        // Setup compute pipeline
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