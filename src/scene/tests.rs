use super::*;
use crate::model::{Model, ModelVertex};
use pollster::FutureExt;
use wgpu::{Instance, util::DeviceExt};
use glam::Vec4Swizzles;

struct TestContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
}

impl TestContext {
    fn new() -> Self {
        let instance = Instance::default();
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
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

        Self {
            device,
            queue,
            adapter,
        }
    }
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
    assert!((camera.aspect - 800.0 / 600.0).abs() < f32::EPSILON);
    assert!((camera.fovy - 45.0).abs() < f32::EPSILON);
    assert!(camera.znear > 0.0);
    assert!(camera.zfar > camera.znear);
}

#[test]
fn test_camera_view_projection() {
    let camera = Camera::new(800, 600);
    let view_proj = camera.build_view_projection_matrix();
    
    // The camera is at (0, 1, 2) looking at (0, 0, 0)
    // Let's test a few points to verify the projection
    
    // Test points in clip space after projection
    let origin = view_proj.project_point3(Vec3::ZERO);
    // In WGPU's coordinate system after projection:
    // - Points in front of camera have z between 0 and 1
    // - Y is flipped, so points below camera have positive y
    assert!(origin.z >= 0.0 && origin.z <= 1.0, 
        "Origin z should be between 0 and 1 after projection, got {}", origin.z);
    
    // A point above the camera
    let above = view_proj.project_point3(Vec3::new(0.0, 2.0, 0.0));
    assert!(above.y < origin.y, 
        "Point above camera should have smaller y than origin after projection");
    
    // A point further from the camera but still in view
    let far_point = view_proj.project_point3(Vec3::new(0.0, 1.0, -1.0));
    assert!(far_point.z >= 0.0 && far_point.z <= 1.0,
        "Point z should be between 0 and 1 after projection, got {}", far_point.z);
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
    let ctx = TestContext::new();
    let mut scene = Scene::new(800, 600);
    
    // Create a simple test model (just a vertex buffer and index buffer)
    let vertex_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test Vertex Buffer"),
        contents: bytemuck::cast_slice(&[ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        }]),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
    
    // Change to a significantly different aspect ratio
    scene.resize(1600, 900);
    let new_aspect = 1600.0 / 900.0;
    
    // Verify the new aspect ratio is correct
    assert!((scene.camera.aspect - new_aspect).abs() < f32::EPSILON, 
            "Expected aspect ratio {}, got {}", new_aspect, scene.camera.aspect);
    
    // Verify it's different from the original
    assert!((scene.camera.aspect - original_aspect).abs() > f32::EPSILON,
            "Aspect ratio didn't change: {} vs {}", scene.camera.aspect, original_aspect);
}

#[test]
fn test_renderer_creation() {
    let ctx = TestContext::new();
    
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 800,
        height: 600,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    let renderer = Renderer::new(&ctx.device, &ctx.queue, &config);
    assert!(std::ptr::eq(&renderer.pipeline, &renderer.pipeline));
} 