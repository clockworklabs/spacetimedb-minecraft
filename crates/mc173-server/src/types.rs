use glam::{DVec3, IVec3, Vec2};
use crate::geom::Face;
use crate::item::ItemStack;

/// The overworld dimension with a blue sky and day cycles.
pub const DIMENSION_OVERWORLD: i32 = 0;
/// The creepy nether dimension.
pub const DIMENSION_NETHER: i32 = -1;

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
    // /// A block entity has been set at this position.
    // BlockEntity {
    //     /// The block entity position.
    //     pos: IVec3,
    //     /// Inner block entity event.
    //     inner: BlockEntityEvent,
    // },
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

/// Type of weather at a specific position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalWeather {
    /// The weather is clear at the position.
    Clear,
    /// It is raining at the position.
    Rain,
    /// It is snowing at the position.
    Snow,
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
    /// A piston has been extended or retracted at the given position.
    Piston {
        /// Face of this piston.
        face: Face,
        /// True if the piston is extending.
        extending: bool,
    },
    /// A note block is playing its note.
    NoteBlock {
        /// The instrument to play.
        instrument: u8,
        /// The note to play.
        note: u8,
    },
}

/// An offline player defines the saved data of a player that is not connected.
#[derive(Debug)]
pub struct OfflinePlayer {
    /// World name.
    pub world: String,
    /// Last saved position of the player.
    pub pos: DVec3,
    /// Last saved look of the player.
    pub look: Vec2,
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

/// Possible biomes, only used server-side for natural mob spawning.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    #[default]
    Void,
    RainForest,
    Swampland,
    SeasonalForest,
    Forest,
    Savanna,
    ShrubLand,
    Taiga,
    Desert,
    Plains,
    IceDesert,
    Tundra,
    Nether,
    Sky,
}