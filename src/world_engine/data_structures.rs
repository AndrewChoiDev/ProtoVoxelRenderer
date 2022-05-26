// A 3D grid with a specified radius
// Coordinates can be used to access the values
pub struct RadiusGrid<T : Clone>
{
    grid : Vec<T>,
    radius : usize,
}


impl <T : Clone> RadiusGrid<T>
{
    pub fn new(radius : usize, default_value : T)
        -> RadiusGrid<T>
    {
        let grid_volume = ((2 * radius) - 1).pow(3);

        RadiusGrid { grid: vec![default_value ; grid_volume], radius}
    }

    pub fn value(&self, grid_coordinates : [isize ; 3])
        -> T
    {
        self.grid[self.grid_index(grid_coordinates)].clone()
    }


    fn grid_index(&self, grid_coordinates : [isize ; 3])
        -> usize
    {
        let offset = (self.radius - 1) as isize;
        let index_coords = [
            grid_coordinates[0] + offset, 
            grid_coordinates[1] + offset, 
            grid_coordinates[2] + offset
        ];


        let grid_width = (self.radius * 2 - 1) as isize;

        (index_coords[0] + index_coords[1] * grid_width + index_coords[2] * grid_width * grid_width)
        as usize
    }




    pub fn set_value(&mut self, grid_coordinates : [isize ; 3], value : T)
    {
        let index = self.grid_index(grid_coordinates);
        self.grid[index] = value;
    }



    pub fn grid_slice(&self)
        -> &[T]
    {
        self.grid.as_slice()
    }
}


use nalgebra as na;

type IntPos = na::Point3<i32>;


// Simple Efficient Sparse Voxel Octree:
// It's like an esvo, but there's no contours,
// and the insertion is so simple that it wastes nodes
pub struct SESVOctree
{
    cds : Vec<ChildDescriptor>,
    pos : IntPos,
    degree : u32,
}

impl SESVOctree
{
    pub fn new(pos : IntPos, degree : u32)
        -> SESVOctree
    {
        let mut sesvo = SESVOctree {cds : Vec::new(), pos, degree};
        sesvo.push_octuple();

        sesvo
    }

    pub fn clear(&mut self)
    {
        self.cds.clear();
        self.push_octuple();
    }

    pub fn cds(&self)
        -> &[ChildDescriptor]
    {
        &self.cds
    }



    // returns the octuple index for this new octuple
    fn push_octuple(&mut self)
        -> u32
    {
        let new_octuple_index = self.cds.len() >> 3;

        for _ in 0..8 
        {
            self.cds.push(ChildDescriptor::new_null())
        }

        new_octuple_index as u32
    }

    pub fn insert(&mut self, pos : IntPos, value : u32)
    {
        let mut cd_index = 0;
        let mut node_pos = self.pos;

        for depth in 0..self.degree 
        {
            let target_octant = self.select_octant(pos, node_pos, depth);

            node_pos =  self.next_node_pos(node_pos, target_octant as i32, depth);

            let target_is_valid = self.cds[cd_index].is_child_valid(target_octant);

            let depth_is_final = depth == (self.degree - 1);

            if target_is_valid && depth_is_final
            {
                break;
            }
           
            

            if !target_is_valid
            {
                if self.cds[cd_index].is_no_child_valid()
                {
                    self.cds[cd_index].octuple_index = self.push_octuple() as u16;
                }

                self.cds[cd_index].set_child_valid(target_octant);
            }

            let child_index = (self.cds[cd_index].octuple_index << 3) as usize | target_octant as usize;

            if !target_is_valid && depth_is_final
            {
                self.cds[child_index].octuple_index = value as u16;
            }

            if !depth_is_final
            {
                cd_index = child_index as usize;
            }
        }
    }

    pub fn insert_no_val(&mut self, pos : IntPos)
    {
        let mut cd_index = 0;
        let mut node_pos = self.pos;

    
        for depth in 0..self.degree 
        {
            let target_octant = self.select_octant(pos, node_pos, depth);

            node_pos =  self.next_node_pos(node_pos, target_octant as i32, depth);

            let target_is_valid = self.cds[cd_index].is_child_valid(target_octant);

            let depth_is_final = depth == (self.degree - 1);


            if target_is_valid && depth_is_final
            {
                break;
            }

            if !depth_is_final && self.cds[cd_index].is_no_child_valid()
            {
                self.cds[cd_index].octuple_index = self.push_octuple() as u16;
            }

            if !target_is_valid
            {
                self.cds[cd_index].set_child_valid(target_octant);
            }
            
            if !depth_is_final
            {
                let child_index = (self.cds[cd_index].octuple_index << 3) as usize | target_octant as usize;

                cd_index = child_index as usize;
            }
        }
    }

    fn select_octant(&self, pos : IntPos, node_pos : IntPos, node_depth : u32)
        -> u32
    {
        let pos_relative = pos - node_pos;

        let node_width_half = 1 << (self.degree - node_depth - 1);

        pos_relative.iter()
        .enumerate()
        .filter(|(_, comp)| **comp >= node_width_half)
        .fold(0, |acc, (i, _)| acc | (1 << i))
    }

    fn next_node_pos(&self, node_pos : IntPos, octant : i32, node_depth : u32)
        -> IntPos
    {
        let child_pos_extent = na::Vector3::<i32>::new(octant % 2, (octant % 4) / 2, octant / 4);
        let child_width = 2i32.pow(self.degree - node_depth - 1);

        node_pos + (child_pos_extent * child_width)
    }
}



#[repr(align(4))]
#[derive(Clone)]
pub struct ChildDescriptor
{
    pub octuple_index : u16,
    pub valid_mask : u8, // nth bit => nth child is valid
}


impl ChildDescriptor
{
    pub fn new_null()
        -> ChildDescriptor
    {
        ChildDescriptor {octuple_index : std::u16::MAX, valid_mask : 0}
    }

    pub fn is_child_valid(&self, octant : u32)
        -> bool
    {
        ((self.valid_mask >> octant) & 1) == 1
    }


    pub fn is_no_child_valid(&self)
        -> bool
    {
        self.valid_mask == 0
    }

    pub fn set_child_valid(&mut self, octant : u32)
    {
        self.valid_mask |= 1 << octant;
    }

    pub fn to_u32(&self)
        -> u32
    {
        ((self.valid_mask as u32) << 16) | (self.octuple_index as u32)
    }
}
