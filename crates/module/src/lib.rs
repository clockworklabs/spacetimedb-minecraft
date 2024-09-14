// Copyright 2024 Clockwork Labs, Inc
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::process::exit;
use std::time::Duration;
use glam::{DVec3, IVec3};
use mc173_module::world::{StdbWorld, DIMENSION_OVERWORLD};
use spacetimedb::{ReducerContext, schedule, spacetimedb, SpacetimeType, Timestamp};
use mc173_module::{block, item};
use mc173_module::chunk::calc_entity_chunk_pos;
use mc173_module::chunk_cache::ChunkCache;
use mc173_module::dvec3::StdbDVec3;
use mc173_module::entity::StdbHuman;
use mc173_module::geom::Face;
use mc173_module::i16vec3::StdbI16Vec3;
use mc173_module::i32vec3::StdbI32Vec3;
use mc173_module::i8vec3::StdbI8Vec2;
use mc173_module::stdb::chunk::{StdbBreakBlockPacket, BreakingBlock, StdbBreakingBlock, StdbTime, StdbChunkUpdate, StdbChunk, ChunkUpdateType};
use mc173_module::stdb::weather::StdbWeather;
use mc173_module::storage::ChunkStorage;
use mc173_module::vec2::StdbVec2;
use crate::config::SPAWN_POS;
use crate::entity::{StdbEntityTracker, StdbEntityTrackerUpdateType};
use crate::offline::StdbOfflinePlayer;
use crate::player::{StdbClientState, StdbConnectionStatus, StdbEntity, StdbOfflineServerPlayer, StdbPlayingState, StdbServerPlayer};
use crate::proto::{StdbLookPacket, StdbPositionLookPacket, StdbPositionPacket};
use crate::world::{StdbServerWorld, StdbTickMode};

pub mod player;
pub mod world;
mod proto;
mod offline;
mod config;
mod entity;

/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

#[spacetimedb(init)]
pub fn init(context: ReducerContext) {
    let init_span = spacetimedb::time_span::Span::start("Init");
    let mut cache = ChunkCache::new();
    let nano_time = context.timestamp.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_nanos();
    // log::info!("Starting Generation");
    // generate_chunks(-5, -5, 5, 5);
    // log::info!("Generation complete");

    let new_world = StdbWorld::insert(
        StdbWorld::new(DIMENSION_OVERWORLD, nano_time)
    ).unwrap();

    StdbServerWorld::insert(StdbServerWorld {
        dimension_id: new_world.dimension_id,
        name: "Boppy's World".to_string(),
        seed: 9999,
        time: 0,
        tick_mode: StdbTickMode::Auto,
        tick_mode_manual: 0,
    }).unwrap();

    StdbTime::insert(StdbTime { id: 0, time: 0 }).unwrap();
    mc173_module::stdb::weather::init();
    mc173_module::stdb::rand::init(nano_time);

    // This has to be here because this is how we schedule tick
    // Do the very fist tick
    tick_inner(&mut cache);
    schedule!(Duration::from_millis(50), tick());
    
    cache.apply();
    init_span.end();
}

#[spacetimedb(reducer)]
pub fn stdb_handle_accept(connection_id: u64) {
    log::info!("New connection started: {}", connection_id);
    let _ = StdbConnectionStatus::insert(StdbConnectionStatus {
        connection_id,
        status: StdbClientState::Handshaking,
    });
}

