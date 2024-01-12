//! Base function for ticking entity.
//! 
//! This module gives in documentation the reference to the Java methods, as known from
//! the decompilation of Minecraft b1.7.3 by RetroMCP.
//! 
//! This module architecture is quite complicated because we want to replicate almost the
//! same logic path as the Notchian implementation, so we need to emulate method 
//! overriding and super class calls. To achieve that, we use base/living/projectile kind
//! enumerations in order to route logic, each function that is created must exists 

use glam::{DVec3, IVec3, Vec2, Vec3Swizzles};

use tracing::trace;

use crate::block::material::Material;
use crate::world::bound::RayTraceKind;
use crate::world::{World, Event, EntityEvent};
use crate::entity::Chicken;
use crate::item::ItemStack;
use crate::geom::{Face, BoundingBox};
use crate::block;

use super::{Entity,
    BaseKind, ProjectileKind, LivingKind, 
    Base, Living, Hurt, ProjectileHit};

use super::common::{self, let_expect};
use super::tick_state;
use super::tick_ai;


/// Entry point tick method for all entities.
pub(super) fn tick(world: &mut World, id: u32, entity: &mut Entity) {
    
    let Entity(base, _) = entity;

    // Just kill the entity if far in the void.
    if base.pos.y < -64.0 {
        world.remove_entity(id, "void");
        return;
    }

    let prev_pos = base.pos;
    let prev_vel = base.vel;
    let prev_look = base.look;

    // Increase the entity lifetime, used by some entities and is interesting for debug.
    base.lifetime += 1;

    match entity {
        Entity(_, BaseKind::Item(_)) => tick_item(world, id, entity),
        Entity(_, BaseKind::Painting(_)) => tick_painting(world, id, entity),
        Entity(_, BaseKind::FallingBlock(_)) => tick_falling_block(world, id, entity),
        Entity(_, BaseKind::Tnt(_)) => tick_tnt(world, id, entity),
        Entity(_, BaseKind::Living(_, _)) => tick_living(world, id, entity),
        Entity(_, BaseKind::Projectile(_, _)) => tick_projectile(world, id, entity),
        Entity(_, _) => tick_base(world, id, entity),
    }

    // Finally check all major changes and push events if needed.
    let Entity(base, _) = entity;

    if prev_pos != base.pos {
        world.push_event(Event::Entity { id, inner: EntityEvent::Position { pos: base.pos } });
    }

    if prev_vel != base.vel {
        world.push_event(Event::Entity { id, inner: EntityEvent::Velocity { vel: base.vel } });
    }

    if prev_look != base.look {
        world.push_event(Event::Entity { id, inner: EntityEvent::Look { look: base.look } });
    }

}


/// REF: Entity::onUpdate
fn tick_base(world: &mut World, id: u32, entity: &mut Entity) {
    tick_state(world, id, entity);
}

