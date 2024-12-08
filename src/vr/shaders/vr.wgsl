struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VRUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_position: vec3<f32>,
    _padding: u32,
};

@group(0) @binding(0)
var<uniform> vr: VRUniform;

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(view_index) view_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    let world_pos = vec4<f32>(model.position, 1.0);
    out.world_position = world_pos.xyz;
    
    // Transform position to clip space using the appropriate view-projection matrix
    out.clip_position = vr.view_proj * world_pos;
    
    // Transform normal to world space (assuming uniform scaling)
    out.world_normal = (vr.view * vec4<f32>(model.normal, 0.0)).xyz;
    
    out.uv = model.uv;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize vectors
    let normal = normalize(in.world_normal);
    let view_dir = normalize(vr.eye_position - in.world_position);
    
    // Basic lighting setup
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let ambient = 0.1;
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Simple specular calculation
    let reflect_dir = reflect(-light_dir, normal);
    let specular = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    
    // Base color from UV coordinates for testing
    let base_color = vec3<f32>(in.uv.x, in.uv.y, 1.0);
    
    // Combine lighting
    let color = base_color * (ambient + diffuse * 0.7) + vec3<f32>(1.0) * specular * 0.3;
    
    return vec4<f32>(color, 1.0);
} 