//! Advanced block placing methods.

use glam::IVec3;

// use crate::block_entity::BlockEntity;
use crate::block::material::Material;
use crate::util::default as def;
use crate::geom::Face;
use crate::block;
use crate::chunk_cache::ChunkCache;
use super::StdbWorld;


/// Methods related to block placing.
impl StdbWorld {

    /// This function checks if the given block id can be placed at a particular position in
    /// the world, the given face indicates toward which face this block should be oriented.
    pub fn can_place_block(&mut self, pos: IVec3, face: Face, id: u8, cache: &mut ChunkCache) -> bool {
        
        let base = match id {
            block::BUTTON if face.is_y() => false,
            block::BUTTON => self.is_block_opaque_cube(pos + face.delta(), cache),
            block::LEVER if face == Face::PosY => false,
            block::LEVER => self.is_block_opaque_cube(pos + face.delta(), cache),
            block::LADDER => self.is_block_opaque_around(pos, cache),
            block::TRAPDOOR if face.is_y() => false,
            block::TRAPDOOR => self.is_block_opaque_cube(pos + face.delta(), cache),
            block::PISTON_EXT |
            block::PISTON_MOVING => false,
            block::DEAD_BUSH => matches!(self.get_block(pos - IVec3::Y, cache), Some((block::SAND, _))),
            // PARITY: Notchian impl checks block light >= 8 or see sky
            block::DANDELION |
            block::POPPY |
            block::SAPLING |
            block::TALL_GRASS => matches!(self.get_block(pos - IVec3::Y, cache), Some((block::GRASS | block::DIRT | block::FARMLAND, _))),
            block::WHEAT => matches!(self.get_block(pos - IVec3::Y, cache), Some((block::FARMLAND, _))),
            block::CACTUS => self.can_place_cactus(pos, cache),
            block::SUGAR_CANES => self.can_place_sugar_canes(pos, cache),
            block::CAKE => self.is_block_solid(pos - IVec3::Y, cache),
            block::CHEST => self.can_place_chest(pos, cache),
            block::WOOD_DOOR |
            block::IRON_DOOR => self.can_place_door(pos, cache),
            block::FENCE => matches!(self.get_block(pos - IVec3::Y, cache), Some((block::FENCE, _))) || self.is_block_solid(pos - IVec3::Y, cache),
            block::FIRE => self.can_place_fire(pos, cache),
            block::TORCH |
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT => self.is_block_opaque_cube(pos + face.delta(), cache),
            // Common blocks that needs opaque block below.
            block::RED_MUSHROOM |        // PARITY: Notchian impl checks block light >= 8 or see sky
            block::BROWN_MUSHROOM |      // PARITY: Notchian impl checks block light >= 8 or see sky
            block::WOOD_PRESSURE_PLATE |
            block::STONE_PRESSURE_PLATE |
            block::PUMPKIN |
            block::PUMPKIN_LIT |
            block::RAIL | 
            block::POWERED_RAIL |
            block::DETECTOR_RAIL |
            block::REPEATER |
            block::REPEATER_LIT |
            block::REDSTONE |
            block::SNOW => self.is_block_opaque_cube(pos - IVec3::Y, cache),
            _ => true,
        };

        // If the block we are placing has an exclusion box and any hard entity is inside,
        // we cancel the prevent the placing.
        // if let Some(bb) = self.get_block_exclusion_box(pos, id) {
        //     if self.has_entity_colliding(bb, true) {
        //         return false;
        //     }
        // }

        base && self.is_block_replaceable(pos, cache)

    }

    fn can_place_cactus(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        for face in Face::HORIZONTAL {
            if self.is_block_solid(pos + face.delta(), cache) {
                return false;
            }
        }
        matches!(self.get_block(pos - IVec3::Y, cache), Some((block::CACTUS | block::SAND, _)))
    }

    fn can_place_sugar_canes(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        let below_pos = pos - IVec3::Y;
        if let Some((block::SUGAR_CANES | block::GRASS | block::DIRT, _)) = self.get_block(below_pos, cache) {
            for face in Face::HORIZONTAL {
                if self.get_block_material(below_pos + face.delta(), cache) == Material::Water {
                    return true;
                }
            }
        }
        false
    }

    fn can_place_chest(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        let mut found_single_chest = false;
        for face in Face::HORIZONTAL {
            // If block on this face is a chest, check if that block also has a chest.
            let neighbor_pos = pos + face.delta();
            if matches!(self.get_block(neighbor_pos, cache), Some((block::CHEST, _))) {
                // We can't put chest
                if found_single_chest {
                    return false;
                }
                // Check if the chest we found isn't a double chest.
                for neighbor_face in Face::HORIZONTAL {
                    // Do not check our potential position.
                    if face != neighbor_face.opposite() {
                        if matches!(self.get_block(neighbor_pos + neighbor_face.delta(), cache), Some((block::CHEST, _))) {
                            return false; // The chest found already is double.
                        }
                    }
                }
                // No other chest found, it's a single chest.
                found_single_chest = true;
            }
        }
        true
    }

