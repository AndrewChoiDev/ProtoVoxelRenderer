use std::sync::{Arc};

use vulkano::
{
    device::{Device},
    descriptor::
    {
        descriptor_set::{UnsafeDescriptorSetLayout,},
    },
    framebuffer::{RenderPassAbstract, Subpass, FramebufferAbstract, Framebuffer},
    pipeline::{GraphicsPipeline, GraphicsPipelineAbstract},
    sync::GpuFuture,
    command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState},
};

use nalgebra as na;
use na::{UnitQuaternion, Point3, Vector3};

use super::super::vk_init::VkRenderContext;


pub struct RenderPasses
{
    raster : Arc<dyn RenderPassAbstract + Send + Sync>,
}

pub struct Pipelines
{
    raster : Arc<dyn GraphicsPipelineAbstract + Send + Sync>,

    pub raster_set_layouts : Vec<Arc<UnsafeDescriptorSetLayout>>,
}

#[derive(Default, Debug, Clone)]
struct VertR { position: [f32; 3], color : [f32 ; 3] }
vulkano::impl_vertex!(VertR, position, color);

mod rvs {
    vulkano_shaders::shader!{
        ty:     "vertex",
        path:   "shaders/raster_vertex.glsl"
    }
}

mod rfs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "shaders/raster.glsl"
    }
}

impl Pipelines
{
    pub fn new(device : Arc<Device>, rps : &RenderPasses)
        -> Pipelines
    {
        let rfs = rfs::Shader::load(device.clone()).unwrap();
        let rvs = rvs::Shader::load(device.clone()).unwrap();

        let raster =
            Arc::new(GraphicsPipeline::start()
                .vertex_input_single_buffer::<VertR>()
                .vertex_shader(rvs.main_entry_point(), ())
                .viewports_dynamic_scissors_irrelevant(1)
                .depth_stencil_simple_depth()
                .depth_write(true)
                .fragment_shader(rfs.main_entry_point(), ())
                .render_pass(Subpass::from(rps.raster.clone(), 0).unwrap())
                .build(device.clone()).unwrap()
            );

        let raster_set_layouts = super::get_set_layouts(&raster.clone());

        Pipelines
        {
            raster,
            raster_set_layouts
        }
    }
}

impl RenderPasses
{
    pub fn new(logical_device : Arc<Device>)
        -> RenderPasses
    {
        let device = logical_device;


        let raster = 
            Arc::new(vulkano::single_pass_renderpass!(
                device.clone(),
                attachments: 
                {
                    color_output: 
                    {
                        load: Clear,
                        store: Store,
                        format: vulkano::format::Format::B10G11R11UfloatPack32,
                        samples: 1,
                    },
                    depth:
                    {
                        load: Clear,
                        store: Store,
                        format: vulkano::format::Format::D32Sfloat,
                        samples: 1,
                    }

                },
                pass:
                {
                    color: [color_output],
                    depth_stencil: {depth}
                }
            ).unwrap());

        RenderPasses {raster}
    }
}

struct FramebufferData
{
    pub raster : Arc<dyn FramebufferAbstract + Send + Sync>,
}

impl FramebufferData
{
    pub fn new(
        imgs : &super::ImgData,
        rps : &RenderPasses)
        -> FramebufferData
    {
        let raster = Arc::new(
            Framebuffer::start(rps.raster.clone())
            .add(imgs.raster_color.clone()).unwrap()
            .add(imgs.raster_depth.clone()).unwrap()
            .build().unwrap()
        );
        
        FramebufferData {raster}
    }
}

struct DescriptorSetData
{

}

impl DescriptorSetData
{
    pub fn new(
        _imgs : &super::ImgData,
        _pips : &Pipelines,
    )
    -> DescriptorSetData
    {
        DescriptorSetData {}
    }
}

pub struct EntityDrawer
{
    vk_ctx : Arc<VkRenderContext>,
    rps : RenderPasses,
    pub pipelines : Pipelines,
    dsets : DescriptorSetData,
    fbs : FramebufferData,
}

