//! Item use in the world.

use glam::{IVec3, DVec3, Vec3};

use crate::entity::{Arrow, Entity, Snowball, Tnt, Bobber, BaseKind, ProjectileKind, Item};
use crate::inventory::InventoryHandle;
use crate::gen::tree::TreeGenerator;
use crate::block::sapling::TreeKind;
use crate::item::{ItemStack, self};
use crate::geom::Face;
use crate::block;

use super::World;
use super::bound::RayTraceKind;


/// Methods related to item usage in the world.
impl World {

    /// Use an item stack on a given block, this is basically the action of left click. 
    /// This function returns the item stack after, if used, this may return an item stack
    /// with size of 0. The face is where the click has hit on the target block.
    pub fn use_stack(&mut self, inv: &mut InventoryHandle, index: usize, pos: IVec3, face: Face, entity_id: u32) {

        let stack = inv.get(index);
        if stack.is_empty() {
            return;
        }
        
        let success = match stack.id {
            0 => false,
            1..=255 => self.use_block_stack(stack.id as u8, stack.damage as u8, pos, face, entity_id),
            item::SUGAR_CANES => self.use_block_stack(block::SUGAR_CANES, 0, pos, face, entity_id),
            item::CAKE => self.use_block_stack(block::CAKE, 0, pos, face, entity_id),
            item::REPEATER => self.use_block_stack(block::REPEATER, 0, pos, face, entity_id),
            item::REDSTONE => self.use_block_stack(block::REDSTONE, 0, pos, face, entity_id),
            item::WOOD_DOOR => self.use_door_stack(block::WOOD_DOOR, pos, face, entity_id),
            item::IRON_DOOR => self.use_door_stack(block::IRON_DOOR, pos, face, entity_id),
            item::BED => self.use_bed_stack(pos, face, entity_id),
            item::DIAMOND_HOE |
            item::IRON_HOE |
            item::STONE_HOE |
            item::GOLD_HOE |
            item::WOOD_HOE => self.use_hoe_stack(pos, face),
            item::WHEAT_SEEDS => self.use_wheat_seeds_stack(pos, face),
            item::DYE if stack.damage == 15 => self.use_bone_meal_stack(pos),
            item::FLINT_AND_STEEL => self.use_flint_and_steel(pos, face),
            _ => false
        };

        if success {
            inv.set(index, stack.inc_damage(1));
        }

    }

    /// Use an item that is not meant to be used on blocks. Such as buckets, boats, bows or
    /// food items...
    pub fn use_raw_stack(&mut self, inv: &mut InventoryHandle, index: usize, entity_id: u32) {

        let stack = inv.get(index);
        if stack.is_empty() {
            return;
        }

        match stack.id {
            item::BUCKET |
            item::WATER_BUCKET |
            item::LAVA_BUCKET => self.use_bucket_stack(inv, index, entity_id),
            item::BOW => self.use_bow_stack(inv, index, entity_id),
            item::SNOWBALL => self.use_snowball_stack(inv, index, entity_id),
            item::FISHING_ROD => self.use_fishing_rod_stack(inv, index, entity_id),
            _ => ()
        }

    }

    /// Place a block toward the given face. This is used for single blocks, multi blocks
    /// are handled apart by other functions that do not rely on the block placing logic.
    fn use_block_stack(&mut self, id: u8, metadata: u8, mut pos: IVec3, mut face: Face, entity_id: u32) -> bool {

        let look = self.get_entity(entity_id).unwrap().0.look;

        if let Some((block::SNOW, _)) = self.get_block(pos) {
            // If a block is placed by clicking on a snow block, replace that snow block.
            face = Face::NegY;
        } else {
            // Get position of the block facing the clicked face.
            pos += face.delta();
            // The block is oriented toward that clicked face.
            face = face.opposite();
        }

        // Some block have special facing when placed.
        match id {
            block::WOOD_STAIR | block::COBBLESTONE_STAIR |
            block::REPEATER | block::REPEATER_LIT => {
                face = Face::from_yaw(look.x);
            }
            block::DISPENSER |
            block::FURNACE | block::FURNACE_LIT |
            block::PUMPKIN | block::PUMPKIN_LIT => {
                face = Face::from_yaw(look.x).opposite();
            }
            block::PISTON => {
                face = Face::from_look(look.x, look.y).opposite();
            }
            _ => {}
        }

        if pos.y >= 127 && block::material::get_material(id).is_solid() {
            return false;
        } if !self.can_place_block(pos, face, id) {
            return false;
        }

        self.place_block(pos, face, id, metadata);
        true

    }

