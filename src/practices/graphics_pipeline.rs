use vulkano::buffer::
    {BufferUsage, CpuAccessibleBuffer};
use vulkano::framebuffer::
    {Framebuffer, Subpass};
use vulkano::command_buffer::
    {AutoCommandBufferBuilder, CommandBuffer};
use vulkano::pipeline::
    {GraphicsPipeline};
use vulkano::command_buffer::DynamicState;
use vulkano::pipeline::viewport::Viewport;

use vulkano::format::{Format};
use vulkano::image::{Dimensions, StorageImage};
use vulkano::sync::GpuFuture;

use image::{ImageBuffer, Rgba};
// use std::convert::Into;
// use vulkano::sync::GpuFuture;

use std::sync::Arc;

#[derive(Default, Copy, Clone)]
struct Vertex
{
    position : [f32; 2]
}


pub fn run(renderer_data : Arc<VkRenderContext::VkRenderContext::vk_renderer::VkRenderContext>)
{
    let (device, queue) = 
        (renderer_data.logical_device.clone(), 
        renderer_data.queue.clone());

    vulkano::impl_vertex!(Vertex, position);

    let vertex1 = Vertex { position: [-0.5, -0.5] };
    let vertex2 = Vertex { position: [ 0.5,  0.5] };
    let vertex3 = Vertex { position: [ 0.5, -0.25] };

    let vertex_buffer = 
        CpuAccessibleBuffer::from_iter(
            device.clone(), 
            BufferUsage::all(),
            false,
            vec![vertex1, vertex2, vertex3]
            .into_iter()
        ).unwrap();

    let image = 
        StorageImage::new(
            device.clone(), 
            Dimensions::Dim2d {width: 1024, height: 1024},
            Format::R8G8B8A8Unorm,
            Some(queue.family())
        ).unwrap();
    
    let buffer = 
        CpuAccessibleBuffer::from_iter(
            device.clone(), 
            BufferUsage::all(), 
            false, 
            (0 .. 1024 * 1024 * 4)
            .map(|_| 0u8)
        )
        .expect("Vulkan Error: Failed to create buffer");

    let render_pass = 
        Arc::new(
            vulkano::single_pass_renderpass!(
                device.clone(),
                attachments: 
                {
                    color: 
                    {
                        load: Clear,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    }
                },
                pass: 
                {
                    color: [color],
                    depth_stencil: {}
                }
            ).unwrap()
        );

    let framebuffer = 
        Arc::new(
            Framebuffer::start(render_pass.clone())
            .add(image.clone()).unwrap()
            .build().unwrap()
        );


    let vertex_shader = 
        v_shader::Shader::load(device.clone())
        .expect("Vulkan Error: Failed to create shader module");

    let fragment_shader = 
        f_shader::Shader::load(device.clone())
        .expect("Vulkan Error: Failed to create shader module");

    let pipeline =
        Arc::new(
            GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone()).unwrap()
        );

    let dynamic_state = 
        DynamicState {
            viewports: 
                Some(vec![
                    Viewport {
                        origin: [0.0, 0.0],
                        dimensions: [1024.0, 1024.0],
                        depth_range: 0.0 .. 1.0
                    }
                ]),
            ..DynamicState::none()
        };
    
    let command_buffer = 
        AutoCommandBufferBuilder::primary_one_time_submit(
            device.clone(),
            queue.family()
        ).unwrap()
        .begin_render_pass(
            framebuffer.clone(), 
            false, 
            vec![[0.0, 0.0, 0.0, 1.0].into()]
        ).unwrap()
        .draw(
            pipeline.clone(), 
            &dynamic_state, 
            vertex_buffer.clone(), (), ()
        )
            .unwrap()
        .end_render_pass().unwrap()
        .copy_image_to_buffer(image.clone(), buffer.clone()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    
    finished
    .then_signal_fence_and_flush().unwrap()
    .wait(None).unwrap();

    let buffer_content = buffer.read().unwrap();

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        1024, 
        1024, 
        &buffer_content[..]
    ).unwrap();

    image.save("triangle.png").unwrap();
}

mod v_shader 
{
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "shaders/vertex_one.glsl"
    }
}

mod f_shader 
{
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "shaders/fragment_red.glsl"
    }
}