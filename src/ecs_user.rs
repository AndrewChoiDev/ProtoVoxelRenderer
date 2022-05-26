use nalgebra as na;
use na::{Point3, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChunkPositionComponent(pub Point3<f32>);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldGridCoordinateComponent(pub Point3<i32>);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityComponent(pub Vector3<f32>);

