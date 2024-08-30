use std::ops::Add;
use glam::IVec3;
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct StdbI16Vec3 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

// impl From<glam::IVec3> for StdbI16Vec3 {
//     fn from(value: glam::IVec3) -> Self {
//         StdbI16Vec3 {
//             x: value.x,
//             y: value.y,
//             z: value.z,
//         }
//     }
// }

// impl From<StdbI16Vec3> for IVec3 {
//     fn from(value: StdbI16Vec3) -> Self {
//         IVec3 {
//             x: value.x,
//             y: value.y,
//             z: value.z,
//         }
//     }
// }