#[spacetimedb(reducer)]
fn stdb_handle_login(connection_id: u64, packet: proto::StdbInLoginPacket) {
    log::info!("New player logged in: {} {}", packet.username, connection_id);

    // This is checked by the translation layer
    // if packet.protocol_version != 14 {
    //     self.send_disconnect(client, format!("Protocol version mismatch!"));
    //     return;
    // }

    let spawn_pos = SPAWN_POS;

    // Get the offline player, if not existing we create a new one with the
    // NOTE(jdetter): support for this is below
    // let offline_player = self.offline_players.entry(packet.username.clone())
    //     .or_insert_with(|| {
    //         let spawn_world = &self.worlds[0];
    //         OfflinePlayer {
    //             world: spawn_world.state.name.clone(),
    //             pos: spawn_pos,
    //             look: Vec2::ZERO,
    //         }
    //     });

    // let (world_index, world) = self.worlds.iter_mut()
    //     .enumerate()
    //     .filter(|(_, world)| world.state.name == offline_player.world)
    //     .next()
    //     .expect("invalid offline player world name");

    let (entity, player) = if let Some(player) = StdbOfflineServerPlayer::filter_by_connection_id(&connection_id) {
        let existing_player = player.player;
        let player = StdbServerPlayer::insert(existing_player.clone()).unwrap();
        (StdbEntity::filter_by_entity_id(&existing_player.entity_id).unwrap(), player)
    } else {
        let new_entity = StdbEntity::insert(StdbEntity {
            entity_id: 0,
            on_ground: false,
            pos: std::convert::Into::<StdbDVec3>::into(spawn_pos).clone(),
            look: StdbVec2 {
                x: 1.0,
                y: 0.0,
            },
            dimension_id: DIMENSION_OVERWORLD,
        }).unwrap();

        let player = StdbServerPlayer::insert(StdbServerPlayer {
            entity_id: new_entity.entity_id.clone(),
            username: packet.username.clone(),
            connection_id,
            spawn_pos: spawn_pos.into(),
        }).unwrap();

        StdbHuman::insert(StdbHuman {
            entity_id: new_entity.entity_id.clone(),
            username: packet.username,
            sleeping: false,
            sneaking: false,
        }).unwrap();
        (new_entity, player)
    };

    // TODO(jdetter): Create an entity for this player
    // let entity = e::Human::new_with(|base, living, player| {
    //     base.pos = offline_player.pos;
    //     base.look = offline_player.look;
    //     base.persistent = false;
    //     base.can_pickup = true;
    //     living.artificial = true;
    //     living.health = 200;  // FIXME: Lot of HP for testing.
    //     player.username = packet.username.clone();
    // });

    // let entity_id = world.spawn_entity(entity);
    // NOTE(jdetter): We don't need to do this anymore
    // world.set_entity_player(entity_id, true);

    // Confirm the login by sending same packet in response.
    // self.net.send(client, OutPacket::Login(proto::OutLoginPacket {
    //     entity_id,
    //     random_seed: world.state.seed,
    //     dimension: match world.get_dimension() {
    //         Dimension::Overworld => 0,
    //         Dimension::Nether => -1,
    //     },
    // }));

    // The standard server sends the spawn position just after login response.
    // self.net.send(client, OutPacket::SpawnPosition(proto::SpawnPositionPacket {
    //     pos: spawn_pos.as_ivec3(),
    // }));

    // Send the initial position for the client.
    // self.net.send(client, OutPacket::PositionLook(proto::PositionLookPacket {
    //     pos: offline_player.pos,
    //     stance: offline_player.pos.y + 1.62,
    //     look: offline_player.look,
    //     on_ground: false,
    // }));

    // Time must be sent once at login to conclude the login phase.
    // self.net.send(client, OutPacket::UpdateTime(proto::UpdateTimePacket {
    //     time: world.get_time(),
    // }));

    // if world.get_weather() != Weather::Clear {
    //     self.net.send(client, OutPacket::Notification(proto::NotificationPacket {
    //         reason: 1,
    //     }));
    // }

    // If this player doesn't already have a tracker, add one
    if StdbEntityTracker::filter_by_entity_id(&player.entity_id).is_none() {
        log::info!("Created new entity tracker: connection_id: {} entity_id: {}", connection_id, player.entity_id);
        StdbEntityTracker::insert(StdbEntityTracker {
            entity_id: player.entity_id,
            distance: 512,
            interval: 2,
            time: 0,
            absolute_countdown_time: 0,
            vel_enable: false,
            pos: StdbI32Vec3 { x: 0, y: 0, z: 0, },
            vel: StdbI16Vec3 { x: 0, y: 0, z: 0, },
            look: StdbI8Vec2 { x: 0, y: 0 },
            sent_pos: StdbI32Vec3 { x: 0, y: 0, z: 0, },
            sent_vel: StdbI16Vec3 { x: 0, y: 0, z: 0, },
            sent_look: StdbI8Vec2 { x: 0, y: 0 },
            last_update_type: StdbEntityTrackerUpdateType::None,
            was_velocity_update: false,
        }).unwrap();
    }

    // Finally insert the player tracker.
    // let server_player = ServerPlayer::new(&self.net, client, entity_id, packet.username, &offline_player);
    // let player_index = world.handle_player_join(server_player);
    let mut world = StdbServerWorld::filter_by_dimension_id(&entity.dimension_id).unwrap();
    world.handle_player_join(player);


    // Replace the previous state with a playing state containing the world and
    // player indices, used to get to the player instance.
    // let previous_state = self.clients.insert(client, ClientState::Playing {
    //     world_index,
    //     player_index,
    // });
    let mut connection_status = StdbConnectionStatus::filter_by_connection_id(&connection_id).unwrap();
    connection_status.status = StdbClientState::Playing {
        0: StdbPlayingState {
            dimension_id: entity.dimension_id,
            entity_id: entity.entity_id,
        },
    };
    StdbConnectionStatus::update_by_connection_id(&connection_id, connection_status);

    // Just a sanity check...
    // debug_assert_eq!(previous_state, Some(ClientState::Handshaking));

    // TODO: Broadcast chat joining chat message.

}

