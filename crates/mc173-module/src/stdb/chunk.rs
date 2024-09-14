use glam::IVec3;
use spacetimedb::{spacetimedb, SpacetimeType};
use crate::chunk::Chunk;
use crate::i32vec3::StdbI32Vec3;

#[spacetimedb(table(public))]
#[derive(Clone)]
pub struct StdbChunk {
    #[primarykey]
    pub chunk_id: u32,
    pub x: i32,
    pub z: i32,

    pub chunk: Chunk,
}

#[spacetimedb(table(public))]
#[derive(Copy, Clone)]
pub struct StdbChunkUpdate {
    #[autoinc]
    #[unique]
    pub update_id: u32,
    pub chunk_id: u32,
    pub update_type: ChunkUpdateType,
}

#[derive(SpacetimeType, Copy, Clone)]
pub enum ChunkUpdateType {
    FullChunkUpdate,
    BlockSet,
}

#[spacetimedb(table(public))]
#[derive(Debug, Clone)]
pub struct StdbBlockSetUpdate {
    #[autoinc]
    #[unique]
    pub update_id: u32,
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub block: u8,
    pub metadata: u8,
}

#[spacetimedb(table(public))]
pub struct StdbChunkView {
    #[primarykey]
    #[autoinc]
    pub view_id: u32,
    pub chunk_id: u32,
    pub observer_id: u32,
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
}

#[spacetimedb(table(public))]
pub struct StdbChunkPopulated {
    #[primarykey]
    // Note: This is not a chunk id, this ID is unique to this table
    pub id: u32,
    pub x: i32,
    pub z: i32,
    pub populated: u8,
}

#[spacetimedb(table(public))]
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
    pub pos: StdbI32Vec3,
    /// The block id.
    pub id: u8,
}

#[spacetimedb(table(public))]
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