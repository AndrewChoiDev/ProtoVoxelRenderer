use std::sync::{Arc};
use vulkano::
{
    device::{Queue},
    descriptor::
    {
        descriptor_set::{UnsafeDescriptorSetLayout,},
        PipelineLayoutAbstract,
    },
    image::{AttachmentImage, ImageUsage, StorageImage, Dimensions},
    sampler::{Sampler, SamplerAddressMode, Filter, MipmapMode},    
    format::{Format},
    sync::GpuFuture,
    command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState},
    
};

use nalgebra as na;
use na::{Vector3};



// use image;

pub struct ImgData
{
    raster_color : Arc<AttachmentImage>,
    raster_depth : Arc<AttachmentImage>,

    ray_directions : Arc<StorageImage<Format>>,
    world_color : Arc<StorageImage<Format>>,
    world_depth : Arc<StorageImage<Format>>,

    postprocessed : Arc<StorageImage<Format>>,
    swapchain_format : Format,

    linear_sampler : Arc<Sampler>,
    nearest_sampler : Arc<Sampler>,
}


impl ImgData
{
    pub fn new(
        dims: [u32 ; 2], 
        logical_device: Arc<vulkano::device::Device>,
        queue : Arc<Queue>,
        swapchain_format: Format)
        -> ImgData
    {
        let compute_family_iter = vec!(queue.family());

        let device = logical_device.clone();


        let postprocessed =
            StorageImage::with_usage(
                device.clone(),
                Dimensions::Dim2d {width: dims[0], height:dims[1]},
                vulkano::format::Format::R16G16B16A16Unorm,
                ImageUsage {storage: true, transfer_destination: true, transfer_source: true, ..ImageUsage::none()},
                compute_family_iter.clone()
                // ImageUsage {}
            ).unwrap();
        
        let raster_color = 
            AttachmentImage::with_usage(
                device.clone(),
                dims,
                vulkano::format::Format::B10G11R11UfloatPack32,
                ImageUsage {storage: true, transfer_source:true, sampled: true, transfer_destination: true, color_attachment: true,..ImageUsage::none()}
            ).unwrap();

        let raster_depth =
            AttachmentImage::with_usage(
                device.clone(), 
                dims,             
                vulkano::format::Format::D32Sfloat,
                ImageUsage {depth_stencil_attachment: true, sampled: true, ..ImageUsage::none()}
            ).unwrap();

        let world_color = 
            StorageImage::with_usage(
                device.clone(),
                Dimensions::Dim2d {width: dims[0], height: dims[1]},
                vulkano::format::Format::B10G11R11UfloatPack32,
                ImageUsage {storage: true, transfer_destination: true, transfer_source: true, ..ImageUsage::none()},
                compute_family_iter.clone()
            ).unwrap();

        let world_depth = 
            StorageImage::with_usage(
                device.clone(),
                Dimensions::Dim2d {width: dims[0], height: dims[1]},
                vulkano::format::Format::R32Sfloat,
                ImageUsage {storage: true, ..ImageUsage::none()},
                compute_family_iter.clone(),
            ).unwrap();

        let ray_directions =
            StorageImage::with_usage(
                device.clone(),
                Dimensions::Dim2d {width: dims[0], height: dims[1]},
                vulkano::format::Format::R16G16B16A16Snorm,
                ImageUsage {storage: true, ..ImageUsage::none()},
                compute_family_iter.clone()
            ).unwrap();
    
        let linear_sampler =
            vulkano::sampler::Sampler::new(
                device.clone(),
                Filter::Linear,
                Filter::Linear,
                MipmapMode::Linear,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                0.0, 1.0, 0.0, 1.0
            ).unwrap();
            
        let nearest_sampler =
            vulkano::sampler::Sampler::new(
                device.clone(),
                Filter::Nearest,
                Filter::Nearest,
                MipmapMode::Linear,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                SamplerAddressMode::ClampToEdge,
                0.0, 1.0, 0.0, 1.0
            ).unwrap();
        
        ImgData
        {
            swapchain_format, ray_directions, world_color, world_depth, postprocessed, raster_color, raster_depth, linear_sampler, nearest_sampler
        }
    }
}


