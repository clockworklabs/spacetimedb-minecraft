//! Item smelting management.

use crate::block::material::Material;
use crate::item::ItemStack;
use crate::{block, item};


/// Find a smelting recipe output from given input item/damage.
pub fn find_smelting_output(id: u16, damage: u16) -> Option<ItemStack> {
    for recipe in RECIPES {
        if (recipe.input.id, recipe.input.damage) == (id, damage) {
            return Some(recipe.output);
        }
    }
    None
}

/// Get burn time of the given item id, returning 0 if the given item id is not a fuel.
pub fn get_burn_ticks(id: u16) -> u16 {
    if let Ok(id) = u8::try_from(id) {
        match id {
            block::SAPLING => 100,
            _ if block::material::get_material(id) == Material::Wood => 300,
            _ => 0,
        }
    } else {
        match id {
            item::COAL => 1600,
            item::LAVA_BUCKET => 20000,
            item::STICK => 100,
            _ => 0,
        }
    }
}

const RECIPES: &'static [Recipe] = &[
    Recipe::new(ItemStack::new_block(block::IRON_ORE, 0), ItemStack::new_single(item::IRON_INGOT, 0)),
    Recipe::new(ItemStack::new_block(block::GOLD_ORE, 0), ItemStack::new_single(item::GOLD_INGOT, 0)),
    Recipe::new(ItemStack::new_block(block::DIAMOND_ORE, 0), ItemStack::new_single(item::DIAMOND, 0)),
    Recipe::new(ItemStack::new_block(block::SAND, 0), ItemStack::new_block(block::GLASS, 0)),
    Recipe::new(ItemStack::new_single(item::RAW_PORKCHOP, 0), ItemStack::new_single(item::COOKED_PORKCHOP, 0)),
    Recipe::new(ItemStack::new_single(item::RAW_FISH, 0), ItemStack::new_single(item::COOKED_FISH, 0)),
    Recipe::new(ItemStack::new_block(block::COBBLESTONE, 0), ItemStack::new_block(block::STONE, 0)),
    Recipe::new(ItemStack::new_single(item::CLAY, 0), ItemStack::new_single(item::BRICK, 0)),
    Recipe::new(ItemStack::new_block(block::CACTUS, 0), ItemStack::new_single(item::DYE, 2)),
    Recipe::new(ItemStack::new_block(block::LOG, 0), ItemStack::new_single(item::COAL, 1)),
];

/// Define a smelting recipe.
struct Recipe {
    /// The item stack that is consumed to produce the output one.
    input: ItemStack,
    /// The output stack that is produced by consuming the input one.
    output: ItemStack,
}

impl Recipe {

    const fn new(input: ItemStack, output: ItemStack) -> Self {
        Self { input, output }
    }

}
