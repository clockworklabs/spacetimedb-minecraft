//! Item crafting management.

use crate::item::{self, ItemStack};
use crate::block;


/// This structure keeps track of the current crafting recipe selected and allows lazy
/// update of the crafting recipe. A crafting recipe is based on a 3x3 item grid.
#[derive(Debug, Default)]
pub struct CraftTracker {
    /// The index and result item of the current selected recipe
    current_recipe: Option<(usize, ItemStack)>,
}

impl CraftTracker {

    /// Update this tracker to track a new 3x3 grid of items.
    pub fn update(&mut self, grid: &[ItemStack; 9]) {

        self.current_recipe = None;

        // Do not search if all slots are empty.
        if grid.iter().copied().all(ItemStack::is_empty) {
            return;
        }

        for (recipe_index, recipe) in RECIPES.iter().enumerate() {
            
            let item = match recipe {
                Recipe::Shaped(shaped) => shaped.check(grid),
                Recipe::Shapeless(shapeless) => shapeless.check(grid),
            };

            if let Some(item) = item {
                self.current_recipe = Some((recipe_index, item));
                break;
            }

        }

    }

    /// If there is a selected recipe, consume the recipe items from the given inventory,
    /// this inventory should be coherent with the one that selected this recipe through
    /// the `update` method. You need to call the `update` method again in order to update
    /// the tracker for the new inventory.
    pub fn consume(&self, grid: &mut [ItemStack; 9]) {

        if self.current_recipe.is_none() {
            return;
        }

        // We just decrement all stack's size in the grid, because stack size is ignored
        // in current patterns.
        for stack in grid.iter_mut() {
            if stack.is_empty() || stack.size == 1 {
                *stack = ItemStack::EMPTY;
            } else {
                *stack = stack.with_size(stack.size - 1);
            }
        }

    }

    /// If a crafting recipe is currently selected, return the result item.
    pub fn recipe(&self) -> Option<ItemStack> {
        self.current_recipe.map(|(_, item)| item)
    }

}


// Here are the stack definitions of crafting recipes.

macro_rules! const_stacks {
    ( $( $name:ident = $value:expr; )* ) => {
        $( const $name: ItemStack = ItemStack { id: $value as u16, size: 1, damage: 0 }; )*
    };
}

const EMPTY: ItemStack = ItemStack::EMPTY;
const PAPER_3: ItemStack = ItemStack::new_sized(item::PAPER, 0, 3);
const FENCE_2: ItemStack = ItemStack::new_block_sized(block::FENCE, 0, 2);
const STONE_SLAB_3: ItemStack = ItemStack::new_block_sized(block::SLAB, 0, 3);
const SANDSTONE_SLAB_3: ItemStack = ItemStack::new_block_sized(block::SLAB, 1, 3);
const WOOD_SLAB_3: ItemStack = ItemStack::new_block_sized(block::SLAB, 2, 3);
const COBBLESTONE_SLAB_3: ItemStack = ItemStack::new_block_sized(block::SLAB, 3, 3);
const LADDER_2: ItemStack = ItemStack::new_block_sized(block::LADDER, 0, 2);
const TRAPDOOR_2: ItemStack = ItemStack::new_block_sized(block::TRAPDOOR, 0, 2);
const WOOD_4: ItemStack = ItemStack::new_block_sized(block::WOOD, 0, 4);
const STICK_4: ItemStack = ItemStack::new_sized(item::STICK, 0, 4);
const TORCH_4: ItemStack = ItemStack::new_block_sized(block::TORCH, 0, 4);
const CHARCOAL: ItemStack = ItemStack::new_single(item::COAL, 1);
const BOWL_4: ItemStack = ItemStack::new_sized(item::BOWL, 0, 4);
const RAIL_16: ItemStack = ItemStack::new_block_sized(block::RAIL, 0, 16);
const POWERED_RAIL_6: ItemStack = ItemStack::new_block_sized(block::POWERED_RAIL, 0, 6);
const DETECTOR_RAIL_6: ItemStack = ItemStack::new_block_sized(block::DETECTOR_RAIL, 0, 6);
const WOOD_STAIR_4: ItemStack = ItemStack::new_block_sized(block::WOOD_STAIR, 0, 4);
const COBBLESTONE_STAIR_4: ItemStack = ItemStack::new_block_sized(block::COBBLESTONE_STAIR, 0, 4);
const ARROW_4: ItemStack = ItemStack::new_sized(item::ARROW, 0, 4);
const LAPIS: ItemStack = ItemStack::new_single(item::DYE, 4);
const COOKIE_8: ItemStack = ItemStack::new_sized(item::COOKIE, 0, 8);
const COCOA: ItemStack = ItemStack::new_single(item::DYE, 3);
const YELLOW_DYE_2: ItemStack = ItemStack::new_sized(item::DYE, 11, 2);
const RED_DYE_2: ItemStack = ItemStack::new_sized(item::DYE, 1, 2);
const BONE_MEAL_2: ItemStack = ItemStack::new_sized(item::DYE, 15, 3);

