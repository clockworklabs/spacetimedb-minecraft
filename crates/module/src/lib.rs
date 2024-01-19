use mc173::gen::{ChunkGenerator, OverworldGenerator};
use spacetimedb::{spacetimedb};
use mc173::chunk::Chunk;

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

#[spacetimedb(reducer)]
pub fn generate_chunk() {
    let generator = OverworldGenerator::new(SEED);
    let mut state = <OverworldGenerator as ChunkGenerator>::State::default();

    log::info!("Starting Generation");

    for x in -3..4 {
        for z in -3..4 {
            let mut chunk = Chunk::new_no_arc();
            generator.gen_terrain(x, z, &mut chunk, &mut state);
            if let Err(_) = StdbChunk::insert(StdbChunk {
                chunk_id: 0, x, z, chunk,
            }) {
                log::error!("Failed to insert unique chunk");
            };
            log::info!("Chunk Generated: {}, {}", x, z);
        }
    }

    log::info!("Generation complete");
}
