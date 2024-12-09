use anyhow::Result;
use std::sync::Arc;
use wgpu::{
    util::DeviceExt,
    Device, Queue, RenderPipeline, Surface, SurfaceConfiguration,
};
use crate::{
    Scene,
    vr::system::VRSystem,
    vr::pipeline,
    model::ModelVertex,
    scene::camera::Camera,
};

#[derive(Debug, Clone, Copy)]
pub enum ForcedMode {
    Standard,
    VR,
}

pub enum RenderMode {
    Standard,
    VR(VRSystem),
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0; 4],
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
        let pos = camera.position;
        self.camera_pos = [pos.x, pos.y, pos.z, 1.0];
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    direction: [f32; 4],
    color: [f32; 4],
    ambient: [f32; 4],
}

impl LightUniform {
    fn new() -> Self {
        Self {
            direction: [-1.0, -1.0, -1.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            ambient: [0.1, 0.1, 0.1, 1.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniform {
    model_matrix: [[f32; 4]; 4],
}

impl ModelUniform {
    fn new(matrix: glam::Mat4) -> Self {
        Self {
            model_matrix: matrix.to_cols_array_2d(),
        }
    }
}

pub struct Renderer<'a> {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: SurfaceConfiguration,
    pub surface: Option<Surface<'a>>,
    pub mode: RenderMode,
    pub render_pipeline: RenderPipeline,
    pub camera_bind_group: wgpu::BindGroup,
    pub light_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
    pub material_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
}

impl<'a> Renderer<'a> {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        config: &SurfaceConfiguration,
        surface: Option<Surface<'a>>,
        forced_mode: ForcedMode,
    ) -> Self {
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let material_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });

        let light_uniform = LightUniform::new();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        let mode = match forced_mode {
            ForcedMode::Standard => RenderMode::Standard,
            ForcedMode::VR => {
                if let Ok(mut vr) = VRSystem::new() {
                    // Initialize VR session with the device
                    if let Err(e) = vr.initialize_session(&device) {
                        log::error!("Failed to initialize VR session: {}", e);
                        RenderMode::Standard
                    } else {
                        RenderMode::VR(vr)
                    }
                } else {
                    RenderMode::Standard
                }
            }
        };

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader2.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &model_bind_group_layout,
                &material_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                    format: config.format,
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
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
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

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            device,
            queue,
            config: config.clone(),
            surface,
            mode,
            render_pipeline,
            camera_bind_group,
            light_bind_group,
            camera_bind_group_layout,
            light_bind_group_layout,
            model_bind_group_layout,
            material_bind_group_layout,
            depth_texture,
            depth_view,
            camera_buffer,
            light_buffer,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.material_bind_group_layout
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            if let Some(surface) = &self.surface {
                surface.configure(&self.device, &self.config);
            }

            // Recreate depth texture with new size
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn render(&mut self, scene: &Scene) -> Result<()> {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        match self.mode {
            RenderMode::Standard => self.render_standard(scene),
            RenderMode::VR(_) => {
                // Extract the VR system
                let mode = std::mem::replace(&mut self.mode, RenderMode::Standard);
                if let RenderMode::VR(mut vr_system) = mode {
                    let result = self.render_vr(scene, &mut vr_system);
                    // Put the VR system back
                    self.mode = RenderMode::VR(vr_system);
                    result
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn render_standard(&mut self, scene: &Scene) -> Result<()> {
        if let Some(surface) = &self.surface {
            let frame = surface.get_current_texture()?;
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);

                for (model, transform) in &scene.objects {
                    let model_uniform = ModelUniform::new(transform.to_matrix());
                    let model_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Model Buffer"),
                        contents: bytemuck::cast_slice(&[model_uniform]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Model Bind Group"),
                        layout: &self.model_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: model_buffer.as_entire_binding(),
                            },
                        ],
                    });

                    render_pass.set_bind_group(2, &model_bind_group, &[]);

                    for mesh in &model.meshes {
                        render_pass.set_bind_group(3, &model.materials[mesh.material_index].bind_group, &[]);
                        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
                    }
                }
            }

            self.queue.submit(Some(encoder.finish()));
            frame.present();
        }

        Ok(())
    }

    fn render_vr(&mut self, scene: &Scene, vr: &mut VRSystem) -> Result<()> {
        // Begin the frame and get frame timing
        let frame_state = vr.begin_frame()?;

        if !frame_state.should_render {
            // Skip rendering if not needed
            return Ok(());
        }

        // Get the swapchain image to render to
        let image_index = vr.acquire_swapchain_image()?;

        // Get view projections for both eyes
        let view_projections = vr.get_view_projections(&frame_state)?;

        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VR Render Encoder"),
        });

        // Get swapchain image layout
        let (width, height) = vr.get_swapchain_image_layout()
            .ok_or_else(|| anyhow::anyhow!("Failed to get swapchain image layout"))?;

        // Get VR pipeline
        let vr_pipeline = vr.get_pipeline()
            .ok_or_else(|| anyhow::anyhow!("VR pipeline not initialized"))?;

        // Create array texture view for the swapchain image
        let swapchain_view = vr_pipeline.create_swapchain_view(&self.device, image_index, width, height)?;

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VR Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set the VR pipeline
            render_pass.set_pipeline(&vr_pipeline.render_pipeline);

            // Render scene for each eye
            for (view_index, view_proj) in view_projections.iter().enumerate() {
                // Update VR uniform buffer with view/projection matrices
                let vr_uniform = pipeline::VRUniform {
                    view_proj: (view_proj.projection * view_proj.view).to_cols_array_2d(),
                    view: view_proj.view.to_cols_array_2d(),
                    proj: view_proj.projection.to_cols_array_2d(),
                    eye_position: [
                        view_proj.pose.position.x,
                        view_proj.pose.position.y,
                        view_proj.pose.position.z,
                    ],
                    _padding: 0,
                };

                self.queue.write_buffer(&vr_pipeline.uniform_buffer, 0, bytemuck::cast_slice(&[vr_uniform]));

                // Set view index for multiview rendering
                render_pass.set_viewport(
                    (width as f32 * view_index as f32) / 2.0,
                    0.0,
                    width as f32 / 2.0,
                    height as f32,
                    0.0,
                    1.0,
                );

                // Set the VR uniform bind group
                render_pass.set_bind_group(0, &vr_pipeline.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);

                // Render each object
                for (model, transform) in &scene.objects {
                    let model_uniform = ModelUniform::new(transform.to_matrix());
                    let model_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Model Buffer"),
                        contents: bytemuck::cast_slice(&[model_uniform]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Model Bind Group"),
                        layout: &self.model_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: model_buffer.as_entire_binding(),
                            },
                        ],
                    });

                    render_pass.set_bind_group(2, &model_bind_group, &[]);

                    for mesh in &model.meshes {
                        render_pass.set_bind_group(3, &model.materials[mesh.material_index].bind_group, &[]);
                        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
                    }
                }
            }
        }

        // Submit command buffer
        self.queue.submit(Some(encoder.finish()));

        // Release swapchain image and end frame
        vr.release_swapchain_image()?;

        // Submit frame with composition layers
        vr.submit_frame(frame_state, &view_projections, width, height)?;

        Ok(())
    }
} 