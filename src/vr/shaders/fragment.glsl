#version 450
#extension GL_EXT_multiview : require

layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;
layout(location = 2) in vec2 frag_uv;
layout(location = 3) in vec3 view_pos;

layout(set = 1, binding = 0) uniform LightUniform {
    vec3 position;
    vec3 color;
    float _padding;
} light;

layout(location = 0) out vec4 frag_color;

void main() {
    vec3 ambient = vec3(0.05, 0.05, 0.05);
    vec3 light_dir = normalize(light.position - world_position);
    vec3 view_dir = normalize(view_pos - world_position);
    vec3 half_dir = normalize(view_dir + light_dir);

    float diffuse = max(dot(world_normal, light_dir), 0.0);
    float specular = pow(max(dot(world_normal, half_dir), 0.0), 32.0);
    vec3 color = ambient + light.color * (diffuse + specular);

    frag_color = vec4(color, 1.0);
} 