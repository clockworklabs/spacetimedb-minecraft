//! Plants feature generation.

use glam::IVec3;

use crate::block::material::Material;
use crate::rand::JavaRandom;
use crate::world::World;
use crate::geom::Face;
use crate::block;

use super::FeatureGenerator;


/// A generator for flower patch.
pub struct PlantGenerator {
    plant_id: u8,
    plant_metadata: u8,
    count: u8,
    find_ground: bool,
}

impl PlantGenerator {

    #[inline]
    pub fn new(plant_id: u8, plant_metadata: u8, count: u8, find_ground: bool) -> Self {
        Self { 
            plant_id, 
            plant_metadata,
            count,
            find_ground,
        }
    }

    #[inline]
    pub fn new_flower(flower_id: u8) -> Self {
        Self::new(flower_id, 0, 64, false)
    }

    #[inline]
    pub fn new_tall_grass(metadata: u8) -> Self {
        Self::new(block::TALL_GRASS, metadata, 128, true)
    }

    #[inline]
    pub fn new_dead_bush() -> Self {
        Self::new(block::DEAD_BUSH, 0, 4, true)
    }

}

impl FeatureGenerator for PlantGenerator {

    fn generate(&mut self, world: &mut World, mut pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        if self.find_ground {
            while pos.y > 0 {
                if !matches!(world.get_block(pos), Some((block::AIR | block::LEAVES, _))) {
                    break;
                }
                pos.y -= 1;
            }
        }

        for _ in 0..self.count {
            
            let place_pos = pos + IVec3 {
                x: rand.next_int_bounded(8) - rand.next_int_bounded(8),
                y: rand.next_int_bounded(4) - rand.next_int_bounded(4),
                z: rand.next_int_bounded(8) - rand.next_int_bounded(8),
            };

            // PARITY: Check parity of "canBlockStay"...
            if world.is_block_air(place_pos) && world.can_place_block(place_pos, Face::NegY, self.plant_id) {
                world.set_block(place_pos, self.plant_id, self.plant_metadata);
            }

        }

        true

    }

}


/// A generator for sugar canes.
pub struct SugarCanesGenerator(());

impl SugarCanesGenerator {
    #[inline]
    pub fn new() -> Self {
        Self(())
    }
}

impl FeatureGenerator for SugarCanesGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        for _ in 0..20 {

            let place_pos = pos + IVec3 {
                x: rand.next_int_bounded(4) - rand.next_int_bounded(4),
                y: 0,
                z: rand.next_int_bounded(4) - rand.next_int_bounded(4),
            };

            if world.is_block_air(place_pos) {
                
                for face in Face::HORIZONTAL {
                    if world.get_block_material(place_pos - IVec3::Y + face.delta()) == Material::Water {

                        let v = rand.next_int_bounded(3) + 1;
                        let height = rand.next_int_bounded(v) + 2;

                        // Check that the bottom cane can be placed.
                        if world.can_place_block(place_pos, Face::NegY, block::SUGAR_CANES) {
                            for dy in 0..height {
                                world.set_block(place_pos + IVec3::new(0, dy, 0), block::SUGAR_CANES, 0);
                            }
                        }

                    }
                }

            }

        }

        true

    }

}


/// A generator for pumpkin.
pub struct PumpkinGenerator(());

impl PumpkinGenerator {
    #[inline]
    pub fn new() -> Self {
        Self(())
    }
}

impl FeatureGenerator for PumpkinGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        for _ in 0..64 {
            
            let place_pos = pos + IVec3 {
                x: rand.next_int_bounded(8) - rand.next_int_bounded(8),
                y: rand.next_int_bounded(4) - rand.next_int_bounded(4),
                z: rand.next_int_bounded(8) - rand.next_int_bounded(8),
            };

            // PARITY: Check parity of "canBlockStay"...
            if world.is_block_air(place_pos) && world.is_block(place_pos - IVec3::Y, block::GRASS) {
                world.set_block(place_pos, block::PUMPKIN, rand.next_int_bounded(4) as u8);
            }

        }

        true

    }

}


/// A generator for cactus.
pub struct CactusGenerator(());

impl CactusGenerator {
    #[inline]
    pub fn new() -> Self {
        Self(())
    }
}

impl FeatureGenerator for CactusGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        for _ in 0..10 {

            let place_pos = pos + IVec3 {
                x: rand.next_int_bounded(8) - rand.next_int_bounded(8),
                y: rand.next_int_bounded(4) - rand.next_int_bounded(4),
                z: rand.next_int_bounded(8) - rand.next_int_bounded(8),
            };

            if world.is_block_air(place_pos) {
                
                let v = rand.next_int_bounded(3) + 1;
                let height = rand.next_int_bounded(v) + 1;

                // Check that the bottom cane can be placed.
                for dy in 0..height {
                    if world.can_place_block(place_pos, Face::NegY, block::CACTUS) {
                        world.set_block(place_pos + IVec3::new(0, dy, 0), block::CACTUS, 0);
                    }
                }

            }

        }

        true

    }

}