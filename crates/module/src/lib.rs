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

use std::time::Duration;
use glam::{DVec3, IVec3};
use mc173_module::world::{Dimension, StdbWorld, World};
use spacetimedb::{ReducerContext, schedule, spacetimedb, SpacetimeType, Timestamp};
use mc173_module::{block, item};
use mc173_module::chunk::calc_entity_chunk_pos;
use mc173_module::geom::Face;
use mc173_module::stdb::chunk::{StdbBreakBlockPacket, BreakingBlock, StdbBreakingBlock, StdbTime};
use mc173_module::stdb::weather::StdbWeather;
use mc173_module::storage::ChunkStorage;

/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

/// Represent the whole state of a world.
#[spacetimedb(table)]
pub struct StdbServerWorldState {
    #[primarykey]
    pub world_id: i32,
    /// World name.
    pub name: String,
    /// The seed of this world, this is sent to the client in order to
    pub seed: i64,
    /// The server-side time, that is not necessarily in-sync with the world time in case
    /// of tick freeze or stepping. This avoids running in socket timeout issues.
    pub time: u64,
    /// True when world ticking is frozen, events are still processed by the world no
    /// longer runs.
    pub tick_mode: TickMode,
    pub tick_mode_manual: u32,
    //// The chunk source used to load and save the world's chunk.
    // storage: mc173_module::storage::ChunkStorage,
    //// Chunks trackers used to send proper block changes packets.
    // chunk_trackers: ChunkTrackers,
    //// Entity tracker, each is associated to the entity id.
    // entity_trackers: HashMap<u32, EntityTracker>,
    //// Instant of the last tick.
    // tick_last: Instant,
    //// Fading average tick duration, in seconds.
    // pub tick_duration: FadingAverage,
    //// Fading average interval between two ticks.
    // pub tick_interval: FadingAverage,
    //// Fading average of events count on each tick.
    // pub events_count: FadingAverage,
}

/// Indicate the current mode for ticking the world.
#[derive(SpacetimeType)]
pub enum TickMode {
    /// The world is ticked on each server tick (20 TPS).
    Auto,
    /// The world if ticked on each server tick (20 TPS), but the counter decrease and
    /// it is no longer ticked when reaching 0.
    // Manual(u32),
    Manual
}

#[spacetimedb(init)]
pub fn init(context: ReducerContext) {
    let nano_time = context.timestamp.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_nanos();
    // log::info!("Starting Generation");
    // generate_chunks(-5, -5, 5, 5);
    // log::info!("Generation complete");

    let new_world = StdbWorld::insert(StdbWorld {
        id: 0,
        world: World::new(Dimension::Overworld, nano_time),
    }).unwrap();

    StdbServerWorldState::insert(StdbServerWorldState {
        world_id: new_world.id,
        name: "Boppy's World".to_string(),
        seed: 9999,
        time: 0,
        tick_mode: TickMode::Auto,
        tick_mode_manual: 0,
    }).unwrap();

    StdbTime::insert(StdbTime { id: 0, time: 0 }).unwrap();
    mc173_module::stdb::weather::init();
    mc173_module::stdb::rand::init(nano_time);

    // This has to be here because this is how we schedule tick
    // Do the very fist tick
    tick();
}


#[spacetimedb(reducer)]
pub fn tick() {
    // Do stuff...
    // Lastly, tick time
    for mut world in StdbWorld::iter() {
        let mut state = StdbServerWorldState::filter_by_world_id(&world.id).unwrap();
        tick_world(&mut world, &mut state);
        let world_id = world.id;
        StdbWorld::update_by_id(&world_id, world);
        StdbServerWorldState::update_by_world_id(&world_id, state);
    }

    // reschedule self
    schedule!(Duration::from_millis(50), tick());
}

