//! Common NBT serde functions for item slots.

use crate::serde::nbt::{NbtParseError, NbtCompoundParse, NbtCompound, NbtListParse, Nbt};
use crate::item::ItemStack;

use super::item_stack_nbt;

/// Create an slot and item stack from a NBT compound.
pub fn from_nbt(comp: NbtCompoundParse) -> Result<(u8, ItemStack), NbtParseError> {
    let slot = comp.get_byte("Slot")? as u8;
    let stack = item_stack_nbt::from_nbt(comp)?;
    Ok((slot, stack))
}

/// Encode a slot and item stack into a NBT compound.
pub fn to_nbt(comp: &mut NbtCompound, slot: u8, stack: ItemStack) -> &mut NbtCompound {
    comp.insert("Slot", slot);
    item_stack_nbt::to_nbt(comp, stack)
}

pub fn from_nbt_to_inv(list: NbtListParse, inv: &mut [ItemStack]) -> Result<(), NbtParseError> {
    for item in list.iter() {
        let (slot, stack) = from_nbt(item.as_compound()?)?;
        if (slot as usize) < inv.len() {
            inv[slot as usize] = stack;
        }
    }
    Ok(())
}

pub fn to_nbt_from_inv(inv: &[ItemStack]) -> Vec<Nbt> {
    let mut list = Vec::new();
    for (index, stack) in inv.iter().copied().enumerate() {
        if index < 256 && !stack.is_empty() {
            let mut comp = NbtCompound::new();
            to_nbt(&mut comp, index as u8, stack);
            list.push(comp.into());
        }
    }
    list
}
