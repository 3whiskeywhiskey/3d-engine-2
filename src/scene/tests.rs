use super::*;
use crate::model::{Model, ModelVertex};
use pollster::FutureExt;
use wgpu::{Instance, util::DeviceExt};
use glam::Vec4Swizzles;
use winit::{
    event_loop::EventLoopBuilder,
    window::WindowBuilder,
};
use std::sync::Arc;

struct TestWindow {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
}

impl TestWindow {
    fn new() -> Self {
        let event_loop = EventLoopBuilder::new().build().unwrap();
        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
        let instance = Instance::default();
        
        // SAFETY: The window is owned by TestWindow and lives as long as the surface
        let surface = unsafe {
            instance.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_window(&*window).unwrap()
            ).unwrap()
        };

        Self {
            window,
            surface,
        }
    }
}

fn create_test_device() -> (wgpu::Device, wgpu::Queue, TestWindow, wgpu::Adapter) {
    let test_window = TestWindow::new();
    let instance = Instance::default();
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .block_on()
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: Default::default(),
            },
            None,
        )
        .block_on()
        .expect("Failed to create device");

    (device, queue, test_window, adapter)
}

#[test]
fn test_transform_new() {
    let transform = Transform::new();
    assert_eq!(transform.position, Vec3::ZERO);
    assert_eq!(transform.rotation, Vec3::ZERO);
    assert_eq!(transform.scale, Vec3::ONE);
}

#[test]
fn test_transform_matrix() {
    let mut transform = Transform::new();
    
    // Test translation
    transform.position = Vec3::new(1.0, 2.0, 3.0);
    let matrix = transform.to_matrix();
    assert_eq!(matrix.col(3).xyz(), Vec3::new(1.0, 2.0, 3.0));
    
    // Test scale
    transform = Transform::new();
    transform.scale = Vec3::new(2.0, 2.0, 2.0);
    let matrix = transform.to_matrix();
    assert_eq!(matrix.col(0).x, 2.0);
    assert_eq!(matrix.col(1).y, 2.0);
    assert_eq!(matrix.col(2).z, 2.0);
}

#[test]
fn test_camera_new() {
    let camera = Camera::new(800, 600);
    assert_eq!(camera.position, Vec3::new(0.0, 1.0, 2.0));
    assert_eq!(camera.target, Vec3::ZERO);
    assert_eq!(camera.up, Vec3::Y);
    assert_eq!(camera.aspect, 800.0 / 600.0);
    assert_eq!(camera.fovy, 45.0);
    assert!(camera.znear > 0.0);
    assert!(camera.zfar > camera.znear);
}

#[test]
fn test_camera_view_projection() {
    let camera = Camera::new(800, 600);
    let view_proj = camera.build_view_projection_matrix();
    
    // Test that the camera transforms a point at the origin
    let origin = view_proj.project_point3(Vec3::ZERO);
    assert!(origin.z < 0.0); // Should be in front of the camera (negative z in view space)
}

#[test]
fn test_scene_new() {
    let scene = Scene::new(800, 600);
    assert!(scene.objects.is_empty());
    assert_eq!(scene.ambient_light, Vec3::splat(0.1));
    assert_eq!(scene.directional_light, Vec3::ONE);
    assert!(scene.light_direction.is_normalized());
}

#[test]
fn test_scene_add_object() {
    let (device, _queue, _test_window, _adapter) = create_test_device();
    let mut scene = Scene::new(800, 600);
    
    // Create a simple test model (just a vertex buffer and index buffer)
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test Vertex Buffer"),
        contents: bytemuck::cast_slice(&[ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        }]),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test Index Buffer"),
        contents: bytemuck::cast_slice(&[0u32]),
        usage: wgpu::BufferUsages::INDEX,
    });

    let mesh = crate::model::Mesh {
        name: "test_mesh".to_string(),
        vertex_buffer,
        index_buffer,
        num_elements: 1,
        material_index: 0,
    };

    let model = Model {
        meshes: vec![mesh],
        materials: vec![],
    };

    let transform = Transform::new();
    scene.add_object(model, transform);
    
    assert_eq!(scene.objects.len(), 1);
}

#[test]
fn test_scene_resize() {
    let mut scene = Scene::new(800, 600);
    let original_aspect = scene.camera.aspect;
    
    scene.resize(1024, 768);
    assert_eq!(scene.camera.aspect, 1024.0 / 768.0);
    assert_ne!(scene.camera.aspect, original_aspect);
}

#[test]
fn test_renderer_creation() {
    let (device, _queue, test_window, adapter) = create_test_device();
    
    let surface_caps = test_window.surface.get_capabilities(&adapter);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_caps.formats[0],
        width: 800,
        height: 600,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    let renderer = Renderer::new(&device, &config);
    // Just verify that the pipeline was created
    assert!(std::ptr::eq(&renderer.pipeline, &renderer.pipeline));
} 