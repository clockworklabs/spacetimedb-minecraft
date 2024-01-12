//! Torch (including redstone torch) metadata functions.

use crate::geom::Face;


/// Get the face this torch is attached to.
#[inline]
pub fn get_face(metadata: u8) -> Option<Face> {
    Some(match metadata {
        1 => Face::NegX,
        2 => Face::PosX,
        3 => Face::NegZ,
        4 => Face::PosZ,
        5 => Face::NegY,
        _ => return None
    })  
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata = match face {
        Face::NegX => 1,
        Face::PosX => 2,
        Face::NegZ => 3,
        Face::PosZ => 4,
        Face::NegY => 5,
        _ => 5,
    }
}
