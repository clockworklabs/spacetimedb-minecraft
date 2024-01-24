// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

use super::chunk::Chunk;
#[allow(unused)]
use spacetimedb_sdk::{
    anyhow::{anyhow, Result},
    identity::Identity,
    reducer::{Reducer, ReducerCallbackId, Status},
    sats::{de::Deserialize, ser::Serialize},
    spacetimedb_lib,
    table::{TableIter, TableType, TableWithPrimaryKey},
    Address,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StdbChunk {
    pub chunk_id: i32,
    pub x: i32,
    pub z: i32,
    pub chunk: Chunk,
}

impl TableType for StdbChunk {
    const TABLE_NAME: &'static str = "StdbChunk";
    type ReducerEvent = super::ReducerEvent;
}

impl TableWithPrimaryKey for StdbChunk {
    type PrimaryKey = i32;
    fn primary_key(&self) -> &Self::PrimaryKey {
        &self.chunk_id
    }
}

impl StdbChunk {
    #[allow(unused)]
    pub fn filter_by_chunk_id(chunk_id: i32) -> Option<Self> {
        Self::find(|row| row.chunk_id == chunk_id)
    }
    #[allow(unused)]
    pub fn filter_by_x(x: i32) -> TableIter<Self> {
        Self::filter(|row| row.x == x)
    }
    #[allow(unused)]
    pub fn filter_by_z(z: i32) -> TableIter<Self> {
        Self::filter(|row| row.z == z)
    }
    #[allow(unused)]
    pub fn filter_by_chunk(chunk: Chunk) -> TableIter<Self> {
        Self::filter(|row| row.chunk == chunk)
    }
}