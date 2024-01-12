//! This module provides various functions for getting the material properties of blocks.

use crate::block;


/// Get material of a block.
pub fn get_material(block: u8) -> Material {
    match block {
        block::STONE |
        block::COBBLESTONE |
        block::BEDROCK |
        block::GOLD_ORE | 
        block::IRON_ORE |
        block::COAL_ORE |
        block::LAPIS_ORE | 
        block::LAPIS_BLOCK |
        block::DISPENSER |
        block::SANDSTONE |
        block::DOUBLE_SLAB |
        block::SLAB |
        block::BRICK |
        block::MOSSY_COBBLESTONE |
        block::OBSIDIAN |
        block::SPAWNER |
        block::DIAMOND_ORE |
        block::FURNACE |
        block::FURNACE_LIT |
        block::COBBLESTONE_STAIR |
        block::STONE_PRESSURE_PLATE |
        block::REDSTONE_ORE |
        block::REDSTONE_ORE_LIT |
        block::NETHERRACK |
        block::GLOWSTONE => Material::Rock,
        block::GRASS => Material::Grass,
        block::DIRT |
        block::FARMLAND => Material::Ground,
        block::WOOD |
        block::LOG |
        block::NOTE_BLOCK |
        block::BOOKSHELF |
        block::WOOD_STAIR |
        block::CHEST |
        block::CRAFTING_TABLE |
        block::SIGN |
        block::WOOD_DOOR |
        block::WALL_SIGN |
        block::WOOD_PRESSURE_PLATE |
        block::JUKEBOX |
        block::FENCE |
        block::LOCKED_CHEST |
        block::TRAPDOOR => Material::Wood,
        block::SAPLING |
        block::TALL_GRASS |
        block::DEAD_BUSH |
        block::DANDELION |
        block::POPPY |
        block::BROWN_MUSHROOM |
        block::RED_MUSHROOM |
        block::WHEAT |
        block::SUGAR_CANES => Material::Plant,
        block::WATER_MOVING |
        block::WATER_STILL => Material::Water,
        block::LAVA_MOVING |
        block::LAVA_STILL => Material::Lava,
        block::SAND |
        block::GRAVEL |
        block::SOULSAND => Material::Sand,
        block::LEAVES => Material::Leaves,
        block::SPONGE => Material::Sponge,
        block::GLASS => Material::Glass,
        block::BED |
        block::WOOL => Material::Cloth,
        block::POWERED_RAIL |
        block::DETECTOR_RAIL |
        block::TORCH |
        block::REDSTONE |
        block::LADDER |
        block::RAIL |
        block::LEVER |
        block::REDSTONE_TORCH |
        block::REDSTONE_TORCH_LIT |
        block::BUTTON |
        block::REPEATER |
        block::REPEATER_LIT => Material::Circuit,
        block::STICKY_PISTON |
        block::PISTON |
        block::PISTON_EXT |
        block::PISTON_MOVING => Material::Piston,
        block::COBWEB => Material::Cobweb,
        block::GOLD_BLOCK |
        block::IRON_BLOCK |
        block::DIAMOND_BLOCK |
        block::IRON_DOOR => Material::Iron,
        block::TNT => Material::Tnt,
        block::FIRE => Material::Fire,
        block::SNOW => Material::Snow,
        block::ICE => Material::Ice,
        block::SNOW_BLOCK => Material::SnowBlock,
        block::CACTUS => Material::Cactus,
        block::CLAY => Material::Clay,
        block::PUMPKIN |
        block::PUMPKIN_LIT => Material::Pumpkin,
        block::PORTAL => Material::Portal,
        block::CAKE => Material::Cake,
        _ => Material::Air
    }
}

/// Return true if a block is a full cube.
pub fn is_cube(block: u8) -> bool {
    match block {
        block::AIR |
        block::BED |
        block::PORTAL |
        block::BUTTON |
        block::CACTUS |
        block::CAKE |
        block::WOOD_DOOR |
        block::IRON_DOOR |
        block::FARMLAND |
        block::FENCE |
        block::FIRE |
        block::WHEAT |
        block::DEAD_BUSH |
        block::RED_MUSHROOM |
        block::BROWN_MUSHROOM |
        block::DANDELION |
        block::POPPY |
        block::SAPLING |
        block::TALL_GRASS |
        block::WATER_MOVING |
        block::WATER_STILL |
        block::LAVA_MOVING |
        block::LAVA_STILL |
        block::LADDER |
        block::LEVER |
        block::PISTON |
        block::PISTON_EXT |
        block::PISTON_MOVING |
        block::WOOD_PRESSURE_PLATE |
        block::STONE_PRESSURE_PLATE |
        block::RAIL |
        block::POWERED_RAIL |
        block::DETECTOR_RAIL |
        block::REPEATER |
        block::REPEATER_LIT |
        block::REDSTONE |
        block::SUGAR_CANES |
        block::SIGN |
        block::WALL_SIGN |
        block::SNOW |
        block::WOOD_STAIR |
        block::COBBLESTONE_STAIR |
        block::SLAB |
        block::TORCH |
        block::REDSTONE_TORCH |
        block::REDSTONE_TORCH_LIT |
        block::TRAPDOOR |
        block::COBWEB => false,
        _ => true,
    }
}

