//! Server world structure.

use std::collections::HashMap;
use std::fmt::format;
use std::time::Instant;

use glam::{DVec3, IVec3, Vec2};
use spacetimedb::{query, spacetimedb, SpacetimeType};
use mc173_module::block;
use mc173_module::chunk::calc_chunk_pos;
use mc173_module::chunk_cache::ChunkCache;
use mc173_module::geom::Face;
use mc173_module::inventory::StdbInventory;
use mc173_module::stdb::chunk::{ChunkUpdateType, StdbBlockSetUpdate, StdbChunk, StdbChunkUpdate};
use mc173_module::world::{LightKind, StdbWorld};
use mc173_module::world::interact::Interaction;
use crate::proto::{self, OutPacket};
use crate::config;
use crate::entity::{StdbEntityTracker, StdbEntityView};
use crate::player::{StdbConnectionStatus, StdbEntity, StdbServerPlayer, StdbTrackedPlayer};
/// A single world in the server, this structure keep tracks of players and entities
/// tracked by players.
// pub struct StdbServerWorld {
//     /// The inner world data structure.
//     pub world: StdbWorld,
//     /// Players currently in the world.
//     pub players: Vec<ServerPlayer>,
//     /// The remaining world state, this is put is a separate struct in order to facilitate
//     /// borrowing when handling player packets.
//     pub state: ServerWorldState,
// }

/// Represent the whole state of a world.
#[spacetimedb(table(public))]
pub struct StdbServerWorld {
    #[primarykey]
    pub dimension_id: i32,
    /// World name.
    pub name: String,
    /// The seed of this world, this is sent to the client in order to
    pub seed: i64,
    /// The server-side time, that is not necessarily in-sync with the world time in case
    /// of tick freeze or stepping. This avoids running in socket timeout issues.
    pub time: u64,
    /// True when world ticking is frozen, events are still processed by the world no
    /// longer runs.
    pub tick_mode: StdbTickMode,
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
pub enum StdbTickMode {
    /// The world is ticked on each server tick (20 TPS).
    Auto,
    /// The world if ticked on each server tick (20 TPS), but the counter decrease and
    /// it is no longer ticked when reaching 0.
    // Manual(u32),
    Manual
}

impl StdbServerWorld {

    ///// Internal function to create a server world.
    // pub fn new(name: impl Into<String>) -> Self {
    //
    //     let mut inner = World::new(Dimension::Overworld);
    //
    //     // Make sure that the world initially have an empty events queue.
    //     inner.swap_events(Some(Vec::new()));
    //
    //     let seed = config::SEED;
    //
    //     Self {
    //         world: inner,
    //         players: Vec::new(),
    //         state: ServerWorldState {
    //             name: name.into(),
    //             seed,
    //             time: 0,
    //             tick_mode: TickMode::Auto,
    //             storage: ChunkStorage::new("test_world/region/", OverworldGenerator::new(seed), 4),
    //             chunk_trackers: ChunkTrackers::new(),
    //             entity_trackers: HashMap::new(),
    //             tick_last: Instant::now(),
    //             tick_duration: FadingAverage::default(),
    //             tick_interval: FadingAverage::default(),
    //             events_count: FadingAverage::default(),
    //         },
    //     }
    // }

    ///// Save this world's resources and block until all resources has been saved.
    // pub fn save(&mut self) {
    //
    //     info!("saving {}...", self.state.name);
    //
    //     for (cx, cz) in self.state.chunk_trackers.drain_save() {
    //         if let Some(snapshot) = self.world.take_chunk_snapshot(cx, cz) {
    //             debug!("saving {} chunk: {cx}/{cz}", self.state.name);
    //             self.state.storage.request_save(snapshot);
    //         }
    //     }
    //
    //     while self.state.storage.request_save_count() != 0 {
    //         if let Some(reply) = self.state.storage.poll() {
    //             match reply {
    //                 ChunkStorageReply::Save { cx, cz, res: Ok(()) } => {
    //                     debug!("saved chunk in storage: {cx}/{cz}");
    //                 }
    //                 ChunkStorageReply::Save { cx, cz, res: Err(err) } => {
    //                     debug!("failed to save chunk in storage: {cx}/{cz}: {err}");
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    //
    // }

