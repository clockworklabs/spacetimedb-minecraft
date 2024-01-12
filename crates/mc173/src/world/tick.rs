//! Block ticking functions.

use glam::{IVec3, DVec3};

use tracing::warn;

use crate::entity::{Item, FallingBlock};
use crate::block::material::Material;
use crate::block_entity::BlockEntity;
use crate::block::sapling::TreeKind;
use crate::gen::tree::TreeGenerator;
use crate::geom::{Face, FaceSet};
use crate::{block, item};

use super::{World, Dimension, Event, BlockEntityEvent, BlockEntityStorage, Weather};


/// Methods related to block scheduled ticking and random ticking.
impl World {

    /// Tick a block in the world. The random boolean indicates if it's a random tick.
    /// This function is unchecked because the caller should ensure that the given id
    /// and metadata is coherent with the given position.
    pub(super) fn tick_block_unchecked(&mut self, pos: IVec3, id: u8, metadata: u8, random: bool) {
        match id {
            // PARITY: Notchian client has random tick on button?
            block::BUTTON if !random => self.tick_button(pos, metadata),
            block::REPEATER if !random => self.tick_repeater(pos, metadata, false),
            block::REPEATER_LIT if !random => self.tick_repeater(pos, metadata, true),
            // PARITY: Notchian client have random tick on redstone torch?
            block::REDSTONE_TORCH if !random => self.tick_redstone_torch(pos, metadata, false),
            block::REDSTONE_TORCH_LIT if !random => self.tick_redstone_torch(pos, metadata, true),
            block::DISPENSER if !random => self.tick_dispenser(pos, metadata),
            block::WATER_MOVING => self.tick_fluid_moving(pos, block::WATER_MOVING, metadata),
            block::LAVA_MOVING => self.tick_fluid_moving(pos, block::LAVA_MOVING, metadata),
            // NOTE: Sugar canes and cactus have the same logic, we just give the block.
            block::SUGAR_CANES |
            block::CACTUS => self.tick_cactus_or_sugar_canes(pos, id, metadata),
            block::CAKE => {}, // Seems unused in MC
            block::WHEAT => self.tick_wheat(pos, metadata),
            block::DETECTOR_RAIL => {},
            block::FARMLAND => {},
            block::FIRE => self.tick_fire(pos, metadata),
            // PARITY: Notchian client check if flowers can stay, we intentionally don't
            // respect that to allow glitched plants to stay.
            block::DANDELION |
            block::POPPY |
            block::DEAD_BUSH |
            block::TALL_GRASS => {},
            // Mushrooms ticking
            block::RED_MUSHROOM |
            block::BROWN_MUSHROOM => self.tick_mushroom(pos, id),
            block::SAPLING => self.tick_sapling(pos, metadata),
            block::SAND |
            block::GRAVEL if !random => self.tick_falling_block(pos, id),
            block::GRASS => {}, // Spread
            block::ICE => {}, // Melt
            block::LEAVES => {}, // Decay
            block::WOOD_PRESSURE_PLATE |
            block::STONE_PRESSURE_PLATE => {}, // Weird, why random tick for redstone?
            block::PUMPKIN |
            block::PUMPKIN_LIT => {}, // Seems unused
            block::REDSTONE_ORE_LIT => self.tick_redstone_ore_lit(pos),
            block::SNOW => {}, // Melt
            block::SNOW_BLOCK => {}, // Melt (didn't know wtf?)
            block::LAVA_STILL => {}, // Specific to lava still
            block::TORCH => {}, // Seems not relevant..
            _ => {}
        }
    }

    /// Tick a button block, this is used to deactivate the button after 20 ticks.
    fn tick_button(&mut self, pos: IVec3, mut metadata: u8) {
        if block::button::is_active(metadata) {
            block::button::set_active(&mut metadata, false);
            self.set_block_notify(pos, block::BUTTON, metadata);
        }
    }

    fn tick_repeater(&mut self, pos: IVec3, metadata: u8, lit: bool) {

        let face = block::repeater::get_face(metadata);
        let delay = block::repeater::get_delay_ticks(metadata);
        let back_powered = self.has_passive_power_from(pos - face.delta(), face);

        if lit && !back_powered {
            self.set_block_notify(pos, block::REPEATER, metadata);
        } else if !lit {
            if !back_powered {
                self.schedule_block_tick(pos, block::REPEATER_LIT, delay);
            }
            self.set_block_notify(pos, block::REPEATER_LIT, metadata);
        }

    }

