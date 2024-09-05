//! Fluid block special functions (mostly for water).


/// Return true if this still/moving fluid block acts like a source.
#[inline]
pub fn is_source(metadata: u8) -> bool {
    metadata == 0
}

/// Force this metadata to be a source fluid block. This basically just overwrite metadata
/// with a 0, which means that distance is 0 and fluid is not falling.
#[inline]
pub fn set_source(metadata: &mut u8) {
    *metadata = 0;
}

/// Get the distance to source of a fluid block. The distance can go up to 7, but does
/// not account for the falling state.
#[inline]
pub fn get_distance(metadata: u8) -> u8 {
    metadata & 7
}

#[inline]
pub fn set_distance(metadata: &mut u8, distance: u8) {
    debug_assert!(distance <= 7);
    *metadata &= !7;
    *metadata |= distance;
} 

/// Get if this fluid block is falling and therefore should not spread on sides.
#[inline]
pub fn is_falling(metadata: u8) -> bool {
    metadata & 8 != 0
}

#[inline]
pub fn set_falling(metadata: &mut u8, falling: bool) {
    *metadata &= !8;
    *metadata |= (falling as u8) << 3;
}

/// This function get the actual distance to the source of a fluid block, this account 
/// both the distance stored in the lower 3 bits, but also for the falling state: if a
/// fluid is falling, it acts like a source block for propagation.
#[inline]
pub fn get_actual_distance(metadata: u8) -> u8 {
    if is_falling(metadata) {
        0
    } else {
        get_distance(metadata)
    }
}

/// Calculate the actual height of a fluid block depending on its metadata.
#[inline]
pub fn get_actual_height(metadata: u8) -> f32 {
    1.0 - (get_actual_distance(metadata) + 1) as f32 / 9.0
}