#[spacetimedb(reducer)]
pub fn stdb_handle_lost(connection_id: u64, lost: bool) -> Result<(), String> {
    let player = StdbServerPlayer::filter_by_connection_id(&connection_id).ok_or(format!("Failed to find player with connection ID: {}", connection_id))?;
    log::info!("lost client #{}", connection_id);
    if let StdbClientState::Playing(playing_state) = StdbConnectionStatus::filter_by_connection_id(&connection_id).ok_or(
        format!("Failed to find playing with connection ID: {}", connection_id))?.status {
        // If the client was playing, remove it from its world.
        let mut world = StdbServerWorld::filter_by_dimension_id(&playing_state.dimension_id).ok_or(
            format!("Failed to find world with dimension ID: {}", &playing_state.dimension_id))?;
        // let world = &mut self.worlds[playing_state.dimension_id];
        world.handle_player_leave(player, lost);
        // if let Some(swapped_player) = world.handle_player_leave(player_index, true) {
        //     // If a player has been swapped in place of the removed one, update the
        //     // swapped one to point to its new index (and same world).
        //     let state = self.clients.get_mut(&swapped_player.client)
        //         .expect("swapped player should be existing");
        //     *state = ClientState::Playing { world_index, player_index };
        // }
    }

    Ok(())
}

#[spacetimedb(reducer)]
pub fn tick() {
    let mut cache = ChunkCache::new();
    tick_inner(&mut cache);
    // reschedule self
    schedule!(Duration::from_millis(50), tick());
    cache.apply();
}

pub fn tick_inner(cache: &mut ChunkCache) {
    // Do stuff...
    // Lastly, tick time
    for mut world in StdbWorld::iter() {
        let mut state = StdbServerWorld::filter_by_dimension_id(&world.dimension_id).unwrap();
        tick_world(&mut world, &mut state, cache);
        let dimension_id = world.dimension_id;
        StdbWorld::update_by_dimension_id(&dimension_id, world);
        StdbServerWorld::update_by_dimension_id(&dimension_id, state);
    }
}

