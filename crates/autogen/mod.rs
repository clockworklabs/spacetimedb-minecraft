// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

use spacetimedb_sdk::callbacks::{DbCallbacks, ReducerCallbacks};
use spacetimedb_sdk::client_api_messages::{Event, TableUpdate};
use spacetimedb_sdk::client_cache::{ClientCache, RowCallbackReminders};
use spacetimedb_sdk::global_connection::with_connection_mut;
use spacetimedb_sdk::identity::Credentials;
use spacetimedb_sdk::reducer::AnyReducerEvent;
use spacetimedb_sdk::spacetime_module::SpacetimeModule;
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
use std::sync::Arc;

pub mod biome;
pub mod chop_terrain_reducer;
pub mod chunk;
pub mod chunk_nibble_array_3;
pub mod generate_chunk_reducer;
pub mod generate_chunks_reducer;
pub mod set_block_reducer;
pub mod stdb_chunk;

pub use biome::*;
pub use chop_terrain_reducer::*;
pub use chunk::*;
pub use chunk_nibble_array_3::*;
pub use generate_chunk_reducer::*;
pub use generate_chunks_reducer::*;
pub use set_block_reducer::*;
pub use stdb_chunk::*;

#[allow(unused)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ReducerEvent {
    ChopTerrain(chop_terrain_reducer::ChopTerrainArgs),
    GenerateChunk(generate_chunk_reducer::GenerateChunkArgs),
    GenerateChunks(generate_chunks_reducer::GenerateChunksArgs),
    SetBlock(set_block_reducer::SetBlockArgs),
}

#[allow(unused)]
pub struct Module;
impl SpacetimeModule for Module {
    fn handle_table_update(
        &self,
        table_update: TableUpdate,
        client_cache: &mut ClientCache,
        callbacks: &mut RowCallbackReminders,
    ) {
        let table_name = &table_update.table_name[..];
        match table_name {
            "StdbChunk" => client_cache
                .handle_table_update_with_primary_key::<stdb_chunk::StdbChunk>(
                    callbacks,
                    table_update,
                ),
            _ => {
                spacetimedb_sdk::log::error!("TableRowOperation on unknown table {:?}", table_name)
            }
        }
    }
    fn invoke_row_callbacks(
        &self,
        reminders: &mut RowCallbackReminders,
        worker: &mut DbCallbacks,
        reducer_event: Option<Arc<AnyReducerEvent>>,
        state: &Arc<ClientCache>,
    ) {
        reminders.invoke_callbacks::<stdb_chunk::StdbChunk>(worker, &reducer_event, state);
    }
    fn handle_event(
        &self,
        event: Event,
        _reducer_callbacks: &mut ReducerCallbacks,
        _state: Arc<ClientCache>,
    ) -> Option<Arc<AnyReducerEvent>> {
        let Some(function_call) = &event.function_call else {
            spacetimedb_sdk::log::warn!("Received Event with None function_call");
            return None;
        };
        #[allow(clippy::match_single_binding)]
        match &function_call.reducer[..] {
            "chop_terrain" => _reducer_callbacks
                .handle_event_of_type::<chop_terrain_reducer::ChopTerrainArgs, ReducerEvent>(
                    event,
                    _state,
                    ReducerEvent::ChopTerrain,
                ),
            "generate_chunk" => _reducer_callbacks
                .handle_event_of_type::<generate_chunk_reducer::GenerateChunkArgs, ReducerEvent>(
                    event,
                    _state,
                    ReducerEvent::GenerateChunk,
                ),
            "generate_chunks" => _reducer_callbacks
                .handle_event_of_type::<generate_chunks_reducer::GenerateChunksArgs, ReducerEvent>(
                    event,
                    _state,
                    ReducerEvent::GenerateChunks,
                ),
            "set_block" => _reducer_callbacks
                .handle_event_of_type::<set_block_reducer::SetBlockArgs, ReducerEvent>(
                    event,
                    _state,
                    ReducerEvent::SetBlock,
                ),
            unknown => {
                spacetimedb_sdk::log::error!("Event on an unknown reducer: {:?}", unknown);
                None
            }
        }
    }
    fn handle_resubscribe(
        &self,
        new_subs: TableUpdate,
        client_cache: &mut ClientCache,
        callbacks: &mut RowCallbackReminders,
    ) {
        let table_name = &new_subs.table_name[..];
        match table_name {
            "StdbChunk" => client_cache
                .handle_resubscribe_for_type::<stdb_chunk::StdbChunk>(callbacks, new_subs),
            _ => {
                spacetimedb_sdk::log::error!("TableRowOperation on unknown table {:?}", table_name)
            }
        }
    }
}

/// Connect to a database named `db_name` accessible over the internet at the URI `spacetimedb_uri`.
///
/// If `credentials` are supplied, they will be passed to the new connection to
/// identify and authenticate the user. Otherwise, a set of `Credentials` will be
/// generated by the server.
pub fn connect<IntoUri>(
    spacetimedb_uri: IntoUri,
    db_name: &str,
    credentials: Option<Credentials>,
) -> Result<()>
where
    IntoUri: TryInto<spacetimedb_sdk::http::Uri>,
    <IntoUri as TryInto<spacetimedb_sdk::http::Uri>>::Error:
        std::error::Error + Send + Sync + 'static,
{
    with_connection_mut(|connection| {
        connection.connect(spacetimedb_uri, db_name, credentials, Arc::new(Module))?;
        Ok(())
    })
}
