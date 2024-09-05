//! Stair block metadata functions.

use crate::geom::Face;


/// Get the face where the stair leads to.
#[inline]
pub fn get_face(metadata: u8) -> Face {
    match metadata & 3 {
        0 => Face::PosX,
        1 => Face::NegX,
        2 => Face::PosZ,
        3 => Face::NegZ,
        _ => unreachable!()
    }
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata = match face {
        Face::PosX => 0,
        Face::NegX => 1,
        Face::PosZ => 2,
        Face::NegZ => 3,
        _ => 0
    };
}
