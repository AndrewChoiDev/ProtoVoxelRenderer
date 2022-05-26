use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent, DeviceEvent};

use std::sync::{Arc, RwLock};

mod vk_init;
pub mod vox_drawer;
use vk_init::vk_renderer;

use nalgebra as na;
use na::{Vector3, Point3, UnitQuaternion, Unit};
use super::input::{InputState, PressKey};
// use super::world_engine::chunk::Chunk;
use super::world_engine::map::Map;

use super::ecs_user::{
    ChunkPositionComponent, 
    WorldGridCoordinateComponent,
    VelocityComponent};


pub fn run(
    input_state : Arc<InputState>,
    rw_cam_dir : Arc<RwLock<UnitQuaternion<f32>>>)
{
    let event_loop = EventLoop::new();
    let (render_ctx, win_ctx) = vk_renderer::vk_ctx_init(get_vk_app_info(), &event_loop);

    let mut map = Map::new([0 ; 3].into(), 2);

    let mut vox_drawer = vox_drawer::VoxDrawer::new(render_ctx.clone(), win_ctx.swapchain.format(), win_ctx.dims(), &map);

    let mut render_now = std::time::Instant::now();

    // let start_instant = std::time::Instant::now();

    let universe = legion::world::Universe::new();

    let mut world = universe.create_world();
    

    let initial_entities =
        world.insert(
            (), 
            (0..20).map(|_| (
                ChunkPositionComponent {0: Point3::origin()}, 
                WorldGridCoordinateComponent {0: Point3::origin()},
                VelocityComponent {0: na::zero()},
            ))
        );

    let player_entity = initial_entities[0].clone();

    (*world.get_component_mut::<ChunkPositionComponent>(player_entity).unwrap()).0 
        += Vector3::new(0.1, 0.1, 0.1) * 16.0;

    // let mut updates = 0;

    event_loop.run( move |event, _, control_flow|
    {
        *control_flow = ControlFlow::Poll;


        match event
        {
            Event::WindowEvent {event: WindowEvent::CloseRequested, ..} =>
            {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent {event: WindowEvent::Resized(_), ..} =>
            {
                // win_ctx.update_swapchain(
                //     win_ctx.surface.window().inner_size().into(),
                // );
                // win_ctx.update_dynamic_state_with_scale(res_scale);

                // let new_dims = win_ctx.scaled_dims(res_scale);

                // vox_drawer.update_dims(new_dims);
            },
            Event::DeviceEvent {event: DeviceEvent::Key(val), ..} =>
            {
                let map_queue = map.input_queue();

                input_state.update_pressed(&PressKey::KeyScancode(val.scancode as usize),
                    val.state == winit::event::ElementState::Pressed, map_queue);
            },
            Event::DeviceEvent {event: DeviceEvent::Button {button, state}, ..} =>
            {
                let map_queue = map.input_queue();

                input_state.update_pressed(&PressKey::MouseButton(button as usize),
                        state == winit::event::ElementState::Pressed, map_queue);
            },
            Event::DeviceEvent {event : DeviceEvent::MouseMotion {delta}, ..} =>
            {
                const SENSE : f32 = 0.003;
                {rotate_cam_dir(delta.0 as f32 * SENSE, delta.1 as f32 * SENSE, &mut *rw_cam_dir.write().unwrap());}
            },
            Event::MainEventsCleared =>
            {
                win_ctx.surface.window().request_redraw();
            }
            Event::RedrawRequested(_) =>
            {
                use legion::query::{Write, IntoQuery};

                win_ctx.surface.window()
                .set_cursor_position(winit::dpi::PhysicalPosition{x: 200, y: 200}).unwrap();

                let dt = render_now.elapsed().as_secs_f32();

                render_now = std::time::Instant::now();

                let dir = {*rw_cam_dir.read().unwrap()};

                {
                    let mut player_velocity = world.get_component_mut::<VelocityComponent>(player_entity).unwrap();

                    let player_acceleration = 
                        super::move_engine::acceleration(
                            (*player_velocity).0.clone(), 
                            input_state.clone(), &dir);

                    (*player_velocity).0 += player_acceleration * dt;
                }

                let chunk_dims_f32 = Vector3::from_iterator(map.chunk_dims().iter().map(|c| *c as f32));


                let step_query = <(Write<WorldGridCoordinateComponent>, Write<ChunkPositionComponent>, Write<VelocityComponent>)>::query();
                for (mut world_grid_coord, mut chunk_pos, mut velocity) in step_query.iter(&mut world)
                {
                    // let acceleration = -1.2 * (velocity.0.magnitude() + 1.8) * velocity.0;


                    velocity.0 *= 0.85;
                    chunk_pos.0 += velocity.0 * dt;

                    world_grid_coord.0 +=
                        chunk_pos.0.coords.component_div(&chunk_dims_f32)
                        .map(|c| c.floor() as i32);
                    
                    chunk_pos.0.iter_mut().enumerate()
                        .for_each(|(i, c)| *c = c.rem_euclid(chunk_dims_f32[i]));
                }
                


                // super::move_engine::run(&mut player_pos, input_state.clone(), &dir, dt);
                let player_chunk_pos = (*world.get_component::<ChunkPositionComponent>(player_entity).unwrap()).0.clone();
                let player_world_grid_coords = (*world.get_component::<WorldGridCoordinateComponent>(player_entity).unwrap()).0.clone();
                map.handle_events(*dir.axis().unwrap(), player_world_grid_coords, player_chunk_pos);
                map.adapt_to_world_position((*world.get_component::<WorldGridCoordinateComponent>(player_entity).unwrap()).0.clone().coords.into());
                map.generate_next_chunk();

                let camera_parameters =
                {
                    vox_drawer::CameraParameters
                    {
                        chunk_position: player_chunk_pos,
                        world_grid_coords: player_world_grid_coords,
                        orientation: dir,
                    }
                };
            
                let render_start_instance = std::time::Instant::now();
                vox_drawer.render_frame(&win_ctx, camera_parameters, &mut map);
                println!("ms: {}", render_start_instance.elapsed().as_secs_f32() * 1000.0);

            },
            _ => ()
        }
    });
}




fn get_vk_app_info<'a>() -> vulkano::instance::ApplicationInfo<'a>
{
    vulkano::instance::ApplicationInfo
    {
        application_name:
            Some("First Engine".into()),
        application_version:
            Some(vulkano::instance::Version
            {major: 0, minor: 1, patch: 0}),
        engine_name:
            Some("First Engine".into()),
        engine_version:
            Some(vulkano::instance::Version
            {major: 0, minor: 1, patch: 0}),
    }
}


fn rotate_cam_dir(azim_rad : f32, polar_rad : f32, cam_dir : &mut UnitQuaternion<f32>)
{
    // let polar_edge = 0.0
    // let mut cam_dir = rw_cam_dir.write().unwrap();
    let (cam_dir_axis, cam_dir_angle) = cam_dir.axis_angle().unwrap();
    let axis_quat =
        *cam_dir
        *
        UnitQuaternion::from_axis_angle(
            &Unit::new_normalize(Vector3::new(
                cam_dir_axis.z, 0.0, -cam_dir_axis.x)),
            polar_rad)
        * UnitQuaternion::from_euler_angles(0.0, azim_rad, 0.0);

    *cam_dir =
         UnitQuaternion::from_axis_angle(&(axis_quat * cam_dir_axis), cam_dir_angle);
}
