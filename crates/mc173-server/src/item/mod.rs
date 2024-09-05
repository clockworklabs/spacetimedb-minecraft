//! Item enumeration and behaviors.

use crate::block;

pub mod attack;


/// Internal macro to easily define blocks registry.
macro_rules! items {
    (
        $($name:ident / $id:literal : $init:expr),* $(,)?
    ) => {

        static ITEMS: [Item; 2002] = {
            let mut arr = [Item::new(""); 2002];
            $(arr[$id as usize] = $init;)*
            arr
        };

        $(pub const $name: u16 = $id + 256;)*

    };
}

const WOOD_MAX_USES: u16 = 59;
const STONE_MAX_USES: u16 = 131;
const IRON_MAX_USES: u16 = 250;
const GOLD_MAX_USES: u16 = 32;
const DIAMOND_MAX_USES: u16 = 1561;

items! {
    IRON_SHOVEL/0:          Item::new("iron_shovel").set_tool(IRON_MAX_USES),
    IRON_PICKAXE/1:         Item::new("iron_pickaxe").set_tool(IRON_MAX_USES),
    IRON_AXE/2:             Item::new("iron_axe").set_tool(IRON_MAX_USES),
    FLINT_AND_STEEL/3:      Item::new("flint_and_steel").set_tool(64),
    APPLE/4:                Item::new("apple"),
    BOW/5:                  Item::new("bow").set_max_stack_size(1),
    ARROW/6:                Item::new("arrow"),
    COAL/7:                 Item::new("coal"), // .set_max_damage(1),
    DIAMOND/8:              Item::new("diamond"),
    IRON_INGOT/9:           Item::new("iron_ingot"),
    GOLD_INGOT/10:          Item::new("gold_ingot"),
    IRON_SWORD/11:          Item::new("iron_sword").set_tool(IRON_MAX_USES),
    WOOD_SWORD/12:          Item::new("wood_sword").set_tool(WOOD_MAX_USES),
    WOOD_SHOVEL/13:         Item::new("wood_shovel").set_tool(WOOD_MAX_USES),
    WOOD_PICKAXE/14:        Item::new("wood_pickaxe").set_tool(WOOD_MAX_USES),
    WOOD_AXE/15:            Item::new("wood_axe").set_tool(WOOD_MAX_USES),
    STONE_SWORD/16:         Item::new("stone_sword").set_tool(STONE_MAX_USES),
    STONE_SHOVEL/17:        Item::new("stone_shovel").set_tool(STONE_MAX_USES),
    STONE_PICKAXE/18:       Item::new("stone_pickaxe").set_tool(STONE_MAX_USES),
    STONE_AXE/19:           Item::new("stone_axe").set_tool(STONE_MAX_USES),
    DIAMOND_SWORD/20:       Item::new("diamond_sword").set_tool(DIAMOND_MAX_USES),
    DIAMOND_SHOVEL/21:      Item::new("diamond_shovel").set_tool(DIAMOND_MAX_USES),
    DIAMOND_PICKAXE/22:     Item::new("diamond_pickaxe").set_tool(DIAMOND_MAX_USES),
    DIAMOND_AXE/23:         Item::new("diamond_axe").set_tool(DIAMOND_MAX_USES),
    STICK/24:               Item::new("stick"),
    BOWL/25:                Item::new("bowl"),
    MUSHROOM_STEW/26:       Item::new("mushroom_stew").set_food(),
    GOLD_SWORD/27:          Item::new("gold_sword").set_tool(GOLD_MAX_USES),
    GOLD_SHOVEL/28:         Item::new("gold_shovel").set_tool(GOLD_MAX_USES),
    GOLD_PICKAXE/29:        Item::new("gold_pickaxe").set_tool(GOLD_MAX_USES),
    GOLD_AXE/30:            Item::new("gold_axe").set_tool(GOLD_MAX_USES),
    STRING/31:              Item::new("string"),
    FEATHER/32:             Item::new("feather"),
    GUNPOWDER/33:           Item::new("gunpowder"),
    WOOD_HOE/34:            Item::new("wood_hoe").set_tool(WOOD_MAX_USES),
    STONE_HOE/35:           Item::new("stone_hoe").set_tool(STONE_MAX_USES),
    IRON_HOE/36:            Item::new("iron_hoe").set_tool(IRON_MAX_USES),
    DIAMOND_HOE/37:         Item::new("diamond_hoe").set_tool(DIAMOND_MAX_USES),
    GOLD_HOE/38:            Item::new("gold_hoe").set_tool(GOLD_MAX_USES),
    WHEAT_SEEDS/39:         Item::new("wheat_seeds"),
    WHEAT/40:               Item::new("wheat"),
    BREAD/41:               Item::new("bread").set_food(),
    LEATHER_HELMET/42:      Item::new("leather_helmet").set_tool(11 * 3),
    LEATHER_CHESTPLATE/43:  Item::new("leather_chestplate").set_tool(16 * 3),
    LEATHER_LEGGINGS/44:    Item::new("leather_leggings").set_tool(15 * 3),
    LEATHER_BOOTS/45:       Item::new("leather_boots").set_tool(13 * 3),
    CHAIN_HELMET/46:        Item::new("chain_helmet").set_tool(11 * 6),
    CHAIN_CHESTPLATE/47:    Item::new("chain_chestplate").set_tool(16 * 6),
    CHAIN_LEGGINGS/48:      Item::new("chain_leggings").set_tool(15 * 6),
    CHAIN_BOOTS/49:         Item::new("chain_boots").set_tool(13 * 6),
    IRON_HELMET/50:         Item::new("iron_helmet").set_tool(11 * 12),
    IRON_CHESTPLATE/51:     Item::new("iron_chestplate").set_tool(16 * 12),
    IRON_LEGGINGS/52:       Item::new("iron_leggings").set_tool(15 * 12),
    IRON_BOOTS/53:          Item::new("iron_boots").set_tool(13 * 12),
    DIAMOND_HELMET/54:      Item::new("diamond_helmet").set_tool(11 * 24),
    DIAMOND_CHESTPLATE/55:  Item::new("diamond_chestplate").set_tool(16 * 24),
    DIAMOND_LEGGINGS/56:    Item::new("diamond_leggings").set_tool(15 * 24),
    DIAMOND_BOOTS/57:       Item::new("diamond_boots").set_tool(13 * 24),
    GOLD_HELMET/58:         Item::new("gold_helmet").set_tool(11 * 6),
    GOLD_CHESTPLATE/59:     Item::new("gold_chestplate").set_tool(16 * 6),
    GOLD_LEGGINGS/60:       Item::new("gold_leggings").set_tool(15 * 6),
    GOLD_BOOTS/61:          Item::new("gold_boots").set_tool(13 * 6),
    FLINT/62:               Item::new("flint"),
    RAW_PORKCHOP/63:        Item::new("raw_porkchop").set_food(),
    COOKED_PORKCHOP/64:     Item::new("cooked_porkchop").set_food(),
    PAINTING/65:            Item::new("painting"),
    GOLD_APPLE/66:          Item::new("gold_apple").set_food(),
    SIGN/67:                Item::new("sign").set_max_stack_size(1),
    WOOD_DOOR/68:           Item::new("wood_door").set_max_stack_size(1),
    BUCKET/69:              Item::new("bucket").set_max_stack_size(1),
    WATER_BUCKET/70:        Item::new("water_bucket").set_max_stack_size(1),
    LAVA_BUCKET/71:         Item::new("lava_bucket").set_max_stack_size(1),
    MINECART/72:            Item::new("minecart").set_max_stack_size(1),
    SADDLE/73:              Item::new("saddle").set_max_stack_size(1),
    IRON_DOOR/74:           Item::new("iron_door").set_max_stack_size(1),
    REDSTONE/75:            Item::new("redstone"),
    SNOWBALL/76:            Item::new("snowball").set_max_stack_size(16),
    BOAT/77:                Item::new("boat").set_max_stack_size(1),
    LEATHER/78:             Item::new("leather"),
    MILK_BUCKET/79:         Item::new("milk_bucket").set_food(),
    BRICK/80:               Item::new("brick"),
    CLAY/81:                Item::new("clay"),
    SUGAR_CANES/82:         Item::new("sugar_canes"),
    PAPER/83:               Item::new("paper"),
    BOOK/84:                Item::new("book"),
    SLIMEBALL/85:           Item::new("slimeball"),
    CHEST_MINECART/86:      Item::new("chest_minecart").set_max_stack_size(1),
    FURNACE_MINECART/87:    Item::new("furnace_minecart").set_max_stack_size(1),
    EGG/88:                 Item::new("egg").set_max_stack_size(16),
    COMPASS/89:             Item::new("compass").set_max_stack_size(1),
    FISHING_ROD/90:         Item::new("fishing_rod").set_tool(64),
    CLOCK/91:               Item::new("clock").set_max_stack_size(1),
    GLOWSTONE_DUST/92:      Item::new("glowstone_dust"),
    RAW_FISH/93:            Item::new("raw_fish").set_food(),
    COOKED_FISH/94:         Item::new("cooked_fish").set_food(),
    DYE/95:                 Item::new("dye"), //.set_max_damage(15),
    BONE/96:                Item::new("bone"),
    SUGAR/97:               Item::new("sugar"),
    CAKE/98:                Item::new("cake").set_max_stack_size(1),
    BED/99:                 Item::new("bed").set_max_stack_size(1),
    REPEATER/100:           Item::new("repeater"),
    COOKIE/101:             Item::new("cookie").set_max_stack_size(8),
    MAP/102:                Item::new("map").set_max_stack_size(1),
    SHEARS/103:             Item::new("shears").set_tool(238),
    RECORD_13/2000:         Item::new("record_13").set_max_stack_size(1),
    RECORD_CAT/2001:        Item::new("record_cat").set_max_stack_size(1),
}


