// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB. EDITS TO THIS FILE
// WILL NOT BE SAVED. MODIFY TABLES IN RUST INSTEAD.

#![allow(unused_imports)]
use spacetimedb_sdk::callbacks::{DbCallbacks, ReducerCallbacks};
use spacetimedb_sdk::client_api_messages::{Event, TableUpdate};
use spacetimedb_sdk::client_cache::{ClientCache, RowCallbackReminders};
use spacetimedb_sdk::global_connection::with_connection_mut;
use spacetimedb_sdk::identity::Credentials;
use spacetimedb_sdk::reducer::AnyReducerEvent;
use spacetimedb_sdk::spacetime_module::SpacetimeModule;
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
pub mod breaking_block;
pub mod chunk;
pub mod chunk_event;
pub mod chunk_nibble_array_3;
pub mod chunk_update_type;
pub mod generate_chunk_reducer;
pub mod generate_chunks_reducer;
pub mod hand_slot_packet;
pub mod handle_look_reducer;
pub mod handle_position_look_reducer;
pub mod handle_position_reducer;
pub mod item_stack;
pub mod java_random;
pub mod light_kind;
pub mod light_update;
pub mod set_weather_reducer;
pub mod stdb_block_set_update;
pub mod stdb_break_block_packet;
pub mod stdb_breaking_block;
pub mod stdb_chunk;
pub mod stdb_chunk_populated;
pub mod stdb_chunk_update;
pub mod stdb_chunk_view;
pub mod stdb_client_state;
pub mod stdb_connection_status;
pub mod stdb_d_vec_3;
pub mod stdb_entity;
pub mod stdb_entity_tracker;
pub mod stdb_entity_tracker_update_type;
pub mod stdb_entity_view;
pub mod stdb_give_item_reducer;
pub mod stdb_hand_slot;
pub mod stdb_handle_accept_reducer;
pub mod stdb_handle_break_block_reducer;
pub mod stdb_handle_hand_slot_reducer;
pub mod stdb_handle_login_reducer;
pub mod stdb_handle_lost_reducer;
pub mod stdb_handle_place_block_reducer;
pub mod stdb_human;
pub mod stdb_i_16_vec_3;
pub mod stdb_i_32_vec_3;
pub mod stdb_i_8_vec_2;
pub mod stdb_in_login_packet;
pub mod stdb_inventory;
pub mod stdb_item_stack;
pub mod stdb_look_packet;
pub mod stdb_offline_player;
pub mod stdb_offline_server_player;
pub mod stdb_place_block_packet;
pub mod stdb_player_window;
pub mod stdb_player_window_chest;
pub mod stdb_playing_state;
pub mod stdb_position_look_packet;
pub mod stdb_position_packet;
pub mod stdb_rand;
pub mod stdb_server_player;
pub mod stdb_server_world;
pub mod stdb_set_block_event;
pub mod stdb_tick_mode;
pub mod stdb_time;
pub mod stdb_tracked_player;
pub mod stdb_vec_2;
pub mod stdb_weather;
pub mod stdb_window_kind;
pub mod stdb_world;
pub mod tick_reducer;
pub mod weather;