    // /// Tick this world.
    // pub fn tick(&mut self) {
    //
    //     let start = Instant::now();
    //     self.state.tick_interval.push((start - self.state.tick_last).as_secs_f32(), 0.02);
    //     self.state.tick_last = start;
    //
    //     // Get server-side time.
    //     let time = self.state.time;
    //     if time == 0 {
    //         self.init();
    //     }
    //
    //     // Poll all chunks to load in the world.
    //     while let Some(reply) = self.state.storage.poll() {
    //         match reply {
    //             ChunkStorageReply::Load { cx, cz, res: Ok(snapshot) } => {
    //                 debug!("loaded chunk from storage: {cx}/{cz}");
    //                 self.world.insert_chunk_snapshot(snapshot);
    //             }
    //             ChunkStorageReply::Load { cx, cz, res: Err(err) } => {
    //                 debug!("failed to load chunk from storage: {cx}/{cz}: {err}");
    //             }
    //             ChunkStorageReply::Save { cx, cz, res: Ok(()) } => {
    //                 debug!("saved chunk in storage: {cx}/{cz}");
    //             }
    //             ChunkStorageReply::Save { cx, cz, res: Err(err) } => {
    //                 debug!("failed to save chunk in storage: {cx}/{cz}: {err}");
    //             }
    //         }
    //     }
    //
    //     // Only run if no tick freeze.
    //     match self.state.tick_mode {
    //         TickMode::Auto => {
    //             self.world.tick()
    //         }
    //         TickMode::Manual(0) => {}
    //         TickMode::Manual(ref mut n) => {
    //             self.world.tick();
    //             *n -= 1;
    //         }
    //     }
    //
    //     // Swap events out in order to proceed them.
    //     let mut events = self.world.swap_events(None).expect("events should be enabled");
    //     self.state.events_count.push(events.len() as f32, 0.001);
    //
    //     for event in events.drain(..) {
    //         match event {
    //             Event::Block { pos, inner } => match inner {
    //                 BlockEvent::Set { id, metadata, prev_id, prev_metadata } =>
    //                     self.handle_block_set(pos, id, metadata, prev_id, prev_metadata),
    //                 BlockEvent::Sound { id, metadata } =>
    //                     self.handle_block_sound(pos, id, metadata),
    //             }
    //             Event::Entity { id, inner } => match inner {
    //                 EntityEvent::Spawn =>
    //                     self.handle_entity_spawn(id),
    //                 EntityEvent::Remove =>
    //                     self.handle_entity_remove(id),
    //                 EntityEvent::Position { pos } =>
    //                     self.handle_entity_position(id, pos),
    //                 EntityEvent::Look { look } =>
    //                     self.handle_entity_look(id, look),
    //                 EntityEvent::Velocity { vel } =>
    //                     self.handle_entity_velocity(id, vel),
    //                 EntityEvent::Pickup { target_id } =>
    //                     self.handle_entity_pickup(id, target_id),
    //                 EntityEvent::Damage =>
    //                     self.handle_entity_damage(id),
    //                 EntityEvent::Dead =>
    //                     self.handle_entity_dead(id),
    //                 EntityEvent::Metadata =>
    //                     self.handle_entity_metadata(id),
    //             }
    //             Event::BlockEntity { pos, inner } => match inner {
    //                 BlockEntityEvent::Set =>
    //                     self.handle_block_entity_set(pos),
    //                 BlockEntityEvent::Remove =>
    //                     self.handle_block_entity_remove(pos),
    //                 BlockEntityEvent::Storage { storage, stack } =>
    //                     self.handle_block_entity_storage(pos, storage, stack),
    //                 BlockEntityEvent::Progress { progress, value } =>
    //                     self.handle_block_entity_progress(pos, progress, value),
    //             }
    //             Event::Chunk { cx, cz, inner } => match inner {
    //                 ChunkEvent::Set => {},
    //                 ChunkEvent::Remove => {}
    //                 ChunkEvent::Dirty => self.state.chunk_trackers.set_dirty(cx, cz),
    //             }
    //             Event::Weather { new, .. } =>
    //                 self.handle_weather_change(new),
    //             Event::Explode { center, radius } =>
    //                 self.handle_explode(center, radius),
    //             Event::DebugParticle { pos, block } =>
    //                 self.handle_debug_particle(pos, block),
    //         }
    //     }
    //
    //     // Reinsert events after processing.
    //     self.world.swap_events(Some(events));
    //
    //     // Send time to every playing clients every second.
    //     if time % 20 == 0 {
    //         let world_time = self.world.get_time();
    //         for player in &self.players {
    //             player.send(OutPacket::UpdateTime(proto::UpdateTimePacket {
    //                 time: world_time,
    //             }));
    //         }
    //     }
    //
    //     // After we collected every block change, update all players accordingly.
    //     self.state.chunk_trackers.update_players(&self.players, &self.world);
    //
    //     // After world events are processed, tick entity trackers.
    //     for tracker in self.state.entity_trackers.values_mut() {
    //         if time % 60 == 0 {
    //             tracker.update_tracking_players(&mut self.players, &self.world);
    //         }
    //         tracker.tick_and_update_players(&self.players);
    //     }
    //
    //     // Drain dirty chunk coordinates and save them.
    //     while let Some((cx, cz)) = self.state.chunk_trackers.next_save() {
    //         if let Some(snapshot) = self.world.take_chunk_snapshot(cx, cz) {
    //             self.state.storage.request_save(snapshot);
    //         }
    //     }
    //
    //     // Update tick duration metric.
    //     let tick_duration = start.elapsed();
    //     self.state.tick_duration.push(tick_duration.as_secs_f32(), 0.02);
    //
    //     // Finally increase server-side tick time.
    //     self.state.time += 1;
    //
    // }
    
