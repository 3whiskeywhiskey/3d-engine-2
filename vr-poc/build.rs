use std::process::Command;
use std::path::Path;

fn main() {
    let shader_dir = Path::new("vr-poc/shaders");

    // Compile vertex shader
    println!("cargo:rerun-if-changed=vr-poc/shaders/triangle.vert");
    let status = Command::new("glslc")
        .args(&[
            shader_dir.join("triangle.vert").to_str().unwrap(),
            "-o",
            shader_dir.join("triangle.vert.spv").to_str().unwrap()
        ])
        .status()
        .expect("Failed to execute glslc");
    
    if !status.success() {
        panic!("Failed to compile vertex shader");
    }

    // Compile fragment shader
    println!("cargo:rerun-if-changed=vr-poc/shaders/triangle.frag");
    let status = Command::new("glslc")
        .args(&[
            shader_dir.join("triangle.frag").to_str().unwrap(),
            "-o",
            shader_dir.join("triangle.frag.spv").to_str().unwrap()
        ])
        .status()
        .expect("Failed to execute glslc");
    
    if !status.success() {
        panic!("Failed to compile fragment shader");
    }
} 