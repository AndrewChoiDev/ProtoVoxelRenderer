use std::sync::{Arc};
use super::input;
use nalgebra as na;
use nalgebra::{UnitQuaternion, Vector3};

pub fn acceleration(
    _velocity: Vector3<f32>, 
    input: Arc<input::InputState>,
    dir: &UnitQuaternion<f32>,
    ) -> na::Vector3<f32>
{



    let get_press = |key| input.pressed(key);
    let get_input_axes = 
        |forward, backward| 
        match forward
        {
            in_f if in_f == backward => 0.0,
            true => 1.0,
            false => -1.0
        };
    let dir_axis = dir.axis().unwrap();
    let forward_axis = na::Vector3::new(dir_axis.x, 0.0, dir_axis.z).normalize();
    let side_axis = na::Vector3::y().cross(&forward_axis);
    let input_dir = 
        nalgebra::Vector3::new(
            get_input_axes(get_press("right"), get_press("left")),
            get_input_axes(get_press("high"), get_press("low")),
            get_input_axes(get_press("forward"), get_press("backward"))
        )
        .try_normalize(0.0)
        .unwrap_or(nalgebra::zero());
    
    const ACCELERATION_MAG : f32 = 40.0;

    (side_axis * input_dir.x + nalgebra::Vector3::y() * input_dir.y + forward_axis * input_dir.z) 
    * ACCELERATION_MAG
}




