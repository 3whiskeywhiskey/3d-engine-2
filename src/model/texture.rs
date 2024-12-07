use std::path::Path;
use image::GenericImageView;
use anyhow::Result;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    fn calculate_padded_bytes_per_row(width: u32) -> u32 {
        let bytes_per_row = width * 4;
        let align = 256;
        (bytes_per_row + align - 1) & !(align - 1)
    }

    fn copy_texture_data(
        data: &[u8],
        dimensions: (u32, u32),
        padded_bytes_per_row: u32,
    ) -> Vec<u8> {
        let unpadded_bytes_per_row = dimensions.0 * 4;
        let total_size = (padded_bytes_per_row * dimensions.1) as usize;
        let mut padded_data = vec![0; total_size];

        for row in 0..dimensions.1 {
            let src_start = (row * unpadded_bytes_per_row) as usize;
            let src_end = src_start + unpadded_bytes_per_row as usize;
            let dst_start = (row * padded_bytes_per_row) as usize;
            let dst_end = dst_start + unpadded_bytes_per_row as usize;

            if src_end <= data.len() && dst_end <= padded_data.len() {
                padded_data[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            } else {
                log::warn!("Skipping row {} due to buffer size mismatch", row);
                break;
            }
        }

        padded_data
    }

    pub fn from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        label: Option<&str>,
    ) -> Result<Self> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        let rgba_raw = rgba.as_raw();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let padded_bytes_per_row = Self::calculate_padded_bytes_per_row(dimensions.0);
        let padded_data = Self::copy_texture_data(rgba_raw, dimensions, padded_bytes_per_row);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &padded_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn from_gltf_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &gltf::image::Data,
        label: Option<&str>,
    ) -> Result<Self> {
        let dimensions = (image.width, image.height);
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let padded_bytes_per_row = Self::calculate_padded_bytes_per_row(dimensions.0);
        let padded_data = Self::copy_texture_data(&image.pixels, dimensions, padded_bytes_per_row);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &padded_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn clone_with_device(&self, device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: self.texture.size(),
            mip_level_count: self.texture.mip_level_count(),
            sample_count: self.texture.sample_count(),
            dimension: self.texture.dimension(),
            format: self.texture.format(),
            usage: self.texture.usage(),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Self {
            texture,
            view,
            sampler,
        }
    }
} 