    // /// Initialize the world by ensuring that every entity is currently tracked. This
    // /// method can be called multiple time and should be idempotent.
    // fn init(&mut self) {
    //
    //     // Ensure that every entity has a tracker.
    //     for (id, entity) in self.world.iter_entities() {
    //         self.state.entity_trackers.entry(id).or_insert_with(|| {
    //             let tracker = EntityTracker::new(id, entity);
    //             tracker.update_tracking_players(&mut self.players, &self.world);
    //             tracker
    //         });
    //     }
    //
    //     // NOTE: Temporary code.
    //     // let (center_cx, center_cz) = chunk::calc_entity_chunk_pos(config::SPAWN_POS);
    //     // for cx in center_cx - 10..=center_cx + 10 {
    //     //     for cz in center_cz - 10..=center_cz + 10 {
    //     //         self.state.storage.request_load(cx, cz);
    //     //     }
    //     // }
    //
    // }

    /// Interact with a block at given position. This function returns the interaction
    /// result to indicate if the interaction was handled, or if it was
    ///
    /// The second argument `breaking` indicates if the interaction originate from a
    /// player breaking the block.
    pub fn interact_block(pos: IVec3, breaking: bool) -> Interaction {

        if let Some((id, metadata)) = Self::get_block(pos) {
            Self::interact_block_unchecked(pos, id, metadata, breaking)
        } else {
            Interaction::None
        }
    }

    /// Use an item stack on a given block, this is basically the action of left click.
    /// This function returns the item stack after, if used, this may return an item stack
    /// with size of 0. The face is where the click has hit on the target block.
    pub fn use_stack(&mut self, inventory_id: u32, index: u32, pos: IVec3, face: Face, entity_id: u32, cache: &mut ChunkCache) {

        let stack = StdbInventory::get(inventory_id, index);
        if stack.is_empty() {
            return;
        }

        let success = match stack.id {
            0 => false,
            1..=255 => Self::use_block_stack(stack.id as u8, stack.damage as u8, pos, face, entity_id, cache),
            // item::SUGAR_CANES => self.use_block_stack(block::SUGAR_CANES, 0, pos, face, entity_id),
            // item::CAKE => self.use_block_stack(block::CAKE, 0, pos, face, entity_id),
            // item::REPEATER => self.use_block_stack(block::REPEATER, 0, pos, face, entity_id),
            // item::REDSTONE => self.use_block_stack(block::REDSTONE, 0, pos, face, entity_id),
            // item::WOOD_DOOR => self.use_door_stack(block::WOOD_DOOR, pos, face, entity_id),
            // item::IRON_DOOR => self.use_door_stack(block::IRON_DOOR, pos, face, entity_id),
            // item::BED => self.use_bed_stack(pos, face, entity_id),
            // item::SIGN => self.use_sign_stack(pos, face, entity_id),
            // item::DIAMOND_HOE |
            // item::IRON_HOE |
            // item::STONE_HOE |
            // item::GOLD_HOE |
            // item::WOOD_HOE => self.use_hoe_stack(pos, face),
            // item::WHEAT_SEEDS => self.use_wheat_seeds_stack(pos, face),
            // item::DYE if stack.damage == 15 => self.use_bone_meal_stack(pos),
            // item::FLINT_AND_STEEL => self.use_flint_and_steel(pos, face),
            // item::PAINTING => self.use_painting(pos, face),
            _ => false
        };

        if success {
            StdbInventory::set(inventory_id, index, stack.inc_damage(1));
        }

    }

    /// Place a block toward the given face. This is used for single blocks, multi blocks
    /// are handled apart by other functions that do not rely on the block placing logic.
    fn use_block_stack(id: u8, metadata: u8, mut pos: IVec3, mut face: Face, entity_id: u32, cache: &mut ChunkCache) -> bool {
        let look = StdbEntity::filter_by_entity_id(&entity_id).expect(
            format!("Player has no entity: {}", entity_id).as_str());
        // let look = self.get_entity(entity_id).unwrap().0.look;

        if let Some((block::SNOW, _)) = Self::get_block(pos) {
            // If a block is placed by clicking on a snow block, replace that snow block.
            face = Face::NegY;
        } else {
            // Get position of the block facing the clicked face.
            pos += face.delta();
            // The block is oriented toward that clicked face.
            face = face.opposite();
        }

        // Some block have special facing when placed.
        match id {
            // TODO(jdetter): come back to these when we have entities properly implemented
            // block::WOOD_STAIR | block::COBBLESTONE_STAIR |
            // block::REPEATER | block::REPEATER_LIT => {
            //     face = Face::from_yaw(look.x);
            // }
            // block::DISPENSER |
            // block::FURNACE | block::FURNACE_LIT |
            // block::PUMPKIN | block::PUMPKIN_LIT => {
            //     face = Face::from_yaw(look.x).opposite();
            // }
            // block::PISTON |
            // block::STICKY_PISTON => {
            //     face = Face::from_look(look.x, look.y).opposite();
            // }
            _ => {}
        }

        if pos.y >= 127 && block::material::get_material(id).is_solid() {
            return false;
        }

        if !Self::can_place_block(pos, face, id) {
            return false;
        }

        Self::place_block(pos, face, id, metadata, cache);
        true
    }