/// Tick this world.
pub fn tick_world(world: &mut StdbWorld, state: &mut StdbServerWorld, cache: &mut ChunkCache) {

    // Get server-side time.
    let time = state.time;
    if time == 0 {
        init_world(world, cache);
    }

    // Poll all chunks to load in the world.
    // while let Some(reply) = self.state.storage.poll() {
    //     match reply {
    //         ChunkStorageReply::Load { cx, cz, res: Ok(snapshot) } => {
    //             debug!("loaded chunk from storage: {cx}/{cz}");
    //             self.world.insert_chunk_snapshot(snapshot);
    //         }
    //         ChunkStorageReply::Load { cx, cz, res: Err(err) } => {
    //             debug!("failed to load chunk from storage: {cx}/{cz}: {err}");
    //         }
    //         ChunkStorageReply::Save { cx, cz, res: Ok(()) } => {
    //             debug!("saved chunk in storage: {cx}/{cz}");
    //         }
    //         ChunkStorageReply::Save { cx, cz, res: Err(err) } => {
    //             debug!("failed to save chunk in storage: {cx}/{cz}: {err}");
    //         }
    //     }
    // }

    // Only run if no tick freeze.
    match state.tick_mode {
        StdbTickMode::Auto => {
            world.tick(cache);
        }
        StdbTickMode::Manual => {
            let mut n = state.tick_mode_manual;
            if n != 0 {
                world.tick(cache);
            }
            state.tick_mode_manual -= 1;
        }
    }

    // Swap events out in order to proceed them.
    // let mut events = self.world.swap_events(None).expect("events should be enabled");
    // self.state.events_count.push(events.len() as f32, 0.001);
    //
    // for event in events.drain(..) {
    //     match event {
    //         Event::Block { pos, inner } => match inner {
    //             BlockEvent::Set { id, metadata, prev_id, prev_metadata } =>
    //                 self.handle_block_set(pos, id, metadata, prev_id, prev_metadata),
    //             BlockEvent::Sound { id, metadata } =>
    //                 self.handle_block_sound(pos, id, metadata),
    //         }
    //         Event::Entity { id, inner } => match inner {
    //             EntityEvent::Spawn =>
    //                 self.handle_entity_spawn(id),
    //             EntityEvent::Remove =>
    //                 self.handle_entity_remove(id),
    //             EntityEvent::Position { pos } =>
    //                 self.handle_entity_position(id, pos),
    //             EntityEvent::Look { look } =>
    //                 self.handle_entity_look(id, look),
    //             EntityEvent::Velocity { vel } =>
    //                 self.handle_entity_velocity(id, vel),
    //             EntityEvent::Pickup { target_id } =>
    //                 self.handle_entity_pickup(id, target_id),
    //             EntityEvent::Damage =>
    //                 self.handle_entity_damage(id),
    //             EntityEvent::Dead =>
    //                 self.handle_entity_dead(id),
    //             EntityEvent::Metadata =>
    //                 self.handle_entity_metadata(id),
    //         }
    //         Event::BlockEntity { pos, inner } => match inner {
    //             BlockEntityEvent::Set =>
    //                 self.handle_block_entity_set(pos),
    //             BlockEntityEvent::Remove =>
    //                 self.handle_block_entity_remove(pos),
    //             BlockEntityEvent::Storage { storage, stack } =>
    //                 self.handle_block_entity_storage(pos, storage, stack),
    //             BlockEntityEvent::Progress { progress, value } =>
    //                 self.handle_block_entity_progress(pos, progress, value),
    //         }
    //         Event::Chunk { cx, cz, inner } => match inner {
    //             ChunkEvent::Set => {}
    //             ChunkEvent::Remove => {}
    //             ChunkEvent::Dirty => self.state.chunk_trackers.set_dirty(cx, cz),
    //         }
    //         Event::Weather { new, .. } =>
    //             self.handle_weather_change(new),
    //         Event::Explode { center, radius } =>
    //             self.handle_explode(center, radius),
    //         Event::DebugParticle { pos, block } =>
    //             self.handle_debug_particle(pos, block),
    //     }
    // }

    // Reinsert events after processing.
    // self.world.swap_events(Some(events));

    // Send time to every playing clients every second.
    // if time % 20 == 0 {
    //     let world_time = self.world.get_time();
    //     for player in &self.players {
    //         player.send(OutPacket::UpdateTime(proto::UpdateTimePacket {
    //             time: world_time,
    //         }));
    //     }
    // }

    // After we collected every block change, update all players accordingly.
    // TODO(jdetter): We should update player trackers here!
    // self.state.chunk_trackers.update_players(&self.players, &self.world);

    // After world events are processed, tick entity trackers.
    // for tracker in self.state.entity_trackers.values_mut() {
    //     if time % 60 == 0 {
    //         tracker.update_tracking_players(&mut self.players, &self.world);
    //     }
    //     tracker.tick_and_update_players(&self.players);
    // }

    // Drain dirty chunks coordinates and save them.
    // TODO(jdetter): We should update player trackers here!
    // while let Some((cx, cz)) = self.state.chunk_trackers.next_save() {
    //     if let Some(snapshot) = self.world.take_chunk_snapshot(cx, cz) {
    //         self.state.storage.request_save(snapshot);
    //     }
    // }

    // Update tick duration metric.
    // let tick_duration = start.elapsed();
    // self.state.tick_duration.push(tick_duration.as_secs_f32(), 0.02);

    // Finally increase server-side tick time.
    state.time += 1;
}

