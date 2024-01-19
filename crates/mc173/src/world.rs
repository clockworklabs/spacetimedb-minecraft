//! Data structure for storing a world (overworld or nether) at runtime.

use std::collections::{HashMap, BTreeSet, HashSet, VecDeque};
use std::collections::hash_map::Entry;
use std::ops::{Deref, DerefMut};
use std::iter::FusedIterator;
use std::cmp::Ordering;
use std::hash::Hash;
use std::cell::Cell;
use std::sync::Arc;
use std::slice;
use std::mem;

use glam::{IVec3, Vec2, DVec3};
use indexmap::IndexMap;

use tracing::trace;

use crate::entity::{Entity, EntityCategory, EntityKind};
use crate::block_entity::BlockEntity;
use crate::biome::Biome;
use crate::chunk::{Chunk,
    calc_chunk_pos, calc_chunk_pos_unchecked, calc_entity_chunk_pos,
    CHUNK_HEIGHT, CHUNK_WIDTH};

use crate::geom::{BoundingBox, Face};
use crate::rand::JavaRandom;
use crate::item::ItemStack;
use crate::block;


// Following modules are order by order of importance, last modules depends on first ones.
pub mod material;
pub mod bound;
pub mod power;
pub mod loot;
pub mod interact;
pub mod place;
pub mod r#break;
pub mod r#use;
pub mod tick;
pub mod notify;
pub mod explode;


// Various thread local vectors that are used to avoid frequent reallocation of 
// temporary vector used in the logic code.
thread_local! {
    /// This thread local vector is used temporally to stores the random ticks to be 
    /// executed. This is mandatory since ticking a block requires full mutable access to
    /// the world, but it's not possible while owning a reference to a chunk.
    static RANDOM_TICKS_PENDING: Cell<Vec<(IVec3, u8, u8)>> = const { Cell::new(Vec::new()) };
    /// A temporary list of loaded chunks positions.
    static LOADED_CHUNKS: Cell<Vec<(i32, i32)>> = const { Cell::new(Vec::new()) };
}


/// A data-structure that fully describes a Minecraft beta 1.7.3 world, with all its 
/// blocks, lights, biomes, entities and block entities. It also keep the current state
/// of the world such as time and weather and allows ticking it step by step.
/// 
/// # Components 
/// 
/// This data structure stores different kind of component:
/// - Chunks, these are the storage for block, light and height map of a 16x16 column in
///   the world with a height of 128. This component has the largest memory footprint
///   overall and is stored in shared reference to avoid too much memory copy.
///   A chunk must be present in order to set block in the world.
/// - Entities, basically anything that needs to be ticked with 3 dimensional coordinates.
///   They can control their own position, velocity and look for example.
/// - Block Entities, this is a mix between entities and blocks, they can be ticked but
///   are attached to a block position that they cannot control.
/// 
/// These components are independent, but are internally optimized for access. For example
/// entities are not directly linked to a chunk, but an iterator over entities within a
/// chunk can be obtained.
/// 
/// This data structure is however not designed to handle automatic chunk loading and 
/// saving, every chunk needs to be manually inserted and removed, same for entities and
/// block entities.
/// 
/// # Logic
/// 
/// This data structure is also optimized for actually running the world's logic if 
/// needed. Such as weather, random block ticking, scheduled block ticking, entity 
/// ticking or block notifications.
/// 
/// # Events
/// 
/// This structure also allows listening for events within it through a queue of 
/// [`Event`], events listening is disabled by default but can be enabled by swapping
/// a `Vec<Event>` into the world using the [`swap_events`](Self::swap_events). Events 
/// are generated either by world's ticking logic or by manual changes to the world. 
/// Events are ordered chronologically, for example and entity cannot be removed before 
/// being spawned.
/// 
/// # Naming convention
/// 
/// Methods provided on this structure should follow a naming convention depending on the
/// action that will apply to the world:
/// - Methods that don't alter the world and return values should be prefixed by `get_`, 
///   these are getters and should not usually compute too much, getters that returns
///   mutable reference should be suffixed with `_mut`;
/// - Getter methods that return booleans should prefer `can_`, `has_` or `is_` prefixes;
/// - Methods that alter the world by running a logic tick should start with `tick_`;
/// - Methods that iterate over some world objects should start with `iter_`, the return
///   iterator type should preferably be a new type (not `impl Iterator`);
/// - Methods that run on internal events can be prefixed by `handle_`;
/// - All other methods should use a proper verb, preferably composed of one-word to
///   reduce possible meanings (e.g. are `schedule_`, `break_`, `spawn_`, `insert_` or
///   `remove_`).
/// 
/// Various suffixes can be added to methods, depending on the world area affected by the
/// method, for example `_in`, `_in_chunk`, `_in_box` or `_colliding`.
/// Any mutation prefix `_mut` should be placed at the very end.
/// 
/// # Roadmap
/// 
/// - Make a diagram to better explain the world structure with entity caching.
#[derive(Clone)]
pub struct World {
    /// When enabled, this contains the list of events that happened in the world since
    /// it was last swapped. This swap behavior is really useful in order to avoid 
    /// borrowing issues, by temporarily taking ownership of events, the caller can get
    /// a mutable reference to that world at the same time.
    events: Option<Vec<Event>>,
    /// The dimension
    dimension: Dimension,
    /// The world time, increasing on each tick. This is used for day/night cycle but 
    /// also for registering scheduled ticks.
    time: u64,
    /// The world's global random number generator, it is used everywhere to randomize
    /// events in the world, such as plant grow.
    rand: JavaRandom,
    /// The mapping of world chunks, with optional world components linked to them, such
    /// as chunk data, entities and block entities. Every world component must be linked
    /// to a world chunk.
    chunks: HashMap<(i32, i32), ChunkComponent>,
    /// Total entities count spawned since the world is running. Also used to give 
    /// entities a unique id.
    entities_count: u32,
    /// The internal list of all loaded entities.
    entities: TickVec<EntityComponent>,
    /// Entities' index mapping from their unique id.
    entities_id_map: HashMap<u32, usize>,
    /// This index map contains a mapping for every player entity.
    entities_player_map: IndexMap<u32, usize>,
    /// Same as entities but for block entities.
    block_entities: TickVec<BlockEntityComponent>,
    /// Mapping of block entities to they block position.
    block_entities_pos_map: HashMap<IVec3, usize>,
    /// Total scheduled ticks count since the world is running.
    block_ticks_count: u64,
    /// Mapping of scheduled ticks in the future.
    block_ticks: BTreeSet<BlockTick>,
    /// A set of all scheduled tick states, used to avoid ticking twice the same position
    /// and block id. 
    block_ticks_states: HashSet<BlockTickState>,
    /// Queue of pending light updates to be processed.
    light_updates: VecDeque<LightUpdate>,
    /// This is the wrapping seed used by random ticks to compute random block positions.
    random_ticks_seed: i32,
    /// The current weather in that world, note that the Notchian server do not work like
    /// this, but rather store two independent state for rain and thunder, but we simplify
    /// the logic in this implementation since it is not strictly needed to be on parity.
    weather: Weather,
    /// Next time when the weather should be recomputed.
    weather_next_time: u64,
    /// The current sky light level, depending on the current time. This value is used
    /// when subtracted from a chunk sky light level.
    sky_light_subtracted: u8,
}

/// Core methods for worlds.
impl World {

    /// Create a new world of the given dimension with no events queue by default, so
    /// events are disabled.
    pub fn new(dimension: Dimension) -> Self {
        Self {
            events: None,
            dimension,
            time: 0,
            rand: JavaRandom::new_seeded(),
            chunks: HashMap::new(),
            entities_count: 0,
            entities: TickVec::new(),
            entities_id_map: HashMap::new(),
            entities_player_map: IndexMap::new(),
            block_entities: TickVec::new(),
            block_entities_pos_map: HashMap::new(),
            block_ticks_count: 0,
            block_ticks: BTreeSet::new(),
            block_ticks_states: HashSet::new(),
            light_updates: VecDeque::new(),
            random_ticks_seed: JavaRandom::new_seeded().next_int(),
            weather: Weather::Clear,
            weather_next_time: 0,
            sky_light_subtracted: 0,
        }
    }

    /// This function can be used to swap in a new events queue and return the previous
    /// one if relevant. Giving *None* events queue disable events registration using
    /// the [`push_event`] method. Swapping out the events is the only way of reading 
    /// them afterward without borrowing the world.
    /// 
    /// [`push_event`]: Self::push_event
    pub fn swap_events(&mut self, events: Option<Vec<Event>>) -> Option<Vec<Event>> {
        mem::replace(&mut self.events, events)
    }

    /// Return true if this world has an internal events queue that enables usage of the
    /// [`push_event`] method.
    /// 
    /// [`push_event`]: Self::push_event
    pub fn has_events(&self) -> bool {
        self.events.is_some()
    }

    /// Push an event in this world. This only actually push the event if events are 
    /// enabled. Events queue can be swapped using [`swap_events`](Self::swap_events) 
    /// method.
    #[inline]
    pub fn push_event(&mut self, event: Event) {
        if let Some(events) = &mut self.events {
            events.push(event);
        }
    }

    /// Get the dimension of this world, this is basically only for sky color on client
    /// and also for celestial angle on the server side for sky light calculation. This
    /// has not direct relation with the actual world generation that is providing this
    /// world with chunks and entities.
    pub fn get_dimension(&self) -> Dimension {
        self.dimension
    }

    /// Get the world time, in ticks.
    pub fn get_time(&self) -> u64 {
        self.time
    }

    /// Get a mutable access to this world's random number generator.
    pub fn get_rand_mut(&mut self) -> &mut JavaRandom {
        &mut self.rand
    }

    /// Get the current weather in the world.
    pub fn get_weather(&self) -> Weather {
        self.weather
    }

    /// Set the current weather in this world. If the weather has changed an event will
    /// be pushed into the events queue.
    pub fn set_weather(&mut self, weather: Weather) {
        if self.weather != weather {
            self.push_event(Event::Weather { prev: self.weather, new: weather });
            self.weather = weather;
        }
    }

    // =================== //
    //   CHUNK SNAPSHOTS   //
    // =================== //

