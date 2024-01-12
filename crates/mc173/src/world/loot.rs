//! Looting functions to spawn items in a world, also contains the loots for each block.

use std::ops::{Mul, Sub};

use glam::{IVec3, DVec3};

use crate::entity::Item;
use crate::item::ItemStack;
use crate::{block, item};

use super::World;


/// Methods related to loot spawning in the world and block loot randomization.
impl World {

    /// Spawn item entity in the world containing the given stack. The velocity of the 
    /// spawned item stack is random and the initial position depends on the given spread.
    /// This item entity will be impossible to pickup for 10 ticks.
    pub fn spawn_loot(&mut self, mut pos: DVec3, stack: ItemStack, spread: f32) {
        
        if spread != 0.0 {
            pos += self.rand.next_float_vec()
                .mul(spread)
                .as_dvec3()
                .sub(spread as f64 * 0.5);
        }

        let entity = Item::new_with(|base, item| {
            base.persistent = true;
            base.pos = pos;
            base.vel.x = self.rand.next_double() * 0.2 - 0.1;
            base.vel.y = 0.2;
            base.vel.z = self.rand.next_double() * 0.2 - 0.1;
            item.stack = stack;
            item.frozen_time = 10;
        });

        self.spawn_entity(entity);

    }

    /// Spawn item entities in the world depending on the loot of the given block id and
    /// metadata. Each block has a different random try count and loots, the given chance
    /// if looting is checked on each try, typically used for explosions.
    pub fn spawn_block_loot(&mut self, pos: IVec3, id: u8, metadata: u8, chance: f32) {
        let tries = self.get_block_loot_tries(id, metadata);
        for try_num in 0..tries {
            if self.rand.next_float() <= self.get_block_loot_chance(id, metadata, try_num, chance) {
                let stack = self.get_block_loot_stack(id, metadata, try_num);
                if !stack.is_empty() {
                    self.spawn_loot(pos.as_dvec3() + 0.5, stack, 0.7);
                }
            }
        }
    }

    /// Get the tries count from a block and metadata.
    fn get_block_loot_tries(&mut self, id: u8, _metadata: u8) -> u8 {
        match id {
            block::AIR => 0,
            block::BOOKSHELF => 0,
            block::CAKE => 0,
            block::CLAY => 4,
            block::WHEAT => 4,  // 1 for wheat item + 3 for seeds
            block::FIRE => 0,
            block::WATER_MOVING |
            block::WATER_STILL |
            block::LAVA_MOVING |
            block::LAVA_STILL => 0,
            block::GLASS => 0,
            block::GLOWSTONE => 2 + self.rand.next_int_bounded(3) as u8,
            block::ICE => 0,
            block::LEAVES if self.rand.next_int_bounded(20) != 0 => 0,
            block::SPAWNER => 0,
            block::LAPIS_ORE => 4 + self.rand.next_int_bounded(5) as u8,
            block::PISTON_EXT |
            block::PISTON_MOVING => 0,
            block::PORTAL => 0,
            block::REDSTONE_ORE |
            block::REDSTONE_ORE_LIT => 4 + self.rand.next_int_bounded(2) as u8,
            block::SNOW => 0,
            block::SNOW_BLOCK => 4,
            block::DOUBLE_SLAB => 2,
            block::TNT => 0,
            _ => 1
        }
    }

    fn get_block_loot_chance(&mut self, id: u8, metadata: u8, try_num: u8, default_chance: f32) -> f32 {
        match id {
            block::WHEAT if try_num != 0 => metadata as f32 / 14.0,  // Fully grown wheat have 0.5 chance.
            _ => default_chance,
        }
    }