/// Return true if a block is a full opaque cube.
pub fn is_opaque_cube(block: u8) -> bool {
    if is_cube(block) {
        match block {
            block::LEAVES |
            block::GLASS |
            block::ICE => false,
            _ => true,
        }
    } else {
        false
    }
}

/// Return true if the given block is a fluid.
pub fn is_fluid(block: u8) -> bool {
    matches!(block, 
        block::WATER_MOVING | block::WATER_STILL | 
        block::LAVA_MOVING | block::LAVA_STILL)
}

/// Return true if the given block can block fluid.
pub fn is_fluid_proof(block: u8) -> bool {
    match block {
        block::AIR => false,
        block::WOOD_DOOR |
        block::IRON_DOOR |
        block::SIGN |
        block::LADDER |
        block::SUGAR_CANES => true,
        _ => get_material(block).is_solid()
    }
}

/// Get the light opacity of a block given its id.
pub fn get_light_opacity(id: u8) -> u8 {
    match id {
        block::AIR => 0,
        block::LEAVES |
        block::COBWEB => 1,
        block::WATER_MOVING |
        block::WATER_STILL |
        block::ICE => 3,
        _ =>  if is_opaque_cube(id) { 255 } else { 0 },
    }
}

/// Get the light emission of a block given its id. The resulting value is between
/// 0 and 15 included, so it's basically a 4-bit unsigned integer.
pub fn get_light_emission(id: u8) -> u8 {
    match id {
        block::BROWN_MUSHROOM => 1,
        block::REDSTONE_TORCH_LIT => 7,
        block::REDSTONE_ORE_LIT |
        block::REPEATER_LIT => 9,
        block::PORTAL => 11,
        block::FURNACE_LIT => 13,
        block::TORCH => 14,
        block::LAVA_MOVING |
        block::LAVA_STILL |
        block::FIRE |
        block::GLOWSTONE |
        block::PUMPKIN_LIT => 15,
        _ => 0
    }
}

/// The block slipperiness for entities.
pub fn get_slipperiness(id: u8) -> f32 {
    match id {
        block::ICE => 0.95,
        _ => 0.6
    }
}

/// Get the break hardness of a block, the block hardness is a value that defines the 
/// time a player need to hit a block before breaking. When the player's tool is able
/// to break the block, the hardness is multiplied by 30 ticks (1.5 seconds), but 100
/// (5.0 seconds) when not able. Some blocks cannot be broken: +inf is returned.
pub fn get_break_hardness(id: u8) -> f32 {
    match id {
        block::LEAVES |
        block::BED |
        block::SNOW_BLOCK => 0.2,
        block::GLASS |
        block::GLOWSTONE => 0.3,
        block::LADDER |
        block::CACTUS |
        block::NETHERRACK => 0.4,
        block::DIRT |
        block::SAND |
        block::STICKY_PISTON |
        block::PISTON |
        block::PISTON_EXT |
        block::LEVER |
        block::STONE_PRESSURE_PLATE |
        block::WOOD_PRESSURE_PLATE |
        block::BUTTON |
        block::ICE |
        block::SOULSAND |
        block::CAKE => 0.5,
        block::GRASS |
        block::GRAVEL |
        block::SPONGE |
        block::FARMLAND |
        block::CLAY => 0.6,
        block::POWERED_RAIL |
        block::DETECTOR_RAIL |
        block::RAIL => 0.7,
        block::SANDSTONE |
        block::NOTE_BLOCK |
        block::WOOL => 0.8,
        block::STONE |
        block::BOOKSHELF => 1.5,
        block::COBBLESTONE |
        block::WOOD |
        block::LOG |
        block::DOUBLE_SLAB |
        block::SLAB |
        block::BRICK |
        block::MOSSY_COBBLESTONE |
        block::WOOD_STAIR |
        block::COBBLESTONE_STAIR |
        block::JUKEBOX |
        block::FENCE => 2.0,
        block::CRAFTING_TABLE |
        block::CHEST => 2.5,
        block::GOLD_ORE |
        block::IRON_ORE |
        block::COAL_ORE |
        block::LAPIS_ORE |
        block::LAPIS_BLOCK |
        block::GOLD_BLOCK |
        block::DIAMOND_ORE |
        block::WOOD_DOOR |
        block::REDSTONE_ORE |
        block::REDSTONE_ORE_LIT |
        block::TRAPDOOR => 3.0,
        block::DISPENSER |
        block::FURNACE |
        block::FURNACE_LIT => 3.5,
        block::COBWEB => 4.0,
        block::IRON_BLOCK |
        block::DIAMOND_BLOCK |
        block::IRON_DOOR |
        block::SPAWNER => 5.0,
        block::OBSIDIAN => 10.0,
        block::BEDROCK |
        block::PISTON_MOVING |
        block::PORTAL |
        block::WATER_MOVING |
        block::WATER_STILL |
        block::LAVA_MOVING |
        block::LAVA_STILL => f32::INFINITY,
        _ => 0.0,
    }
}