const_stacks! {
    SUGAR_CANES     = item::SUGAR_CANES;
    PAPER           = item::PAPER;
    BOOK            = item::BOOK;
    STICK           = item::STICK;
    DIAMOND         = item::DIAMOND;
    WOOD            = block::WOOD;
    JUKEBOX         = block::JUKEBOX;
    REDSTONE        = item::REDSTONE;
    NOTE_BLOCK      = block::NOTE_BLOCK;
    BOOKSHELF       = block::BOOKSHELF;
    SNOWBALL        = item::SNOWBALL;
    SNOW_BLOCK      = block::SNOW_BLOCK;
    CLAY            = item::CLAY;
    CLAY_BLOCK      = block::CLAY;
    BRICK           = item::BRICK;
    BRICK_BLOCK     = block::BRICK;
    GLOWSTONE_DUST  = item::GLOWSTONE_DUST;
    GLOWSTONE       = block::GLOWSTONE;
    STRING          = item::STRING;
    WOOL            = block::WOOL;
    GUNPOWDER       = item::GUNPOWDER;
    SAND            = block::SAND;
    TNT             = block::TNT;
    BED             = item::BED;
    STONE           = block::STONE;
    SANDSTONE       = block::SANDSTONE;
    COBBLE          = block::COBBLESTONE;
    WOOD_DOOR       = item::WOOD_DOOR;
    IRON_DOOR       = item::IRON_DOOR;
    IRON_INGOT      = item::IRON_INGOT;
    SIGN            = item::SIGN;
    SUGAR           = item::SUGAR;
    MILK_BUCKET     = item::MILK_BUCKET;
    WHEAT           = item::WHEAT;
    EGG             = item::EGG;
    CAKE            = item::CAKE;
    LOG             = block::LOG;
    COAL            = item::COAL;
    GOLD_INGOT      = item::GOLD_INGOT;
    STONE_PRESSURE_PLATE = block::STONE_PRESSURE_PLATE;
    MINECART        = item::MINECART;
    PUMPKIN         = block::PUMPKIN;
    TORCH           = block::TORCH;
    PUMPKIN_LIT     = block::PUMPKIN_LIT;
    CHEST_MINECART  = item::CHEST_MINECART;
    FURNACE_MINECART = item::FURNACE_MINECART;
    CHEST           = block::CHEST;
    FURNACE         = block::FURNACE;
    BOAT            = item::BOAT;
    BUCKET          = item::BUCKET;
    FLINT_AND_STEEL = item::FLINT_AND_STEEL;
    FLINT           = item::FLINT;
    BREAD           = item::BREAD;
    FISHING_ROD     = item::FISHING_ROD;
    PAINTING        = item::PAINTING;
    APPLE           = item::APPLE;
    GOLD_APPLE      = item::GOLD_APPLE;
    LEVER           = block::LEVER;
    REDSTONE_TORCH  = block::REDSTONE_TORCH_LIT;
    REPEATER        = item::REPEATER;
    CLOCK           = item::CLOCK;
    COMPASS         = item::COMPASS;
    MAP             = item::MAP;
    BUTTON          = block::BUTTON;
    WOOD_PRESSURE_PLATE = block::WOOD_PRESSURE_PLATE;
    DISPENSER       = block::DISPENSER;
    BOW             = item::BOW;
    FEATHER         = item::FEATHER;
    PISTON          = block::PISTON;
    STICKY_PISTON   = block::STICKY_PISTON;
    SLIMEBALL       = item::SLIMEBALL;
    SHEARS          = item::SHEARS;
    BOWL            = item::BOWL;
    MUSHROOM_STEW   = item::MUSHROOM_STEW;
    BROWN_MUSHROOM  = block::BROWN_MUSHROOM;
    RED_MUSHROOM    = block::RED_MUSHROOM;
    CRAFTING_TABLE  = block::CRAFTING_TABLE;
    LEATHER         = item::LEATHER;
    DANDELION       = block::DANDELION;
    POPPY           = block::POPPY;
    BONE            = item::BONE;
}