/// Get an item from its numeric id.
pub fn from_id(id: u16) -> &'static Item {
    if id < 256 {
        block::item(id as u8)
    } else {
        &ITEMS[(id - 256) as usize]
    }
}

/// Find an item id from its name. **Note that this will not find block items.
pub fn from_name(name: &str) -> Option<u16> {
    ITEMS.iter().enumerate()
        .find(|(_, item)| item.name == name)
        .map(|(i, _)| (i + 256) as u16)
}


/// This structure describe a block.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    /// The name of the item, used for debug purpose.
    pub name: &'static str,
    /// Set to true if this item is derived from a block.
    pub block: bool,
    /// Maximum stack size for this item.
    pub max_stack_size: u16,
    /// Maximum possible damage for this item.
    pub max_damage: u16,
}

impl Item {

    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            block: false,
            max_stack_size: 64,
            max_damage: 0,
        }
    }

    const fn set_tool(self, max_damage: u16) -> Self {
        self.set_max_stack_size(1).set_max_damage(max_damage)
    }

    const fn set_food(self) -> Self {
        self.set_max_stack_size(1)
    }

    const fn set_max_stack_size(mut self, max_stack_size: u16) -> Self {
        self.max_stack_size = max_stack_size;
        self
    }

    const fn set_max_damage(mut self, max_damage: u16) -> Self {
        self.max_damage = max_damage;
        self
    }

}


