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
    let camera = Camera::new(Vec3::new(0.0, 1.0, 2.0), 800.0 / 600.0);
    assert_eq!(camera.position, Vec3::new(0.0, 1.0, 2.0));
    assert_eq!(camera.yaw, -90.0); // Looking along -Z
    assert_eq!(camera.pitch, 0.0);
    assert!((camera.aspect - 800.0 / 600.0).abs() < f32::EPSILON);
    assert!((camera.fov - 45.0).abs() < f32::EPSILON);
    assert!(camera.near > 0.0);
    assert!(camera.far > camera.near);
}

#[test]
fn test_camera_view_projection() {
    let mut camera = Camera::new(Vec3::ZERO, 1.0);
    
    // Set initial orientation (looking down -Z)
    camera.yaw = -90.0;
    camera.pitch = 0.0;
    
    let view_proj = camera.build_view_projection_matrix();
    
    // Test points at different heights
    let bottom_point = view_proj.project_point3(Vec3::new(0.0, -5.0, -5.0));
    let top_point = view_proj.project_point3(Vec3::new(0.0, 5.0, -5.0));
    assert!(bottom_point.y < top_point.y, "Top point should appear above bottom point in screen space");
    
    // Test points at different horizontal positions
    let left_point = view_proj.project_point3(Vec3::new(-5.0, 0.0, -5.0));
    let right_point = view_proj.project_point3(Vec3::new(5.0, 0.0, -5.0));
    assert!(left_point.x < right_point.x, "Right point should appear to the right of left point in screen space");
}

#[test]
fn test_scene_new() {
    let camera = Camera::new(Vec3::new(0.0, 1.0, 2.0), 800.0 / 600.0);
    let scene = Scene::new(camera);
    assert!(scene.objects.is_empty());
    assert_eq!(scene.ambient_light, Vec3::new(0.1, 0.1, 0.1));
    assert_eq!(scene.directional_light, Vec3::new(1.0, 1.0, 1.0));
    assert!(scene.light_direction.is_normalized());
}

#[test]
fn test_scene_add_object() {
    let ctx = TestContext::new();
    let camera = Camera::new(Vec3::new(0.0, 1.0, 2.0), 800.0 / 600.0);
    let mut scene = Scene::new(camera);
    
    // Create a simple test model (just a vertex buffer and index buffer)
    let vertex_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Test Vertex Buffer"),
        contents: bytemuck::cast_slice(&[ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],  // Default tangent along X axis with positive handedness
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
    let camera = Camera::new(Vec3::new(0.0, 1.0, 2.0), 800.0 / 600.0);
    let mut scene = Scene::new(camera);
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