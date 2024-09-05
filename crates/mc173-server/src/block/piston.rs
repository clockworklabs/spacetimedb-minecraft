//! Piston behaviors.

use crate::geom::Face;


/// Get the facing of the piston base or extension.
#[inline]
pub fn get_face(metadata: u8) -> Option<Face> {
    Some(match metadata & 7 {
        0 => Face::NegY,
        1 => Face::PosY,
        2 => Face::NegZ,
        3 => Face::PosZ,
        4 => Face::NegX,
        5 => Face::PosX,
        _ => return None
    })
}

/// Set the facing of the piston base or extension.
#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !7;
    *metadata |= face as u8;
}

/// Get if a piston base has extended or not.
#[inline]
pub fn is_base_extended(metadata: u8) -> bool {
    metadata & 8 != 0
}

/// Set if a piston base has extended or not.
#[inline]
pub fn set_base_extended(metadata: &mut u8, extended: bool) {
    *metadata &= !8;
    *metadata |= (extended as u8) << 3;
}

/// Get if a piston extension is sticky or not.
#[inline]
pub fn is_ext_sticky(metadata: u8) -> bool {
    is_base_extended(metadata)  // Same bit so we use same function
}

/// Set a piston extension to be sticky or not.
#[inline]
pub fn set_ext_sticky(metadata: &mut u8, sticky: bool) {
    set_base_extended(metadata, sticky)
}