pub fn get_set_layouts(pipeline : &dyn PipelineLayoutAbstract)
    -> Vec<Arc<UnsafeDescriptorSetLayout>>
{
    (0..pipeline.num_sets()).into_iter()
    .map(|i| pipeline.descriptor_set_layout(i).unwrap().clone()).collect()
}

use super::vk_init::VkRenderContext;
mod vox_map_context;
use vox_map_context::VoxMapContext;


mod top_down_world_drawer;


mod entity_drawer;
use entity_drawer::EntityDrawer;

mod postprocess_drawer;
use postprocess_drawer::PostprocessDrawer;

pub struct VoxDrawer
{
    vk_ctx : Arc<VkRenderContext>,

    imgs : ImgData,
    aspect_ratio : f32,

    tree_draw : top_down_world_drawer::WorldDrawer,
    entity_draw : EntityDrawer,
    post_process_draw : PostprocessDrawer,

    vox_map_ctx : VoxMapContext,

    prior_future_state : Option<Box<dyn GpuFuture>>,

    frame_num : u32,
}

#[derive(Debug)]
pub struct CameraParameters
{
    pub chunk_position : na::Point3<f32>,
    pub world_grid_coords : na::Point3<i32>,
    pub orientation : na::UnitQuaternion<f32>,
}


pub const NEAR : f32 = 0.001;
pub const FAR : f32 = 200.0;

use super::super::world_engine as world_eng;
use world_eng::map::Map;

use super::vk_init::VkWindowContext;

impl VoxDrawer

{
    pub fn new(vk_ctx : Arc<VkRenderContext>, swapchain_format : Format, dims : [u32 ; 2], map : &Map)
        -> VoxDrawer
    {
        let device = vk_ctx.logical_device.clone();
        let queue = vk_ctx.queue.clone();

        let imgs = ImgData::new(dims, device.clone(), queue.clone(), swapchain_format);

        // find a better way to store/recalculate this variable later...
        let aspect_ratio = 
            dims[0] as f32 / dims[1] as f32;
        
        let entity_draw = EntityDrawer::new(vk_ctx.clone(), &imgs);
        let post_process_draw = PostprocessDrawer::new(vk_ctx.clone(), &imgs);

        let tree_draw = top_down_world_drawer::WorldDrawer::new(vk_ctx.clone(), &imgs);

        let vox_map_ctx = VoxMapContext::new(queue.clone(), map, &tree_draw.pipelines);

        let prior_future_state = Some(Box::new(vulkano::sync::now(device.clone())) as Box<dyn GpuFuture>);

        VoxDrawer {
            vk_ctx, 
            imgs,
            aspect_ratio,
            entity_draw, post_process_draw, tree_draw, 
            vox_map_ctx, 
            prior_future_state,
            frame_num : 0
        }
    }



