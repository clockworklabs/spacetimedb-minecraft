// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

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
pub struct SetBlockArgs {
    pub pos_x: i32,
    pub pos_y: i32,
    pub pos_z: i32,
    pub id: u8,
    pub metadata: u8,
}

impl Reducer for SetBlockArgs {
    const REDUCER_NAME: &'static str = "set_block";
}

#[allow(unused)]
pub fn set_block(pos_x: i32, pos_y: i32, pos_z: i32, id: u8, metadata: u8) {
    SetBlockArgs {
        pos_x,
        pos_y,
        pos_z,
        id,
        metadata,
    }
    .invoke();
}

#[allow(unused)]
pub fn on_set_block(
    mut __callback: impl FnMut(&Identity, Option<Address>, &Status, &i32, &i32, &i32, &u8, &u8)
        + Send
        + 'static,
) -> ReducerCallbackId<SetBlockArgs> {
    SetBlockArgs::on_reducer(move |__identity, __addr, __status, __args| {
        let SetBlockArgs {
            pos_x,
            pos_y,
            pos_z,
            id,
            metadata,
        } = __args;
        __callback(
            __identity, __addr, __status, pos_x, pos_y, pos_z, id, metadata,
        );
    })
}

#[allow(unused)]
pub fn once_on_set_block(
    __callback: impl FnOnce(&Identity, Option<Address>, &Status, &i32, &i32, &i32, &u8, &u8)
        + Send
        + 'static,
) -> ReducerCallbackId<SetBlockArgs> {
    SetBlockArgs::once_on_reducer(move |__identity, __addr, __status, __args| {
        let SetBlockArgs {
            pos_x,
            pos_y,
            pos_z,
            id,
            metadata,
        } = __args;
        __callback(
            __identity, __addr, __status, pos_x, pos_y, pos_z, id, metadata,
        );
    })
}

#[allow(unused)]
pub fn remove_on_set_block(id: ReducerCallbackId<SetBlockArgs>) {
    SetBlockArgs::remove_on_reducer(id);
}
