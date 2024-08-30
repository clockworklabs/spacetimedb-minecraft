use std::ops::Add;
use glam::{DVec3, IVec3};
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct StdbDVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl StdbDVec3 {
    pub fn as_dvec3(&self) -> DVec3 {
        DVec3::new(self.x, self.y, self.z)
    }
}

impl From<DVec3> for StdbDVec3 {
    fn from(value: DVec3) -> Self {
        StdbDVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<StdbDVec3> for DVec3 {
    fn from(value: StdbDVec3) -> Self {
        DVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}