    /// Insert a chunk snapshot into this world at its position with all entities and 
    /// block entities attached to it.
    pub fn insert_chunk_snapshot(&mut self, snapshot: ChunkSnapshot) {
        
        self.set_chunk(snapshot.cx, snapshot.cz, snapshot.chunk);
        
        for entity in snapshot.entities {
            debug_assert_eq!(calc_entity_chunk_pos(entity.0.pos), (snapshot.cx, snapshot.cz), "incoherent entity in chunk snapshot");
            self.spawn_entity_inner(entity);
        }

        for (pos, block_entity) in snapshot.block_entities {
            debug_assert_eq!(calc_chunk_pos_unchecked(pos), (snapshot.cx, snapshot.cz), "incoherent block entity in chunk snapshot");
            self.set_block_entity_inner(pos, block_entity);
        }

    }

    /// Create a snapshot of a chunk's content, this only works if chunk data is existing.
    /// This operation can be costly depending on the number of entities in the chunk, but
    /// is free regarding the block and light data because it use shared reference.
    pub fn take_chunk_snapshot(&self, cx: i32, cz: i32) -> Option<ChunkSnapshot> {
        let chunk_comp = self.chunks.get(&(cx, cz))?;
        let chunk = chunk_comp.data.as_ref()?;
        Some(ChunkSnapshot {
            cx, 
            cz,
            chunk: Arc::clone(&chunk),
            entities: chunk_comp.entities.values()
                // Ignoring entities being updated, silently for now.
                .filter_map(|&index| self.entities.get(index).unwrap().inner.clone())
                .collect(),
            block_entities: chunk_comp.block_entities.iter()
                .filter_map(|(&pos, &index)| self.block_entities.get(index).unwrap().inner.clone()
                    .map(|e| (pos, e)))
                .collect(),
        })
    }

