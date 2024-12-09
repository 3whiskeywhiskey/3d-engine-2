struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.normal = normalize(model.normal);
    out.tangent = normalize(model.tangent.xyz);
    out.bitangent = normalize(cross(out.normal, out.tangent) * model.tangent.w);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let normal_map = textureSample(t_normal, s_normal, in.tex_coords);
    
    // Convert normal from [0,1] to [-1,1] range
    let normal = normalize(normal_map.xyz * 2.0 - 1.0);
    
    // Basic lighting calculation
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let diffuse_strength = max(dot(normal, light_dir), 0.1);
    
    return vec4<f32>(diffuse.rgb * diffuse_strength, diffuse.a);
} 