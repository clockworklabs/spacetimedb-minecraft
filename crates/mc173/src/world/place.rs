//! Advanced block placing methods.

use glam::IVec3;

use crate::block_entity::BlockEntity;
use crate::block::material::Material;
use crate::util::default as def;
use crate::geom::Face;
use crate::block;

use super::World;


/// Methods related to block placing.
impl World {

    /// This function checks if the given block id can be placed at a particular position in
    /// the world, the given face indicates toward which face this block should be oriented.
    pub fn can_place_block(&mut self, pos: IVec3, face: Face, id: u8) -> bool {
        
        let base = match id {
            block::BUTTON if face.is_y() => false,
            block::BUTTON => self.is_block_opaque_cube(pos + face.delta()),
            block::LEVER if face == Face::PosY => false,
            block::LEVER => self.is_block_opaque_cube(pos + face.delta()),
            block::LADDER => self.is_block_opaque_around(pos),
            block::TRAPDOOR if face.is_y() => false,
            block::TRAPDOOR => self.is_block_opaque_cube(pos + face.delta()),
            block::PISTON_EXT |
            block::PISTON_MOVING => false,
            block::DEAD_BUSH => matches!(self.get_block(pos - IVec3::Y), Some((block::SAND, _))),
            // PARITY: Notchian impl checks block light >= 8 or see sky
            block::DANDELION |
            block::POPPY |
            block::SAPLING |
            block::TALL_GRASS => matches!(self.get_block(pos - IVec3::Y), Some((block::GRASS | block::DIRT | block::FARMLAND, _))),
            block::WHEAT => matches!(self.get_block(pos - IVec3::Y), Some((block::FARMLAND, _))),
            block::CACTUS => self.can_place_cactus(pos),
            block::SUGAR_CANES => self.can_place_sugar_canes(pos),
            block::CAKE => self.is_block_solid(pos - IVec3::Y),
            block::CHEST => self.can_place_chest(pos),
            block::WOOD_DOOR |
            block::IRON_DOOR => self.can_place_door(pos),
            block::FENCE => matches!(self.get_block(pos - IVec3::Y), Some((block::FENCE, _))) || self.is_block_solid(pos - IVec3::Y),
            block::FIRE => self.can_place_fire(pos),
            block::TORCH |
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT => self.is_block_opaque_cube(pos + face.delta()),
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
            block::SNOW => self.is_block_opaque_cube(pos - IVec3::Y),
            _ => true,
        };

        // If the block we are placing has an exclusion box and any hard entity is inside,
        // we cancel the prevent the placing.
        if let Some(bb) = self.get_block_exclusion_box(pos, id) {
            if self.has_entity_colliding(bb, true) {
                return false;
            }
        }

        base && self.is_block_replaceable(pos)

    }

    fn can_place_cactus(&mut self, pos: IVec3) -> bool {
        for face in Face::HORIZONTAL {
            if self.is_block_solid(pos + face.delta()) {
                return false;
            }
        }
        matches!(self.get_block(pos - IVec3::Y), Some((block::CACTUS | block::SAND, _)))
    }

    fn can_place_sugar_canes(&mut self, pos: IVec3) -> bool {
        let below_pos = pos - IVec3::Y;
        if let Some((block::SUGAR_CANES | block::GRASS | block::DIRT, _)) = self.get_block(below_pos) {
            for face in Face::HORIZONTAL {
                if self.get_block_material(below_pos + face.delta()) == Material::Water {
                    return true;
                }
            }
        }
        false
    }

