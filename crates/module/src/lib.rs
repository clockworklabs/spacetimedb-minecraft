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

use glam::IVec3;
use mc173_module::gen::{ChunkGenerator, OverworldGenerator};
use spacetimedb::{spacetimedb};
use mc173_module::block;
use mc173_module::chunk::{calc_chunk_pos, Chunk};
use mc173_module::world::{BlockEvent, ChunkEvent, Event, LightKind};

#[spacetimedb(table)]
pub struct StdbChunk {
    #[primarykey]
    #[autoinc]
    chunk_id: i32,
    x: i32,
    z: i32,
    chunk: Chunk,
}

/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

#[spacetimedb(init)]
pub fn init() {
    log::info!("Starting Generation");
    generate_chunks(-5, -5, 5, 5);
    log::info!("Generation complete");
}

#[spacetimedb(reducer)]
pub fn generate_chunk(x: i32, z: i32) {
    let generator = OverworldGenerator::new(SEED);
    let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
    let mut chunk = Chunk::new_no_arc();
    generator.gen_terrain(x, z, &mut chunk, &mut state);
    if let Err(_) = StdbChunk::insert(StdbChunk {
        chunk_id: 0, x, z, chunk,
    }) {
        log::error!("Failed to insert unique chunk");
    };
    log::info!("Chunk Generated: {}, {}", x, z);
}

#[spacetimedb(reducer)]
pub fn generate_chunks(from_x: i32, from_z: i32, to_x: i32, to_z: i32) {
    let handle = spacetimedb::time_span::Span::start("spacetimedb chunk generation func");
    let generator = OverworldGenerator::new(SEED);
    let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
    for x in from_x..to_x {
        for z in from_z..to_z {
            let handle = spacetimedb::time_span::Span::start("spacetimedb individual chunk");
            if StdbChunk::filter_by_x(&x).find(|mz| mz.z == z).is_some() {
                log::info!("Chunk Skipped: {}, {}", x, z);
                continue;
            }

            let mut chunk = Chunk::new_no_arc();
            generator.gen_terrain(x, z, &mut chunk, &mut state);
            log::info!("Chunk Generated: {}, {}", x, z);

            if let Err(_) = StdbChunk::insert(StdbChunk {
                chunk_id: 0, x, z, chunk,
            }) {
                log::error!("Failed to insert unique chunk");
            };
        }
    }
}

/*pub fn break_block(pos_x: i32, pos_y: i32, pos_z: i32) -> Option<(u8, u8)> {
    let (prev_id, prev_metadata) = self.set_block_notify(pos, block::AIR, 0)?;
    self.spawn_block_loot(pos, prev_id, prev_metadata, 1.0);
    Some((prev_id, prev_metadata))
}*/

#[spacetimedb(reducer)]
pub fn chop_terrain(pos_x: i32, pos_y: i32, pos_z: i32, size: i32) {
    log::info!("Calling chop_terrain: {} {} {}", pos_x, pos_y, pos_z);
    for x in pos_x..(pos_x + size) {
        for z in pos_z..(pos_z + size) {
            for y in pos_y..128 {
                set_block(x, y, z, 0, 0);
            }
        }
    }
}

#[spacetimedb(reducer)]
pub fn set_block(pos_x: i32, pos_y: i32, pos_z: i32, id: u8, metadata: u8) {
    let pos = IVec3::new(pos_x, pos_y, pos_z);

    let (cx, cz) = calc_chunk_pos(pos).unwrap();
    let mut chunk = StdbChunk::filter_by_x(&cx).find(|c| c.z == cz).unwrap();
    let (prev_id, prev_metadata) = chunk.chunk.get_block(pos);

    if id != prev_id || metadata != prev_metadata {
        chunk.chunk.set_block(pos, id, metadata);
        chunk.chunk.recompute_height(pos);

        // Schedule light updates if the block light properties have changed.
        /*if block::material::get_light_opacity(id) != block::material::get_light_opacity(prev_id)
            || block::material::get_light_emission(id) != block::material::get_light_emission(prev_id) {
            self.schedule_light_update(pos, LightKind::Block);
            self.schedule_light_update(pos, LightKind::Sky);
        }*/

/*        self.push_event(Event::Block {
            pos,
            inner: BlockEvent::Set {
                id,
                metadata,
                prev_id,
                prev_metadata,
            }
        });*/

        // self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
        log::info!("Set Block: {} {} {}", pos_x, pos_y, pos_z);

        let chunk_id = chunk.chunk_id;
        StdbChunk::update_by_chunk_id(&chunk_id, chunk);
    }

    // Some((prev_id, prev_metadata))
}

fn handle_break_block(&mut self, world: &mut World, packet: proto::BreakBlockPacket) {

    let face = match packet.face {
        0 => Face::NegY,
        1 => Face::PosY,
        2 => Face::NegZ,
        3 => Face::PosZ,
        4 => Face::NegX,
        5 => Face::PosX,
        _ => return,
    };

    let Some(entity) = world.get_entity_mut(self.entity_id) else { return };
    let pos = IVec3::new(packet.x, packet.y as i32, packet.z);

    tracing::trace!("packet: {packet:?}");
    // TODO: Use server time for breaking blocks.

    let in_water = entity.0.in_water;
    let on_ground = entity.0.on_ground;
    let mut stack = self.main_inv[self.hand_slot as usize];

    if packet.status == 0 {

        // Special case to extinguish fire.
        if world.is_block(pos + face.delta(), block::FIRE) {
            world.set_block_notify(pos + face.delta(), block::AIR, 0);
        }

        // We ignore any interaction result for the left click (break block) to
        // avoid opening an inventory when breaking a container.
        // NOTE: Interact before 'get_block': relevant for redstone_ore lit.
        world.interact_block(pos);

        // Start breaking a block, ignore if the position is invalid.
        if let Some((id, _)) = world.get_block(pos) {

            let break_duration = world.get_break_duration(stack.id, id, in_water, on_ground);
            if break_duration.is_infinite() {
                // Do nothing, the block is unbreakable.
            } else if break_duration == 0.0 {
                world.break_block(pos);
            } else {
                self.breaking_block = Some(BreakingBlock {
                    start_time: world.get_time(), // + (break_duration * 0.7) as u64,
                    pos,
                    id,
                });
            }

        }

    } else if packet.status == 2 {
        // Block breaking should be finished.
        if let Some(state) = self.breaking_block.take() {
            if state.pos == pos && world.is_block(pos, state.id) {
                let break_duration = world.get_break_duration(stack.id, state.id, in_water, on_ground);
                let min_time = state.start_time + (break_duration * 0.7) as u64;
                if world.get_time() >= min_time {
                    world.break_block(pos);
                } else {
                    warn!("from {}, incoherent break time, expected {min_time} but got {}", self.username, world.get_time());
                }
            } else {
                warn!("from {}, incoherent break position, expected  {}, got {}", self.username, pos, state.pos);
            }
        }
    } else if packet.status == 4 {
        // Drop the selected item.

        if !stack.is_empty() {

            stack.size -= 1;
            self.main_inv[self.hand_slot as usize] = stack.to_non_empty().unwrap_or_default();

            self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                window_id: 0,
                slot: 36 + self.hand_slot as i16,
                stack: stack.to_non_empty(),
            }));

            self.drop_stack(world, stack.with_size(1), false);

        }

    }

}
