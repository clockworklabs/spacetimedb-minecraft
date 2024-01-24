//! Common metadata functions.

use crate::geom::Face;


/// Get facing of the dispenser.
pub fn get_face(metadata: u8) -> Option<Face> {
    Some(match metadata {
        2 => Face::NegZ,
        3 => Face::PosZ,
        4 => Face::NegX,
        5 => Face::PosX,
        _ => return None
    })
}

/// Set facing of the dispenser.
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata = match face {
        Face::NegY => 0,
        Face::PosY => 1,
        Face::NegZ => 2,
        Face::PosZ => 3,
        Face::NegX => 4,
        Face::PosX => 5,
    }
}