macro_rules! tool {
    ( pickaxe $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, $mat, EMPTY, STICK, EMPTY, EMPTY, STICK, EMPTY], 3)
    };
    ( axe $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, STICK, $mat, STICK, EMPTY], 2)
    };
    ( shovel $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, STICK, STICK], 1)
    };
    ( hoe $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, STICK, EMPTY, STICK, EMPTY], 2)
    };
    ( sword $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, STICK], 1)
    };
}

macro_rules! armor {
    ( helmet $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, $mat, $mat, EMPTY, $mat], 3)
    };
    ( chestplate $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, EMPTY, $mat, $mat, $mat, $mat, $mat, $mat, $mat], 3)
    };
    ( leggings $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, $mat, $mat, $mat, EMPTY, $mat, $mat, EMPTY, $mat], 3)
    };
    ( boots $result:expr, $mat:expr ) => {
        Recipe::new_shaped(ItemStack::new_single($result, 0), &[$mat, EMPTY, $mat, $mat, EMPTY, $mat], 3)
    };
}

macro_rules! ore_block {
    ( construct $block:expr, $ore:expr ) => {
        Recipe::new_shaped(ItemStack::new_block($block, 0), &[$ore, $ore, $ore, $ore, $ore, $ore, $ore, $ore, $ore], 3)
    };
    ( destruct $block:expr, $ore:expr ) => {
        Recipe::new_shaped($ore.with_size(9), &[ItemStack::new_block($block, 0)], 1)
    };
}

macro_rules! dye_mix {
    ( $meta:literal * $count:literal, [ $( $pattern_meta:literal ),+ ] ) => {
        Recipe::new_shapeless(ItemStack::new_sized(item::DYE, $meta, $count), &[ $( ItemStack::new_single(item::DYE, $pattern_meta) ),+ ])
    };
}