    fn can_place_chest(&mut self, pos: IVec3) -> bool {
        let mut found_single_chest = false;
        for face in Face::HORIZONTAL {
            // If block on this face is a chest, check if that block also has a chest.
            let neighbor_pos = pos + face.delta();
            if matches!(self.get_block(neighbor_pos), Some((block::CHEST, _))) {
                // We can't put chest
                if found_single_chest {
                    return false;
                }
                // Check if the chest we found isn't a double chest.
                for neighbor_face in Face::HORIZONTAL {
                    // Do not check our potential position.
                    if face != neighbor_face.opposite() {
                        if matches!(self.get_block(neighbor_pos + neighbor_face.delta()), Some((block::CHEST, _))) {
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

    fn can_place_door(&mut self, pos: IVec3) -> bool {
        self.is_block_opaque_cube(pos - IVec3::Y) && self.is_block_replaceable(pos + IVec3::Y)
    }

    fn can_place_fire(&mut self, pos: IVec3) -> bool {
        if self.is_block_opaque_cube(pos - IVec3::Y) {
            true
        } else {
            for face in Face::ALL {
                if let Some((block, _)) = self.get_block(pos + face.delta()) {
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
    pub fn place_block(&mut self, pos: IVec3, face: Face, id: u8, metadata: u8) {
        
        match id {
            block::BUTTON => self.place_faced(pos, face, id, metadata, block::button::set_face),
            block::TRAPDOOR => self.place_faced(pos, face, id, metadata, block::trapdoor::set_face),
            block::PISTON => self.place_faced(pos, face, id, metadata, block::piston::set_face),
            block::WOOD_STAIR | 
            block::COBBLESTONE_STAIR => self.place_faced(pos, face, id, metadata, block::stair::set_face),
            block::REPEATER | 
            block::REPEATER_LIT => self.place_faced(pos, face, id, metadata, block::repeater::set_face),
            block::PUMPKIN | 
            block::PUMPKIN_LIT => self.place_faced(pos, face, id, metadata, block::pumpkin::set_face),
            block::FURNACE | 
            block::FURNACE_LIT |
            block::DISPENSER => self.place_faced(pos, face, id, metadata, block::dispenser::set_face),
            block::TORCH |
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT => self.place_faced(pos, face, id, metadata, block::torch::set_face),
            block::LEVER => self.place_lever(pos, face, metadata),
            block::LADDER => self.place_ladder(pos, face, metadata),
            _ => {
                self.set_block_notify(pos, id, metadata);
            }
        }

        match id {
            block::CHEST => self.set_block_entity(pos, BlockEntity::Chest(def())),
            block::FURNACE => self.set_block_entity(pos, BlockEntity::Furnace(def())),
            block::DISPENSER => self.set_block_entity(pos, BlockEntity::Dispenser(def())),
            block::SPAWNER => self.set_block_entity(pos, BlockEntity::Spawner(def())),
            block::NOTE_BLOCK => self.set_block_entity(pos, BlockEntity::NoteBlock(def())),
            block::JUKEBOX => self.set_block_entity(pos, BlockEntity::Jukebox(def())),
            _ => {}
        }

    }

    /// Generic function to place a block that has a basic facing function.
    fn place_faced(&mut self, pos: IVec3, face: Face, id: u8, mut metadata: u8, func: impl FnOnce(&mut u8, Face)) {
        func(&mut metadata, face);
        self.set_block_notify(pos, id, metadata);
    }

    fn place_lever(&mut self, pos: IVec3, face: Face, mut metadata: u8) {
        // When facing down, randomly pick the orientation.
        block::lever::set_face(&mut metadata, face, match face {
            Face::NegY => self.rand.next_choice(&[Face::PosZ, Face::PosX]),
            _ => Face::PosY,
        });
        self.set_block_notify(pos, block::LEVER, metadata);
    }

    fn place_ladder(&mut self, pos: IVec3, mut face: Face, mut metadata: u8) {
        // Privileging desired face, but if desired face cannot support a ladder.
        if face.is_y() || !self.is_block_opaque_cube(pos + face.delta()) {
            // NOTE: Order is important for parity with client.
            for around_face in [Face::PosZ, Face::NegZ, Face::PosX, Face::NegX] {
                if self.is_block_opaque_cube(pos + around_face.delta()) {
                    face = around_face;
                    break;
                }
            }
        }
        block::ladder::set_face(&mut metadata, face);
        self.set_block_notify(pos, block::LADDER, metadata);
    }

    /// Check is there are at least one opaque block around horizontally.
    fn is_block_opaque_around(&mut self, pos: IVec3) -> bool {
        for face in Face::HORIZONTAL {
            if self.is_block_opaque_cube(pos + face.delta()) {
                return true;
            }
        }
        false
    }

}
