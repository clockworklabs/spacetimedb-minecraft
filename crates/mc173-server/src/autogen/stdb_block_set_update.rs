// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

#![allow(unused_imports)]
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
pub struct StdbBlockSetUpdate {
    pub update_id: u32,
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub block: u8,
    pub metadata: u8,
}

impl TableType for StdbBlockSetUpdate {
    const TABLE_NAME: &'static str = "StdbBlockSetUpdate";
    type ReducerEvent = super::ReducerEvent;
}

impl StdbBlockSetUpdate {
    #[allow(unused)]
    pub fn filter_by_update_id(update_id: u32) -> TableIter<Self> {
        Self::filter(|row| row.update_id == update_id)
    }
    #[allow(unused)]
    pub fn find_by_update_id(update_id: u32) -> Option<Self> {
        Self::find(|row| row.update_id == update_id)
    }
    #[allow(unused)]
    pub fn filter_by_x(x: i32) -> TableIter<Self> {
        Self::filter(|row| row.x == x)
    }
    #[allow(unused)]
    pub fn filter_by_y(y: i8) -> TableIter<Self> {
        Self::filter(|row| row.y == y)
    }
    #[allow(unused)]
    pub fn filter_by_z(z: i32) -> TableIter<Self> {
        Self::filter(|row| row.z == z)
    }
    #[allow(unused)]
    pub fn filter_by_block(block: u8) -> TableIter<Self> {
        Self::filter(|row| row.block == block)
    }
    #[allow(unused)]
    pub fn filter_by_metadata(metadata: u8) -> TableIter<Self> {
        Self::filter(|row| row.metadata == metadata)
    }
}