const RECIPES: &'static [Recipe] = &[
    Recipe::new_shaped(PAPER_3,             &[SUGAR_CANES, SUGAR_CANES, SUGAR_CANES], 3),
    Recipe::new_shaped(BOOK,                &[PAPER, PAPER, PAPER], 1),
    Recipe::new_shaped(FENCE_2,             &[STICK, STICK, STICK, STICK, STICK, STICK], 3),
    Recipe::new_shaped(JUKEBOX,             &[WOOD, WOOD, WOOD, WOOD, DIAMOND, WOOD, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(NOTE_BLOCK,          &[WOOD, WOOD, WOOD, WOOD, REDSTONE, WOOD, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(BOOKSHELF,           &[WOOD, WOOD, WOOD, BOOK, BOOK, BOOK, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(SNOW_BLOCK,          &[SNOWBALL, SNOWBALL, SNOWBALL, SNOWBALL], 2),
    Recipe::new_shaped(CLAY_BLOCK,          &[CLAY, CLAY, CLAY, CLAY], 2),
    Recipe::new_shaped(BRICK_BLOCK,         &[BRICK, BRICK, BRICK, BRICK], 2),
    Recipe::new_shaped(GLOWSTONE,           &[GLOWSTONE_DUST, GLOWSTONE_DUST, GLOWSTONE_DUST, GLOWSTONE_DUST], 2),
    Recipe::new_shaped(WOOL,                &[STRING, STRING, STRING, STRING], 2),
    Recipe::new_shaped(TNT,                 &[GUNPOWDER, SAND, GUNPOWDER, SAND, GUNPOWDER, SAND, GUNPOWDER, SAND, GUNPOWDER], 3),
    Recipe::new_shaped(STONE_SLAB_3,        &[STONE, STONE, STONE], 3),
    Recipe::new_shaped(SANDSTONE_SLAB_3,    &[SANDSTONE, SANDSTONE, SANDSTONE], 3),
    Recipe::new_shaped(WOOD_SLAB_3,         &[WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(COBBLESTONE_SLAB_3,  &[COBBLE, COBBLE, COBBLE], 3),
    Recipe::new_shaped(LADDER_2,            &[STICK, EMPTY, STICK, STICK, STICK, STICK, STICK, EMPTY, STICK], 3),
    Recipe::new_shaped(WOOD_DOOR,           &[WOOD, WOOD, WOOD, WOOD, WOOD, WOOD], 2),
    Recipe::new_shaped(TRAPDOOR_2,          &[WOOD, WOOD, WOOD, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(IRON_DOOR,           &[IRON_INGOT, IRON_INGOT, IRON_INGOT, IRON_INGOT, IRON_INGOT, IRON_INGOT], 2),
    Recipe::new_shaped(SIGN,                &[WOOD, WOOD, WOOD, WOOD, WOOD, WOOD, EMPTY, STICK, EMPTY], 3),
    Recipe::new_shaped(CAKE,                &[MILK_BUCKET, MILK_BUCKET, MILK_BUCKET, SUGAR, EGG, SUGAR, WHEAT, WHEAT, WHEAT], 3),
    Recipe::new_shaped(SUGAR,               &[SUGAR_CANES], 1),
    Recipe::new_shaped(WOOD_4,              &[LOG], 1),
    Recipe::new_shaped(STICK_4,             &[WOOD, WOOD], 1),
    Recipe::new_shaped(TORCH_4,             &[COAL, STICK], 1),
    Recipe::new_shaped(TORCH_4,             &[CHARCOAL, STICK], 1),
    Recipe::new_shaped(BOWL_4,              &[WOOD, EMPTY, WOOD, EMPTY, WOOD, EMPTY], 3),
    Recipe::new_shaped(RAIL_16,             &[IRON_INGOT, EMPTY, IRON_INGOT, IRON_INGOT, STICK, IRON_INGOT, IRON_INGOT, EMPTY, IRON_INGOT], 3),
    Recipe::new_shaped(POWERED_RAIL_6,      &[GOLD_INGOT, EMPTY, GOLD_INGOT, GOLD_INGOT, STICK, GOLD_INGOT, GOLD_INGOT, REDSTONE, GOLD_INGOT], 3),
    Recipe::new_shaped(DETECTOR_RAIL_6,     &[IRON_INGOT, EMPTY, IRON_INGOT, IRON_INGOT, STONE_PRESSURE_PLATE, IRON_INGOT, IRON_INGOT, REDSTONE, IRON_INGOT], 3),
    Recipe::new_shaped(MINECART,            &[IRON_INGOT, EMPTY, IRON_INGOT, IRON_INGOT, IRON_INGOT, IRON_INGOT], 3),
    Recipe::new_shaped(PUMPKIN_LIT,         &[PUMPKIN, TORCH], 1),
    Recipe::new_shaped(CHEST_MINECART,      &[CHEST, MINECART], 1),
    Recipe::new_shaped(FURNACE_MINECART,    &[FURNACE, MINECART], 1),
    Recipe::new_shaped(BOAT,                &[WOOD, EMPTY, WOOD, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(BUCKET,              &[IRON_INGOT, EMPTY, IRON_INGOT, EMPTY, IRON_INGOT, EMPTY], 3),
    Recipe::new_shaped(FLINT_AND_STEEL,     &[IRON_INGOT, EMPTY, EMPTY, FLINT], 2),
    Recipe::new_shaped(BREAD,               &[WHEAT, WHEAT, WHEAT], 3),
    Recipe::new_shaped(WOOD_STAIR_4,        &[WOOD, EMPTY, EMPTY, WOOD, WOOD, EMPTY, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(FISHING_ROD,         &[EMPTY, EMPTY, STICK, EMPTY, STICK, STRING, STICK, EMPTY, STRING], 3),
    Recipe::new_shaped(COBBLESTONE_STAIR_4, &[COBBLE, EMPTY, EMPTY, COBBLE, COBBLE, EMPTY, COBBLE, COBBLE, COBBLE], 3),
    Recipe::new_shaped(PAINTING,            &[STICK, STICK, STICK, STICK, WOOL, STICK, STICK, STICK, STICK], 3),
    Recipe::new_shaped(GOLD_APPLE,          &[GOLD_INGOT, GOLD_INGOT, GOLD_INGOT, GOLD_INGOT, APPLE, GOLD_INGOT, GOLD_INGOT, GOLD_INGOT], 3),
    Recipe::new_shaped(LEVER,               &[STICK, COBBLE], 1),
    Recipe::new_shaped(REDSTONE_TORCH,      &[REDSTONE, STICK], 1),
    Recipe::new_shaped(REPEATER,            &[REDSTONE_TORCH, REDSTONE, REDSTONE_TORCH, STONE, STONE, STONE], 3),
    Recipe::new_shaped(CLOCK,               &[EMPTY, GOLD_INGOT, EMPTY, GOLD_INGOT, REDSTONE, GOLD_INGOT, EMPTY, GOLD_INGOT, EMPTY], 3),
    Recipe::new_shaped(COMPASS,             &[EMPTY, IRON_INGOT, EMPTY, IRON_INGOT, REDSTONE, IRON_INGOT, EMPTY, IRON_INGOT, EMPTY], 3),
    Recipe::new_shaped(MAP,                 &[PAPER, PAPER, PAPER, PAPER, COMPASS, PAPER, PAPER, PAPER, PAPER], 3),
    Recipe::new_shaped(BUTTON,              &[STONE, STONE], 1),
    Recipe::new_shaped(STONE_PRESSURE_PLATE, &[STONE, STONE], 2),
    Recipe::new_shaped(WOOD_PRESSURE_PLATE, &[WOOD, WOOD], 2),
    Recipe::new_shaped(DISPENSER,           &[COBBLE, COBBLE, COBBLE, COBBLE, BOW, COBBLE, COBBLE, REDSTONE, COBBLE], 3),
    Recipe::new_shaped(PISTON,              &[WOOD, WOOD, WOOD, COBBLE, IRON_INGOT, COBBLE, COBBLE, REDSTONE, COBBLE], 3),
    Recipe::new_shaped(STICKY_PISTON,       &[SLIMEBALL, PISTON], 1),
    Recipe::new_shaped(BED,                 &[WOOL, WOOL, WOOL, WOOD, WOOD], 3),
    Recipe::new_shaped(SHEARS,              &[IRON_INGOT, EMPTY, EMPTY, IRON_INGOT], 2),
    Recipe::new_shaped(BOW,                 &[EMPTY, STICK, STRING, STICK, EMPTY, STRING, EMPTY, STICK, STRING], 3),
    Recipe::new_shaped(ARROW_4,             &[FLINT, STICK, FEATHER], 1),
    Recipe::new_shaped(MUSHROOM_STEW,       &[RED_MUSHROOM, BROWN_MUSHROOM, BOWL], 1),
    Recipe::new_shaped(MUSHROOM_STEW,       &[BROWN_MUSHROOM, RED_MUSHROOM, BOWL], 1),
    Recipe::new_shaped(COOKIE_8,            &[WHEAT, COCOA, WHEAT], 3),
    Recipe::new_shaped(CHEST,               &[WOOD, WOOD, WOOD, WOOD, EMPTY, WOOD, WOOD, WOOD, WOOD], 3),
    Recipe::new_shaped(FURNACE,             &[COBBLE, COBBLE, COBBLE, COBBLE, EMPTY, COBBLE, COBBLE, COBBLE, COBBLE], 3),
    Recipe::new_shaped(CRAFTING_TABLE,      &[WOOD, WOOD, WOOD, WOOD], 2),
    Recipe::new_shaped(SANDSTONE,           &[SAND, SAND, SAND, SAND], 2),
    // Tools...
    tool!(pickaxe item::WOOD_PICKAXE, WOOD),
    tool!(pickaxe item::STONE_PICKAXE, COBBLE),
    tool!(pickaxe item::GOLD_PICKAXE, GOLD_INGOT),
    tool!(pickaxe item::IRON_PICKAXE, IRON_INGOT),
    tool!(pickaxe item::DIAMOND_PICKAXE, DIAMOND),
    tool!(axe item::WOOD_AXE, WOOD),
    tool!(axe item::STONE_AXE, COBBLE),
    tool!(axe item::GOLD_AXE, GOLD_INGOT),
    tool!(axe item::IRON_AXE, IRON_INGOT),
    tool!(axe item::DIAMOND_AXE, DIAMOND),
    tool!(shovel item::WOOD_SHOVEL, WOOD),
    tool!(shovel item::STONE_SHOVEL, COBBLE),
    tool!(shovel item::GOLD_SHOVEL, GOLD_INGOT),
    tool!(shovel item::IRON_SHOVEL, IRON_INGOT),
    tool!(shovel item::DIAMOND_SHOVEL, DIAMOND),
    tool!(hoe item::WOOD_HOE, WOOD),
    tool!(hoe item::STONE_HOE, COBBLE),
    tool!(hoe item::GOLD_HOE, GOLD_INGOT),
    tool!(hoe item::IRON_HOE, IRON_INGOT),
    tool!(hoe item::DIAMOND_HOE, DIAMOND),
    tool!(sword item::WOOD_SWORD, WOOD),
    tool!(sword item::STONE_SWORD, COBBLE),
    tool!(sword item::GOLD_SWORD, GOLD_INGOT),
    tool!(sword item::IRON_SWORD, IRON_INGOT),
    tool!(sword item::DIAMOND_SWORD, DIAMOND),
    // Armors...
    armor!(helmet item::LEATHER_HELMET, LEATHER),
    armor!(helmet item::GOLD_HELMET, GOLD_INGOT),
    armor!(helmet item::IRON_HELMET, IRON_INGOT),
    armor!(helmet item::DIAMOND_HELMET, DIAMOND),
    armor!(chestplate item::LEATHER_CHESTPLATE, LEATHER),
    armor!(chestplate item::GOLD_CHESTPLATE, GOLD_INGOT),
    armor!(chestplate item::IRON_CHESTPLATE, IRON_INGOT),
    armor!(chestplate item::DIAMOND_CHESTPLATE, DIAMOND),
    armor!(leggings item::LEATHER_LEGGINGS, LEATHER),
    armor!(leggings item::GOLD_LEGGINGS, GOLD_INGOT),
    armor!(leggings item::IRON_LEGGINGS, IRON_INGOT),
    armor!(leggings item::DIAMOND_LEGGINGS, DIAMOND),
    armor!(boots item::LEATHER_BOOTS, LEATHER),
    armor!(boots item::GOLD_BOOTS, GOLD_INGOT),
    armor!(boots item::IRON_BOOTS, IRON_INGOT),
    armor!(boots item::DIAMOND_BOOTS, DIAMOND),
    // Ore blocks...
    ore_block!(construct block::IRON_BLOCK, IRON_INGOT),
    ore_block!(construct block::GOLD_BLOCK, GOLD_INGOT),
    ore_block!(construct block::DIAMOND_BLOCK, DIAMOND),
    ore_block!(construct block::LAPIS_BLOCK, LAPIS),
    ore_block!(destruct block::IRON_BLOCK, IRON_INGOT),
    ore_block!(destruct block::GOLD_BLOCK, GOLD_INGOT),
    ore_block!(destruct block::DIAMOND_BLOCK, DIAMOND),
    ore_block!(destruct block::LAPIS_BLOCK, LAPIS),
    // Dyes...
    Recipe::new_shapeless(YELLOW_DYE_2, &[DANDELION]),
    Recipe::new_shapeless(RED_DYE_2,    &[POPPY]),
    Recipe::new_shapeless(BONE_MEAL_2,  &[BONE]),
    dye_mix!(9 * 2, [1, 15]),
    dye_mix!(14 * 2, [1, 11]),
    dye_mix!(10 * 2, [2, 15]),
    dye_mix!(8 * 2, [0, 15]),
    dye_mix!(7 * 2, [8, 15]),
    dye_mix!(7 * 3, [0, 15, 15]),
    dye_mix!(12 * 2, [4, 15]),
    dye_mix!(6 * 2, [4, 2]),
    dye_mix!(5 * 2, [4, 1]),
    dye_mix!(13 * 2, [5, 9]),
    dye_mix!(13 * 3, [4, 1, 9]),
    dye_mix!(13 * 4, [4, 1, 1, 15]),
];


/// The recipe enumeration stores different types of recipes.
/// 
/// **Note that crafting recipes currently ignore the stack size in of patterns.**
enum Recipe {
    /// A shaped crafting recipe requires the items to be in a specific pattern, the 
    /// pattern has a size and if smaller than 3x3 it can be moved everywhere in the 
    /// table.
    Shaped(ShapedRecipe),
    /// A shapeless crafting just define a list of items that must be present in the
    /// crafting grid, each stack must be present once.
    Shapeless(ShapelessRecipe),
}

struct ShapedRecipe {
    result: ItemStack,
    pattern: &'static [ItemStack],
    width: u8,
}

struct ShapelessRecipe {
    result: ItemStack,
    pattern: &'static [ItemStack],
}

impl Recipe {
    
    const fn new_shaped(result: ItemStack, pattern: &'static [ItemStack], width: u8) -> Self {
        Self::Shaped(ShapedRecipe { result, pattern, width })
    }

    const fn new_shapeless(result: ItemStack, pattern: &'static [ItemStack]) -> Self {
        Self::Shapeless(ShapelessRecipe { result, pattern })
    }

}

impl ShapedRecipe {

    /// Check if this shaped recipe can be crafted with the given inventory of the given 
    /// size and items.
    fn check(&self, grid: &[ItemStack; 9]) -> Option<ItemStack> {

        // Compute recipe size based on pattern length and width.
        // NOTE: We compute the height in which the pattern fit.
        let recipe_width = self.width as usize;
        let recipe_height = (self.pattern.len() + recipe_width - 1) / recipe_width;

        // Recipe size cannot fit in the given inventory shape: discard immediately.
        // NOTE: This also avoids arithmetics underflow just below.
        if recipe_width > 3 || recipe_height > 3 {
            return None;
        }

        // For each possible starting point in the inventory, check.
        for start_x in 0..=(3 - recipe_width) {
            'out: for start_y in 0..=(3 - recipe_height) {

                let mut normal_valid = true;
                let mut flip_valid = true;

                for dx in 0..3 {
                    for dy in 0..3 {
                        let stack = grid[dx + dy * 3];
                        if dx < start_x || dx >= start_x + recipe_width || dy < start_y || dy >= start_y + recipe_height {
                            // We are outside the checked region, slot should be empty.
                            if !stack.is_empty() {
                                continue 'out;
                            }
                        } else {

                            // We are in the checked region.
                            let pattern_x = dx - start_x;
                            let pattern_y = dy - start_y;
                            let flip_pattern_x = recipe_width - pattern_x - 1;

                            if normal_valid {
                                let normal_stack = self.pattern[pattern_x + pattern_y * recipe_width];
                                normal_valid = 
                                    (normal_stack.is_empty() && stack.is_empty()) ||
                                    (!stack.is_empty() && (normal_stack.id, normal_stack.damage) == (stack.id, stack.damage));
                            }

                            if flip_valid {
                                let flip_stack = self.pattern[flip_pattern_x + pattern_y * recipe_width];
                                flip_valid = 
                                    (flip_stack.is_empty() && stack.is_empty()) ||
                                    (!stack.is_empty() && (flip_stack.id, flip_stack.damage) == (stack.id, stack.damage));
                            }

                            if !normal_valid && !flip_valid {
                                continue 'out;
                            }
                            
                        }
                    }
                }

                if normal_valid || flip_valid {
                    return Some(self.result);
                }

            }
        }

        None

    }

}

impl ShapelessRecipe {

    /// Check if this shapeless recipe can be crafted with the given inventory items.
    fn check(&self, inv: &[ItemStack; 9]) -> Option<ItemStack> {
        
        // Too few stacks for the current pattern: discard immediately.
        if inv.len() < self.pattern.len() {
            return None;
        }

        let mut pat_matched = 0u32;

        'inv: for stack in inv.iter().copied() {
            if !stack.is_empty() {
                for (i, pat_stack) in self.pattern.iter().copied().enumerate() {
                    if pat_matched & (1 << i) == 0 {
                        if (pat_stack.id, pat_stack.damage) == (stack.id, stack.damage) {
                            pat_matched |= 1 << i;
                            continue 'inv;
                        }
                    }
                }
                // If we land here, we did not found the required item in the pattern.
                return None;
            }
        }

        // Not all the pattern has been matched
        if pat_matched != (1 << self.pattern.len()) - 1 {
            None
        } else {
            Some(self.result)
        }

    }

}
