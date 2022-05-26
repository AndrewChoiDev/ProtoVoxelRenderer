use super::super::world_engine::data_structures::SESVOctree;

pub trait VoxelPrefab
{
    fn tree_volume(&self)
        -> SESVOctree;
    fn palette_volume(&self)
        -> Vec<u8>;
    fn palette(&self)
        -> [u32 ; 256];
}


mod dot_vox_wrapper;
mod byte_voxels;
// pub mod voxel_barrel;
pub mod standard_voxel_prefab;