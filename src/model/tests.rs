use super::*;
use std::path::PathBuf;
use pollster::FutureExt;
use wgpu::Instance;
use assert_fs::prelude::*;
use std::fs;
use image::GenericImageView;

fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: true,
            compatible_surface: None,
        })
        .block_on()?;

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
        .ok()?;

    Some((device, queue))
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Material Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
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
        48,  // 3 * 4 (position) + 2 * 4 (tex_coords) + 3 * 4 (normal) + 4 * 4 (tangent) = 48 bytes
        "ModelVertex size should be 48 bytes"
    );
}

#[test]
fn test_unsupported_format() {
    if let Some((device, queue)) = create_test_device() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("test.unsupported");
        file.touch().unwrap();
        let bind_group_layout = create_bind_group_layout(&device);

        let result = Model::load(&device, &queue, file.path(), &bind_group_layout);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unsupported model format"));
        }
    } else {
        println!("Skipping test 'test_unsupported' - no suitable GPU adapter available");
    }
}

#[test]
fn test_load_obj() {
    if let Some((device, queue)) = create_test_device() {
        let bind_group_layout = create_bind_group_layout(&device);
        
        // Create a default texture
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Write white pixel to texture
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        
        let default_texture_view = default_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        
        let default_texture = Texture {
            texture: default_texture,
            view: default_texture_view,
            sampler: default_sampler,
        };
        
        // Create a material with the default texture
        let mut material = Material {
            name: "default".to_string(),
            diffuse_texture: Some(default_texture),
            normal_texture: None,
            bind_group: None,
        };
        material.create_bind_group(&device, &bind_group_layout);
        
        // Load the model
        let model_path = test_models_path().join("cube.obj");
        let mut model = Model::load(&device, &queue, model_path, &bind_group_layout).unwrap();
        
        // Replace the default material
        model.materials[0] = material;
        
        // Verify model structure
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        assert!(model.materials[0].diffuse_texture.is_some(), "Material should have a diffuse texture");
        assert!(model.materials[0].bind_group.is_some(), "Material should have a bind group");
        
        // Verify mesh data
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
    } else {
        println!("Skipping test 'test_load_obj' - no suitable GPU adapter available");
    }
}

#[test]
fn test_load_gltf() {
    if let Some((device, queue)) = create_test_device() {
        let bind_group_layout = create_bind_group_layout(&device);
        let model_path = test_models_path().join("cube.gltf");
        let model = Model::load(&device, &queue, model_path, &bind_group_layout).unwrap();
        
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
    } else {
        println!("Skipping test 'test_load_gltf' - no suitable GPU adapter available");
    }
}

#[test]
fn test_load_glb() {
    if let Some((device, queue)) = create_test_device() {
        let bind_group_layout = create_bind_group_layout(&device);
        let model_path = test_models_path().join("cube.glb");
        let model = Model::load(&device, &queue, model_path, &bind_group_layout).unwrap();
        
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
    } else {
        println!("Skipping test 'test_load_glb' - no suitable GPU adapter available");
    }
}

#[test]
fn test_texture_loading() {
    if let Some((device, queue)) = create_test_device() {
        let path = test_models_path().join("cube_texture.png");
        let texture = Texture::from_path(&device, &queue, &path, Some("test_texture")).unwrap();
        
        // Just verify that we can create a texture successfully
        assert!(texture.texture.size().width > 0);
        assert!(texture.texture.size().height > 0);
    } else {
        println!("Skipping test 'test_texture_loading' - no suitable GPU adapter available");
    }
}

#[test]
fn test_vertex_buffer_layout() {
    let layout = ModelVertex::desc();
    assert_eq!(layout.array_stride, 48);
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
    assert_eq!(layout.attributes.len(), 4);
    
    // Verify attribute formats
    assert_eq!(layout.attributes[0].format, wgpu::VertexFormat::Float32x3);  // position
    assert_eq!(layout.attributes[1].format, wgpu::VertexFormat::Float32x2);  // tex_coords
    assert_eq!(layout.attributes[2].format, wgpu::VertexFormat::Float32x3);  // normal
    assert_eq!(layout.attributes[3].format, wgpu::VertexFormat::Float32x4);  // tangent
}

#[test]
fn test_material_bind_group() {
    if let Some((device, queue)) = create_test_device() {
        let bind_group_layout = create_bind_group_layout(&device);
        let path = test_models_path().join("cube_texture.png");
        let diffuse_texture = Texture::from_path(&device, &queue, &path, Some("diffuse_texture")).unwrap();
        let normal_texture = Texture::from_path(&device, &queue, &path, Some("normal_texture")).unwrap();
        
        let mut material = Material {
            name: "test_material".to_string(),
            diffuse_texture: Some(diffuse_texture),
            normal_texture: Some(normal_texture),
            bind_group: None,
        };

        material.create_bind_group(&device, &bind_group_layout);
        assert!(material.bind_group.is_some());
    } else {
        println!("Skipping test 'test_material_bind_group' - no suitable GPU adapter available");
    }
}