    fn tick_redstone_torch(&mut self, pos: IVec3, metadata: u8, lit: bool) {

        // TODO: Check torch burnout...

        let Some(torch_face) = block::torch::get_face(metadata) else { return };
        let powered = self.has_passive_power_from(pos + torch_face.delta(), torch_face.opposite());

        if lit {
            if powered {
                self.set_block_notify(pos, block::REDSTONE_TORCH, metadata);
            }
        } else {
            if !powered {
                self.set_block_notify(pos, block::REDSTONE_TORCH_LIT, metadata);
            }
        }

    }

    fn tick_dispenser(&mut self, pos: IVec3, metadata: u8) {

        let Some(face) = block::dispenser::get_face(metadata) else { return };

        // TODO: Also check for power above? (likely quasi connectivity?)

        if !self.has_passive_power(pos) {
            return;
        }

        let Some(BlockEntity::Dispenser(dispenser)) = self.get_block_entity_mut(pos) else { return };

        if let Some(index) = dispenser.pick_random_index() {

            let mut stack = dispenser.inv[index];
            let dispense_stack = stack.with_size(1);
            stack.size -= 1;
            stack = stack.to_non_empty().unwrap_or_default();
            dispenser.inv[index] = stack;

            self.push_event(Event::BlockEntity { 
                pos, 
                inner: BlockEntityEvent::Storage { 
                    storage: BlockEntityStorage::Standard(index as u8),
                    stack,
                },
            });

            let origin_pos = pos.as_dvec3() + face.delta().as_dvec3() * 0.6 + 0.5;

            if dispense_stack.id == item::ARROW {
                warn!("TODO: shot arrow from dispenser");
            } else if dispense_stack.id == item::EGG {
                warn!("TODO: shot egg from dispenser");
            } else if dispense_stack.id == item::SNOWBALL {
                warn!("TODO: shot snowball from dispenser");
            } else {

                let entity = Item::new_with(|base, item| {
                    
                    base.persistent = true;
                    base.pos = origin_pos - DVec3::Y * 0.3;
                    
                    let rand_vel = self.rand.next_double() * 0.1 + 0.2;
                    base.vel = face.delta().as_dvec3() * rand_vel;
                    base.vel += self.rand.next_gaussian_vec() * 0.0075 * 6.0;

                    item.stack = dispense_stack;

                });

                self.spawn_entity(entity);

                // TODO: Play effect 1000 (click with pitch 1.0)

            }

        } else {
            // TODO: Play effect 1001 (click with pitch 1.2) in world.
        }

    }

    /// Tick a cactus.
    fn tick_cactus_or_sugar_canes(&mut self, pos: IVec3, id: u8, metadata: u8) {

        // If the block above is air, count how many cactus block are below.
        if self.is_block_air(pos + IVec3::Y) {
            
            for dy in 1.. {
                if !self.is_block(pos - IVec3::new(0, dy, 0), id) {
                    break;
                } else if dy == 2 {
                    // Two cactus blocks below, should not grow more.
                    return;
                }
            }

            if metadata == 15 {
                self.set_block_notify(pos + IVec3::Y, id, 0);
                self.set_block_notify(pos, id, 0);
            } else {
                self.set_block_notify(pos, id, metadata + 1);
            }

        }

    }

    /// Tick a wheat crop, grow it if possible.
    fn tick_wheat(&mut self, pos: IVec3, metadata: u8) {

        // Do not tick if light level is too low or already fully grown.
        if self.get_light(pos).max_real() < 9 || metadata >= 7 {
            return;
        }

        // Growth rate.
        let mut rate = 1.0;
        
        // Check each block below and add to the rate depending on its type.
        for x in pos.x - 1..=pos.x + 1 {
            for z in pos.z - 1..=pos.z + 1 {

                let below_pos = IVec3::new(x, pos.y - 1, z);
                if let Some((below_id, below_metadata)) = self.get_block(below_pos) {
                    
                    let mut below_rate = match (below_id, below_metadata) {
                        (block::FARMLAND, 0) => 1.0,
                        (block::FARMLAND, _) => 3.0,
                        _ => continue,
                    };

                    if x != pos.x || z != pos.z {
                        below_rate /= 4.0;
                    }
                    
                    rate += below_rate;

                }

            }
        }
        
        // Calculate the growth rate, it depends on surrounding wheat crops.
        let mut same_faces = FaceSet::new();
        let mut same_corner = false;

        for face in Face::HORIZONTAL {
            let face_pos = pos + face.delta();
            if matches!(self.get_block(face_pos), Some((block::WHEAT, _))) {
                same_faces.insert(face);
            }
            let corner_pos = face_pos + face.rotate_right().delta();
            if matches!(self.get_block(corner_pos), Some((block::WHEAT, _))) {
                // Same corner is enough to divide the growth rate, so we break here.
                same_corner = true;
                break;
            }
        }
        
        if same_corner || (same_faces.contains_x() && same_faces.contains_z()) {
            rate /= 2.0;
        }

        // Randomly grow depending on the calculated rate.
        if self.rand.next_int_bounded((100.0 / rate) as i32) == 0 {
            self.set_block_notify(pos, block::WHEAT, metadata + 1);
        }

    }

