#version 460 core

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0) uniform usampler3D vox_map;
layout(set = 0, binding = 1) uniform usampler3D mapGrid;
// layout(set = 0, binding = 1, r32ui) uniform readonly uimage3D mapGrid;


layout(set = 1, binding = 0, rgba16_snorm) uniform readonly image2D rayDirections;
layout(set = 1, binding = 1, r11f_g11f_b10f) uniform writeonly image2D outputColor;
layout(set = 1, binding = 2, r32f) uniform image2D depth;


const uint volumePrefabCount = 6;

layout(set = 2, binding = 0) uniform usampler3D bitVolumeAtlas;
layout(set = 2, binding = 1) uniform usampler3D paletteVolumeAtlas;
layout(set = 2, binding = 2) uniform sampler1DArray rgbPalettes;


struct VoxelPrefab
{
    uint volumeId;
    uint paletteVolumeId;
    uint paletteId;
};

layout(set = 2, binding = 3) readonly buffer
VoxPrefabMetas
{
    VoxelPrefab voxelPrefabs[volumePrefabCount];
};


// Highly dynamic data...
layout(push_constant) uniform PushConsts {
    vec3 pos;
    ivec3 gridPos;
} pushConsts;

struct RayResult
{
    float rayLength;
    vec3 norm;

    ivec3 mapCoords;
    ivec3 prefabCoords;
    ivec3 chunkCoords;
};


uint getVoxel(ivec3 c, uint chunkId)
{
    return texelFetch(vox_map, ivec3(c.x, c.y, c.z + (chunkId - 1) * 16), 0).r;   
}

bool checkPrefabVoxel(ivec3 c, uint volumeId)
{
    ivec3 prefab_coords = (c / 2);
    prefab_coords.z += int(volumeId - 1) * 12;

    uint octo_voxel = 
        texelFetch(
            bitVolumeAtlas, 
            prefab_coords, 0).r;

    ivec3 mod_coords = c % 2;

    int nth_bit = mod_coords.x + mod_coords.y * 2 + mod_coords.z * 4;

    return bitfieldExtract(octo_voxel, nth_bit, 1) != 0;
}

vec3 prefabColor(ivec3 coords, uint voxelId)
{
    VoxelPrefab prefab = voxelPrefabs[voxelId - 1];

    coords.z += int(prefab.paletteVolumeId) * 24;

    return 
        texelFetch(
            rgbPalettes, 
            ivec2(
                texelFetch(
                    paletteVolumeAtlas, 
                    coords, 0).r, 
                prefab.paletteId
            ), 0
        ).rgb;
}


bool TraceVoxelPrefab(
    uint voxelId, vec3 rayDir, vec3 rayPos, 
    inout RayResult rayResult)
{
    if (voxelId == 0) // empty voxel
    {
        return false;
    }

    uint volumeId = voxelPrefabs[voxelId - 1].volumeId;

    if (volumeId == 0) // filled voxel
    {
        return true;
    }

    uint dims = 24;

    vec3 modelRayPos = (mod(rayPos, 1.0)) * dims;

    ivec3 mapPos = ivec3(floor(modelRayPos));

    if (checkPrefabVoxel(mapPos, volumeId))
    {
        rayResult.prefabCoords = mapPos;
        return true;
    }

    vec3 deltaDist = abs(1.0 / rayDir);

    vec3 rayStep = vec3(sign(rayDir));

    vec3 t_max =
        (
            sign(rayDir) * (vec3(mapPos) - modelRayPos) 
            + (sign(rayDir) * 0.5) 
            + vec3(0.5)
        ) * deltaDist;
    
    bool hit = false;

    bvec3 mask;

    for (int i = 0; i < 500; i++)
    {
        mask = lessThanEqual(t_max.xyz, min(t_max.yzx, t_max.zxy));
		mapPos += ivec3(vec3(mask) * rayStep);

        if (mapPos.x < 0 || mapPos.x > dims - 1
            || mapPos.y < 0 || mapPos.y > dims - 1
            || mapPos.z < 0 || mapPos.z > dims - 1)
        {
            break;
        }
        if (checkPrefabVoxel(mapPos, volumeId))
        {
            hit = true;
            break;
        }

		t_max += vec3(mask) * deltaDist;
    }

    rayResult.prefabCoords = mapPos;
    rayResult.norm = vec3(mask) * -sign(rayStep);
    rayResult.rayLength += min(min(t_max.x, t_max.y), t_max.z) / float(dims);
    return hit;
}

uint chunkIdFromCoords(ivec3 chunkCoords)
{
    ivec3 gridSize = textureSize(mapGrid, 0);

    return texelFetch(mapGrid, chunkCoords + (gridSize / 2), 0).r;
}


