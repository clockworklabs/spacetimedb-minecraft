use spacetimedb::spacetimedb;
use mc173_module::item::ItemStack;

#[spacetimedb(table)]
pub struct StdbInventory {
    pub id: u32,
    // pub inv: [ItemStack; 27],
    inv: Vec<ItemStack>
}