/// The block resistance to explosions. When an explosion happens, each ray of the 
/// explosion starts with an intensity that equals the radius of the explosion multiplied
/// by a uniform amount between 0.7 and 1.4... The resistance is the amount subtracted
/// on each step of the ray.
pub fn get_explosion_resistance(id: u8) -> f32 {
    match id {
        block::AIR => 0.0,
        block::WOOD |
        block::GOLD_ORE |
        block::IRON_ORE |
        block::COAL_ORE |
        block::LAPIS_ORE |
        block::LAPIS_BLOCK |
        block::WOOD_STAIR |
        block::DIAMOND_ORE |
        block::IRON_DOOR |
        block::REDSTONE_ORE |
        block::REDSTONE_ORE_LIT => 15.0 / 5.0,
        block::STONE |
        block::COBBLESTONE |
        block::GOLD_BLOCK |
        block::IRON_BLOCK |
        block::DOUBLE_SLAB |
        block::SLAB |
        block::BRICK |
        block::MOSSY_COBBLESTONE |
        block::DIAMOND_BLOCK |
        block::COBBLESTONE_STAIR |
        block::JUKEBOX => 30.0 / 5.0,
        block::OBSIDIAN => 6000.0 / 5.0,
        block::BEDROCK => 18000000.0 / 5.0,
        _ => get_break_hardness(id),
    }
}

pub fn get_fire_flammability(id: u8) -> u16 {
    match id {
        block::WOOD |
        block::FENCE |
        block::WOOD_STAIR |
        block::LOG => 5,
        block::TNT => 15,
        block::LEAVES |
        block::BOOKSHELF |
        block::WOOL => 30,
        block::TALL_GRASS => 60,
        _ => 0
    }
}

/// Get the burn rate for the given block. The returned rate is used to randomly destroy
/// a block that was on fire and replace it with fire. Blocks that are horizontal to the
/// fire have n/300 chance of being destroyed and vertical blocks have n/250 every 40 
/// game ticks.
pub fn get_fire_burn(id: u8) -> u16 {
    match id {
        block::WOOD |
        block::FENCE |
        block::WOOD_STAIR |
        block::BOOKSHELF => 20,
        block::LEAVES |
        block::WOOL => 60,
        block::TNT |
        block::TALL_GRASS => 100,
        _ => 0,
    }
}

/// Common block properties of blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Material {
    #[default]
    Air,
    Grass,
    Ground,
    Wood,
    Rock,
    Iron,
    Water,
    Lava,
    Leaves,
    Plant,
    Sponge,
    Cloth,
    Fire,
    Sand,
    Circuit,
    Glass,
    Tnt,
    Wug,
    Ice,
    Snow,
    SnowBlock,
    Cactus,
    Clay,
    Pumpkin,
    Portal,
    Cake,
    Cobweb,
    Piston,
}

impl Material {

    pub fn is_solid(self) -> bool {
        !matches!(self, 
            Self::Air |
            Self::Water |
            Self::Lava |
            Self::Plant |
            Self::Snow |
            Self::Circuit |
            Self::Portal |
            Self::Fire)
    }

    pub fn is_fluid(self) -> bool {
        matches!(self, Self::Water | Self::Lava)
    }

    pub fn is_translucent(self) -> bool {
        matches!(self, 
            Self::Leaves | 
            Self::Glass | 
            Self::Tnt | 
            Self::Ice | 
            Self::Snow | 
            Self::Cactus)
    }

    pub fn is_opaque(self) -> bool {
        !self.is_translucent() && self.is_solid()
    }

    pub fn is_replaceable(self) -> bool {
        matches!(self, 
            Self::Air |
            Self::Water |
            Self::Lava |
            Self::Snow |
            Self::Fire)
    }

    pub fn is_breakable_by_default(self) -> bool {
        !matches!(self,
            Self::Rock |
            Self::Iron |
            Self::Snow |
            Self::SnowBlock |
            Self::Cobweb
        )
    }

}
