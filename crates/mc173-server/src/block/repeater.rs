//! Redstone repeater metadata functions.

use crate::geom::Face;


/// Get the face where the repeater send power.
#[inline]
pub fn get_face(metadata: u8) -> Face {
    match metadata & 3 {
        0 => Face::NegZ,
        1 => Face::PosX,
        2 => Face::PosZ,
        3 => Face::NegX,
        _ => unreachable!()
    }
}

/// Set the face where the repeater send power.
#[inline]
pub fn set_face(metadata: &mut u8, face: Face) {
    *metadata &= !3;
    *metadata |= match face {
        Face::NegZ => 0,
        Face::PosX => 1,
        Face::PosZ => 2,
        Face::NegX => 3,
        _ => 0
    };
}

/// Get the delay of the repeater.
#[inline]
pub fn get_delay(metadata: u8) -> u8 {
    (metadata & 0b1100) >> 2
}

/// Set the delay of the repeater.
#[inline]
pub fn set_delay(metadata: &mut u8, delay: u8) {
    *metadata &= !0b1100;
    *metadata |= (delay & 0b11) << 2;
}

/// Get the delay of the repeater in ticks.
#[inline]
pub fn get_delay_ticks(metadata: u8) -> u64 {
    (get_delay(metadata) as u64 + 1) * 2
}
