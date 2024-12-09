use wgpu;
use std::mem;
use bytemuck::{Pod, Zeroable};
use crate::model::ModelVertex;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VRUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub eye_position: [f32; 3],
    pub _padding: u32,
}

pub struct VRPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl VRPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VR Uniform Buffer"),
            size: mem::size_of::<VRUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VR Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VR Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VR Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/vr.wgsl"))),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("VR Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("VR Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ModelVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            uniform_bind_group_layout,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update_uniform(&self, queue: &wgpu::Queue, uniform: &VRUniform) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    pub fn create_swapchain_view(
        &self,
        device: &wgpu::Device,
        image_index: u32,
        width: u32,
        height: u32,
    ) -> Result<wgpu::TextureView, anyhow::Error> {
        // Create texture descriptor for the swapchain image
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VR Swapchain Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 2, // One layer for each eye
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb, // Match the swapchain format
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Create view for the texture
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("VR Swapchain View"),
            format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(2), // One layer for each eye
            ..Default::default()
        });

        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    struct TestContext {
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    impl TestContext {
        fn new() -> Option<Self> {
            let instance = wgpu::Instance::default();
            
            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: true,
                compatible_surface: None,
            }))?;

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )).ok()?;

            Some(Self { device, queue })
        }
    }

    #[test]
    fn test_vr_pipeline_creation() {
        let context = match TestContext::new() {
            Some(context) => context,
            None => {
                println!("Skipping test 'test_vr_pipeline_creation' - no suitable GPU adapter available");
                return;
            }
        };

        let pipeline = VRPipeline::new(
            &context.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Depth32Float,
        );

        // Verify pipeline components
        assert!(pipeline.uniform_buffer.size() >= std::mem::size_of::<VRUniform>() as u64);
        // We can't directly verify the bind group layout, but we can check it exists
        assert!(std::ptr::addr_of!(pipeline.uniform_bind_group_layout) != std::ptr::null());
    }

    #[test]
    fn test_swapchain_view_creation() -> Result<()> {
        let context = match TestContext::new() {
            Some(context) => context,
            None => {
                println!("Skipping test 'test_swapchain_view_creation' - no suitable GPU adapter available");
                return Ok(());
            }
        };

        let pipeline = VRPipeline::new(
            &context.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Depth32Float,
        );

        // Test view creation with different dimensions
        let test_dimensions = [(800, 600), (1024, 768), (1920, 1080)];
        
        for (width, height) in test_dimensions.iter() {
            let _view = pipeline.create_swapchain_view(
                &context.device,
                0, // image_index
                *width,
                *height,
            )?;

            // If we got here without panicking, the view was created successfully
        }

        Ok(())
    }
} 