/// Initialize the world by ensuring that every entity is currently tracked. This
/// method can be called multiple time and should be idempotent.
fn init_world(world: &mut StdbWorld, cache: &mut ChunkCache) {

    // // Ensure that every entity has a tracker.
    // for (id, entity) in self.world.iter_entities() {
    //     self.state.entity_trackers.entry(id).or_insert_with(|| {
    //         let tracker = EntityTracker::new(id, entity);
    //         tracker.update_tracking_players(&mut self.players, &self.world);
    //         tracker
    //     });
    // }

    // NOTE: Temporary code.
    let init_world_span = spacetimedb::time_span::Span::start("Init World");
    let size = 1;
    let (center_cx, center_cz) = calc_entity_chunk_pos(DVec3::new(0.0, 100.0, 0.0));
    for cx in center_cx - size..=center_cx + size {
        for cz in center_cz - size..=center_cz + size {
            let request_load_span = spacetimedb::time_span::Span::start(format!("Generate Chunk: {}, {}", cx, cz).as_str());
            ChunkStorage::request_load(world, cx, cz, cache);
            request_load_span.end();
        }
    }
    init_world_span.end();
}

#[spacetimedb(reducer)]
pub fn generate_chunks(from_x: i32, from_z: i32, to_x: i32, to_z: i32) {
    let generate_chunks_span = spacetimedb::time_span::Span::start("Generate Chunks");
    let mut cache = ChunkCache::new();
    let mut world = StdbWorld::filter_by_dimension_id(&DIMENSION_OVERWORLD).unwrap();
    for x in from_x..to_x {
        for z in from_z..to_z {
            let inner_handle = spacetimedb::time_span::Span::start("spacetimedb individual chunk");
            ChunkStorage::request_load(&mut world, x, z, &mut cache);
            inner_handle.end();
            let _ = StdbChunkUpdate::insert(StdbChunkUpdate {
                update_id: 0,
                chunk_id: StdbChunk::xz_to_chunk_id(x, z),
                update_type: ChunkUpdateType::FullChunkUpdate,
            });
        }
    }
    cache.apply();
    generate_chunks_span.end();
}



// #[spacetimedb(reducer)]
// pub fn set_time(time: u64) {
//     StdbTime::update_by_id(&0, StdbTime {
//         id: 0,
//         time,
//     });
// }

// pub fn tick_time() {
//     let mut current_time = StdbTime::filter_by_id(&0).unwrap();
//     current_time.time += 1;
//     StdbTime::update_by_id(&0, current_time);
// }

#[spacetimedb(reducer)]
pub fn generate_chunk(x: i32, z: i32) {
    let mut cache = ChunkCache::new();
    let mut world = StdbWorld::filter_by_dimension_id(&DIMENSION_OVERWORLD).unwrap();
    ChunkStorage::request_load(&mut world, x, z, &mut cache);
    log::info!("Chunk Generated: {}, {}", x, z);
    cache.apply();
}

/*pub fn break_block(pos_x: i32, pos_y: i32, pos_z: i32) -> Option<(u8, u8)> {
    let (prev_id, prev_metadata) = self.set_block_notify(pos, block::AIR, 0)?;
    self.spawn_block_loot(pos, prev_id, prev_metadata, 1.0);
    Some((prev_id, prev_metadata))
}*/