    /// Remove a chunk at given chunk coordinates and return a snapshot of it. If there
    /// is no chunk at the coordinates but entities or block entities are present, None
    /// is returned but entities and block entities are removed from the world.
    pub fn remove_chunk_snapshot(&mut self, cx: i32, cz: i32) -> Option<ChunkSnapshot> {
        
        let chunk_comp = self.chunks.remove(&(cx, cz))?;
        let mut ret = None;

        let entities = chunk_comp.entities.keys()
            .filter_map(|&id| self.remove_entity_inner(id, false, "remove chunk snapshot").unwrap().inner)
            .collect();
        
        let block_entities = chunk_comp.block_entities.keys()
            .filter_map(|&pos| self.remove_block_entity_inner(pos, false).unwrap().inner
                .map(|e| (pos, e)))
            .collect();
        
        if let Some(chunk) = chunk_comp.data {

            ret = Some(ChunkSnapshot { 
                cx, 
                cz,
                chunk,
                entities,
                block_entities,
            });

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Remove });

        }

        ret

    }

    // =================== //
    //        CHUNKS       //
    // =================== //

    /// Raw function to add a chunk to the world at the given coordinates. Note that the
    /// given chunk only contains block and light data, so no entity or block entity will
    /// be added by this function.
    /// 
    /// If any chunk is existing at this coordinate, it's just replaced and all entities
    /// and block entities are not touched.
    /// 
    /// Only entities and block entities that are in a chunk will be ticked.
    pub fn set_chunk(&mut self, cx: i32, cz: i32, chunk: Arc<Chunk>) {
       
        let chunk_comp = self.chunks.entry((cx, cz)).or_default();
        let was_unloaded = chunk_comp.data.replace(chunk).is_none();
        
        if was_unloaded {
            for &index in chunk_comp.entities.values() {
                self.entities.get_mut(index).unwrap().loaded = true;
            }
            for &index in chunk_comp.block_entities.values() {
                self.block_entities.get_mut(index).unwrap().loaded = true;
            }
        }
        
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Set });

    }

    /// Return true if a given chunk is present in the world.
    pub fn contains_chunk(&self, cx: i32, cz: i32) -> bool {
        self.chunks.get(&(cx, cz)).is_some_and(|c| c.data.is_some())
    }

    /// Get a reference to a chunk, if existing.
    pub fn get_chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        self.chunks.get(&(cx, cz)).and_then(|c| c.data.as_deref())
    }

    /// Get a mutable reference to a chunk, if existing.
    pub fn get_chunk_mut(&mut self, cx: i32, cz: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&(cx, cz)).and_then(|c| c.data.as_mut().map(Arc::make_mut))
    }

    /// Remove a chunk that may not exists. Note that this only removed the chunk data,
    /// not its entities and block entities.
    pub fn remove_chunk(&mut self, cx: i32, cz: i32) -> Option<Arc<Chunk>> {
        
        let chunk_comp = self.chunks.get_mut(&(cx, cz))?;
        let ret = chunk_comp.data.take();
        
        if ret.is_some() {

            for &index in chunk_comp.entities.values() {
                self.entities.get_mut(index).unwrap().loaded = false;
            }

            for &index in chunk_comp.block_entities.values() {
                self.block_entities.get_mut(index).unwrap().loaded = false;
            }

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Remove });

        }

        ret
        
    }

    // =================== //
    //        BLOCKS       //
    // =================== //

    /// Set block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned, but if it is existing the previous block and metadata
    /// is returned. This function also push a block change event and update lights
    /// accordingly.
    pub fn set_block(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {

        let (cx, cz) = calc_chunk_pos(pos)?;
        let chunk = self.get_chunk_mut(cx, cz)?;
        let (prev_id, prev_metadata) = chunk.get_block(pos);
        
        if id != prev_id || metadata != prev_metadata {

            chunk.set_block(pos, id, metadata);
            chunk.recompute_height(pos);

            // Schedule light updates if the block light properties have changed.
            if block::material::get_light_opacity(id) != block::material::get_light_opacity(prev_id)
            || block::material::get_light_emission(id) != block::material::get_light_emission(prev_id) {
                self.schedule_light_update(pos, LightKind::Block);
                self.schedule_light_update(pos, LightKind::Sky);
            }

            self.push_event(Event::Block { 
                pos, 
                inner: BlockEvent::Set {
                    id, 
                    metadata,
                    prev_id, 
                    prev_metadata, 
                } 
            });

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

        }

        Some((prev_id, prev_metadata))

    }

    /// Same as the [`set_block`] method, but the previous block and new block are 
    /// notified of that removal and addition.
    /// 
    /// [`set_block`]: Self::set_block
    pub fn set_block_self_notify(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = self.set_block(pos, id, metadata)?;
        self.notify_change_unchecked(pos, prev_id, prev_metadata, id, metadata);
        Some((prev_id, prev_metadata))
    }

    /// Same as the [`set_block_self_notify`] method, but additionally the blocks around 
    /// are notified of that neighbor change.
    /// 
    /// [`set_block_self_notify`]: Self::set_block_self_notify
    pub fn set_block_notify(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = self.set_block_self_notify(pos, id, metadata)?;
        self.notify_blocks_around(pos, id);
        Some((prev_id, prev_metadata))
    }

    /// Get block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned.
    pub fn get_block(&self, pos: IVec3) -> Option<(u8, u8)> {
        let (cx, cz) = calc_chunk_pos(pos)?;
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_block(pos))
    }

    // =================== //
    //        HEIGHT       //
    // =================== //

    /// Get saved height of a chunk column, Y component is ignored in the position.
    pub fn get_height(&self, pos: IVec3) -> Option<u8> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_height(pos))
    }

    // =================== //
    //        LIGHTS       //
    // =================== //

    /// Get light level at the given position, in range 0..16.
    pub fn get_light(&self, mut pos: IVec3) -> Light {
        
        if pos.y > 127 {
            pos.y = 127;
        }

        let mut light = Light {
            block: 0,
            sky: 15,
            sky_real: 0,
        };

        if let Some((cx, cz)) = calc_chunk_pos(pos) {
            if let Some(chunk) = self.get_chunk(cx, cz) {
                light.block = chunk.get_block_light(pos);
                light.sky = chunk.get_sky_light(pos);
            }
        }

        light.sky_real = light.sky.saturating_sub(self.sky_light_subtracted);
        light

    }

    /// Schedule a light update to be processed in a future tick.
    ///  
    /// See [`tick_light`](Self::tick_light).
    pub fn schedule_light_update(&mut self, pos: IVec3, kind: LightKind) {
        self.light_updates.push_back(LightUpdate { 
            kind,
            pos,
            credit: 15,
        });
    }

    /// Get the number of light updates remaining to process.
    #[inline]
    pub fn get_light_update_count(&self) -> usize {
        self.light_updates.len()
    }

    // =================== //
    //        BIOMES       //
    // =================== //

    /// Get the biome at some position (Y component is ignored).
    pub fn get_biome(&self, pos: IVec3) -> Option<Biome> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_biome(pos))
    }

    // =================== //
    //       ENTITIES      //
    // =================== //

    /// Internal function to ensure monomorphization and reduce bloat of the 
    /// generic [`spawn_entity`].
    #[inline(never)]
    fn spawn_entity_inner(&mut self, entity: Box<Entity>) -> u32 {

        // Get the next unique entity id.
        let id = self.entities_count;
        self.entities_count = self.entities_count.checked_add(1)
            .expect("entity count overflow");

        let kind = entity.kind();
        trace!("spawn entity #{id} ({:?})", kind);

        let (cx, cz) = calc_entity_chunk_pos(entity.0.pos);
        let chunk_comp = self.chunks.entry((cx, cz)).or_default();
        let entity_index = self.entities.push(EntityComponent {
            inner: Some(entity),
            id,
            cx,
            cz,
            loaded: chunk_comp.data.is_some(),
            kind,
        });

        chunk_comp.entities.insert(id, entity_index);
        self.entities_id_map.insert(id, entity_index);
        
        self.push_event(Event::Entity { id, inner: EntityEvent::Spawn });
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

        id

    }

    /// Spawn an entity in this world, this function gives it a unique id and ensure 
    /// coherency with chunks cache.
    /// 
    /// **This function is legal to call from ticking entities, but such entities will be
    /// ticked once in the same cycle as the currently ticking entity.**
    #[inline(always)]
    pub fn spawn_entity(&mut self, entity: impl Into<Box<Entity>>) -> u32 {
        // NOTE: This method is just a wrapper to erase generics.
        self.spawn_entity_inner(entity.into())
    }

    /// Return true if an entity is present from its id.
    pub fn contains_entity(&self, id: u32) -> bool {
        self.entities_id_map.contains_key(&id)
    }

    /// Return the number of entities in the world, loaded or not.
    #[inline]
    pub fn get_entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get a generic entity from its unique id. This generic entity can later be checked
    /// for being of a particular type. None can be returned if no entity is existing for
    /// this id or if the entity is the current entity being updated.
    pub fn get_entity(&self, id: u32) -> Option<&Entity> {
        let index = *self.entities_id_map.get(&id)?;
        self.entities.get(index).unwrap().inner.as_deref()
    }

    /// Get a generic entity from its unique id. This generic entity can later be checked
    /// for being of a particular type. None can be returned if no entity is existing for
    /// this id or if the entity is the current entity being updated.
    pub fn get_entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        let index = *self.entities_id_map.get(&id)?;
        self.entities.get_mut(index).unwrap().inner.as_deref_mut()
    }

    /// Remove an entity with given id, returning some boxed entity is successful. This
    /// returns true if the entity has been successfully removed removal, the entity's
    /// storage is guaranteed to be freed after return, but the entity footprint in the
    /// world will be cleaned only after ticking.
    pub fn remove_entity(&mut self, id: u32, reason: &str) -> bool {
        self.remove_entity_inner(id, true, reason).is_some()
    }

    /// Internal version of [`remove_entity`] that returns the removed component.
    /// 
    /// The caller can specify if the entity is known to be in an existing chunk
    /// component, if the caller know that the chunk component is no longer existing,
    /// it avoids panicking.
    /// 
    /// The given reason is only used for log tracing.
    fn remove_entity_inner(&mut self, id: u32, has_chunk: bool, reason: &str) -> Option<EntityComponent> {

        let index = self.entities_id_map.remove(&id)?;

        // Also remove the entity from the player map, if it was.
        self.entities_player_map.remove(&id);
        
        let comp = self.entities.remove(index);
        let swapped_index = self.entities.len();
        debug_assert_eq!(comp.id, id, "entity incoherent id");

        trace!("remove entity #{id} ({:?}): {reason}", comp.kind);
        
        // Directly remove the entity from its chunk if needed.
        let (cx, cz) = (comp.cx, comp.cz);
        if has_chunk {
            let removed_index = self.chunks.get_mut(&(cx, cz))
                .expect("entity chunk is missing")
                .entities
                .remove(&id);
            debug_assert_eq!(removed_index, Some(index), "entity is incoherent in its chunk");
        }

        // The entity that has been swapped has a new index, so we need to update its
        // index into the chunk cache...
        if let Some(swapped_comp) = self.entities.get(index) {

            let prev_index = self.entities_id_map.insert(swapped_comp.id, index);
            debug_assert_eq!(prev_index, Some(swapped_index), "swapped entity is incoherent");

            // Update the index of the entity within the player map, if it is a player.
            self.entities_player_map.entry(swapped_comp.id).and_modify(|i| *i = index);
            
            let (swapped_cx, swapped_cz) = (swapped_comp.cx, swapped_comp.cz);
            if has_chunk || (swapped_cx, swapped_cz) != (cx, cz) {
                
                let removed_index = self.chunks.get_mut(&(swapped_cx, swapped_cz))
                    .expect("swapped entity chunk is missing")
                    .entities
                    .insert(swapped_comp.id, index);
                debug_assert_eq!(removed_index, Some(swapped_index), "swapped entity is incoherent in its chunk");
            
            }

        }

        self.push_event(Event::Entity { id, inner: EntityEvent::Remove });
        if has_chunk {
            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
        }

        Some(comp)

    }
    
    // =================== //
    //   PLAYER ENTITIES   //
    // =================== //

    /// Set an entity that is already existing to be a player entity. Player entities are
    /// used as dynamic anchors in the world that are used for things like natural entity
    /// despawning when players are too far away, or for looking at players.
    /// 
    /// This methods returns true if the property has been successfully set.
    pub fn set_entity_player(&mut self, id: u32, player: bool) -> bool {
        let Some(&index) = self.entities_id_map.get(&id) else { return false };
        if player {
            self.entities_player_map.insert(id, index);
        } else {
            self.entities_player_map.remove(&id);
        }
        true
    }

    /// Returns true if the given entity by its id is a player entity. This also returns
    /// false if the entity isn't existing.
    pub fn is_entity_player(&mut self, id: u32) -> bool {
        self.entities_player_map.contains_key(&id)
    }

    /// Returns the number of player entities in the world, loaded or not.
    #[inline]
    pub fn get_entity_player_count(&self) -> usize {
        self.entities_player_map.len()
    }

    // =================== //
    //   BLOCK ENTITIES    //
    // =================== //

    /// Inner function to set block entity at given position, used to elide generics.
    #[inline(never)]
    fn set_block_entity_inner(&mut self, pos: IVec3, block_entity: Box<BlockEntity>) {

        trace!("set block entity {pos}");

        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        match self.block_entities_pos_map.entry(pos) {
            Entry::Occupied(o) => {

                // If there is current a block entity at this exact position, we'll just
                // replace it in-place, this avoid all the insertion of cache coherency.
                let index = *o.into_mut();
                self.block_entities.get_mut(index).unwrap().inner = Some(block_entity);
                // We also need to invalid the value to remove it from the current tick
                // linked list, if any ticking is currently happening.
                self.block_entities.invalidate(index);

                self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Remove });

            }
            Entry::Vacant(v) => {

                let chunk_comp = self.chunks.entry((cx, cz)).or_default();
                let block_entity_index = self.block_entities.push(BlockEntityComponent {
                    inner: Some(block_entity),
                    loaded: chunk_comp.data.is_some(),
                    pos,
                });

                chunk_comp.block_entities.insert(pos, block_entity_index);
                v.insert(block_entity_index);

            }
        }

        self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Set });
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

    }

    /// Set the block entity at the given position. If a block entity was already at the
    /// position, it is removed silently.
    #[inline(always)]
    pub fn set_block_entity(&mut self, pos: IVec3, block_entity: impl Into<Box<BlockEntity>>) {
        self.set_block_entity_inner(pos, block_entity.into());
    }

    /// Returns true if some block entity is present in the world.
    pub fn contains_block_entity(&self, pos: IVec3) -> bool {
        self.block_entities_pos_map.contains_key(&pos)
    }

    /// Return the number of block entities in the world, loaded or not.
    #[inline]
    pub fn get_block_entity_count(&self) -> usize {
        self.block_entities.len()
    }

    /// Get a block entity from its position.
    pub fn get_block_entity(&self, pos: IVec3) -> Option<&BlockEntity> {
        let index = *self.block_entities_pos_map.get(&pos)?;
        self.block_entities.get(index).unwrap().inner.as_deref()
    }

    /// Get a block entity from its position.
    pub fn get_block_entity_mut(&mut self, pos: IVec3) -> Option<&mut BlockEntity> {
        let index = *self.block_entities_pos_map.get(&pos)?;
        self.block_entities.get_mut(index).unwrap().inner.as_deref_mut()
    }

    /// Remove a block entity from a position. Returning true if successful, in this case
    /// the block entity storage is guaranteed to be freed, but the block entity footprint
    /// in this world will be definitely cleaned after ticking.
    pub fn remove_block_entity(&mut self, pos: IVec3) -> bool {
        self.remove_block_entity_inner(pos, true).is_some()
    }

    /// Internal version of `remove_block_entity` that returns the removed component.
    /// 
    /// The caller can specify if the block entity is known to be in an existing chunk
    /// component, if the caller know that the chunk component is no longer existing,
    /// it avoids panicking.
    fn remove_block_entity_inner(&mut self, pos: IVec3, has_chunk: bool) -> Option<BlockEntityComponent> {
        
        let index = self.block_entities_pos_map.remove(&pos)?;
        trace!("remove block entity {pos}");
        
        let comp = self.block_entities.remove(index);
        let swapped_index = self.block_entities.len();
        debug_assert_eq!(comp.pos, pos, "block entity incoherent position");
        
        // Directly remove the block entity from its chunk if needed.
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        if has_chunk {
            let removed_index = self.chunks.get_mut(&(cx, cz))
                .expect("block entity chunk is missing")
                .block_entities
                .remove(&pos);
            debug_assert_eq!(removed_index, Some(index), "block entity is incoherent in its chunk");
        }

        // A block entity has been swapped at the removed index, so we need to update any
        // reference to this block entity.
        if let Some(swapped_comp) = self.block_entities.get(index) {

            let prev_index = self.block_entities_pos_map.insert(swapped_comp.pos, index);
            debug_assert_eq!(prev_index, Some(swapped_index), "swapped block entity is incoherent");

            let (swapped_cx, swapped_cz) = calc_chunk_pos_unchecked(swapped_comp.pos);
            if has_chunk || (swapped_cx, swapped_cz) != (cx, cz) {
                
                let removed_index = self.chunks.get_mut(&(swapped_cx, swapped_cz))
                    .expect("swapped block entity chunk is missing")
                    .block_entities
                    .insert(swapped_comp.pos, index);
                debug_assert_eq!(removed_index, Some(swapped_index), "swapped block entity is incoherent in its chunk");
                
            }

        }

        self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Remove });
        if has_chunk {
            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
        }

        Some(comp)

    }

    // =================== //
    //   SCHEDULED TICKS   //
    // =================== //

    /// Schedule a tick update to happen at the given position, for the given block id
    /// and with a given delay in ticks. The block tick is not scheduled if a tick was
    /// already scheduled for that exact block id and position.
    pub fn schedule_block_tick(&mut self, pos: IVec3, id: u8, delay: u64) {

        let uid = self.block_ticks_count;
        self.block_ticks_count = self.block_ticks_count.checked_add(1)
            .expect("scheduled ticks count overflow");

        let state = BlockTickState { pos, id };
        if self.block_ticks_states.insert(state) {
            self.block_ticks.insert(BlockTick { time: self.time + delay, state, uid });
        }

    }

    /// Return the current number of scheduled block ticks waiting.
    #[inline]
    pub fn get_block_tick_count(&self) -> usize {
        self.block_ticks.len()
    }

    // =================== //
    //      ITERATORS      //
    // =================== //

    /// Iterate over all blocks in the given area where max is excluded.
    #[inline]
    pub fn iter_blocks_in(&self, min: IVec3, max: IVec3) -> BlocksInIter<'_> {
        BlocksInIter::new(self, min, max)
    }

    /// Iterate over all blocks in the chunk at given coordinates.
    #[inline]
    pub fn iter_blocks_in_chunk(&self, cx: i32, cz: i32) -> BlocksInChunkIter<'_> {
        BlocksInChunkIter::new(self, cx, cz)
    }

    /// Iterate over all entities in the world.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities(&self) -> EntitiesIter<'_> {
        EntitiesIter(self.entities.iter())
    }

    /// Iterator over all entities in the world through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_mut(&mut self) -> EntitiesIterMut<'_> {
        EntitiesIterMut(self.entities.iter_mut())
    }

    /// Iterate over all player entities in the world.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_player_entities(&self) -> PlayerEntitiesIter<'_> {
        PlayerEntitiesIter {
            indices: Some(self.entities_player_map.values()),
            entities: &self.entities,
        }
    }

    /// Iterate over all player entities in the world through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_player_entities_mut(&mut self) -> PlayerEntitiesIterMut<'_> {
        PlayerEntitiesIterMut {
            indices: Some(self.entities_player_map.values()),
            entities: &mut self.entities,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }
    }

    /// Iterate over all entities of the given chunk.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_in_chunk(&self, cx: i32, cz: i32) -> EntitiesInChunkIter<'_> {
        EntitiesInChunkIter {
            indices: self.chunks.get(&(cx, cz)).map(|comp| comp.entities.values()),
            entities: &self.entities,
        }
    }

    /// Iterate over all entities of the given chunk through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_in_chunk_mut(&mut self, cx: i32, cz: i32) -> EntitiesInChunkIterMut<'_> {
        EntitiesInChunkIterMut {
            indices: self.chunks.get(&(cx, cz)).map(|comp| comp.entities.values()),
            entities: &mut self.entities,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }
    }

    /// Iterate over all entities colliding with the given bounding box.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_colliding(&self, bb: BoundingBox) -> EntitiesCollidingIter<'_> {

        let (start_cx, start_cz) = calc_entity_chunk_pos(bb.min - 2.0);
        let (end_cx, end_cz) = calc_entity_chunk_pos(bb.max + 2.0);

        EntitiesCollidingIter {
            chunks: ChunkComponentsIter { 
                chunks: &self.chunks, 
                range: ChunkRange::new(start_cx, start_cz, end_cx, end_cz) },
            indices: None,
            entities: &self.entities,
            bb,
        }

    }

    /// Iterate over all entities colliding with the given bounding box through mut ref.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_colliding_mut(&mut self, bb: BoundingBox) -> EntitiesCollidingIterMut<'_> {
        
        let (start_cx, start_cz) = calc_entity_chunk_pos(bb.min - 2.0);
        let (end_cx, end_cz) = calc_entity_chunk_pos(bb.max + 2.0);

        EntitiesCollidingIterMut {
            chunks: ChunkComponentsIter { 
                chunks: &self.chunks, 
                range: ChunkRange::new(start_cx, start_cz, end_cx, end_cz) },
            indices: None,
            entities: &mut self.entities,
            bb,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }

    }

    /// Return true if any entity is colliding the given bounding box. The hard argument
    /// can be set to true in order to only check for "hard" entities, hard entities can
    /// prevent block placements and entity spawning.
    pub fn has_entity_colliding(&self, bb: BoundingBox, hard: bool) -> bool {
        self.iter_entities_colliding(bb)
            .any(|(_, entity)| !hard || entity.kind().is_hard())
    }

    // =================== //
    //       TICKING       //
    // =================== //
    
    /// Tick the world, this ticks all entities.
    /// TODO: Guard this from being called recursively from tick functions.
    pub fn tick(&mut self) {

        if self.time % 20 == 0 {
            // println!("time: {}", self.time);
            // println!("weather: {:?}", self.weather);
            // println!("weather_next_time: {}", self.weather_next_time);
            // println!("sky_light_subtracted: {}", self.sky_light_subtracted);
        }

        self.tick_weather();
        // TODO: Wake up all sleeping player if day time.
        
        self.tick_natural_spawn();

        self.tick_sky_light();

        self.time += 1;

        self.tick_blocks();
        self.tick_entities();
        self.tick_block_entities();

        self.tick_light(1000);
        
    }

    /// Update current weather in the world.
    fn tick_weather(&mut self) {

        // No weather in the nether.
        if self.dimension == Dimension::Nether {
            return;
        }

        // When it's time to recompute weather.
        if self.time >= self.weather_next_time {

            // Don't update weather on first world tick.
            if self.time != 0 {
                let new_weather = match self.weather {
                    Weather::Clear => self.rand.next_choice(&[Weather::Rain, Weather::Thunder]),
                    _ => self.rand.next_choice(&[self.weather, Weather::Clear]),
                };
                self.set_weather(new_weather);
            }

            let bound = if self.weather == Weather::Clear { 168000 } else { 12000 };
            let delay = self.rand.next_int_bounded(bound) as u64 + 12000;
            self.weather_next_time = self.time + delay;

        }

    }

    /// Do natural animal and mob spawning in the world.
    fn tick_natural_spawn(&mut self) {

        /// The maximum manhattan distance a chunk can be loaded.
        const CHUNK_MAX_DIST: u32 = 8;
        /// The minimum distance required from any player entity to spawn.
        const SPAWN_MIN_DIST_SQUARED: f64 = 24.0 * 24.0;

        // Categories of entities to spawn, also used to count how many are currently 
        // loaded in the world. We have 4 slots in this array because there are 4
        // entity categories.
        let mut categories_count = [0; EntityCategory::ALL.len()];

        // Count every entity category.
        for comp in self.entities.iter() {
            if comp.loaded {
                if let Some(entity) = comp.inner.as_deref() {
                    categories_count[entity.category() as usize] += 1;
                }
            }
        }

        // Temporary list of chunks loaded by data and players in range.
        let mut loaded_chunks = LOADED_CHUNKS.take();
        loaded_chunks.clear();
        loaded_chunks.extend(self.chunks.iter()
            .filter_map(|(&pos, comp)| comp.data.is_some().then_some(pos)));
        loaded_chunks.retain(|&(cx, cz)| {
            self.entities_player_map.values()
                .map(|&index| self.entities.get(index).unwrap())
                .any(|comp| comp.cx.abs_diff(cx) <= CHUNK_MAX_DIST && comp.cz.abs_diff(cz) <= CHUNK_MAX_DIST)
        });

        for category in EntityCategory::ALL {

            let max_world_count = category.natural_spawn_max_world_count();

            // Skip the category if it cannot spawn.
            if max_world_count == 0 {
                continue;
            }
            // Skip the category if it already has enough loaded entities.
            if categories_count[category as usize] > max_world_count * self.chunks.len() / 256 {
                continue;
            }

            for &(cx, cz) in &loaded_chunks {

                // Temporary borrowing of chunk data to query biome and block.
                let chunk = self.chunks.get(&(cx, cz)).unwrap();
                let chunk_data = chunk.data.as_deref().unwrap();

                let biome = chunk_data.get_biome(IVec3::ZERO);
                let kinds = biome.natural_entity_kinds(category);

                // Ignore this chunk is its biome cannot spawn any entity.
                if kinds.is_empty() {
                    continue;
                }

                // Next we pick a random spawn position within the chunk and check it.
                let center_pos = IVec3 {
                    x: cx * 16 + self.rand.next_int_bounded(16),
                    y: self.rand.next_int_bounded(128),
                    z: cz * 16 + self.rand.next_int_bounded(16),
                };

                // If the block is not valid to spawn the category in, skip chunk.
                let (block, _) = chunk_data.get_block(center_pos);
                if block::material::get_material(block) != category.natural_spawn_material() {
                    continue;
                }

                let chance_sum = kinds.iter().map(|kind| kind.chance).sum::<u16>();
                let index = self.rand.next_int_bounded(chance_sum as i32) as u16;
                let mut chance_acc = 0;
                let mut kind = kinds[0].kind;

                for test_kind in kinds {
                    chance_acc += test_kind.chance;
                    if index < chance_acc {
                        kind = test_kind.kind;
                        break;
                    }
                }

                // Keep the maximum chunk count to compare with spawn count.
                let max_chunk_count = kind.natural_spawn_max_chunk_count();

                // Keep track of the total number of entity spawned in that chunk.
                let mut spawn_count = 0usize;

                'pack: for _ in 0..3 {

                    let mut spawn_pos = center_pos;

                    'chain: for _ in 0..4 {

                        spawn_pos += IVec3 {
                            x: self.rand.next_int_bounded(6) - self.rand.next_int_bounded(6),
                            y: self.rand.next_int_bounded(1) - self.rand.next_int_bounded(1),
                            z: self.rand.next_int_bounded(6) - self.rand.next_int_bounded(6),
                        };

                        // Preliminary check if the block position is valid.
                        if category == EntityCategory::WaterAnimal {

                            // Water animals can only spawn in liquid.
                            if !self.get_block_material(spawn_pos).is_fluid() {
                                continue;
                            }

                            // Water animals cannot spawn if above block is opaque.
                            if self.is_block_opaque_cube(spawn_pos + IVec3::Y) {
                                continue;
                            }

                        } else {
                            
                            // The 2 block column should not be opaque cube.
                            if self.is_block_opaque_cube(spawn_pos) || self.is_block_opaque_cube(spawn_pos + IVec3::Y) {
                                continue;
                            }

                            // Block below should be opaque.
                            if !self.is_block_opaque_cube(spawn_pos - IVec3::Y) {
                                continue;
                            }

                            // PARITY: We don't do the fluid block check because it would
                            // be redundant with the check in 'can_natural_spawn'.

                        }

                        let spawn_pos = spawn_pos.as_dvec3() + DVec3::new(0.5, 0.0, 0.5);

                        // PARITY: We check that this entity would be in the 128.0 block 
                        // no-despawn range of at least one player. This avoid entities
                        // to be instantly removed after spawning.
                        let mut close_player = false;
                        for (_, Entity(player_base, _)) in self.iter_player_entities() {
                            // If there is a player too close to that spawn point, abort.
                            let player_dist_sq = player_base.pos.distance_squared(spawn_pos);
                            if player_dist_sq < SPAWN_MIN_DIST_SQUARED {
                                continue 'chain;
                            } else if player_dist_sq <= 128.0 * 128.0 {
                                close_player = true;
                            }
                        }

                        // Skip if no player is in range to keep this natural entity.
                        if !close_player {
                            continue;
                        }

                        // TODO: Do not spawn inside spawn chunks

                        let mut entity = kind.new_default(spawn_pos);
                        entity.0.persistent = true;
                        entity.0.look.x = self.rand.next_float() * std::f32::consts::TAU;

                        // Important to init natural spawn before checking if it can spawn
                        // because slime may be resized, so this can change the bb.
                        entity.init_natural_spawn(self);

                        // Skip if the entity cannot be spawned.
                        if !entity.can_natural_spawn(self) {
                            continue;
                        }

                        // TODO: Spawning spider has 1% chance of being spider jockey.
                        // TODO: Random sheep wool color.

                        self.spawn_entity(entity);
                        spawn_count += 1;
                        if spawn_count >= max_chunk_count {
                            break 'pack;
                        }

                    }

                }

            }

        }

        // To avoid too short allocation...
        LOADED_CHUNKS.set(loaded_chunks);

    }

    /// Update the sky light value depending on the current time, it is then used to get
    /// the real light value of blocks.
    fn tick_sky_light(&mut self) {

        let time_wrapped = self.time % 24000;
        let mut half_turn = (time_wrapped as f32 + 1.0) / 24000.0 - 0.25;

        if half_turn < 0.0 {
            half_turn += 1.0;
        } else if half_turn > 1.0 {
            half_turn -= 1.0;
        }

        let celestial_angle = match self.dimension {
            Dimension::Nether => 0.5,
            _ => half_turn + (1.0 - ((half_turn * std::f32::consts::PI).cos() + 1.0) / 2.0 - half_turn) / 3.0,
        };

        let factor = (celestial_angle * std::f32::consts::TAU).cos() * 2.0 + 0.5;
        let factor = factor.clamp(0.0, 1.0);
        let factor = match self.weather {
            Weather::Clear => 1.0,
            Weather::Rain => 0.6875,
            Weather::Thunder => 0.47265625,
        } * factor;

        self.sky_light_subtracted = ((1.0 - factor) * 11.0) as u8;

    }

    /// Internal function to tick the internal scheduler.
    fn tick_blocks(&mut self) {

        debug_assert_eq!(self.block_ticks.len(), self.block_ticks_states.len());

        // Schedule ticks...
        while let Some(tick) = self.block_ticks.first() {
            if self.time > tick.time {
                // This tick should be activated.
                let tick = self.block_ticks.pop_first().unwrap();
                assert!(self.block_ticks_states.remove(&tick.state));
                // Check coherency of the scheduled tick and current block.
                if let Some((id, metadata)) = self.get_block(tick.state.pos) {
                    if id == tick.state.id {
                        self.tick_block_unchecked(tick.state.pos, id, metadata, false);
                    }
                }
            } else {
                // Our set is ordered by time first, so we break when past current time. 
                break;
            }
        }

        // Random ticking...
        let mut pending_random_ticks = RANDOM_TICKS_PENDING.take();
        debug_assert!(pending_random_ticks.is_empty());

        // Random tick only on loaded chunks.
        for (&(cx, cz), chunk) in &mut self.chunks {
            if let Some(chunk_data) = &chunk.data {

                // TODO: Lightning strikes.
                // TODO: Random snowing.

                let chunk_pos = IVec3::new(cx * CHUNK_WIDTH as i32, 0, cz * CHUNK_WIDTH as i32);
                
                // Minecraft run 80 random ticks per tick per chunk.
                for _ in 0..80 {

                    self.random_ticks_seed = self.random_ticks_seed
                        .wrapping_mul(3)
                        .wrapping_add(1013904223);

                    let rand = self.random_ticks_seed >> 2;
                    let pos = IVec3::new((rand >> 0) & 15, (rand >> 16) & 127, (rand >> 8) & 15);

                    let (id, metadata) = chunk_data.get_block(pos);
                    pending_random_ticks.push((chunk_pos + pos, id, metadata));

                }

            }
        }

        for (pos, id, metadata) in pending_random_ticks.drain(..) {
            self.tick_block_unchecked(pos, id, metadata, true);
        }

        RANDOM_TICKS_PENDING.set(pending_random_ticks);

    }

    /// Internal function to tick all entities.
    fn tick_entities(&mut self) {

        self.entities.reset();

        while let Some((_, comp)) = self.entities.current_mut() {

            if !comp.loaded {
                self.entities.advance();
                continue;
            }

            let mut entity = comp.inner.take()
                .expect("entity was already being updated");

            let id = comp.id;
            let (prev_cx, prev_cz) = (comp.cx, comp.cz);
            entity.tick(&mut *self, id);

            // Get the component again, the entity may have been removed.
            if let Some((index, comp)) = self.entities.current_mut() {

                debug_assert_eq!(comp.id, id, "entity id incoherent");

                // Check if the entity moved to another chunk...
                let (new_cx, new_cz) = calc_entity_chunk_pos(entity.0.pos);
                comp.inner = Some(entity);

                if (prev_cx, prev_cz) != (new_cx, new_cz) {

                    // NOTE: This part is really critical as this ensures Memory Safety
                    // in iterators and therefore avoids Undefined Behaviors. Each entity
                    // really needs to be in a single chunk at a time.
                    
                    let removed_index = self.chunks.get_mut(&(prev_cx, prev_cz))
                        .expect("entity previous chunk is missing")
                        .entities.remove(&id);
                    debug_assert_eq!(removed_index, Some(index), "entity is incoherent in its previous chunk");

                    // Update the world entity to its new chunk and orphan state.
                    comp.cx = new_cx;
                    comp.cz = new_cz;

                    // Insert the entity in its new chunk.
                    let new_chunk_comp = self.chunks.entry((new_cx, new_cz)).or_default();
                    let insert_success = new_chunk_comp.entities.insert(id, index).is_none();
                    debug_assert!(insert_success, "entity was already present in its new chunk");
                    // Update the loaded flag of the entity depending on the new chunk
                    // being loaded or not.
                    comp.loaded = new_chunk_comp.data.is_some();

                    self.push_event(Event::Chunk { cx: prev_cx, cz: prev_cz, inner: ChunkEvent::Dirty });
                    self.push_event(Event::Chunk { cx: new_cx, cz: new_cz, inner: ChunkEvent::Dirty });

                }

            }

            self.entities.advance();

        }

    }

    fn tick_block_entities(&mut self) {

        self.block_entities.reset();

        while let Some((_, comp)) = self.block_entities.current_mut() {
            
            if !comp.loaded {
                self.block_entities.advance();
                continue;
            }

            let mut block_entity = comp.inner.take()
                .expect("block entity was already being updated");

            let pos = comp.pos;
            block_entity.tick(self, pos);

            // Get the component again and re-insert the block entity.
            if let Some((_, comp)) = self.block_entities.current_mut() {
                comp.inner = Some(block_entity);
            }

            self.block_entities.advance();

        }

    }

    /// Tick pending light updates for a maximum number of light updates. This function
    /// returns true only if all light updates have been processed.
    pub fn tick_light(&mut self, limit: usize) {

        // IMPORTANT NOTE: This algorithm is terrible but works, I've been trying to come
        // with a better one but it has been too complicated so far.
        
        for _ in 0..limit {

            let Some(update) = self.light_updates.pop_front() else { break };

            let mut max_face_emission = 0;
            for face in Face::ALL {

                let face_pos = update.pos + face.delta();

                let Some((cx, cz)) = calc_chunk_pos(face_pos) else { continue };
                let Some(chunk) = self.get_chunk_mut(cx, cz) else { continue };

                let face_emission = match update.kind {
                    LightKind::Block => chunk.get_block_light(face_pos),
                    LightKind::Sky => chunk.get_sky_light(face_pos),
                };

                max_face_emission = max_face_emission.max(face_emission);
                if max_face_emission == 15 {
                    break;
                }

            }

            let Some((cx, cz)) = calc_chunk_pos(update.pos) else { continue };
            let Some(chunk) = self.get_chunk_mut(cx, cz) else { continue };

            let (id, _) = chunk.get_block(update.pos);
            let opacity = block::material::get_light_opacity(id).max(1);

            let emission = match update.kind {
                LightKind::Block => block::material::get_light_emission(id),
                LightKind::Sky => {
                    // If the block is above ground, then it has
                    let column_height = chunk.get_height(update.pos) as i32;
                    if update.pos.y >= column_height { 15 } else { 0 }
                }
            };

            let new_light = emission.max(max_face_emission.saturating_sub(opacity));
            let mut changed = false;
            let mut sky_exposed = false;

            match update.kind {
                LightKind::Block => {
                    if chunk.get_block_light(update.pos) != new_light {
                        chunk.set_block_light(update.pos, new_light);
                        changed = true;
                    }
                }
                LightKind::Sky => {
                    if chunk.get_sky_light(update.pos) != new_light {
                        chunk.set_sky_light(update.pos, new_light);
                        changed = true;
                        sky_exposed = emission == 15;
                    }
                }
            }

            if changed {
                self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
            }

            if changed && update.credit >= 1 {
                for face in Face::ALL {
                    // Do not propagate light upward when the updated block is above 
                    // ground, so all blocks above are also exposed and should already
                    // be at max level.
                    if face == Face::PosY && sky_exposed {
                        continue;
                    }
                    self.light_updates.push_back(LightUpdate {
                        kind: update.kind,
                        pos: update.pos + face.delta(),
                        credit: update.credit - 1,
                    });
                }
            }
            
        }

    }

}


