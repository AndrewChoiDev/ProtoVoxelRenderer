use super::super::world_engine as world_eng;

use world_eng::ChunkGenerator;
use world_eng::voxel_manager::PrefabManager;
use world_eng::chunk_generators::{TerrainChunkGenerator};

use std::collections::HashSet;

use super::data_structures::RadiusGrid;

use super::super::input as input;

use input::KeyEventQueue;

use nalgebra as na;
use na::{Point3, Vector3};

use super::data_structures::SESVOctree;

pub struct LegacyMap
{
    chunk_trees : Vec<SESVOctree>, // maximum of ~586 nodes

    // Looks up view grid coordinates via a chunk index
    chunk_view_grid_coordinates : Vec<Vector3<i32>>,

    // flags associated with each chunk
    chunk_generated_flags : Vec<bool>, // chunk is ready to be used
    chunk_updated_flags : Vec<bool>, // chunk needs to be updated for the gpu

    // A grid for looking up a chunk ids via a grid coordinate
    chunk_id_radius_grid : RadiusGrid<usize>,

    chunk_view_tree : SESVOctree,

    // All possible grid coordinates
    view_grid_coordinate_set : HashSet<Vector3<i32>>, 

    // The world grid position of the center chunk
    world_grid_point : Point3<i32>,

    view_radius : usize,

    chunk_length : usize,
    chunk_volume : usize,
    chunk_num : usize,

    pub prefab_manager : PrefabManager,

    input_event_queue : KeyEventQueue,

    chunk_generator : Box<dyn ChunkGenerator>,
}




impl LegacyMap
{
    pub fn new(viewer_world_grid_point : [i32 ; 3], view_radius : usize, chunk_length : usize)
        -> LegacyMap
    {
        let prefab_manager = PrefabManager::new();
        let chunk_volume = chunk_length.pow(3);

        let view_grid_coordinate_set = LegacyMap::generate_view_grid_coordinate_set(view_radius);

        let (chunk_num, chunk_view_grid_coordinates) = 
            (view_grid_coordinate_set.len(), view_grid_coordinate_set.iter().cloned().collect());


        let world_grid_point = viewer_world_grid_point.into();

        let chunk_trees = 
            (0..chunk_num).into_iter()
            .map(|_| SESVOctree::new(na::Point3::origin(), 4))
            .collect();

        let chunk_view_tree = 
            SESVOctree::new(
                na::Vector3::repeat(1 - (view_radius as i32)).into(), 
                (view_radius as f32).log2().ceil() as u32);

        let map = LegacyMap {
            chunk_trees,
            chunk_view_grid_coordinates,
            chunk_generated_flags : vec![false ; chunk_num],
            chunk_updated_flags : vec![false ; chunk_num],
            chunk_id_radius_grid : RadiusGrid::new(view_radius, 0),
            chunk_view_tree,
            view_grid_coordinate_set,
            world_grid_point, 
            view_radius, chunk_length, chunk_volume, chunk_num,
            prefab_manager,
            input_event_queue : KeyEventQueue::new(set!("interact_1", "interact_2")),
            chunk_generator : Box::new(TerrainChunkGenerator::new()),
        };

        map
    }


    pub fn construct_chunk_octree(octree : &mut SESVOctree, chunk_length : usize, chunk_block_ids : &[u8])
    {
        octree.clear();

        let dims = &[chunk_length ; 3];

        for (i, block_id) in chunk_block_ids.iter().enumerate().filter(|(_, id)| **id != 0)
        {
            let chunk_grid_coords : na::Vector3<usize> =
                Self::index_to_coord(dims, i).into();
            
            octree.insert(chunk_grid_coords.map(|c| c as i32).into(), (block_id - 1) as u32);
        }
    }


    pub fn render_tree(&self)
        -> Option<&SESVOctree>
    {
        let chunk_id = self.chunk_id_radius_grid.value([0 ; 3]);


        if chunk_id != 0
        {
            return Some(&self.chunk_trees[chunk_id - 1]);
        }
        None
    }

    pub fn chunk_tree(&self, c_index : usize)
        -> &SESVOctree
    {
        &self.chunk_trees[c_index]
    }


    pub fn chunk_view_tree(&self)
        -> &SESVOctree
    {
        return &self.chunk_view_tree;
    }


