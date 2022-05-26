use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, UnsafeDescriptorSetLayout};
use vulkano::descriptor::descriptor::{DescriptorDesc, DescriptorDescTy, ShaderStages, DescriptorBufferDesc};
use vulkano::pipeline::{ComputePipeline};
use vulkano::sync::GpuFuture;

use std::sync::Arc;

pub fn run(renderer_data : Arc<VkRenderContext::VkRenderContext::vk_renderer::VkRenderContext>)
{
    let (device, queue) = (renderer_data.logical_device.clone(), renderer_data.queue.clone());

    let data_iter = 0 .. 65536;
    let data_buffer = 
        CpuAccessibleBuffer::from_iter(
            device.clone(), BufferUsage::all(), false, data_iter
        )
        .expect("failed to create buffer");

    let shader = 
        cs::Shader::load(device.clone())
        .expect("failed to create shader module");

    let compute_pipeline = 
        Arc::new(
            ComputePipeline::new(
                device.clone(), &shader.main_entry_point(), &()
            )
            .expect("failed to create compute pipeline")
        );

    let descriptor_desc = 
        Some(
        DescriptorDesc {
            ty: DescriptorDescTy::Buffer(DescriptorBufferDesc {dynamic: Some(false), storage: true}), 
            array_count: 1, 
            stages: ShaderStages {
                vertex: false, 
                tessellation_control: false, 
                tessellation_evaluation: false, 
                geometry: false, 
                fragment: false, 
                compute: true
            }, 
            readonly: false
        }
        );


    let unsafe_descriptor_set_layout =
        Arc::new(
            UnsafeDescriptorSetLayout::new(device.clone(), Some(descriptor_desc))
            .expect("Descriptor Set failed to create")
        );

    let set = 
        Arc::new(
            PersistentDescriptorSet::start(
                unsafe_descriptor_set_layout.clone()
            )
            .add_buffer(data_buffer.clone())
            .unwrap()
            .build()
            .unwrap()
        );
    
    let command_buffer = 
        AutoCommandBufferBuilder::new(
            device.clone(), queue.family()
        )
        .unwrap()
        .dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ())
        .unwrap()
        .build().unwrap();

    let finished = 
        command_buffer
        .execute(queue.clone())
        .unwrap();
    
    finished
    .then_signal_fence_and_flush()
    .unwrap()
    .wait(None)
    .unwrap();

    let content = 
        data_buffer
        .read()
        .unwrap();

    for (n, val) in content.iter().enumerate() 
    {
        assert_eq!(*val, n as u32 * 12);
    }
}


mod cs 
{
    vulkano_shaders::shader!
    {
        ty:"compute",
        path:"shaders/practice_copy.glsl"
    }
}