/// REF: EntityItem::onUpdate
fn tick_item(world: &mut World, id: u32, entity: &mut Entity) {

    tick_base(world, id, entity);
    let_expect!(Entity(base, BaseKind::Item(item)) = entity);

    if item.frozen_time > 0 {
        item.frozen_time -= 1;
    }

    // Update item velocity.
    base.vel.y -= 0.04;

    // If the item is in lava, apply random motion like it's burning.
    // PARITY: The real client don't use 'in_lava', check if problematic.
    if base.in_lava {
        base.vel.y = 0.2;
        base.vel.x = ((base.rand.next_float() - base.rand.next_float()) * 0.2) as f64;
        base.vel.z = ((base.rand.next_float() - base.rand.next_float()) * 0.2) as f64;
    }

    // If the item is in an opaque block.
    let block_pos = base.pos.floor().as_ivec3();
    if world.is_block_opaque_cube(block_pos) {

        let delta = base.pos - block_pos.as_dvec3();

        // Find a block face where we can bump the item.
        let bump_face = Face::ALL.into_iter()
            .filter(|face| !world.is_block_opaque_cube(block_pos + face.delta()))
            .map(|face| {
                let mut delta = delta[face.axis_index()];
                if face.is_pos() {
                    delta = 1.0 - delta;
                }
                (face, delta)
            })
            .min_by(|&(_, delta1), &(_, delta2)| delta1.total_cmp(&delta2))
            .map(|(face, _)| face);

        // If we found a non opaque face then we bump the item to that face.
        if let Some(bump_face) = bump_face {
            let accel = (base.rand.next_float() * 0.2 + 0.1) as f64;
            if bump_face.is_neg() {
                base.vel[bump_face.axis_index()] = -accel;
            } else {
                base.vel[bump_face.axis_index()] = accel;
            }
        }
        
    }

    // Move the item while checking collisions if needed.
    apply_base_vel(world, id, base, base.vel, 0.0);

    let mut slipperiness = 0.98;

    if base.on_ground {

        slipperiness = 0.1 * 0.1 * 58.8;

        let ground_pos = IVec3 {
            x: base.pos.x.floor() as i32,
            y: base.bb.min.y.floor() as i32 - 1,
            z: base.pos.z.floor() as i32,
        };

        if let Some((ground_id, _)) = world.get_block(ground_pos) {
            if ground_id != block::AIR {
                slipperiness = block::material::get_slipperiness(ground_id);
            }
        }

    }

    // Slow its velocity depending on ground slipperiness.
    base.vel.x *= slipperiness as f64;
    base.vel.y *= 0.98;
    base.vel.z *= slipperiness as f64;
    
    if base.on_ground {
        base.vel.y *= -0.5;
    }

    // Kill the item self after 5 minutes (5 * 60 * 20).
    if base.lifetime >= 6000 {
        world.remove_entity(id, "item too old");
    }

}

/// REF: EntityPainting::onUpdate
fn tick_painting(_world: &mut World, _id: u32, entity: &mut Entity) {

    // NOTE: Not calling tick_base
    let_expect!(Entity(_, BaseKind::Painting(painting)) = entity);

    painting.check_valid_time += 1;
    if painting.check_valid_time >= 100 {
        painting.check_valid_time = 0;
        // TODO: check painting validity and destroy it if not valid
    }

}

/// REF: EntityFallingSand::onUpdate
fn tick_falling_block(world: &mut World, id: u32, entity: &mut Entity) {

    // NOTE: Not calling tick_base
    let_expect!(Entity(base, BaseKind::FallingBlock(falling_block)) = entity);

    if falling_block.block_id == 0 {
        world.remove_entity(id, "falling block has not block");
        return;
    }

    base.vel.y -= 0.04;

    apply_base_vel(world, id, base, base.vel, 0.0);

    if base.on_ground {

        base.vel *= DVec3::new(0.7, -0.5, 0.7);
        world.remove_entity(id, "falling block on ground");

        let block_pos = base.pos.floor().as_ivec3();
        if world.can_place_block(block_pos, Face::PosY, falling_block.block_id) {
            world.set_block_notify(block_pos, falling_block.block_id, 0);
        } else {
            world.spawn_loot(base.pos, ItemStack::new_block(falling_block.block_id, 0), 0.0);
        }

    } else if base.lifetime > 100 {
        world.remove_entity(id, "falling block too old");
        world.spawn_loot(base.pos, ItemStack::new_block(falling_block.block_id, 0), 0.0);
    }

}

/// REF: EntityTNTPrimed::onUpdate
fn tick_tnt(world: &mut World, id: u32, entity: &mut Entity) {

    // NOTE: Not calling tick_base
    let_expect!(Entity(base, BaseKind::Tnt(tnt)) = entity);

    base.vel.y -= 0.04;
    apply_base_vel(world, id, base, base.vel, 0.0);
    base.vel.y *= 0.98;

    if base.on_ground {
        base.vel *= DVec3::new(0.7, -0.5, 0.7);
    }

    tnt.fuse_time = tnt.fuse_time.saturating_sub(1);
    if tnt.fuse_time == 0 {
        world.remove_entity(id, "tnt explode");
        world.explode(base.pos, 4.0, false, None);
    }

}

