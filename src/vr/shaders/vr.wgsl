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

@group(1) @binding(0)
var<uniform> light: LightUniform;

@group(2) @binding(0)
var<uniform> model: ModelUniform;

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;
@group(3) @binding(2)
var t_normal: texture_2d<f32>;
@group(3) @binding(3)
var s_normal: sampler;

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
    @location(2) world_pos: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @builtin(view_index) view_index: u32,
};

@vertex
fn vs_main(
    model_in: VertexInput,
    @builtin(view_index) view_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model_matrix * vec4<f32>(model_in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.tex_coords = model_in.tex_coords;
    
    // Transform normal and tangent to world space
    let normal = normalize((model.model_matrix * vec4<f32>(model_in.normal, 0.0)).xyz);
    let tangent = normalize((model.model_matrix * vec4<f32>(model_in.tangent.xyz, 0.0)).xyz);
    let bitangent = cross(normal, tangent) * model_in.tangent.w;
    
    out.normal = normal;
    out.tangent = tangent;
    out.bitangent = bitangent;
    out.world_pos = world_pos.xyz;
    out.view_index = view_index;
    return out;
}

fn calculate_normal(in: VertexOutput) -> vec3<f32> {
    // Sample normal map and transform from [0,1] to [-1,1] range
    let normal_sample = textureSample(t_normal, s_normal, in.tex_coords);
    let normal_map = normal_sample.xyz * 2.0 - 1.0;
    
    // Construct TBN matrix for transforming from tangent to world space
    let N = normalize(in.normal);
    let T = normalize(in.tangent);
    let B = normalize(in.bitangent);
    let TBN = mat3x3<f32>(T, B, N);
    
    // Transform normal from tangent space to world space
    return normalize(TBN * normal_map);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = calculate_normal(in);
    let light_dir = normalize(light.direction.xyz);
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(view_dir - light_dir);

    // Sample texture
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // Ambient
    let ambient = light.ambient.rgb * tex_color.rgb;

    // Diffuse
    let diff = max(dot(normal, -light_dir), 0.0);
    let diffuse = light.color.rgb * diff * tex_color.rgb;

    // Specular
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = light.color.rgb * spec * 0.5;

    // Ambient occlusion (simple)
    let ao = max(dot(normal, vec3<f32>(0.0, 1.0, 0.0)), 0.0) * 0.2 + 0.8;

    let final_color = (ambient + diffuse + specular) * ao;
    return vec4<f32>(final_color, tex_color.a);
} 