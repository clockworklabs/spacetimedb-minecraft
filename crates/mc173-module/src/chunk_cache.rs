use std::collections::BTreeMap;
use crate::stdb::chunk::StdbChunk;
use crate::util::default;

pub struct ChunkCache(BTreeMap<u64, Option<StdbChunk>>);

impl ChunkCache {
    pub fn empty() -> Self {
        Self(<_>::default())
    }

    pub fn fetch() -> Self {
        Self(StdbChunk::iter().map(|s| (s.chunk_id, Some(s))).collect())
    }

    pub fn iter_cached_values(&self) -> impl Iterator<Item = &StdbChunk> {
        self.0.values().flatten()
    }

    pub fn flush(self) {
        for entry in self.0 {
            match entry {
                (id, Some(chunk)) => {
                    StdbChunk::update_by_chunk_id(&id, chunk);
                }
                _ => {}
            }
        }
    }

    pub fn filter_by_chunk_id(&mut self, chunk_id: u64) -> Option<&mut StdbChunk> {
        self.0
            .entry(chunk_id)
            .or_insert_with(|| StdbChunk::filter_by_chunk_id(&chunk_id))
            .as_mut()
    }

    pub fn filter_by_coords(&mut self, x: i32, z: i32) -> Option<&mut StdbChunk> {
        self.filter_by_chunk_id(StdbChunk::chunk_coords_to_id(x, z))
    }
}
