//! Chunk serialization and deserialization from NBT compound.

use crate::world::ChunkSnapshot;

use super::nbt::{Nbt, NbtParseError, NbtCompound};

pub mod block_entity_nbt;
pub mod entity_kind_nbt;
pub mod item_stack_nbt;
pub mod entity_nbt;
pub mod slot_nbt;
pub mod chunk_nbt;

pub fn from_nbt(root: &Nbt) -> Result<ChunkSnapshot, NbtParseError> {
    chunk_nbt::from_nbt(root.parse().as_compound()?)
}

pub fn to_nbt(snapshot: &ChunkSnapshot) -> Nbt {
    let mut comp = NbtCompound::new();
    chunk_nbt::to_nbt(&mut comp, snapshot);
    Nbt::Compound(comp)
}
