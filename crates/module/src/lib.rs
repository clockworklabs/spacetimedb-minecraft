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

mod world;

use std::collections::HashMap;
use std::time::{Duration, Instant, UNIX_EPOCH};
use glam::{DVec3, IVec3};
use mc173_module::{block, chunk, item};
use mc173_module::chunk::{calc_chunk_pos, Chunk};
use mc173_module::gen::{ChunkGenerator, OverworldGenerator};
use mc173_module::world::{BlockEvent, ChunkEvent, Dimension, Event, LightKind, StdbWorld, World};
use spacetimedb::{ReducerContext, schedule, spacetimedb, SpacetimeType, Timestamp};
use spacetimedb::rt::ReducerInfo;
use mc173_module::block::material::Material;
use mc173_module::geom::Face;
use mc173_module::stdb::chunk::BreakBlockPacket;
use mc173_module::stdb::chunk::StdbBreakingBlock;
use mc173_module::stdb::chunk::StdbTime;
use mc173_module::stdb::chunk::StdbChunk;

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
    /// The chunk source used to load and save the world's chunk.
    // storage: ChunkStorage,
    /// Chunks trackers used to send proper block changes packets.
    // chunk_trackers: ChunkTrackers,
    /// Entity tracker, each is associated to the entity id.
    // entity_trackers: HashMap<u32, EntityTracker>,
    /// Instant of the last tick.
    // tick_last: Instant,
    /// Fading average tick duration, in seconds.
    // pub tick_duration: FadingAverage,
    /// Fading average interval between two ticks.
    // pub tick_interval: FadingAverage,
    /// Fading average of events count on each tick.
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
    }).unwrap();

    StdbTime::insert(StdbTime { id: 0, time: 0 }).unwrap();
    mc173_module::stdb::weather::init();
    mc173_module::stdb::rand::init(nano_time);

    // This has to be here because this is how we schedule tick
    // Do the very fist tick
    tick(context);
}


#[spacetimedb(reducer)]
pub fn tick(context: ReducerContext) {
    // Do stuff...
    // Lastly, tick time
    let nano_time = context.timestamp.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_nanos();
    for world in StdbWorld::iter() {
        tick_world(world.id, nano_time);
        let world_id = world.id;
        StdbWorld::update_by_id(&world_id, world);
    }

    // reschedule self
    schedule!(Duration::from_millis(50), tick());
}

