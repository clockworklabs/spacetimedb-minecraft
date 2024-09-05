//! Pumpkin block metadata functions.

use crate::geom::Face;


/// Get the face where the pumpkin is carved.
#[inline]
pub fn get_face(metadata: u8) -> Face {
    match metadata & 3 {
        0 => Face::PosZ,
        1 => Face::NegX,
        2 => Face::NegZ,
        3 => Face::PosX,
        _ => unreachable!()
    }
}

/// Set the face where the pumpkin is carved.
#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata |= match face {
        Face::PosZ => 0,
        Face::NegX => 1,
        Face::NegZ => 2,
        Face::PosX => 3,
        _ => 0
    };
}
