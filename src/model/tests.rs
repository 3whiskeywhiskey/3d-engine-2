use super::*;
use std::path::PathBuf;
use pollster::FutureExt;
use wgpu::Instance;
use assert_fs::prelude::*;

fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = Instance::default();
    instance.request_adapter(&wgpu::RequestAdapterOptions::default())
        .block_on()
        .expect("Failed to find an appropriate adapter")
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
        .expect("Failed to create device")
}

fn test_models_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("models");
    path
}

#[test]
fn test_model_vertex_size() {
    assert_eq!(
        std::mem::size_of::<ModelVertex>(),
        32,  // 3 * 4 (position) + 2 * 4 (tex_coords) + 3 * 4 (normal) = 32 bytes
        "ModelVertex size should be 32 bytes"
    );
}

#[test]
fn test_unsupported_format() {
    let (device, queue) = create_test_device();
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("test.unsupported");
    file.touch().unwrap();

    let result = Model::load(&device, &queue, file.path());
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Unsupported model format"));
    }
}

#[test]
fn test_load_obj() {
    let (device, queue) = create_test_device();
    let model_path = test_models_path().join("cube.obj");
    let model = Model::load(&device, &queue, model_path).unwrap();
    
    assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
    assert_eq!(model.materials.len(), 1, "Cube should have one material");
    
    let mesh = &model.meshes[0];
    assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
}

#[test]
fn test_load_gltf() {
    let (device, queue) = create_test_device();
    let model_path = test_models_path().join("cube.gltf");
    let model = Model::load(&device, &queue, model_path).unwrap();
    
    assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
    assert_eq!(model.materials.len(), 1, "Cube should have one material");
    
    let mesh = &model.meshes[0];
    assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
}

#[test]
fn test_load_glb() {
    let (device, queue) = create_test_device();
    let model_path = test_models_path().join("cube.glb");
    let model = Model::load(&device, &queue, model_path).unwrap();
    
    assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
    assert_eq!(model.materials.len(), 1, "Cube should have one material");
    
    let mesh = &model.meshes[0];
    assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
}

#[test]
fn test_texture_loading() {
    let (device, queue) = create_test_device();
    let path = test_models_path().join("cube_texture.png");
    let texture = Texture::from_path(&device, &queue, &path, Some("test_texture")).unwrap();
    
    // Just verify that we can create a texture successfully
    assert!(texture.texture.size().width > 0);
    assert!(texture.texture.size().height > 0);
}

#[test]
fn test_vertex_buffer_layout() {
    let layout = ModelVertex::desc();
    assert_eq!(layout.array_stride, 32);
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
    assert_eq!(layout.attributes.len(), 3);
}

#[test]
fn test_material_bind_group() {
    let (device, queue) = create_test_device();
    let path = test_models_path().join("cube_texture.png");
    let texture = Texture::from_path(&device, &queue, &path, Some("test_texture")).unwrap();
    
    let mut material = Material {
        name: "test_material".to_string(),
        diffuse_texture: Some(texture),
        bind_group: None,
        bind_group_layout: None,
    };

    material.create_bind_group(&device);
    assert!(material.bind_group.is_some());
    assert!(material.bind_group_layout.is_some());
} 