bool TraceVoxels(
    vec3 rayDir, vec3 rayOrigin,
    ivec3 chunkCoords,
    out RayResult rayResult)
{
    // default initializations for out variables
    rayResult = RayResult(0.0, -rayDir, ivec3(0.0), ivec3(0.0), chunkCoords);

    { // Correct ray origin and chunk coords
        const float chunkLength = 16.0;
        vec3 chunkRayOrigin = rayOrigin / chunkLength;
        chunkCoords += ivec3(floor(chunkRayOrigin));
        rayOrigin = (chunkRayOrigin - floor(chunkRayOrigin)) * 16.0;
    }

    // Woo traversal preperation stage
    ivec3 mapPos = ivec3(floor(rayOrigin));

    
    vec3 deltaDist = abs(1.0 / rayDir);
    vec3 rayStep = vec3(sign(rayDir));
    ivec3 iRayStep = ivec3(rayStep);
    vec3 t_max =
        (
            sign(rayDir) * (vec3(mapPos) - rayOrigin) 
            + (sign(rayDir) * 0.5) 
            + vec3(0.5)
        ) * deltaDist;

    uint chunkId = chunkIdFromCoords(chunkCoords);
    if (chunkId == 0)
    {
        return false;
    }
    uint voxelId = getVoxel(mapPos, chunkId);

    // A trace through the ray origin's voxel
    if (TraceVoxelPrefab(
            voxelId, rayDir, 
            rayOrigin, 
            rayResult)) 
    {
        return true;
    }


    bvec3 mask;

    int mapLimit = 16 - 1;

    // woo traversal loop
    for (int i = 0; i < 500; i++)
    {
        mask = lessThanEqual(t_max.xyz, min(t_max.yzx, t_max.zxy));
		mapPos += ivec3(mask) * iRayStep;

        if (any(lessThan(mapPos, ivec3(0)))
            || any(greaterThan(mapPos, ivec3(mapLimit))))
        {
            ivec3 iMask = ivec3(mask);
            chunkCoords += iMask * iRayStep;
            chunkId = chunkIdFromCoords(chunkCoords);
            if (chunkId == 0)
            {
                break;
            }

            rayResult.chunkCoords = chunkCoords;
            // t_max -= vec3(mask) * rayStep * 0.0001;

            mapPos = (mapPos + ivec3(16)) % ivec3(16);
        }
        
        // Position's voxel is tested
        voxelId = getVoxel(mapPos, chunkId);
        if (voxelId >= 1)
        {
            rayResult.rayLength = min(min(t_max.x, t_max.y), t_max.z);
            rayResult.norm = vec3(mask) * -rayStep;
        }
        if (TraceVoxelPrefab(
                voxelId, rayDir, 
                (rayResult.rayLength + 0.0001) * rayDir + rayOrigin, 
                rayResult)) 
        {
            rayResult.mapCoords = ivec3(floor(mapPos));
            return true;
        }

		t_max += vec3(mask) * deltaDist;
    }

    rayResult.rayLength = min(min(t_max.x, t_max.y), t_max.z);

    return false;
}

vec3 GetRayDirection()
{
    return imageLoad(rayDirections, ivec2(gl_GlobalInvocationID.xy)).xyz;
}

float CheckShadow(vec3 lightDir, float lightDist, vec3 rayStart, vec3 surfaceNorm, ivec3 chunkCoords)
{
    vec3 offsetRayStart = rayStart + surfaceNorm * 0.0001;

    RayResult rayResult;
    if (!TraceVoxels(lightDir, offsetRayStart, chunkCoords, rayResult))
    {
        return 1.0;
    }

    if (rayResult.rayLength < lightDist)
    {
        return 0.0;
    }
    return 1.0;

}

vec3 GetDirectionalLightValue(vec3 lightColor, vec3 lightDir, vec3 chunkHitPoint, RayResult rayResult)
{
    float shadow = CheckShadow(lightDir, 1000000.0, chunkHitPoint, rayResult.norm, rayResult.chunkCoords);

    return lightColor * shadow * dot(rayResult.norm, lightDir);
}


void main()
{
    ivec2 pixel_pos = ivec2(gl_GlobalInvocationID.xy);
    float rast_length = imageLoad(depth, pixel_pos).r;

    const float f = 200.0;
    const float n = 0.001;

    vec3 rayDir = GetRayDirection();

    RayResult rayResult;
    bool hit = TraceVoxels(rayDir, pushConsts.pos, ivec3(0, 0, 0), rayResult);


    float rayLength = rayResult.rayLength;
    vec3 norm = rayResult.norm;


    if (rayLength > rast_length)
    {
        imageStore(outputColor, pixel_pos, vec4(1.0));
        return;
    }

    imageStore(depth, pixel_pos, vec4(rayLength));

    if (!hit)
    {
        vec4 color = vec4(0.5, 0.5, 0.95, 1.0);
        // color = vec4(1.0 - exp(-rayLength / 8.0));        
        imageStore(outputColor, pixel_pos, color);
        return;
    }



    // vec3 hitPoint = pushConsts.pos + rayLength * rayDir;
    
    vec3 lightPos = vec3(-4.0, -3.0, 5.0);

    vec3 chunkHitPoint = pushConsts.pos + rayLength * rayDir - vec3((rayResult.chunkCoords) * 16);

    // vec3 lightOne = GetLightValue(vec3(1.0, 10.0, 200.0), lightPos, hitPoint, norm);
    vec3 lightTwo = GetDirectionalLightValue(vec3(40.0, 15.0, 15.0), normalize(vec3(0.3, 1.0, 0.2)), chunkHitPoint, rayResult);
    // vec3 lightThree = GetLightValue(vec3(50.0, 40.0, 30.0), pushConsts.pos + vec3(0.0, 0.0, 2.0), hitPoint, norm);

    vec3 diffuse = lightTwo + vec3(0.015, 0.015, 0.02) * 6.0;
    // diffuse = vec3(1.0);

    uint voxelId = getVoxel(rayResult.mapCoords, chunkIdFromCoords(rayResult.chunkCoords));
    vec3 albedo = vec3(1.0);

    if (voxelId >= 1)
    {
        uint dims = 24;
        albedo = pow(prefabColor(rayResult.prefabCoords, voxelId), vec3(2.2));
    }

    vec4 color = vec4(albedo * diffuse, 1.0);
    // color = vec4(1.0 - exp(-rayLength / 8.0));
    // color = vec4(1.0 - exp(-length(chunkHitPoint) / 8.0));
    // color = vec4(1.0 - length(vec3(rayResult.chunkCoords) / 4.0));
    imageStore(outputColor, pixel_pos, color);
    // f_color = ;
    // f_color = vec4(norm / 4.0 + abs(norm) * 0.5, 1.0);
}