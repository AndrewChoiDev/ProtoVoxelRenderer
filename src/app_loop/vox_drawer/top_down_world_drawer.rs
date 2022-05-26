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
};

use nalgebra as na;
use na::{UnitQuaternion, Point3, Vector3};

pub struct Pipelines
{
    ray_gen : Arc<dyn ComputePipelineAbstract + Send + Sync>,
    ray_traverse : Arc<dyn ComputePipelineAbstract + Send + Sync>,
    depth_to_length : Arc<dyn ComputePipelineAbstract + Send + Sync>,

    pub ray_gen_set_layouts : Vec<Arc<UnsafeDescriptorSetLayout>>,
    pub ray_traverse_set_layouts : Vec<Arc<UnsafeDescriptorSetLayout>>,
    pub depth_to_length_set_layouts : Vec<Arc<UnsafeDescriptorSetLayout>>,
}



mod ray_gen_cs {
    vulkano_shaders::shader!{
        ty:     "compute",
        path:   "shaders/ray_generation.glsl"
    }
}


pub mod ray_traverse_cs {
    vulkano_shaders::shader!{
        ty:     "compute",
        path:   "shaders/tree_traverse.glsl"
    }
}

mod depth_to_length_cs {
    vulkano_shaders::shader!{
        ty:     "compute",
        path:   "shaders/depth_to_length.glsl"
    } 
}

impl Pipelines
{
    pub fn new(logical_device : Arc<Device>)
        -> Pipelines
    {
        let device = logical_device;

        let ray_gen = 
            Arc::new(
                ComputePipeline::new(
                    device.clone(), 
                    &ray_gen_cs::Shader::load(device.clone()).unwrap().main_entry_point(), &()
                ).unwrap()
            );

        // let thing = ray_traverse_cs::Shader::ty::RayResult {};

        let ray_traverse = 
            Arc::new(
                ComputePipeline::new(
                    device.clone(),
                    &ray_traverse_cs::Shader::load(device.clone()).unwrap().main_entry_point(), &()
                ).unwrap()
            );

        let depth_to_length =
            Arc::new(
                ComputePipeline::new(
                    device.clone(),
                    &depth_to_length_cs::Shader::load(device.clone()).unwrap().main_entry_point(), &()
                ).unwrap()
            );

        let ray_gen_set_layouts = super::get_set_layouts(&ray_gen.clone());
        let ray_traverse_set_layouts = super::get_set_layouts(&ray_traverse.clone());
        let depth_to_length_set_layouts = super::get_set_layouts(&depth_to_length.clone());

        Pipelines 
        {
            ray_gen, ray_traverse, depth_to_length, 
            ray_gen_set_layouts, ray_traverse_set_layouts, depth_to_length_set_layouts
        }
    }
}

struct DescriptorSetData
{
    ray_gen_pool : FixedSizeDescriptorSetsPool,
    ray_traverse_pool : FixedSizeDescriptorSetsPool,
    depth_to_length_pool : FixedSizeDescriptorSetsPool,

    pub ray_gen_set : Arc<dyn DescriptorSet + Send + Sync>,
    pub ray_traverse_set : Arc<dyn DescriptorSet + Send + Sync>,
    pub depth_to_length_set : Arc<dyn DescriptorSet + Send + Sync>,
}

impl DescriptorSetData
{
    pub fn new(
        imgs : &super::ImgData,
        pips : &Pipelines,
    )
        -> DescriptorSetData
    {
        let mut ray_gen_pool = FixedSizeDescriptorSetsPool::new(pips.ray_gen_set_layouts[0].clone());
        let mut ray_traverse_pool = 
            FixedSizeDescriptorSetsPool::new(pips.ray_traverse_set_layouts[1].clone());
        let mut depth_to_length_pool = FixedSizeDescriptorSetsPool::new(pips.depth_to_length_set_layouts[0].clone());

        let ray_gen_set =   
            Arc::new(
                ray_gen_pool.next()
                .add_image(imgs.ray_directions.clone()).unwrap()
                .build().unwrap()
            );

        let ray_traverse_set =
            Arc::new(
                ray_traverse_pool.next()
                .add_image(imgs.ray_directions.clone()).unwrap()
                .add_image(imgs.world_color.clone()).unwrap()
                .add_image(imgs.world_depth.clone()).unwrap()
                .build().unwrap()
            );

        let depth_to_length_set = 
            Arc::new(
                depth_to_length_pool.next()
                .add_sampled_image(imgs.raster_depth.clone(), imgs.linear_sampler.clone()).unwrap()
                .add_image(imgs.world_depth.clone()).unwrap()
                .build().unwrap()
            );

        DescriptorSetData
        {
            ray_gen_pool, ray_traverse_pool, depth_to_length_pool,
            ray_gen_set, ray_traverse_set, depth_to_length_set
        }
    }
}

use super::super::vk_init::VkRenderContext;

pub struct WorldDrawer
{
    vk_ctx : Arc<VkRenderContext>,
    pub pipelines : Pipelines,
    dsets : DescriptorSetData,
}


use super::vox_map_context::VoxMapContext;

impl WorldDrawer
{
    pub fn new(vk_ctx : Arc<VkRenderContext>, imgs : &super::ImgData)
        -> WorldDrawer
    {
        let device = vk_ctx.logical_device.clone();
        let pipelines = Pipelines::new(device.clone());
        let dsets = DescriptorSetData::new(&imgs, &pipelines);
        WorldDrawer {vk_ctx, pipelines, dsets}
    }

