//! NBT serialization and deserialization for [`ChunkSnapshot`] type.

use std::sync::Arc;

use crate::serde::nbt::{NbtCompoundParse, NbtCompound, NbtParseError, Nbt};
use crate::world::ChunkSnapshot;

use super::block_entity_nbt;
use super::entity_nbt;

pub fn from_nbt(comp: NbtCompoundParse) -> Result<ChunkSnapshot, NbtParseError> {

    let level = comp.get_compound("Level")?;
    let cx = level.get_int("xPos")?;
    let cz = level.get_int("zPos")?;

    let mut snapshot = ChunkSnapshot::new(cx, cz);
    let chunk = Arc::get_mut(&mut snapshot.chunk).unwrap();
    
    // This is annoying to make so much copies but we have no choice for know because 
    // this is not yet possible to directly deserialize into an existing buffer.
    chunk.block.copy_from_slice(level.get_byte_array("Blocks")?);
    chunk.metadata.inner.copy_from_slice(level.get_byte_array("Data")?);
    chunk.block_light.inner.copy_from_slice(level.get_byte_array("BlockLight")?);
    chunk.sky_light.inner.copy_from_slice(level.get_byte_array("SkyLight")?);
    chunk.height.copy_from_slice(level.get_byte_array("HeightMap")?);

    for item in level.get_list("Entities")?.iter() {
        let entity = entity_nbt::from_nbt(item.as_compound()?)?;
        snapshot.entities.push(entity);
    }

    for item in level.get_list("TileEntities")?.iter() {
        let (pos, block_entity) = block_entity_nbt::from_nbt(item.as_compound()?)?;
        snapshot.block_entities.insert(pos, block_entity);
    }

    Ok(snapshot)

}

pub fn to_nbt<'a>(comp: &'a mut NbtCompound, snapshot: &ChunkSnapshot) -> &'a mut NbtCompound {

    let mut level = NbtCompound::new();

    level.insert("xPos", snapshot.cx);
    level.insert("zPos", snapshot.cz);

    level.insert("Blocks", snapshot.chunk.block.to_vec());
    level.insert("Data", snapshot.chunk.metadata.inner.to_vec());
    level.insert("BlockLight", snapshot.chunk.block_light.inner.to_vec());
    level.insert("SkyLight", snapshot.chunk.sky_light.inner.to_vec());
    level.insert("HeightMap", snapshot.chunk.height.to_vec());

    level.insert("Entities", snapshot.entities.iter()
        .filter_map(|entity| {
            let mut comp = NbtCompound::new();
            if entity_nbt::to_nbt(&mut comp, &entity).is_some() {
                Some(Nbt::Compound(comp))
            } else {
                None
            }
        })
        .collect::<Vec<_>>());

    level.insert("TileEntities", snapshot.block_entities.iter()
        .map(|(&pos, block_entity)| {
            let mut comp = NbtCompound::new();
            block_entity_nbt::to_nbt(&mut comp, pos, &block_entity);
            Nbt::Compound(comp)
        })
        .collect::<Vec<_>>());

    comp.insert("Level", level);
    comp

}
