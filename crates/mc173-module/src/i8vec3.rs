use std::ops::Add;
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct StdbI8Vec2 {
    pub x: i8,
    pub y: i8,
}

// impl From<glam::IVec2> for StdbI8Vec2 {
//     fn from(value: glam::IVec2) -> Self {
//         StdbI8Vec2 {
//             x: value.x,
//             y: value.y,
//         }
//     }
// }
//
// impl From<StdbI8Vec2> for IVec2 {
//     fn from(value: StdbI8Vec2) -> Self {
//         IVec2 {
//             x: value.x,
//             y: value.y,
//         }
//     }
// }