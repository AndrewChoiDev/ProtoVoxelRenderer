pub mod object;

pub mod data_structures;

pub mod voxel_manager;

pub mod chunk_generators;

pub mod map;

pub mod displaced_chunks;


pub trait ChunkGenerator
{
    fn generate_chunk(&self,
        block_ids : &mut[u8], 
        world_grid_position : [i32 ; 3], 
        chunk_dims : [usize ; 3]);
}