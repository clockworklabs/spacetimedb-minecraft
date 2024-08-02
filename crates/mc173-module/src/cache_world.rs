use glam::IVec3;
use crate::block;
use crate::chunk::calc_chunk_pos;
use crate::chunk_cache::ChunkCache;
use crate::geom::Face;
use crate::ivec3::StdbIVec3;
use crate::stdb::chunk::StdbChunk;
use crate::world::{ChunkEvent, LightKind, StdbChunkEvent, StdbWorld, World};

pub struct CacheWorld<'a> {
    world: &'a StdbWorld,
    pub cache: ChunkCache,
}

impl CacheWorld {
    pub fn new(world: &StdbWorld) -> Self {
        CacheWorld {
            world,
            cache: ChunkCache::empty(),
        }
    }

    pub fn flush(&mut self) {
        self.cache.flush();
    }

    /// Get a reference to a chunk, if existing.
    pub fn get_chunk(&mut self, cx: i32, cz: i32) -> Option<&mut StdbChunk> {
        self.cache.filter_by_coords(cx, cz)
        // self.chunks.get(&(cx, cz)).and_then(|c| c.data.as_deref())
    }

    /// Tick pending light updates for a maximum number of light updates. This function
    /// returns true only if all light updates have been processed.
    /// TODO: Don't just copy code, we should make a tick_light_inner or something
    pub fn tick_light(&mut self, limit: usize) {
        self.world.world.tick_light()

    }
}

