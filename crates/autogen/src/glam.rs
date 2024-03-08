use glam::IVec3;
use crate::autogen::StdbIVec3;

impl From<StdbIVec3> for IVec3 {
    fn from(value: StdbIVec3) -> Self {
        IVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}