/// Types of dimensions, used for ambient effects in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// The overworld dimension with a blue sky and day cycles.
    Overworld,
    /// The creepy nether dimension.
    Nether,
}

/// Type of weather currently in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weather {
    /// The weather is clear.
    Clear,
    /// It is raining.
    Rain,
    /// It is thundering.
    Thunder,
}

/// Light values of a position in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Light {
    /// Block light level.
    pub block: u8,
    /// Sky light level.
    pub sky: u8,
    /// The real sky light level, depending on the time and weather.
    pub sky_real: u8,
}

impl Light {

    /// Calculate the maximum static light level (without time/weather attenuation).
    #[inline]
    pub fn max(self) -> u8 {
        u8::max(self.block, self.sky)
    }

    /// Calculate the maximum real light level (with time/weather attenuation).
    #[inline]
    pub fn max_real(self) -> u8 {
        u8::max(self.block, self.sky_real)
    }

    /// Calculate the block brightness from its light levels.
    #[inline]
    pub fn brightness(self) -> f32 {
        // TODO: In nether, OFFSET is 0.1
        const OFFSET: f32 = 0.05;
        let base = 1.0 - self.max_real() as f32 / 15.0;
        (1.0 - base) * (base * 3.0 + 1.0) * (1.0 - OFFSET) + OFFSET
    }

}