    /// Place a door item at given position.
    fn use_door_stack(&mut self, block_id: u8, mut pos: IVec3, face: Face, entity_id: u32) -> bool {

        if face != Face::PosY {
            return false;
        } else {
            pos += IVec3::Y;
        }

        if pos.y >= 127 {
            return false;
        } else if !self.can_place_block(pos, face.opposite(), block_id) {
            return false;
        }

        // The door face the opposite of the placer's look.
        let look = self.get_entity(entity_id).unwrap().0.look;
        let mut door_face = Face::from_yaw(look.x).opposite();
        let mut flip = false;
        
        // Here we count the block on the left and right (from the door face), this will
        // change the default orientation of the door.
        let left_pos = pos + door_face.rotate_left().delta();
        let right_pos = pos + door_face.rotate_right().delta();

        // Temporary closure to avoid boiler plate just after.
        let is_door_block = |pos| {
            self.get_block(pos).map(|(id, _)| id == block_id).unwrap_or(false)
        };

        let left_door = is_door_block(left_pos) || is_door_block(left_pos + IVec3::Y);
        let right_door = is_door_block(right_pos) || is_door_block(right_pos + IVec3::Y);

        if right_door && !left_door {
            flip = true;
        } else {

            let left_count = 
                self.is_block_opaque_cube(left_pos) as u8 + 
                self.is_block_opaque_cube(left_pos + IVec3::Y) as u8;
            
            let right_count = 
                self.is_block_opaque_cube(right_pos) as u8 + 
                self.is_block_opaque_cube(right_pos + IVec3::Y) as u8;

            if left_count > right_count {
                flip = true;
            }

        }

        let mut metadata = 0;

        // To flip the door, we rotate it left and open it by default.
        if flip {
            block::door::set_open(&mut metadata, true);
            door_face = door_face.rotate_left();
        }

        block::door::set_face(&mut metadata, door_face);
        self.set_block_notify(pos, block_id, metadata);

        block::door::set_upper(&mut metadata, true);
        self.set_block_notify(pos + IVec3::Y, block_id, metadata);

        true

    }

    fn use_bed_stack(&mut self, mut pos: IVec3, face: Face, entity_id: u32) -> bool {

        if face != Face::PosY {
            return false;
        } else {
            pos += IVec3::Y;
        }

        let look = self.get_entity(entity_id).unwrap().0.look;
        let bed_face = Face::from_yaw(look.x);
        let head_pos = pos + bed_face.delta();

        if !matches!(self.get_block(pos), Some((block::AIR, _))) {
            return false;
        } else if !matches!(self.get_block(head_pos), Some((block::AIR, _))) {
            return false;
        } else if !self.is_block_opaque_cube(pos - IVec3::Y) || !self.is_block_opaque_cube(head_pos - IVec3::Y) {
            return false;
        }

        let mut metadata = 0;
        block::bed::set_face(&mut metadata, bed_face);
        self.set_block_notify(pos, block::BED, metadata);
        block::bed::set_head(&mut metadata, true);
        self.set_block_notify(head_pos, block::BED, metadata);

        true

    }

    fn use_hoe_stack(&mut self, pos: IVec3, face: Face) -> bool {
        
        if let Some((id, _)) = self.get_block(pos) {
            if let Some((above_id, _)) = self.get_block(pos + IVec3::Y) {
                if (face != Face::NegY && above_id == block::AIR && id == block::GRASS) || id == block::DIRT {
                    self.set_block_notify(pos, block::FARMLAND, 0);
                    return true;
                }
            }
        }

        false

    }

    fn use_wheat_seeds_stack(&mut self, pos: IVec3, face: Face) -> bool {

        if face == Face::PosY {
            if let Some((block::FARMLAND, _)) = self.get_block(pos) {
                if let Some((block::AIR, _)) = self.get_block(pos + IVec3::Y) {
                    self.set_block_notify(pos + IVec3::Y, block::WHEAT, 0);
                    return true;
                }
            }
        }

        false

    }

    fn use_bone_meal_stack(&mut self, pos: IVec3) -> bool {

        let Some((block, metadata)) = self.get_block(pos) else { return false };

        if block == block::SAPLING {
            
            let mut gen = match block::sapling::get_kind(metadata) {
                TreeKind::Oak if self.get_rand_mut().next_int_bounded(10) == 0 => TreeGenerator::new_big(),
                TreeKind::Oak => TreeGenerator::new_oak(),
                TreeKind::Birch => TreeGenerator::new_birch(),
                TreeKind::Spruce => TreeGenerator::new_spruce2(),
            };
            
            gen.generate_from_sapling(self, pos);
            true

        } else {
            false
        }

    }

