use glam::IVec3;
use spacetimedb::{spacetimedb, SpacetimeType};
use crate::chunk::Chunk;

#[spacetimedb(table)]
pub struct StdbChunk {
    #[primarykey]
    #[autoinc]
    pub chunk_id: i32,
    pub x: i32,
    pub z: i32,

    pub chunk: Chunk,
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
pub struct BreakBlockPacket {
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
    pub pos_x: i32,
    pub pos_y: i32,
    pub pos_z: i32,
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