    // Command buffer for converting the depth buffer into a buffer of ray lengths
    pub fn cmd_buf_depth_to_length(&self, 
        dynamic_state : &DynamicState,
        dir : UnitQuaternion<f32>,
        pos : na::Point3<f32>,
    )
        -> AutoCommandBuffer
    {
        let dynamic_state_dims = dynamic_state.viewports.as_ref().unwrap()[0].dimensions;
        let aspect_ratio =
            dynamic_state_dims[0] as f32 / dynamic_state_dims[1] as f32;

        let compute_dims = [(dynamic_state_dims[0] as u32 + 7) / 8, (dynamic_state_dims[1] as u32 + 7) / 8, 1];

        let mut acbb = 
            AutoCommandBufferBuilder::primary_one_time_submit(
                self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.family()
            ).unwrap();

        acbb
        .dispatch(
            compute_dims,
            self.pipelines.depth_to_length.clone(),
            self.dsets.depth_to_length_set.clone(),
            self.depth_to_length_pc(dir, pos, aspect_ratio)
        ).unwrap();

        acbb.build().unwrap()
    }

    pub fn cmd_buf_draw(
        &self,
        dynamic_state : &DynamicState,
        vox_map_ctx : &VoxMapContext,
        camera : super::CameraParameters,
        frame_index : i32)
        -> AutoCommandBuffer
    {

        let dynamic_state_dims = dynamic_state.viewports.as_ref().unwrap()[0].dimensions;
        let compute_dims = [(dynamic_state_dims[0] as u32 + 7) / 8, (dynamic_state_dims[1] as u32 + 7) / 8, 1];
        let aspect_ratio =
            dynamic_state_dims[0] as f32 / dynamic_state_dims[1] as f32;


        let mut acbb = 
            AutoCommandBufferBuilder::primary_one_time_submit(
                self.vk_ctx.logical_device.clone(), self.vk_ctx.queue.family()).unwrap();

        // Voxel ray trace
        acbb
        .dispatch(
            compute_dims,
            self.pipelines.ray_gen.clone(),
            self.dsets.ray_gen_set.clone(),
            self.ray_gen_pc(camera.orientation, aspect_ratio, frame_index)
        ).unwrap();
        acbb
        .dispatch(
            compute_dims,
            self.pipelines.ray_traverse.clone(),
            vec!(
                vox_map_ctx.tree_set.clone(),
                self.dsets.ray_traverse_set.clone(),
                vox_map_ctx.prefab_set.clone(), 
            ), 
            self.ray_traverse_pc(camera, vox_map_ctx.chunk_count())
        ).unwrap();
        
        acbb.build().unwrap()
    }


    fn ray_gen_pc(&self, dir : UnitQuaternion<f32>, aspect_ratio : f32, frame_index : i32)
        -> ray_gen_cs::ty::Orient
    {
        let cam_axes = get_camera_orientation(dir);

        let fov = 150.0f32;

        let f_basis =
            (cam_axes.2
            * (aspect_ratio).hypot(1.0))
            / (fov.to_radians() / 2.0).tan();
        
        ray_gen_cs::ty::Orient
        {
            hDir : (cam_axes.0).into(),
            vDir : (cam_axes.1).into(),
            fBasis : f_basis.into(),
            frameIndex : frame_index,
            _dummy0 : Default:: default(),
            _dummy1 : Default:: default(),
        }
    }

    fn depth_to_length_pc(&self, dir : UnitQuaternion<f32>,
        pos : Point3<f32>, aspect_ratio : f32)
        -> depth_to_length_cs::ty::PushConsts
    {
        let cam_axes = get_camera_orientation(dir);

        let fov = 150.0f32;


        let target = pos + cam_axes.2;

        // println!("{}", cam_axes.1);
        let pc = depth_to_length_cs::ty::PushConsts
        {
            pos: pos.coords.into(),
            // gridPos : pos.coords.into(),
            projInv : nalgebra::Perspective3::new(aspect_ratio, super::get_vertical_fov(fov, aspect_ratio), super::NEAR, super::FAR).inverse().into(),
            viewInv : nalgebra::Isometry3::look_at_rh(&pos, &target, &-cam_axes.1).inverse().to_homogeneous().into(),
            _dummy0 : Default:: default(),
        };

        pc
    }



    fn ray_traverse_pc(&self, camera : super::CameraParameters, chunk_count : u32)
        -> ray_traverse_cs::ty::PushConsts
    {
        // let thing = ray_traverse_cs::ty::RayResult {};
        // println!("{}", cam_axes.1);
        let pc = ray_traverse_cs::ty::PushConsts
        {
            pos: camera.chunk_position.coords.into(),
            chunkCount: chunk_count,
        };
        pc
    }
}

pub fn get_camera_orientation(
    quat : UnitQuaternion<f32>
    ) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>)
{
    let (unit_f_axis, roll_rad) = quat.axis_angle().unwrap();

    let h_axis =
    {
        let roll_quaternion = UnitQuaternion::from_axis_angle(&(unit_f_axis), roll_rad);
        (roll_quaternion * Vector3::y().cross(&unit_f_axis)).normalize()
    };
    let v_axis = unit_f_axis.cross(&h_axis);
    let f_axis = unit_f_axis.into_inner();

    (h_axis, v_axis, f_axis)
}