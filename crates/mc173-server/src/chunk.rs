use std::io;
use std::io::Write;
use std::sync::Arc;
use glam::{DVec3, IVec3};
use crate::autogen::{StdbChunk, StdbServerPlayer};
use crate::autogen::Biome;
use crate::proto;
use crate::proto::OutPacket;
use crate::server::Server;

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

impl StdbChunk {
    pub fn xz_to_chunk_id(x: i32, z: i32) -> u32 {
        // bounds check x and z, which must both fit in an i16
        assert!(x >= -32768 && x <= 32767);
        assert!(z >= -32768 && z <= 32767);
        ((x as u32) << 16) | (z as u32 & 0xFFFF)
    }

    pub fn id_to_x_y(id: u32) -> (i32, i32) {
        let x = (id >> 16) as i32;
        let z = (id & 0xFFFF) as i32;
        (x, z)
    }

    /// Get the height at the given position, the Y component is ignored.
    ///
    /// The height value corresponds to the Y value of the first block above the column
    /// with full sky light.
    #[inline]
    pub fn get_height(&self, pos: IVec3) -> u8 {
        self.chunk.height[calc_2d_index(pos)]
    }

    /// Get block light level at the given global position (rebased to chunk-local).
    /// Panics if Y component of the position is not between 0 and 128 (excluded).
    #[inline]
    pub fn get_block_light(&self, pos: IVec3) -> u8 {
        self.chunk.block_light.inner[calc_3d_index(pos)]
    }

    /// Get sky light level at the given global position (rebased to chunk-local).
    /// Panics if Y component of the position is not between 0 and 128 (excluded).
    #[inline]
    pub fn get_sky_light(&self, pos: IVec3) -> u8 {
        self.chunk.sky_light.inner[calc_3d_index(pos)]
    }

    /// Get the biome at the given position, the Y component is ignored.
    #[inline]
    pub fn get_biome(&self, pos: IVec3) -> Biome {
        self.chunk.biome[calc_2d_index(pos)].clone()
    }

    pub fn send_full(&self, server: &Server, connection_id: u64) {
        let client = server.clients[&connection_id];
        server.net.send(client, OutPacket::ChunkState(proto::ChunkStatePacket {
            cx: self.x,
            cz: self.z,
            init: true
        }));

        let from = IVec3 {
            x: self.x * 16,
            y: 0,
            z: self.z * 16,
        };

        let size = IVec3 {
            x: 16,
            y: 128,
            z: 16,
        };

        server.net.send(client, OutPacket::ChunkData(new_chunk_data_packet(self, from, size)));
    }

    /// Write this chunk's data to the given writer, the data is copied from the start
    /// point for the given size. Note that this function may change the start and size
    /// of the area to be more efficient while while writing data.
    pub fn write_data(&self, mut writer: impl Write, from: &mut IVec3, size: &mut IVec3) -> io::Result<()> {

        // If the Y component is not properly aligned for copying nibble bytes, adjust it.
        if from.y % 2 != 0 {
            from.y -= 1;
            size.y += 1;
        }

        // After check start point, we check that size if a multiple of 2.
        size.y = (size.y + 1) & !1;

        debug_assert!(from.y % 2 == 0);
        debug_assert!(size.y % 2 == 0);
        debug_assert!(size.x <= 16 && size.y <= 128 && size.z <= 16);

        let height = size.y as usize;
        let half_height = height / 2; // Used for nibble arrays.

        let from = *from;
        let to = from + *size; // Exclusive

        // If we want a full chunk
        if size.x == 16 && size.z == 16 && size.y == 128 {
            writer.write_all(&self.chunk.block)?;
            writer.write_all(&self.chunk.metadata.inner)?;
            writer.write_all(&self.chunk.block_light.inner)?;
            writer.write_all(&self.chunk.sky_light.inner)?;
        } else {

            for x in from.x..to.x {
                for z in from.z..to.z {
                    // Start index, Y component is first so we can copy the whole column.
                    let index = calc_3d_index(IVec3::new(x, from.y, z));
                    writer.write_all(&self.chunk.block[index..index + height])?;
                }
            }

            for x in from.x..to.x {
                for z in from.z..to.z {
                    let index = calc_3d_index(IVec3::new(x, from.y, z)) / 2;
                    writer.write_all(&self.chunk.metadata.inner[index..index + half_height])?;
                }
            }

            for x in from.x..to.x {
                for z in from.z..to.z {
                    let index = calc_3d_index(IVec3::new(x, from.y, z)) / 2;
                    writer.write_all(&self.chunk.block_light.inner[index..index + half_height])?;
                }
            }

            for x in from.x..to.x {
                for z in from.z..to.z {
                    let index = calc_3d_index(IVec3::new(x, from.y, z)) / 2;
                    writer.write_all(&self.chunk.sky_light.inner[index..index + half_height])?;
                }
            }

        }

        Ok(())

    }
}

/// Create a new chunk data packet for the given chunk. This only works for a single
/// chunk and the given coordinate should be part of that chunk. The two arguments "from"
/// and "to" are inclusive but might be modified to include more blocks if ths reduces
/// computation.
pub fn new_chunk_data_packet(chunk: &StdbChunk, mut from: IVec3, mut size: IVec3) -> proto::ChunkDataPacket {

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    debug_assert!(size.x != 0 && size.y != 0 && size.z != 0);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    chunk.write_data(&mut encoder, &mut from, &mut size).unwrap();

    debug_assert!(size.x != 0 && size.y != 0 && size.z != 0);

    proto::ChunkDataPacket {
        x: from.x,
        y: from.y as i16,
        z: from.z,
        x_size: size.x as u8,
        y_size: size.y as u8,
        z_size: size.z as u8,
        compressed_data: Arc::new(encoder.finish().unwrap()),
    }

}