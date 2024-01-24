//! Chest block entity.

use crate::item::ItemStack;


#[derive(Debug, Clone, Default)]
pub struct ChestBlockEntity {
    /// The inventory of the chest.
    pub inv: Box<[ItemStack; 27]>,
}
