#version 460 core

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(binding = 0, r11f_g11f_b10f) uniform image2D outputColor;
layout(binding = 1, r32f) uniform image2D outputDepth;

void main()
{
    ivec2 pixel_pos = ivec2(gl_GlobalInvocationID.xy);
    imageStore(outputColor, pixel_pos, vec4(0.5, 0.1, 0.6, 1.0));
    imageStore(outputDepth, pixel_pos, vec4(20.0));
}