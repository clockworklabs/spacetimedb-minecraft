//! This modules provide the biome enumeration, it is stored in each chunk on the 2D grid.
//! The Notchian implementation doesn't store the biomes, so they are generated on each
//! chunk load, biomes are also not sent to the client, so it is also recomputed 
//! client-side in order to have the proper foliage color.

use crate::entity::{EntityCategory, EntityKind};


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

impl Biome {

    /// Return true if it is possible to rain in a chunk.
    #[inline]
    pub fn has_rain(self) -> bool {
        match self {
            Biome::Desert |
            Biome::IceDesert |
            Biome::Nether |
            Biome::Sky => false,
            _ => true
        }
    }

    /// Return true if this is snowing in the biome.
    #[inline]
    pub fn has_snow(self) -> bool {
        match self {
            Biome::Taiga |
            Biome::IceDesert |
            Biome::Tundra => true,
            _ => false
        }
    }

    /// Get the natural entity kinds for the given category and this current biome.
    pub fn natural_entity_kinds(self, category: EntityCategory) -> &'static [NaturalEntityKind] {
        
        const ANIMALS: &'static [NaturalEntityKind] = &[
            NaturalEntityKind::new(EntityKind::Sheep, 12),
            NaturalEntityKind::new(EntityKind::Pig, 10),
            NaturalEntityKind::new(EntityKind::Chicken, 10),
            NaturalEntityKind::new(EntityKind::Cow, 8),
            // Only in Forest/Taiga:
            NaturalEntityKind::new(EntityKind::Wolf, 2),
        ];

        const WATER_ANIMALS: &'static [NaturalEntityKind] = &[
            NaturalEntityKind::new(EntityKind::Squid, 10),
        ];

        const MOBS: &'static [NaturalEntityKind] = &[
            NaturalEntityKind::new(EntityKind::Spider, 10),
            NaturalEntityKind::new(EntityKind::Zombie, 10),
            NaturalEntityKind::new(EntityKind::Skeleton, 10),
            NaturalEntityKind::new(EntityKind::Creeper, 10),
            NaturalEntityKind::new(EntityKind::Slime, 10),
        ];

        const NETHER_MOBS: &'static [NaturalEntityKind] = &[
            NaturalEntityKind::new(EntityKind::Ghast, 10),
            NaturalEntityKind::new(EntityKind::PigZombie, 10),
        ];

        const SKY_ANIMALS: &'static [NaturalEntityKind] = &[
            NaturalEntityKind::new(EntityKind::Chicken, 10),
        ];
        
        match self {
            Biome::Void => &[],
            Biome::RainForest |
            Biome::Swampland |
            Biome::SeasonalForest |
            Biome::Savanna |
            Biome::ShrubLand |
            Biome::Desert |
            Biome::Plains |
            Biome::IceDesert |
            Biome::Tundra => {
                match category {
                    EntityCategory::Animal => &ANIMALS[..ANIMALS.len() - 1], // Skip wolf
                    EntityCategory::WaterAnimal => WATER_ANIMALS,
                    EntityCategory::Mob => MOBS,
                    EntityCategory::Other => &[]
                }
            }
            Biome::Forest |
            Biome::Taiga => {
                match category {
                    EntityCategory::Animal => ANIMALS, // Don't skip wolf
                    EntityCategory::WaterAnimal => WATER_ANIMALS,
                    EntityCategory::Mob => MOBS,
                    EntityCategory::Other => &[]
                }
            }
            Biome::Nether => {
                match category {
                    EntityCategory::Mob => NETHER_MOBS,
                    _ => &[]
                }
            }
            Biome::Sky => {
                match category {
                    EntityCategory::Animal => SKY_ANIMALS,
                    _ => &[]
                }
            }
            
        }
    }

}


/// Describe a natural 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NaturalEntityKind {
    /// The entity kind.
    pub kind: EntityKind,
    /// The higher the rate is, the higher probability is to spawn.
    pub chance: u16,
}

impl NaturalEntityKind {

    #[inline]
    pub const fn new(kind: EntityKind, chance: u16) -> Self {
        Self { kind, chance }
    }

}
