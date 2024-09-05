use std::ops::Add;
use glam::{DVec3, IVec3};
use crate::autogen::StdbDVec3;

impl StdbDVec3 {
    pub fn as_dvec3(self) -> DVec3 {
        DVec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
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