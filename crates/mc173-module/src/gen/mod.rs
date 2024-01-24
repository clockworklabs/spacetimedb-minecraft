//! World generation module.
//! 
//! PARITY: The parity of world generation is really hard to get fully exact, mostly
//! because Minecraft itself is not at parity with itself! The world generation scheduling
//! has a huge impact on chunk populating, so this implementation is on parity but it may
//! not give exact same world on each generation, just like Minecraft. Terrain however,
//! should be exactly the same on same run.

use glam::IVec3;

use crate::rand::JavaRandom;
use crate::chunk::Chunk;
use crate::world::World;

// World gen-specific mathematic functions.
pub mod math;
pub mod noise;

// Feature generators.
pub mod dungeon;
pub mod plant;
pub mod vein;
pub mod liquid;
pub mod tree;

// Chunks carvers.
pub mod cave;

// World generators.
mod overworld;
pub use overworld::OverworldGenerator;


/// A trait for all chunk generators, a chunk generator is immutable, if any mutable 
/// state needs to be stored, the `State` associated type can be used.
pub trait ChunkGenerator {

    /// Type of the cache that is only owned by a single worker.
    type State: Default;

    /// Generate only the chunk biomes, this may never be called and this is not called
    /// before [`gen_terrain`](Self::gen_terrain).
    fn gen_biomes(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut Self::State);

    /// Generate the given chunk's terrain, this should also generate the biomes 
    /// associated to the terrain generation. The separate method 
    /// [`gen_biomes`](Self::gen_biomes) is not called before that function.
    fn gen_terrain(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut Self::State);

    /// Populate a chunk that is present in a world, note that this world is internal
    /// to the generator, this chunk will then be transferred to the real world when
    /// done. Populate usually applies with an offset of 8 blocks into the chunk with
    /// a 16x16 populate area, this means that neighbor chunks affected are also
    /// guaranteed to be loaded.
    fn gen_features(&self, cx: i32, cz: i32, world: &mut World, state: &mut Self::State);

}


/// A trait common to all feature generators.
pub trait FeatureGenerator {

    /// Generate the feature at the given position in the world with given RNG.
    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool;

}
