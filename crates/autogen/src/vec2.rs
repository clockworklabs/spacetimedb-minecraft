use std::ops::Add;
use glam::{IVec3, Vec2};
use crate::autogen::StdbVec2;

impl From<glam::Vec2> for StdbVec2 {
    fn from(value: glam::Vec2) -> Self {
        StdbVec2 {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<StdbVec2> for Vec2 {
    fn from(value: StdbVec2) -> Self {
        Vec2 {
            x: value.x,
            y: value.y,
        }
    }
}