/// Different kind of lights in the word.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    /// Block light level, the light spread in all directions and blocks have a minimum 
    /// opacity of 1 in all directions, each block has its own light emission.
    Block,
    /// Sky light level, same as block light but light do not decrease when going down
    /// and every block above height have is has an emission of 15.
    Sky,
}

/// An event that happened in the world.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// An event with a block.
    Block {
        /// The position of the block.
        pos: IVec3,
        /// Inner block event.
        inner: BlockEvent,
    },
    /// An event with an entity given its id.
    Entity {
        /// The unique id of the entity.
        id: u32,
        /// Inner entity event.
        inner: EntityEvent,
    },
    /// A block entity has been set at this position.
    BlockEntity {
        /// The block entity position.
        pos: IVec3,
        /// Inner block entity event.
        inner: BlockEntityEvent,
    },
    /// A chunk event.
    Chunk {
        /// The chunk X position.
        cx: i32,
        /// The chunk Z position.
        cz: i32,
        /// Inner chunk event.
        inner: ChunkEvent,
    },
    /// The weather in the world has changed.
    Weather {
        /// Previous weather in the world.
        prev: Weather,
        /// New weather in the world.
        new: Weather,
    },
    /// Explode blocks.
    Explode {
        /// Center position of the explosion.
        center: DVec3,
        /// Radius of the explosion around center.
        radius: f32,
    },
    /// An event to debug and spawn block break particles at the given position.
    DebugParticle {
        /// The block position to spawn particles at.
        pos: IVec3,
        /// The block to break at this position.
        block: u8,
    }
}