    /// Place the block at the given position in the world oriented toward given face. Note
    /// that this function do not check if this is legal, it will do what's asked. Also, the
    /// given metadata may be modified to account for the placement.
    pub fn place_block(pos: IVec3, face: Face, id: u8, metadata: u8, cache: &mut ChunkCache) {

        match id {
            // TODO(jdetter): Implement these when we support entities
            // block::BUTTON => self.place_faced(pos, face, id, metadata, block::button::set_face),
            // block::TRAPDOOR => self.place_faced(pos, face, id, metadata, block::trapdoor::set_face),
            // block::PISTON |
            // block::STICKY_PISTON => self.place_faced(pos, face, id, metadata, block::piston::set_face),
            // block::WOOD_STAIR |
            // block::COBBLESTONE_STAIR => self.place_faced(pos, face, id, metadata, block::stair::set_face),
            // block::REPEATER |
            // block::REPEATER_LIT => self.place_faced(pos, face, id, metadata, block::repeater::set_face),
            // block::PUMPKIN |
            // block::PUMPKIN_LIT => self.place_faced(pos, face, id, metadata, block::pumpkin::set_face),
            // block::FURNACE |
            // block::FURNACE_LIT |
            // block::DISPENSER => self.place_faced(pos, face, id, metadata, block::dispenser::set_face),
            // block::TORCH |
            // block::REDSTONE_TORCH |
            // block::REDSTONE_TORCH_LIT => self.place_faced(pos, face, id, metadata, block::torch::set_face),
            // block::LEVER => self.place_lever(pos, face, metadata),
            // block::LADDER => self.place_ladder(pos, face, metadata),
            _ => {
                Self::set_block_notify(pos, id, metadata, cache);
            }
        }

        // match id {
        //     block::CHEST => self.set_block_entity(pos, BlockEntity::Chest(def())),
        //     block::FURNACE => self.set_block_entity(pos, BlockEntity::Furnace(def())),
        //     block::DISPENSER => self.set_block_entity(pos, BlockEntity::Dispenser(def())),
        //     block::SPAWNER => self.set_block_entity(pos, BlockEntity::Spawner(def())),
        //     block::NOTE_BLOCK => self.set_block_entity(pos, BlockEntity::NoteBlock(def())),
        //     block::JUKEBOX => self.set_block_entity(pos, BlockEntity::Jukebox(def())),
        //     _ => {}
        // }
    }

    /// This function checks if the given block id can be placed at a particular position in
    /// the world, the given face indicates toward which face this block should be oriented.
    pub fn can_place_block(pos: IVec3, face: Face, id: u8) -> bool {

        let base = match id {
            // TODO(jdetter): Come back to these after we have entities properly implemented
            // block::BUTTON if face.is_y() => false,
            // block::BUTTON => self.is_block_opaque_cube(pos + face.delta()),
            // block::LEVER if face == Face::PosY => false,
            // block::LEVER => self.is_block_opaque_cube(pos + face.delta()),
            // block::LADDER => self.is_block_opaque_around(pos),
            // block::TRAPDOOR if face.is_y() => false,
            // block::TRAPDOOR => self.is_block_opaque_cube(pos + face.delta()),
            // block::PISTON_EXT |
            // block::PISTON_MOVING => false,
            // block::DEAD_BUSH => matches!(self.get_block(pos - IVec3::Y), Some((block::SAND, _))),
            // // PARITY: Notchian impl checks block light >= 8 or see sky
            // block::DANDELION |
            // block::POPPY |
            // block::SAPLING |
            // block::TALL_GRASS => matches!(self.get_block(pos - IVec3::Y), Some((block::GRASS | block::DIRT | block::FARMLAND, _))),
            // block::WHEAT => matches!(self.get_block(pos - IVec3::Y), Some((block::FARMLAND, _))),
            // block::CACTUS => self.can_place_cactus(pos),
            // block::SUGAR_CANES => self.can_place_sugar_canes(pos),
            // block::CAKE => self.is_block_solid(pos - IVec3::Y),
            // block::CHEST => self.can_place_chest(pos),
            // block::WOOD_DOOR |
            // block::IRON_DOOR => self.can_place_door(pos),
            // block::FENCE => matches!(self.get_block(pos - IVec3::Y), Some((block::FENCE, _))) || self.is_block_solid(pos - IVec3::Y),
            // block::FIRE => self.can_place_fire(pos),
            // block::TORCH |
            // block::REDSTONE_TORCH |
            // block::REDSTONE_TORCH_LIT => self.is_block_normal_cube(pos + face.delta()),
            // // Common blocks that needs opaque block below.
            // block::RED_MUSHROOM |        // PARITY: Notchian impl checks block light >= 8 or see sky
            // block::BROWN_MUSHROOM => self.is_block_opaque_cube(pos - IVec3::Y),
            // block::SNOW => self.is_block_opaque_cube(pos - IVec3::Y),
            // block::WOOD_PRESSURE_PLATE |
            // block::STONE_PRESSURE_PLATE |
            // block::PUMPKIN |
            // block::PUMPKIN_LIT |
            // block::RAIL |
            // block::POWERED_RAIL |
            // block::DETECTOR_RAIL |
            // block::REPEATER |
            // block::REPEATER_LIT |
            // block::REDSTONE => self.is_block_normal_cube(pos - IVec3::Y),
            _ => true,
        };

        // If the block we are placing has an exclusion box and any hard entity is inside,
        // we cancel the prevent the placing.
        // TODO(jdetter): Add this back in when we add support for collision boxes in stdb
        // if let Some(bb) = self.get_block_exclusion_box(pos, id) {
        //     if self.has_entity_colliding(bb, true) {
        //         return false;
        //     }
        // }

        base && Self::is_block_replaceable(pos)

    }