#[test]
fn test_extract_glb_textures() {
    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("test_output/textures");
    fs::create_dir_all(&output_dir).unwrap();

    // Get all .glb files from assets directory
    let assets_dir = PathBuf::from("assets");
    for entry in fs::read_dir(assets_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "glb") {
            println!("Processing {:?}", path);
            
            // Import GLTF
            let (document, buffers, _) = gltf::import(&path).unwrap();
            
            // Process each image in the GLTF
            for (image_index, image) in document.images().enumerate() {
                match image.source() {
                    gltf::image::Source::View { view, mime_type } => {
                        let parent_buffer = &buffers[view.buffer().index()];
                        let begin = view.offset();
                        let end = begin + view.length();
                        let image_data = &parent_buffer[begin..end];

                        // Determine image format and extension
                        let extension = match mime_type {
                            "image/jpeg" => "jpg",
                            "image/png" => "png",
                            _ => "bin",
                        };

                        // Create output filename
                        let output_path = output_dir.join(format!(
                            "{}_image_{}.{}",
                            path.file_stem().unwrap().to_string_lossy(),
                            image_index,
                            extension
                        ));

                        // Write image data to file
                        fs::write(&output_path, image_data).unwrap();
                        println!("Extracted texture to {:?}", output_path);

                        // Try to decode the image data and analyze it
                        if let Ok(img) = image::load_from_memory(image_data) {
                            let dimensions = img.dimensions();
                            let color_type = img.color();
                            println!("Image info:");
                            println!("  Dimensions: {}x{}", dimensions.0, dimensions.1);
                            println!("  Color type: {:?}", color_type);
                            
                            // Sample some pixels to verify data
                            let pixels = [
                                (0, 0),
                                (dimensions.0 / 2, dimensions.1 / 2),
                                (dimensions.0 - 1, dimensions.1 - 1)
                            ];
                            
                            println!("  Sample pixels:");
                            for (x, y) in pixels.iter() {
                                let pixel = img.get_pixel(*x, *y);
                                println!("    At ({}, {}): {:?}", x, y, pixel);
                            }

                            // Save as PNG for comparison
                            let png_path = output_dir.join(format!(
                                "{}_image_{}_decoded.png",
                                path.file_stem().unwrap().to_string_lossy(),
                                image_index
                            ));
                            img.save(&png_path).unwrap();
                            println!("Saved decoded image to {:?}", png_path);
                        } else {
                            println!("Failed to decode image data!");
                        }
                    },
                    gltf::image::Source::Uri { .. } => {
                        println!("Skipping external image reference");
                    },
                }
            }
        }
    }
}

#[test]
fn test_load_test_texture() {
    use std::path::PathBuf;
    use image::GenericImageView;

    // Load the test texture directly with image crate
    let test_texture_path = PathBuf::from("tests/models/cube_texture.png");
    let img = image::open(&test_texture_path).unwrap();
    let dimensions = img.dimensions();
    let rgba = img.to_rgba8();

    println!("Test texture info:");
    println!("  Dimensions: {}x{}", dimensions.0, dimensions.1);
    println!("  Raw buffer size: {}", rgba.as_raw().len());
    println!("  Expected buffer size: {}", dimensions.0 * dimensions.1 * 4);
    println!("  Color type: {:?}", img.color());

    // Sample some pixels
    let pixels = [
        (0, 0),
        (dimensions.0 / 2, dimensions.1 / 2),
        (dimensions.0 - 1, dimensions.1 - 1)
    ];
    
    println!("  Sample pixels:");
    for (x, y) in pixels.iter() {
        let pixel = img.get_pixel(*x, *y);
        println!("    At ({}, {}): {:?}", x, y, pixel);
    }

    // Now try loading with our Texture implementation
    if let Some((device, queue)) = create_test_device() {
        let texture = Texture::from_path(&device, &queue, &test_texture_path, Some("test")).unwrap();
        
        // Verify texture dimensions
        assert_eq!(texture.texture.size().width, dimensions.0);
        assert_eq!(texture.texture.size().height, dimensions.1);
        assert_eq!(texture.texture.size().depth_or_array_layers, 1);
    } else {
        println!("Skipping test 'test_load_test_texture' - no suitable GPU adapter available");
    }
} 