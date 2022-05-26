use winit::window::
    {Window};
use winit::event::
    {Event, WindowEvent};
use winit::event_loop::
    {ControlFlow};
use std::thread;
use std::sync::{Arc, RwLock, Mutex};
use std::rc::{Rc};
use vulkano::device::{Device, Queue};
use vulkano::descriptor::pipeline_layout::{PipelineLayout, RuntimePipelineDesc};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, BufferAccess};
use vulkano::descriptor::descriptor::{DescriptorDesc, DescriptorImageDesc, DescriptorDescTy, DescriptorImageDescDimensions, DescriptorImageDescArray, ShaderStages};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, UnsafeDescriptorSetLayout, PersistentDescriptorSetImg};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, CommandBuffer};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::{SwapchainImage, StorageImage, Dimensions, ImmutableImage, ImageUsage, ImageLayout, MipmapsCount};
use vulkano::pipeline::{GraphicsPipeline, ComputePipeline};
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{AcquireError, SwapchainCreationError};
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::sync;
use vulkano::format::{Format};
use nalgebra::{Vector3, Point3, Matrix3, UnitQuaternion, Unit};
use std::time;
use cpal::traits::{DeviceTrait, HostTrait, StreamIdTrait, EventLoopTrait};
use cpal::{StreamData, UnknownTypeOutputBuffer};
// use std::f32;
#[derive(Default, Debug, Clone)]
struct Vertex { position: [f32; 2] }