pub use biome::*;
pub use breaking_block::*;
pub use chunk::*;
pub use chunk_event::*;
pub use chunk_nibble_array_3::*;
pub use chunk_update_type::*;
pub use generate_chunk_reducer::*;
pub use generate_chunks_reducer::*;
pub use hand_slot_packet::*;
pub use handle_look_reducer::*;
pub use handle_position_look_reducer::*;
pub use handle_position_reducer::*;
pub use item_stack::*;
pub use java_random::*;
pub use light_kind::*;
pub use light_update::*;
pub use set_weather_reducer::*;
pub use stdb_block_set_update::*;
pub use stdb_break_block_packet::*;
pub use stdb_breaking_block::*;
pub use stdb_chunk::*;
pub use stdb_chunk_populated::*;
pub use stdb_chunk_update::*;
pub use stdb_chunk_view::*;
pub use stdb_client_state::*;
pub use stdb_connection_status::*;
pub use stdb_d_vec_3::*;
pub use stdb_entity::*;
pub use stdb_entity_tracker::*;
pub use stdb_entity_tracker_update_type::*;
pub use stdb_entity_view::*;
pub use stdb_give_item_reducer::*;
pub use stdb_hand_slot::*;
pub use stdb_handle_accept_reducer::*;
pub use stdb_handle_break_block_reducer::*;
pub use stdb_handle_hand_slot_reducer::*;
pub use stdb_handle_login_reducer::*;
pub use stdb_handle_lost_reducer::*;
pub use stdb_handle_place_block_reducer::*;
pub use stdb_human::*;
pub use stdb_i_16_vec_3::*;
pub use stdb_i_32_vec_3::*;
pub use stdb_i_8_vec_2::*;
pub use stdb_in_login_packet::*;
pub use stdb_inventory::*;
pub use stdb_item_stack::*;
pub use stdb_look_packet::*;
pub use stdb_offline_player::*;
pub use stdb_offline_server_player::*;
pub use stdb_place_block_packet::*;
pub use stdb_player_window::*;
pub use stdb_player_window_chest::*;
pub use stdb_playing_state::*;
pub use stdb_position_look_packet::*;
pub use stdb_position_packet::*;
pub use stdb_rand::*;
pub use stdb_server_player::*;
pub use stdb_server_world::*;
pub use stdb_set_block_event::*;
pub use stdb_tick_mode::*;
pub use stdb_time::*;
pub use stdb_tracked_player::*;
pub use stdb_vec_2::*;
pub use stdb_weather::*;
pub use stdb_window_kind::*;
pub use stdb_world::*;
pub use tick_reducer::*;
pub use weather::*;

