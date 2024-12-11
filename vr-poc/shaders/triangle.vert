#version 450
#extension GL_EXT_multiview : require

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 fragColor;

// Per-view transformations
layout(set = 0, binding = 0) uniform ViewData {
    mat4 viewMatrix[2];
    mat4 projectionMatrix[2];
} viewData;

void main() {
    // Use gl_ViewIndex to access the correct view matrix
    mat4 viewMatrix = viewData.viewMatrix[gl_ViewIndex];
    mat4 projectionMatrix = viewData.projectionMatrix[gl_ViewIndex];
    
    gl_Position = projectionMatrix * viewMatrix * vec4(position, 1.0);
    fragColor = color;
} 