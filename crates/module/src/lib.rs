use std::sync::Arc;
use mc173::gen::{ChunkGenerator, OverworldGenerator};
use spacetimedb::spacetimedb;
use mc173::chunk::Chunk;

#[spacetimedb(table)]
struct ChunkStorage {
    #[unique]
    x: i32,
    #[unique]
    z: i32,
    chunk: Chunk
}

/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

#[spacetimedb(reducer)]
pub fn generate_chunk(x: i32, z: i32) {
    let generator = OverworldGenerator::new(SEED);
    let mut state = <OverworldGenerator as ChunkGenerator>::State::default();
    let mut chunk = Chunk::new();
    log::info!("Starting Generation");

    generator.gen_terrain(0, 0, &mut chunk, &mut state);
    if let Err(_) = ChunkStorage::insert(ChunkStorage {
        x, z, chunk,
    }) {
        log::error!("Failed to insert unique chunk");
    };
    log::info!("Generation Complete.")
}