/// REF: EntityLiving::onUpdate
fn tick_living(world: &mut World, id: u32, entity: &mut Entity) {

    // Super call.
    tick_base(world, id, entity);

    tick_ai(world, id, entity);

    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);

    if living.jumping {
        if base.in_water || base.in_lava {
            base.vel.y += 0.04;
        } else if base.on_ground {
            base.vel.y += 0.42 + 0.1; // FIXME: Added 0.1 to make it work
        }
    }

    living.accel_strafing *= 0.98;
    living.accel_forward *= 0.98;
    living.yaw_velocity *= 0.9;

    tick_living_pos(world, id, base, living, living_kind);
    tick_living_push(world, id, base);
    
}

/// REF: 
/// - EntityArrow::onUpdate
/// - EntitySnowball::onUpdate
/// - EntityFireball::onUpdate
/// - EntityEgg:onUpdate
fn tick_projectile(world: &mut World, id: u32, entity: &mut Entity) {

    // Super call.
    tick_base(world, id, entity);

    let_expect!(Entity(base, BaseKind::Projectile(projectile, projectile_kind)) = entity);

    projectile.shake = projectile.shake.saturating_sub(1);
    projectile.state_time = projectile.state_time.saturating_add(1);

    // If this is a fishing rod bobber, we force its position, but we also prevent it to
    // enter the block hit state.
    if let ProjectileKind::Bobber(bobber) = projectile_kind {

        // Kill the bobber by default, keep it if the owner is still alive and still
        // declare it has the its bobber.
        let mut remove_bobber = true;

        if let Some(owner_id) = projectile.owner_id {
            if let Some(Entity(owner_base, _)) = world.get_entity(owner_id) {
                if owner_base.bobber_id == Some(id) {
                    remove_bobber = false;
                }
            }
        }

        if remove_bobber {
            world.remove_entity(id, "bobber has no owner");
            return;
        }

        if let Some(attached_id) = bobber.attached_id {
            if let Some(Entity(attached_base, _)) = world.get_entity(attached_id) {
                base.pos.x = attached_base.pos.x;
                base.pos.y = attached_base.bb.min.y + attached_base.size.height as f64 * 0.8;
                base.pos.z = attached_base.pos.z;
                projectile.state = None;
                common::update_bounding_box_from_pos(base);
                return;
            } else {
                bobber.attached_id = None;
            }
        }

    }

    if let Some(hit) = projectile.state {
        if (hit.block, hit.metadata) == world.get_block(hit.pos).unwrap() {
            if projectile.state_time == 1200 {
                world.remove_entity(id, "projectile in block for too long");
            }
        } else {
            trace!("entity #{id}, no longer in block...");
            base.vel *= (base.rand.next_float_vec() * 0.2).as_dvec3();
            projectile.state = None;
            projectile.state_time = 0;
            // PARITY: The Notchian implementation directly execute the following code
            // but only for the fishing bobber.
        }
    } else {

        // Check if we hit a block, if so we update the projectile velocity.
        let hit_block = world.ray_trace_blocks(base.pos, base.vel, RayTraceKind::Colliding);

        // If we hit a block we constrain the velocity to avoid entering the block.
        if let Some(hit_block) = &hit_block {
            base.vel = hit_block.ray;
        }

        // Only prevent collision with owner for the first 4 ticks. The fireball is the
        // only one to be 24 ticks instead.
        let owner_invincible_time = match projectile_kind {
            ProjectileKind::Fireball(_) => 25,
            _ => 5
        };
        let owner_id = projectile.owner_id.filter(|_| projectile.state_time < owner_invincible_time);
        
        // We try to find an entity that collided with the ray.
        let hit_entity = world.iter_entities_colliding_mut(base.bb.offset(base.vel).inflate(DVec3::ONE))
            // Filter out entities that we cannot collide with.
            .filter(|(target_id, Entity(_, target_base_kind))| {
                match target_base_kind {
                    BaseKind::Item(_) |
                    BaseKind::LightningBolt(_) |
                    BaseKind::Projectile(_, _) => false,
                    // Do not collide with owner...
                    _ => Some(*target_id) != owner_id,
                }
            })
            // Check if the current ray intersects with the entity bounding box,
            // inflated by 0.3, if so we return the entity and the ray length^2.
            .filter_map(|(target_id, target_entity)| {
                target_entity.0.bb
                    .inflate(DVec3::splat(0.3))
                    .calc_ray_trace(base.pos, base.vel)
                    .map(|(new_ray, _)| (target_id, target_entity, new_ray.length_squared()))
            })
            // Take the entity closer to the origin.
            .min_by(|(_, _, len1), (_, _, len2)| len1.total_cmp(len2))
            // Don't keep the ray length.
            .map(|(target_id, target_entity, _)| (target_id, target_entity));

        // The logic when hitting a block or entity depends on projectile kind.
        match projectile_kind {
            ProjectileKind::Arrow(_) => {

                if let Some((_, Entity(hit_base, _))) = hit_entity {
                    hit_base.hurt.push(Hurt { 
                        damage: 4, 
                        origin_id: projectile.owner_id,
                    });
                } else if let Some(hit_block) = hit_block {

                    projectile.state = Some(ProjectileHit {
                        pos: hit_block.pos,
                        block: hit_block.block,
                        metadata: hit_block.metadata,
                    });
    
                    projectile.shake = 7;
    
                    // This is used to prevent the client to moving the arrow on its own 
                    // above the block hit, we use the hit face to take away the arrow 
                    // from colliding with the face. This is caused by the really weird 
                    // function 'Entity::setPositionAndRotation2' from Notchian 
                    // implementation that modify the position we sent and move any entity
                    // out of the block while inflating the bounding box by 1/32 
                    // horizontally. We use 2/32 here in order to account for precision 
                    // errors.
                    //
                    // Ideally, this should be implemented server-side as it is a Notchian
                    // implementation issue rather than an issue with the ticking itself.
                    if hit_block.face == Face::PosY {
                        // No inflate need on that face.
                        base.pos.y += base.size.center as f64;
                    } else if hit_block.face == Face::NegY {
                        // For now we do not adjust for negative face because this 
                        // requires offset the entity by its whole height and it make no 
                        // sense on client side, not more sense that the current behavior.
                    } else {
                        base.pos += hit_block.face.delta().as_dvec3() * (base.size.width / 2.0 + (2.0 / 32.0)) as f64;
                    }

                }

            }
            ProjectileKind::Snowball(_) |
            ProjectileKind::Egg(_) => {

                if let Some((_, Entity(hit_base, _))) = hit_entity {
                    hit_base.hurt.push(Hurt { 
                        damage: 0, 
                        origin_id: projectile.owner_id,
                    });
                }

                if hit_entity.is_some() || hit_block.is_some() {
                    
                    world.remove_entity(id, "projectile hit");

                    // For egg we try to spawn a chicken.
                    if let ProjectileKind::Egg(_) = projectile_kind {
                        if base.rand.next_int_bounded(8) == 0 {

                            let mut count = 1usize;
                            if base.rand.next_int_bounded(32) == 0 {
                                count = 4;
                            }

                            for _ in 0..count {
                                world.spawn_entity(Chicken::new_with(|new_base, new_living, _| {
                                    new_base.persistent = true;
                                    new_base.pos = base.pos;
                                    new_base.look.x = base.rand.next_float() * std::f32::consts::TAU;
                                    new_living.health = 4;
                                }));
                            }

                        }
                    }

                }

            }
            ProjectileKind::Fireball(_) => {

                if hit_entity.is_some() || hit_block.is_some() {
                    world.remove_entity(id, "fireball hit");
                    world.explode(base.pos, 1.0, true, projectile.owner_id);
                }

            }
            ProjectileKind::Bobber(bobber) => {

                if let Some((hit_id, Entity(hit_base, _))) = hit_entity {

                    hit_base.hurt.push(Hurt { 
                        damage: 0, 
                        origin_id: projectile.owner_id,
                    });

                    bobber.attached_id = Some(hit_id);

                } else if let Some(hit_block) = hit_block {

                    projectile.state = Some(ProjectileHit {
                        pos: hit_block.pos,
                        block: hit_block.block,
                        metadata: hit_block.metadata,
                    });

                }

            }
        }

        base.pos += base.vel;
        
        base.look.x = f64::atan2(base.vel.x, base.vel.z) as f32;
        base.look.y = f64::atan2(base.vel.y, base.vel.xz().length()) as f32;
        
        // The velocity update depends on projectile kind.
        if let ProjectileKind::Fireball(fireball) = projectile_kind {
            
            if base.in_water {
                base.vel *= 0.8;
            } else {
                base.vel *= 0.95;
            }

            base.vel += fireball.accel;

        } else if let ProjectileKind::Bobber(bobber) = projectile_kind {

            // PARITY: The bobber in Notchian implementation is really weird, so I just
            // tried here to make a better logic that do not use the block collision
            // after the ray tracing, this would be redundant for not so much improvement.
            
            let mut fluid_boost = 0.0;
            for delta in 0u8..5 {

                let min_y = base.bb.min.y + base.bb.size_y() * (delta + 0) as f64 / 5.0;
                let max_y = base.bb.min.y + base.bb.size_y() * (delta + 1) as f64 / 5.0;

                let check_bb = BoundingBox {
                    min: DVec3 { 
                        x: base.bb.min.x, 
                        y: min_y, 
                        z: base.bb.min.z,
                    },
                    max: DVec3 {
                        x: base.bb.max.x,
                        y: max_y,
                        z: base.bb.max.z,
                    },
                };

                if common::has_fluids_colliding(world, check_bb, Material::Water) {
                    fluid_boost += 1.0 / 5.0;
                }

            }

            if fluid_boost > 0.0 {

                if bobber.catch_time > 0 {
                    bobber.catch_time -= 1;
                } else {

                    let chance = 500;
                    // TODO: If canLightningStrikeAt, it's 300

                    if base.rand.next_int_bounded(chance) == 0 {
                        bobber.catch_time = base.rand.next_int_bounded(30) as u16 + 10;
                        base.vel.y -= 0.2;
                    }

                }

            }

            if bobber.catch_time > 0 {
                base.vel.y -= (base.rand.next_float() * base.rand.next_float() * base.rand.next_float()) as f64 * 0.2;
            }

            let mut vel_factor = 0.92;
            // TODO: vel_factor = 0.5 if collided.

            base.vel.y += (fluid_boost * 2.0 - 1.0) * 0.04;
            if fluid_boost > 0.0 {
                vel_factor *= 0.9;
                base.vel.y *= 0.8;
            }

            base.vel *= vel_factor;

        } else {
            
            if base.in_water {
                base.vel *= 0.8;
            } else {
                base.vel *= 0.99;
            }

            base.vel.y -= 0.03;
        
        }

        // Really important!
        common::update_bounding_box_from_pos(base);

    }
    
}

