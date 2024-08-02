use glam::IVec3;
use spacetimedb::{spacetimedb, SpacetimeType};
use crate::chunk::Chunk;
use crate::ivec3::StdbIVec3;

#[spacetimedb(table)]
pub struct StdbChunk {
    #[primarykey]
    #[autoinc]
    pub chunk_id: u64,
    pub chunk: Chunk,
}

impl StdbChunk {
    pub fn insert_at_coords(x: i32, z: i32, chunk: Chunk) -> Option<StdbChunk> {
        let chunk_id = Self::chunk_coords_to_id(x, z);
        match StdbChunk::insert(StdbChunk {
            chunk_id,
            chunk,
        }) {
            Ok(chunk) => Some(chunk),
            Err(_) => None,
        }
    }
    pub fn filter_by_x_z(x: i32, z: i32) -> Option<StdbChunk> {
        let id = Self::chunk_coords_to_id(x, z);
        StdbChunk::filter_by_chunk_id(&id)
    }

    pub fn chunk_coords_to_id(x: i32, z: i32) -> u64 {
        // Convert i32s to u32s to ensure correct bit representation
        let x_u32 = x as u32;
        let z_u32 = z as u32;

        // Shift the bits of the first number (a_u32) left by 32 bits and convert to u64
        let a_u64 = (x_u32 as u64) << 32;

        // Convert the second number (b_u32) to u64
        let b_u64 = z_u32 as u64;

        // Combine the two u64 numbers using bitwise OR
        a_u64 | b_u64
    }

    pub fn chunk_id_to_coords(chunk_id: u64) -> (i32, i32) {
        // Extract the first i32 from the higher 32 bits by right-shifting 32 bits and then converting to i32
        let x = (chunk_id >> 32) as i32;
        // Extract the second i32 from the lower 32 bits by applying a mask and then converting to i32
        let z = (chunk_id & 0xFFFFFFFF) as i32;
        (x, z)
    }
}

#[spacetimedb(table)]
pub struct StdbChunkPopulated {
    #[primarykey]
    #[autoinc]
    // Note: This is not a chunk id, this ID is unique to this table
    pub id: i32,
    pub x: i32,
    pub z: i32,
    pub populated: u8,
}

#[spacetimedb(table)]
pub struct StdbTime {
    #[unique]
    pub id: i32,
    pub time: u64,
}

#[derive(Debug, Clone, SpacetimeType)]
pub struct StdbBreakBlockPacket {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub face: u8,
    pub status: u8,
}

/// State of a player breaking a block.
#[derive(SpacetimeType)]
pub struct BreakingBlock {
    /// The start time of this block breaking.
    pub start_time: u64,
    /// The position of the block.
    pub pos: StdbIVec3,
    /// The block id.
    pub id: u8,
}

#[spacetimedb(table)]
pub struct StdbBreakingBlock {
    #[unique]
    pub entity_id: u32,
    pub state: BreakingBlock,
}

// impl StdbChunk {
//     pub fn is_block(&self, pos: IVec3, id: u8) -> bool {
//         let (pos_id, _) = self.chunk.get_block(pos);
//         return pos_id == id;
//     }
//
//     pub fn get_block(&self, pos: IVec3) -> Option<(u8, u8)> {
//         Some(self.chunk.get_block(pos))
//     }
// }