use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::sync::GpuFuture;

pub fn run(renderer_data : std::sync::Arc<VkRenderContext::VkRenderContext::vk_renderer::VkRenderContext>)
{
    let (device, queue) = (renderer_data.logical_device.clone(), renderer_data.queue.clone());
    // Buffer values will be copied to dest_content
    let source_content = 0 .. 64;
    let source = CpuAccessibleBuffer::from_iter
        (device.clone(), BufferUsage::all(), false, source_content)
        .expect("failed to create buffer");

    // Buffer values will receive values from source_content
    let dest_content = (0 .. 64).map(|_| 0);
    let dest = CpuAccessibleBuffer::from_iter
        (device.clone(), BufferUsage::all(), false, dest_content)
        .expect("failed to create buffer");

    // Command buffer creation (copies values from one buffer to another)
    let command_buffer = AutoCommandBufferBuilder::new
        (device.clone(), queue.family()).unwrap()
        .copy_buffer(source.clone(), dest.clone()).unwrap()
        .build().unwrap();

    // Execute the command and waits until its finished...
    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    // Read and print a value from each buffer...
    let src_content = source.read().unwrap();
    let dest_content = dest.read().unwrap();

    // formal check for equilivance
    assert_eq!(&*src_content, &*dest_content);

    println!("Buffer copied!")
}