pub fn run(render_ctx, window_ctx, )
{
    // renderer_data.surface.window().set_fullscreen(Some(winit::window::Fullscreen::Exclusive(renderer_data.monitor_handle.video_modes().next().unwrap())));
    let (device, mut swapchain, images, surface, queue) = 
        (renderer_data.logical_device.clone(), 
        renderer_data.swapchain.clone(),
        renderer_data.images.clone(),
        renderer_data.surface.clone(),
        renderer_data.queue.clone());
    let position = Arc::new(RwLock::new(Point3::new(0.0_f32, 2.0_f32, 0.0_f32)));
    let position_a = position.clone();
    let position_b = position.clone();
    let start_instant = time::Instant::now();

    thread::spawn(move || {

        let host = cpal::default_host();
        let event_loop = host.event_loop();
        let device = 
            host.default_output_device()
            .expect("Cpal Error: Could not find a default output device");
        let mut supported_formats_range = 
            device.supported_output_formats()
            .expect("Cpal Error: Could not query formats");
        let format = 
            supported_formats_range.next()
            .expect("Cpal Error: Could not find a supported format")
            .with_max_sample_rate();

        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

        event_loop.play_stream(stream_id).expect("Cpal Error: Could not play stream");

        event_loop.run(move |stream_id, stream_result| {
            let stream_data = match stream_result {
                Ok(data) => data,
                Err(err) => {
                    eprintln!("Cpal Error: occured on stream {:?} - {}", stream_id, err);
                    return;
                }
            };
            let sample_rate = format.sample_rate.0 as f32;
            let mut sample_clock = 0f32;
            let mut next_value = move || {
                sample_clock = (sample_clock + 1.0) % sample_rate;
                (sample_clock * 140.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
            };
            println!("Helo");

            match stream_data {
                StreamData::Output { buffer: UnknownTypeOutputBuffer::U16(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        *elem = u16::max_value() / 2;
                    }
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        *elem = 0;
                    }
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        let value = next_value();
                        *elem = value / 15.0;
                    }
                },
                _ => ()
            }
        });

        // run_audio(&device, &format);
    });


    let pos_child = 
    thread::spawn(move || {
        let mut now = time::Instant::now();
        loop
        {
            let dt = now.elapsed().as_secs_f32();
            now = time::Instant::now();
            // Write to the rw-typed position
            {
                let mut pos_w = position_a.write().unwrap();
                let sin_scalar = start_instant.elapsed().as_secs_f32().sin();
                let cos_scalar = start_instant.elapsed().as_secs_f32().cos();
                
                *pos_w = Point3::new(10.0 * cos_scalar, 7.0 * cos_scalar, 7.0 * sin_scalar) + Vector3::new(0.0001, 4.0001, 0.0001);
            }
            // println!("dt (ms): {}", dt * 1000.0);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });


    // let render_child =
    // thread::spawn(move || {

        let vertex_buffer = {
            vulkano::impl_vertex!(Vertex, position);

            CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, [
                Vertex { position: [-1.0, -1.0] },
                Vertex { position: [-1.0, 1.0] },
                Vertex { position: [1.0, -1.0] },
                Vertex { position: [1.0, 1.0]}
            ].iter().cloned()).unwrap()
        };


        let vs = vs::Shader::load(device.clone()).unwrap();
        let fs = fs::Shader::load(device.clone()).unwrap();

        let render_pass = Arc::new(vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                // `color` is a custom name we give to the first and only attachment.
                color: {
                    // `load: Clear` means that we ask the GPU to clear the content of this
                    // attachment at the start of the drawing.
                    load: Clear,
                    // `store: Store` means that we ask the GPU to store the output of the draw
                    // in the actual image. We could also ask it to discard the result.
                    store: Store,
                    // `format: <ty>` indicates the type of the format of the image. This has to
                    // be one of the types of the `vulkano::format` module (or alternatively one
                    // of your structs that implements the `FormatDesc` trait). Here we use the
                    // same format as the swapchain.
                    format: swapchain.format(),
                    // TODO:
                    samples: 1,
                }
            },
            pass: {
                // We use the attachment named `color` as the one and only color attachment.
                color: [color],
                // No depth-stencil attachment is indicated with empty brackets.
                depth_stencil: {}
            }
        ).unwrap());



        let (vox_img, vox_set) = get_voxel_map(device.clone(), queue.clone());


        let pipeline = Arc::new(GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vertex>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_strip()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone()).unwrap());



        let mut dynamic_state = DynamicState {..DynamicState::none()};

        let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

        let mut render_now = std::time::Instant::now();

    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);
    renderer_data.event_loop.run(
        move |event, _, control_flow|
        {
            *control_flow = ControlFlow::Poll;

            match event
            {
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        input: winit::event::KeyboardInput {
                            state: winit::event::ElementState::Pressed,
                            virtual_keycode: Some(virtual_code),
                            ..
                        },
                        ..
                    }, 
                    ..
                } => 
                {
                    println!("{:?}", virtual_code);
                },
                Event::WindowEvent {event: WindowEvent::CloseRequested, ..}
                    => 
                {
                    *control_flow = ControlFlow::Exit;
                },
                Event::WindowEvent {event: WindowEvent::Resized(_), ..} => 
                {
                    let dimensions : [u32; 2] = surface.window().inner_size().into();

                    let (new_swapchain, new_images) = match swapchain.recreate_with_dimensions(dimensions) {
                        Ok(r) => r,
                        Err(err) => panic!("{:?}", err)
                    };

                    swapchain = new_swapchain;

                    framebuffers = window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);
                },
                Event::DeviceEvent {event, ..} =>
                {
                    match event
                    {
                        winit::event::DeviceEvent::MouseMotion { delta } =>
                        {
                            println!("Mouse moved: {:?}", delta);
                        },
                        _ => ()
                    }
                },
                Event::RedrawEventsCleared => 
                {
                    // println!("frame timing (ms): {}", render_now.elapsed().as_secs_f32() * 1000.0);
                    render_now = std::time::Instant::now();
                    previous_frame_end.as_mut().unwrap().cleanup_finished();

                    render_now = time::Instant::now();

                    let (img_num, _, acquire_future) =
                        match vulkano::swapchain::acquire_next_image(swapchain.clone(), None) 
                        {
                            Ok(r) => r,
                            Err(err) => panic!("{:?}", err)
                        };

                    let clear_values = vec!([0.0, 0.0, 1.0, 1.0].into());

                    let position = position_b.read().unwrap();
                    let cam_orientation = 
                        get_camera_orientation(
                            *position, Point3::new(0.0, 0.0, 0.0), 0.0
                        );
                    let a : [[f32 ; 3] ; 3] = cam_orientation.into();

                    let dimensions = swapchain.dimensions();
                    let aspect_ratio = dimensions[1] as f32 / dimensions[0] as f32;

                    let pc = fs::ty::PushConsts
                    {
                        pos : (*position).coords.into(),
                        hDir : a[0],
                        vDir : a[1],
                        fDir : a[2],
                        aspectRatio : aspect_ratio,
                        fov : 90.0,
                        _dummy0 : Default:: default(),
                        _dummy1 : Default:: default(),
                        _dummy2 : Default::default()
                    };

                    let command_buffer = 
                        AutoCommandBufferBuilder::primary_one_time_submit(
                            device.clone(), queue.family()
                        ).unwrap()
                        .begin_render_pass(
                            framebuffers[img_num].clone(), 
                            false, 
                            clear_values
                        ).unwrap()
                        .draw(
                            pipeline.clone(),
                            &dynamic_state,
                            vertex_buffer.clone(), 
                            vox_set.clone(),
                            pc
                        ).unwrap()
                        .end_render_pass().unwrap()
                        .build().unwrap();

                    
                    let future = 
                        previous_frame_end.take().unwrap()
                        .join(acquire_future)
                        .then_execute(queue.clone(), command_buffer).unwrap()
                        .then_swapchain_present(queue.clone(), swapchain.clone(), img_num)
                        .then_signal_fence_and_flush();
                    
                    match future
                    {
                        Ok(future) => {
                            previous_frame_end = Some(Box::new(future) as Box<_>);
                        },
                        Err(e) => {
                            println!("{:?}", e);
                            previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                        }
                    }
                }
                _ => ()
            }
        }
    );
}

// fn run_audio(device : &cpal::Device, format : &cpal::Format)
// {
//     let sample_rate = format.sample_rate.0 as f32;
//     let channels = format.channels as usize;

//     let mut sample_clock = 0f32;
//     let mut next_value = move || {
//         sample_clock = (sample_clock + 1.0) % sample_rate;
//         (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
//     };

