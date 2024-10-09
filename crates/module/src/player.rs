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

use std::collections::HashSet;

use glam::{DVec3, Vec2, IVec3};
use spacetimedb::{query, spacetimedb, SpacetimeType};
use mc173_module::chunk;
use mc173_module::dvec3::StdbDVec3;
use mc173_module::i32vec3::StdbI32Vec3;
use mc173_module::stdb::chunk::{StdbChunk, StdbChunkView};
use mc173_module::vec2::StdbVec2;
use crate::generate_chunk;
use crate::player::StdbClientState::Playing;
use crate::proto::{StdbLookPacket, StdbPositionLookPacket, StdbPositionPacket};

/// A server player is an actual
#[spacetimedb(table(public))]
#[derive(Clone)]
pub struct StdbServerPlayer {
    /// The network handle for the network server.
    // net: Network,
    /// The network client used to send packets through the network to that player.
    // pub client: NetworkClient,
    /// The entity id this player is controlling.
    #[primarykey]
    #[autoinc]
    pub entity_id: u32,
    #[unique]
    pub connection_id: u64,
    /// The username of that player.
    pub username: String,

    // TODO: possibly remove this later if we don't need it. For now this connection ID is used
    //  by the translation layer to associate a StdbServerPlayer with a physical network connection.
    pub spawn_pos: StdbDVec3,
    //// Set of chunks that are already sent to the player.
    // pub tracked_chunks: HashSet<(i32, i32)>,
    //// Set of tracked entities by this player, all entity ids in this set are considered
    //// known and rendered by the client, when the entity will disappear, a kill packet
    //// should be sent.
    // pub tracked_entities: HashSet<u32>,
    //// The main player inventory including the hotbar in the first 9 slots.
    // main_inv: Box<[ItemStack; 36]>,
    //// The armor player inventory.
    // armor_inv: Box<[ItemStack; 4]>,
    //// The item stacks for the 3x3 crafting grid. Also support the 2x2 as top left slots.
    // craft_inv: Box<[ItemStack; 9]>,
    //// The item stack in the cursor of the client's using a window.
    // cursor_stack: ItemStack,
    //// The slot current selected for the hand. Must be in range 0..9.
    // hand_slot: u8,
    //// The total number of windows that have been opened by this player, this is also
    //// used to generate a unique window id. This id should never be zero because it is
    //// reserved for the player inventory.
    // window_count: u32,
    //// The current window opened on the client side. Note that the player inventory is
    //// not registered here while opened because we can't know when it is. However we
    //// know that its window id is always 0.
    // window: Window,
    //// This crafting tracker is used to update the current craft being constructed by
    //// the player in the current window (player inventory or crafting table interface).
    // craft_tracker: CraftTracker,
    //// If the player is breaking a block, this record the breaking state.
    // breaking_block: Option<BreakingBlock>,
}

#[spacetimedb(table(public))]
pub struct StdbOfflineServerPlayer {
    #[primarykey]
    pub connection_id: u64,
    pub username: String,
    pub player: StdbServerPlayer,
}

