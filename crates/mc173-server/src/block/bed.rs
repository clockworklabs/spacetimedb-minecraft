//! Bed special functions for metadata.

use crate::geom::Face;


/// Get the facing of a bed.
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

/// Set the facing of a bed.
#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata |= match face {
        Face::PosZ => 0,
        Face::NegX => 1,
        Face::NegZ => 2,
        Face::PosX => 3,
        _ => 0,
    }
}

/// Return true if the bed is occupied.
#[inline]
pub fn is_occupied(metadata: u8) -> bool {
    metadata & 4 != 0
}

/// Set if the bed is occupied or not.
#[inline]
pub fn set_occupied(metadata: &mut u8, occupied: bool) {
    *metadata &= !4;
    *metadata |= (occupied as u8) << 2;
}

/// Return true if the bed block is the head piece.
#[inline]
pub fn is_head(metadata: u8) -> bool {
    metadata & 8 != 0
}

/// Set if the bed block is the head piece or not.
#[inline]
pub fn set_head(metadata: &mut u8, head: bool) {
    *metadata &= !8;
    *metadata |= (head as u8) << 3;
}
