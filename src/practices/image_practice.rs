use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::format::{Format, ClearValue};
use vulkano::image::{Dimensions, StorageImage};
use vulkano::sync::GpuFuture;

use image::{ImageBuffer, Rgba};


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
        .clear_color_image(image.clone(), ClearValue::Float([0.0, 0.0, 1.0, 1.0]))
        .unwrap()
        .copy_image_to_buffer(image.clone(), buffer.clone())
        .unwrap()
        .build()
        .unwrap();

    let finished = 
        command_buffer.execute(queue.clone())
        .unwrap();
    
    finished
    .then_signal_fence_and_flush()
    .unwrap()
    .wait(None)
    .unwrap();

    let buffer_content = buffer.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("blue.png").unwrap();
}