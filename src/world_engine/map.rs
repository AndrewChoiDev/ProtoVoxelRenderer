use super::data_structures::SESVOctree;

use nalgebra as na;

use super::super::world_engine as world_eng;

use world_eng::ChunkGenerator;
use world_eng::voxel_manager::PrefabManager;
use world_eng::chunk_generators::{TerrainChunkGenerator};

use std::collections::HashSet;

use world_eng::displaced_chunks::DisplacedChunks;

use super::super::input as input;

use input::KeyEventQueue;


// A map organizes and manages a set of chunks
pub struct Map
{
    chunks : DisplacedChunks,

    chunk_generator : Box<dyn ChunkGenerator>,

    world_grid_pos : na::Point3<i32>,

    pub prefab_manager : PrefabManager,

    input_event_queue : KeyEventQueue,
}

impl Map
{
    pub fn new(viewer_world_grid_pos : na::Point3<i32>, view_radius : usize)
        -> Map
    {
        let prefab_manager = PrefabManager::new();

        Map
        {
            chunks : DisplacedChunks::new(Map::radius_displacement_set(view_radius)),
            chunk_generator : Box::new(TerrainChunkGenerator::new()),
            world_grid_pos : viewer_world_grid_pos,
            prefab_manager,
            input_event_queue : KeyEventQueue::new(set!("interact_1", "interact_2"))
        }
    }

    fn radius_displacement_set(view_radius : usize)
        -> HashSet<na::Vector3<i32>>
    {
        let mut displacement_set = HashSet::new();

        let r = view_radius as i32;
        let axis_iter = (1 - r)..r;

        let view_radius = (view_radius - 1) as f32;

        for x in axis_iter.clone() {
        for y in axis_iter.clone() {
        for z in axis_iter.clone() {

            let displacement = na::Vector3::new(x, y, z);

            if Map::displacement_in_length(displacement, view_radius)
            {
                displacement_set.insert(displacement);
            }

        }}}

        displacement_set
    }

    fn displacement_in_length(displacement : na::Vector3<i32>, length : f32)
        -> bool
    {
        // The magnitude of the view coordinate is compared with the radius
        displacement.iter()
        .fold(0f32, |acc, c| acc + (c * c) as f32)
        .sqrt() 
            <= length
    }

    pub fn input_queue(&mut self)
        -> &mut KeyEventQueue
    {
        &mut self.input_event_queue
    }

    pub fn chunk_dims(&self)
        -> [usize ; 3]
    {
        [1 << world_eng::displaced_chunks::CHUNK_EXPONENT ; 3]
    }

    pub fn chunk_len(&self)
        -> usize
    {
        let dims = self.chunk_dims();

        dims[0] * dims[1] * dims[2]
    }

    pub fn chunk_count(&self)
        -> usize
    {
        self.chunks.len()
    }

    pub fn handle_events(&mut self, _look_dir : na::Vector3<f32>, _world_grid_coord : na::Point3<i32>, _chunk_pos : na::Point3<f32>)
    {

    }


    // Temporary method
    pub fn render_tree(&self)
        -> Option<&SESVOctree>
    {
        for c_index in 0..self.chunk_len() 
        {
            if self.chunks.get_displacement(c_index) == na::Vector3::repeat(0)
            {
                return self.chunks.get_tree(c_index);
            }
        }

        None
    }

    pub fn adapt_to_world_position(&mut self, viewer_world_grid_pos : na::Point3<i32>)
    {
        let viewer_displacement = viewer_world_grid_pos - self.world_grid_pos;

        if viewer_displacement == na::zero::<na::Vector3<i32>>()
        {
            return;
        }

        // chunks must be displaced in the opposite direction
        // to have correct relative positioning
        self.chunks.displace(-viewer_displacement);
        self.world_grid_pos += viewer_displacement;
    }

    pub fn generate_next_chunk(&mut self)
    {
        if let Some(c_index) = self.chunks.use_chunk()
        {
            let chunk_world_grid_pos = self.world_grid_pos + self.chunks.get_displacement(c_index);

            let mut block_buffer = vec![std::u8::MAX ; self.chunk_len()];

            self.chunk_generator.generate_chunk(&mut block_buffer, chunk_world_grid_pos.coords.into(), self.chunk_dims());

            let c_width = 1 << world_eng::displaced_chunks::CHUNK_EXPONENT;
            let c_area = c_width * c_width;

            for index in 0..self.chunk_len() 
            {
                if block_buffer[index] == std::u8::MAX
                {
                    continue;
                }

                let index_cast = index as i32;
                
                let coords = [index_cast % c_width, (index_cast % c_area) / c_width, index_cast / c_area];

                self.chunks.insert_block(c_index, block_buffer[index], coords.into());
                
            }
        }
    }
}

