use super::texture::Texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl Material {
    pub fn clone_with_device(&self, device: &wgpu::Device) -> Self {
        let diffuse_texture = self.diffuse_texture.as_ref().map(|texture| {
            texture.clone_with_device(device)
        });

        let mut material = Self {
            name: self.name.clone(),
            diffuse_texture,
            bind_group: None,
            bind_group_layout: None,
        };

        material.create_bind_group(device);
        material
    }

    pub fn create_bind_group(&mut self, device: &wgpu::Device) {
        if let Some(texture) = &self.diffuse_texture {
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("diffuse_bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
            });

            self.bind_group = Some(bind_group);
            self.bind_group_layout = Some(bind_group_layout);
        }
    }
} 