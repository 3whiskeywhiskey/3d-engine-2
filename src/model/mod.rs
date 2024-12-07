use wgpu::util::DeviceExt;

mod texture;
mod material;
mod mesh;
mod vertex;
mod loader;

pub use texture::Texture;
pub use material::Material;
pub use mesh::Mesh;
pub use vertex::ModelVertex;
pub use loader::Model;

#[cfg(test)]
mod tests; 

impl Model {
    pub fn from_vertices(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[ModelVertex],
        indices: &[u32],
        texture_view: wgpu::TextureView,
        material_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create a default normal texture (flat surface)
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Normal Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload default normal data (pointing straight up)
        queue.write_texture(
            normal_texture.as_image_copy(),
            &[127, 127, 255, 255], // Normal map value for [0, 0, 1]
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let normal_texture_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a single mesh
        let mesh = Mesh {
            name: "floor".to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material_index: 0,
        };

        // Create a single material
        let material = Material {
            name: "floor_material".to_string(),
            diffuse_texture: None,
            normal_texture: None,
            bind_group: Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("Floor Material Bind Group"),
            })),
        };

        // Calculate bounds
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for vertex in vertices {
            for i in 0..3 {
                min[i] = min[i].min(vertex.position[i]);
                max[i] = max[i].max(vertex.position[i]);
            }
        }

        Self {
            meshes: vec![mesh],
            materials: vec![material],
            bounds_min: min,
            bounds_max: max,
        }
    }
} 