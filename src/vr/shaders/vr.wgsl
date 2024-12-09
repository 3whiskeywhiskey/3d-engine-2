struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
}

struct LightUniform {
    position: vec3<f32>,
    color: vec3<f32>,
    _padding: f32,
}

struct ModelUniform {
    model_matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> light: LightUniform;
@group(2) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) view_pos: vec3<f32>,
}

@vertex
fn vs_main(
    model_vertex: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_position = model.model_matrix * vec4<f32>(model_vertex.position, 1.0);
    out.world_position = world_position.xyz;
    out.world_normal = normalize(model_vertex.normal);
    out.clip_position = camera.view_proj * world_position;
    out.uv = model_vertex.uv;
    out.view_pos = camera.view_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient = vec3<f32>(0.05, 0.05, 0.05);
    let light_dir = normalize(light.position - in.world_position);
    let view_dir = normalize(in.view_pos - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    let specular = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);
    let color = ambient + light.color * (diffuse + specular);

    return vec4<f32>(color, 1.0);
} 