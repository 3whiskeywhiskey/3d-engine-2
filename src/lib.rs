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

pub struct State<'a> {
    pub window: Arc<Window>,
    pub renderer: Renderer<'a>,
    pub scene: Scene,
}

impl<'a> State<'a> {
    pub async fn new(window: Window, forced_mode: ForcedMode) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        let surface = instance.create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find appropriate adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Primary Device"),
                required_features: wgpu::Features::MULTIVIEW 
                    | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::DEPTH_CLIP_CONTROL
                    | wgpu::Features::MULTI_DRAW_INDIRECT
                    | wgpu::Features::TEXTURE_BINDING_ARRAY 
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                required_limits: wgpu::Limits {
                    max_push_constant_size: 128,
                    max_texture_array_layers: 32,  // Required for multiview
                    max_vertex_buffers: 8,
                    max_storage_buffers_per_shader_stage: 8,
                    max_storage_textures_per_shader_stage: 8,
                    max_sampled_textures_per_shader_stage: 16,
                    max_samplers_per_shader_stage: 16,
                    max_bindings_per_bind_group: 1000,
                    max_uniform_buffers_per_shader_stage: 12,
                    max_uniform_buffer_binding_size: 16384,
                    max_storage_buffer_binding_size: 128 * 1024 * 1024,  // 128MB
                    max_vertex_buffer_array_stride: 2048,
                    max_compute_workgroup_storage_size: 16384,
                    max_compute_invocations_per_workgroup: 256,
                    max_compute_workgroup_size_x: 256,
                    max_compute_workgroup_size_y: 256,
                    max_compute_workgroup_size_z: 64,
                    max_compute_workgroups_per_dimension: 65535,
                    max_texture_dimension_1d: 8192,
                    max_texture_dimension_2d: 8192,
                    max_texture_dimension_3d: 2048,
                    ..wgpu::Limits::default()
                },
                memory_hints: Default::default(),
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let renderer = Renderer::new(
            Arc::new(device),
            Arc::new(queue),
            &config,
            Some(surface),
            forced_mode
        );
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

    pub fn update(&mut self) {
        self.scene.update();
        if let Some(vr) = self.renderer.get_vr_system() {
            if let Err(e) = vr.update() {
                println!("Failed to update VR session state: {}", e);
            }
        }
    }
} 