    fn use_flint_and_steel(&mut self, pos: IVec3, face: Face) -> bool {

        if self.is_block(pos, block::TNT) {
            self.spawn_entity(Tnt::new_with(|new_base, new_tnt| {
                new_base.pos = pos.as_dvec3() + 0.5;
                new_tnt.fuse_time = 80;
            }));
            self.set_block_notify(pos, block::AIR, 0);
        } else {
            let fire_pos = pos + face.delta();
            if self.is_block_air(fire_pos) {
                self.set_block_notify(fire_pos, block::FIRE, 0);
            }
        }

        true

    }

    fn use_bucket_stack(&mut self, inv: &mut InventoryHandle, index: usize, entity_id: u32) {

        let stack = inv.get(index);
        let fluid_id = match stack.id {
            item::BUCKET => block::AIR,
            item::WATER_BUCKET => block::WATER_MOVING,
            item::LAVA_BUCKET => block::LAVA_MOVING,
            _ => unimplemented!()
        };

        let entity = self.get_entity(entity_id).unwrap();
        
        let origin = entity.0.pos + DVec3::new(0.0, 1.62, 0.0);
        
        let yaw_dx = -entity.0.look.x.sin();
        let yaw_dz = entity.0.look.x.cos();
        let pitch_dy = -entity.0.look.y.sin();
        let pitch_h = entity.0.look.y.cos();
        let ray = Vec3::new(yaw_dx * pitch_h, pitch_dy, yaw_dz * pitch_h).as_dvec3() * 5.0;

        // NOTE: We only hit fluid sources when we use an empty bucket.
        let kind = if fluid_id == block::AIR {
            RayTraceKind::OverlayWithFluid
        } else {
            RayTraceKind::Overlay
        };

        let Some(hit) = self.ray_trace_blocks(origin, ray, kind) else { 
            // We did not hit anything...
            return 
        };
        
        let mut new_stack;

        // The bucket is empty.
        if fluid_id == block::AIR {

            let Some((id, metadata)) = self.get_block(hit.pos) else { return };

            // Fluid must be a source.
            if !block::fluid::is_source(metadata) {
                return;
            }

            new_stack = match id {
                block::WATER_MOVING | block::WATER_STILL => ItemStack::new_single(item::WATER_BUCKET, 0),
                block::LAVA_MOVING | block::LAVA_STILL => ItemStack::new_single(item::LAVA_BUCKET, 0),
                _ => return
            };

            self.set_block_notify(hit.pos, block::AIR, 0);

        } else {

            let pos = hit.pos + hit.face.delta();
            let Some((id, _)) = self.get_block(pos) else { return };

            if id == block::AIR || !block::material::get_material(id).is_solid() {
                self.set_block_notify(pos, fluid_id, 0);
                // world.schedule_tick(pos, fluid_id, 5); // TODO: 30 for lava.
            }

            new_stack = ItemStack::new_single(item::BUCKET, 0);

        }

        if stack.size > 1 {
            inv.push_front(&mut new_stack);
            // Only if there was space in the inventory we actually remove previous one.
            if new_stack.is_empty() {
                inv.set(index, stack.with_size(stack.size - 1));
            }
        } else {
            inv.set(index, new_stack);
        }

    }

    fn use_bow_stack(&mut self, inv: &mut InventoryHandle, _index: usize, entity_id: u32) {
        
        // Consume an arrow from the inventory.
        if !inv.consume(ItemStack::new_single(item::ARROW, 0)) {
            return;
        }

        let Entity(base, _) = self.get_entity(entity_id).unwrap();

        let arrow = Arrow::new_with(|arrow_base, arrow_projectile, arrow| {
            
            arrow_base.pos = base.pos;
            arrow_base.pos.y += base.eye_height as f64;
            arrow_base.look = base.look;

            let (yaw_sin, yaw_cos) = arrow_base.look.x.sin_cos();
            let (pitch_sin, pitch_cos) = arrow_base.look.y.sin_cos();

            arrow_base.vel.x = (-yaw_sin * pitch_cos) as f64;
            arrow_base.vel.z = (yaw_cos * pitch_cos) as f64;
            arrow_base.vel.y = (-pitch_sin) as f64;
            
            arrow_base.vel += arrow_base.rand.next_gaussian_vec() * 0.0075;
            arrow_base.vel *= 1.5;

            arrow_projectile.owner_id = Some(entity_id);
            arrow.from_player = true;

        });

        self.spawn_entity(arrow);

    }

