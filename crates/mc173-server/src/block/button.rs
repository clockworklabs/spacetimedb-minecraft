//! Button special functions for metadata.

use crate::geom::Face;


/// The the face the button is connected to. In b1.7.3, buttons can only attach to X/Z 
/// faces, not neg/pos Y.
#[inline]
pub fn get_face(metadata: u8) -> Option<Face> {
    Some(match metadata & 7 {
        1 => Face::NegX,
        2 => Face::PosX,
        3 => Face::NegZ,
        4 => Face::PosZ,
        _ => return None
    })
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !7;
    *metadata |= match face {
        Face::NegY => 0,
        Face::PosY => 0,
        Face::NegZ => 3,
        Face::PosZ => 4,
        Face::NegX => 1,
        Face::PosX => 2,
    }
}

/// Return true if the button is currently active.
#[inline]
pub fn is_active(metadata: u8) -> bool {
    metadata & 8 != 0
}

/// Set the button active or not.
#[inline]
pub fn set_active(metadata: &mut u8, active: bool) {
    *metadata &= !8;
    *metadata |= (active as u8) << 3;
}
