//! LAdder special functions for metadata.

use crate::geom::Face;


/// The the face the button is connected to. In b1.7.3, buttons can only attach to X/Z 
/// faces, not neg/pos Y.
#[inline]
pub fn get_face(metadata: u8) -> Face {
    match metadata & 3 {
        0 => Face::PosZ,
        1 => Face::NegZ,
        2 => Face::PosX,
        3 => Face::NegX,
        _ => unreachable!()
    }
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata |= match face {
        Face::PosZ => 0,
        Face::NegZ => 1,
        Face::PosX => 2,
        Face::NegX => 3,
        _ => 0
    }
}

/// Return true if the trapdoor is currently open.
#[inline]
pub fn is_open(metadata: u8) -> bool {
    metadata & 4 != 0
}

/// Set the trapdoor open or not.
#[inline]
pub fn set_open(metadata: &mut u8, active: bool) {
    *metadata &= !4;
    *metadata |= (active as u8) << 2;
}