/// An event with a block.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockEvent {
    /// A block has been changed in the world.
    Set {
        /// The new block id.
        id: u8,
        /// The new block metadata.
        metadata: u8,
        /// Previous block id.
        prev_id: u8,
        /// Previous block metadata.
        prev_metadata: u8,
    },
    /// Play the block activation sound at given position and id/metadata.
    Sound {
        /// Current id of the block.
        id: u8,
        /// Current metadata of the block.
        metadata: u8,
    },
}

/// An event with an entity.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityEvent {
    /// The entity has been spawned. The initial chunk position is given.
    Spawn,
    /// The entity has been removed. The last chunk position is given.
    Remove,
    /// The entity changed its position.
    Position {
        pos: DVec3,
    },
    /// The entity changed its look.
    Look {
        look: Vec2,
    },
    /// The entity changed its velocity.
    Velocity {
        vel: DVec3,
    },
    /// The entity has picked up another entity, such as arrow or item. Note that the
    /// target entity is not removed by this event, it's only a hint that this happened
    /// just before the entity may be removed.
    Pickup {
        /// The id of the picked up entity.
        target_id: u32,
    },
    /// The entity is damaged and the damage animation should be played by frontend.
    Damage,
    /// The entity is dead and the dead animation should be played by frontend.
    Dead,
    /// Some unspecified entity metadata has changed.
    Metadata,
}

/// An event with a block entity.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockEntityEvent {
    /// The block entity has been set at its position.
    Set,
    /// The block entity has been removed at its position.
    Remove,
    /// A block entity have seen some of its stored item stack changed.
    Storage {
        /// The storage targeted by this event.
        storage: BlockEntityStorage,
        /// The next item stack at this index.
        stack: ItemStack,
    },
    /// A block entity has made some progress.
    Progress {
        /// The kind of progress targeted by this event.
        progress: BlockEntityProgress,
        /// Progress value.
        value: u16,
    },
}

/// Represent the storage slot for a block entity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockEntityStorage {
    /// The storage slot is referencing a classic linear inventory at given index.
    Standard(u8),
    FurnaceInput,
    FurnaceOutput,
    FurnaceFuel,
}

/// Represent the progress update for a block entity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockEntityProgress {
    FurnaceSmeltTime,
    FurnaceBurnMaxTime,
    FurnaceBurnRemainingTime,
}

/// An event with a chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkEvent {
    /// The chunk has been set at its position. A chunk may have been replaced at that
    /// position.
    Set,
    /// The chunk has been removed from its position.
    Remove,
    /// Any chunk component (block, light, entity, block entity) has been modified in the
    /// chunk so it's marked dirty.
    Dirty,
}

/// A snapshot contains all of the content within a chunk, block, light, height map,
/// entities and block entities are all included. This structure can be considered as
/// a "view" because the chunk data (around 80 KB) is referenced to with a [`Arc`], that
/// allows either uniquely owning it, or sharing it with a world, which is the case when
/// saving a chunk.
#[derive(Clone)]
pub struct ChunkSnapshot {
    /// The X chunk coordinate.
    pub cx: i32,
    /// The Z chunk coordinate.
    pub cz: i32,
    /// The block, light and height map data of the chunk.
    pub chunk: Arc<Chunk>,
    /// The entities in that chunk, note that entities are not guaranteed to have a 
    /// position that is within chunk boundaries.
    pub entities: Vec<Box<Entity>>,
    /// Block entities in that chunk, all block entities are mapped to their absolute
    /// coordinates in the world.
    pub block_entities: HashMap<IVec3, Box<BlockEntity>>,
}

impl ChunkSnapshot {

    /// Create a new empty chunk view of the given coordinates.
    pub fn new(cx: i32, cz: i32) -> Self {
        Self {
            cx,
            cz,
            chunk: Chunk::new(),
            entities: Vec::new(),
            block_entities: HashMap::new(),
        }
    }

}


/// This internal structure is used to keep data associated to a chunk coordinate X/Z.
/// It could store chunk data, entities and block entities when present. If a world chunk
/// does not contain data, it is considered **unloaded**. It is also impossible to get
/// a snapshot of an unloaded chunk.
/// 
/// Entities and block entities in **unloaded** chunks are no longer updated as soon as
/// they enter that unloaded chunk.
/// 
/// Note: cloning a chunk component will also clone the chunk's Arc, therefore the whole
/// chunk content is actually cloned only when written to.
#[derive(Default, Clone)]
struct ChunkComponent {
    /// Underlying chunk. This is important to understand why the data chunk is stored 
    /// in an Atomically Reference-Counted container: first the chunk structure is large
    /// (around 80 KB) so we want it be stored in heap while the Arc container allows us
    /// to work with the chunk in a Clone-On-Write manner.
    /// 
    /// In normal conditions, this chunk will not be shared and so it could be mutated 
    /// using the [`Arc::get_mut`] method that allows mutating the Arc's value if only
    /// one reference exists. But there are situations when we want to have more 
    /// references to that chunk data, for example when saving the chunk we'll temporarily
    /// create a Arc referencing this chunk and pass it to the threaded loader/saver.
    /// If the chunk is mutated while being saved, we'll just clone it and replace this
    /// Arc with a new one that, by definition, has only one reference, all of this based
    /// on the [`Arc::make_mut`] method. Depending on save being fast or not, this clone
    /// will be more or less likely to happen.
    data: Option<Arc<Chunk>>,
    /// Entities belonging to this chunk.
    entities: IndexMap<u32, usize>,
    /// Block entities belonging to this chunk.
    block_entities: HashMap<IVec3, usize>,
}

/// Internal type for storing a world entity and keep track of its current chunk.
#[derive(Debug, Clone)]
struct EntityComponent {
    /// The entity storage.
    inner: Option<Box<Entity>>,
    /// Unique entity id is duplicated here to allow us to access it event when entity
    /// is updating.
    id: u32,
    /// The chunk X coordinate where this component is cached.
    cx: i32,
    /// The chunk Z coordinate where this component is cached.
    cz: i32,
    /// True when the chunk this entity is in is loaded with data.
    loaded: bool,
    /// This field describes the initial entity kind of the entity when spawned, it should
    /// not be changed afterward by ticking functions.
    kind: EntityKind,
}

/// Internal type for storing a world block entity.
#[derive(Debug, Clone)]
struct BlockEntityComponent {
    /// The block entity storage.
    inner: Option<Box<BlockEntity>>,
    /// True when the chunk this block entity is in is loaded with data.
    loaded: bool,
    /// Position of that block entity.
    pos: IVec3,
}

/// A block tick position, this is always linked to a [`ScheduledTick`] being added to
/// the tree map, this structure is also stored appart in order to check that two ticks
/// are not scheduled for the same position and block id.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct BlockTickState {
    /// Position of the block to tick.
    pos: IVec3,
    /// The expected id of the block, if the block has no longer this id, this tick is
    /// ignored.
    id: u8,
}

/// A block tick scheduled in the future, it's associated to a world time in a tree map.
/// This structure is ordered by time and then by position, this allows to have multiple
/// block update at the same time but for different positions.
#[derive(Clone, Eq)]
struct BlockTick {
    /// This tick unique id within the world.
    uid: u64,
    /// The time to tick the block.
    time: u64,
    /// State of that scheduled tick.
    state: BlockTickState,
}

impl PartialEq for BlockTick {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid && self.time == other.time
    }
}

impl PartialOrd for BlockTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for BlockTick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
            .then(self.uid.cmp(&other.uid))
    }
}

/// A light update to apply to the world.
#[derive(Clone)]
struct LightUpdate {
    /// Light kind targeted by this update, the update only applies to one of the kind.
    kind: LightKind,
    /// The position of the light update.
    pos: IVec3,
    /// Credit remaining to update light, this is used to limit the number of updates
    /// produced by a block chance initial update. Initial value is something like 15
    /// and decrease for each propagation, when it reaches 0 the light update stops 
    /// propagating.
    credit: u8,
}

/// A tick vector is an internal structure used for both entities and block entities,
/// it acts as a dynamically linked list where where the iteration order is defined before
/// ticking and where insertion and removal of elements doesn't not affect the ordering.
/// 
/// We use this complex data structure because we want to avoid most of the overhead when
/// ticking entities and block entities, so we want to avoid moving entities (even the
/// pointers) from/to stack too much. We also want to keep cache efficiency, this is why
/// we recompute the linked list upon modifications in order to have a ascending pointer
/// iteration by default, that may be invalidated if any value is removed.
#[derive(Clone)]
struct TickVec<T> {
    /// The inner vector containing all cells with inserted values.
    inner: Vec<TickCell<T>>,
    /// This boolean indicate if the any value has been insert or removed from the vector.
    /// This avoids recomputing the tick linked list on every tick reset.
    modified: bool,
    /// The index of the cell currently ticked.
    index: usize,
    /// Set to true if the cell currently ticked has been invalidated.
    invalidated: bool,
}

/// A cell in the tick vector, its contains previous and next index of the cell to tick.
#[derive(Clone)]
struct TickCell<T> {
    /// The actual value stored.
    value: T,
    /// The index of the previous value to tick.
    /// Set to the cell index itself it there is no previous cell to tick.
    prev: usize,
    /// The index of the next value to tick.
    /// Set to the cell index itself it there is no previous cell to tick.
    next: usize,
}

