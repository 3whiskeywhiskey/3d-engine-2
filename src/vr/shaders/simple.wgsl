// Simple vertex input with just position
struct VertexInput {
    @location(0) position: vec3<f32>,
}

// Simple vertex output with just clip position
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

// Camera uniform with view-projection matrices for both eyes
struct Camera {
    view_proj: array<mat4x4<f32>, 2>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    vertex: VertexInput,
    @builtin(view_index) view_id: u32
) -> VertexOutput {
    var out: VertexOutput;
    // Transform vertex position using the appropriate view-projection matrix
    out.clip_position = camera.view_proj[view_id] * vec4<f32>(vertex.position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Simple solid red color for testing
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
} 