    pub fn render_frame<W>(&mut self, 
        win_ctx : &VkWindowContext<W>, 
        camera : CameraParameters, map : &mut Map)
        where W : Send + Sync + 'static
    {
        let device = self.vk_ctx.logical_device.clone();
        let queue = self.vk_ctx.queue.clone();

        self.prior_future_state.as_mut().unwrap().cleanup_finished();

        self.update_render_scale();

        let (img_num, _, acquire_future) =
            match vulkano::swapchain::acquire_next_image(win_ctx.swapchain.clone(), None)
            {
                Ok(r) => r,
                Err(err) => panic!("{:?}", err)
            };

        let cmd_map_update = self.vox_map_ctx.update(queue.clone(), map);

        let ratio = self.aspect_ratio;
        let model_pos = 1.00 * Vector3::new(0.0f32, 0.6, 0.90);

        let cmd_raster_render = 
            self.entity_draw.cmd_buf_raster(
                &win_ctx.dynamic_state, ratio, camera.orientation, camera.chunk_position, model_pos);
            

        let cmd_depth_to_length =
            self.tree_draw.cmd_buf_depth_to_length(
                &win_ctx.dynamic_state,
                camera.orientation, 
                camera.chunk_position
            );

        let frame_index = ((self.frame_num % 64) + 1) as i32;

        // let cmd_world_render =
        //     self.world_draw.cmd_buf_draw(
        //         &win_ctx.dynamic_state, &self.vox_map_ctx, camera, frame_index
        //     );
        
        let cmd_tree_render = 
            self.tree_draw.cmd_buf_draw(
                &win_ctx.dynamic_state, &self.vox_map_ctx, camera, frame_index
            );
        
        let cmd_post_process =
            self.post_process_draw.cmd_post_process(
                &win_ctx.dynamic_state,
            );
        
        let cmd_swapchain_blit =
            self.cmd_buf_blit_to_swapchain(
                &win_ctx.dynamic_state,
                win_ctx.images[img_num].clone()
            );
        
        let future_one =
            self.prior_future_state.take().unwrap()
            .then_execute(queue.clone(), cmd_map_update).unwrap()
            .then_execute(queue.clone(), cmd_raster_render).unwrap()
            .then_execute(queue.clone(), cmd_depth_to_length).unwrap()
            // .then_execute(queue.clone(), cmd_world_render).unwrap()
            .then_execute(queue.clone(), cmd_tree_render).unwrap()
            .then_execute(queue.clone(), cmd_post_process).unwrap();

        
        future_one.then_signal_fence().wait(None).unwrap();

        let future_two =
            vulkano::sync::now(device.clone())
            .join(acquire_future)
            .then_execute(queue.clone(), cmd_swapchain_blit).unwrap()
            .then_swapchain_present(queue.clone(), win_ctx.swapchain.clone(), img_num)
            .then_signal_fence_and_flush();

        match future_two
        {
            Ok(future) =>
            {
                self.prior_future_state = Some(Box::new(future));
            }
            Err(e) =>
            {
                println!("{:?}", e);
                self.prior_future_state = Some(Box::new(vulkano::sync::now(device.clone())));
            }
        }

        self.frame_num += 1;
    }

    // implement later...
    pub fn update_render_scale(&mut self)
    {

    }

    pub fn update_dims(&mut self, dims : [u32 ; 2])
    {
        self.imgs = ImgData::new(dims, self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.clone(), self.imgs.swapchain_format);
    }

    pub fn cmd_buf_blit_to_swapchain<W : 'static>(&self, dynamic_state : &DynamicState, target : Arc<vulkano::image::SwapchainImage<W>>)
        -> AutoCommandBuffer
        where W : Send + Sync
    {
        let render_extent =
        {
            let dims = dynamic_state.viewports.iter().next().unwrap()[0].dimensions;
            [dims[0] as i32, dims[1] as i32, 1]
        };
        let win_extent =
        {
            let dims : [u32 ; 2] = target.dimensions();
            [dims[0] as i32,dims[1] as i32,1]
        };
        
        let mut acbb = 
            AutoCommandBufferBuilder::primary_one_time_submit(
                    self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.family()).unwrap();

        acbb
        .blit_image(
            self.imgs.postprocessed.clone(),
            [0 ; 3], render_extent, 0, 0,
            target.clone(),
            [0; 3], win_extent, 0, 0,
            1, vulkano::sampler::Filter::Nearest
        ).unwrap();

        acbb
        .build().unwrap()
    }

}


pub fn get_vertical_fov(diagonal_fov : f32, ratio : f32)
    -> f32
{
    (
        (diagonal_fov.to_radians() / 2.0).tan() 
        / (1.0 + ratio.powi(2)).sqrt()
    ).atan() * 2.0
}
