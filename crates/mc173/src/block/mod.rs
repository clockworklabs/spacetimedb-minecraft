//! Block enumeration and functions to query their metadata state.

use crate::item::Item;

// Block behaviors.
pub mod material;

// Block specific functions for their metadata.
pub mod dispenser;
pub mod trapdoor;
pub mod repeater;
pub mod pumpkin;
pub mod sapling;
pub mod button;
pub mod ladder;
pub mod piston;
pub mod lever;
pub mod stair;
pub mod torch;
pub mod fluid;
pub mod door;
pub mod bed;


/// Internal macro to easily define blocks registry.
macro_rules! blocks {
    (
        $($ident:ident / $id:literal : $name:literal),* $(,)?
    ) => {

        static NAMES: [&'static str; 256] = {
            let mut arr = [""; 256];
            $(arr[$id as usize] = $name;)*
            arr
        };

        const ITEMS: [Item; 256] = {
            let mut arr = [Item {
                name: "",
                block: true,
                max_stack_size: 64,
                max_damage: 0,
            }; 256];
            $(arr[$id as usize].name = $name;)*
            arr
        };

        $(pub const $ident: u8 = $id;)*

    };
}

blocks! {
    AIR/0:              "air",
    STONE/1:            "stone",
    GRASS/2:            "grass",
    DIRT/3:             "dirt",
    COBBLESTONE/4:      "cobblestone",
    WOOD/5:             "wood",
    SAPLING/6:          "sapling",
    BEDROCK/7:          "bedrock",
    WATER_MOVING/8:     "water_moving",
    WATER_STILL/9:      "water_still",
    LAVA_MOVING/10:     "lava_moving",
    LAVA_STILL/11:      "lava_still",
    SAND/12:            "sand",
    GRAVEL/13:          "gravel",
    GOLD_ORE/14:        "gold_ore",
    IRON_ORE/15:        "iron_ore",
    COAL_ORE/16:        "coal_ore",
    LOG/17:             "log",
    LEAVES/18:          "leaves",
    SPONGE/19:          "sponge",
    GLASS/20:           "glass",
    LAPIS_ORE/21:       "lapis_ore",
    LAPIS_BLOCK/22:     "lapis_block",
    DISPENSER/23:       "dispenser",
    SANDSTONE/24:       "sandstone",
    NOTE_BLOCK/25:      "note_block",
    BED/26:             "bed",
    POWERED_RAIL/27:    "powered_rail",
    DETECTOR_RAIL/28:   "detector_rail",
    STICKY_PISTON/29:   "sticky_piston",
    COBWEB/30:          "cobweb",
    TALL_GRASS/31:      "tall_grass",
    DEAD_BUSH/32:       "dead_bush",
    PISTON/33:          "piston",
    PISTON_EXT/34:      "piston_ext",
    WOOL/35:            "wool",
    PISTON_MOVING/36:   "piston_moving",
    DANDELION/37:       "dandelion",
    POPPY/38:           "poppy",
    BROWN_MUSHROOM/39:  "brown_mushroom",
    RED_MUSHROOM/40:    "red_mushroom",
    GOLD_BLOCK/41:      "gold_block",
    IRON_BLOCK/42:      "iron_block",
    DOUBLE_SLAB/43:     "double_slab",
    SLAB/44:            "slab",
    BRICK/45:           "brick",
    TNT/46:             "tnt",
    BOOKSHELF/47:       "bookshelf",
    MOSSY_COBBLESTONE/48: "mossy_cobblestone",
    OBSIDIAN/49:        "obsidian",
    TORCH/50:           "torch",
    FIRE/51:            "fire",
    SPAWNER/52:         "spawner",
    WOOD_STAIR/53:      "wood_stair",
    CHEST/54:           "chest",
    REDSTONE/55:        "redstone",
    DIAMOND_ORE/56:     "diamond_ore",
    DIAMOND_BLOCK/57:   "diamond_block",
    CRAFTING_TABLE/58:  "crafting_table",
    WHEAT/59:           "wheat",
    FARMLAND/60:        "farmland",
    FURNACE/61:         "furnace",
    FURNACE_LIT/62:     "furnace_lit",
    SIGN/63:            "sign",
    WOOD_DOOR/64:       "wood_door",
    LADDER/65:          "ladder",
    RAIL/66:            "rail",
    COBBLESTONE_STAIR/67: "cobblestone_stair",
    WALL_SIGN/68:       "wall_sign",
    LEVER/69:           "lever",
    STONE_PRESSURE_PLATE/70: "stone_pressure_plate",
    IRON_DOOR/71:       "iron_door",
    WOOD_PRESSURE_PLATE/72: "wood_pressure_plate",
    REDSTONE_ORE/73:    "redstone_ore",
    REDSTONE_ORE_LIT/74: "redstone_ore_lit",
    REDSTONE_TORCH/75:  "redstone_torch",
    REDSTONE_TORCH_LIT/76:  "redstone_torch_lit",
    BUTTON/77:          "button",
    SNOW/78:            "snow",
    ICE/79:             "ice",
    SNOW_BLOCK/80:      "snow_block",
    CACTUS/81:          "cactus",
    CLAY/82:            "clay",
    SUGAR_CANES/83:     "sugar_canes",
    JUKEBOX/84:         "jukebox",
    FENCE/85:           "fence",
    PUMPKIN/86:         "pumpkin",
    NETHERRACK/87:      "netherrack",
    SOULSAND/88:        "soulsand",
    GLOWSTONE/89:       "glowstone",
    PORTAL/90:          "portal",
    PUMPKIN_LIT/91:     "pumpkin_lit",
    CAKE/92:            "cake",
    REPEATER/93:        "repeater",
    REPEATER_LIT/94:    "repeater_lit",
    LOCKED_CHEST/95:    "locked_chest",
    TRAPDOOR/96:        "trapdoor",
}

/// Find a block name from its id.
#[inline]
pub fn name(id: u8) -> &'static str {
    NAMES[id as usize]
}

/// Find a block id from its name.
pub fn from_name(name: &str) -> Option<u8> {
    NAMES.iter().position(|&n| n == name).map(|n| n as u8)
}

/// Find the item associated to the given block id. 
/// TODO: This should be removed in the future when items module is reworked.
#[inline]
pub fn item(id: u8) -> &'static Item {
    &ITEMS[id as usize]
}
