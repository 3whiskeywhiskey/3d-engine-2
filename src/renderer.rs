use crate::vr::VRSystem;
use crate::scene::Scene;
use anyhow::Result;

#[derive(Clone, Copy, Debug)]
pub enum ForcedMode {
    Auto,
    Flat,
    VR,
}

#[derive(Debug)]
pub enum RenderMode {
    Standard,
    VR(VRSystem),
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub mode: RenderMode,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    pub material_bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, config: &wgpu::SurfaceConfiguration, surface: wgpu::Surface<'static>, forced_mode: ForcedMode) -> Self {
        // Try to initialize VR based on forced_mode
        let mode = match forced_mode {
            ForcedMode::Flat => {
                log::info!("Using flat mode (forced)");
                RenderMode::Standard
            },
            ForcedMode::VR => {
                match VRSystem::new() {
                    Ok(mut vr) => {
                        log::info!("Using VR mode (forced)");
                        if let Err(e) = vr.initialize_session(&device) {
                            log::error!("Failed to initialize VR session: {}", e);
                            RenderMode::Standard
                        } else {
                            RenderMode::VR(vr)
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to initialize VR system: {}", e);
                        RenderMode::Standard
                    }
                }
            },
            ForcedMode::Auto => {
                match VRSystem::new() {
                    Ok(mut vr) if vr.is_hmd_available() => {
                        log::info!("VR headset detected, using VR mode");
                        if let Err(e) = vr.initialize_session(&device) {
                            log::error!("Failed to initialize VR session: {}", e);
                            RenderMode::Standard
                        } else {
                            RenderMode::VR(vr)
                        }
                    },
                    _ => {
                        log::info!("No VR headset detected, using standard mode");
                        RenderMode::Standard
                    }
                }
            }
        };

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
            ],
        });

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, config.width, config.height);

        Self {
            surface,
            device,
            queue,
            config: config.clone(),
            mode,
            depth_texture: Some(depth_texture),
            depth_view: Some(depth_view),
            material_bind_group_layout,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, new_size.width, new_size.height);
            self.depth_texture = Some(depth_texture);
            self.depth_view = Some(depth_view);
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self, scene: &Scene) -> Result<()> {
        match &mut self.mode {
            RenderMode::Standard => self.render_standard(scene),
            RenderMode::VR(vr) => {
                let vr = vr as *mut VRSystem;
                // SAFETY: We know vr is valid as it's part of self.mode
                // and we're only using it through a method that takes &mut self
                unsafe { self.render_vr(scene, &mut *vr) }
            }
        }
    }

    fn render_standard(&mut self, scene: &Scene) -> Result<()> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                depth_stencil_attachment: self.depth_view.as_ref().map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render scene here
            scene.render(render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn render_vr(&mut self, scene: &Scene, vr: &mut VRSystem) -> Result<()> {
        // VR rendering implementation
        Ok(())
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;
    use serial_test::serial;
    use std::sync::Arc;

    struct TestContext {
        window: Arc<winit::window::Window>,
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
    }

    impl TestContext {
        fn new() -> Self {
            let event_loop = EventLoop::new().unwrap();
            let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                dx12_shader_compiler: Default::default(),
                flags: wgpu::InstanceFlags::empty(),
                gles_minor_version: wgpu::Gles3MinorVersion::default(),
            });

            let surface = unsafe {
                instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap()).unwrap()
            };

            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })).unwrap();

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )).unwrap();

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface.get_capabilities(&adapter).formats[0],
                width: window.inner_size().width,
                height: window.inner_size().height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            Self {
                window,
                surface,
                device,
                queue,
                config,
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_renderer_creation() {
        let ctx = TestContext::new();
        let renderer = Renderer::new(ctx.device, ctx.queue, &ctx.config, ctx.surface, ForcedMode::Flat);
        assert!(matches!(renderer.mode, RenderMode::Standard));
    }

    #[tokio::test]
    #[serial]
    async fn test_renderer_mode_selection() {
        let ctx = TestContext::new();
        let renderer = Renderer::new(ctx.device, ctx.queue, &ctx.config, ctx.surface, ForcedMode::Auto);
        
        // The mode should be either Standard or VR, depending on HMD availability
        match renderer.mode {
            RenderMode::Standard => {
                println!("Running in standard mode");
            }
            RenderMode::VR(_) => {
                println!("Running in VR mode");
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_renderer_resize() {
        let ctx = TestContext::new();
        let mut renderer = Renderer::new(ctx.device, ctx.queue, &ctx.config, ctx.surface, ForcedMode::Flat);
        
        let new_size = winit::dpi::PhysicalSize::new(800, 600);
        renderer.resize(new_size);
        
        assert_eq!(renderer.config.width, 800);
        assert_eq!(renderer.config.height, 600);
    }
} 