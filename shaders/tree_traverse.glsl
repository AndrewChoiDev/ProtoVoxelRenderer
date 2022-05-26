#version 460 core

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;


layout(set = 0, binding = 0, r32ui) uniform readonly uimage1DArray treeArray;


layout(set = 1, binding = 0, rgba16_snorm) uniform readonly image2D rayDirections;
layout(set = 1, binding = 1, r11f_g11f_b10f) uniform writeonly image2D outputColor;
layout(set = 1, binding = 2, r32f) uniform image2D depth;


layout(set = 2, binding = 0) uniform usampler3D paletteVolumeAtlas;
layout(set = 2, binding = 1) uniform sampler1DArray rgbPalettes;

layout(push_constant) uniform PushConsts {
    vec3 pos;
    uint chunkCount;
} pushConsts;

const uint NULL_INDEX = 100000000; // hundred million


struct RayResult
{
    uint flag;
    uint nodeValue;
    vec3 end;
    ivec3 nodePos;
};

struct RayNodeQuery
{
    uint nodeAndOctant;
    ivec3 nodePos;
};


// These min/max funcs run as often as hell
float minComp(vec3 value)
{
    return min(min(value.x, value.y), value.z);
}
float maxComp(vec3 value)
{
    return max(max(value.x, value.y), value.z);
}

int imaxComp(vec3 value)
{
    int imaxProto = (value[0] > value[1])? 0 : 1;
    return (value[imaxProto] > value[2])? imaxProto : 2;
}
int iminComp(vec3 value)
{
    int iminProto = (value[0] < value[1])? 0 : 1;
    return (value[iminProto] < value[2])? iminProto : 2;
}

// This is like once per pixel
int[8] orientMask(ivec3 signDir)
{
    int orientMask[8];

    for (int i = 0; i < 8; i++)
    {
        ivec3 coords = ivec3(i % 2, (i % 4) / 2, i / 4);

        for (int j = 0; j < 3; j++)
        {
            if (signDir[j] == -1)
            {
                coords[j] = (coords[j] + 1) % 2;
            }
        }

        orientMask[i] = coords.x + coords.y * 2 + coords.z * 4;
    }

    return orientMask;
}

const uvec2 FIRST_CHILD_COMPONENTS_LOOKUP_TABLE[3] =
{
    ivec2(1, 2),
    ivec2(0, 2),
    ivec2(0, 1)
};

// This runs pretty often
uint getFirstChild(mat3 t)
{
    int imaxTlow = imaxComp(t[0]);

    uvec2 comps = FIRST_CHILD_COMPONENTS_LOOKUP_TABLE[imaxTlow];

    ivec2 checks = ivec2(
        greaterThan(
            vec2(t[0][imaxTlow]),
            vec2(t[1][comps[0]], t[1][comps[1]])
        )
    );

    return (checks[0] << comps[0]) | (checks[1] << comps[1]);
}

const ivec3 OCTANT_POS_LOOKUP_TABLE[8] =
{
    ivec3(0, 0, 0),
    ivec3(1, 0, 0),
    ivec3(0, 1, 0),
    ivec3(1, 1, 0),
    ivec3(0, 0, 1),
    ivec3(1, 0, 1),
    ivec3(0, 1, 1),
    ivec3(1, 1, 1)
};


// This function runs pretty freakin often...
mat3 tValuesForOctant(uint octant, mat3 t)
{
    mat3 tOctant;

    ivec3 tBounds = OCTANT_POS_LOOKUP_TABLE[octant];

    for (int i = 0; i < 2; i += 1)
    {
        for (int comp = 0; comp < 3; comp++)
        {
            tOctant[i << 1][comp] = t[tBounds[comp] + i][comp];
        }
    }
    
    tOctant[1] = mix(tOctant[0], tOctant[2], 0.5);

    return tOctant;
}

const uvec3[8] NEXT_CHILD_NODE_LOOKUP_TABLE =
{
    uvec3(1, 2, 4),
    uvec3(8, 3, 5),
    uvec3(3, 8, 6),
    uvec3(8, 8, 7),
    uvec3(5, 6, 8),
    uvec3(8, 7, 8),
    uvec3(7, 8, 8),
    uvec3(8, 8, 8)
};

mat4 tTransformConstruct(vec3 invDir, vec3 edgeStart)
{
    vec4 invDir4 = vec4(invDir, 1.0);
    vec4 start4 = vec4(-edgeStart, 1.0);

    mat4 tTransform = mat4(
        invDir4.x, 0, 0, 0,
        0, invDir4.y, 0, 0,
        0, 0, invDir4.z, 0,
        0, 0, 0, invDir4.w
    );
    tTransform[3] = invDir4 * start4;

    return tTransform;
}


mat3 tFromRayNode(ivec3 cornerStart, ivec3 cornerEnd, vec3 start, mat4 tTransform)
{
    mat4 corners = 
    {
        vec4(cornerStart, 1.0),
        vec4(mix(cornerStart, cornerEnd, 0.5), 1.0),
        vec4(cornerEnd, 1.0),
        vec4(0.0)
    };

    return mat3(tTransform * corners);
}


const int TREE_DEGREE = 9;

const int PREFAB_STOP_DEGREE = 4;


