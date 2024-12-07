use super::texture::Texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Material {
    pub fn clone_with_device(&self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let diffuse_texture = self.diffuse_texture.as_ref().map(|texture| {
            texture.clone_with_device(device, queue)
        });

        let mut material = Self {
            name: self.name.clone(),
            diffuse_texture,
            bind_group: None,
        };

        material.create_bind_group(device, layout);
        material
    }

    pub fn create_bind_group(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) {
        if let Some(texture) = &self.diffuse_texture {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("diffuse_bind_group"),
                layout,
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
        }
    }
} 