/// Tick this world.
pub fn tick_world(world: &mut StdbWorld, state: &mut StdbServerWorldState) {

    // Get server-side time.
    let time = state.time;
    if time == 0 {
        init_world(world, state);
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
        TickMode::Auto => {
            world.world.tick();
        }
        TickMode::Manual => {
            let mut n = state.tick_mode_manual;
            if n != 0 {
                world.world.tick();
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
    // self.state.chunk_trackers.update_players(&self.players, &self.world);

    // After world events are processed, tick entity trackers.
    // for tracker in self.state.entity_trackers.values_mut() {
    //     if time % 60 == 0 {
    //         tracker.update_tracking_players(&mut self.players, &self.world);
    //     }
    //     tracker.tick_and_update_players(&self.players);
    // }

    // Drain dirty chunks coordinates and save them.
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
fn init_world(world: &mut StdbWorld, state: &mut StdbServerWorldState) {

    // // Ensure that every entity has a tracker.
    // for (id, entity) in self.world.iter_entities() {
    //     self.state.entity_trackers.entry(id).or_insert_with(|| {
    //         let tracker = EntityTracker::new(id, entity);
    //         tracker.update_tracking_players(&mut self.players, &self.world);
    //         tracker
    //     });
    // }

    // NOTE: Temporary code.
    let size = 1;
    let (center_cx, center_cz) = calc_entity_chunk_pos(DVec3::new(0.0, 100.0, 0.0));
    for cx in center_cx - size..=center_cx + size {
        for cz in center_cz - size..=center_cz + size {
            ChunkStorage::request_load(world, cx, cz);
        }
    }
}

// #[spacetimedb(reducer)]
// pub fn generate_chunks(from_x: i32, from_z: i32, to_x: i32, to_z: i32) {
//     let handle = spacetimedb::time_span::Span::start("spacetimedb chunk generation func");
//     let generator = OverworldGenerator::new(SEED);
//     let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
//     for x in from_x..to_x {
//         for z in from_z..to_z {
//             let handle = spacetimedb::time_span::Span::start("spacetimedb individual chunk");
//             if StdbChunk::filter_by_x(&x).find(|mz| mz.z == z).is_some() {
//                 log::info!("Chunk Skipped: {}, {}", x, z);
//                 continue;
//             }
//
//             let mut chunk = Chunk::new_no_arc();
//             generator.gen_terrain(x, z, &mut chunk, &mut state);
//             log::info!("Chunk Generated: {}, {}", x, z);
//
//             if let Err(_) = StdbChunk::insert(StdbChunk {
//                 chunk_id: 0,
//                 x,
//                 z,
//                 chunk,
//             }) {
//                 log::error!("Failed to insert unique chunk");
//             };
//         }
//     }
// }



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

// #[spacetimedb(reducer)]
// pub fn generate_chunk(x: i32, z: i32) {
//     let generator = OverworldGenerator::new(SEED);
//     let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
//     let mut chunk = Chunk::new_no_arc();
//     generator.gen_terrain(x, z, &mut chunk, &mut state);
//     if let Err(_) = StdbChunk::insert(StdbChunk {
//         chunk_id: 0,
//         x,
//         z,
//         chunk,
//     }) {
//         log::error!("Failed to insert unique chunk");
//     };
//     log::info!("Chunk Generated: {}, {}", x, z);
// }



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
//     let mut chunk = StdbChunk::filter_by_x(&cx).find(|c| c.z == cz).unwrap();
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
fn handle_break_block(entity_id: u32, packet: StdbBreakBlockPacket) {

    // TODO(jdetter): replace this when we migrate entities
    let username = "Boppy";

    // NOTE: Instead of just grabbing an arbirary world, we should use the world that the player is in
    let mut world = StdbWorld::filter_by_id(&1).unwrap();

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
    log::info!("Breaking block: {} {} {}", packet.x, packet.y, packet.z);
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
        if world.world.is_block(pos + face.delta(), block::FIRE) {
            world.world.set_block_notify(pos + face.delta(), block::AIR, 0);
        }

        // We ignore any interaction result for the left click (break block) to
        // avoid opening an inventory when breaking a container.
        // NOTE: Interact before 'get_block': relevant for redstone_ore lit.
        // world.world.interact_block(pos);

        // Start breaking a block, ignore if the position is invalid.
        if let Some((id, _)) = world.world.get_block(pos) {

            // let break_duration = world.get_break_duration(stack.id, id, in_water, on_ground);
            let break_duration = world.world.get_break_duration(hand_item, id, in_water, on_ground);
            if break_duration.is_infinite() {
                // Do nothing, the block is unbreakable.
            } else if break_duration == 0.0 {
                world.world.break_block(pos);
            } else {
                // self.breaking_block = Some(BreakingBlock {
                //     start_time: world.get_time(), // + (break_duration * 0.7) as u64,
                //     pos,
                //     id,
                // });
                let new_breaking_block = StdbBreakingBlock {
                    entity_id: entity_id,
                    state: BreakingBlock {
                        start_time: world.world.get_time(), // + (break_duration * 0.7) as u64,
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
            if <mc173_module::ivec3::StdbIVec3 as Into<IVec3>>::into(some_breaking_block.state.pos) == pos && world.world.is_block(pos, some_breaking_block.state.id) {
                // let break_duration = world.world.get_break_duration(stack.id, breaking_block.id, in_water, on_ground);
                let break_duration = world.world.get_break_duration(hand_item, some_breaking_block.state.id, in_water, on_ground);
                let min_time = some_breaking_block.state.start_time + (break_duration * 0.7) as u64;
                if world.world.get_time() >= min_time {
                    world.world.break_block(pos);
                } else {

                    log::warn!("from {}, incoherent break time, expected {min_time} but got {}", username, world.world.get_time());

                }
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
}



