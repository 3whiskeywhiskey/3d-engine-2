use super::texture::Texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<Texture>,
    pub normal_texture: Option<Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Material {
    pub fn clone_with_device(&self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let diffuse_texture = self.diffuse_texture.as_ref().map(|texture| {
            texture.clone_with_device(device, queue)
        });
        let normal_texture = self.normal_texture.as_ref().map(|texture| {
            texture.clone_with_device(device, queue)
        });

        let mut material = Self {
            name: self.name.clone(),
            diffuse_texture,
            normal_texture,
            bind_group: None,
        };

        material.create_bind_group(device, layout);
        material
    }

    pub fn create_bind_group(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) {
        let diffuse_texture = self.diffuse_texture.as_ref().unwrap_or_else(|| {
            panic!("Material {} must have a diffuse texture", self.name)
        });

        // Use a default normal map (flat surface) if none is provided
        let normal_texture = self.normal_texture.as_ref().unwrap_or(diffuse_texture);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{}_bind_group", self.name)),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
        });

        self.bind_group = Some(bind_group);
    }
} 