    /// Return true if the block at given position can be replaced.
    pub fn is_block_replaceable(pos: IVec3) -> bool {
        if let Some((id, _)) = Self::get_block(pos) {
            block::material::get_material(id).is_replaceable()
        } else {
            false
        }
    }

    /// Internal function to handle block interaction at given position and with known
    /// block and metadata.
    pub(super) fn interact_block_unchecked(pos: IVec3, id: u8, metadata: u8, breaking: bool) -> Interaction {
        match id {
            // block::BUTTON => self.interact_button(pos, metadata),
            // block::LEVER => self.interact_lever(pos, metadata),
            // block::TRAPDOOR => self.interact_trapdoor(pos, metadata),
            // block::IRON_DOOR => true,
            // block::WOOD_DOOR => self.interact_wood_door(pos, metadata),
            // block::REPEATER |
            // block::REPEATER_LIT => self.interact_repeater(pos, id, metadata),
            // block::REDSTONE_ORE => self.interact_redstone_ore(pos),
            // block::CRAFTING_TABLE => return Interaction::CraftingTable { pos },
            // block::CHEST => return self.interact_chest(pos),
            // block::FURNACE |
            // block::FURNACE_LIT => return self.interact_furnace(pos),
            // block::DISPENSER => return self.interact_dispenser(pos),
            // block::NOTE_BLOCK => self.interact_note_block(pos, breaking),
            block::BUTTON => Interaction::Handled,
            block::LEVER => Interaction::Handled,
            block::TRAPDOOR => Interaction::Handled,
            block::IRON_DOOR => Interaction::Handled,
            block::WOOD_DOOR => Interaction::Handled,
            block::REPEATER |
            block::REPEATER_LIT => Interaction::Handled,
            block::REDSTONE_ORE => Interaction::Handled,
            block::CRAFTING_TABLE => Interaction::Handled,
            block::CHEST => Interaction::Handled,
            block::FURNACE |
            block::FURNACE_LIT => Interaction::Handled,
            block::DISPENSER => Interaction::Handled,
            block::NOTE_BLOCK => Interaction::Handled,
            _ => return Interaction::None
        }.into()
    }

    /// Get block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned.
    pub fn get_block(pos: IVec3) -> Option<(u8, u8)> {
        let (cx, cz) = calc_chunk_pos(pos)?;
        let chunk = Self::get_chunk(cx, cz)?;
        Some(chunk.chunk.get_block(pos))
    }

    pub fn get_chunk(cx: i32, cz: i32) -> Option<StdbChunk> {
        let chunk_id = StdbChunk::xz_to_chunk_id(cx, cz);
        StdbChunk::filter_by_chunk_id(&chunk_id)
    }

