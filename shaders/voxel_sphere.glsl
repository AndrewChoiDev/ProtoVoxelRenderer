#version 450
layout(local_size_x = 8, local_size_y = 8, local_size_z = 8) in;
layout(set = 0, binding = 0, r8ui) uniform writeonly uimage3D img;

float rand(vec2 co){
    return fract(sin(dot(co.xy ,vec2(12.9898,78.233))) * 43758.5453);
}

void main() {
    vec3 norm_coordinates = 2.0 * (gl_GlobalInvocationID.xyz) / vec3(imageSize(img)) - vec3(1);

    float norm_length = length(norm_coordinates);
    int i = 0;
    if (norm_length >= 0.9 - 0.2 * rand(vec2(norm_coordinates.x, length(norm_coordinates.yz))))
    {
        i = 255;
    }
    if (norm_length <= 0.1)
    {
        i = 255;
    }
    if (norm_coordinates.y < -0.25)
    {
        i = 255;
    }

    imageStore(img, ivec3(gl_GlobalInvocationID.xyz), ivec4(i));
}