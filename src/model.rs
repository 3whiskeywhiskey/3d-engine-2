use std::path::Path;
use anyhow::Result;
use wgpu::util::DeviceExt;

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
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let (document, buffers, images) = gltf::import(path)?;
        
        // TODO: Implement GLTF loading
        // This will involve:
        // 1. Loading meshes from the GLTF buffers
        // 2. Loading materials and textures
        // 3. Converting everything to our internal format
        
        todo!("Implement GLTF loading")
    }

    fn load_obj(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        // TODO: Implement OBJ loading
        todo!("Implement OBJ loading")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::io::Write;
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