/// A borrowed version of the tick vector, this is better than referencing the vector
/// because it avoid one level of indirection.
#[repr(transparent)]
struct TickSlice<T>([TickCell<T>]);

impl<T> TickVec<T> {

    /// This is our end of list sentinel.
    const END: usize = usize::MAX;

    /// Construct a new empty tick vector, this doesn't not allocate.
    #[inline]
    const fn new() -> Self {
        Self {
            inner: Vec::new(),
            modified: false,
            index: Self::END,
            invalidated: false,
        }
    }

    /// Push a new element at the end of this vector, the value index is also returned.
    #[inline]
    fn push(&mut self, value: T) -> usize {
        let index = self.inner.len();
        self.inner.push(TickCell {
            value,
            prev: Self::END,
            next: Self::END,
        });
        self.modified = true;
        index
    }

    /// Invalid the value at the given index, therefore removing it from any tick linked
    /// list. If it is currently being updated
    fn invalidate(&mut self, index: usize) {

        let cell = &mut self.inner[index];
        let prev = mem::replace(&mut cell.prev, Self::END);
        let next = mem::replace(&mut cell.next, Self::END);

        // Start by updating the link of the previous/next cells for this removed cell.
        if prev != Self::END {
            // This condition is to keep the end-of-chain property.
            self.inner[prev].next = next;
        }

        if next != Self::END {
            self.inner[next].prev = prev;
        }

        // The last thing to keep in-sync is the tick index, if it was referencing the
        // removed value or the swapped one.
        if self.index == index {
            self.invalidated = true;
            self.index = next;
        }

    }

    /// Remove an element in this vector at the given index, the element is swapped with
    /// the last element of the vector.
    fn remove(&mut self, index: usize) -> T {
        
        // The first step is not to remove the cell, but remove it from its linked list.
        // This simplifies at lot the swapped cell update because we know that the removed
        // cell cannot be in any linked list, and so not in the swapped cell linked list.
        self.invalidate(index);

        // Once it is no longer in a linked list, swap remove it.
        let cell = self.inner.swap_remove(index);

        // This is the previous index of the cell that was swapped in place, if any.
        let swapped_index = self.inner.len();

        // If a cell has been swapped in place of the removed index, we need to update
        // the cell referencing it as its next ticking cell.
        if let Some(&TickCell { prev, next, .. }) = self.inner.get(index) {

            // Relink the previous cell to the new index of this one.
            if prev != Self::END {
                self.inner[prev].next = index;
            }

            // Relink the next cell to the next index of this one.
            if next != Self::END {
                self.inner[next].prev = index;
            }
            
        }

        if self.index == swapped_index {
            // Just update to the next index of the swapped value.
            self.index = index;
        }
        
        self.modified = true;
        
        cell.value

    }

    /// Reset the current tick index.
    fn reset(&mut self) {
        
        if self.inner.is_empty() {
            self.index = Self::END;
            return;
        }

        self.index = 0;

        // Update the tick linked list...
        if mem::take(&mut self.modified) {

            for i in 1..self.inner.len() {
                self.inner[i - 1].next = i;
                self.inner[i].prev = i - 1;
            }
            
            if let Some(cell) = self.inner.first_mut() {
                cell.prev = Self::END;
            }

            if let Some(cell) = self.inner.last_mut() {
                cell.next = Self::END;
            }

        }

    }

    /// Go to the next entity to tick.
    fn advance(&mut self) {
        // Do nothing if the current value was removed, because we already advanced the 
        // index before removing it.
        if self.invalidated {
            self.invalidated = false;
        } else {
            if let Some(cell) = self.inner.get(self.index) {
                self.index = cell.next;
            } else {
                // If the index is invalid, it should be the end sentinel.
                debug_assert_eq!(self.index, Self::END);
            }
        }
    }

    /// Get the current value being ticked.
    #[inline]
    #[allow(unused)]
    fn current(&self) -> Option<(usize, &T)> {
        if self.invalidated { return None }
        self.inner.get(self.index).map(|cell| (self.index, &cell.value))
    }

    /// Get the current value being ticked through as mutable reference.
    #[inline]
    fn current_mut(&mut self) -> Option<(usize, &mut T)> {
        if self.invalidated { return None }
        self.inner.get_mut(self.index).map(|cell| (self.index, &mut cell.value))
    }

}

impl<T> TickSlice<T> {

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    #[allow(unused)]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index).map(|cell| &cell.value)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.0.get_mut(index).map(|cell| &mut cell.value)
    }

    #[inline]
    fn iter(&self) -> TickIter<'_, T> {
        TickIter { inner: self.0.iter() }
    }

    #[inline]
    fn iter_mut(&mut self) -> TickIterMut<'_, T> {
        TickIterMut { inner: self.0.iter_mut() }
    }

}

impl<T> Deref for TickVec<T> {

    type Target = TickSlice<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: TickSlice<T> structure has the same layout as [TickCell<T>] because it
        // has transparent representation, so we can just cast the pointers.
        let slice = &self.inner[..];
        unsafe { &*(slice as *const [TickCell<T>] as *const TickSlice<T>) }
    }

}

impl<T> DerefMut for TickVec<T> {

    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Same as above.
        let slice = &mut self.inner[..];
        unsafe { &mut *(slice as *mut [TickCell<T>] as *mut TickSlice<T>) }
    }
    
}

struct TickIter<'a, T> {
    inner: slice::Iter<'a, TickCell<T>>
}

impl<T> FusedIterator for TickIter<'_, T> {}
impl<'a, T> Iterator for TickIter<'a, T> {

    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|cell| &cell.value)
    }

}

struct TickIterMut<'a, T> {
    inner: slice::IterMut<'a, TickCell<T>>
}

impl<T> FusedIterator for TickIterMut<'_, T> {}
impl<'a, T> Iterator for TickIterMut<'a, T> {

    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|cell| &mut cell.value)
    }

}

/// An iterator for blocks in a world area. 
/// This yields the block position, id and metadata.
pub struct BlocksInIter<'a> {
    /// Back-reference to the containing world.
    world: &'a World,
    /// This contains a temporary reference to the chunk being analyzed. This is used to
    /// avoid repeatedly fetching chunks' map.
    chunk: Option<(i32, i32, Option<&'a Chunk>)>,
    /// Minimum coordinate to fetch.
    start: IVec3,
    /// Maximum coordinate to fetch (exclusive).
    end: IVec3,
    /// Next block to fetch.
    cursor: IVec3,
}

impl<'a> BlocksInIter<'a> {

    #[inline]
    fn new(world: &'a World, mut start: IVec3, mut end: IVec3) -> Self {

        debug_assert!(start.x <= end.x && start.y <= end.y && start.z <= end.z);

        start.y = start.y.clamp(0, CHUNK_HEIGHT as i32 - 1);
        end.y = end.y.clamp(0, CHUNK_HEIGHT as i32 - 1);

        // If one the component is in common, because max is exclusive, there will be no
        // blocks at all to read, so we set max to min so it will directly ends.
        if start.x == end.x || start.y == end.y || start.z == end.z {
            end = start;
        }

        Self {
            world,
            chunk: None,
            start,
            end,
            cursor: start,
        }

    }

}

impl FusedIterator for BlocksInIter<'_> {}
impl Iterator for BlocksInIter<'_> {

    type Item = (IVec3, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        
        // X is the last updated component, so when it reaches max it's done.
        if self.cursor.x == self.end.x {
            return None;
        }

        // We are at the start of a new column, update the chunk.
        if self.cursor.y == self.start.y {
            // NOTE: Unchecked because the Y value is clamped in the constructor.
            let (cx, cz) = calc_chunk_pos_unchecked(self.cursor);
            if !matches!(self.chunk, Some((ccx, ccz, _)) if (ccx, ccz) == (cx, cz)) {
                self.chunk = Some((cx, cz, self.world.get_chunk(cx, cz)));
            }
        }

        // If there is no chunk at the position, defaults to (id = 0, metadata = 0).
        let mut ret = (self.cursor, 0, 0);

        // If a chunk exists for the current column.
        if let Some((_, _, Some(chunk))) = self.chunk {
            let (block, metadata) = chunk.get_block(self.cursor);
            ret.1 = block;
            ret.2 = metadata;
        }

        // This component order is important because it matches the internal layout of
        // chunks, and therefore improve cache efficiency.
        self.cursor.y += 1;
        if self.cursor.y == self.end.y {
            self.cursor.y = self.start.y;
            self.cursor.z += 1;
            if self.cursor.z == self.end.z {
                self.cursor.z = self.start.z;
                self.cursor.x += 1;
            }
        }

        Some(ret)

    }

}


/// An iterator for blocks in a world chunk. 
pub struct BlocksInChunkIter<'a> {
    /// Back-reference to the containing world. None if the chunk doesn't exists or the
    /// iterator is exhausted.
    chunk: Option<&'a Chunk>,
    /// Current position that is iterated in the chunk.
    cursor: IVec3,
}

impl<'a> BlocksInChunkIter<'a> {

    #[inline]
    fn new(world: &'a World, cx: i32, cz: i32) -> Self {
        Self {
            chunk: world.get_chunk(cx, cz),
            cursor: IVec3::new(cx * CHUNK_WIDTH as i32, 0, cz * CHUNK_WIDTH as i32),
        }
    }

}

impl FusedIterator for BlocksInChunkIter<'_> {}
impl Iterator for BlocksInChunkIter<'_> {

    type Item = (IVec3, u8, u8);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {

        let (block, metadata) = self.chunk?.get_block(self.cursor);
        let ret = (self.cursor, block, metadata);

        // This component order is important because it matches the internal layout of
        // chunks, and therefore improve cache efficiency. When incrementing component,
        // when we reach the next multiple of 16 (for X/Z), we reset the coordinate.
        self.cursor.y += 1;
        if self.cursor.y >= CHUNK_HEIGHT as i32 {
            self.cursor.y = 0;
            self.cursor.z += 1;
            if self.cursor.z & 0b1111 == 0 {
                self.cursor.z -= 16;
                self.cursor.x += 1;
                if self.cursor.x & 0b1111 == 0 {
                    // X is the last coordinate to be updated, when we reach it then we
                    // set chunk to none because iterator is exhausted.
                    self.chunk = None;
                }
            }
        }

        Some(ret)

    }

}

