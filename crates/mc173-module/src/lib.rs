//! A Minecraft beta 1.7.3 server backend in Rust.

use std::path::Prefix::DeviceNS;
use glam::{DVec3, IVec3};
use spacetimedb::{spacetimedb, SpacetimeType};
use crate::entity::StdbDVec3;

pub mod io;
pub mod util;
pub mod geom;
pub mod rand;

pub mod block;
pub mod item;
pub mod entity;
pub mod block_entity;
pub mod biome;

pub mod inventory;
pub mod craft;
pub mod smelt;
pub mod path;

pub mod chunk;
pub mod world;
pub mod storage;
pub mod gen;

#[spacetimedb(table)]
pub struct StdbTime {
    #[unique]
    id: i32,
    time: u64,
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
    entity_id: u32,
    state: BreakingBlock,
}

pub fn is_block_opaque_cube(pos: IVec3) -> bool {
    if let Some((id, _)) = self.get_block(pos) {
        block::material::is_opaque_cube(id)
    } else {
        false
    }
}

impl From<StdbDVec3> for DVec3 {
    fn from(value: StdbDVec3) -> Self {
        DVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}