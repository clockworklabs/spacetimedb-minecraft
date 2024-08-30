use glam::IVec3;
use crate::autogen::StdbI32Vec3;

impl From<StdbI32Vec3> for IVec3 {
    fn from(value: StdbI32Vec3) -> Self {
        IVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}