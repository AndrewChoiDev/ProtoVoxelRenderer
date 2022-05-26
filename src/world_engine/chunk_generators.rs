use noise::NoiseFn;
use nalgebra as na;

pub struct TestChunkGenerator
{
}

impl TestChunkGenerator
{
    pub fn new()
        -> TestChunkGenerator
    {
        TestChunkGenerator {}
    }
}

impl super::ChunkGenerator for TestChunkGenerator
{
    fn generate_chunk(&self,
        block_ids : &mut[u8], 
        world_grid_position : [i32 ; 3], 
        chunk_dims : [usize ; 3])
    {
        let noise_func = noise::SuperSimplex::new();

        for i in 0..(block_ids.len())
        {
            if world_grid_position[1] > 0
            {
                block_ids[i] = 0;
                continue
            }
            let [u,v,w] = index_to_uvw(&chunk_dims, i);

            let mut val = 0;
            let centered_uvw =
            {
                let center = |a| 2.0 * (a - 0.5);
                (center(u), center(v), center(w))
            };
            let c_uvw_len = centered_uvw.0.hypot(centered_uvw.1).hypot(centered_uvw.2);
            if c_uvw_len > 1.3
            {
                val = 1;
            }
            let frequency = 1.1;
            let noise_input = [
                (u as f64 + (world_grid_position[0] as f64)) * frequency, 
                (w as f64 + (world_grid_position[2] as f64)) * frequency
            ];
            
            if val == 0 && v < 0.5 && v < (noise_func.get(noise_input) * 0.3 + 0.2) as f32
            {
                val = 2;
            }
            block_ids[i] = val;
        }
    }
}


pub struct TerrainChunkGenerator
{
    noise_gen : noise::SuperSimplex,
}


impl TerrainChunkGenerator
{
    pub fn new()
        -> TerrainChunkGenerator
    {
        TerrainChunkGenerator {noise_gen : noise::SuperSimplex::new()}
    }
}

impl super::ChunkGenerator for TerrainChunkGenerator
{
    fn generate_chunk(&self,
        block_ids : &mut[u8], 
        world_grid_position : [i32 ; 3], 
        chunk_dims : [usize ; 3])
    {
        let sphere_chunk_grid_pos = na::Point3::new(3, 7, 9);
        let sphere_world_grid_pos = na::Point3::new(1, 2, -1);

        let chunk_dims_signed = [chunk_dims[0] as i32, chunk_dims[1] as i32, chunk_dims[2] as i32];

        for x in 0..(chunk_dims_signed[0])
        {
        for z in 0..(chunk_dims_signed[1])
        {
            let frequency = 0.3;

            
            let noise_input = 
            [
                ((x + (world_grid_position[0] * chunk_dims_signed[0])) as f64) * frequency,
                ((z + (world_grid_position[2] * chunk_dims_signed[2])) as f64) * frequency,
            ];
            
            let noise_value = (self.noise_gen.get(noise_input) * 6.0).floor() as i32;

            for y in 0..(chunk_dims_signed[2])
            {
                let mut val = std::u8::MAX;

                let world_y_pos = y + (world_grid_position[1] * chunk_dims_signed[2]);

                if world_y_pos < noise_value - 6
                {
                    val = 2;
                }
                else if world_y_pos < noise_value - 5
                {
                    val = 1;
                }

                let index = coord_to_index(&chunk_dims, [x as usize, y as usize, z as usize]);

                let world_grid_point : na::Point3<i32> = world_grid_position.into();

                let sphere_disp = 
                    ((sphere_world_grid_pos - world_grid_point) * 16)
                    + (sphere_chunk_grid_pos - na::Point3::new(x, y, z));

                let radius = 15;
            
                if sphere_disp.fold(0, |acc, c| acc + c * c) <= (radius * radius)
                {
                    val = 3;
                }

                block_ids[index] = val;

            }
        }
        }
    } 
}



    pub fn coord_to_index(dims : &[usize ; 3], coords : [usize ; 3])
        -> usize
    {
        coords[0] + coords[1] * dims[0] + coords[2] * dims[0] * dims[1]
    }

    pub fn index_to_coord(dims : &[usize ; 3], i: usize)
        -> [usize ; 3]
    {
        let z = i / (dims[0] * dims[1]);

        let y = i / (dims[0]) - z * dims[1];

        let x = i % dims[0];

        [x, y, z]
    }

    // Ranges : ([], [], [])
    pub fn coord_to_uvw(dims : &[usize ; 3], coord : &[usize ; 3])
        -> [f32 ; 3]
    {
        let pos = |a, b| (a as f32 + 0.5) / b as f32;

        [pos(coord[0], dims[0]), pos(coord[1], dims[1]), pos(coord[2], dims[2])]
    }

    pub fn index_to_uvw(dims : &[usize ; 3], i: usize)
        -> [f32 ; 3]
    {
        coord_to_uvw(dims, &index_to_coord(dims, i))
    }