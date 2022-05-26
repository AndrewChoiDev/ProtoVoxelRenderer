use std::collections::HashSet;

use super::data_structures::SESVOctree;
use nalgebra as na;

pub const CHUNK_EXPONENT: u32 = 4;

// Displaced Chunks are chunks of map data that are used
// based on their integer displacement from the chunk the viewer resides in.
// All chunk properties are accessed via a unique chunk index.
// Properties are stored using a "Structure of Arrays" format.
pub struct DisplacedChunks
{
    // A block in a chunk is stored via an identifying number
    // that acts as an index to a prefab list. These numbers are stored
    // in an octree format. 
    blocks : Vec<SESVOctree>,
    
    // All chunks are assigned a displacement from the viewer's current chunk,
    // e.g., the chunk directly above the player's current chunk 
    // is given the displacement (0, 1, 0)
    displacement : Vec<na::Vector3<i32>>,
    
    in_use_flags : Vec<bool>,
    
    // If true, then the chunk needs to be written to the gpu
    dirty_flags : Vec<bool>,
    
    // The set of all possible chunk displacements
    displacement_set : HashSet<na::Vector3<i32>>,
}


impl DisplacedChunks
{
    pub fn new(displacement_set : HashSet<na::Vector3<i32>>)
        -> DisplacedChunks
    {
        let num = displacement_set.len();

        let blocks = 
            (0..num).into_iter()
            .map(|_| SESVOctree::new(na::Point3::origin(), CHUNK_EXPONENT))
            .collect();

        DisplacedChunks 
        {
            blocks,
            displacement : displacement_set.iter().cloned().collect(),
            in_use_flags : vec![false ; num],
            dirty_flags : vec![false ; num],
            displacement_set,
        }
    }

    pub fn len(&self)
        -> usize
    {
        self.blocks.len()
    }

    pub fn insert_block(&mut self, chunk_index : usize, block : u8, pos_in_chunk : na::Point3<i32>)
    {
        if !self.in_use_flags[chunk_index]
        {
            panic!("Attempted to insert into chunk not in use!");
        }

        self.blocks[chunk_index].insert(pos_in_chunk, block as u32);
        self.dirty_flags[chunk_index] = true;
    }



    fn closest_unused_chunk_index(&self)
        -> Option<usize>
    {
        (0..self.len())
        .filter(|&index| 
            !self.in_use_flags[index])
        .min_by_key(|&index| 
            DisplacedChunks::mag_squared(self.displacement[index]))
    }



    // gets the squared magnitude of an i32 na vector
    fn mag_squared(disp : na::Vector3<i32>)
        -> i32
    {
        disp.iter().fold(0, |acc, i| acc + i * i)
    }

    // All chunks are shifted and assigned new displacements.
    // If this new displacement is NOT within the set,
    // then it will be flagged as unused and given a different, unoccupied displacement

    // Invalid chunk indices will be returned
    pub fn displace(&mut self, displacement : na::Vector3<i32>)
        -> Vec<usize>
    {
        let mut occupied_displacement_set = HashSet::new();
        let mut invalid_indices = Vec::new();

        // The chunks are displaced in this loop
        // Invalid displacements are recorded for the next loop
        for index in 0..self.len()
        {
            self.displacement[index] += displacement;

            occupied_displacement_set.insert(self.displacement[index].clone());

            // new displacement is not part of the set
            // the index is now invalid
            if !self.displacement_set.contains(&self.displacement[index])
            {
                invalid_indices.push(index);
            }
        }

        let mut unoccupied_displacement_iter =
            self.displacement_set.difference(&occupied_displacement_set).cloned();

        // Invalid chunk updating (flagging, new valid displacement)
        for &invalid_index in &invalid_indices
        {
            self.in_use_flags[invalid_index] = false;
            
            self.displacement[invalid_index] = unoccupied_displacement_iter.next().unwrap();
        }

        invalid_indices
    }
    
    pub fn use_chunk(&mut self)
        -> Option<usize>
    {
        let some_index = self.closest_unused_chunk_index();
        
        if let Some(index) = some_index
        {
            self.blocks[index].clear();

            self.in_use_flags[index] = true;
            self.dirty_flags[index] = true;
        }

        some_index
    }

    pub fn get_displacement(&self, index : usize)
        -> na::Vector3<i32>
    {
        self.displacement[index]
    }

    pub fn is_chunk_in_use(&self, index : usize)
        -> bool
    {
        self.in_use_flags[index]
    }

    // return none if the tree is not in use
    pub fn get_tree(&self, index : usize)
        -> Option<&SESVOctree>
    {
        if !self.in_use_flags[index]
        {
            return None;
        }

        Some(&self.blocks[index])
    }
}