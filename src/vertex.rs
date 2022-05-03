use ash::vk;

pub struct VertexInputDescription {
    pub bindings: Vec<vk::VertexInputBindingDescription>,
    pub attributes: Vec<vk::VertexInputAttributeDescription>,
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub colour: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn description() -> VertexInputDescription {
        VertexInputDescription {
            bindings: vec![vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<Vertex>() as _,
                input_rate: vk::VertexInputRate::VERTEX,
            }],
            attributes: vec![
                vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: 0,
                },
                vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 1,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: (std::mem::size_of::<f32>() * 3) as u32,
                },
                vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 2,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: (std::mem::size_of::<f32>() * 6) as u32,
                },
                vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 3,
                    format: vk::Format::R32G32_SFLOAT,
                    offset: (std::mem::size_of::<f32>() * 9) as u32,
                },
            ],
        }
    }
}