impl EntityDrawer
{
    pub fn new(vk_ctx : Arc<VkRenderContext>, imgs : &super::ImgData)
        -> EntityDrawer
    {
        let device = vk_ctx.logical_device.clone();
        let rps = RenderPasses::new(device.clone());
        let pipelines = Pipelines::new(device.clone(), &rps);
        let dsets = DescriptorSetData::new(&imgs, &pipelines);
        let fbs = FramebufferData::new(&imgs, &rps);

        EntityDrawer {vk_ctx, rps, pipelines, dsets, fbs}
    }


    pub fn cmd_buf_raster(
        &self,
        dynamic_state : &DynamicState,
        aspect_ratio : f32,
        dir : UnitQuaternion<f32>,
        pos : Point3<f32>,
        _model_place : Vector3<f32>
        )
        -> AutoCommandBuffer
    {

        let (tri_vertices, v_buf_future) = {
            vulkano::buffer::ImmutableBuffer::from_iter(
                vec![
                    VertR { position: [0.0, 0.0, 0.0], color: [1.0, 1.0, 1.0]},
                    VertR { position: [1.0, 0.0, 0.0], color: [0.0, 1.0, 0.0] },
                    VertR { position: [1.0, 1.0, 0.0], color: [1.0, 0.0, 0.0] },
                    VertR { position: [0.0, 1.0, 0.0], color: [0.0, 0.0, 1.0] },
                    VertR { position: [0.0, 0.0, 1.0], color: [0.0, 1.0, 0.0] },
                    VertR { position: [1.0, 0.0, 1.0], color: [0.0, 0.0, 1.0] },
                    VertR { position: [1.0, 1.0, 1.0], color: [0.1, 0.1, 0.1] },
                    VertR { position: [0.0, 1.0, 1.0], color: [0.0, 1.0, 0.0] },

                ].into_iter(),
                vulkano::buffer::BufferUsage::vertex_buffer(), self.vk_ctx.queue.clone()
            ).unwrap()
        };

        v_buf_future.then_signal_fence_and_flush().unwrap().wait(None).unwrap();





        let (index_vertices, i_buf_future) = {
            vulkano::buffer::ImmutableBuffer::from_iter(
                vec![
                    0u16, 1, 2, 2, 3, 0,
                    2, 3, 6, 6, 3, 7,
                    4, 5, 6, 6, 7, 4,
                    0, 3, 4, 4, 3, 7,
                    1, 2, 5, 5, 2, 6,
                    0, 1, 4, 4, 1, 5

                ].into_iter(),
                vulkano::buffer::BufferUsage::index_buffer(), self.vk_ctx.queue.clone()
            ).unwrap()
        };


        i_buf_future.then_signal_fence_and_flush().unwrap().wait(None).unwrap();


        // Converts diagonal fov to fovy
        let fovy = super::get_vertical_fov(150f32, aspect_ratio);
        let proj = nalgebra::Perspective3::new(aspect_ratio, fovy, super::NEAR, super::FAR);

        let f_axis = dir.axis().unwrap();
        let up = dir * f_axis.cross(&Vector3::y().cross(&f_axis));
        

        let view = nalgebra::Isometry3::look_at_rh(&pos, &(pos + f_axis.into_inner()), &-up);

        let model = nalgebra::Isometry3::new(na::zero(), na::zero());

        let pvm = proj.into_inner() * (view * model).to_homogeneous();


        let pc = rvs::ty::SpaceMatrices
        {
            pvm: pvm.into()
        };

        let mut acbb = 
            AutoCommandBufferBuilder::primary_one_time_submit(
                    self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.family()
                ).unwrap();

        acbb
        .begin_render_pass(
            self.fbs.raster.clone(), false,
            vec!([0.0, 0.0, 0.0, 0.0].into(), vulkano::format::ClearValue::Depth(1.0)),
        ).unwrap();
        acbb
        .draw_indexed(
            self.pipelines.raster.clone(),
            &dynamic_state,
            vec![tri_vertices.clone()],
            index_vertices,
            (), pc
        ).unwrap();
        acbb
        .end_render_pass().unwrap();

        acbb.build().unwrap()
    }

}