/// Tick a living entity to push/being pushed an entity.
fn tick_living_push(world: &mut World, _id: u32, base: &mut Base) {

    // TODO: pushing minecart

    // For each colliding entity, precalculate the velocity to add to both entities.
    for (_, push_entity) in world.iter_entities_colliding_mut(base.bb.inflate(DVec3::new(0.2, 0.0, 0.2))) {
        
        let Entity(push_base, push_base_kind) = push_entity;

        match push_base_kind {
            BaseKind::Boat(_) |
            BaseKind::Living(_, _) |
            BaseKind::Minecart(_) => {}
            _ => continue // Other entities cannot be pushed.
        }

        let mut dx = base.pos.x - push_base.pos.x;
        let mut dz = base.pos.z - push_base.pos.z;
        let mut delta = f64::max(dx.abs(), dz.abs());
        
        if delta >= 0.01 {
            
            delta = delta.sqrt();
            dx /= delta;
            dz /= delta;

            let delta_inv = 1.0 / delta;
            dx *= delta_inv;
            dz *= delta_inv;
            dx *= 0.05;
            dz *= 0.05;

            let delta = DVec3::new(dx, 0.0, dz);
            
            push_base.vel -= delta;
            base.vel += delta;

        }

    }

}

/// REF: EntityLiving::moveEntityWithHeading
fn tick_living_pos(world: &mut World, id: u32, base: &mut Base, living: &mut Living, living_kind: &mut LivingKind) {

    // Squid has no special rule for moving.
    if let LivingKind::Squid(squid) = living_kind {

        // PARITY: The squid moving loving logic is handled in EntitySquid::onLivingEntity
        // but we move it here in this implementation for simplicity of the squid impl.

        // Initial random value for animation speed.
        if squid.animation_speed == 0.0 {
            squid.animation_speed = 1.0 / (base.rand.next_float() + 1.0) * 0.2;
        }

        // If the squid animation reaches a full circle, reset it and pick a new rando
        squid.animation += squid.animation_speed;
        if squid.animation > std::f32::consts::TAU {
            squid.animation -= std::f32::consts::TAU;
            if base.rand.next_int_bounded(10) == 0 {
                squid.animation_speed = 1.0 / (base.rand.next_float() + 1.0) * 0.2;
            }
        }

        if base.in_water {

            // PARITY: We use our 'living.accel_forward' as the squid acceleration.
            if squid.animation < std::f32::consts::PI {
                let progress = squid.animation / std::f32::consts::PI;
                if progress > 0.75 {
                    living.accel_forward = 1.0;
                }
            } else {
                living.accel_forward *= 0.9;
            }

            // PARITY: As defined in 'tick_squid_ai', we use the squid look instead of
            // the Notchian impl acceleration vector, so we need to multiply the vector.
            let (yaw_sin, yaw_cos) = base.look.x.sin_cos();
            let pitch_sin = base.look.y.sin();
            base.vel.x = (yaw_cos * 0.2 * living.accel_forward) as f64;
            base.vel.z = (yaw_sin * 0.2 * living.accel_forward) as f64;
            base.vel.y = (pitch_sin * 0.1 * living.accel_forward) as f64;

        } else {
            base.vel.x = 0.0;
            base.vel.z = 0.0;
            base.vel.y -= 0.08;
            base.vel.y *= 0.98;
        }

        apply_base_vel(world, id, base, base.vel, 0.5);
        return;

    }

    // All living entities have step height 0.5;
    let step_height = 0.5;

    // REF: EntityFlying::moveEntityWithHeading
    let flying = matches!(living_kind, LivingKind::Ghast(_));

    if base.in_water {
        apply_living_accel(base, living, 0.02);
        apply_base_vel(world, id, base, base.vel, step_height);
        base.vel *= 0.8;
        if !flying {
            base.vel.y -= 0.02;
        }
        // TODO: If collided horizontally
    } else if base.in_lava {
        apply_living_accel(base, living, 0.02);
        apply_base_vel(world, id, base, base.vel, step_height);
        base.vel *= 0.5;
        if !flying {
            base.vel.y -= 0.02;
        }
        // TODO: If collided horizontally
    } else {

        let mut slipperiness = 0.91;

        if base.on_ground {
            slipperiness = 546.0 * 0.1 * 0.1 * 0.1;
            let ground_pos = base.pos.as_ivec3();
            if let Some((ground_id, _)) = world.get_block(ground_pos) {
                if ground_id != 0 {
                    slipperiness = block::material::get_slipperiness(ground_id) * 0.91;
                }
            }
        }

        // Change entity velocity if on ground or not.
        let vel_factor = match base.on_ground {
            true => 0.16277136 / (slipperiness * slipperiness * slipperiness) * 0.1,
            false => 0.02,
        };

        apply_living_accel(base, living, vel_factor);
        
        // TODO: Is on ladder

        apply_base_vel(world, id, base, base.vel, step_height);

        // TODO: Collided horizontally and on ladder

        if flying {
            base.vel *= slipperiness as f64;
        } else {
            base.vel.y -= 0.08;
            base.vel.y *= 0.98;
            base.vel.x *= slipperiness as f64;
            base.vel.z *= slipperiness as f64;
        }

    }

}

