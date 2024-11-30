use std::sync::Arc;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage}, 
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage}, 
    descriptor_set::{allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet}, 
    device::{Device, Queue}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, 
    pipeline::{Pipeline, PipelineBindPoint}, 
    sync::{self, GpuFuture}
};
use crate::vulkan::vulkan::{ComputeShader, VulkanAllocation};

mod cs {
    vulkano_shaders::shader!{
        ty: "compute",
        src: r"
            #version 460

            layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

            layout(set = 0, binding = 0) buffer Data {
                uint data[];
            } buf;

            void main() {
                uint idx = gl_GlobalInvocationID.x;
                buf.data[idx] *= 13;
            }
        ",
    }
}

pub fn compute_test(device : Arc<Device>, queue : Arc<Queue>, allocator : Arc<VulkanAllocation>) {
    let memory_allocator = allocator.general_allocator.clone();
    let command_buffer_allocator = &allocator.buffer_allocator;

    // Create compute shader
    let shader = cs::load(device.clone()).expect("failed to create shader module");
    let cs = shader.entry_point("main").unwrap();

    let compute = ComputeShader::new(cs, device.clone());
    let compute_pipeline = compute.pipeline;

    // Setup data buffer
    // We will apply compute shader to this data buffer
    let data_iter = 0..65536u32;
    let data_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data_iter,
    )
    .expect("failed to create buffer");

    // Setup descriptor sets for our data buffer
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone(), Default::default());
    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();

    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        layout.clone(),
        [WriteDescriptorSet::buffer(0, data_buffer.clone())], // 0 is the binding
        [],
    )
    .unwrap();

    // Setup buffer builder command
    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    
    let work_group_counts = [1024, 1, 1];
    
    // Define buffer builder command
    command_buffer_builder
    .bind_pipeline_compute(compute_pipeline.clone())
    .unwrap()
    .bind_descriptor_sets(
        PipelineBindPoint::Compute,
        compute_pipeline.layout().clone(),
        0,
        descriptor_set,
    )
    .unwrap()
    .dispatch(work_group_counts)
    .unwrap();
    
    let command_buffer = command_buffer_builder.build().unwrap();

    // Execute buffer creation command
    let future = sync::now(device.clone())
    .then_execute(queue.clone(), command_buffer)
    .unwrap()
    .then_signal_fence_and_flush()
    .unwrap();

    future.wait(None).unwrap();

    // Get new data buffer values
    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        assert_eq!(*val, n as u32 * 13);
    }
}