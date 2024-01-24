//! Cave generation utility.

use glam::{IVec3, DVec3};

use crate::rand::JavaRandom;
use crate::chunk::Chunk;
use crate::block;

use super::math::MinecraftMath;


/// A cave generator.
pub struct CaveGenerator {
    /// Max chunk radius for the caves.
    radius: u8,
}

impl CaveGenerator {

    pub fn new(radius: u8) -> Self {
        Self {
            radius,
        }
    }

    /// Generate all caves in the given chunk.
    pub fn generate(&self, cx: i32, cz: i32, chunk: &mut Chunk, seed: i64) {

        let mut rand = JavaRandom::new(seed);

        let x_mul = rand.next_long().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
        let z_mul = rand.next_long().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
        let radius = self.radius as i32;

        for from_cx in cx - radius..=cx + radius {
            for from_cz in cz - radius..=cz + radius {
                
                let chunk_seed = i64::wrapping_add(
                    (from_cx as i64).wrapping_mul(x_mul), 
                    (from_cz as i64).wrapping_mul(z_mul)
                ) ^ seed;

                rand.set_seed(chunk_seed);
                self.generate_from(from_cx, from_cz, cx, cz, chunk, &mut rand);
                
            }
        }

    }

    /// Internal function to generate a cave from a chunk and modify the chunk if that
    /// cave come in.
    fn generate_from(&self, from_cx: i32, from_cz: i32, cx: i32, cz: i32, chunk: &mut Chunk, rand: &mut JavaRandom) {

        let count = rand.next_int_bounded(40);
        let count = rand.next_int_bounded(count + 1);
        let count = rand.next_int_bounded(count + 1);

        if rand.next_int_bounded(15) != 0 {
            return;
        }

        for _ in 0..count {

            let start = IVec3 {
                x: from_cx * 16 + rand.next_int_bounded(16),
                y: {
                    let v = rand.next_int_bounded(120);
                    rand.next_int_bounded(v + 8)
                },
                z: from_cz * 16 + rand.next_int_bounded(16),
            }.as_dvec3();

            let mut normal_count = 1;
            if rand.next_int_bounded(4) == 0 {
                let start_width = rand.next_float() * 6.0 + 1.0;
                self.generate_node(cx, cz, chunk, rand, start, start_width, 0.0, 0.0, -1, -1, 0.5);
                normal_count += rand.next_int_bounded(4);
            }

            for _ in 0..normal_count {
                let yaw = rand.next_float() * f32::MC_PI * 2.0;
                let pitch = (rand.next_float() - 0.5) * 2.0 / 8.0;
                let start_width = rand.next_float() * 2.0 + rand.next_float();
                self.generate_node(cx, cz, chunk, rand, start, start_width, yaw, pitch, 0, 0, 1.0);
            }

        }

    }

