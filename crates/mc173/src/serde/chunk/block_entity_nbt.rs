//! NBT serialization and deserialization for [`BlockEntity`] type.

use glam::IVec3;

use crate::block_entity::note_block::NoteBlockBlockEntity;
use crate::block_entity::dispenser::DispenserBlockEntity;
use crate::block_entity::furnace::FurnaceBlockEntity;
use crate::block_entity::jukebox::JukeboxBlockEntity;
use crate::block_entity::spawner::SpawnerBlockEntity;
use crate::block_entity::piston::PistonBlockEntity;
use crate::block_entity::chest::ChestBlockEntity;
use crate::block_entity::sign::SignBlockEntity;
use crate::block_entity::BlockEntity;
use crate::entity::EntityKind;
use crate::item::ItemStack;
use crate::geom::Face;

use crate::serde::nbt::{NbtParseError, NbtCompound, NbtCompoundParse};

use super::entity_kind_nbt;
use super::slot_nbt;

pub fn from_nbt(comp: NbtCompoundParse) -> Result<(IVec3, Box<BlockEntity>), NbtParseError> {

    let x = comp.get_int("x")?;
    let y = comp.get_int("y")?;
    let z = comp.get_int("z")?;

    let id = comp.get_string("id")?;
    let block_entity = Box::new(match id {
        "Chest" => {
            let mut chest = ChestBlockEntity::default();
            slot_nbt::from_nbt_to_inv(comp.get_list("Items")?, &mut chest.inv[..])?;
            BlockEntity::Chest(chest)
        }
        "Furnace" => {
            let mut inv = [ItemStack::EMPTY; 3];
            slot_nbt::from_nbt_to_inv(comp.get_list("Items")?, &mut inv[..])?;
            let mut furnace = FurnaceBlockEntity::default();
            furnace.input_stack = inv[0];
            furnace.fuel_stack = inv[1];
            furnace.output_stack = inv[2];
            furnace.burn_remaining_ticks = comp.get_short("BurnTime")?.max(0) as u16;
            furnace.smelt_ticks = comp.get_short("CookTime")?.max(0) as u16;
            // TODO: burn max ticks
            BlockEntity::Furnace(furnace)
        }
        "Trap" => {
            let mut dispenser = DispenserBlockEntity::default();
            slot_nbt::from_nbt_to_inv(comp.get_list("Items")?, &mut dispenser.inv[..])?;
            BlockEntity::Dispenser(dispenser)
        }
        "MobSpawner" => {
            let mut spawner = SpawnerBlockEntity::default();
            spawner.entity_kind = entity_kind_nbt::from_nbt(comp.get_string("EntityId")?).unwrap_or(EntityKind::Pig);
            spawner.remaining_time = comp.get_short("Delay")? as u16;
            BlockEntity::Spawner(spawner)
        }
        "Music" => {
            let mut note_block = NoteBlockBlockEntity::default();
            note_block.note = comp.get_byte("note")? as u8;
            BlockEntity::NoteBlock(note_block)
        }
        "Piston" => {
            let mut piston = PistonBlockEntity::default();
            piston.block = comp.get_int("blockId")? as u8;
            piston.metadata = comp.get_int("blockData")? as u8;
            piston.face = match comp.get_int("facing")? {
                0 => Face::NegY,
                1 => Face::PosY,
                2 => Face::NegZ,
                3 => Face::PosZ,
                4 => Face::NegX,
                _ => Face::PosX,
            };
            piston.progress = comp.get_float("progress")?;
            piston.extending = comp.get_boolean("extending")?;
            BlockEntity::Piston(piston)
        }
        "Sign" => {
            let mut sign = SignBlockEntity::default();
            for (i, key) in ["Text1", "Text2", "Text3", "Text4"].into_iter().enumerate() {
                sign.lines[i] = comp.get_string(key)?.to_string();
            }
            BlockEntity::Sign(sign)
        }
        "RecordPlayer" => {
            BlockEntity::Jukebox(JukeboxBlockEntity { 
                record: comp.get_int("Record")? as u32
            })
        }
        _ => return Err(NbtParseError::new(format!("{}/id", comp.path()), "valid block entity id"))
    });

    Ok((IVec3::new(x, y, z), block_entity))

}

pub fn to_nbt<'a>(comp: &'a mut NbtCompound, pos: IVec3, block_entity: &BlockEntity) -> &'a mut NbtCompound {

    comp.insert("x", pos.x);
    comp.insert("y", pos.y);
    comp.insert("z", pos.z);

    match block_entity {
        BlockEntity::Chest(chest) => {
            comp.insert("id", "Chest");
            comp.insert("Items", slot_nbt::to_nbt_from_inv(&chest.inv[..]));
        }
        BlockEntity::Furnace(furnace) => {
            comp.insert("id", "Furnace");
            comp.insert("Items", slot_nbt::to_nbt_from_inv(&[furnace.input_stack, furnace.fuel_stack, furnace.output_stack]));
            comp.insert("BurnTime", furnace.burn_remaining_ticks);
            comp.insert("CookTime", furnace.smelt_ticks);
        }
        BlockEntity::Dispenser(dispenser) => {
            comp.insert("id", "Trap");
            comp.insert("Items", slot_nbt::to_nbt_from_inv(&dispenser.inv[..]));
        }
        BlockEntity::Spawner(spawner) => {
            comp.insert("id", "MobSpawner");
            comp.insert("EntityId", entity_kind_nbt::to_nbt(spawner.entity_kind).unwrap_or(format!("Pig")));
            comp.insert("Delay", spawner.remaining_time.min(i16::MAX as _) as i16);
        }
        BlockEntity::NoteBlock(note_block) => {
            comp.insert("id", "Music");
            comp.insert("note", note_block.note);
        }
        BlockEntity::Piston(piston) => {
            comp.insert("id", "Piston");
            comp.insert("blockId", piston.block as u32);
            comp.insert("blockData", piston.metadata as u32);
            comp.insert("facing", match piston.face {
                Face::NegY => 0i32,
                Face::PosY => 1,
                Face::NegZ => 2,
                Face::PosZ => 3,
                Face::NegX => 4,
                Face::PosX => 5,
            });
            comp.insert("progress", piston.progress);
            comp.insert("extending", piston.extending);
        }
        BlockEntity::Sign(sign) => {
            comp.insert("id", "Sign");
            for (i, key) in ["Text1", "Text2", "Text3", "Text4"].into_iter().enumerate() {
                comp.insert(key, sign.lines[i].as_str());
            }
        }
        BlockEntity::Jukebox(jukebox) => {
            comp.insert("id", "RecordPlayer");
            comp.insert("Record", jukebox.record);
        }
    }

    comp

}
