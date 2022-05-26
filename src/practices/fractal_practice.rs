use vulkano::buffer::
    {BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::
    {AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::
    {PersistentDescriptorSet, UnsafeDescriptorSetLayout};
use vulkano::descriptor::descriptor::
    {DescriptorDesc, DescriptorDescTy, ShaderStages, 
    DescriptorImageDesc, DescriptorImageDescDimensions, DescriptorImageDescArray};
use vulkano::pipeline::
    {ComputePipeline};

use vulkano::format::{Format};
use vulkano::image::{Dimensions, StorageImage};
use image::{ImageBuffer, Rgba};

use vulkano::sync::GpuFuture;

use std::sync::Arc;

pub fn run(renderer_data : Arc<VkRenderContext::VkRenderContext::vk_renderer::VkRenderContext>)
{
    let (device, queue) = 
        (renderer_data.logical_device.clone(), 
        renderer_data.queue.clone());

    let image = 
        StorageImage::new(
            device.clone(), 
            Dimensions::Dim2d {width: 1024, height: 1024},
            Format::R8G8B8A8Unorm,
            Some(queue.family())
        )
        .unwrap();

    let shader = 
        cs::Shader::load(device.clone())
        .expect("Vulkan Error: Failed to create shader module");
    
    let compute_pipeline = 
        Arc::new(
            ComputePipeline::new(
                device.clone(), &shader.main_entry_point(), &()
            )
            .expect("Vulkan Error: Failed to create compute pipeline")
        );
    
    let descriptor_desc =
        Some(
        DescriptorDesc {
            ty: DescriptorDescTy::Image(
                DescriptorImageDesc
                {
                    sampled: false,
                    dimensions: DescriptorImageDescDimensions::TwoDimensional,
                    format: Some(Format::R8G8B8A8Unorm),
                    multisampled: false,
                    array_layers: DescriptorImageDescArray::NonArrayed
                }
            ),
            array_count: 1,
            stages: ShaderStages {
                compute: true, 
                ..ShaderStages::none()
            },
            readonly: false 
        }
        );

    let set =
        Arc::new(
            PersistentDescriptorSet::start(
                Arc::new(
                    UnsafeDescriptorSetLayout::new(
                        device.clone(),
                        Some(descriptor_desc)
                    )
                    .expect("Vulkan Error: Failed to create Descriptor Set")
                )
            )
            .add_image(image.clone())
            .unwrap()
            .build()
            .unwrap()
        );
    
    let buffer =
        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            (0 .. 1024 * 1024 * 4).map(|_| 0u8)
        )
        .expect("Vulkan Error: Failed to create buffer");
    
    let command_buffer =
        AutoCommandBufferBuilder::new(
            device.clone(), queue.family()
        )
        .unwrap()
        .dispatch(
            [1024 / 8, 1024 / 8, 1], 
            compute_pipeline.clone(),
            set.clone(),
            ()
        )
        .unwrap()
        .copy_image_to_buffer(
            image.clone(),
            buffer.clone()
        )
        .unwrap()
        .build()
        .unwrap();

    let finished = 
        command_buffer
        .execute(queue.clone())
        .unwrap();
    
    finished
    .then_signal_fence_and_flush()
    .unwrap()
    .wait(None)
    .unwrap();

    let buffer_content = buffer.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("fractal.png").unwrap();
}

mod cs 
{
    vulkano_shaders::shader!
    {
        ty:"compute",
        path:"shaders/mandelbrot.glsl"
    }
}