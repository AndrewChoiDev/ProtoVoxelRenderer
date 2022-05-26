use std::sync::{Arc};
use vulkano::
{
    device::{Queue},
    descriptor::
    {
        DescriptorSet,    
        descriptor_set::{FixedSizeDescriptorSetsPool, PersistentDescriptorSet},
    },
    image::{StorageImage, Dimensions, ImageUsage, ImmutableImage},
    format::{Format},
    sampler,
    buffer::{CpuBufferPool},
    command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder},
    sync::GpuFuture,
};

use super::super::super::world_engine as world_eng;

use world_eng::map::Map;

use world_eng::data_structures::SESVOctree;



// use super::top_down_world_drawer::ray_traverse_cs as TreeGLSL;
// use world_eng::data_structures::ChildDescriptor;

pub struct VoxMapContext
{
    
    palette_volume_atlas : Arc<ImmutableImage<Format>>,
    palette_array : Arc<ImmutableImage<Format>>,

    sampler : Arc<sampler::Sampler>,

    tree_img : Arc<StorageImage<Format>>,
    
    tree_pool : FixedSizeDescriptorSetsPool,
    pub tree_set : Arc<dyn DescriptorSet + Send + Sync>,
    pub prefab_set : Arc<dyn DescriptorSet + Send + Sync>,

    tree_update_buffer : CpuBufferPool<u32>,

    chunk_count : u32,
}


impl VoxMapContext
{
    pub fn new(
        queue : Arc<vulkano::device::Queue>, 
        map : &Map, 
        pips_tree : &super::top_down_world_drawer::Pipelines)
        -> VoxMapContext
    {
        let chunk_count = map.chunk_count() as u32;

        let tree_update_buffer = 
            CpuBufferPool::upload(queue.device().clone());

        let mut tree_pool = FixedSizeDescriptorSetsPool::new(pips_tree.ray_traverse_set_layouts[0].clone());

        let tree_img = allocate_vk_tree_img(queue.clone(), chunk_count, 1);


        let sampler = 
            sampler::Sampler::new(
                queue.device().clone(), 
                sampler::Filter::Nearest, 
                sampler::Filter::Nearest, 
                sampler::MipmapMode::Nearest, 
                sampler::SamplerAddressMode::ClampToEdge, 
                sampler::SamplerAddressMode::ClampToEdge, 
                sampler::SamplerAddressMode::ClampToEdge, 
                0.0, 1.0, 0.0, 1.0
            ).unwrap();

        
        let (palette_volume_atlas, pva_future) = 
        ImmutableImage::from_iter(
            map.prefab_manager.prefabs[0].palette_volume().iter().cloned(),
            Dimensions::Dim3d {width: 32, height: 32, depth: 32},
            Format::R8Uint,
            queue.clone()
        ).unwrap();
        
        

        let (palette_array, pa_future) = 
        ImmutableImage::from_iter(
            map.prefab_manager.prefabs[0].palette().iter().cloned(),
            Dimensions::Dim1dArray {width: 256, array_layers: 1},
            Format::R8G8B8A8Unorm,
            queue.clone()
        ).unwrap();
        
        let prefab_set =
            Arc::new(PersistentDescriptorSet::start(pips_tree.ray_traverse_set_layouts[2].clone())
            .add_sampled_image(palette_volume_atlas.clone(), sampler.clone()).unwrap()
            .add_sampled_image(palette_array.clone(), sampler.clone()).unwrap()
            .build().unwrap());

        let tree_set =
            Arc::new(
                tree_pool.next()
                .add_image(tree_img.clone()).unwrap()
                .build()
                .unwrap()
            );

        let vmc = 
            VoxMapContext { palette_volume_atlas, palette_array, sampler, tree_img, tree_pool, tree_set, prefab_set,
                tree_update_buffer, chunk_count};

        let mut init_acbb =
            AutoCommandBufferBuilder::primary_one_time_submit(queue.device().clone(), queue.family()).unwrap();

        vmc.insert_prefabs(&mut init_acbb, &map);


        vulkano::sync::now(queue.device().clone())
        .then_execute(queue.clone(), init_acbb.build().unwrap()).unwrap()
        .join(pa_future)
        .join(pva_future)
        .then_signal_fence()
        .wait(None).unwrap();
    
        vmc
    }

    pub fn insert_prefabs(&self, acbb : &mut AutoCommandBufferBuilder, map : &Map)
    {
        self.update_image_tree(acbb, &map.prefab_manager.prefabs[0].tree_volume(), (4 + map.chunk_count()) as u32);
    }
    
    
    pub fn update(&mut self, queue : Arc<vulkano::device::Queue>, map : &mut Map)
        -> AutoCommandBuffer
    {
        // let octree = map.construct_octree();
        let mut builder =
            AutoCommandBufferBuilder::primary_one_time_submit(queue.device().clone(), queue.family()).unwrap();

        // update render tree for chunks
        if let Some(tree) = map.render_tree()
        {
            self.update_image_tree(&mut builder, tree, 4);
        }

        builder
        .build().unwrap()
    }

    fn update_image_tree(&self, acbb : &mut AutoCommandBufferBuilder, tree : &SESVOctree, layer : u32)
    {
        let nodes : Vec<u32> = tree.cds().iter().map(|n| n.to_u32()).collect();
        let node_len = nodes.len();

        let tree_data = Arc::new(self.tree_update_buffer.chunk(nodes).unwrap());

        acbb
        .copy_buffer_to_image_dimensions(
            tree_data,
            self.tree_img.clone(),
            [0 ; 3],
            [node_len as u32, 1, 1],
            layer,
            1,
            0
        ).unwrap();
    }

    pub fn chunk_count(&self)
        -> u32
    {
        self.chunk_count
    }

}

fn allocate_vk_tree_img(queue : Arc<Queue>, chunk_count : u32, volume_prefab_count : u32)
    -> Arc<StorageImage<Format>>
{
    const TOP_TREE_NUM : u32 = 4; // world grid trees

    StorageImage::with_usage(
        queue.device().clone(),
        Dimensions::Dim1dArray {
            width : 4700, 
            array_layers : TOP_TREE_NUM + chunk_count + volume_prefab_count},
        Format::R32Uint,
        ImageUsage {transfer_destination : true, storage : true, ..ImageUsage::none()},
        vec!(queue.family()),
    ).unwrap()
}

// fn allocate_vk_map_img(chunk_dims : [usize ; 3], chunk_num : usize, queue : Arc<Queue>)
//     -> Arc<StorageImage<Format>>
// {
//     let (width, height, depth) =
//         (chunk_dims[0] as u32, chunk_dims[1] as u32,
//         (chunk_dims[2] * chunk_num) as u32);
//     let img = StorageImage::with_usage(
//             queue.device().clone(),
//             Dimensions::Dim3d {width, height, depth},
//             Format::R8Uint,
//             ImageUsage {transfer_destination: true, sampled: true, ..ImageUsage::none()},
//             vec!(queue.family()),
//         ).unwrap();
//     img
// }

// pub fn allocate_vk_map_grid_img(view_radius : usize, queue : Arc<Queue>)
//     -> Arc<StorageImage<Format>>
// {
//     let diameter = (view_radius * 2 - 1) as u32;
//     let img = StorageImage::with_usage(
//             queue.device().clone(),
//             Dimensions::Dim3d {width : diameter, height : diameter, depth : diameter},
//             Format::R32Uint,
//             ImageUsage {transfer_destination: true, sampled: true, ..ImageUsage::none()},
//             vec!(queue.family()),
//         ).unwrap();
//     img
// }