#[spacetimedb(table(public))]
pub struct StdbTrackedPlayer {
    #[primarykey]
    #[autoinc]
    pub track_id: u32,
    // The entity ID of the player who is tracking
    pub from_id: u32,
    // The entity ID of the player who is being tracked
    pub to_id: u32,
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdbClientState {
    /// This client is not yet connected to the world.
    Handshaking,
    /// This client is actually playing into a world.
    Playing(StdbPlayingState),
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdbPlayingState {
    /// Index of the world this player is in.
    pub dimension_id: i32,
    /// Index of the player within the server world.
    pub entity_id: u32,
}

#[spacetimedb(table(public))]
pub struct StdbConnectionStatus {
    #[unique]
    pub connection_id: u64,
    pub status: StdbClientState,
}

// TODO(jdetter): This will be removed when we actually implement entities
#[spacetimedb(table(public))]
#[derive(Clone)]
pub struct StdbEntity {
    #[autoinc]
    #[primarykey]
    pub entity_id: u32,
    // TODO: This should be part of proper entities
    pub on_ground: bool,
    /// Last position sent by the client.
    pub pos: StdbDVec3,
    /// Last look sent by the client.
    pub look: StdbVec2,
    /// The dimension in which this entity lives
    pub dimension_id: i32,
}

#[derive(SpacetimeType)]
pub enum StdbWindowKind {
    /// The player inventory is the default window that is always opened if no other
    /// window is opened, it also always has the id 0, it contains the armor and craft
    /// matrix.
    Player,
    /// The client-side has a crafting table window opened on the given block pos.
    CraftingTable,
    /// The client-side has a chest window opened referencing the listed block entities.
    Chest,
    /// The client-side has a furnace window onto the given block entity.
    Furnace,
    /// The client-side has a dispenser window onto the given block entity.
    Dispenser,
}

#[spacetimedb(table(public))]
pub struct StdbPlayerWindow {
    pub player_id: u32,
    pub kind: StdbWindowKind,
    pub pos: StdbI32Vec3,
}

#[spacetimedb(table(public))]
pub struct StdbPlayerWindowChest {
    pub player_id: u32,
    pub pos: Vec<StdbI32Vec3>,
}

impl StdbServerPlayer {
    /// Handle a position packet.
    pub fn handle_position(self, packet: StdbPositionPacket) {
        self.handle_position_look_inner(Some(packet.pos), None, packet.on_ground);
    }

    /// Handle a look packet.
    pub fn handle_look(self, packet: StdbLookPacket) {
        self.handle_position_look_inner(None, Some(packet.look), packet.on_ground);
    }

    /// Handle a position and look packet.
    pub fn handle_position_look(self, packet: StdbPositionLookPacket) {
        self.handle_position_look_inner(Some(packet.pos), Some(packet.look), packet.on_ground);
    }

    pub fn handle_position_look_inner(&self, pos: Option<StdbDVec3>, look: Option<StdbVec2>, on_ground: bool) {
        let mut entity = StdbEntity::filter_by_entity_id(&self.entity_id).expect(
            format!("Could not find player with id: {}", self.entity_id).as_str());
        // let entity = world.get_entity_mut(self.entity_id).expect("incoherent player entity");
        entity.on_ground = on_ground;

        if let Some(pos) = pos {
            entity.pos = pos;
            // TODO(jdetter): Add entity teleportation to players
            // entity.teleport(pos);
        }

        if let Some(look) = look {
            entity.look = StdbVec2 {
                x: look.x.to_radians(),
                y: look.y.to_radians()
            };
            // TODO(jdetter): Add entity look to players
            // entity.0.look = self.look;
        }

        StdbEntity::update_by_entity_id(&entity.entity_id, entity.clone());
        Self::update_chunks(entity.entity_id);

        // if pos.is_some() {
        //     world.push_event(Event::Entity { id: self.entity_id, inner: EntityEvent::Position { pos: self.pos } });
        //     self.update_chunks(world);
        // }
        //
        // if look.is_some() {
        //     world.push_event(Event::Entity { id: self.entity_id, inner: EntityEvent::Look { look: self.look } });
        // }
    }

    /// Update the chunks sent to this player.
    pub fn update_chunks(player_id: u32) {
        let player_entity = StdbEntity::filter_by_entity_id(&player_id).unwrap();
        let (ocx, ocz) = chunk::calc_entity_chunk_pos(player_entity.pos.as_dvec3());
        let view_range = 2;

        for cx in (ocx - view_range)..(ocx + view_range) {
            for cz in (ocz - view_range)..(ocz + view_range) {
                let chunk_id = StdbChunk::xz_to_chunk_id(cx, cz);
                let chunk = StdbChunk::filter_by_chunk_id(&chunk_id).unwrap_or_else(|| {
                    generate_chunk(cx, cz);
                    StdbChunk::filter_by_chunk_id(&chunk_id).unwrap()
                });

                if query!(|view: StdbChunkView| view.chunk_id == chunk_id && view.observer_id == player_id).next().is_none() {
                    let _ = StdbChunkView::insert(StdbChunkView {
                        view_id: 0,
                        chunk_id,
                        observer_id: player_id,
                    });
                }

                // if let Some(chunk) = world.get_chunk(cx, cz) {
                //     if self.tracked_chunks.insert((cx, cz)) {
                //
                //         self.send(OutPacket::ChunkState(proto::ChunkStatePacket {
                //             cx, cz, init: true
                //         }));
                //
                //         let from = IVec3 {
                //             x: cx * 16,
                //             y: 0,
                //             z: cz * 16,
                //         };
                //
                //         let size = IVec3 {
                //             x: 16,
                //             y: 128,
                //             z: 16,
                //         };
                //
                //         self.send(OutPacket::ChunkData(new_chunk_data_packet(chunk, from, size)));
                //
                //     }
                // }

            }
        }
    }
}
