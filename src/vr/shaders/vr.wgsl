struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct LightUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
};

struct ModelUniform {
    model_matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> light: LightUniform;

@group(0) @binding(2)
var<uniform> model: ModelUniform;

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

@vertex
fn vs_main(
    model_in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    let world_pos = model.model_matrix * vec4<f32>(model_in.position, 1.0);
    out.world_position = world_pos.xyz;
    
    // Transform position to clip space using the appropriate view-projection matrix
    out.clip_position = camera.view_proj * world_pos;
    
    // Transform normal to world space (assuming uniform scaling)
    out.world_normal = (model.model_matrix * vec4<f32>(model_in.normal, 0.0)).xyz;
    
    out.uv = model_in.uv;
    
    return out;
}

@fragment
fn fs_main(
    @builtin(view_index) view_index: u32,
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    // Normalize vectors
    let normal = normalize(in.world_normal);
    let view_dir = normalize(camera.camera_pos.xyz - in.world_position);
    
    // Basic lighting setup
    let light_dir = normalize(light.direction.xyz);
    let ambient = light.ambient.rgb;
    let diffuse = max(dot(normal, -light_dir), 0.0);
    
    // Simple specular calculation
    let half_dir = normalize(view_dir - light_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);
    
    // Base color from UV coordinates for testing
    let base_color = vec3<f32>(in.uv.x, in.uv.y, 1.0);
    
    // Combine lighting
    let color = base_color * (ambient + diffuse * light.color.rgb * 0.7) + light.color.rgb * specular * 0.3;
    
    return vec4<f32>(color, 1.0);
} 