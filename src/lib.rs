pub mod renderer;
pub mod scene;
pub mod model;
pub mod vr;
pub mod demo;

pub use renderer::{Renderer, ForcedMode};
pub use scene::{Scene, Camera, Transform};
pub use model::{Model, ModelVertex};

use std::sync::Arc;
use winit::window::Window;

pub struct State {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    pub scene: Scene,
}

impl State {
    pub fn new(window: Window, forced_mode: ForcedMode) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        println!("Creating WGPU instance...");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: if cfg!(target_os = "macos") {
                wgpu::Backends::METAL
            } else {
                wgpu::Backends::VULKAN
            },
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        println!("Creating surface...");
        println!("Window info - width: {}, height: {}", size.width, size.height);
        let surface = instance.create_surface(window.clone())
            .expect("Failed to create surface");

        println!("Requesting adapter...");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find appropriate adapter");

        let info = adapter.get_info();
        println!("Using adapter: {:?}", info);
        println!("Adapter backend: {:?}", info.backend);
        println!("Adapter device: {}", info.device);
        println!("Adapter driver: {}", info.driver);
        println!("Adapter driver info: {}", info.driver_info);

        let mut limits = wgpu::Limits::default();
        if cfg!(target_os = "macos") {
            // Ensure we don't exceed Metal's limits
            limits.max_texture_dimension_2d = 16384;
            limits.max_bind_groups = 4;
        }

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Primary Device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        println!("Surface capabilities: {:?}", surface_caps);
        
        let surface_format = if cfg!(target_os = "macos") {
            // Prefer BGRA8UnormSrgb for Metal
            surface_caps.formats.iter()
                .copied()
                .find(|f| f == &wgpu::TextureFormat::Bgra8UnormSrgb)
                .unwrap_or(surface_caps.formats[0])
        } else {
            surface_caps.formats.iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(surface_caps.formats[0])
        };

        println!("Selected surface format: {:?}", surface_format);

        let present_mode = if cfg!(target_os = "macos") {
            // Prefer immediate mode on Metal for lower latency
            surface_caps.present_modes.iter()
                .copied()
                .find(|&mode| mode == wgpu::PresentMode::Immediate)
                .unwrap_or(surface_caps.present_modes[0])
        } else {
            surface_caps.present_modes[0]
        };

        println!("Selected present mode: {:?}", present_mode);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let renderer = Renderer::new(device, queue, &config, Some(surface), forced_mode);
        let scene = demo::create_demo_scene(&renderer, size.width, size.height);

        Self {
            window,
            renderer,
            scene,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.renderer.resize(new_size);
            self.scene.resize(new_size.width, new_size.height);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.render(&self.scene)
            .map_err(|e| {
                log::error!("Render error: {}", e);
                wgpu::SurfaceError::Lost
            })
    }
} 