use anyhow::Result;
use wgpu;
use winit::window::Window;
use crate::vr::VRSystem;

pub enum RenderMode {
    Standard,
    VR(VRSystem),
}

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub mode: RenderMode,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        // Try to initialize VR first
        let vr_mode = VRSystem::new().ok().filter(|vr| vr.is_hmd_available());
        
        // Initialize standard graphics components
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
        
        let surface = unsafe { instance.create_surface(&window) }?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find appropriate adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Create depth texture for standard rendering
        let (depth_texture, depth_view) = Self::create_depth_texture(&device, config.width, config.height);

        // Choose render mode based on VR availability
        let mode = match vr_mode {
            Some(mut vr) => {
                log::info!("VR headset detected, using VR mode");
                // Initialize VR session with the device
                vr.initialize_session(&device)?;
                RenderMode::VR(vr)
            }
            None => {
                log::info!("No VR headset detected, using standard mode");
                RenderMode::Standard
            }
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            mode,
            depth_texture: Some(depth_texture),
            depth_view: Some(depth_view),
        })
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

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate depth texture
            let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, new_size.width, new_size.height);
            self.depth_texture = Some(depth_texture);
            self.depth_view = Some(depth_view);
        }
    }

    pub fn render(&mut self) -> Result<()> {
        match &mut self.mode {
            RenderMode::Standard => self.render_standard(),
            RenderMode::VR(vr) => self.render_vr(vr),
        }
    }

    fn render_standard(&mut self) -> Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        store: true,
                    },
                })],
                depth_stencil_attachment: self.depth_view.as_ref().map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn render_vr(&mut self, vr: &mut VRSystem) -> Result<()> {
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VR Render Encoder"),
        });

        // Get swapchain image layout
        let (width, height) = vr.get_swapchain_image_layout()
            .ok_or_else(|| anyhow::anyhow!("Failed to get swapchain layout"))?;

        // Create depth texture for VR
        let (_, depth_view) = Self::create_depth_texture(&self.device, width, height);

        // Create texture view for each eye
        for i in 0..2 {
            let view_attachment = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("VR Eye {} Texture", i)),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let view_attachment_view = view_attachment.create_view(&wgpu::TextureViewDescriptor::default());

            // Begin render pass for this eye
            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("VR Eye {} Render Pass", i)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_attachment_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });

                // TODO: Add actual rendering commands here
            }
        }

        // Submit command buffer
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_renderer_creation() {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let renderer = Renderer::new(&window).await;
        assert!(renderer.is_ok(), "Failed to create renderer: {:?}", renderer.err());
    }

    #[tokio::test]
    #[serial]
    async fn test_renderer_mode_selection() {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let renderer = Renderer::new(&window).await.unwrap();
        
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
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let mut renderer = Renderer::new(&window).await.unwrap();
        
        let new_size = winit::dpi::PhysicalSize::new(800, 600);
        renderer.resize(new_size);
        
        assert_eq!(renderer.config.width, 800);
        assert_eq!(renderer.config.height, 600);
    }
} 