use std::path::Path;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::collections::HashMap;
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

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn clone_with_device(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            meshes: self.meshes.iter().map(|mesh| mesh.clone_with_device(device, queue)).collect(),
            materials: self.materials.iter().map(|material| material.clone_with_device(device)).collect(),
        }
    }

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
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let (document, buffers, images) = gltf::import(path)?;

        let mut meshes = Vec::new();
        let mut materials = Vec::new();

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

            let mut material = Material {
                name: material.name().unwrap_or("").to_string(),
                diffuse_texture,
                bind_group: None,
                bind_group_layout: None,
            };

            // Create bind group if we have a texture
            if material.diffuse_texture.is_some() {
                material.create_bind_group(device);
            }

            materials.push(material);
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&obj_data.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
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