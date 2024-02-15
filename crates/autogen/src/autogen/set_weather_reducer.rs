// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

use super::weather::Weather;
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
pub struct SetWeatherArgs {
    pub weather: Weather,
}

impl Reducer for SetWeatherArgs {
    const REDUCER_NAME: &'static str = "set_weather";
}

#[allow(unused)]
pub fn set_weather(weather: Weather) {
    SetWeatherArgs { weather }.invoke();
}

#[allow(unused)]
pub fn on_set_weather(
    mut __callback: impl FnMut(&Identity, Option<Address>, &Status, &Weather) + Send + 'static,
) -> ReducerCallbackId<SetWeatherArgs> {
    SetWeatherArgs::on_reducer(move |__identity, __addr, __status, __args| {
        let SetWeatherArgs { weather } = __args;
        __callback(__identity, __addr, __status, weather);
    })
}

#[allow(unused)]
pub fn once_on_set_weather(
    __callback: impl FnOnce(&Identity, Option<Address>, &Status, &Weather) + Send + 'static,
) -> ReducerCallbackId<SetWeatherArgs> {
    SetWeatherArgs::once_on_reducer(move |__identity, __addr, __status, __args| {
        let SetWeatherArgs { weather } = __args;
        __callback(__identity, __addr, __status, weather);
    })
}

#[allow(unused)]
pub fn remove_on_set_weather(id: ReducerCallbackId<SetWeatherArgs>) {
    SetWeatherArgs::remove_on_reducer(id);
}