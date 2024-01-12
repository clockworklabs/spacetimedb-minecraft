//! Block entities structures and logic implementation.

use glam::IVec3;

use crate::world::World;

pub mod chest;
pub mod furnace;
pub mod dispenser;
pub mod spawner;
pub mod note_block;
pub mod piston;
pub mod sign;
pub mod jukebox;


/// All kinds of block entities.
#[derive(Debug, Clone)]
pub enum BlockEntity {
    Chest(chest::ChestBlockEntity),
    Furnace(furnace::FurnaceBlockEntity),
    Dispenser(dispenser::DispenserBlockEntity),
    Spawner(spawner::SpawnerBlockEntity),
    NoteBlock(note_block::NoteBlockBlockEntity),
    Piston(piston::PistonBlockEntity),
    Sign(sign::SignBlockEntity),
    Jukebox(jukebox::JukeboxBlockEntity),
}

impl BlockEntity {

    /// Tick the block entity at its position in the world.
    pub fn tick(&mut self, world: &mut World, pos: IVec3) {
        match self {
            BlockEntity::Chest(_) => (),
            BlockEntity::Furnace(furnace) => furnace.tick(world, pos),
            BlockEntity::Dispenser(_) => (),
            BlockEntity::Spawner(spawner) => spawner.tick(world, pos),
            BlockEntity::NoteBlock(_) => (),
            BlockEntity::Piston(piston) => piston.tick(world, pos),
            BlockEntity::Sign(_) => (),
            BlockEntity::Jukebox(_) => (),
        }
    }

}
