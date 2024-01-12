//! Clay and ore patch feature.

use glam::{IVec3, DVec3};

use crate::rand::JavaRandom;
use crate::world::World;
use crate::block;

use super::math::MinecraftMath;
use super::FeatureGenerator;


/// A generator for mob spawner dungeon.
pub struct VeinGenerator {
    replace_id: u8,
    place_id: u8,
    count: u8,
}

impl VeinGenerator {

    #[inline]
    pub fn new(replace_id: u8, place_id: u8, count: u8) -> Self {
        Self { 
            replace_id,
            place_id, 
            count,
        }
    }

    #[inline]
    pub fn new_clay(count: u8) -> Self {
        Self::new(block::SAND, block::CLAY, count)
    }

    #[inline]
    pub fn new_ore(place_id: u8, count: u8) -> Self {
        Self::new(block::STONE, place_id, count)
    }

}

impl FeatureGenerator for VeinGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {

        let angle = rand.next_float() * f32::MC_PI;
        let (angle_sin, angle_cos) = angle.mc_sin_cos();
        let angle_sin = angle_sin * self.count as f32 / 8.0;
        let angle_cos = angle_cos * self.count as f32 / 8.0;

        let line_start = DVec3 {
            x: ((pos.x + 8) as f32 + angle_sin) as f64,
            y: (pos.y + rand.next_int_bounded(3) + 2) as f64,
            z: ((pos.z + 8) as f32 + angle_cos) as f64,
        };

        let line_stop = DVec3 {
            x: ((pos.x + 8) as f32 - angle_sin) as f64,
            y: (pos.y + rand.next_int_bounded(3) + 2) as f64,
            z: ((pos.z + 8) as f32 - angle_cos) as f64,
        };

        for i in 0..=self.count {

            // Interpolation.
            let center_pos = line_start + (line_stop - line_start) * i as f64 / self.count as f64;

            let base_size = rand.next_double() * self.count as f64 / 16.0;
            let size = ((i as f32 * f32::MC_PI / self.count as f32).mc_sin() + 1.0) as f64 * base_size + 1.0;
            let half_size = size / 2.0;

            let start = (center_pos - half_size).floor().as_ivec3();
            let stop = (center_pos + half_size).floor().as_ivec3();

            for x in start.x..=stop.x {
                for z in start.z..=stop.z {
                    for y in start.y..=stop.y {

                        let place_pos = IVec3::new(x, y, z);
                        let delta = (place_pos.as_dvec3() + 0.5 - center_pos) / half_size;

                        if delta.length_squared() < 1.0 {
                            if world.is_block(place_pos, self.replace_id) {
                                world.set_block(place_pos, self.place_id, 0);
                            }
                        }

                    }
                }
            }

        }

        true

    }

}
