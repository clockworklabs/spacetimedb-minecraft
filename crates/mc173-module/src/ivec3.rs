use std::ops::Add;
use glam::IVec3;
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy)]
pub struct StdbIVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl From<glam::IVec3> for StdbIVec3 {
    fn from(value: glam::IVec3) -> Self {
        StdbIVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<StdbIVec3> for IVec3 {
    fn from(value: StdbIVec3) -> Self {
        IVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}