/// Update a living entity velocity according to its strafing/forward accel.
pub fn apply_living_accel(base: &mut Base, living: &mut Living, factor: f32) {

    let mut strafing = living.accel_strafing;
    let mut forward = living.accel_forward;
    let mut dist = Vec2::new(forward, strafing).length();
    if dist >= 0.01 {
        dist = dist.max(1.0);
        dist = factor / dist;
        strafing *= dist;
        forward *= dist;
        let (yaw_sin, yaw_cos) = base.look.x.sin_cos();
        base.vel.x += (strafing * yaw_cos - forward * yaw_sin) as f64;
        base.vel.z += (forward * yaw_cos + strafing * yaw_sin) as f64;
    }
    
}

/// Common method for moving an entity by a given amount while checking collisions.
/// 
/// REF: Entity::moveEntity
pub fn apply_base_vel(world: &mut World, _id: u32, base: &mut Base, delta: DVec3, step_height: f32) {

    if base.no_clip {
        base.bb += delta;
        base.on_ground = false;
    } else {

        // TODO: 

        // TODO: If in cobweb:
        // delta *= DVec3::new(0.25, 0.05, 0.25)
        // base.vel = DVec3::ZERO

        // TODO: Sneaking on ground

        let colliding_bb = base.bb.expand(delta);

        // Compute a new delta that doesn't collide with above boxes.
        let mut new_delta = delta;
        
        // Use a temporarily owned thread local for colliding boxes.
        common::BOUNDING_BOX.with_borrow_mut(|colliding_bbs| {

            debug_assert!(colliding_bbs.is_empty());

            colliding_bbs.extend(world.iter_blocks_boxes_colliding(colliding_bb));
            colliding_bbs.extend(world.iter_entities_colliding(colliding_bb)
                .filter_map(|(_entity_id, entity)| {
                    // Only the boat entity acts like a hard bounding box.
                    if let Entity(base, BaseKind::Boat(_)) = entity {
                        Some(base.bb)
                    } else {
                        None
                    }
                }));

            // Check collision on Y axis.
            for colliding_bb in &*colliding_bbs {
                new_delta.y = colliding_bb.calc_y_delta(base.bb, new_delta.y);
            }
    
            base.bb += DVec3::new(0.0, new_delta.y, 0.0);
    
            // Check collision on X axis.
            for colliding_bb in &*colliding_bbs {
                new_delta.x = colliding_bb.calc_x_delta(base.bb, new_delta.x);
            }
    
            base.bb += DVec3::new(new_delta.x, 0.0, 0.0);
    
            // Check collision on Z axis.
            for colliding_bb in &*colliding_bbs {
                new_delta.z = colliding_bb.calc_z_delta(base.bb, new_delta.z);
            }
            
            base.bb += DVec3::new(0.0, 0.0, new_delta.z);

            // Finally clear the cache.
            colliding_bbs.clear();
                
        });

        let collided_x = delta.x != new_delta.x;
        let collided_y = delta.y != new_delta.y;
        let collided_z = delta.z != new_delta.z;
        let on_ground = collided_y && delta.y < 0.0; // || self.on_ground

        // Apply step if relevant.
        if step_height > 0.0 && on_ground && (collided_x || collided_z) {
            // TODO: todo!("handle step motion");
        }

        base.on_ground = on_ground;

        if on_ground {
            if base.fall_distance > 0.0 {
                // TODO: Damage?
            }
            base.fall_distance = 0.0;
        } else if new_delta.y < 0.0 {
            base.fall_distance -= new_delta.y as f32;
        }

        if collided_x {
            base.vel.x = 0.0;
        }

        if collided_y {
            base.vel.y = 0.0;
        }

        if collided_z {
            base.vel.z = 0.0;
        }

    }

    common::update_pos_from_bounding_box(base);

}