#[allow(unused)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ReducerEvent {
    GenerateChunk(generate_chunk_reducer::GenerateChunkArgs),
    GenerateChunks(generate_chunks_reducer::GenerateChunksArgs),
    HandleLook(handle_look_reducer::HandleLookArgs),
    HandlePosition(handle_position_reducer::HandlePositionArgs),
    HandlePositionLook(handle_position_look_reducer::HandlePositionLookArgs),
    SetWeather(set_weather_reducer::SetWeatherArgs),
    StdbGiveItem(stdb_give_item_reducer::StdbGiveItemArgs),
    StdbHandleAccept(stdb_handle_accept_reducer::StdbHandleAcceptArgs),
    StdbHandleBreakBlock(stdb_handle_break_block_reducer::StdbHandleBreakBlockArgs),
    StdbHandleHandSlot(stdb_handle_hand_slot_reducer::StdbHandleHandSlotArgs),
    StdbHandleLogin(stdb_handle_login_reducer::StdbHandleLoginArgs),
    StdbHandleLost(stdb_handle_lost_reducer::StdbHandleLostArgs),
    StdbHandlePlaceBlock(stdb_handle_place_block_reducer::StdbHandlePlaceBlockArgs),
    Tick(tick_reducer::TickArgs),
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
						"StdbBlockSetUpdate" => client_cache.handle_table_update_no_primary_key::<stdb_block_set_update::StdbBlockSetUpdate>(callbacks, table_update),
			"StdbBreakingBlock" => client_cache.handle_table_update_no_primary_key::<stdb_breaking_block::StdbBreakingBlock>(callbacks, table_update),
			"StdbChunk" => client_cache.handle_table_update_with_primary_key::<stdb_chunk::StdbChunk>(callbacks, table_update),
			"StdbChunkPopulated" => client_cache.handle_table_update_with_primary_key::<stdb_chunk_populated::StdbChunkPopulated>(callbacks, table_update),
			"StdbChunkUpdate" => client_cache.handle_table_update_no_primary_key::<stdb_chunk_update::StdbChunkUpdate>(callbacks, table_update),
			"StdbChunkView" => client_cache.handle_table_update_with_primary_key::<stdb_chunk_view::StdbChunkView>(callbacks, table_update),
			"StdbConnectionStatus" => client_cache.handle_table_update_no_primary_key::<stdb_connection_status::StdbConnectionStatus>(callbacks, table_update),
			"StdbEntity" => client_cache.handle_table_update_with_primary_key::<stdb_entity::StdbEntity>(callbacks, table_update),
			"StdbEntityTracker" => client_cache.handle_table_update_with_primary_key::<stdb_entity_tracker::StdbEntityTracker>(callbacks, table_update),
			"StdbEntityView" => client_cache.handle_table_update_with_primary_key::<stdb_entity_view::StdbEntityView>(callbacks, table_update),
			"StdbHandSlot" => client_cache.handle_table_update_with_primary_key::<stdb_hand_slot::StdbHandSlot>(callbacks, table_update),
			"StdbHuman" => client_cache.handle_table_update_with_primary_key::<stdb_human::StdbHuman>(callbacks, table_update),
			"StdbInventory" => client_cache.handle_table_update_with_primary_key::<stdb_inventory::StdbInventory>(callbacks, table_update),
			"StdbItemStack" => client_cache.handle_table_update_with_primary_key::<stdb_item_stack::StdbItemStack>(callbacks, table_update),
			"StdbOfflinePlayer" => client_cache.handle_table_update_no_primary_key::<stdb_offline_player::StdbOfflinePlayer>(callbacks, table_update),
			"StdbOfflineServerPlayer" => client_cache.handle_table_update_with_primary_key::<stdb_offline_server_player::StdbOfflineServerPlayer>(callbacks, table_update),
			"StdbPlayerWindow" => client_cache.handle_table_update_no_primary_key::<stdb_player_window::StdbPlayerWindow>(callbacks, table_update),
			"StdbPlayerWindowChest" => client_cache.handle_table_update_no_primary_key::<stdb_player_window_chest::StdbPlayerWindowChest>(callbacks, table_update),
			"StdbRand" => client_cache.handle_table_update_no_primary_key::<stdb_rand::StdbRand>(callbacks, table_update),
			"StdbServerPlayer" => client_cache.handle_table_update_with_primary_key::<stdb_server_player::StdbServerPlayer>(callbacks, table_update),
			"StdbServerWorld" => client_cache.handle_table_update_with_primary_key::<stdb_server_world::StdbServerWorld>(callbacks, table_update),
			"StdbSetBlockEvent" => client_cache.handle_table_update_no_primary_key::<stdb_set_block_event::StdbSetBlockEvent>(callbacks, table_update),
			"StdbTime" => client_cache.handle_table_update_no_primary_key::<stdb_time::StdbTime>(callbacks, table_update),
			"StdbTrackedPlayer" => client_cache.handle_table_update_with_primary_key::<stdb_tracked_player::StdbTrackedPlayer>(callbacks, table_update),
			"StdbWeather" => client_cache.handle_table_update_with_primary_key::<stdb_weather::StdbWeather>(callbacks, table_update),
			"StdbWorld" => client_cache.handle_table_update_with_primary_key::<stdb_world::StdbWorld>(callbacks, table_update),
			_ => spacetimedb_sdk::log::error!("TableRowOperation on unknown table {:?}", table_name),
}
    }
    fn invoke_row_callbacks(
        &self,
        reminders: &mut RowCallbackReminders,
        worker: &mut DbCallbacks,
        reducer_event: Option<Arc<AnyReducerEvent>>,
        state: &Arc<ClientCache>,
    ) {
        reminders.invoke_callbacks::<stdb_block_set_update::StdbBlockSetUpdate>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_breaking_block::StdbBreakingBlock>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_chunk::StdbChunk>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_chunk_populated::StdbChunkPopulated>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_chunk_update::StdbChunkUpdate>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_chunk_view::StdbChunkView>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_connection_status::StdbConnectionStatus>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_entity::StdbEntity>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_entity_tracker::StdbEntityTracker>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_entity_view::StdbEntityView>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_hand_slot::StdbHandSlot>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_human::StdbHuman>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_inventory::StdbInventory>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_item_stack::StdbItemStack>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_offline_player::StdbOfflinePlayer>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_offline_server_player::StdbOfflineServerPlayer>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_player_window::StdbPlayerWindow>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_player_window_chest::StdbPlayerWindowChest>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_rand::StdbRand>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_server_player::StdbServerPlayer>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_server_world::StdbServerWorld>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_set_block_event::StdbSetBlockEvent>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_time::StdbTime>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_tracked_player::StdbTrackedPlayer>(
            worker,
            &reducer_event,
            state,
        );
        reminders.invoke_callbacks::<stdb_weather::StdbWeather>(worker, &reducer_event, state);
        reminders.invoke_callbacks::<stdb_world::StdbWorld>(worker, &reducer_event, state);
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
						"generate_chunk" => _reducer_callbacks.handle_event_of_type::<generate_chunk_reducer::GenerateChunkArgs, ReducerEvent>(event, _state, ReducerEvent::GenerateChunk),
			"generate_chunks" => _reducer_callbacks.handle_event_of_type::<generate_chunks_reducer::GenerateChunksArgs, ReducerEvent>(event, _state, ReducerEvent::GenerateChunks),
			"handle_look" => _reducer_callbacks.handle_event_of_type::<handle_look_reducer::HandleLookArgs, ReducerEvent>(event, _state, ReducerEvent::HandleLook),
			"handle_position" => _reducer_callbacks.handle_event_of_type::<handle_position_reducer::HandlePositionArgs, ReducerEvent>(event, _state, ReducerEvent::HandlePosition),
			"handle_position_look" => _reducer_callbacks.handle_event_of_type::<handle_position_look_reducer::HandlePositionLookArgs, ReducerEvent>(event, _state, ReducerEvent::HandlePositionLook),
			"set_weather" => _reducer_callbacks.handle_event_of_type::<set_weather_reducer::SetWeatherArgs, ReducerEvent>(event, _state, ReducerEvent::SetWeather),
			"stdb_give_item" => _reducer_callbacks.handle_event_of_type::<stdb_give_item_reducer::StdbGiveItemArgs, ReducerEvent>(event, _state, ReducerEvent::StdbGiveItem),
			"stdb_handle_accept" => _reducer_callbacks.handle_event_of_type::<stdb_handle_accept_reducer::StdbHandleAcceptArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandleAccept),
			"stdb_handle_break_block" => _reducer_callbacks.handle_event_of_type::<stdb_handle_break_block_reducer::StdbHandleBreakBlockArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandleBreakBlock),
			"stdb_handle_hand_slot" => _reducer_callbacks.handle_event_of_type::<stdb_handle_hand_slot_reducer::StdbHandleHandSlotArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandleHandSlot),
			"stdb_handle_login" => _reducer_callbacks.handle_event_of_type::<stdb_handle_login_reducer::StdbHandleLoginArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandleLogin),
			"stdb_handle_lost" => _reducer_callbacks.handle_event_of_type::<stdb_handle_lost_reducer::StdbHandleLostArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandleLost),
			"stdb_handle_place_block" => _reducer_callbacks.handle_event_of_type::<stdb_handle_place_block_reducer::StdbHandlePlaceBlockArgs, ReducerEvent>(event, _state, ReducerEvent::StdbHandlePlaceBlock),
			"tick" => _reducer_callbacks.handle_event_of_type::<tick_reducer::TickArgs, ReducerEvent>(event, _state, ReducerEvent::Tick),
			unknown => { spacetimedb_sdk::log::error!("Event on an unknown reducer: {:?}", unknown); None }
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
            "StdbBlockSetUpdate" => client_cache
                .handle_resubscribe_for_type::<stdb_block_set_update::StdbBlockSetUpdate>(
                    callbacks, new_subs,
                ),
            "StdbBreakingBlock" => client_cache
                .handle_resubscribe_for_type::<stdb_breaking_block::StdbBreakingBlock>(
                    callbacks, new_subs,
                ),
            "StdbChunk" => client_cache
                .handle_resubscribe_for_type::<stdb_chunk::StdbChunk>(callbacks, new_subs),
            "StdbChunkPopulated" => client_cache
                .handle_resubscribe_for_type::<stdb_chunk_populated::StdbChunkPopulated>(
                    callbacks, new_subs,
                ),
            "StdbChunkUpdate" => client_cache
                .handle_resubscribe_for_type::<stdb_chunk_update::StdbChunkUpdate>(
                    callbacks, new_subs,
                ),
            "StdbChunkView" => client_cache
                .handle_resubscribe_for_type::<stdb_chunk_view::StdbChunkView>(callbacks, new_subs),
            "StdbConnectionStatus" => client_cache
                .handle_resubscribe_for_type::<stdb_connection_status::StdbConnectionStatus>(
                    callbacks, new_subs,
                ),
            "StdbEntity" => client_cache
                .handle_resubscribe_for_type::<stdb_entity::StdbEntity>(callbacks, new_subs),
            "StdbEntityTracker" => client_cache
                .handle_resubscribe_for_type::<stdb_entity_tracker::StdbEntityTracker>(
                    callbacks, new_subs,
                ),
            "StdbEntityView" => client_cache
                .handle_resubscribe_for_type::<stdb_entity_view::StdbEntityView>(
                    callbacks, new_subs,
                ),
            "StdbHandSlot" => client_cache
                .handle_resubscribe_for_type::<stdb_hand_slot::StdbHandSlot>(callbacks, new_subs),
            "StdbHuman" => client_cache
                .handle_resubscribe_for_type::<stdb_human::StdbHuman>(callbacks, new_subs),
            "StdbInventory" => client_cache
                .handle_resubscribe_for_type::<stdb_inventory::StdbInventory>(callbacks, new_subs),
            "StdbItemStack" => client_cache
                .handle_resubscribe_for_type::<stdb_item_stack::StdbItemStack>(callbacks, new_subs),
            "StdbOfflinePlayer" => client_cache
                .handle_resubscribe_for_type::<stdb_offline_player::StdbOfflinePlayer>(
                    callbacks, new_subs,
                ),
            "StdbOfflineServerPlayer" => client_cache
                .handle_resubscribe_for_type::<stdb_offline_server_player::StdbOfflineServerPlayer>(
                callbacks, new_subs,
            ),
            "StdbPlayerWindow" => client_cache
                .handle_resubscribe_for_type::<stdb_player_window::StdbPlayerWindow>(
                    callbacks, new_subs,
                ),
            "StdbPlayerWindowChest" => client_cache
                .handle_resubscribe_for_type::<stdb_player_window_chest::StdbPlayerWindowChest>(
                    callbacks, new_subs,
                ),
            "StdbRand" => {
                client_cache.handle_resubscribe_for_type::<stdb_rand::StdbRand>(callbacks, new_subs)
            }
            "StdbServerPlayer" => client_cache
                .handle_resubscribe_for_type::<stdb_server_player::StdbServerPlayer>(
                    callbacks, new_subs,
                ),
            "StdbServerWorld" => client_cache
                .handle_resubscribe_for_type::<stdb_server_world::StdbServerWorld>(
                    callbacks, new_subs,
                ),
            "StdbSetBlockEvent" => client_cache
                .handle_resubscribe_for_type::<stdb_set_block_event::StdbSetBlockEvent>(
                    callbacks, new_subs,
                ),
            "StdbTime" => {
                client_cache.handle_resubscribe_for_type::<stdb_time::StdbTime>(callbacks, new_subs)
            }
            "StdbTrackedPlayer" => client_cache
                .handle_resubscribe_for_type::<stdb_tracked_player::StdbTrackedPlayer>(
                    callbacks, new_subs,
                ),
            "StdbWeather" => client_cache
                .handle_resubscribe_for_type::<stdb_weather::StdbWeather>(callbacks, new_subs),
            "StdbWorld" => client_cache
                .handle_resubscribe_for_type::<stdb_world::StdbWorld>(callbacks, new_subs),
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