// #[spacetimedb(reducer)]
// pub fn chop_terrain(pos_x: i32, pos_y: i32, pos_z: i32, size: i32) {
//     log::info!("Calling chop_terrain: {} {} {}", pos_x, pos_y, pos_z);
//     for x in pos_x..(pos_x + size) {
//         for z in pos_z..(pos_z + size) {
//             for y in pos_y..128 {
//                 set_block(x, y, z, 0, 0);
//             }
//         }
//     }
// }

// #[spacetimedb(reducer)]
// pub fn set_block(pos_x: i32, pos_y: i32, pos_z: i32, id: u8, metadata: u8) {
//     let pos = IVec3::new(pos_x, pos_y, pos_z);
//
//     let (cx, cz) = calc_chunk_pos(pos).unwrap();
//     let mut chunk = StdbChunk::filter_by_id(StdbChunk::calc_chunk_id(cx, cz)).unwrap();
//     let (prev_id, prev_metadata) = chunk.chunk.get_block(pos);
//
//     if id != prev_id || metadata != prev_metadata {
//         chunk.chunk.set_block(pos, id, metadata);
//         chunk.chunk.recompute_height(pos);
//
//         // Schedule light updates if the block light properties have changed.
//         /*if block::material::get_light_opacity(id) != block::material::get_light_opacity(prev_id)
//             || block::material::get_light_emission(id) != block::material::get_light_emission(prev_id) {
//             self.schedule_light_update(pos, LightKind::Block);
//             self.schedule_light_update(pos, LightKind::Sky);
//         }*/
//
//         /*        self.push_event(Event::Block {
//             pos,
//             inner: BlockEvent::Set {
//                 id,
//                 metadata,
//                 prev_id,
//                 prev_metadata,
//             }
//         });*/
//
//         // self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
//         log::info!("Set Block: {} {} {}", pos_x, pos_y, pos_z);
//
//         let chunk_id = chunk.chunk_id;
//         StdbChunk::update_by_chunk_id(&chunk_id, chunk);
//     }
//
//     // Some((prev_id, prev_metadata))
// }

