// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct LightUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.normal = normalize(model.normal);
    out.world_pos = model.position;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let light_dir = normalize(light.direction.xyz);
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(view_dir - light_dir);

    // Ambient
    let ambient = light.ambient.rgb;

    // Diffuse
    let diff = max(dot(normal, -light_dir), 0.0);
    let diffuse = light.color.rgb * diff;

    // Specular
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = light.color.rgb * spec * 0.5;

    // Ambient occlusion (simple)
    let ao = max(dot(normal, vec3<f32>(0.0, 1.0, 0.0)), 0.0) * 0.2 + 0.8;

    let final_color = (ambient + diffuse + specular) * ao;
    return vec4<f32>(final_color, 1.0);
} 