/// Tick this world.
pub fn tick_world(world_id: i32, nano_time: u128) {
    let state = StdbServerWorldState::filter_by_world_id(&world_id).unwrap();
    let world = StdbWorld::filter_by_id(&world_id).unwrap();

    // Get server-side time.
    let time = state.time;
    if time == 0 {
        init_world(world_id: i32, nano_time: u128);
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
            world.tick()
        }
        TickMode::Manual => {
            let mut n = state.tick_mode_manual;
            if n != 0 {
                world.tick();
            }
            *n -= 1;
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
fn init_world(world_id: i32, nano_time: u128) {

    // // Ensure that every entity has a tracker.
    // for (id, entity) in self.world.iter_entities() {
    //     self.state.entity_trackers.entry(id).or_insert_with(|| {
    //         let tracker = EntityTracker::new(id, entity);
    //         tracker.update_tracking_players(&mut self.players, &self.world);
    //         tracker
    //     });
    // }

    // NOTE: Temporary code.
    let (center_cx, center_cz) = chunk::calc_entity_chunk_pos(DVec3::new(0.0, 100.0, 0.0));
    for cx in center_cx - 10..=center_cx + 10 {
        for cz in center_cz - 10..=center_cz + 10 {
            state.storage.request_load(cx, cz);
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

// #[spacetimedb(reducer)]
// fn handle_break_block(entity_id: u32, packet: BreakBlockPacket) {
    //
    // let face = match packet.face {
    //     0 => Face::NegY,
    //     1 => Face::PosY,
    //     2 => Face::NegZ,
    //     3 => Face::PosZ,
    //     4 => Face::NegX,
    //     5 => Face::PosX,
    //     _ => return,
    // };
    //
    // // let Some(entity) = world.get_entity_mut(self.entity_id) else { return; };
    // let pos = IVec3::new(packet.x, packet.y as i32, packet.z);
    // let (cx, cz) = calc_chunk_pos(pos).unwrap();
    // let chunk = StdbChunk::filter_by_x(&cx).find(|c|c.z == cz).unwrap();
    // let breaking_block = StdbBreakingBlock::filter_by_entity_id(&entity_id);
    // let time = StdbTime::filter_by_id(&0).unwrap().time;
    //
    // // tracing::trace!("packet: {packet:?}");
    // // TODO: Use server time for breaking blocks.
    //
    // // let in_water = entity.0.in_water;
    // // let on_ground = entity.0.on_ground;
    // // let mut stack = self.main_inv[self.hand_slot as usize];
    //
    // if packet.status == 0 {
    //
    //     // Special case to extinguish fire.
    //     // if chunk.is_block(pos + face.delta(), block::FIRE) {
    //     //     world.set_block_notify(pos + face.delta(), block::AIR, 0);
    //     // }
    //
    //     // We ignore any interaction result for the left click (break block) to
    //     // avoid opening an inventory when breaking a container.
    //     // NOTE: Interact before 'get_block': relevant for redstone_ore lit.
    //     // world.interact_block(pos);
    //
    //     // Start breaking a block, ignore if the position is invalid.
    //     if let Some((id, _)) = chunk.chunk.get_block(pos) {
    //         let breaking_pos = IVec3 {
    //             x: packet.x,
    //             y: packet.y as i32,
    //             z: packet.z,
    //         };
    //         // let break_duration = get_break_duration(stack.id, id, in_water, on_ground);
    //         let break_duration = get_break_duration(0, id, false, true);
    //         if break_duration.is_infinite() {
    //             // Do nothing, the block is unbreakable.
    //         } else if break_duration == 0.0 {
    //             set_block(breaking_pos.x, breaking_pos.y, breaking_pos.z, 0, 0);
    //         } else {
    //             let new_breaking_block = mc173_module::stdb::chunk::BreakingBlock {
    //                 start_time: time,
    //                 pos_x: pos.x,
    //                 pos_y: pos.y,
    //                 pos_z: pos.z,
    //                 id,
    //             };
    //
    //             match breaking_block {
    //                 None => {
    //                     StdbBreakingBlock::insert(StdbBreakingBlock {
    //                         entity_id,
    //                         state: new_breaking_block,
    //                     }).unwrap();
    //                 },
    //                 Some(_) => {
    //                     StdbBreakingBlock::update_by_entity_id(&entity_id, StdbBreakingBlock {
    //                         entity_id,
    //                         state: new_breaking_block,
    //                     });
    //                 }
    //             }
    //         }
    //     }
    // } else if packet.status == 2 {
    //     if let Some(breaking_block) = breaking_block {
    //         StdbBreakingBlock::delete_by_entity_id(&entity_id);
    //
    //         // Block breaking should be finished.
    //         let breaking_pos = IVec3 {
    //             x: breaking_block.state.pos_x,
    //             y: breaking_block.state.pos_y,
    //             z: breaking_block.state.pos_z,
    //         };
    //
    //         if breaking_pos == pos && chunk.is_block(pos, breaking_block.state.id) {
    //             // let break_duration = get_break_duration(stack.id, state.id, in_water, on_ground);
    //             let break_duration = get_break_duration(0, breaking_block.state.id, false, true);
    //             let min_time = breaking_block.state.start_time + (break_duration * 0.7) as u64;
    //             if time >= min_time {
    //                 set_block(breaking_pos.x, breaking_pos.y, breaking_pos.z, 0, 0);
    //                 // world.break_block(pos);
    //             } else {
    //                 // log::warn!("from {}, incoherent break time, expected {min_time} but got {}", self.username, world.get_time());
    //                 log::warn!("from {entity_id}, incoherent break time, expected {min_time} but got {}", time);
    //             }
    //         } else {
    //             // log::warn!("from {}, incoherent break position, expected  {}, got {}", self.username, pos, state.pos);
    //             log::warn!("from {entity_id}, incoherent break position, expected  {pos}, got {breaking_pos}");
    //         }
    //     }
    // } else if packet.status == 4 {
    //     // TODO: find out what this is? Do we need it?
    //     // Drop the selected item.
    //     //
    //     // if !stack.is_empty() {
    //     //     stack.size -= 1;
    //     //     self.main_inv[self.hand_slot as usize] = stack.to_non_empty().unwrap_or_default();
    //     //
    //     //     self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
    //     //         window_id: 0,
    //     //         slot: 36 + self.hand_slot as i16,
    //     //         stack: stack.to_non_empty(),
    //     //     }));
    //     //
    //     //     self.drop_stack(world, stack.with_size(1), false);
    //     // }
    // }
// }

// pub fn get_break_duration(item_id: u16, block_id: u8, in_water: bool, on_ground: bool) -> f32 {
//
//     // TODO: Maybe remove hardness from the block definition, because it's only used in
//     // the game for break duration.
//
//     let hardness = block::material::get_break_hardness(block_id);
//     if hardness.is_infinite() {
//         f32::INFINITY
//     } else {
//
//         // The hardness value in the game is registered as ticks, with a multiplier
//         // depending on the player's conditions and tools.
//
//         if can_break(item_id, block_id) {
//
//             let mut env_modifier = get_break_speed(item_id, block_id);
//
//             if in_water {
//                 env_modifier /= 5.0;
//             }
//
//             if !on_ground {
//                 env_modifier /= 5.0;
//             }
//
//             hardness * 30.0 / env_modifier
//
//         } else {
//             hardness * 100.0
//         }
//
//     }
//
// }

// /// Check if an item (given its id) can break a block without speed penalties and
// /// loose the items.
// fn can_break(item_id: u16, block_id: u8) -> bool {
//
//     match block_id {
//         block::OBSIDIAN => matches!(item_id,
//                 item::DIAMOND_PICKAXE),
//         block::DIAMOND_ORE |
//         block::DIAMOND_BLOCK |
//         block::GOLD_ORE |
//         block::GOLD_BLOCK |
//         block::REDSTONE_ORE |
//         block::REDSTONE_ORE_LIT => matches!(item_id,
//                 item::DIAMOND_PICKAXE |
//                 item::IRON_PICKAXE),
//         block::IRON_ORE |
//         block::IRON_BLOCK |
//         block::LAPIS_ORE |
//         block::LAPIS_BLOCK => matches!(item_id,
//                 item::DIAMOND_PICKAXE |
//                 item::IRON_PICKAXE |
//                 item::STONE_PICKAXE),
//         block::COBWEB => matches!(item_id,
//                 item::SHEARS |
//                 item::DIAMOND_SWORD |
//                 item::IRON_SWORD |
//                 item::STONE_SWORD |
//                 item::GOLD_SWORD |
//                 item::WOOD_SWORD),
//         block::SNOW |
//         block::SNOW_BLOCK => matches!(item_id,
//                 item::DIAMOND_SHOVEL |
//                 item::IRON_SHOVEL |
//                 item::STONE_SHOVEL |
//                 item::GOLD_SHOVEL |
//                 item::WOOD_SHOVEL),
//         _ => {
//
//             let material = block::material::get_material(block_id);
//             if material.is_breakable_by_default() {
//                 return true;
//             }
//
//             match item_id {
//                 item::DIAMOND_PICKAXE |
//                 item::IRON_PICKAXE |
//                 item::STONE_PICKAXE |
//                 item::GOLD_PICKAXE |
//                 item::WOOD_PICKAXE => matches!(material, Material::Rock | Material::Iron),
//                 _ => false
//             }
//
//         }
//     }
//
// }

// /// Get the speed multiplier for breaking a given block with a given item.
// fn get_break_speed(item_id: u16, block_id: u8) -> f32 {
//
//     const DIAMOND_SPEED: f32 = 8.0;
//     const IRON_SPEED: f32 = 6.0;
//     const STONE_SPEED: f32 = 4.0;
//     const WOOD_SPEED: f32 = 2.0;
//     const GOLD_SPEED: f32 = 12.0;
//
//     match block_id {
//         block::WOOD |
//         block::BOOKSHELF |
//         block::LOG |
//         block::CHEST => {
//             // Axe
//             match item_id {
//                 item::DIAMOND_AXE => DIAMOND_SPEED,
//                 item::IRON_AXE => IRON_SPEED,
//                 item::STONE_AXE => STONE_SPEED,
//                 item::WOOD_AXE => WOOD_SPEED,
//                 item::GOLD_AXE => GOLD_SPEED,
//                 _ => 1.0,
//             }
//         }
//         block::COBBLESTONE |
//         block::SLAB |
//         block::DOUBLE_SLAB |
//         block::STONE |
//         block::SANDSTONE |
//         block::MOSSY_COBBLESTONE |
//         block::IRON_ORE |
//         block::IRON_BLOCK |
//         block::GOLD_ORE |
//         block::GOLD_BLOCK |
//         block::COAL_ORE |
//         block::DIAMOND_ORE |
//         block::DIAMOND_BLOCK |
//         block::ICE |
//         block::NETHERRACK |
//         block::LAPIS_ORE |
//         block::LAPIS_BLOCK => {
//             // Pickaxe
//             match item_id {
//                 item::DIAMOND_PICKAXE => DIAMOND_SPEED,
//                 item::IRON_PICKAXE => IRON_SPEED,
//                 item::STONE_PICKAXE => STONE_SPEED,
//                 item::WOOD_PICKAXE => WOOD_SPEED,
//                 item::GOLD_PICKAXE => GOLD_SPEED,
//                 _ => 1.0,
//             }
//         }
//         block::GRASS |
//         block::DIRT |
//         block::SAND |
//         block::GRAVEL |
//         block::SNOW |
//         block::SNOW_BLOCK |
//         block::CLAY |
//         block::FARMLAND => {
//             // Shovel
//             match item_id {
//                 item::DIAMOND_SHOVEL => DIAMOND_SPEED,
//                 item::IRON_SHOVEL => IRON_SPEED,
//                 item::STONE_SHOVEL => STONE_SPEED,
//                 item::WOOD_SHOVEL => WOOD_SPEED,
//                 item::GOLD_SHOVEL => GOLD_SPEED,
//                 _ => 1.0,
//             }
//         }
//         block::COBWEB => {
//             match item_id {
//                 item::SHEARS |
//                 item::DIAMOND_SWORD |
//                 item::IRON_SWORD |
//                 item::STONE_SWORD |
//                 item::GOLD_SWORD |
//                 item::WOOD_SWORD => 15.0,
//                 _ => 1.0,
//             }
//         }
//         block::LEAVES => {
//             match item_id {
//                 item::SHEARS => 15.0,
//                 _ => 1.0,
//             }
//         }
//         _ => match item_id {
//             item::DIAMOND_SWORD |
//             item::IRON_SWORD |
//             item::STONE_SWORD |
//             item::GOLD_SWORD |
//             item::WOOD_SWORD => 1.5,
//             _ => 1.0,
//         }
//     }
//
// }


