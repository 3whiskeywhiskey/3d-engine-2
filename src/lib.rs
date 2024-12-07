use std::sync::Arc;
use winit::{
    event_loop::EventLoop,
    window::Window,
};
use glam::Vec3;
use std::path::Path;

pub mod model;
pub mod scene;

use scene::{Scene, Renderer, camera::Camera, Transform};
use model::Model;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    pub scene: Scene,
    renderer: Renderer,
}

impl State {
    pub fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .unwrap();

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
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let camera = Camera::new(
            Vec3::new(0.0, 1.0, 2.0),
            size.width as f32 / size.height as f32,
        );
        let mut scene = Scene::new(camera);
        let renderer = Renderer::new(&device, &queue, &config);

        // Load test models
        let model1 = Model::load(
            &device,
            &queue,
            Path::new("assets/8b16ddeb-f011-4f13-bab7-615edd40aee9.glb"),
            &renderer.material_bind_group_layout,
        ).expect("Failed to load model 1");

        let model2 = Model::load(
            &device,
            &queue,
            Path::new("assets/cb088356-1d69-41a5-b46d-4bc22aafa1b7.glb"),
            &renderer.material_bind_group_layout,
        ).expect("Failed to load model 2");

        // Add multiple instances of each model with different transforms
        let positions = [
            Vec3::new(-3.0, 0.0, -3.0),
            Vec3::new(3.0, 0.0, -3.0),
            Vec3::new(-3.0, 0.0, 3.0),
            Vec3::new(3.0, 0.0, 3.0),
        ];

        let rotations = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, std::f32::consts::PI * 0.5, 0.0),
            Vec3::new(0.0, std::f32::consts::PI, 0.0),
            Vec3::new(0.0, std::f32::consts::PI * 1.5, 0.0),
        ];

        // Add instances of model1
        for i in 0..2 {
            let mut transform = Transform::new();
            transform.position = positions[i];
            transform.rotation = rotations[i];
            transform.scale = Vec3::splat(1.0);
            scene.add_object(model1.clone_with_device(&device, &queue, &renderer.material_bind_group_layout), transform);
        }

        // Add instances of model2
        for i in 2..4 {
            let mut transform = Transform::new();
            transform.position = positions[i];
            transform.rotation = rotations[i];
            transform.scale = Vec3::splat(1.0);
            scene.add_object(model2.clone_with_device(&device, &queue, &renderer.material_bind_group_layout), transform);
        }

        // Set up lighting
        scene.set_ambient_light(0.2);
        scene.set_directional_light(
            Vec3::new(1.0, 0.9, 0.8), // Warm sunlight color
            Vec3::new(-1.0, -1.0, -0.5).normalize(), // Sun direction
        );

        Self {
            surface,
            device,
            queue,
            config,
            window,
            scene,
            renderer,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.scene.resize(width, height);
            self.renderer.resize(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());
        self.renderer.render(&self.device, &self.queue, &view, &self.scene)?;
        frame.present();
        Ok(())
    }
} 