/// An item stack defines the actual number of items and their damage value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemStack {
    /// The item id.
    pub id: u16,
    /// The stack size.
    pub size: u16,
    /// The damage value of the stack.
    pub damage: u16,
}

impl ItemStack {

    pub const EMPTY: Self = Self { id: block::AIR as u16, size: 0, damage: 0 };

    /// Shortcut constructor for an item stack with single item.
    pub const fn new_single(id: u16, damage: u16) -> Self {
        Self { id, size: 1, damage }
    }
    
    pub const fn new_sized(id: u16, damage: u16, size: u16) -> Self {
        Self { id, size, damage }
    }

    /// Shortcut constructor for an item stack constructed from a block id and metadata.
    pub const fn new_block(id: u8, metadata: u8) -> Self {
        Self { id: id as u16, size: 1, damage: metadata as u16}
    }

    pub const fn new_block_sized(id: u8, metadata: u8, size: u16) -> Self {
        Self { id: id as u16, size, damage: metadata as u16 }
    }

    pub const fn with_size(mut self, size: u16) -> ItemStack {
        self.size = size;
        self
    }

    pub const fn with_damage(mut self, damage: u16) -> ItemStack {
        self.damage = damage;
        self
    }

    /// Return true if this item stack is air, which is a special case where the item 
    /// stack represent an empty slot.
    pub fn is_empty(self) -> bool {
        self.id == block::AIR as u16 || self.size == 0
    }

    /// Simplify this item stack by converting it into `None` if the item is just a air
    /// block, which is equivalent to no item for Minecraft, regardless of the damage 
    /// value or stack size.
    pub fn to_non_empty(self) -> Option<ItemStack> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    /// Modify this item stack and make it empty after returning the contained item stack,
    /// only if not already empty.
    pub fn take_non_empty(&mut self) -> Option<ItemStack> {
        let ret = self.to_non_empty();
        *self = Self::EMPTY;
        ret
    }

    /// Increment damage to this item, if max damage is reached for that item, the stack
    /// size will be decremented (saturating at 0).
    pub fn inc_damage(mut self, amount: u16) -> ItemStack {
        if amount != 0 {
            self.damage = self.damage.saturating_add(amount);
            if self.damage > from_id(self.id).max_damage {
                self.size = self.size.saturating_sub(1);
                self.damage = 0;
            }
        }
        self
    }

}