    fn view_coordinate_in_radius(view_coordinate : Vector3<i32>, view_radius : f32)
        -> bool
    {
        // The magnitude of the view coordinate is compared with the radius
        view_coordinate.iter()
        .fold(0f32, |acc, c| acc + (c * c) as f32)
        .sqrt() 
            <= view_radius
    }

    fn generate_view_grid_coordinate_set(view_radius : usize)
        -> HashSet<Vector3<i32>>
    {
        let mut vg_coord_set = HashSet::new();

        let r = view_radius as i32;
        let axis_iter = (1 - r)..r;

        let view_radius = (view_radius - 1) as f32;

        for x in axis_iter.clone() {
        for y in axis_iter.clone() {
        for z in axis_iter.clone() {

            let vg_coord = Vector3::new(x, y, z);

            if LegacyMap::view_coordinate_in_radius(vg_coord, view_radius)
            {
                vg_coord_set.insert(vg_coord);
            }

        }}}

        vg_coord_set
    }



    pub fn adapt_to_world_position(&mut self, viewer_world_grid_coordinate : Point3<i32>)
    {
        let grid_delta = 
            viewer_world_grid_coordinate
            - self.world_grid_point;

        if grid_delta == na::zero::<na::Vector3<i32>>()
        {
            return;
        }

        self.world_grid_point += grid_delta;


        let mut occupied_grid_coordinate_set = HashSet::new();

        let mut chunk_index_coords_invalid = Vec::new();

        let ref mut w_vg_coords = self.chunk_view_grid_coordinates;

        // chunk index
        for c_index in 0..self.chunk_num
        {
            w_vg_coords[c_index] -= grid_delta;

            occupied_grid_coordinate_set.insert(w_vg_coords[c_index].clone());

            if !LegacyMap::view_coordinate_in_radius(w_vg_coords[c_index], (self.view_radius - 1) as f32)
            {
                chunk_index_coords_invalid.push(c_index);
            }
        }

        // An iterator over the set of grid coordinates in view but not occupied
        let mut unoccupied_grid_coordinates_iter = 
            self.view_grid_coordinate_set.difference(&occupied_grid_coordinate_set).cloned();


        for c_index in chunk_index_coords_invalid
        {
            self.chunk_generated_flags[c_index] = false;

            w_vg_coords[c_index] = unoccupied_grid_coordinates_iter.next().unwrap();
        }

        self.chunk_id_radius_grid = self.chunk_id_radius_grid_from_vg_coords();

        self.update_chunk_view_octree_from_vg_coords();

    }


    pub fn handle_events(&mut self, look_dir : Vector3<f32>, world_grid_coord : Point3<i32>, chunk_pos : Point3<f32>)
    {
        for event in self.input_event_queue.pop_events(5)
        {
            // println!("event: {:?}", event);
            match event
            {
                (keys, true) if keys.contains(&String::from("interact_1")) =>
                {
                    // if let Some(traversed_voxels) = self.trace_map(look_dir, chunk_pos, world_grid_coord)
                    // {
                    //     let (chunk_index, chunk_grid_coord) = traversed_voxels.last().unwrap();

                    //     self.set_block_id(*chunk_index, *chunk_grid_coord, 0);
                    // }
                    // if let Some(block_meta) = self.trace_map(Vector3::z(), )
                },
                _ => ()
            }
        }
    }


    
    pub fn generate_next_chunk(&mut self)
    {
        // if let Some(c_index) = self.find_chunk_to_generate()
        // {
        //     let chunk_block_id_slice = 
        //         &mut self.block_ids[c_index * self.chunk_volume..(c_index + 1) * self.chunk_volume];
        //     let chunk_world_grid_coordinate = 
        //         self.world_grid_point 
        //         + self.chunk_view_grid_coordinates[c_index];
        

        //     self.chunk_generated_flags[c_index] = true;

        //     self.chunk_generator.generate_chunk(chunk_block_id_slice, chunk_world_grid_coordinate.coords.into(), [self.chunk_length ; 3]);
        //     Self::construct_chunk_octree(&mut self.chunk_trees[c_index], self.chunk_length, chunk_block_id_slice);

        //     self.chunk_updated_flags[c_index] = true;

        //     self.chunk_id_radius_grid.set_value(
        //         self.chunk_view_grid_coordinates[c_index].map(|c| c as isize).into(),
        //         c_index + 1
        //     );
        //     self.chunk_view_tree.insert(
        //         self.chunk_view_grid_coordinates[c_index].into(), 
        //         c_index as u32
        //     );
        // }
    }