    /// Get the drop item stack from a block and metadata. This is called for each try.
    fn get_block_loot_stack(&mut self, id: u8, metadata: u8, try_num: u8) -> ItemStack {
        match id {
            // Bed only drop if not head piece. 
            block::BED if block::bed::is_head(metadata) => ItemStack::EMPTY,
            block::BED => ItemStack::new_single(item::BED, 0),
            // Cake.
            block::CAKE => ItemStack::EMPTY,
            // Clay.
            block::CLAY => ItemStack::new_single(item::CLAY, 0),
            // Wheat, only drop if reached max stage.
            block::WHEAT if try_num == 0 && metadata != 7 => return ItemStack::EMPTY,
            block::WHEAT if try_num == 0 => ItemStack::new_single(item::WHEAT, 0),
            block::WHEAT => ItemStack::new_single(item::WHEAT_SEEDS, 0),
            // Dead bush.
            block::DEAD_BUSH => ItemStack::EMPTY,
            // Door only drop if lower part.
            block::WOOD_DOOR | 
            block::IRON_DOOR if block::door::is_upper(metadata) => ItemStack::EMPTY,
            block::WOOD_DOOR => ItemStack::new_single(item::WOOD_DOOR, 0),
            block::IRON_DOOR => ItemStack::new_single(item::IRON_DOOR, 0),
            // Farmland and grass.
            block::FARMLAND |
            block::GRASS => ItemStack::new_block(block::DIRT, 0),
            // Fluids.
            block::WATER_MOVING |
            block::WATER_STILL |
            block::LAVA_MOVING |
            block::LAVA_STILL => ItemStack::EMPTY,
            // Furnace.
            block::FURNACE |
            block::FURNACE_LIT => ItemStack::new_block(block::FURNACE, 0),
            // Glowstone.
            block::GLOWSTONE => ItemStack::new_single(item::GLOWSTONE_DUST, 0),
            // Gravel.
            block::GRAVEL if self.rand.next_int_bounded(10) == 0 => ItemStack::new_single(item::FLINT, 0),
            // Leaves.
            block::LEAVES => ItemStack::new_block(block::SAPLING, metadata & 3),
            // Spawner.
            block::SPAWNER => ItemStack::EMPTY,
            // Ores.
            block::COAL_ORE => ItemStack::new_single(item::COAL, 0),
            block::DIAMOND_ORE => ItemStack::new_single(item::DIAMOND, 0),
            block::REDSTONE_ORE |
            block::REDSTONE_ORE_LIT => ItemStack::new_single(item::REDSTONE, 0),
            block::LAPIS_ORE => ItemStack::new_single(item::DYE, 4),
            // Piston.
            block::PISTON_EXT |
            block::PISTON_MOVING => ItemStack::EMPTY,
            // Redstone components.
            block::REDSTONE => ItemStack::new_single(item::REDSTONE, 0),
            block::REPEATER |
            block::REPEATER_LIT => ItemStack::new_single(item::REPEATER, 0),
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT => ItemStack::new_block(block::REDSTONE_TORCH_LIT, 0),
            // Sugar cane.
            block::SUGAR_CANES => ItemStack::new_single(item::SUGAR_CANES, 0),
            // Signs.
            block::SIGN |
            block::WALL_SIGN => ItemStack::new_single(item::SIGN, 0),
            // Snow.
            block::SNOW_BLOCK |
            block::SNOW => ItemStack::new_single(item::SNOWBALL, 0),
            // Double slab.
            block::SLAB |
            block::DOUBLE_SLAB => ItemStack::new_block(block::SLAB, metadata),
            // Stone.
            block::STONE => ItemStack::new_block(block::COBBLESTONE, 0),
            // Tall grass.
            block::TALL_GRASS if self.rand.next_int_bounded(8) == 0 => ItemStack::new_single(item::WHEAT_SEEDS, 0),
            block::TALL_GRASS => ItemStack::EMPTY,
            // Cobweb.
            block::COBWEB => ItemStack::new_single(item::STRING, 0),
            // Log type.
            block::LOG => ItemStack::new_block(block::LOG, metadata),
            // Wool color.
            block::WOOL => ItemStack::new_block(block::WOOL, metadata),
            // Sapling type.
            block::SAPLING => ItemStack::new_block(block::SAPLING, metadata & 3),
            // Default, drop the block's item.
            _ => ItemStack::new_block(id, 0),
        }
    }

}