    /// Handle a player joining this world.
    pub fn handle_player_join(&mut self, player: StdbServerPlayer) {

        // Initial tracked entities.
        for tracker in StdbEntityView::filter_by_target_id(&player.entity_id) {
            StdbEntityView::delete_by_view_id(&tracker.view_id);
            // let tracker = StdbEntityTracker::filter_by_entity_id(&tracker.target_id).unwrap();
            // tracker.update_tracking_player(&player);
        }

        let player_entity = StdbEntity::filter_by_entity_id(&player.entity_id).unwrap();
        for other_entity in StdbEntity::filter_by_dimension_id(&player_entity.dimension_id) {
            if other_entity.entity_id != player_entity.entity_id {
                // TODO(jdetter): Check distance
                let _ = StdbEntityView::insert(StdbEntityView {
                    view_id: 0,
                    observer_id: other_entity.entity_id,
                    target_id: player_entity.entity_id,
                });

                let _ = StdbEntityView::insert(StdbEntityView {
                    view_id: 0,
                    observer_id: player_entity.entity_id,
                    target_id: other_entity.entity_id,
                });
            }
        }

        // for tracker in self.state.entity_trackers.values() {
        //     tracker.update_tracking_player(&mut player, &self.world);
        // }

        StdbServerPlayer::update_chunks(player.entity_id);
        // player.update_chunks(&self.world);

        // let player_index = self.players.len();
        // self.players.push(player);
        // player_index
    }

    /// Handle a player leaving this world, this should remove its entity. The `lost`
    /// argument indicates if the player is leaving because of a lost connection or not.
    /// If the connection was not lost, chunks and entities previously tracked by the
    /// player are send to be untracked.
    ///
    /// **Note that** this function swap remove the player, so the last player in this
    /// world's list is moved to the given player index. So if it exists, you should
    /// update all indices pointing to the swapped player. This method returns, if
    /// existing, the player that was swapped.
    pub fn handle_player_leave(&self, player: StdbServerPlayer, lost: bool) {
        // Remove the player tracker.
        // let mut player = self.players.swap_remove(player_index);

        // Kill the entity associated to the player.
        // self.world.remove_entity(player.entity_id, "server player left");
        // StdbConnectionStatus::delete_by_connection_id(&connection_id);

        // If player has not lost connection but it's just leaving the world, we just
        // send it untrack packets.
        // if !lost {
        //
        //     // Take and replace it with an empty set (no overhead).
        //     let tracked_entities = std::mem::take(&mut player.tracked_entities);
        //
        //     // Untrack all its entities.
        //     for entity_id in tracked_entities {
        //         let tracker = self.state.entity_trackers.get(&entity_id).expect("incoherent tracked entity");
        //         tracker.kill_entity(&mut player);
        //     }
        // }

        // No other players should be tracking this player and this player shouldn't be tracking any other players
        for tracker in query!(|tracked: StdbTrackedPlayer| tracked.to_id == player.entity_id || tracked.from_id == player.entity_id) {
            StdbTrackedPlayer::delete_by_track_id(&tracker.track_id);
        }
    }

    // /// Handle a block change world event.
    // fn handle_block_set(&mut self, pos: IVec3, id: u8, metadata: u8, prev_id: u8, _prev_metadata: u8) {
    //
    //     // Notify the tracker of the block change, this is used to notify the player
    //     self.state.chunk_trackers.set_block(pos, id, metadata);
    //
    //     // If the block was a crafting table, if any player has a crafting table
    //     // window referencing this block then we force close it.
    //     let break_crafting_table = id != prev_id && prev_id == block::CRAFTING_TABLE;
    //     if break_crafting_table {
    //         for player in &mut self.players {
    //             player.close_block_window(&mut self.world, pos);
    //         }
    //     }
    //
    // }

    // fn handle_block_sound(&mut self, pos: IVec3, _block: u8, _metadata: u8) {
    //     let (cx, cz) = chunk::calc_chunk_pos_unchecked(pos);
    //     for player in &self.players {
    //         if player.tracked_chunks.contains(&(cx, cz)) {
    //             player.send(OutPacket::EffectPlay(proto::EffectPlayPacket {
    //                 effect_id: 1003,
    //                 x: pos.x,
    //                 y: pos.y as i8,
    //                 z: pos.z,
    //                 effect_data: 0,
    //             }));
    //         }
    //     }
    // }

    // fn handle_explode(&mut self, center: DVec3, radius: f32) {
    //     let (cx, cz) = chunk::calc_entity_chunk_pos(center);
    //     for player in &self.players {
    //         if player.tracked_chunks.contains(&(cx, cz)) {
    //             player.send(OutPacket::Explosion(proto::ExplosionPacket {
    //                 x: center.x,
    //                 y: center.y,
    //                 z: center.z,
    //                 size: radius,
    //                 blocks: vec![],
    //             }));
    //         }
    //     }
    // }

    // /// Handle an entity spawn world event.
    // fn handle_entity_spawn(&mut self, id: u32) {
    //     // The entity may have already been removed.
    //     if let Some(entity) = self.world.get_entity(id) {
    //         self.state.entity_trackers.entry(id).or_insert_with(|| {
    //             let tracker = EntityTracker::new(id, entity);
    //             tracker.update_tracking_players(&mut self.players, &self.world);
    //             tracker
    //         });
    //     }
    // }

    // /// Handle an entity kill world event.
    // fn handle_entity_remove(&mut self, id: u32) {
    //     // The entity may not be spawned yet (read above).
    //     if let Some(tracker) = self.state.entity_trackers.remove(&id) {
    //         tracker.untrack_players(&mut self.players);
    //     };
    // }

