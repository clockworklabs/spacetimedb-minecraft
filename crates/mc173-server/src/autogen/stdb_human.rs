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
pub struct StdbHuman {
    pub entity_id: u32,
    pub username: String,
    pub sleeping: bool,
    pub sneaking: bool,
}

impl TableType for StdbHuman {
    const TABLE_NAME: &'static str = "StdbHuman";
    type ReducerEvent = super::ReducerEvent;
}

impl TableWithPrimaryKey for StdbHuman {
    type PrimaryKey = u32;
    fn primary_key(&self) -> &Self::PrimaryKey {
        &self.entity_id
    }
}

impl StdbHuman {
    #[allow(unused)]
    pub fn filter_by_entity_id(entity_id: u32) -> TableIter<Self> {
        Self::filter(|row| row.entity_id == entity_id)
    }
    #[allow(unused)]
    pub fn find_by_entity_id(entity_id: u32) -> Option<Self> {
        Self::find(|row| row.entity_id == entity_id)
    }
    #[allow(unused)]
    pub fn filter_by_username(username: String) -> TableIter<Self> {
        Self::filter(|row| row.username == username)
    }
    #[allow(unused)]
    pub fn filter_by_sleeping(sleeping: bool) -> TableIter<Self> {
        Self::filter(|row| row.sleeping == sleeping)
    }
    #[allow(unused)]
    pub fn filter_by_sneaking(sneaking: bool) -> TableIter<Self> {
        Self::filter(|row| row.sneaking == sneaking)
    }
}