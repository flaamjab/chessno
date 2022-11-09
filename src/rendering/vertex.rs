use std::mem::size_of;

use erupt::vk;
use memoffset::offset_of;

use crate::rendering::mesh::Mesh;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn zeroed() -> Self {
        Vertex {
            pos: [0.0; 3],
            uv: [0.0; 3],
            color: [0.0; 3],
        }
    }

    pub fn binding_desc<'a>() -> vk::VertexInputBindingDescriptionBuilder<'a> {
        vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .input_rate(vk::VertexInputRate::VERTEX)
            .stride(size_of::<Vertex>() as u32)
    }

    pub fn attribute_descs<'a>() -> Vec<vk::VertexInputAttributeDescriptionBuilder<'a>> {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            }
            .into_builder(),
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            }
            .into_builder(),
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            }
            .into_builder(),
        ]
        .into()
    }
}
