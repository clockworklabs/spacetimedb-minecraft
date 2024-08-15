use std::collections::HashMap;
use crate::stdb::chunk::StdbChunk;

pub struct ChunkCache {
    pub chunks: HashMap<u32, Option<StdbChunk>>,
}

impl ChunkCache {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn get_chunk(&mut self, x: i32, z: i32) -> Option<StdbChunk> {
        let chunk_id = StdbChunk::xz_to_chunk_id(x, z);
        let chunk = self.chunks.get(&chunk_id);
        match chunk {
            Some(chunk) => chunk.clone(),
            None => {
                let loaded = StdbChunk::filter_by_chunk_id(&chunk_id);
                self.chunks.insert(chunk_id, loaded.clone());
                loaded
            },
        }
    }

    pub fn set_chunk(&mut self, mut chunk: StdbChunk) {
        let chunk_id = StdbChunk::xz_to_chunk_id(chunk.x, chunk.z);
        chunk.chunk_id = chunk_id;
        self.chunks.insert(chunk_id, Some(chunk));
    }

    pub fn apply(&self) {
        for (chunk_id, chunk) in self.chunks.iter() {
            match chunk {
                Some(chunk) => {
                    match StdbChunk::filter_by_chunk_id(&chunk_id) {
                        None => {
                            StdbChunk::insert(chunk.clone()).unwrap();
                        }
                        Some(_) => {
                            StdbChunk::update_by_chunk_id(&chunk_id, chunk.clone());
                        }
                    }
                }
                None => (),
            }
        }
    }
}