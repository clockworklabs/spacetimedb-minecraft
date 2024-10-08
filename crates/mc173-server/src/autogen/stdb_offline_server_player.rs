// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

#![allow(unused_imports)]
use super::stdb_server_player::StdbServerPlayer;
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
pub struct StdbOfflineServerPlayer {
    pub username: String,
    pub player: StdbServerPlayer,
}

impl TableType for StdbOfflineServerPlayer {
    const TABLE_NAME: &'static str = "StdbOfflineServerPlayer";
    type ReducerEvent = super::ReducerEvent;
}

impl TableWithPrimaryKey for StdbOfflineServerPlayer {
    type PrimaryKey = String;
    fn primary_key(&self) -> &Self::PrimaryKey {
        &self.username
    }
}

impl StdbOfflineServerPlayer {
    #[allow(unused)]
    pub fn filter_by_username(username: String) -> TableIter<Self> {
        Self::filter(|row| row.username == username)
    }
    #[allow(unused)]
    pub fn find_by_username(username: String) -> Option<Self> {
        Self::find(|row| row.username == username)
    }
}
