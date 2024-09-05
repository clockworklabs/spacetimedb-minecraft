//! Sign (post/wall) block metadata functions.

use crate::geom::Face;


/// Get the face where the stair leads to.
#[inline]
pub fn get_wall_face(metadata: u8) -> Option<Face> {
    Some(match metadata {
        5 => Face::PosX,
        4 => Face::NegX,
        3 => Face::PosZ,
        2 => Face::NegZ,
        _ => return None
    })
}

#[inline]
pub fn set_wall_face(metadata: &mut u8, face: Face) {
    *metadata = match face {
        Face::PosX => 5,
        Face::NegX => 4,
        Face::PosZ => 3,
        Face::NegZ => 2,
        _ => panic!("invalid wall face")
    };
}

/// Get the sign post yaw angle.
#[inline]
pub fn get_post_yaw(metadata: u8) -> f32 {
    (metadata as f32 - 0.5) / 16.0 * std::f32::consts::TAU
}

/// Set the sign post yaw angle, approximated due to metadata being in range 0..16.
#[inline]
pub fn set_post_yaw(metadata: &mut u8, yaw: f32) {
    *metadata = (yaw / std::f32::consts::TAU * 16.0 + 0.5) as i32 as u8 & 15;
}
