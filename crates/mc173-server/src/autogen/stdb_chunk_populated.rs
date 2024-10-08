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
pub struct StdbChunkPopulated {
    pub id: u32,
    pub x: i32,
    pub z: i32,
    pub populated: u8,
}

impl TableType for StdbChunkPopulated {
    const TABLE_NAME: &'static str = "StdbChunkPopulated";
    type ReducerEvent = super::ReducerEvent;
}

impl TableWithPrimaryKey for StdbChunkPopulated {
    type PrimaryKey = u32;
    fn primary_key(&self) -> &Self::PrimaryKey {
        &self.id
    }
}

impl StdbChunkPopulated {
    #[allow(unused)]
    pub fn filter_by_id(id: u32) -> TableIter<Self> {
        Self::filter(|row| row.id == id)
    }
    #[allow(unused)]
    pub fn find_by_id(id: u32) -> Option<Self> {
        Self::find(|row| row.id == id)
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
    pub fn filter_by_populated(populated: u8) -> TableIter<Self> {
        Self::filter(|row| row.populated == populated)
    }
}
