#version 460 core

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;


layout(binding = 0) uniform sampler2D rasterDepth;
layout(binding = 1, r32f) uniform writeonly image2D outputDepth;



layout(push_constant) uniform PushConsts {
    vec3 pos;
    // ivec3 gridPos;
    mat4 projInv;
    mat4 viewInv;
} pushConsts;


void main()
{
    ivec2 pixelPos = ivec2(gl_GlobalInvocationID.xy);
    float depth = texelFetch(rasterDepth, pixelPos, 0).r;

    vec2 dims = gl_WorkGroupSize.xy * gl_NumWorkGroups.xy;
    vec2 uv = (2.0 * gl_GlobalInvocationID.xy) / dims - vec2(1.0);

    float z = depth * 2.0 - 1.0;
    vec4 clipSpacePosition = vec4(uv, depth, 1.0);
    vec4 viewSpacePosition = pushConsts.projInv * clipSpacePosition;
    viewSpacePosition /= viewSpacePosition.w;

    vec4 worldSpacePosition = pushConsts.viewInv * viewSpacePosition;

    vec3 rastPos = worldSpacePosition.xyz;

    float rastLength = length(rastPos - pushConsts.pos);

    imageStore(outputDepth, pixelPos, vec4(rastLength));
}