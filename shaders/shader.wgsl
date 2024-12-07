// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.normal = model.normal;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // For now, just use a simple color based on the normal
    let light_dir = normalize(vec3<f32>(-1.0, -1.0, -1.0));
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, -light_dir), 0.0);
    let color = vec3<f32>(0.7, 0.7, 0.7) * diffuse + vec3<f32>(0.1, 0.1, 0.1);
    return vec4<f32>(color, 1.0);
} 