    /// Tick a fire and try spreading it.
    fn tick_fire(&mut self, pos: IVec3, metadata: u8) {

        // Infini burn if nether rack.
        let (below_block, _) = self.get_block(pos - IVec3::Y).unwrap_or_default();

        let below_netherrack = below_block == block::NETHERRACK;
        let below_opaque = block::material::is_opaque_cube(below_block);  // PARITY: Notchian impl use "normal cube"
        
        // Test if the fire can stay...
        let can_stay = 
            /* below_opaque || */ 
            self.weather == Weather::Clear ||
            false;  // TODO: Other can rain tests
            
        if can_stay {

            if metadata < 15 {
                let new_metadata = (metadata + self.rand.next_int_bounded(3) as u8 / 2).min(15);
                self.set_block(pos, block::FIRE, new_metadata);
            }

            self.schedule_block_tick(pos, block::FIRE, 40);

            // Check if any block around can catch fire.
            let catch_fire = Face::ALL.into_iter()
                .filter(|face| {
                    let (block, _) = self.get_block(pos + face.delta()).unwrap_or_default();
                    block::material::get_fire_flammability(block) > 0
                })
                .collect::<FaceSet>();

            if !below_netherrack && !catch_fire.is_empty() {
                if !below_opaque || metadata > 3 {
                    self.set_block_notify(pos, block::AIR, 0);
                }
            } else if !below_netherrack 
                   && !catch_fire.contains(Face::NegY) 
                   && metadata == 15 
                   && self.rand.next_int_bounded(4) == 0 {
                self.set_block_notify(pos, block::AIR, 0);
            } else {

                for face in Face::ALL {

                    let face_pos = pos + face.delta();
                    let (block, _) = self.get_block(face_pos).unwrap_or_default();
                    let burn = block::material::get_fire_burn(block);

                    let bound = if face.is_y() { 250 } else { 300 };
                    if self.rand.next_int_bounded(bound) < burn as i32 {
                        if self.rand.next_int_bounded(metadata as i32 + 10) < 5 && !false /* can block be rained on */ {
                            let new_metadata = (metadata + self.rand.next_int_bounded(5) as u8 / 4).min(15);
                            self.set_block_notify(face_pos, block::FIRE, new_metadata);
                        } else {
                            self.set_block_notify(face_pos, block::AIR, 0);
                        }
                    }

                }

                // Now try to spread the fire further.
                for bx in pos.x - 1..=pos.x + 1 {
                    for bz in pos.z - 1..=pos.z + 1 {
                        for by in pos.y - 1..=pos.y + 4 {
                            let check_pos = IVec3::new(bx, by, bz);
                            if check_pos != pos {

                                let mut bound = 100;
                                if check_pos.y > pos.y + 1 {
                                    bound += (check_pos.y - (pos.y + 1)) * 100;
                                }

                                // Here we get the maximum flammability around...
                                let flammability = Face::ALL.into_iter()
                                    .map(|face| self.get_block(check_pos + face.delta()).unwrap_or_default())
                                    .map(|(block, _)| block::material::get_fire_flammability(block))
                                    .max()
                                    .unwrap_or(0);

                                if flammability != 0 {
                                    let catch = (flammability as i32 + 40) / (metadata as i32 + 30);
                                    if catch > 0 
                                    && self.rand.next_int_bounded(bound) <= catch
                                    && self.weather == Weather::Clear {

                                        let new_metadata = (metadata + self.rand.next_int_bounded(5) as u8 / 4).min(15);
                                        self.set_block_notify(check_pos, block::FIRE, new_metadata);
                                        
                                    }
                                }

                            }
                        }
                    }
                }

            }

        } else {
            self.set_block_notify(pos, block::AIR, 0);
        }

    }