/// An iterator over all entities in the world.
pub struct EntitiesIter<'a>(TickIter<'a, EntityComponent>);

impl FusedIterator for EntitiesIter<'_> {}
impl ExactSizeIterator for EntitiesIter<'_> {}
impl<'a> Iterator for EntitiesIter<'a> {
    
    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(comp) = self.0.next() {
            if let Some(ret) = comp.inner.as_deref() {
                return Some((comp.id, ret));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

}

/// An iterator over all entities in the world through mutable references.
pub struct EntitiesIterMut<'a>(TickIterMut<'a, EntityComponent>);

impl FusedIterator for EntitiesIterMut<'_> {}
impl ExactSizeIterator for EntitiesIterMut<'_> {}
impl<'a> Iterator for EntitiesIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(comp) = self.0.next() {
            if let Some(ret) = comp.inner.as_deref_mut() {
                return Some((comp.id, ret));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

}

// TODO: we are currently using type alias because the logic is exactly the same and it's
// a pain to implement, maybe just use a wrapper in the future.
/// An iterator of player entities in the world.
pub type PlayerEntitiesIter<'a> = EntitiesInChunkIter<'a>;

/// An iterator of player entities in the world through mutable references.
pub type PlayerEntitiesIterMut<'a> = EntitiesInChunkIterMut<'a>;

/// An iterator of entities within a chunk.
pub struct EntitiesInChunkIter<'a> {
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a TickSlice<EntityComponent>,
}

impl FusedIterator for EntitiesInChunkIter<'_> {}
impl<'a> Iterator for EntitiesInChunkIter<'a> {

    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&index) = self.indices.as_mut()?.next() {
            // We ignore updated entities.
            let comp = self.entities.get(index).unwrap();
            if let Some(entity) = comp.inner.as_deref() {
                return Some((comp.id, entity));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(indices) = &self.indices {
            indices.size_hint()
        } else {
            (0, Some(0))
        }
    }

}

/// An iterator of entities within a chunk through mutable references.
pub struct EntitiesInChunkIterMut<'a> {
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a mut TickSlice<EntityComponent>,
    /// Only used when debug assertions are enabled in order to ensure the safety
    /// of the lifetime transmutation.
    #[cfg(debug_assertions)]
    returned_pointers: HashSet<*mut Entity>,
}

impl FusedIterator for EntitiesInChunkIterMut<'_> {}
impl<'a> Iterator for EntitiesInChunkIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&index) = self.indices.as_mut()?.next() {
            // We ignore updated entities.
            let comp = self.entities.get_mut(index).unwrap();
            if let Some(entity) = comp.inner.as_deref_mut() {

                // Only check uniqueness of returned pointer with debug assertions.
                #[cfg(debug_assertions)] {
                    assert!(self.returned_pointers.insert(entity), "wrong unsafe contract");
                }

                // SAFETY: We know that returned indices are unique because they come from
                // a map iterator that have unique "usize" keys. So each entity will be 
                // accessed and mutated once and in one place only. So we transmute the 
                // lifetime to 'a, instead of using the default `'self`. This is almost 
                // the same as the implementation of mutable slice iterators where we can
                // get mutable references to all slice elements at once.
                let entity = unsafe { &mut *(entity as *mut Entity) };
                return Some((comp.id, entity));

            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(indices) = &self.indices {
            indices.size_hint()
        } else {
            (0, Some(0))
        }
    }

}

/// An iterator of entities that collide with a bounding box.
pub struct EntitiesCollidingIter<'a> {
    /// Chunk components iter whens indices is exhausted.
    chunks: ChunkComponentsIter<'a>,
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a TickSlice<EntityComponent>,
    /// Bounding box to check.
    bb: BoundingBox,
}

impl FusedIterator for EntitiesCollidingIter<'_> {}
impl<'a> Iterator for EntitiesCollidingIter<'a> {

    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // LOOP SAFETY: This loop should not cause infinite iterator because self.indices
        // will eventually be none because it is set to none when it is exhausted. 
        loop {

            if self.indices.is_none() {
                self.indices = Some(self.chunks.next()?.entities.values());
            }

            // If there is no next index, set indices to none and loop over.
            if let Some(&index) = self.indices.as_mut().unwrap().next() {
                let comp = self.entities.get(index).unwrap();
                // We ignore updated/not colliding entities.
                if let Some(entity) = comp.inner.as_deref() {
                    if entity.0.bb.intersects(self.bb) {
                        return Some((comp.id, entity));
                    }
                }
            } else {
                self.indices = None;
            }

        }
    }

}

/// An iterator of entities that collide with a bounding box through mutable references.
pub struct EntitiesCollidingIterMut<'a> {
    /// Chunk components iter whens indices is exhausted.
    chunks: ChunkComponentsIter<'a>,
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a mut TickSlice<EntityComponent>,
    /// Bounding box to check.
    bb: BoundingBox,
    /// Only used when debug assertions are enabled in order to ensure the safety
    /// of the lifetime transmutation.
    #[cfg(debug_assertions)]
    returned_pointers: HashSet<*mut Entity>,
}

impl FusedIterator for EntitiesCollidingIterMut<'_> {}
impl<'a> Iterator for EntitiesCollidingIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // LOOP SAFETY: This loop should not cause infinite iterator because self.indices
        // will eventually be none because it is set to none when it is exhausted.
        loop {

            if self.indices.is_none() {
                self.indices = Some(self.chunks.next()?.entities.values());
            }

            // If there is no next index, set indices to none and loop over.
            if let Some(&index) = self.indices.as_mut().unwrap().next() {
                let comp = self.entities.get_mut(index).unwrap();
                // We ignore updated/not colliding entities.
                if let Some(entity) = comp.inner.as_deref_mut() {
                    if entity.0.bb.intersects(self.bb) {

                        #[cfg(debug_assertions)] {
                            assert!(self.returned_pointers.insert(entity), "wrong unsafe contract");
                        }

                        // SAFETY: Read safety note of 'EntitiesInChunkIterMut', however
                        // we have additional constraint, because we iterate different 
                        // index map iterators so we are no longer guaranteed uniqueness
                        // of returned indices. However, our world implementation ensures
                        // that any entity is only present in a single chunk.
                        let entity = unsafe { &mut *(entity as *mut Entity) };
                        return Some((comp.id, entity));
                        
                    }
                }
            } else {
                self.indices = None;
            }

        }
    }

}

/// Internal iterator chunk components in a range.
struct ChunkComponentsIter<'a> {
    /// Map of chunk components that we 
    chunks: &'a HashMap<(i32, i32), ChunkComponent>,
    /// The range of chunks to iterate on.
    range: ChunkRange,
}

impl FusedIterator for ChunkComponentsIter<'_> {}
impl<'a> Iterator for ChunkComponentsIter<'a> {

    type Item = &'a ChunkComponent;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((cx, cz)) = self.range.next() {
            if let Some(comp) = self.chunks.get(&(cx, cz)) {
                return Some(comp);
            }
        }
        None
    }

}

/// Internal iterator of chunk coordinates, both start and end are inclusive.
struct ChunkRange {
    cx: i32,
    cz: i32,
    start_cx: i32,
    end_cx: i32,
    end_cz: i32,
}

impl ChunkRange {

    // Construct a chunk range iterator, note that both start and end are included in the
    // range.
    #[inline]
    fn new(start_cx: i32, start_cz: i32, end_cx: i32, end_cz: i32) -> Self {
        Self {
            cx: start_cx,
            cz: start_cz,
            start_cx,
            end_cx,
            end_cz,
        }
    }

}

impl FusedIterator for ChunkRange {}
impl Iterator for ChunkRange {

    type Item = (i32, i32);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        
        if self.cx > self.end_cx || self.cz > self.end_cz {
            return None;
        }

        let ret = (self.cx, self.cz);

        self.cx += 1;
        if self.cx > self.end_cx {
            self.cx = self.start_cx;
            self.cz += 1;
        }

        Some(ret)

    }

}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn chunk_range() {

        assert_eq!(ChunkRange::new(0, 0, 0, 0).collect::<Vec<_>>(), [(0, 0)]);
        assert_eq!(ChunkRange::new(0, 0, 1, 0).collect::<Vec<_>>(), [(0, 0), (1, 0)]);
        assert_eq!(ChunkRange::new(0, 0, 1, 1).collect::<Vec<_>>(), [(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert_eq!(ChunkRange::new(0, 0, -1, 0).collect::<Vec<_>>(), []);
        assert_eq!(ChunkRange::new(0, 0, 0, -1).collect::<Vec<_>>(), []);
        assert_eq!(ChunkRange::new(0, 0, -1, -1).collect::<Vec<_>>(), []);

    }

    #[test]
    fn tick_vec() {

        // We want to extensively test this data structure since it is highly critical 
        // and its coherency must be guaranteed at runtime to avoid any issue. A lot of
        // debug assertions are used throughout the code to validate most of the
        // invariants, in addition to the unit tests.

        let mut v = TickVec::<char>::new();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
        assert_eq!(v.current(), None);

        // Construct ['a', 'b', 'c']
        assert_eq!(v.push('a'), 0);
        assert_eq!(v.push('b'), 1);
        assert_eq!(v.push('c'), 2);
        assert_eq!(v.current(), None);
        assert_eq!(v.get(0), Some(&'a'));
        assert_eq!(v.get(1), Some(&'b'));
        assert_eq!(v.get(2), Some(&'c'));
        assert_eq!(v.len(), 3);

        // Construct ['c', 'b']
        assert_eq!(v.remove(0), 'a');
        assert_eq!(v.get(0), Some(&'c'));
        assert_eq!(v.len(), 2);

        // Test cursor
        v.reset();
        assert_eq!(v.current(), Some((0, &'c')));
        v.advance();
        assert_eq!(v.current(), Some((1, &'b')));
        v.advance();
        assert_eq!(v.current(), None);

        // Test cursor remove, and construct ['b', 'a']
        v.reset();
        assert_eq!(v.current(), Some((0, &'c')));
        assert_eq!(v.remove(0), 'c');
        assert_eq!(v.current(), None);
        v.advance();
        assert_eq!(v.current(), Some((0, &'b')));
        assert_eq!(v.push('a'), 1);
        v.advance();
        assert_eq!(v.current(), None);
        assert_eq!(v.get(1), Some(&'a'));

        // Test cursor remove swap
        v.reset();
        assert_eq!(v.remove(1), 'a');
        assert_eq!(v.current(), Some((0, &'b')));
        v.advance();
        assert_eq!(v.current(), None);

    }

}
