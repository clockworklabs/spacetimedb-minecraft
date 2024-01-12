//! Dungeon generator.

use glam::IVec3;

use crate::block_entity::spawner::SpawnerBlockEntity;
use crate::block_entity::chest::ChestBlockEntity;
use crate::block_entity::BlockEntity;
use crate::item::{ItemStack, self};
use crate::entity::EntityKind;
use crate::rand::JavaRandom;
use crate::world::World;
use crate::geom::Face;
use crate::block;

use super::FeatureGenerator;


/// A generator for mob spawner dungeon.
pub struct DungeonGenerator {}

impl DungeonGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl DungeonGenerator {

    fn gen_chest_stack(&self, rand: &mut JavaRandom) -> ItemStack {
        match rand.next_int_bounded(11) {
            0 => ItemStack::new_single(item::SADDLE, 0),
            1 => ItemStack::new_sized(item::IRON_INGOT, 0, rand.next_int_bounded(4) as u16 + 1),
            2 => ItemStack::new_single(item::BREAD, 0),
            3 => ItemStack::new_single(item::BREAD, 0),
            4 => ItemStack::new_sized(item::GUNPOWDER, 0, rand.next_int_bounded(4) as u16 + 1),
            5 => ItemStack::new_sized(item::STRING, 0, rand.next_int_bounded(4) as u16 + 1),
            6 => ItemStack::new_single(item::BUCKET, 0),
            7 if rand.next_int_bounded(100) == 0 => 
                ItemStack::new_single(item::GOLD_APPLE, 0),
            8 if rand.next_int_bounded(2) == 0 => 
                ItemStack::new_sized(item::REDSTONE, 0, rand.next_int_bounded(4) as u16 + 1),
            9 if rand.next_int_bounded(10) == 0 => match rand.next_int_bounded(2) {
                0 => ItemStack::new_single(item::RECORD_13, 0),
                1 => ItemStack::new_single(item::RECORD_CAT, 0),
                _ => unreachable!(),
            }
            10 => ItemStack::new_single(item::DYE, 3),
            _ => ItemStack::EMPTY,
        }
    }

    fn gen_spawner_entity(&self, rand: &mut JavaRandom) -> EntityKind {
        match rand.next_int_bounded(4) {
            0 => EntityKind::Skeleton,
            1 | 2 => EntityKind::Zombie,
            3 => EntityKind::Spider,
            _ => unreachable!()
        }
    }

}

impl FeatureGenerator for DungeonGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        let x_radius = rand.next_int_bounded(2) + 2;
        let z_radius = rand.next_int_bounded(2) + 2;
        let height = 3;
        let mut air_count = 0usize;

        let start = pos - IVec3::new(x_radius + 1, 1, x_radius + 1);
        let end = pos + IVec3::new(x_radius + 1, height + 1, x_radius + 1);

        for x in start.x..=end.x {
            for y in start.y..=end.y {
                for z in start.z..=end.z {
                    
                    let check_pos = IVec3::new(x, y, z);
                    let check_material = world.get_block_material(check_pos);

                    if y == start.y && !check_material.is_solid() {
                        return false;
                    } else if y == end.y && !check_material.is_solid() {
                        return false;
                    } else if y == pos.y && (x == start.x || x == end.x || z == start.z || z == end.z) {
                        if world.is_block_air(check_pos) && world.is_block_air(check_pos + IVec3::Y) {
                            air_count += 1;
                        }
                    }

                }
            }
        }

        if air_count < 1 || air_count > 5 {
            return false;
        }

        // Carve the dungeon and fill walls.
        for x in start.x..=end.x {
            for y in (start.y..end.y).rev() {
                for z in start.z..=end.z {

                    // PARITY: Notchian impl actually use set_block_notify.

                    let carve_pos = IVec3::new(x, y, z);
                    if x != start.x && y != start.y && z != start.z && x != end.x && z != end.z {
                        world.set_block(carve_pos, block::AIR, 0);
                    } else if y >= 0 && !world.get_block_material(carve_pos - IVec3::Y).is_solid() {
                        world.set_block(carve_pos, block::AIR, 0);
                    } else if world.get_block_material(carve_pos).is_solid() {
                        if y == start.y && rand.next_int_bounded(4) != 0 {
                            world.set_block(carve_pos, block::MOSSY_COBBLESTONE, 0);
                        } else {
                            world.set_block(carve_pos, block::COBBLESTONE, 0);
                        }
                    }

                }
            }
        }

        // Place chests.
        for _ in 0..2 {
            
            'chest_try: for _ in 0..3 {

                let chest_pos = pos + IVec3 {
                    x: rand.next_int_bounded(x_radius * 2 + 1) - x_radius,
                    y: 0,
                    z: rand.next_int_bounded(z_radius * 2 + 1) - z_radius,
                };

                if world.is_block_air(pos) {

                    let mut solid_count = 0usize;
                    for face in Face::HORIZONTAL {
                        if world.get_block_material(chest_pos + face.delta()).is_solid() {
                            solid_count += 1;
                            if solid_count > 1 {
                                continue 'chest_try;
                            }
                        }
                    }

                    if solid_count == 0 {
                        continue 'chest_try;
                    }

                    let mut chest = ChestBlockEntity::default();

                    // Pick 8 random items.
                    for _ in 0..8 {
                        
                        let stack = self.gen_chest_stack(rand);
                        if !stack.is_empty() {
                            *rand.next_choice_mut(&mut chest.inv[..]) = stack;
                        }

                    }

                    world.set_block(chest_pos, block::CHEST, 0);
                    world.set_block_entity(chest_pos, BlockEntity::Chest(chest));
                    break;

                }

            }

        }

        let mut spawner = SpawnerBlockEntity::default();
        spawner.entity_kind = self.gen_spawner_entity(rand);
        world.set_block(pos, block::SPAWNER, 0);
        world.set_block_entity(pos, BlockEntity::Spawner(spawner));

        true

    }

}