    fn find_chunk_to_generate(&self)
        -> Option<usize>
    {
        (0..self.chunk_num)
        .filter(|c_index| !self.chunk_generated_flags[*c_index]) // not occupied
        .min_by_key(|c_index| self.chunk_view_grid_coordinates[*c_index].iter().fold(0, |acc, i| acc + i * i))   
    }


    pub fn chunk_num(&self)
        -> usize
    {
        self.chunk_num
    }

    pub fn chunk_dims(&self)
        -> [usize ; 3]
    {
        [self.chunk_length ; 3]
    }



    pub fn view_radius(&self)
        -> usize
    {
        self.view_radius
    }

    pub fn flag(&self, chunk_index : usize)
        -> bool
    {
        self.chunk_updated_flags[chunk_index]
    }


    // pub fn set_block_id(&mut self, chunk_index : usize, chunk_grid_coord : [usize ; 3], id : u8)
    // {
    //     if id == 0
    //     {
    //         return;
    //     }

    //     let insert_pos = na::Point3::new(chunk_grid_coord[0] as i32, chunk_grid_coord[1] as i32, chunk_grid_coord[2] as i32);

    //     self.block_ids[chunk_index * self.chunk_volume + Self::coord_to_index(&[self.chunk_length ; 3], chunk_grid_coord)] = id;
    //     self.chunk_trees[chunk_index].insert(insert_pos, (id - 1) as u32);
    //     self.chunk_updated_flags[chunk_index] = true;
    // }

    pub fn coord_to_index(dims : &[usize ; 3], coords : [usize ; 3])
        -> usize
    {
        coords[0] + coords[1] * dims[0] + coords[2] * dims[0] * dims[1]
    }

    pub fn index_to_coord(dims : &[usize ; 3], index : usize)
        -> [usize ; 3]
    {
        let area = dims[0] * dims[1];
        [index % dims[0], (index % area) / dims[0], index / area]
    }


    pub fn block_ids_closest_flagged_chunk(&mut self)
        -> Option<(usize, Vec<u8>)>
    {
    //     let some_chunk_index =
    //         self.chunk_updated_flags.iter()
    //         .enumerate()
    //         .filter_map(|(i, flag)| if *flag {Some(i)} else {None})
    //         .min_by_key(|i|
    //         {
    //             // magnitude of the grid position
    //             self.chunk_view_grid_coordinates[*i].iter().fold(0, |acc, j| acc + j * j)
    //         });
        
    //     if let Some(chunk_index) = some_chunk_index
    //     {
    //         {
    //             self.chunk_updated_flags[chunk_index] = false;
    //         }
    //         Some((chunk_index, self.block_ids(chunk_index)))
    //     }
    //     else
    //     {
    //         None
    //     }

        None
    }

    pub fn map_grid_img_data(&self)
        -> Vec<u32>
    {
    
        self.chunk_id_radius_grid.grid_slice().iter().map(|c| *c as u32).collect()
    }

    fn chunk_id_radius_grid_from_vg_coords(&self)
        -> RadiusGrid<usize>
    {
        let mut radius_grid = RadiusGrid::new(self.view_radius, 0);

        for c_index in 0..self.chunk_num
        {
            let vg_pos = self.chunk_view_grid_coordinates[c_index];

            // Chunk is ignored if it is flagged as not generated
            // or if it's out of view range
            if !self.chunk_generated_flags[c_index]
                || !LegacyMap::view_coordinate_in_radius(vg_pos, (self.view_radius - 1) as f32)
            {
                continue;
            }

            radius_grid.set_value(vg_pos.map(|c| c as isize).into(), c_index + 1);
        }

        radius_grid
    }

    fn update_chunk_view_octree_from_vg_coords(&mut self)
    {
        self.chunk_view_tree.clear();

        for c_index in 0..self.chunk_num
        {
            let vg_pos = self.chunk_view_grid_coordinates[c_index];

            if !self.chunk_generated_flags[c_index]
                || !LegacyMap::view_coordinate_in_radius(vg_pos, (self.view_radius - 1) as f32)
            {
                continue;
            }

            self.chunk_view_tree.insert(vg_pos.into(), c_index as u32);
        }
    }

    pub fn input_queue(&mut self)
        -> &mut KeyEventQueue
    {
        &mut self.input_event_queue
    }
}