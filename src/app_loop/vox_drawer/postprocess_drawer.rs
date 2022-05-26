use std::sync::{Arc};


use vulkano::
{
    device::{Device},
    descriptor::
    {
        DescriptorSet,    
        descriptor_set::{FixedSizeDescriptorSetsPool, UnsafeDescriptorSetLayout,},
    },  
    pipeline::{ComputePipeline, ComputePipelineAbstract},
    command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState},
    // pipeline::viewport::Viewport,
    
};


pub struct Pipelines
{
    post : Arc<dyn ComputePipelineAbstract + Send + Sync>,

    pub post_set_layouts : Vec<Arc<UnsafeDescriptorSetLayout>>,
}

mod ms {
    vulkano_shaders::shader!{
        ty:     "compute",
        path:   "shaders/monitor_pass.glsl"
    }
}

impl Pipelines
{
    pub fn new(logical_device : Arc<Device>)
        -> Pipelines
    {
        let device = logical_device;

        let post =
            Arc::new(
                ComputePipeline::new(
                    device.clone(),
                    &ms::Shader::load(device.clone()).unwrap().main_entry_point(), &()
                ).unwrap()
            );
        
        let post_set_layouts = super::get_set_layouts(&post.clone());

        Pipelines {
            post,
            post_set_layouts}
    }
}


struct DescriptorSetsData
{
    post_pool : FixedSizeDescriptorSetsPool,

    pub post_set : Arc<dyn DescriptorSet + Send + Sync>,
}

impl DescriptorSetsData
{
    pub fn new(
        imgs : &super::ImgData,
        pips : &Pipelines,
        )
        -> DescriptorSetsData
    {
        let mut post_pool = 
            FixedSizeDescriptorSetsPool::new(pips.post_set_layouts[0].clone());

        let post_set =
            Arc::new(
                post_pool.next()
                .add_image(imgs.world_color.clone()).unwrap()
                .add_image(imgs.world_depth.clone()).unwrap()
                .add_image(imgs.postprocessed.clone()).unwrap()
                .build().unwrap()
            );
        

        DescriptorSetsData {
            post_pool, 
            post_set}
    }
}

use super::super::vk_init::VkRenderContext;

pub struct PostprocessDrawer
{
    vk_ctx : Arc<VkRenderContext>,
    pub pipelines : Pipelines,
    dsets : DescriptorSetsData,
}

impl PostprocessDrawer
{
    pub fn new(vk_ctx : Arc<VkRenderContext>, imgs : &super::ImgData)
        -> PostprocessDrawer
    {
        let device = vk_ctx.logical_device.clone();
        let pipelines = Pipelines::new(device.clone());
        let dsets = 
            DescriptorSetsData::new(
                &imgs,
                &pipelines
            );
        PostprocessDrawer {vk_ctx, pipelines, dsets}
    }
    pub fn cmd_post_process(&self,
        dynamic_state : &DynamicState,
        )
        -> AutoCommandBuffer
    {
        let dynamic_state_dims = dynamic_state.viewports.as_ref().unwrap()[0].dimensions;

        let compute_dims = [(dynamic_state_dims[0] as u32 + 7) / 8, (dynamic_state_dims[1] as u32 + 7) / 8, 1];

        let mut acbb = 
            AutoCommandBufferBuilder::primary_one_time_submit(
                self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.family()).unwrap();
        acbb
        // final tonemapping and gamma correction
        .dispatch(
            compute_dims,
            self.pipelines.post.clone(),
            vec!(
                self.dsets.post_set.clone(),
            ),
            ()
        ).unwrap();

        acbb.build().unwrap()
    }
}