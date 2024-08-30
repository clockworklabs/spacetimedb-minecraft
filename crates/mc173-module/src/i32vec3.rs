use std::ops::Add;
use glam::IVec3;
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct StdbI32Vec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl From<glam::IVec3> for StdbI32Vec3 {
    fn from(value: glam::IVec3) -> Self {
        StdbI32Vec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<StdbI32Vec3> for IVec3 {
    fn from(value: StdbI32Vec3) -> Self {
        IVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}