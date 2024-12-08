struct VRUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> vr_uniform: VRUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position
    out.clip_position = vr_uniform.view_proj * vec4<f32>(vertex.position, 1.0);
    
    // Transform normal and tangent to view space
    let normal = (vr_uniform.view * vec4<f32>(vertex.normal, 0.0)).xyz;
    let tangent = (vr_uniform.view * vec4<f32>(vertex.tangent.xyz, 0.0)).xyz;
    let bitangent = cross(normal, tangent) * vertex.tangent.w;
    
    out.normal = normalize(normal);
    out.tangent = normalize(tangent);
    out.bitangent = normalize(bitangent);
    
    // Pass through texture coordinates
    out.tex_coords = vertex.tex_coords;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation
    let light_dir = normalize(vec3<f32>(1.0, 1.0, -1.0));
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Base color with simple UV visualization
    let base_color = vec3<f32>(
        0.5 + 0.5 * sin(in.tex_coords.x * 10.0),
        0.5 + 0.5 * cos(in.tex_coords.y * 10.0),
        0.5
    );
    
    // Combine lighting and color
    let color = base_color * (0.2 + 0.8 * diffuse);
    
    return vec4<f32>(color, 1.0);
} 