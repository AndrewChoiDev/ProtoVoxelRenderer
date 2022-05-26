
#version 450

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

// layout (set = 0, binding = 0, rgba32f) uniform image2D inputColor;
// layout (input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput inputColor;
layout (set = 0, binding = 0, r11f_g11f_b10f) uniform readonly image2D inputColor;
layout(set = 0, binding = 1, r32f) uniform readonly image2D depth;

layout (set = 0, binding = 2, rgba16) uniform writeonly image2D outputColor; 



float A = 0.15;
float B = 0.50;
float C = 0.10;
float D = 0.20;
float E = 0.02;
float F = 0.30;
float W = 11.2;

vec3 Uncharted2Tonemap(vec3 x)
{
   return ((x*(A*x+C*B)+D*E)/(x*(A*x+B)+D*F))-E/F;
}



void main()
{
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);

    vec3 color = imageLoad(inputColor, pos).rgb;

    vec3 curr = Uncharted2Tonemap(2.0 * color);

    vec3 whiteScale = 1.0 / Uncharted2Tonemap(W * vec3(1.0));

    color = curr * whiteScale;

    color = pow(color, vec3(1.0 / 2.2));

    imageStore(outputColor, pos, vec4(color, 1.0));
}