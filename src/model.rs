use std::path::Path;
use anyhow::Result;
use wgpu::util::DeviceExt;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::collections::HashMap;

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

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<wgpu::Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
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
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let (_document, _buffers, _images) = gltf::import(path)?;
        
        // TODO: Implement GLTF loading
        todo!("Implement GLTF loading")
    }

    fn load_obj(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
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
                        // TODO: Load materials from MTL file
                        materials.push(Material {
                            name: "default".to_string(),
                            diffuse_texture: None,
                            bind_group: None,
                        });
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
} 