#version 460 core


layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(push_constant) uniform Orient
{
    vec3 hDir;
    vec3 vDir;
    vec3 fBasis;
    int frameIndex;
} orient;

layout (binding = 0, rgba16_snorm) uniform writeonly image2D rayDirections;

// Used for subpixel jitter
vec2 halton (int index)
{
    const vec2 coprimes = vec2(2.0, 3.0);
    vec2 s = vec2(index, index);
	vec4 a = vec4(1,1,0,0);
    while (s.x > 0. && s.y > 0.)
    {
        a.xy = a.xy/coprimes;
        a.zw += a.xy*mod(s, coprimes);
        s = floor(s/coprimes);
    }
    return a.zw;
}


void main()
{
    vec3 rayDirection = vec3(1.0);

    vec2 dims = gl_WorkGroupSize.xy * gl_NumWorkGroups.xy;

    vec2 subpixel_displacement = halton(orient.frameIndex + 1) - 0.5;
    subpixel_displacement = vec2(0.0);
    vec2 uv = ((2.0 * (gl_GlobalInvocationID.xy + subpixel_displacement)) - dims) / dims.y;


    ivec2 store_pos = ivec2(gl_GlobalInvocationID.x, dims.y - gl_GlobalInvocationID.y);

    imageStore(
        rayDirections, 
        ivec2(store_pos),
        vec4(normalize(uv.x * orient.hDir + uv.y * orient.vDir + orient.fBasis), 0.0)
    );
}