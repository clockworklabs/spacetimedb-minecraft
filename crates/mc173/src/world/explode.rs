//! Make explosion in world.

use glam::{DVec3, IVec3};

use tracing::trace;

use crate::geom::BoundingBox;
use crate::rand::JavaRandom;

use crate::world::bound::RayTraceKind;
use crate::entity::{Entity, Hurt};
use crate::world::Event;
use crate::block;

use super::World;


/// Methods related to explosions.
impl World {

    /// Make an explosion in the world at the given position and size. The explosion can
    /// optionally propagate flames around.
    pub fn explode(&mut self, center: DVec3, radius: f32, set_fire: bool, origin_id: Option<u32>) {
        
        /// This is the step to advance each explosion ray.
        const STEP: f32 = 0.3;

        trace!("explode, center: {center}, radius: {radius}, set fire: {set_fire}, origin id: {origin_id:?}");

        let mut rand = JavaRandom::new_seeded();
        let mut affected_pos = Vec::new();

        // Start by computing each destroyed block.
        for dx in 0..16 {
            for dy in 0..16 {
                for dz in 0..16 {
                    if dx == 0 || dx == 15 || dy == 0 || dy == 15 || dz == 0 || dz == 15 {

                        // Calculate the normalized of the explosion ray.
                        let dir = (IVec3::new(dx, dy, dz).as_vec3() / 15.0) * 2.0 - 1.0;
                        let dir = dir.normalize() * STEP;
                        let dir = dir.as_dvec3();

                        // The initial intensity of this ray of explosion.
                        let mut intensity = radius * (0.7 + self.rand.next_float() * 0.6);
                        let mut check_pos = center;

                        while intensity > 0.0 {

                            let block_pos = check_pos.floor().as_ivec3();
                            let Some((block, _)) = self.get_block(block_pos) else {
                                break // Just abort this ray if we enter unloaded chunk.
                            };

                            // NOTE: This should properly handle the infinite resistance
                            // returned by some blocks, this will just set intensity to
                            // negative infinity and stop the loop.
                            intensity -= (block::material::get_explosion_resistance(block) + 0.3) * STEP;
                            if intensity > 0.0 {
                                
                                if set_fire 
                                && block == block::AIR 
                                && self.is_block_opaque_cube(block_pos - IVec3::Y) 
                                && rand.next_int_bounded(3) == 0 {
                                    self.set_block_notify(block_pos, block::FIRE, 0);
                                }

                                affected_pos.push((block_pos, block != block::AIR));

                            }

                            check_pos += dir;
                            intensity -= (12.0 / 16.0) * STEP;

                        }

                    }
                }
            }
        }

        // Calculate the explosion bounding box.
        let diameter = (radius * 2.0) as f64;
        let bb = BoundingBox {
            min: (center - diameter - 1.0).floor(),
            max: (center + diameter + 1.0).floor(),
        };

        let mut damaged_entities = Vec::new();

        // Calculate the amount of damage to apply to each entity in the bounding box.
        for (collided_id, Entity(collided_base, _)) in self.iter_entities_colliding(bb) {
            
            let delta = collided_base.pos - center;
            let dist = delta.length();
            let dist_norm = dist as f32 / radius; 
            
            if dist_norm <= 1.0 {
                
                let dir = delta / dist; 
                
                // The goal here is to compute how many rays starting from every point in
                // the entity bounding box we reach the explosion center. The more 
                let ray = collided_base.bb.min - center;
                let step = 1.0 / (collided_base.bb.size() * 2.0 + 1.0);
                
                // This is the offset to apply to the ray to go to different point into 
                // the bounding box, step by step.
                let mut ray_offset = DVec3::ZERO;
                let mut ray_pass = 0usize;
                let mut ray_count = 0usize;

                while ray_offset.x <= 1.0 {
                    ray_offset.y = 0.0;
                    while ray_offset.y <= 1.0 {
                        ray_offset.z = 0.0;
                        while ray_offset.z <= 1.0 {
                            ray_pass += self.ray_trace_blocks(center, ray + ray_offset, RayTraceKind::Overlay).is_none() as usize;
                            ray_count += 1;
                            ray_offset.z += step.z;
                        }
                        ray_offset.y += step.y;
                    }
                    ray_offset.x += step.x;
                }

                // The final damage depends on the distance and the number of rays.
                let damage_factor = (1.0 - dist_norm) * (ray_pass as f32 / ray_count as f32);
                let damage = (damage_factor * damage_factor + damage_factor) / 2.0 * 8.0 * radius + 1.0;
                let damage = damage as u16;
                
                damaged_entities.push((collided_id, damage, dir * damage_factor as f64));

            }

        }

        // Finally alter entities.
        for (eid, damage, accel) in damaged_entities {
            
            let Entity(base, _) = self.get_entity_mut(eid).unwrap();

            base.hurt.push(Hurt {
                damage,
                origin_id,
            });

            base.vel += accel;

        }

        // Finally drain the destroyed pos and remove blocks.
        for (pos, should_destroy) in affected_pos {
            if should_destroy {
                // We can unwrap because these position were previously checked.
                let (prev_block, prev_metadata) = self.set_block_notify(pos, block::AIR, 0).unwrap();
                self.spawn_block_loot(pos, prev_block, prev_metadata, 0.3);
            }
        }

        self.push_event(Event::Explode { center, radius });

    }

}
