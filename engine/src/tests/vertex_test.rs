use std::sync::Arc;

use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, device::Device, memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter}, pipeline::graphics::vertex_input::Vertex, shader::ShaderModule};

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct VulkanVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
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
        let vertex1 = VulkanVertex {
            position: [-0.5, -0.5],
        };
    
        let vertex2 = VulkanVertex {
            position: [0.0, 0.5],
        };
    
        let vertex3 = VulkanVertex {
            position: [0.5, -0.25],
        };
    
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
            vec![vertex1, vertex2, vertex3],
        )?;
    
        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");
    
        Triangle {
            vertex_buffer : vbo,
            vertex_shader : vs,
            fragment_shader : fs
        }
    }
}