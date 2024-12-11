use ash::vk;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12)
                .build(),
        ]
    }
}

pub const VERTICES: [Vertex; 3] = [
    Vertex { pos: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },    // Top - Red
    Vertex { pos: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },  // Bottom left - Green
    Vertex { pos: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },   // Bottom right - Blue
]; 

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewData {
    pub view_matrices: [[f32; 16]; 2],
    pub projection_matrices: [[f32; 16]; 2],
} 