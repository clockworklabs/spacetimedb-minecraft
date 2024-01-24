//! NBT serialization and deserialization for [`ItemStack`] type.

use crate::serde::nbt::{NbtParseError, NbtCompound, NbtCompoundParse};
use crate::item::ItemStack;

/// Create an item stack from a NBT compound.
pub fn from_nbt(comp: NbtCompoundParse) -> Result<ItemStack, NbtParseError> {
    let id = comp.get_short("id")? as u16;
    let size = comp.get_byte("Count")?.max(0) as u16;
    let damage = comp.get_short("Damage")? as u16;
    Ok(ItemStack { id, size, damage })
}

/// Encode an item stack into a NBT compound.
pub fn to_nbt(comp: &mut NbtCompound, stack: ItemStack) -> &mut NbtCompound {
    comp.insert("id", stack.id);
    comp.insert("Count", stack.size.min(i8::MAX as _) as i8);
    comp.insert("Damage", stack.damage);
    comp
}
