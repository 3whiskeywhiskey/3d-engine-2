use anyhow::Result;
use shaderc::ShaderKind;
use std::fs;

pub fn compile_wgsl_to_spirv(_source: &str, shader_kind: ShaderKind, _entry_point: &str) -> Result<Vec<u32>> {
    // Read the pre-compiled SPIR-V file
    let spv_path = match shader_kind {
        ShaderKind::Vertex => "src/vr/shaders/vertex.spv",
        ShaderKind::Fragment => "src/vr/shaders/fragment.spv",
        _ => return Err(anyhow::anyhow!("Unsupported shader kind")),
    };
    
    let spv_data = fs::read(spv_path)?;
    
    // Convert bytes to u32 slice
    let words = spv_data.chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<_>>();
    
    Ok(words)
} 