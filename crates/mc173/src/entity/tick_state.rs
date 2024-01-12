//! Tick state of the entity.

use std::ops::Add;

use glam::DVec3;

use crate::entity::{Hurt, LivingKind, ProjectileKind};
use crate::world::{World, Event, EntityEvent};
use crate::block::material::Material;
use crate::item::{self, ItemStack};
use crate::block;

use super::{Entity, BaseKind, Base, Living};
use super::common::{self, let_expect};


/// Tick base method that is common to every entity kind, this is split in Notchian impl
/// so we split it here.
pub(super) fn tick_state(world: &mut World, id: u32, entity: &mut Entity) {
    match entity {
        Entity(_, BaseKind::Living(_, _)) => tick_state_living(world, id, entity),
        Entity(_, _) => tick_state_base(world, id, entity),
    }
}

/// REF: Entity::onEntityUpdate
fn tick_state_base(world: &mut World, id: u32, entity: &mut Entity) {
        
    let Entity(base, base_kind) = entity;

    // Compute the bounding box used for water collision, it depends on the entity kind.
    let water_bb = match base_kind {
        BaseKind::Item(_) => base.bb,
        _ => base.bb.inflate(DVec3::new(-0.001, -0.4 - 0.001, -0.001)),
    };

    // Search for water block in the water bb.
    base.in_water = false;
    let mut water_vel = DVec3::ZERO;
    for (pos, block, metadata) in world.iter_blocks_in_box(water_bb) {
        let material = block::material::get_material(block);
        if material == Material::Water {
            let height = block::fluid::get_actual_height(metadata);
            if water_bb.max.y.add(1.0).floor() >= pos.y as f64 + height as f64 {
                base.in_water = true;
                water_vel += common::calc_fluid_vel(world, pos, material, metadata);
            }
        }
    }

    // Finalize normalisation and apply if not zero.
    let water_vel = water_vel.normalize_or_zero();
    if water_vel != DVec3::ZERO {
        base.vel += water_vel * 0.014;
    }

    // Extinguish and cancel fall if in water.
    if base.in_water {
        base.fire_time = 0;
        base.fall_distance = 0.0;
    } else if matches!(base_kind, BaseKind::Living(_, LivingKind::Ghast(_) | LivingKind::PigZombie(_))) {
        base.fire_time = 0;
    }

    if base.fire_time > 0 {
        if base.fire_time % 20 == 0 {
            base.hurt.push(Hurt { damage: 1, origin_id: None });
        }
        base.fire_time -= 1;
    }

    // Check if there is a lava block colliding...
    let lava_bb = base.bb.inflate(DVec3::new(-0.1, -0.4, -0.1));
    base.in_lava = world.iter_blocks_in_box(lava_bb)
        .any(|(_, block, _)| block::material::get_material(block) == Material::Lava);

    // If this entity can pickup other ones, trigger an event.
    if base.can_pickup {

        // Temporarily owned vector to avoid allocation.
        common::ENTITY_ID.with_borrow_mut(|picked_up_entities| {

            debug_assert!(picked_up_entities.is_empty());
            
            for (entity_id, entity) in world.iter_entities_colliding(base.bb.inflate(DVec3::new(1.0, 0.0, 1.0))) {

                match &entity.1 {
                    BaseKind::Item(item) => {
                        if item.frozen_time == 0 {
                            picked_up_entities.push(entity_id);
                        }
                    }
                    BaseKind::Projectile(projectile, ProjectileKind::Arrow(arrow)) => {
                        if projectile.state.is_some() && arrow.from_player {
                            picked_up_entities.push(entity_id);
                        }
                    }
                    _ => {}
                }
            }

            for entity_id in picked_up_entities.drain(..) {
                world.push_event(Event::Entity { 
                    id, 
                    inner: EntityEvent::Pickup { 
                        target_id: entity_id,
                    },
                });
            }

        });

    }

}