// An implementation of efficient parametric octree intersection
RayResult traverse(vec3 ro, vec3 dir)
{
    // A default result is prepared
    RayResult result = RayResult(0, NULL_INDEX, ro, ivec3(0));

    vec3 invDir = 1.0 / dir; // used for parameterization calculations
    ivec3 signDir = ivec3(sign(dir));

    ivec3 treePos = ivec3(0); // TODO: dynamic this one also

    // Calculates the corner positions of the octree corresponding to the sign direction
    ivec3 cornerStartDir = (-signDir + ivec3(1)) / int(2);
    ivec3 cornerEndDir = (signDir + ivec3(1)) / int(2);

    ivec3 cornerStart = treePos + cornerStartDir << TREE_DEGREE;
    ivec3 cornerEnd = treePos + cornerEndDir << TREE_DEGREE;


    // The point on the surface of the octree where the parameterization starts
    vec3 edgeStart = ro - minComp((ro - cornerStart) * invDir) * dir;

    mat4 tTransform = tTransformConstruct(invDir, edgeStart);

    // mat3 t values are ordered as such:
    // t[0] => the lower bound
    // t[1] => the average/middle bound
    // t[2] => the higher bound
    mat3 t = tFromRayNode(cornerStart, cornerEnd, edgeStart, tTransform);

    // The parameterization value corresponding to the ray origin
    float tRO = (ro.x - edgeStart.x) * invDir.x;

    // A mask that maps octants to other octants
    // based on ray orientation
    int orientMask[8] = orientMask(signDir);

    RayNodeQuery stack[8];
    stack[0] = RayNodeQuery(bitfieldInsert(0, getFirstChild(t), 28, 4), ivec3(0));

    uint depth = 0;
    uint layer = 4;

    for (int iter = 0; iter < 500; iter++)
    {
        // Extracted values from packed nodeAndOctant variable
        uint cdIndex = bitfieldExtract(stack[depth].nodeAndOctant, 0, 28);

        // uint cd = cds[cdIndex];
        uint cd = imageLoad(treeArray, ivec2(cdIndex, layer)).x;
        // First two bytes used for first child index
        uint firstChildIndex = bitfieldExtract(cd, 0, 16) << 3;
        // Next byte is used for valid mask
        uint validMask = bitfieldExtract(cd, 16, 8);

        uint octant = bitfieldExtract(stack[depth].nodeAndOctant, 28, 4);

        while (octant != 8)
        {
            uint orientedOctant = orientMask[octant];

            mat3 tChild = tValuesForOctant(octant, t);

            // The stack is updated to use the next intersected octant for a future iteration
            stack[depth].nodeAndOctant = 
                bitfieldInsert(
                    stack[depth].nodeAndOctant, 
                    NEXT_CHILD_NODE_LOOKUP_TABLE[octant][iminComp(tChild[2])], 
                    28, 4
                );
            
            if (((validMask >> orientedOctant) & 1) == 0
                || minComp(tChild[2]) <= tRO)
            {
                octant = bitfieldExtract(stack[depth].nodeAndOctant, 28, 4);
                continue;
            }

            uint nextCDIndex = firstChildIndex | orientedOctant;
            if (depth == PREFAB_STOP_DEGREE - 1)
            {
                layer = 4 + pushConsts.chunkCount;
                nextCDIndex = 0;
            }

            ivec3 childNodePos = 
                stack[depth].nodePos 
                + (OCTANT_POS_LOOKUP_TABLE[orientedOctant] << (TREE_DEGREE - depth - 1));

            if (depth == TREE_DEGREE - 1)
            {
                result.end = edgeStart + (maxComp(tChild[0]) * dir);
                result.nodeValue = 1;
                result.flag = 1;
                result.nodePos = childNodePos;
                return result;
            }


            depth++;
            stack[depth].nodeAndOctant =
                bitfieldInsert(
                    nextCDIndex, 
                    getFirstChild(tChild), 
                    28, 4
                );

            stack[depth].nodePos = childNodePos;

            t = tChild;

            break;
        }


        if (octant == 8)
        {
            if (depth == 0)
            {
                return result;
            }

            if (depth == PREFAB_STOP_DEGREE)
            {
                layer = 4;
            }

            depth--;
            cornerStart = stack[depth].nodePos + (cornerStartDir << (TREE_DEGREE - depth));
            cornerEnd = stack[depth].nodePos + (cornerEndDir << (TREE_DEGREE - depth));
            t = tFromRayNode(cornerStart, cornerEnd, edgeStart, tTransform);
        }
    }

    return result;
}


void main()
{
    ivec2 pixelPos = ivec2(gl_GlobalInvocationID.xy);
    float rastLength = imageLoad(depth, pixelPos).r;

    vec3 rayDir = imageLoad(rayDirections, pixelPos).xyz;

    vec4 outColor = vec4(rayDir, 1.0);

    vec3 offset = vec3(16.0, 16.0, 16.0) * 0.0;


    vec3 ro = (pushConsts.pos * (1 << (TREE_DEGREE - PREFAB_STOP_DEGREE)) + offset);
    RayResult result = traverse(ro, rayDir);


    if (result.flag == 1)
    {
        float rayLength = length(result.end - ro) / 32.0;

        outColor = vec4(1.0 - exp(-rayLength / 8.0));

        vec3 prefabColor =
            texelFetch(rgbPalettes, 
                ivec2(
                    texelFetch(paletteVolumeAtlas, result.nodePos % 32, 0).r, 
                    0
                ), 0
            ).rgb;

        outColor = vec4(pow(prefabColor, vec3(2.2)), 1.0);

        // outColor.b *= result.nodeValue * 0.1;
        //outColor = vec4(vec3(result.nodePos % 32) / 32.0, 1.0);
    }

    // ivec3 signDir = ivec3(sign(rayDir));
    // outColor = vec4(vec3((-signDir + ivec3(1)) / int(2)), 1.0);

    imageStore(outputColor, pixelPos, outColor);
}