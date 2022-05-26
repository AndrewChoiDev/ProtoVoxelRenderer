use std::sync::{Arc};

use super::super::world_engine as world_eng;
use world_eng::object as world_objs;

// use super::data_structures::SESVOctree;

use world_objs::VoxelPrefab;
use world_objs::standard_voxel_prefab::StandardVoxelPrefab;
// use nalgebra as na;

pub struct PrefabManager
{
    pub prefabs : Vec<Arc<dyn VoxelPrefab + Send + Sync>>,
}

impl PrefabManager
{
    pub fn new()
        -> PrefabManager
    {
        let prefabs : Vec<Arc<dyn VoxelPrefab + Send + Sync>> =
            vec!(
                Arc::new(StandardVoxelPrefab::new("resources/magica voxels/32 set/bricks.vox")),
                Arc::new(StandardVoxelPrefab::new("resources/magica voxels/32 set/plank_tile.vox")),
                Arc::new(StandardVoxelPrefab::new("resources/magica voxels/32 set/stone_stairs.vox")),
                Arc::new(StandardVoxelPrefab::new("resources/magica voxels/32 set/grass.vox")),
                Arc::new(StandardVoxelPrefab::new("resources/magica voxels/32 set/ridged_stone.vox")),
            );
            
        PrefabManager {prefabs}
    }
}