    fn use_snowball_stack(&mut self, inv: &mut InventoryHandle, index: usize, entity_id: u32) {

        let stack = inv.get(index);
        inv.set(index, stack.inc_damage(1));

        let Entity(base, _) = self.get_entity(entity_id).unwrap();

        let snowball = Snowball::new_with(|throw_base, throw_projectile, _| {
            
            throw_base.pos = base.pos;
            throw_base.pos.y += base.eye_height as f64 - 0.1;
            throw_base.look = base.look;

            let (yaw_sin, yaw_cos) = throw_base.look.x.sin_cos();
            let (pitch_sin, pitch_cos) = throw_base.look.y.sin_cos();

            // PARITY: Notchian implementation multiplies the initial velocity Y component
            // by 0.4 for unknown reason, to fix the aim issue we removed this here.
            throw_base.vel.x = (-yaw_sin * pitch_cos) as f64;
            throw_base.vel.z = (yaw_cos * pitch_cos) as f64;
            throw_base.vel.y = (-pitch_sin) as f64;
            
            throw_base.pos.x += throw_base.vel.x * 0.16;
            throw_base.pos.z += throw_base.vel.z * 0.16;

            throw_base.vel += throw_base.rand.next_gaussian_vec() * 0.0075;
            throw_base.vel *= 1.5;

            throw_projectile.owner_id = Some(entity_id);

        });

        self.spawn_entity(snowball);

    }

    fn use_fishing_rod_stack(&mut self, inv: &mut InventoryHandle, index: usize, entity_id: u32) {

        let Entity(base, _) = self.get_entity_mut(entity_id).unwrap();

        // Save the pos before dropping the base reference.
        let base_pos = base.pos;
        let base_look = base.look;
        let mut new_bobber_id = base.bobber_id;

        let mut item_damage = 0;

        if let Some(bobber_id) = new_bobber_id {
            
            if let Some(Entity(bobber_base, BaseKind::Projectile(bobber_projectile, ProjectileKind::Bobber(bobber)))) = self.get_entity(bobber_id) {

                let bobber_pos = bobber_base.pos;

                let bobber_delta = base_pos - bobber_pos;
                let bobber_dist = bobber_delta.length();
                let mut bobber_accel = bobber_delta * 0.1;
                bobber_accel.y += bobber_dist.sqrt() * 0.08;

                if let Some(attached_id) = bobber.attached_id {
                    if let Some(Entity(attached_base, _)) = self.get_entity_mut(attached_id) {
                        attached_base.vel += bobber_accel;
                        item_damage = 3;
                    }
                } else if bobber.catch_time > 0 {

                    self.spawn_entity(Item::new_with(|item_base, item| {
                        item_base.persistent = true;
                        item_base.pos = bobber_pos;
                        item_base.vel = bobber_accel;
                        item.stack = ItemStack::new_single(item::RAW_FISH, 0);
                    }));

                    item_damage = 1;

                } else if bobber_projectile.state.is_some() {
                    item_damage = 2;
                }

            }

            self.remove_entity(bobber_id, "bobber retracted");
            new_bobber_id = None;

        } else {

            let bobber = Bobber::new_with(|throw_base, throw_projectile, _| {
            
                throw_base.pos = base_pos;
                throw_base.pos.y += 1.62 - 0.1;
                throw_base.look = base_look;
    
                let (yaw_sin, yaw_cos) = throw_base.look.x.sin_cos();
                let (pitch_sin, pitch_cos) = throw_base.look.y.sin_cos();
    
                // PARITY: Notchian implementation multiplies the initial velocity Y component
                // by 0.4 for unknown reason, to fix the aim issue we removed this here.
                throw_base.vel.x = (-yaw_sin * pitch_cos) as f64;
                throw_base.vel.z = (yaw_cos * pitch_cos) as f64;
                throw_base.vel.y = (-pitch_sin) as f64;
                
                throw_base.pos.x += throw_base.vel.x * 0.16;
                throw_base.pos.z += throw_base.vel.z * 0.16;
    
                throw_base.vel += throw_base.rand.next_gaussian_vec() * 0.0075;
                throw_base.vel *= 1.5;
    
                throw_projectile.owner_id = Some(entity_id);
    
            });
    
            new_bobber_id = Some(self.spawn_entity(bobber));

        }

        let Entity(base, _) = self.get_entity_mut(entity_id).unwrap();
        base.bobber_id = new_bobber_id;

        let stack = inv.get(index);
        inv.set(index, stack.inc_damage(item_damage));

    }

}