    fn can_place_door(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        self.is_block_opaque_cube(pos - IVec3::Y, cache) && self.is_block_replaceable(pos + IVec3::Y, cache)
    }

    fn can_place_fire(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        if self.is_block_opaque_cube(pos - IVec3::Y, cache) {
            true
        } else {
            for face in Face::ALL {
                if let Some((block, _)) = self.get_block(pos + face.delta(), cache) {
                    if block::material::get_fire_flammability(block) != 0 {
                        return true;
                    }
                }
            }
            false
        }
    }

    /// Place the block at the given position in the world oriented toward given face. Note
    /// that this function do not check if this is legal, it will do what's asked. Also, the
    /// given metadata may be modified to account for the placement.
    pub fn place_block(&mut self, pos: IVec3, face: Face, id: u8, metadata: u8, cache: &mut ChunkCache) {
        
        match id {
            block::BUTTON => self.place_faced(pos, face, id, metadata, block::button::set_face, cache),
            block::TRAPDOOR => self.place_faced(pos, face, id, metadata, block::trapdoor::set_face, cache),
            block::PISTON => self.place_faced(pos, face, id, metadata, block::piston::set_face, cache),
            block::WOOD_STAIR | 
            block::COBBLESTONE_STAIR => self.place_faced(pos, face, id, metadata, block::stair::set_face, cache),
            block::REPEATER | 
            block::REPEATER_LIT => self.place_faced(pos, face, id, metadata, block::repeater::set_face, cache),
            block::PUMPKIN | 
            block::PUMPKIN_LIT => self.place_faced(pos, face, id, metadata, block::pumpkin::set_face, cache),
            block::FURNACE | 
            block::FURNACE_LIT |
            block::DISPENSER => self.place_faced(pos, face, id, metadata, block::dispenser::set_face, cache),
            block::TORCH |
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT => self.place_faced(pos, face, id, metadata, block::torch::set_face, cache),
            block::LEVER => self.place_lever(pos, face, metadata, cache),
            block::LADDER => self.place_ladder(pos, face, metadata, cache),
            _ => {
                self.set_block_notify(pos, id, metadata, cache);
            }
        }

        // match id {
        //     block::CHEST => self.set_block_entity(pos, BlockEntity::Chest(def())),
        //     block::FURNACE => self.set_block_entity(pos, BlockEntity::Furnace(def())),
        //     block::DISPENSER => self.set_block_entity(pos, BlockEntity::Dispenser(def())),
        //     block::SPAWNER => self.set_block_entity(pos, BlockEntity::Spawner(def())),
        //     block::NOTE_BLOCK => self.set_block_entity(pos, BlockEntity::NoteBlock(def())),
        //     block::JUKEBOX => self.set_block_entity(pos, BlockEntity::Jukebox(def())),
        //     _ => {}
        // }

    }

    /// Generic function to place a block that has a basic facing function.
    fn place_faced(&mut self, pos: IVec3, face: Face, id: u8, mut metadata: u8, func: impl FnOnce(&mut u8, Face), cache: &mut ChunkCache) {
        func(&mut metadata, face);
        self.set_block_notify(pos, id, metadata, cache);
    }

    fn place_lever(&mut self, pos: IVec3, face: Face, mut metadata: u8, cache: &mut ChunkCache) {
        // When facing down, randomly pick the orientation.
        block::lever::set_face(&mut metadata, face, match face {
            Face::NegY => self.rand.next_choice(&[Face::PosZ, Face::PosX]),
            _ => Face::PosY,
        });
        self.set_block_notify(pos, block::LEVER, metadata, cache);
    }

    fn place_ladder(&mut self, pos: IVec3, mut face: Face, mut metadata: u8, cache: &mut ChunkCache) {
        // Privileging desired face, but if desired face cannot support a ladder.
        if face.is_y() || !self.is_block_opaque_cube(pos + face.delta(), cache) {
            // NOTE: Order is important for parity with client.
            for around_face in [Face::PosZ, Face::NegZ, Face::PosX, Face::NegX] {
                if self.is_block_opaque_cube(pos + around_face.delta(), cache) {
                    face = around_face;
                    break;
                }
            }
        }
        block::ladder::set_face(&mut metadata, face);
        self.set_block_notify(pos, block::LADDER, metadata, cache);
    }

    /// Check is there are at least one opaque block around horizontally.
    fn is_block_opaque_around(&mut self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        for face in Face::HORIZONTAL {
            if self.is_block_opaque_cube(pos + face.delta(), cache) {
                return true;
            }
        }
        false
    }

}
