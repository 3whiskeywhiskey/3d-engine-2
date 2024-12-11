#version 450
#extension GL_EXT_multiview : require

// Uniform buffer bindings
layout(set = 0, binding = 0) uniform CameraUniform {
    mat4 view[2];
    mat4 proj[2];
    mat4 view_proj[2];
    vec4 view_pos[2];
} camera;

layout(set = 1, binding = 0) uniform LightUniform {
    vec3 position;
    vec3 color;
    float _padding;
} light;

layout(set = 2, binding = 0) uniform ModelUniform {
    mat4 model_matrix;
} model;

// Vertex inputs
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

// Vertex outputs
layout(location = 0) out vec3 world_position;
layout(location = 1) out vec3 world_normal;
layout(location = 2) out vec2 frag_uv;
layout(location = 3) out vec3 view_pos;

void main() {
    vec4 world_pos = model.model_matrix * vec4(position, 1.0);
    world_position = world_pos.xyz;
    world_normal = normalize(normal);
    gl_Position = camera.view_proj[gl_ViewIndex] * world_pos;
    frag_uv = uv;
    view_pos = camera.view_pos[gl_ViewIndex].xyz;
} 