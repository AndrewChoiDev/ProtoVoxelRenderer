pub struct VoxelBarrel
{
    dims : [usize ; 3],
    top_texture : Vec<u32>,
    side_front_texture : Vec<u32>,
    side_right_texture : Vec<u32>,
    bottom_texture : Vec<u32>
}

use super::dot_vox_wrapper::DotVoxWrapper;
use super::byte_voxels::BitVoxels;
use super::VoxFace;

impl VoxelBarrel
{
    pub fn new(vox_file_path : &str)
        -> VoxelBarrel
    {
        let vox_data_wrap = DotVoxWrapper::new(vox_file_path);

        let volume = BitVoxels::new(&vox_data_wrap, 0);

        let dims = vox_data_wrap.dims(0);

        // let slice = get_2D_slice(&model, VoxFace::XZ, 1);
        let side_front_texture = super::get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::XY, [false, true, false]);
        let top_texture = super::get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::XZ, [false, true, true]);
        let bottom_texture = super::get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::XZ, [true, true, false]);
        let side_right_texture = super::get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::ZY, [false, true, true]);
        // let side_left_texture = get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::ZY, [true, true, false]);
        // let side_back_texture = get_ortho_texture(&volume, &vox_data_wrap, 0, VoxFace::XY, [true, true, true]);
        // let side_four_texture = get_ortho_texture(&volume, &vox_data, 0, VoxFace::YZ, false);


        // VoxBarrel::save_texture(24, 24, 
        //     &side_front_texture, "side");

        VoxelBarrel {dims, top_texture, side_front_texture, side_right_texture, bottom_texture}
    }
    

    pub fn save_texture(width : u32, height : u32, texture : &Vec<u32>, name : &str)
    {
        // how do i do this via iterator?
        let byte_texture : Vec<u8> = 
        {
            let mut result = Vec::new();

            texture.iter().for_each( |color|
                result.extend(
                    color.to_le_bytes().iter()
                )
            );

            result
        };

        let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, byte_texture.as_slice()).unwrap();

        image.save(format!("resources/saved_test_textures/{}.png", name)).unwrap();
    }
}

impl super::TexturedVoxelPrefab for VoxelBarrel
{
    fn dims(&self)
        -> [usize ; 3]
    {
        self.dims
    }

    fn get_texture(&self)
        -> Vec<u32>
    {
        self.top_texture.iter()
        .chain(self.side_front_texture.iter())
        .chain(self.side_right_texture.iter())
        .chain(self.bottom_texture.iter())
        .cloned().collect()
    }
    

    fn texture_count(&self)
        -> u32
    {4}

    fn face_texture_offsets(&self)
        -> [u32 ; 6]
    {
        [0, 1, 2, 2, 1, 3]
    }
}