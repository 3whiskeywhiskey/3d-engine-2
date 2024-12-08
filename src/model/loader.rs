use std::path::Path;
use std::io::{BufReader, BufRead};
use std::fs::File;
use anyhow::Result;
use wgpu::util::DeviceExt;

use super::{Mesh, Material, ModelVertex, Texture};

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

            // Create vertex with default tangent
            let vertex = ModelVertex {
                position: self.positions[position_idx as usize],
                tex_coords: if tex_coord_idx >= 0 { self.tex_coords[tex_coord_idx as usize] } else { [0.0, 0.0] },
                normal: if normal_idx >= 0 { self.normals[normal_idx as usize] } else { [0.0, 1.0, 0.0] },
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent along X axis
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

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

impl Model {
    // Calculate the bounding box for a set of vertices
    fn calculate_bounds(vertices: &[ModelVertex]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];

        for vertex in vertices {
            for i in 0..3 {
                min[i] = min[i].min(vertex.position[i]);
                max[i] = max[i].max(vertex.position[i]);
            }
        }

        (min, max)
    }

    pub fn clone_with_device(&self, device: &wgpu::Device, queue: &wgpu::Queue, material_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        Self {
            meshes: self.meshes.iter().map(|mesh| mesh.clone_with_device(device, queue)).collect(),
            materials: self.materials.iter().map(|material| material.clone_with_device(device, queue, material_bind_group_layout)).collect(),
            bounds_min: self.bounds_min,
            bounds_max: self.bounds_max,
        }
    }

    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
        material_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "glb" | "gltf" => Self::load_gltf(device, queue, path, material_bind_group_layout),
            "obj" => Self::load_obj(device, queue, path, material_bind_group_layout),
            _ => Err(anyhow::anyhow!("Unsupported model format: {}", extension))
        }
    }

    fn load_gltf(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
        material_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let (document, buffers, images) = gltf::import(path)?;

        let mut meshes = Vec::new();
        let mut materials = Vec::new();
        // Track overall bounds of the model
        let mut overall_min = [f32::INFINITY; 3];
        let mut overall_max = [f32::NEG_INFINITY; 3];

        // Load materials first
        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();
            
            // Try to load the base color texture
            let mut diffuse_texture = None;
            if let Some(info) = pbr.base_color_texture() {
                let texture = info.texture();
                let source = texture.source().index();
                if let Ok(texture) = Texture::from_gltf_image(
                    device,
                    queue,
                    &images[source],
                    Some(&format!("texture_{}", source))
                ) {
                    diffuse_texture = Some(texture);
                }
            }

            // Try to load the normal map
            let mut normal_texture = None;
            if let Some(normal) = material.normal_texture() {
                let texture = normal.texture();
                let source = texture.source().index();
                if let Ok(texture) = Texture::from_gltf_image(
                    device,
                    queue,
                    &images[source],
                    Some(&format!("normal_{}", source))
                ) {
                    normal_texture = Some(texture);
                }
            }

            let mut material = Material {
                name: material.name().unwrap_or("").to_string(),
                diffuse_texture,
                normal_texture,
                bind_group: None,
            };

            // Create bind group if we have textures
            material.create_bind_group(device, material_bind_group_layout);
            materials.push(material);
        }

        // Ensure we have at least one material
        if materials.is_empty() {
            materials.push(Material {
                name: "default".to_string(),
                diffuse_texture: None,
                normal_texture: None,
                bind_group: None,
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

                // Get tangents (or generate default)
                let tangents: Vec<[f32; 4]> = reader
                    .read_tangents()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| {
                        // Generate default tangents (this is a simplified version)
                        positions.iter().map(|_| [1.0, 0.0, 0.0, 1.0]).collect()
                    });

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
                    .zip(tangents.iter())
                    .map(|(((pos, tex), norm), tan)| ModelVertex {
                        position: *pos,
                        tex_coords: *tex,
                        normal: *norm,
                        tangent: *tan,
                    })
                    .collect();

                // Update the model's bounding box
                let (mesh_min, mesh_max) = Self::calculate_bounds(&vertices);
                for i in 0..3 {
                    overall_min[i] = overall_min[i].min(mesh_min[i]);
                    overall_max[i] = overall_max[i].max(mesh_max[i]);
                }

                // Create vertex buffer
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
                });

                // Create index buffer
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
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
            bounds_min: overall_min,
            bounds_max: overall_max,
        })
    }

    fn load_obj(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        path: &Path,
        material_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let mut obj_data = ObjData::new();
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Parse OBJ file
        for line in reader.lines() {
            let line = line?;
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0] {
                "v" => {
                    if tokens.len() < 4 {
                        continue;
                    }
                    let x = tokens[1].parse::<f32>()?;
                    let y = tokens[2].parse::<f32>()?;
                    let z = tokens[3].parse::<f32>()?;
                    obj_data.positions.push([x, y, z]);
                }
                "vt" => {
                    if tokens.len() < 3 {
                        continue;
                    }
                    let u = tokens[1].parse::<f32>()?;
                    let v = tokens[2].parse::<f32>()?;
                    obj_data.tex_coords.push([u, v]);
                }
                "vn" => {
                    if tokens.len() < 4 {
                        continue;
                    }
                    let x = tokens[1].parse::<f32>()?;
                    let y = tokens[2].parse::<f32>()?;
                    let z = tokens[3].parse::<f32>()?;
                    obj_data.normals.push([x, y, z]);
                }
                "f" => {
                    if tokens.len() < 4 {
                        continue;
                    }
                    obj_data.process_face(&tokens[1..])?;
                }
                _ => {}
            }
        }

        // Calculate model bounds
        let (overall_min, overall_max) = Self::calculate_bounds(&obj_data.vertices);

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&obj_data.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&obj_data.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
        });

        // Create mesh
        let mesh = Mesh {
            name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: obj_data.indices.len() as u32,
            material_index: 0,
        };

        // Create default material
        let mut material = Material {
            name: "default".to_string(),
            diffuse_texture: None,
            normal_texture: None,
            bind_group: None,
        };

        // Create bind group
        material.create_bind_group(device, material_bind_group_layout);

        Ok(Self {
            meshes: vec![mesh],
            materials: vec![material],
            bounds_min: overall_min,
            bounds_max: overall_max,
        })
    }

    pub fn extract_glb_textures(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _data: &[u8],
        _material_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Vec<Material>> {
        // Implementation for extracting textures from GLB
        unimplemented!()
    }

    pub fn load_texture(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _path: &Path,
        _label: Option<&str>,
    ) -> Result<Texture> {
        // Implementation for loading texture
        unimplemented!()
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for mesh in &self.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            
            if let Some(material) = self.materials.get(mesh.material_index as usize) {
                if let Some(bind_group) = &material.bind_group {
                    render_pass.set_bind_group(1, bind_group, &[]);
                }
            }
            
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
} 