    /// Tick a mushroom to try spreading it.
    fn tick_mushroom(&mut self, pos: IVec3, id: u8) {
        if self.rand.next_int_bounded(100) == 0 {

            let spread_pos = pos + IVec3 {
                x: self.rand.next_int_bounded(3) - 1,
                y: self.rand.next_int_bounded(2) - self.rand.next_int_bounded(2),
                z: self.rand.next_int_bounded(3) - 1,
            };

            if self.get_light(spread_pos).max() < 13 {
                if self.is_block_air(spread_pos) {
                    if self.is_block_opaque_cube(spread_pos - IVec3::Y) {
                        self.set_block_notify(spread_pos, id, 0);
                    }
                }
            }

        }
    }

    /// Tick a sapling to grow it.
    fn tick_sapling(&mut self, pos: IVec3, mut metadata: u8) {
        if self.get_light(pos + IVec3::Y).max_real() >= 9 && self.rand.next_int_bounded(30) == 0 {
            if block::sapling::is_growing(metadata) {
                
                let mut gen = match block::sapling::get_kind(metadata) {
                    TreeKind::Oak if self.rand.next_int_bounded(10) == 0 => TreeGenerator::new_big(),
                    TreeKind::Oak => TreeGenerator::new_oak(),
                    TreeKind::Birch => TreeGenerator::new_birch(),
                    TreeKind::Spruce => TreeGenerator::new_spruce2(),
                };

                gen.generate_from_sapling(self, pos);

            } else {
                block::sapling::set_growing(&mut metadata, true);
                self.set_block_notify(pos, block::SAPLING, metadata);
            }
        }
    }

    fn tick_falling_block(&mut self, pos: IVec3, id: u8) {
        let (below_block, _) = self.get_block(pos - IVec3::Y).unwrap_or_default();
        if below_block == 0 || below_block == block::FIRE || block::material::is_fluid(below_block) {

            self.spawn_entity(FallingBlock::new_with(|base, falling_block| {
                base.persistent = true;
                base.pos = pos.as_dvec3() + 0.5;
                falling_block.block_id = id;
            }));

            self.set_block_notify(pos, block::AIR, 0);
            
        }
    }

    fn tick_redstone_ore_lit(&mut self, pos: IVec3) {
        self.set_block_notify(pos, block::REDSTONE_ORE, 0);
    }

    /// Tick a moving fluid block.
    fn tick_fluid_moving(&mut self, pos: IVec3, flowing_id: u8, mut metadata: u8) {

        // +1 to get still fluid id.
        let still_id = flowing_id + 1;
        let material = block::material::get_material(flowing_id);

        // Default distance to decrement on each block unit.
        let dist_drop = match flowing_id {
            block::LAVA_MOVING if self.get_dimension() != Dimension::Nether => 2,
            _ => 1,
        };

        // The id below is used many time after, so we query it here.
        let below_pos = pos - IVec3::Y;
        let (below_id, below_metadata) = self.get_block(below_pos).unwrap_or_default();

        // Update this fluid state.
        if !block::fluid::is_source(metadata) {

            // Default to 8, so if no fluid block is found around, fluid will disappear.
            let mut shortest_dist = 8;
            let mut sources_around = 0u8;

            for face in [Face::NegX, Face::PosX, Face::NegZ, Face::PosZ] {
                if let Some((face_id, face_metadata)) = self.get_block(pos + face.delta()) {
                    // Only if this block is of the same type.
                    // +1 to get the "still" id.
                    if face_id == flowing_id || face_id == still_id {
                        let face_dist = block::fluid::get_actual_distance(face_metadata);
                        shortest_dist = shortest_dist.min(face_dist);
                        if block::fluid::is_source(face_metadata) {
                            sources_around += 1;
                        }
                    }
                }
            }

            let mut new_metadata = shortest_dist + dist_drop;
            if new_metadata > 7 {
                // Just mark that the metadata is invalid, fluid should disappear.
                new_metadata = 0xFF;
            }

            // If the top block on top is the same fluid, this become a falling state fluid.
            if let Some((above_id, above_metadata)) = self.get_block(pos + IVec3::Y) {
                if above_id == flowing_id || above_id == still_id {
                    // Copy the above metadata but force falling state.
                    new_metadata = above_metadata;
                    block::fluid::set_falling(&mut new_metadata, true);
                }
            }

            // Infinite water sources!
            if sources_around >= 2 && flowing_id == block::WATER_MOVING {
                if block::material::get_material(below_id).is_solid() {
                    block::fluid::set_source(&mut new_metadata);
                } else if below_id == flowing_id || below_id == still_id {
                    if block::fluid::is_source(below_metadata) {
                        block::fluid::set_source(&mut new_metadata);
                    }
                }
            }

            // TODO: Weird lava stuff.

            if new_metadata != metadata {
                metadata = new_metadata;
                if new_metadata == 0xFF {
                    self.set_block_notify(pos, block::AIR, 0);
                } else {
                    self.set_block_notify(pos, flowing_id, new_metadata);
                }
            } else {
                // Metadata is the same, set still.
                self.set_block(pos, still_id, metadata);
            }

        } else {
            // Moving source is systematically set to still source.
            self.set_block(pos, still_id, metadata);
        }

        // The block has been removed, don't propagate it.
        if metadata == 0xFF {
            return;
        }

        // Check if we can flow below.
        let blocked_below = block::material::is_fluid_proof(below_id);

        if !block::material::is_fluid(below_id) && !blocked_below {
            // The block below is not a fluid block and do not block fluids, the fluid 
            // below is set to a falling version of the current block.
            block::fluid::set_falling(&mut metadata, true);
            self.set_block_notify(below_pos, flowing_id, metadata);
        } else if block::fluid::is_source(metadata) || blocked_below {

            // The block is a source or is blocked below, we spread it horizontally.
            let flow_faces = self.calc_fluid_flow_faces(pos, material);

            // FIXME: Dist drop is always 1 if source block
            let new_dist = block::fluid::get_actual_distance(metadata) + dist_drop;
            if new_dist > 7 {
                return;
            }

            for face in Face::HORIZONTAL {
                if flow_faces.contains(face) {
                    let face_pos = pos + face.delta();
                    if let Some((face_id, _)) = self.get_block(face_pos) {
                        if !block::material::is_fluid(face_id) && !block::material::is_fluid_proof(face_id) {
                            // TODO: Break only for water.
                            self.break_block(face_pos);
                            self.set_block_notify(face_pos, flowing_id, new_dist);
                        }
                    }
                }
            }

        }

    }