    // /// Handle an entity position world event.
    // fn handle_entity_position(&mut self, id: u32, pos: DVec3) {
    //     if let Some(tracker) = self.state.entity_trackers.get_mut(&id) {
    //         tracker.set_pos(pos);
    //     }
    // }

    // /// Handle an entity look world event.
    // fn handle_entity_look(&mut self, id: u32, look: Vec2) {
    //     if let Some(tracker) = self.state.entity_trackers.get_mut(&id) {
    //         tracker.set_look(look);
    //     }
    // }

    // /// Handle an entity look world event.
    // fn handle_entity_velocity(&mut self, id: u32, vel: DVec3) {
    //     if let Some(tracker) = self.state.entity_trackers.get_mut(&id) {
    //         tracker.set_vel(vel);
    //     }
    // }

    // /// Handle an entity pickup world event.
    // fn handle_entity_pickup(&mut self, id: u32, target_id: u32) {
    //
    //     let Some(Entity(_, target_kind)) = self.world.get_entity_mut(target_id) else { return };
    //     let Some(player) = self.players.iter_mut().find(|p| p.entity_id == id) else {
    //         // This works only on entities handled by players.
    //         return
    //     };
    //
    //     // Used only for picking arrow.
    //     let mut arrow_stack = ItemStack::new_single(item::ARROW, 0);
    //
    //     let stack = match target_kind {
    //         BaseKind::Item(item)
    //             => &mut item.stack,
    //         BaseKind::Projectile(projectile, ProjectileKind::Arrow(_))
    //             if projectile.shake == 0
    //             => &mut arrow_stack,
    //         // Other entities cannot be picked up.
    //         _ => return,
    //     };
    //
    //     player.pickup_stack(stack);
    //
    //     // If the item stack has been emptied, kill the entity.
    //     if stack.size == 0 {
    //         self.world.remove_entity(target_id, "picked up");
    //     }
    //
    //     for player in &self.players {
    //         if player.tracked_entities.contains(&target_id) {
    //             player.send(OutPacket::EntityPickup(proto::EntityPickupPacket {
    //                 entity_id: id,
    //                 picked_entity_id: target_id,
    //             }));
    //         }
    //     }
    //
    // }
    //
    // /// Handle an entity damage event.
    // fn handle_entity_damage(&mut self, id: u32) {
    //
    //     self.handle_entity_status(id, 2);
    //
    //     // TODO: This is temporary code, we need to make a common method to update health.
    //     for player in &self.players {
    //         if player.entity_id == id {
    //             if let Entity(_, BaseKind::Living(living, _)) = self.world.get_entity(id).unwrap() {
    //                 player.send(OutPacket::UpdateHealth(proto::UpdateHealthPacket {
    //                     health: living.health.min(i16::MAX as _) as i16,
    //                 }));
    //             }
    //         }
    //     }
    //
    // }
    //
    // /// Handle an entity dead event (the entity is not yet removed).
    // fn handle_entity_dead(&mut self, id: u32) {
    //     self.handle_entity_status(id, 3);
    // }
    //
    // /// Handle an entity damage/dead or other status for an entity.
    // fn handle_entity_status(&mut self, id: u32, status: u8) {
    //     for player in &self.players {
    //         if player.tracked_entities.contains(&id) || player.entity_id == id {
    //             player.send(OutPacket::EntityStatus(proto::EntityStatusPacket {
    //                 entity_id: id,
    //                 status,
    //             }));
    //         }
    //     }
    // }
    //
    // fn handle_entity_metadata(&mut self, id: u32) {
    //     if let Some(tracker) = self.state.entity_trackers.get_mut(&id) {
    //         for player in &self.players {
    //             if player.tracked_entities.contains(&id) {
    //                 tracker.update_entity(player, &self.world);
    //             }
    //         }
    //     }
    // }
    //
    // /// Handle a block entity set event.
    // fn handle_block_entity_set(&mut self, _pos: IVec3) {
    //
    // }
    //
    // /// Handle a block entity remove event.
    // fn handle_block_entity_remove(&mut self, pos: IVec3) {
    //
    //     // Close the inventory of all entities that had a window opened for this block.
    //     for player in &mut self.players {
    //         player.close_block_window(&mut self.world, pos);
    //     }
    //
    // }
    //
    // /// Handle a storage event for a block entity.
    // fn handle_block_entity_storage(&mut self, pos: IVec3, storage: BlockEntityStorage, stack: ItemStack) {
    //
    //     // Update any player that have a window opened on that block entity.
    //     for player in &mut self.players {
    //         player.update_block_window_storage(pos, storage, stack);
    //     }
    //
    // }
    //
    // fn handle_block_entity_progress(&mut self, pos: IVec3, progress: BlockEntityProgress, value: u16) {
    //
    //     // Update any player that have a window opened on that block entity.
    //     for player in &mut self.players {
    //         player.update_block_window_progress(pos, progress, value);
    //     }
    //
    // }
    //
    // /// Handle weather change in the world.
    // fn handle_weather_change(&mut self, weather: Weather) {
    //     for player in &self.players {
    //         player.send(OutPacket::Notification(proto::NotificationPacket {
    //             reason: if weather == Weather::Clear { 2 } else { 1 },
    //         }));
    //     }
    // }
    //
    // fn handle_debug_particle(&mut self, pos: IVec3, block: u8) {
    //     let (cx, cz) = chunk::calc_chunk_pos_unchecked(pos);
    //     for player in &self.players {
    //         if player.tracked_chunks.contains(&(cx, cz)) {
    //             player.send(OutPacket::EffectPlay(proto::EffectPlayPacket {
    //                 effect_id: 2001,
    //                 x: pos.x,
    //                 y: pos.y as i8,
    //                 z: pos.z,
    //                 effect_data: block as u32,
    //             }));
    //         }
    //     }
    // }

