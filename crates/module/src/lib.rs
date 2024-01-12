use std::sync::Arc;
use mc173::gen::{ChunkGenerator, OverworldGenerator};
use spacetimedb::spacetimedb;
use mc173::chunk::Chunk;

struct ChunkStorage {
    chunk: Chunk
}

#[spacetimedb(table)]
pub struct Person {
    name: String
}

/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

#[spacetimedb(reducer)]
pub fn generate_chunk() {
    let generator = OverworldGenerator::new(SEED);
    let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
    let mut chunk = Chunk::new();
    let chunk_access = Arc::get_mut(&mut chunk).unwrap();
    log::info!("Starting Generation");

    generator.gen_terrain(0, 0, chunk_access, &mut state);
    log::info!("Generation Complete.")
}