/// REF: EntityLiving::onEntityUpdate
fn tick_state_living(world: &mut World, id: u32, entity: &mut Entity) {

    // Super call.
    tick_state_base(world, id, entity);

    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);
    
    // Suffocate entities if inside opaque cubes (except for sleeping players).
    let mut check_suffocate = true;
    if let LivingKind::Human(human) = living_kind {
        check_suffocate = !human.sleeping;
    }

    if check_suffocate {
        for i in 0u8..8 {
            
            let delta = DVec3 {
                x: (((i >> 0) & 1) as f64 - 0.5) * base.size.width as f64 * 0.9,
                y: (((i >> 1) & 1) as f64 - 0.5) * 0.1 + base.eye_height as f64,
                z: (((i >> 2) & 1) as f64 - 0.5) * base.size.width as f64 * 0.9,
            };

            if world.is_block_opaque_cube(base.pos.add(delta).floor().as_ivec3()) {
                // One damage per tick (not overwriting if already set to higher).
                base.hurt.push(Hurt {
                    damage: 1,
                    origin_id: None,
                });
                break;
            }

        }
    }

    // TODO: Air time underwater


    // If the zombie/skeleton see the sky light, set it on fire.
    if matches!(living_kind, LivingKind::Zombie(_) | LivingKind::Skeleton(_)) {
        let block_pos = base.pos.floor().as_ivec3();
        let height = world.get_height(block_pos).unwrap_or(0) as i32;
        if block_pos.y >= height {
            let light = common::get_entity_light(world, base);
            if light.sky_real >= 12 {
                if base.rand.next_float() * 30.0 < (light.brightness() - 0.4) * 2.0 {
                    base.fire_time = 300;
                }
            }
        }
    }

    // Lava damage and fire time.
    if base.in_lava {
        base.hurt.push(Hurt { damage: 4, origin_id: None });
        base.fire_time = 600;
    }

    // Decrease countdowns.
    living.hurt_time = living.hurt_time.saturating_sub(1);

    /// The hurt time when hit for the first time.
    /// PARITY: The Notchian impl doesn't actually use hurt time but another variable
    ///  that have the exact same behavior, so we use hurt time here to be more
    ///  consistent. We also avoid the divide by two thing that is useless.
    const HURT_INITIAL_TIME: u16 = 10;

    // We keep the entity that killed it.
    let mut killer_id = None;

    while let Some(hurt) = base.hurt.pop() {

        // Don't go further if entity is already dead.
        if living.health == 0 {
            break;
        }

        // Reset the interaction time of the entity when it get hurt.
        living.wander_time = 0;

        // Calculate the actual damage dealt on this tick depending on cooldown.
        let mut actual_damage = 0;
        if living.hurt_time == 0 {
            
            living.hurt_time = HURT_INITIAL_TIME;
            living.hurt_last_damage = hurt.damage;
            actual_damage = hurt.damage;
            world.push_event(Event::Entity { id, inner: EntityEvent::Damage });

            if let Some(origin_id) = hurt.origin_id {
                if let Some(Entity(origin_base, _)) = world.get_entity(origin_id) {
                    let mut dir = origin_base.pos - base.pos;
                    dir.y = 0.0; // We ignore verticale delta.
                    while dir.length_squared() < 1.0e-4 {
                        dir = DVec3 {
                            x: (base.rand.next_double() - base.rand.next_double()) * 0.01,
                            y: 0.0,
                            z: (base.rand.next_double() - base.rand.next_double()) * 0.01,
                        }
                    }
                    common::update_knock_back(base, dir);
                }
            }

        } else if hurt.damage > living.hurt_last_damage {
            actual_damage = hurt.damage - living.hurt_last_damage;
            living.hurt_last_damage = hurt.damage;
        }

        // Apply damage.
        if actual_damage != 0 {
            
            living.health = living.health.saturating_sub(actual_damage);
            
            // The entity have been killed.
            if living.health == 0 {
                killer_id = hurt.origin_id;
            }

            // TODO: For players, take armor into account.

        }

    }

    if living.health == 0 {

        // If this is the first death tick, push event and drop loots.
        if living.death_time == 0 {
            
            world.push_event(Event::Entity { id, inner: EntityEvent::Dead });
            spawn_living_loot(world, base, living, living_kind);

            // If we know the killer id and we are a creeper, check if this the killer
            // is a skeleton, in which case we drop a music disk.
            if let LivingKind::Creeper(_) = living_kind {
                if let Some(killer_id) = killer_id {
                    
                    if let Some(Entity(_, BaseKind::Living(_, LivingKind::Skeleton(_)))) = world.get_entity(killer_id) {
                        let item = base.rand.next_choice(&[item::RECORD_13, item::RECORD_CAT]);
                        let stack = ItemStack::new_single(item, 0);
                        world.spawn_loot(base.pos, stack, 0.0);
                    }

                }
            }

        }

        living.death_time += 1;
        if living.death_time > 20 {
            world.remove_entity(id, "health dead");
        }

    }
    
}


fn spawn_living_loot(world: &mut World, base: &mut Base, _living: &mut Living, living_kind: &mut LivingKind) {
    
    let stack = match living_kind {
        LivingKind::Chicken(_) => 
            ItemStack::new_single(item::FEATHER, 0),
        LivingKind::Cow(_) => 
            ItemStack::new_single(item::LEATHER, 0),
        LivingKind::Creeper(_) => 
            ItemStack::new_single(item::GUNPOWDER, 0),
        LivingKind::Ghast(_) => 
            ItemStack::new_single(item::GUNPOWDER, 0),
        LivingKind::Pig(_) => {
            if base.fire_time == 0 {
                ItemStack::new_single(item::RAW_PORKCHOP, 0)
            } else {
                ItemStack::new_single(item::COOKED_PORKCHOP, 0)
            }
        }
        LivingKind::PigZombie(_) => 
            ItemStack::new_single(item::COOKED_PORKCHOP, 0),
        LivingKind::Sheep(sheep) if !sheep.sheared => 
            ItemStack::new_block(block::WOOL, sheep.color),
        LivingKind::Skeleton(_) => {
            spawn_many_loot(world, base.pos, ItemStack::new_single(item::ARROW, 0), base.rand.next_int_bounded(3) as usize);
            spawn_many_loot(world, base.pos, ItemStack::new_single(item::BONE, 0), base.rand.next_int_bounded(3) as usize);
            return;
        }
        LivingKind::Slime(slime) if slime.size == 0 => 
            ItemStack::new_single(item::SLIMEBALL, 0),
        LivingKind::Spider(_) => 
            ItemStack::new_single(item::STRING, 0),
        LivingKind::Squid(_) => 
            ItemStack::new_single(item::DYE, 0),
        LivingKind::Zombie(_) => 
            ItemStack::new_single(item::FEATHER, 0),
        _ => return
    };

    let count = match living_kind {
        LivingKind::Squid(_) => 1 + base.rand.next_int_bounded(3) as usize,
        _ => base.rand.next_int_bounded(3) as usize,
    };

    spawn_many_loot(world, base.pos, stack, count);

}


fn spawn_many_loot(world: &mut World, pos: DVec3, stack: ItemStack, count: usize) {
    for _ in 0..count {
        world.spawn_loot(pos, stack, 0.0);
    }
}