    /// Generate a cave node with the given properties.
    fn generate_node(&self, 
        cx: i32, cz: i32, chunk: &mut Chunk, chunk_rand: &mut JavaRandom,
        mut pos: DVec3, 
        start_width: f32, 
        mut yaw: f32, 
        mut pitch: f32,
        mut offset: i32,
        mut length: i32,
        height_scale: f64,
    ) {

        let cx_mid = (cx * 16 + 8) as f64;
        let cz_mid = (cz * 16 + 8) as f64;

        let mut rand = JavaRandom::new(chunk_rand.next_long());

        // The length is the maximum length of the cave from start point to any end.
        if length <= 0 {
            let v = self.radius as i32 * 16 - 16;
            length = v - rand.next_int_bounded(v / 4);
        }

        // The offset is the current generation point in the length of the cave, must
        // be in range 0..length.
        let mut auto_offset = false;
        if offset == -1 {
            offset = length / 2;
            auto_offset = true;
        }

        debug_assert!(offset >= 0 && offset < length);

        // The is the offset where the next nodes will be generated.
        let new_nodes_offset = rand.next_int_bounded(length / 2) + length / 4;
        // Determine if the cave will be less chaotic.
        let stable_pitch = rand.next_int_bounded(6) == 0;

        let mut pitch_scale = 0.0;
        let mut yaw_scale = 0.0;

        'main: for offset in offset..length {

            // The sine here is made is used to make the cave less large at the ends.
            let width = 1.5 + ((offset as f32 * f32::MC_PI / length as f32).mc_sin() * start_width * 1.0) as f64;
            let height = width * height_scale;

            let (pitch_sin, pitch_cos) = pitch.mc_sin_cos();
            let (yaw_sin, yaw_cos) = yaw.mc_sin_cos();

            pos.x += (yaw_cos * pitch_cos) as f64;
            pos.y += pitch_sin as f64;
            pos.z += (yaw_sin * pitch_cos) as f64;

            // Here we stabilize pitch around 0 degrees to make the cave more horizontal.
            if stable_pitch {
                pitch *= 0.92;
            } else {
                pitch *= 0.7;
            }

            pitch += pitch_scale * 0.1;
            yaw += yaw_scale * 0.1;
            pitch_scale *= 0.9;
            yaw_scale *= 12.0 / 16.0;
            pitch_scale += (rand.next_float() - rand.next_float()) * rand.next_float() * 2.0;
            yaw_scale += (rand.next_float() - rand.next_float()) * rand.next_float() * 4.0;

            // Generate two perpendicular nodes to the current offset.
            if !auto_offset && offset == new_nodes_offset && start_width > 1.0 {

                self.generate_node(cx, cz, chunk, chunk_rand,
                    pos, 
                    rand.next_float() * 0.5 + 0.5, 
                    yaw - f32::MC_PI * 0.5, 
                    pitch / 3.0, 
                    offset, length, 1.0);

                self.generate_node(cx, cz, chunk, chunk_rand,
                    pos, 
                    rand.next_float() * 0.5 + 0.5, 
                    yaw + f32::MC_PI * 0.5, 
                    pitch / 3.0, 
                    offset, length, 1.0);
                
                return;

            }

            if !auto_offset && rand.next_int_bounded(4) == 0 {
                continue;
            }

            let cx_mid_delta = pos.x - cx_mid;
            let cz_mid_delta = pos.z - cz_mid;
            let remaining_length = (length - offset) as f64;
            let c = (start_width + 2.0 + 16.0) as f64;

            // Heuristic to abort the cave generation if we are too far from the target
            // chunk middle.
            if cx_mid_delta.powi(2) + cz_mid_delta.powi(2) - remaining_length.powi(2) > c.powi(2) {
                return;
            }

            // The following code is used to actually carve the cave into the target 
            // chunk, this condition shortcut if we are too far from target chunk.
            if pos.x < cx_mid - 16.0 - width * 2.0 || pos.z < cz_mid - 16.0 - width * 2.0 || pos.x > cx_mid + 16.0 + width * 2.0 || pos.z > cz_mid + 16.0 + width * 2.0 {
                continue;
            }

            let size = DVec3::new(width, height, width);

            // Calculate the absolute start/end of the zone to carve out.
            let mut start = (pos - size).floor().as_ivec3();
            let mut end = (pos + size).floor().as_ivec3();

            // Calculate relative chunk coordinates.
            start -= IVec3::new(cx * 16 + 1, 1, cz * 16 + 1);
            end -= IVec3::new(cx * 16 - 1, -1, cz * 16 - 1);

            // Finally clamp the values to be valid for chunk coordinates.
            // NOTE: End is exclusive.
            let start = start.max(IVec3::new(0, 1, 0));
            let end = end.min(IVec3::new(16, 120, 16));

            // Check all block an abort if water is present in the carve area.
            for bx in start.x..end.x {
                for bz in start.z..end.z {
                    let mut by = end.y + 1;
                    while by >= start.y - 1 {
                        if by < 128 {

                            let carve_pos = IVec3::new(bx, by, bz);

                            if let (block::WATER_MOVING | block::WATER_STILL, _) = chunk.get_block(carve_pos) {
                                // Encountered water, do not carve this.
                                continue 'main;
                            } else if by != start.y - 1 && bx != start.x && bx != end.x - 1 && bz != start.z && bz != end.z - 1 {
                                // NOTE: I don't really understand that...
                                by = start.y;
                            }
    
                            by -= 1;
    
                        }
                    }
                }
            }

            // Finally do the carving.
            for bx in start.x..end.x {
                let dx = ((bx + cx * 16) as f64 + 0.5 - pos.x) / width;
                for bz in start.z..end.z {
                    let dz = ((bz + cz * 16) as f64 + 0.5 - pos.z) / width;

                    // We carve a cylinder.<
                    let xz_dist_sq = dx.powi(2) + dz.powi(2);
                    if xz_dist_sq >= 1.0 {
                        continue;
                    }

                    // Set to true whenever we carve a grass block.
                    let mut carving_surface = false;

                    for by in (start.y..=end.y - 1).rev() {
                        let dy = (by as f64 + 0.5 - pos.y) / height;

                        // We carve a ball.
                        if dy <= -0.7 || xz_dist_sq + dy.powi(2) >= 1.0 {
                            continue;
                        }

                        // NOTE: +1 because the java code is weird.
                        let carve_pos = IVec3::new(bx, by + 1, bz);
                        let (prev_id, _) = chunk.get_block(carve_pos);

                        // Read above.
                        if prev_id == block::GRASS {
                            carving_surface  = true;
                        }

                        // Only carve these blocks.
                        if let block::STONE | block::DIRT | block::GRASS = prev_id {
                            if by < 10 {
                                // Place a lava below y 10, it seems that the Notchian
                                // implementation place moving lava in order to use the
                                // random tick to make lava flowing.
                                chunk.set_block(carve_pos, block::LAVA_MOVING, 0);
                            } else {
                                // Just place air.
                                chunk.set_block(carve_pos, block::AIR, 0);
                                // If we are carving surface and the block below is dirt,
                                // replace it with grass. This also explains why we go
                                // from end Y to start Y.
                                if carving_surface {
                                    let below_pos = carve_pos - IVec3::Y;
                                    if let (block::DIRT, _) = chunk.get_block(below_pos) {
                                        chunk.set_block(below_pos, block::GRASS, 0);
                                    }
                                }
                            }
                        }

                    }

                }
            }

            // Auto offset just generate one node.
            if auto_offset {
                break;
            }

        }

    }

}
