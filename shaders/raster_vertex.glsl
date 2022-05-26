#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform SpaceMatrices {
    mat4 pvm;
} space;

// layout(binding = 0) uniform SpaceMatrices
// {
//     mat4 pvm;
// } space;

void main() {
    // gl_Position = (space.projection * (space.view * space.model)) * vec4(position, 1.0);
    gl_Position = space.pvm * vec4(position, 1.0);
    fragColor = color;


}