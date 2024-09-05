use glam::{DVec3, IVec3};
use crate::autogen::{Chunk, StdbChunk, StdbWorld};
use crate::autogen::Biome;

/// Chunk size in both X and Z coordinates.
pub const CHUNK_WIDTH: usize = 16;
/// Chunk height.
pub const CHUNK_HEIGHT: usize = 128;
/// Internal chunk 2D size, in number of columns per chunk.
pub const CHUNK_2D_SIZE: usize = CHUNK_WIDTH * CHUNK_WIDTH;
/// Internal chunk 3D size, in number of block per chunk.
pub const CHUNK_3D_SIZE: usize = CHUNK_HEIGHT * CHUNK_2D_SIZE;


/// Calculate the index in the chunk's arrays for the given position (local or not). This
/// is the same layout used by Minecraft's code `_xxx xzzz zyyy yyyy`. Only firsts
/// relevant bits are taken in each coordinate component.
#[inline]
fn calc_3d_index(pos: IVec3) -> usize {
    debug_assert!(pos.y >= 0 && pos.y < CHUNK_HEIGHT as i32);
    let x = pos.x as u32 & 0b1111;
    let z = pos.z as u32 & 0b1111;
    let y = pos.y as u32 & 0b1111111;
    ((x << 11) | (z << 7) | (y << 0)) as usize
}

/// Calculate the index in the chunk's 2D arrays for the given position (local or not).
/// Y position is ignored.
#[inline]
fn calc_2d_index(pos: IVec3) -> usize {
    let x = pos.x as u32 & 0b1111;
    let z = pos.z as u32 & 0b1111;
    ((z << 4) | (x << 0)) as usize
}

/// Calculate the chunk position corresponding to the given block position. This returns
/// no position if the Y coordinate is invalid.
#[inline]
pub fn calc_chunk_pos(pos: IVec3) -> Option<(i32, i32)> {
    if pos.y < 0 || pos.y >= CHUNK_HEIGHT as i32 {
        None
    } else {
        Some(calc_chunk_pos_unchecked(pos))
    }
}

/// Calculate the chunk position corresponding to the given block position. The Y
/// coordinate is ignored, so it may be invalid.
#[inline]
pub fn calc_chunk_pos_unchecked(pos: IVec3) -> (i32, i32) {
    (pos.x >> 4, pos.z >> 4)
}

/// Calculate the chunk position where the given entity should be cached.
#[inline]
pub fn calc_entity_chunk_pos(pos: DVec3) -> (i32, i32) {
    // NOTE: Using unchecked because entities don't have limit for Y value.
    calc_chunk_pos_unchecked(pos.floor().as_ivec3())
}

/// Light values of a position in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Light {
    /// Block light level.
    pub block: u8,
    /// Sky light level.
    pub sky: u8,
    /// The real sky light level, depending on the time and weather.
    pub sky_real: u8,
}

impl Light {

    /// Calculate the maximum static light level (without time/weather attenuation).
    #[inline]
    pub fn max(self) -> u8 {
        u8::max(self.block, self.sky)
    }

    /// Calculate the maximum real light level (with time/weather attenuation).
    #[inline]
    pub fn max_real(self) -> u8 {
        u8::max(self.block, self.sky_real)
    }

    /// Calculate the block brightness from its light levels.
    #[inline]
    pub fn brightness(self) -> f32 {
        // TODO: In nether, OFFSET is 0.1
        const OFFSET: f32 = 0.05;
        let base = 1.0 - self.max_real() as f32 / 15.0;
        (1.0 - base) * (base * 3.0 + 1.0) * (1.0 - OFFSET) + OFFSET
    }

}

impl StdbWorld {
    /// Get a reference to a chunk, if existing.
    pub fn get_chunk(&self, cx: i32, cz: i32) -> Option<StdbChunk> {
        StdbChunk::find_by_chunk_id(StdbChunk::xz_to_chunk_id(cx, cz))
        // self.chunks.get(&(cx, cz)).and_then(|c| c.data.as_deref())
    }

    // =================== //
    //        HEIGHT       //
    // =================== //

    /// Get saved height of a chunk column, Y component is ignored in the position. The
    /// returned height is a signed 32 bit integer, but the possible value is only in
    /// range 0..=128, but it's easier to deal with `i32` because of vectors.
    pub fn get_height(&self, pos: IVec3) -> Option<i32> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_height(pos) as i32)
    }

    // =================== //
    //        LIGHTS       //
    // =================== //

    /// Get light level at the given position, in range 0..16.
    pub fn get_light(&self, mut pos: IVec3) -> Light {

        if pos.y > 127 {
            pos.y = 127;
        }

        let mut light = Light {
            block: 0,
            sky: 15,
            sky_real: 0,
        };

        if let Some((cx, cz)) = calc_chunk_pos(pos) {
            if let Some(chunk) = self.get_chunk(cx, cz) {
                light.block = chunk.get_block_light(pos);
                light.sky = chunk.get_sky_light(pos);
            }
        }

        light.sky_real = light.sky.saturating_sub(self.sky_light_subtracted);
        light

    }

    // =================== //
    //        BIOMES       //
    // =================== //

    /// Get the biome at some position (Y component is ignored).
    pub fn get_biome(&self, pos: IVec3) -> Option<Biome> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_biome(pos))
    }
}