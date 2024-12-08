use std::sync::Arc;
use winit::window::Window;
use glam::Vec3;
use std::path::Path;

pub mod model;
pub mod scene;
pub mod vr;
pub mod renderer;

use scene::{Scene, camera::Camera, Transform};
use model::{Model, ModelVertex};
use renderer::{Renderer, ForcedMode};

pub struct State {
    window: Arc<Window>,
    pub scene: Scene,
    renderer: Renderer,
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

        let camera = Camera::new(
            Vec3::new(0.0, 8.0, 16.0),
            size.width as f32 / size.height as f32,
        );
        let mut scene = Scene::new(camera);
        let renderer = Renderer::new(device, queue, &config, surface, forced_mode);

        // Add floor plane (20x20 meters)
        let floor_vertices = vec![
            ModelVertex { 
                position: [-10.0, 0.0, -10.0], 
                normal: [0.0, 1.0, 0.0], 
                tex_coords: [0.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            ModelVertex { 
                position: [10.0, 0.0, -10.0], 
                normal: [0.0, 1.0, 0.0], 
                tex_coords: [1.0, 0.0],  // One full texture repeat across 20 meters
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            ModelVertex { 
                position: [10.0, 0.0, 10.0], 
                normal: [0.0, 1.0, 0.0], 
                tex_coords: [1.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
            ModelVertex { 
                position: [-10.0, 0.0, 10.0], 
                normal: [0.0, 1.0, 0.0], 
                tex_coords: [0.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
            },
        ];

        let floor_indices = vec![0, 2, 1, 0, 3, 2];

        // Create checkerboard texture
        let texture_size = 512u32; // Larger texture for better quality
        let texture_data = create_checkerboard_texture(texture_size);
        
        let floor_texture = renderer.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Floor Texture"),
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        renderer.queue().write_texture(
            floor_texture.as_image_copy(),
            &texture_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
        );

        let floor_texture_view = floor_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create floor model
        let floor_model = Model::from_vertices(
            renderer.device(),
            renderer.queue(),
            &floor_vertices,
            &floor_indices,
            floor_texture_view,
            &renderer.material_bind_group_layout,
        );

        // Add floor to scene with identity transform
        let floor_transform = Transform::new();
        scene.add_object(floor_model, floor_transform);

        // Load test models
        let model1 = Model::load(
            renderer.device(),
            renderer.queue(),
            Path::new("assets/2c0f9e16-66c8-4891-bfb6-d79394ee56b8.glb"),
            &renderer.material_bind_group_layout,
        ).expect("Failed to load model 1");

        let model2 = Model::load(
            renderer.device(),
            renderer.queue(),
            Path::new("assets/f411cb1d-8c7f-4863-926a-40b8242bd166.glb"),
            &renderer.material_bind_group_layout,
        ).expect("Failed to load model 2");

        // Calculate Y offsets to place models on floor
        // We need to offset by the negative of the minimum Y coordinate to place the bottom at y=0
        let model1_y_offset = -model1.bounds_min[1];
        let model2_y_offset = -model2.bounds_min[1];

        // Add multiple instances of each model with different transforms
        let positions = [
            Vec3::new(-3.0, model1_y_offset, -3.0),
            Vec3::new(3.0, model1_y_offset, -3.0),
            Vec3::new(-3.0, model2_y_offset, 3.0),
            Vec3::new(3.0, model2_y_offset, 3.0),
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
            scene.add_object(model1.clone_with_device(renderer.device(), renderer.queue(), &renderer.material_bind_group_layout), transform);
        }

        // Add instances of model2
        for i in 2..4 {
            let mut transform = Transform::new();
            transform.position = positions[i];
            transform.rotation = rotations[i];
            transform.scale = Vec3::splat(1.0);
            scene.add_object(model2.clone_with_device(renderer.device(), renderer.queue(), &renderer.material_bind_group_layout), transform);
        }

        // Set up more dramatic lighting
        scene.set_ambient_light(0.3); // Increase ambient light
        scene.set_directional_light(
            Vec3::new(1.0, 1.0, 1.0), // White light
            Vec3::new(-0.5, -1.0, -0.5).normalize(), // Light coming from above and slightly to the side
        );

        Self {
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
            self.renderer.resize(winit::dpi::PhysicalSize::new(width, height));
            self.scene.resize(width, height);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.render(&self.scene).map_err(|e| {
            log::error!("Render error: {}", e);
            wgpu::SurfaceError::Lost
        })
    }
}

fn create_checkerboard_texture(size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let squares_per_side = 20; // We want 20x20 squares for our 20x20 meter floor
    let square_size = size / squares_per_side;

    for y in 0..size {
        for x in 0..size {
            let square_x = x / square_size;
            let square_y = y / square_size;
            let is_white = (square_x + square_y) % 2 == 0;
            
            let color = if is_white {
                [200u8, 200u8, 200u8, 255u8] // Light gray
            } else {
                [120u8, 120u8, 120u8, 255u8] // Dark gray
            };
            
            data.extend_from_slice(&color);
        }
    }
    data
} 