    /// Same as the [`set_block_self_notify`] method, but additionally the blocks around
    /// are notified of that neighbor change.
    ///
    /// [`set_block_self_notify`]: Self::set_block_self_notify
    pub fn set_block_notify(pos: IVec3, id: u8, metadata: u8, cache: &mut ChunkCache) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = Self::set_block_self_notify(pos, id, metadata, cache)?;
        // self.notify_blocks_around(pos, id, cache);
        Some((prev_id, prev_metadata))
    }

    /// Same as the [`set_block`] method, but the previous block and new block are
    /// notified of that removal and addition.
    ///
    /// [`set_block`]: Self::set_block
    pub fn set_block_self_notify(pos: IVec3, id: u8, metadata: u8, cache: &mut ChunkCache) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = Self::set_block(pos, id, metadata, cache)?;
        // self.notify_change_unchecked(pos, prev_id, prev_metadata, id, metadata);
        Some((prev_id, prev_metadata))
    }

    /// Set block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned, but if it is existing the previous block and metadata
    /// is returned. This function also push a block change event and update lights
    /// accordingly.
    pub fn set_block(pos: IVec3, id: u8, metadata: u8, cache: &mut ChunkCache) -> Option<(u8, u8)> {
        let (cx, cz) = calc_chunk_pos(pos)?;
        let mut chunk = cache.get_chunk(cx, cz)?;

        let result = Self::set_block_inner(pos, id, metadata, &mut chunk);

        if result.0 {
            let id = chunk.chunk_id;
            cache.set_chunk(chunk);
            // StdbChunk::update_by_chunk_id(&id, chunk);
        }

        Some((result.1, result.2))
    }

    pub fn set_block_inner(pos: IVec3, id: u8, metadata: u8, chunk: &mut StdbChunk) -> (bool, u8, u8) {

        let (prev_id, prev_metadata) = chunk.chunk.get_block(pos);
        let mut changed = false;

        if id != prev_id || metadata != prev_metadata {
            changed = true;

            chunk.chunk.set_block(pos, id, metadata);
            chunk.chunk.recompute_height(pos);

            // Schedule light updates if the block light properties have changed.
            // if block::material::get_light_opacity(id) != block::material::get_light_opacity(prev_id)
            //     || block::material::get_light_emission(id) != block::material::get_light_emission(prev_id) {
            //     self.schedule_light_update(pos, LightKind::Block);
            //     self.schedule_light_update(pos, LightKind::Sky);
            // }

            // TODO: Another event that we don't care about in the SpacetimeDB module
            // StdbSetBlockEvent::insert(StdbSetBlockEvent {
            //     pos: pos.into(),
            //     new_id: id,
            //     new_metadata: metadata,
            //     old_id: prev_id,
            //     old_metadata: prev_metadata,
            // });

            // self.push_event(Event::Block {
            //     pos,
            //     inner: BlockEvent::Set {
            //         id,
            //         metadata,
            //         prev_id,
            //         prev_metadata,
            //     }
            // });

            let chunk_update = StdbChunkUpdate::insert(StdbChunkUpdate {
                update_id: 0,
                chunk_id: StdbChunk::xz_to_chunk_id(chunk.x, chunk.z),
                update_type: ChunkUpdateType::BlockSet,
            }).unwrap();

            StdbBlockSetUpdate::insert(StdbBlockSetUpdate {
                update_id: chunk_update.update_id,
                x: pos.x,
                y: pos.y as i8,
                z: pos.z,
                block: id,
                metadata,
            }).unwrap();

            // StdbChunkEvent::insert(StdbChunkEvent {
            //     x: chunk.x,
            //     z: chunk.z,
            //     inner: ChunkEvent::Dirty
            // });
            // self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

        }

        (changed, prev_id, prev_metadata)
    }
}
