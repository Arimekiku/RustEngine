use std::sync::Arc;

use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, device::Device, memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter}, pipeline::graphics::vertex_input::Vertex, shader::ShaderModule, swapchain::{self, SwapchainCreateInfo, SwapchainPresentInfo}, sync::{self, future::FenceSignalFuture, GpuFuture}, Validated, VulkanError};
use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

use crate::vulkan::vulkan::VulkanToolset;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct VulkanVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

impl VulkanVertex {
    pub fn new(x : f32, y : f32) -> VulkanVertex {
        let vertex = VulkanVertex {
            position : [x, y]
        };

        vertex
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

pub struct Triangle {
    pub vertex_buffer : Subbuffer<[VulkanVertex]>,
    pub vertex_shader : Arc<ShaderModule>,
    pub fragment_shader : Arc<ShaderModule>,
}

impl Triangle {
    pub fn new(memory_allocator : Arc<dyn MemoryAllocator>, device : &Arc<Device>) -> Triangle {
        let vbo = vec![
            VulkanVertex::new(-0.5, -0.5),
            VulkanVertex::new( 0.0,  0.5),
            VulkanVertex::new( 0.5, -0.25),
        ];
    
        let vbo = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vbo,
        ).unwrap();
    
        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");
    
        Triangle {
            vertex_buffer : vbo,
            vertex_shader : vs,
            fragment_shader : fs
        }
    }
}

pub fn window_test(toolset : VulkanToolset, event_loop : EventLoop<()>) {
    let window = toolset.get_vulkan_window().to_owned().clone();
    let mut viewport = window.get_window_viewport().to_owned();
    let (mut swapchain, images) = window.get_swapchain();
    
    let device = toolset.logical_device.clone();
    let allocator = &toolset.memory_allocator;
    let triangle = Arc::new(Triangle::new(allocator.general_allocator.clone(), &device));

    let pipeline = toolset.create_graphics_pipeline(&triangle.vertex_shader, &triangle.fragment_shader);
    let framebuffers = window.create_framebuffers(images.to_vec());
    let mut command_buffer = toolset.create_command_buffers(&triangle.vertex_buffer, &pipeline, &framebuffers);

    let mut window_resized = false;
    let mut recreate_swapchain = false;

    let frames_in_flight = images.len();
    let mut fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
    let mut previous_fence_i = 0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { 
                event : WindowEvent::Resized(_),
                ..
            } => {
                window_resized = true;
            },
            Event::MainEventsCleared => {
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                
                    let native_window = window.get_native_window();
                    let new_dimensions = native_window.inner_size();
                
                    let (new_swapchain, new_images) = swapchain
                        .recreate(SwapchainCreateInfo {
                            image_extent: new_dimensions.into(),
                            ..swapchain.create_info()
                        })
                        .expect("failed to recreate swapchain: {e}");
                    swapchain = new_swapchain;
                    let new_framebuffers = window.create_framebuffers(new_images);
                
                    if window_resized {
                        window_resized = false;
                        viewport.extent = new_dimensions.into();

                        let fs = triangle.fragment_shader.clone();
                        let vs = triangle.vertex_shader.clone();
                        let vbo = triangle.vertex_buffer.clone();

                        let new_pipeline = toolset.create_graphics_pipeline(&vs, &fs);
                        command_buffer = toolset.create_command_buffers(&vbo, &new_pipeline, &new_framebuffers);
                    }
                }

                let (image_i, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None)
                    .map_err(Validated::unwrap)
                {
                    Ok(r) => r,
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("failed to acquire next image: {e}"),
                };

                if suboptimal {
                    recreate_swapchain = true;
                }

                // wait for the fence related to this image to finish (normally this would be the oldest fence)
                if let Some(image_fence) = &fences[image_i as usize] {
                    image_fence.wait(None).unwrap();
                }

                let previous_future = match fences[previous_fence_i as usize].clone() {
                    // Create a NowFuture
                    None => {
                        let mut now = sync::now(device.clone());
                        now.cleanup_finished();

                        now.boxed()
                    }
                    // Use the existing FenceSignalFuture
                    Some(fence) => fence.boxed(),
                };

                let queue = toolset.device_queue.clone();
                let future = previous_future
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer[image_i as usize].clone())
                    .unwrap()
                    .then_swapchain_present(
                        queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                    )
                    .then_signal_fence_and_flush();

                fences[image_i as usize] = match future.map_err(Validated::unwrap) {
                    Ok(value) => Some(Arc::new(value)),
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        None
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        None
                    }
                };

                previous_fence_i = image_i;
            },
            _ => ()
        }
    });
}