//     let err_fn = |err| eprintln!("Cpal Error: occured on stream - {}", err);

//     let stream = device.build_output_stream(
//         format,
//         move |data: &mut [u32]| write_data(data, channels, &mut next_value),
//         err_fn
//     ).unwrap();
// }

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState
    ) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> 
{
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0 .. 1.0,
    };
    dynamic_state.viewports = Some(vec!(viewport));


    images.iter().map(|image| {
        Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        ) as Arc<dyn FramebufferAbstract + Send + Sync>
    }).collect::<Vec<_>>()
}

fn get_voxel_map(device : Arc<Device>, queue : Arc<Queue>)
    -> (Arc<ImmutableImage<Format>>, Arc<PersistentDescriptorSet<((), PersistentDescriptorSetImg<Arc<ImmutableImage<Format>>>)>>)
{
    const VOX_SIZE : u32 = 64;

    let proto_voxel_img = 
        StorageImage::new(
            device.clone(), 
            Dimensions::Dim3d {width: VOX_SIZE, height: VOX_SIZE, depth: VOX_SIZE}, 
            // Dimensions::Cubemap {size: VOX_SIZE},
            Format::R8Uint, 
            Some(queue.family())
        ).unwrap();

    let (voxel_img, vox_img_init) =
        ImmutableImage::uninitialized(
            device.clone(), 
            Dimensions::Dim3d {width: VOX_SIZE, height: VOX_SIZE, depth: VOX_SIZE}, 
            Format::R8Uint, 
            MipmapsCount::Log2, 
            ImageUsage {storage: true, transfer_destination: true, ..ImageUsage::none()}, 
            ImageLayout::General, 
            Some(queue.family())
        ).unwrap();
        
    
    let shader = cs::Shader::load(device.clone()).expect("failed to create shader module");
    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
    .expect("failed to create compute pipeline"));
    
    let descriptor_desc =
    Some(
    DescriptorDesc {
        ty: DescriptorDescTy::Image(
            DescriptorImageDesc
            {
                sampled: false,
                dimensions: DescriptorImageDescDimensions::ThreeDimensional,
                format: Some(Format::R8Uint),
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
                    ).expect("Vulkan Error: Failed to create Descriptor Set")
                )
            )
            .add_image(proto_voxel_img.clone())
            .unwrap()
            .build()
            .unwrap()
        );


    let command_buffer = 
        AutoCommandBufferBuilder::new(
            device.clone(), queue.family()
        ).unwrap()
        .dispatch(
            [VOX_SIZE / 8, VOX_SIZE / 8, VOX_SIZE / 8], 
            compute_pipeline.clone(), 
            set.clone(), 
            ()
        ).unwrap()
        .copy_image(
            proto_voxel_img.clone(), [0, 0, 0], 0, 0, 
            vox_img_init, [0, 0, 0], 0, 0, 
            [VOX_SIZE, VOX_SIZE, VOX_SIZE], 1
        ).unwrap()
        .build().unwrap();
    let finished = 
        command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    let vox_descriptor_desc =
        Some(
        DescriptorDesc {
            ty: DescriptorDescTy::Image(
                DescriptorImageDesc
                {
                    sampled: false,
                    dimensions: DescriptorImageDescDimensions::ThreeDimensional,
                    format: Some(Format::R8Uint),
                    multisampled: false,
                    array_layers: DescriptorImageDescArray::NonArrayed
                }
            ),
            array_count: 1,
            stages: ShaderStages {
                // vertex: true,
                fragment: true, 
                ..ShaderStages::none()
            },
            readonly: true 
        }
        );

    let vox_set =
        Arc::new(
            PersistentDescriptorSet::start(
                Arc::new(
                    UnsafeDescriptorSetLayout::new(
                        device.clone(),
                        Some(vox_descriptor_desc)
                    ).expect("Vulkan Error: Failed to create Descriptor Set")
                )
            )
            .add_image(voxel_img.clone()).unwrap()
            .build()
            .unwrap()
        );


    (voxel_img, vox_set)
}


// Returns a unit matrix with three perpendicular column vectors
// Axis order: (horizontal, vertical, forward)
fn get_camera_orientation(
    position        : Point3<f32>,
    focus           : Point3<f32>,
    roll_angle      : f32,
    ) -> Matrix3<f32>
{
    let f_axis = Unit::new_normalize(focus - position);
    let h_axis = 
    {
        let roll_quaternion = UnitQuaternion::from_axis_angle(&(f_axis), roll_angle.to_radians());
        roll_quaternion * Vector3::y().cross(&f_axis)
    };
    let v_axis = f_axis.cross(&h_axis);

    Matrix3::from_columns(&[h_axis, v_axis, f_axis.into_inner()])
}

mod vs {
    vulkano_shaders::shader!{
        ty:     "vertex",
        path:   "shaders/vertex_one.glsl"
    }
}

mod fs {
    vulkano_shaders::shader!{
        ty:     "fragment",
        path:   "shaders/fragment_red.glsl"
    }
}

mod cs {
    vulkano_shaders::shader!{
        ty:     "compute",
        path:   "shaders/voxel_sphere.glsl"
    }
}