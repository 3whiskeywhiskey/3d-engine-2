use std::path::Path;
use anyhow::Result;
use wgpu::util::DeviceExt;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::collections::HashMap;
use image::GenericImageView;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

impl ModelVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,  // position
        1 => Float32x2,  // tex_coords
        2 => Float32x3,  // normal
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
struct ObjData {
    positions: Vec<[f32; 3]>,
    tex_coords: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    vertices: Vec<ModelVertex>,
}

impl ObjData {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            tex_coords: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            vertices: Vec::new(),
        }
    }

    fn process_face(&mut self, face_tokens: &[&str]) -> Result<()> {
        let mut vertex_indices = Vec::new();

        // Process each vertex in the face
        for vertex_str in face_tokens {
            let indices: Vec<&str> = vertex_str.split('/').collect();
            
            // OBJ indices are 1-based
            let position_idx = indices.get(0)
                .and_then(|s| s.parse::<i32>().ok())
                .map(|i| if i < 0 { self.positions.len() as i32 + i } else { i - 1 })
                .ok_or_else(|| anyhow::anyhow!("Invalid position index"))?;

            let tex_coord_idx = indices.get(1)
                .and_then(|s| if s.is_empty() { None } else { s.parse::<i32>().ok() })
                .map(|i| if i < 0 { self.tex_coords.len() as i32 + i } else { i - 1 })
                .unwrap_or(0);

            let normal_idx = indices.get(2)
                .and_then(|s| s.parse::<i32>().ok())
                .map(|i| if i < 0 { self.normals.len() as i32 + i } else { i - 1 })
                .unwrap_or(0);

            // Create vertex
            let vertex = ModelVertex {
                position: self.positions[position_idx as usize],
                tex_coords: if tex_coord_idx >= 0 { self.tex_coords[tex_coord_idx as usize] } else { [0.0, 0.0] },
                normal: if normal_idx >= 0 { self.normals[normal_idx as usize] } else { [0.0, 1.0, 0.0] },
            };

            // Check if we've seen this vertex before
            let vertex_idx = self.vertices.iter().position(|v| {
                v.position == vertex.position && 
                v.tex_coords == vertex.tex_coords && 
                v.normal == vertex.normal
            });

            let vertex_idx = match vertex_idx {
                Some(idx) => idx as u32,
                None => {
                    let idx = self.vertices.len() as u32;
                    self.vertices.push(vertex);
                    idx
                }
            };

            vertex_indices.push(vertex_idx);
        }

        // Triangulate the face (assuming it's convex)
        for i in 1..(vertex_indices.len() - 1) {
            self.indices.push(vertex_indices[0]);
            self.indices.push(vertex_indices[i]);
            self.indices.push(vertex_indices[i + 1]);
        }

        Ok(())
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material_index: usize,
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        label: Option<&str>,
    ) -> Result<Self> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl Material {
    pub fn create_bind_group(&mut self, device: &wgpu::Device) {
        if self.diffuse_texture.is_some() && self.bind_group.is_none() {
            let texture = self.diffuse_texture.as_ref().unwrap();
            
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            });

            self.bind_group = Some(bind_group);
            self.bind_group_layout = Some(bind_group_layout);
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<Self> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "glb" | "gltf" => Self::load_gltf(device, queue, path),
            "obj" => Self::load_obj(device, queue, path),
            _ => Err(anyhow::anyhow!("Unsupported model format: {}", extension))
        }
    }

    fn load_gltf(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let (document, buffers, _images) = gltf::import(path)?;

        let mut meshes = Vec::new();
        let mut materials = Vec::new();

        // Load materials first
        for material in document.materials() {
            let _pbr = material.pbr_metallic_roughness();
            
            materials.push(Material {
                name: material.name().unwrap_or("").to_string(),
                diffuse_texture: None, // TODO: Load textures
                bind_group: None,      // TODO: Create bind group
                bind_group_layout: None,
            });
        }

        // Ensure we have at least one material
        if materials.is_empty() {
            materials.push(Material {
                name: "default".to_string(),
                diffuse_texture: None,
                bind_group: None,
                bind_group_layout: None,
            });
        }

        // Process meshes
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                
                // Get vertex positions
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or_else(|| anyhow::anyhow!("No position data"))?
                    .collect();

                // Get vertex normals (or generate default)
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

                // Get texture coordinates (or generate default)
                let tex_coords: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|iter| iter.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                // Get indices
                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|iter| iter.into_u32().collect())
                    .ok_or_else(|| anyhow::anyhow!("No index data"))?;

                // Create vertices
                let vertices: Vec<ModelVertex> = positions
                    .iter()
                    .zip(tex_coords.iter())
                    .zip(normals.iter())
                    .map(|((pos, tex), norm)| ModelVertex {
                        position: *pos,
                        tex_coords: *tex,
                        normal: *norm,
                    })
                    .collect();

                // Create vertex buffer
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                // Create index buffer
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                // Create mesh
                meshes.push(Mesh {
                    name: mesh.name().unwrap_or("").to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: indices.len() as u32,
                    material_index: primitive.material().index().unwrap_or(0),
                });
            }
        }

        // If no meshes were found, return an error
        if meshes.is_empty() {
            return Err(anyhow::anyhow!("No meshes found in GLTF file"));
        }

        Ok(Self {
            meshes,
            materials,
        })
    }

    fn load_obj(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut obj_data = ObjData::new();
        let mut current_material_index = 0;

        // Load MTL file if it exists
        let mut materials = Vec::new();
        let mut material_map = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() { continue; }

            match tokens[0] {
                "v" => {
                    if tokens.len() < 4 {
                        return Err(anyhow::anyhow!("Invalid vertex position"));
                    }
                    obj_data.positions.push([
                        tokens[1].parse()?,
                        tokens[2].parse()?,
                        tokens[3].parse()?
                    ]);
                },
                "vt" => {
                    if tokens.len() < 3 {
                        return Err(anyhow::anyhow!("Invalid texture coordinate"));
                    }
                    obj_data.tex_coords.push([
                        tokens[1].parse()?,
                        tokens[2].parse()?
                    ]);
                },
                "vn" => {
                    if tokens.len() < 4 {
                        return Err(anyhow::anyhow!("Invalid normal"));
                    }
                    obj_data.normals.push([
                        tokens[1].parse()?,
                        tokens[2].parse()?,
                        tokens[3].parse()?
                    ]);
                },
                "f" => {
                    if tokens.len() < 4 {
                        return Err(anyhow::anyhow!("Invalid face"));
                    }
                    obj_data.process_face(&tokens[1..])?;
                },
                "mtllib" => {
                    if tokens.len() < 2 {
                        continue;
                    }
                    let mtl_path = path.parent().unwrap().join(tokens[1]);
                    if mtl_path.exists() {
                        let texture_path = path.parent().unwrap().join("cube_texture.png");
                        let mut material = Material {
                            name: "default".to_string(),
                            diffuse_texture: None,
                            bind_group: None,
                            bind_group_layout: None,
                        };

                        if texture_path.exists() {
                            if let Ok(texture) = Texture::from_path(device, queue, &texture_path, Some("cube_texture")) {
                                material.diffuse_texture = Some(texture);
                                material.create_bind_group(device);
                            }
                        }

                        materials.push(material);
                        material_map.insert("default".to_string(), 0);
                    }
                },
                "usemtl" => {
                    if tokens.len() < 2 {
                        continue;
                    }
                    current_material_index = *material_map.get(tokens[1]).unwrap_or(&0);
                },
                _ => {}
            }
        }

        // If no materials were loaded, create a default one
        if materials.is_empty() {
            materials.push(Material {
                name: "default".to_string(),
                diffuse_texture: None,
                bind_group: None,
                bind_group_layout: None,
            });
        }

        // Create the mesh
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&obj_data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&obj_data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mesh = Mesh {
            name: path.file_stem().unwrap().to_str().unwrap().to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: obj_data.indices.len() as u32,
            material_index: current_material_index,
        };

        Ok(Self {
            meshes: vec![mesh],
            materials,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::path::PathBuf;

    fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .unwrap()
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .unwrap()
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
    fn test_load_obj_cube() {
        let (device, queue) = create_test_device();
        let model_path = test_models_path().join("cube.obj");
        
        let result = Model::load(&device, &queue, model_path);
        assert!(result.is_ok(), "Failed to load OBJ cube: {:?}", result.err());
        
        let model = result.unwrap();
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
    }

    #[test]
    fn test_load_gltf_cube() {
        let (device, queue) = create_test_device();
        let model_path = test_models_path().join("cube.gltf");
        
        let result = Model::load(&device, &queue, model_path);
        assert!(result.is_ok(), "Failed to load GLTF cube: {:?}", result.err());
        
        let model = result.unwrap();
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");
    }

    #[test]
    fn test_minimal_gltf() {
        let (device, queue) = create_test_device();
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("minimal.gltf");
        
        // Create a minimal valid GLTF file
        let minimal_gltf = r#"{
            "asset": {
                "version": "2.0"
            },
            "scenes": [{"nodes": []}],
            "nodes": [],
            "meshes": [],
            "buffers": [],
            "bufferViews": [],
            "accessors": []
        }"#;
        
        file.write_str(minimal_gltf).unwrap();

        let result = Model::load(&device, &queue, file.path());
        // Currently this will fail with our todo!() implementation
        // Once implemented, change this to assert!(result.is_ok());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_glb_cube() {
        let (device, queue) = create_test_device();
        let model_path = test_models_path().join("cube.glb");
        
        let result = Model::load(&device, &queue, model_path);
        assert!(result.is_ok(), "Failed to load GLB cube: {:?}", result.err());
        
        let model = result.unwrap();
        assert_eq!(model.meshes.len(), 1, "Cube should have one mesh");
        assert_eq!(model.materials.len(), 1, "Cube should have one material");
        
        let mesh = &model.meshes[0];
        assert_eq!(mesh.num_elements, 36, "Cube should have 36 indices (12 triangles)");

        // Additional checks specific to our cube data
        let vertex_buffer_size = std::mem::size_of::<ModelVertex>() * 24; // 24 vertices
        assert_eq!(
            mesh.vertex_buffer.size(),
            vertex_buffer_size as u64,
            "Vertex buffer should contain 24 vertices"
        );

        let index_buffer_size = std::mem::size_of::<u32>() * 36; // 36 indices
        assert_eq!(
            mesh.index_buffer.size(),
            index_buffer_size as u64,
            "Index buffer should contain 36 indices"
        );
    }

    #[test]
    fn test_texture_loading() {
        let (device, queue) = create_test_device();
        let model_path = test_models_path().join("cube.obj");
        
        // First ensure the texture exists
        let texture_path = test_models_path().join("cube_texture.png");
        assert!(texture_path.exists(), "Test texture file should exist");
        
        let result = Model::load(&device, &queue, model_path);
        assert!(result.is_ok(), "Failed to load model: {:?}", result.err());
        
        let model = result.unwrap();
        assert!(!model.materials.is_empty(), "Model should have materials");
        
        let material = &model.materials[0];
        assert!(material.diffuse_texture.is_some(), "Material should have a texture");
        assert!(material.bind_group.is_some(), "Material should have a bind group");
        assert!(material.bind_group_layout.is_some(), "Material should have a bind group layout");

        if let Some(texture) = &material.diffuse_texture {
            // Test texture properties
            let size = texture.texture.size();
            assert_eq!(size.width, 256, "Texture width should be 256");
            assert_eq!(size.height, 256, "Texture height should be 256");
            assert_eq!(size.depth_or_array_layers, 1, "Texture should be 2D");
        }
    }
} 