    fn calc_fluid_flow_faces(&mut self, pos: IVec3, material: Material) -> FaceSet {

        let mut lowest_cost = u8::MAX;
        let mut set = FaceSet::new();

        for face in Face::HORIZONTAL {

            let face_pos = pos + face.delta();
            let (face_block, face_metadata) = self.get_block(face_pos).unwrap_or_default();

            if !block::material::is_fluid_proof(face_block) {
                if block::material::get_material(face_block) != material || !block::fluid::is_source(face_metadata) {

                    let face_below_pos = face_pos - IVec3::Y;
                    let (face_below_block, _) = self.get_block(face_below_pos).unwrap_or_default();
                    
                    let face_cost;
                    if !block::material::is_fluid_proof(face_below_block) {
                        face_cost = 0;
                    } else {
                        face_cost = self.calc_fluid_flow_cost(face_pos, material, face, 1);
                    }

                    // If this face has the lowest cost, that means that all previous face
                    // are no longer of the lowest cost so we clear.
                    if face_cost < lowest_cost {
                        set.clear();
                        lowest_cost = face_cost;
                    }

                    // If our face has the lowest cost, we insert it. 
                    if face_cost == lowest_cost {
                        set.insert(face);
                    }

                }
            }

        }

        set

    }

    /// Internal function to calculate the flow cost of a fluid toward the given face. If
    /// the face is not given, all faces are checked, and the recursive calls have the 
    /// face set to all four horizontal faces.
    fn calc_fluid_flow_cost(&mut self, pos: IVec3, material: Material, origin_face: Face, cost: u8) -> u8 {
        
        let mut lowest_cost = u8::MAX;

        for face in Face::HORIZONTAL {
            // Do not check the face from where the check come.
            if face != origin_face.opposite() {

                let face_pos = pos + face.delta();
                let (face_block, face_metadata) = self.get_block(face_pos).unwrap_or_default();

                if !block::material::is_fluid_proof(face_block) {
                    if block::material::get_material(face_block) != material || !block::fluid::is_source(face_metadata) {

                        let face_below_pos = face_pos - IVec3::Y;
                        let (face_below_block, _) = self.get_block(face_below_pos).unwrap_or_default();
                        if !block::material::is_fluid_proof(face_below_block) {
                            return cost;
                        }

                        if cost < 4 {
                            lowest_cost = lowest_cost.min(self.calc_fluid_flow_cost(face_pos, material, origin_face, cost + 1));
                        }

                    }
                }

            }
        }

        lowest_cost

    }

}
