//! Door block specific logic.

use crate::geom::Face;


/// Get the face of this door.
#[inline]
pub fn get_face(metadata: u8) -> Face {
    match metadata & 3 {
        0 => Face::NegX,
        1 => Face::NegZ,
        2 => Face::PosX,
        3 => Face::PosZ,
        _ => unreachable!()
    }
}

#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata |= match face {
        Face::NegY => 0,
        Face::PosY => 0,
        Face::NegX => 0,
        Face::PosX => 2,
        Face::NegZ => 1,
        Face::PosZ => 3,
    }
}

/// If the block is a door (iron/wood), get if it's in open state.
#[inline]
pub fn is_open(metadata: u8) -> bool {
    metadata & 4 != 0
}

#[inline]
pub fn set_open(metadata: &mut u8, open: bool) {
    *metadata &= !4;
    *metadata |= (open as u8) << 2;
}

/// Return true if this door block is the upper part.
#[inline]
pub fn is_upper(metadata: u8) -> bool {
    metadata & 8 != 0
}

#[inline]
pub fn set_upper(metadata: &mut u8, upper: bool) {
    *metadata &= !8;
    *metadata |= (upper as u8) << 3;
}

/// Get the actual face of this door, depending on its face and open state.
#[inline]
pub fn get_actual_face(metadata: u8) -> Face {
    let face = get_face(metadata);
    if is_open(metadata) {
        face.rotate_right()
    } else {
        face
    }
}
