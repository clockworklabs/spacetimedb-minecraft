//! LAdder special functions for metadata.

use crate::geom::Face;


/// The the face the button is connected to. In b1.7.3, buttons can only attach to X/Z 
/// faces, not neg/pos Y.
#[inline]
pub fn get_face(metadata: u8) -> Option<Face> {
    Some(match metadata {
        2 => Face::PosZ,
        3 => Face::NegZ,
        4 => Face::PosX,
        5 => Face::NegX,
        _ => return None
    })
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata = match face {
        Face::PosZ => 2,
        Face::NegZ => 3,
        Face::PosX => 4,
        Face::NegX => 5,
        _ => 0
    }
}
