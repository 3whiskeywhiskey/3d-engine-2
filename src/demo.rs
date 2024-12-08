use std::path::Path;
use glam::Vec3;
use crate::{Scene, Camera, Transform, Model, ModelVertex, Renderer};

pub fn create_demo_scene(renderer: &Renderer, width: u32, height: u32) -> Scene {
    let camera = Camera::new(
        Vec3::new(0.0, 8.0, 16.0),
        width as f32 / height as f32,
    );
    let mut scene = Scene::new(camera);

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

    scene
}

fn create_checkerboard_texture(size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let square_size = size / 8; // 8x8 checkerboard

    for y in 0..size {
        for x in 0..size {
            let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;
            let color = if is_white { 200u8 } else { 50u8 };
            data.extend_from_slice(&[color, color, color, 255]);
        }
    }

    data
} 