//! Interaction of players with blocks in the world.

use glam::IVec3;

use crate::block_entity::BlockEntity;
use crate::geom::Face;
use crate::block;

use super::World;


/// Methods related to block interactions when client clicks on a block.
impl World {

    /// Interact with a block at given position. This function returns true if an 
    /// interaction has been handled and some action happened to the world, which should
    /// typically prevent usage of the player's hand item.
    pub fn interact_block(&mut self, pos: IVec3) -> Interaction {
        if let Some((id, metadata)) = self.get_block(pos) {
            self.interact_block_unchecked(pos, id, metadata)
        } else {
            Interaction::None
        }
    }

    /// Internal function to handle block interaction at given position and with known
    /// block and metadata. The function returns true if an interaction has been handled.
    pub(super) fn interact_block_unchecked(&mut self, pos: IVec3, id: u8, metadata: u8) -> Interaction {
        match id {
            block::BUTTON => self.interact_button(pos, metadata),
            block::LEVER => self.interact_lever(pos, metadata),
            block::TRAPDOOR => self.interact_trapdoor(pos, metadata),
            block::IRON_DOOR => true,
            block::WOOD_DOOR => self.interact_wood_door(pos, metadata),
            block::REPEATER |
            block::REPEATER_LIT => self.interact_repeater(pos, id, metadata),
            block::REDSTONE_ORE => self.interact_redstone_ore(pos),
            block::CRAFTING_TABLE => return Interaction::CraftingTable { pos },
            block::CHEST => return self.interact_chest(pos),
            block::FURNACE |
            block::FURNACE_LIT => return self.interact_furnace(pos),
            block::DISPENSER => return self.interact_dispenser(pos),
            _ => return Interaction::None
        }.into()
    }

    /// Interact with a button block.
    fn interact_button(&mut self, pos: IVec3, mut metadata: u8) -> bool {
        if !block::button::is_active(metadata) {
            block::button::set_active(&mut metadata, true);
            self.set_block_notify(pos, block::BUTTON, metadata);
            self.schedule_block_tick(pos, block::BUTTON, 20);
        }
        true
    }

    fn interact_lever(&mut self, pos: IVec3, mut metadata: u8) -> bool {
        let active = block::lever::is_active(metadata);
        block::lever::set_active(&mut metadata, !active);
        self.set_block_notify(pos, block::LEVER, metadata);
        true
    }

    fn interact_trapdoor(&mut self, pos: IVec3, mut metadata: u8) -> bool {
        let active = block::trapdoor::is_open(metadata);
        block::trapdoor::set_open(&mut metadata, !active);
        self.set_block_notify(pos, block::TRAPDOOR, metadata);
        true
    }

    fn interact_wood_door(&mut self, pos: IVec3, mut metadata: u8) -> bool {

        if block::door::is_upper(metadata) {
            if let Some((block::WOOD_DOOR, metadata)) = self.get_block(pos - IVec3::Y) {
                self.interact_wood_door(pos - IVec3::Y, metadata);
            }
        } else {

            let open = block::door::is_open(metadata);
            block::door::set_open(&mut metadata, !open);

            self.set_block_notify(pos, block::WOOD_DOOR, metadata);

            if let Some((block::WOOD_DOOR, _)) = self.get_block(pos + IVec3::Y) {
                block::door::set_upper(&mut metadata, true);
                self.set_block_notify(pos + IVec3::Y, block::WOOD_DOOR, metadata);
            }

        }

        true

    }

    fn interact_repeater(&mut self, pos: IVec3, id: u8, mut metadata: u8) -> bool {
        let delay = block::repeater::get_delay(metadata);
        block::repeater::set_delay(&mut metadata, (delay + 1) % 4);
        self.set_block_notify(pos, id, metadata);
        true
    }

    fn interact_redstone_ore(&mut self, pos: IVec3) -> bool {
        self.set_block_notify(pos, block::REDSTONE_ORE_LIT, 0);
        false  // Notchian client lit the ore but do not mark the interaction.
    }

    fn interact_chest(&mut self, pos: IVec3) -> Interaction {

        let Some(BlockEntity::Chest(_)) = self.get_block_entity(pos) else {
            return Interaction::Handled
        };

        if self.is_block_opaque_cube(pos + IVec3::Y) {
            return Interaction::Handled;
        }

        for face in Face::HORIZONTAL {
            let face_pos = pos + face.delta();
            if self.is_block(face_pos, block::CHEST) && self.is_block_opaque_cube(face_pos + IVec3::Y) {
                return Interaction::Handled;
            }
        }

        let mut all_pos = vec![pos];

        // NOTE: Same order as Notchian server for parity, we also insert first or last
        // depending on the neighbor chest being on neg or pos face, like Notchian client.
        for face in [Face::NegX, Face::PosX, Face::NegZ, Face::PosZ] {
            let face_pos = pos + face.delta();
            if let Some(BlockEntity::Chest(_)) = self.get_block_entity(face_pos) {
                if face.is_neg() {
                    all_pos.insert(0, face_pos);
                } else {
                    all_pos.push(face_pos);
                }
            }
        }

        Interaction::Chest { pos: all_pos }

    }

    fn interact_furnace(&mut self, pos: IVec3) -> Interaction {
        if let Some(BlockEntity::Furnace(_)) = self.get_block_entity(pos) {
            Interaction::Furnace { pos }
        } else {
            Interaction::None
        }
    }

    fn interact_dispenser(&mut self, pos: IVec3) -> Interaction {
        if let Some(BlockEntity::Dispenser(_)) = self.get_block_entity(pos) {
            Interaction::Dispenser { pos }
        } else {
            Interaction::None
        }
    }

}


/// The result of an interaction with a block in the world.
#[derive(Debug, Clone)]
pub enum Interaction {
    /// No interaction has been handled.
    None,
    /// An interaction has been handled by the world.
    Handled,
    /// A crafting table has been interacted, the front-end should interpret this and 
    /// open the crafting table window.
    CraftingTable {
        /// Position of the crafting table being interacted.
        pos: IVec3,
    },
    /// A chest has been interacted, the front-end should interpret this and open the
    /// chest window.
    Chest {
        /// Positions of the chest block entities to connect to, from top layer in the
        /// window to bottom one. They have been checked to exists before.
        pos: Vec<IVec3>,
    },
    /// A furnace has been interacted, the front-end should interpret this and open the
    /// furnace window.
    Furnace {
        /// Position of the furnace block entity to connect to, it has been checked to
        /// exists.
        pos: IVec3,
    },
    /// A dispenser has been interacted, the front-end should interpret this and open
    /// the dispenser window.
    Dispenser {
        /// Position of the dispenser block entity to connect to, it has been checked to
        /// exists.
        pos: IVec3,
    },
}

impl From<bool> for Interaction {
    #[inline]
    fn from(value: bool) -> Self {
        if value { Self::Handled } else { Self::None }
    }
}