#[spacetimedb(reducer)]
pub fn handle_break_block(entity_id: u32, packet: StdbBreakBlockPacket) {

    let mut cache = ChunkCache::new();

    // NOTE: Instead of just grabbing an arbirary world, we should use the world that the player is in
    let entity = StdbEntity::filter_by_entity_id(&entity_id).unwrap();
    let player = StdbServerPlayer::filter_by_entity_id(&entity_id).unwrap();
    let username = player.username;
    let mut world = StdbWorld::filter_by_dimension_id(&entity.dimension_id).unwrap();

    let face = match packet.face {
        0 => Face::NegY,
        1 => Face::PosY,
        2 => Face::NegZ,
        3 => Face::PosZ,
        4 => Face::NegX,
        5 => Face::PosX,
        _ => return,
    };

    // let Some(entity) = world.get_entity_mut(self.entity_id) else { return };
    let pos : IVec3 = IVec3 {
        x: packet.x,
        y: packet.y as i32,
        z: packet.z,
    };

    // tracing::trace!("packet: {packet:?}");
    log::info!("Breaking block: {} {} {} status={}", packet.x, packet.y, packet.z, packet.status);
    // TODO: Use server time for breaking blocks.

    // let in_water = entity.0.in_water;
    // let on_ground = entity.0.on_ground;
    let in_water = false;
    let on_ground = true;
    // let mut stack = self.main_inv[self.hand_slot as usize];
    let mut hand_item = item::DIAMOND_PICKAXE;

    let mut stdb_breaking_block = StdbBreakingBlock::filter_by_entity_id(&entity_id);

    if packet.status == 0 {

        // Special case to extinguish fire.
        if world.is_block(pos + face.delta(), block::FIRE, &mut cache) {
            world.set_block_notify(pos + face.delta(), block::AIR, 0, &mut cache);
        }

        // We ignore any interaction result for the left click (break block) to
        // avoid opening an inventory when breaking a container.
        // NOTE: Interact before 'get_block': relevant for redstone_ore lit.
        // world.interact_block(pos);

        // Start breaking a block, ignore if the position is invalid.
        if let Some((id, _)) = world.get_block(pos, &mut cache) {

            // let break_duration = world.get_break_duration(stack.id, id, in_water, on_ground);
            let break_duration = world.get_break_duration(hand_item, id, in_water, on_ground);
            if break_duration.is_infinite() {
                // Do nothing, the block is unbreakable.
            } else if break_duration == 0.0 {
                world.break_block(pos, &mut cache);
            } else {
                // self.breaking_block = Some(BreakingBlock {
                //     start_time: world.get_time(), // + (break_duration * 0.7) as u64,
                //     pos,
                //     id,
                // });
                let new_breaking_block = StdbBreakingBlock {
                    entity_id: entity_id,
                    state: BreakingBlock {
                        start_time: world.get_time(), // + (break_duration * 0.7) as u64,
                        pos: pos.into(),
                        id,
                    }
                };

                match stdb_breaking_block {
                    None => {
                        StdbBreakingBlock::insert(new_breaking_block).unwrap();
                    }
                    Some(_) => {
                        StdbBreakingBlock::update_by_entity_id(&entity_id, new_breaking_block);
                    }
                }

            }

        }

    } else if packet.status == 2 {
        // Block breaking should be finished.
        if let Some(some_breaking_block) = stdb_breaking_block.take() {
            if <mc173_module::i32vec3::StdbI32Vec3 as Into<IVec3>>::into(some_breaking_block.state.pos) == pos && world.is_block(pos, some_breaking_block.state.id, &mut cache) {
                // let break_duration = world.get_break_duration(stack.id, breaking_block.id, in_water, on_ground);
                // let break_duration = world.get_break_duration(hand_item, some_breaking_block.state.id, in_water, on_ground);
                // let min_time = some_breaking_block.state.start_time + (break_duration * 0.7) as u64;
                // if world.get_time() >= min_time {
                //     world.break_block(pos, &mut cache);
                // } else {
                //     log::warn!("from {}, incoherent break time, expected {min_time} but got {}", username, world.get_time());
                // }

                world.break_block(pos, &mut cache);
            } else {
                log::warn!("from {}, incoherent break position", username);
            }
        }
    } else if packet.status == 4 {
        // Drop the selected item.

        // if !stack.is_empty() {
        //
        //     stack.size -= 1;
        //     self.main_inv[self.hand_slot as usize] = stack.to_non_empty().unwrap_or_default();
        //
        //     self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
        //         window_id: 0,
        //         slot: 36 + self.hand_slot as i16,
        //         stack: stack.to_non_empty(),
        //     }));
        //
        //     self.drop_stack(world, stack.with_size(1), false);
        //
        // }
    }

    cache.apply();
}

#[spacetimedb(reducer)]
fn handle_position(entity_id: u32, packet: StdbPositionPacket) {
    let mut player = StdbServerPlayer::filter_by_entity_id(&entity_id).expect(
        format!("Could not find player with id: {}", entity_id).as_str());
    player.handle_position_look_inner(Some(packet.pos), None, packet.on_ground);
    log::info!("Updated Player position connection_id: {} username: {}", player.connection_id, player.username);
}

/// Handle a look packet.
#[spacetimedb(reducer)]
fn handle_look(entity_id: u32, packet: StdbLookPacket) {
    let mut player = StdbServerPlayer::filter_by_entity_id(&entity_id).expect(
        format!("Could not find player with id: {}", entity_id).as_str());
    player.handle_position_look_inner(None, Some(packet.look), packet.on_ground);
    log::info!("Updated Player look connection_id: {} username: {}", player.connection_id, player.username);
}

/// Handle a position and look packet.
#[spacetimedb(reducer)]
fn handle_position_look(entity_id: u32, packet: StdbPositionLookPacket) {
    let mut player = StdbServerPlayer::filter_by_entity_id(&entity_id).expect(
        format!("Could not find player with id: {}", entity_id).as_str());
    player.handle_position_look_inner(Some(packet.pos), Some(packet.look), packet.on_ground);
    log::info!("Updated Player position and look: connection_id {} username: {}", player.connection_id, player.username);
}