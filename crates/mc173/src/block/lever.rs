//! Lever special functions for metadata.

use crate::geom::Face;


/// The the face the lever is connected to. In b1.7.3, levers can only attach to X/Z and
/// bottom Y. This function also returns the secondary face where this lever's stick 
/// points to when not active.
#[inline]
pub fn get_face(metadata: u8) -> Option<(Face, Face)> {
    Some(match metadata & 7 {
        1 => (Face::NegX, Face::PosY),
        2 => (Face::PosX, Face::PosY),
        3 => (Face::NegZ, Face::PosY),
        4 => (Face::PosZ, Face::PosY),
        5 => (Face::NegY, Face::PosZ),
        6 => (Face::NegY, Face::PosX),
        _ => return None
    })
}

/// Set the face the lever is connected to and the direction of the lever's stick when
/// not active, not that X/Z faces forces the direction to positive Y. Only positive X/Z
/// should be used when facing bottom, other values will be forced to positive Z.
#[inline]
pub fn set_face(metadata: &mut u8, face: Face, dir: Face) {
    *metadata &= !7;
    *metadata |= match (face, dir) {
        (Face::NegY, Face::PosZ) => 5,
        (Face::NegY, _) => 6,
        (Face::PosY, _) => 0,
        (Face::NegZ, _) => 3,
        (Face::PosZ, _) => 4,
        (Face::NegX, _) => 1,
        (Face::PosX, _) => 2,
    }
}

/// Return true if the lever is currently active.
#[inline]
pub fn is_active(metadata: u8) -> bool {
    metadata & 8 != 0
}

/// Set the lever active or not.
#[inline]
pub fn set_active(metadata: &mut u8, active: bool) {
    *metadata &= !8;
    *metadata |= (active as u8) << 3;
}
