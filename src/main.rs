#![allow(dead_code)]

mod audio_engine;
mod app_loop;
#[macro_use]
mod input;
mod move_engine;
mod world_engine;

mod ecs_user;

use std::sync::{Arc, RwLock};

use nalgebra as na;

fn main()
{
    let cam_dir = 
        Arc::new(RwLock::new(nalgebra::UnitQuaternion::from_axis_angle(
                    &na::Unit::new_unchecked(na::Vector3::z()), 0.0001)
        ));
    let input_data = Arc::new(input::InputState::new());

    app_loop::run(input_data.clone(), cam_dir.clone());
}
