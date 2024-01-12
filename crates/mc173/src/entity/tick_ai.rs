//! Tick AI of the entity.

use std::ops::Add;

use glam::{Vec2, DVec3, IVec3};
use tracing::trace;

use crate::entity::{Fireball, Path, LookTarget};
use crate::world::{World, Event, EntityEvent};
use crate::path::PathFinder;

use super::{Entity, BaseKind, LivingKind, EntityCategory};
use super::common::{self, let_expect};
use super::tick_attack;


/// Tick entity "artificial intelligence", like attacking players.
pub(super) fn tick_ai(world: &mut World, id: u32, entity: &mut Entity) {
    match entity {
        Entity(_, BaseKind::Living(_, LivingKind::Human(_))) => (),
        Entity(_, BaseKind::Living(_, LivingKind::Ghast(_))) => tick_ghast_ai(world, id, entity),
        Entity(_, BaseKind::Living(_, LivingKind::Squid(_))) => tick_squid_ai(world, id, entity),
        Entity(_, BaseKind::Living(_, LivingKind::Slime(_))) => tick_slime_ai(world, id, entity),
        Entity(_, BaseKind::Living(_, _)) => tick_ground_ai(world, id, entity),
        _ => unreachable!("invalid argument for this function")
    }
}

/// This is the fallback for all ground entities to just look in random directions.
/// 
/// REF: EntityLiving::updatePlayerActionState
fn tick_living_ai(world: &mut World, _id: u32, entity: &mut Entity) {

    /// Multiplier for random yaw velocity: 20 deg
    const YAW_VELOCITY_MUL: f32 = 0.3490658503988659;
    /// Maximum distance for looking at a target.
    const LOOK_AT_MAX_DIST: f64 = 8.0;
    /// Default look step when looking at a target.
    const LOOK_STEP: Vec2 = Vec2::new(0.17453292519943295, 0.6981317007977318);
    /// Slow look step used for sitting dogs.
    const SLOW_LOOK_STEP: Vec2 = Vec2::new(0.17453292519943295, 0.3490658503988659);

    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);

    living.accel_strafing = 0.0;
    living.accel_forward = 0.0;

    if base.rand.next_float() < 0.02 {
        if let Some((target_entity_id, _, _)) = common::find_closest_player_entity(world, base.pos, LOOK_AT_MAX_DIST) {
            living.look_target = Some(LookTarget {
                entity_id: target_entity_id,
                remaining_time: base.rand.next_int_bounded(20) as u32 + 10,
            });
        } else {
            living.yaw_velocity = (base.rand.next_float() - 0.5) * YAW_VELOCITY_MUL;
        }
    }

    // If the entity should have a target, just look at it if possible, and stop if
    // the target should end or is too far away.
    if let Some(target) = &mut living.look_target {

        target.remaining_time = target.remaining_time.saturating_sub(1);
        let mut target_release = target.remaining_time == 0;

        if let Some(Entity(target_base, _)) = world.get_entity(target.entity_id) {
            
            let mut look_step = LOOK_STEP;
            if let LivingKind::Wolf(wolf) = living_kind {
                if wolf.sitting {
                    look_step = SLOW_LOOK_STEP;
                }
            }

            common::update_look_at_entity_by_step(base, target_base, look_step);
            
            if target_base.pos.distance_squared(base.pos) > LOOK_AT_MAX_DIST.powi(2) {
                target_release = false;
            }

        } else {
            // Entity is dead.
            target_release = false;
        }

        if target_release {
            living.look_target = None;
        }

    } else {

        if base.rand.next_float() < 0.05 {
            living.yaw_velocity = (base.rand.next_float() - 0.5) * YAW_VELOCITY_MUL;
        }

        base.look.x += living.yaw_velocity;
        base.look.y = 0.0;

    }

    if base.in_water || base.in_lava {
        living.jumping = base.rand.next_float() < 0.8;
    }

}

/// Tick an ground creature (animal/mob) entity AI.
/// 
/// REF: EntityCreature::updatePlayerActionState
fn tick_ground_ai(world: &mut World, id: u32, entity: &mut Entity) {

    /// Maximum distance for the path finder.
    const PATH_FINDER_MAX_DIST: f32 = 16.0;
    /// Look step when looking at an attacked entity: 30/30 deg
    const LOOK_STEP: Vec2 = Vec2::new(0.5235987755982988, 0.5235987755982988);

    /// Internal structure that defines the target for the path finder.
    struct Target {
        /// Target position.
        pos: DVec3,
        /// True if the path should overwrite the current entity path, even when it
        /// was not found, therefore removing the previous one. 
        overwrite: bool,
    }

    if tick_natural_despawn(world, id, entity) {
        return;
    }

    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);
    
    // Target position to path find to.
    let mut target_pos = None;
    // Set to true when the entity should strafe while following its path.
    let mut should_strafe = false;

    // Start by finding an attack target, or attack the existing one.
    if let Some(target_id) = living.attack_target {

        if let Some(Entity(target_base, BaseKind::Living(_, _))) = world.get_entity(target_id) {

            let dist_squared = base.pos.distance_squared(target_base.pos);
            let eye_track = common::can_eye_track(world, base, target_base);

            target_pos = Some(Target { 
                pos: target_base.pos, 
                overwrite: true,
            });
            
            tick_attack(world, id, entity, target_id, dist_squared, eye_track, &mut should_strafe);

        } else {
            // Entity has been release by the attack function.
            trace!("entity #{id}, attack target released");
            living.attack_target = None;
        }

    } else  {
        
        // Depending on the entity, we search an attack target or not...
        let search_around = match living_kind {
            LivingKind::Creeper(_) => true,
            LivingKind::Giant(_) => true,
            LivingKind::Skeleton(_) => true,
            LivingKind::Zombie(_) => true,
            LivingKind::PigZombie(pig_zombie) => pig_zombie.anger,
            LivingKind::Wolf(wolf) => wolf.angry,
            LivingKind::Spider(_) => common::get_entity_light(world, base).brightness() < 0.5,
            _ => false,
        };

        if search_around {
            if let Some((target_id, Entity(target_base, _), _)) = common::find_closest_player_entity(world, base.pos, 16.0) {
                trace!("entity #{id}, attack target found: #{target_id}");
                living.attack_target = Some(target_id);
                target_pos = Some(Target { 
                    pos: target_base.pos, 
                    overwrite: true,
                });
            }
        }

    }

    // Here we need to rematch the whole entity because we passed it to `tick_attack`
    // and we are no longer guaranteed of its type.
    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);

    // If the entity has not attacked its target entity and is path finder toward it, 
    // there is 95% chance too go into the then branch.
    if should_strafe || living.attack_target.is_none() || (living.path.is_some() && base.rand.next_int_bounded(20) != 0) {
        // If the entity has not attacked and if the path is not none, there is 1.25% 
        // chance to recompute the path, if the path is none there is 2.484375% chance.
        if !should_strafe && ((living.path.is_none() && base.rand.next_int_bounded(80) == 0) || base.rand.next_int_bounded(80) == 0) {

            // The path weight function depends on the entity type.
            let weight_func = common::path_weight_func(living_kind);
            let best_pos = (0..10)
                .map(|_| {
                    IVec3 {
                        x: base.pos.x.add((base.rand.next_int_bounded(13) - 6) as f64).floor() as i32,
                        y: base.pos.y.add((base.rand.next_int_bounded(7) - 3) as f64).floor() as i32,
                        z: base.pos.z.add((base.rand.next_int_bounded(13) - 6) as f64).floor() as i32,
                    }
                })
                .map(|pos| (pos, weight_func(world, pos)))
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .unwrap().0;

            target_pos = Some(Target { 
                pos: best_pos.as_dvec3() + 0.5,
                overwrite: false,  // If the path is not found, continue current one.
            });
            
        }
    }

    // At the end, we can have an entity or a block to target.
    if let Some(target) = target_pos {

        // trace!("entity #{id}, path finding: {}", target.pos);

        let path = PathFinder::new(world)
            .find_path_from_bounding_box(base.bb, target.pos, PATH_FINDER_MAX_DIST)
            .map(Path::from);

        if target.overwrite || path.is_some() {
            living.path = path;
        }

    }

    // Now that we no longer need the world, we can borrow the target entity, if any.
    // Note that we expect this entity to exists because the attack method called
    // above should return None if the entity has been removed.
    let attack_target = living.attack_target
        .map(|id| world.get_entity(id).unwrap());

    if let Some(path) = &mut living.path {

        if base.rand.next_int_bounded(100) != 0 {

            let bb_size = base.bb.size();
            let double_width = bb_size.x * 2.0;

            let mut next_pos = None;
            
            while let Some(pos) = path.point() {

                let mut pos = pos.as_dvec3();
                pos.x += (bb_size.x + 1.0) * 0.5;
                pos.z += (bb_size.z + 1.0) * 0.5;

                // Advance the path to the next point only if distance to current one is 
                // too short. We only check the horizontal distance, because Y delta is 0.
                let pos_dist_sq = pos.distance_squared(DVec3::new(base.pos.x, pos.y, base.pos.z));
                if pos_dist_sq < double_width * double_width {
                    path.advance();
                } else {
                    next_pos = Some(pos);
                    break;
                }

            }

            living.jumping = false;

            if let Some(next_pos) = next_pos {

                let dx = next_pos.x - base.pos.x;
                let dy = next_pos.y - base.bb.min.y.add(0.5).floor();
                let dz = next_pos.z - base.pos.z;

                let move_speed = match living_kind {
                    LivingKind::Giant(_) |
                    LivingKind::Zombie(_) |
                    LivingKind::PigZombie(_) => 0.5,
                    LivingKind::Spider(_) => 0.8,
                    _ => 0.5,
                };

                living.accel_forward = move_speed;
                base.look.x = f64::atan2(dz, dx) as f32 - std::f32::consts::FRAC_PI_2;

                // Make some weird strafing if we just attacked the player.
                if should_strafe {
                    if let Some(Entity(target_base, _)) = attack_target {
                        let dx = target_base.pos.x - base.pos.x;
                        let dz = target_base.pos.z - base.pos.z;
                        base.look.x = f64::atan2(dz, dx) as f32 - std::f32::consts::FRAC_PI_2;
                        living.accel_strafing = -base.look.x.sin() * living.accel_forward;
                        living.accel_forward = base.look.x.cos() * living.accel_forward;
                    }
                }

                if dy > 0.0 {
                    living.jumping = true;
                }

            } else {
                // trace!("entity #{id}, path finished");
                living.path = None;
            }

            // Look at the player we are attacking.
            if let Some(Entity(target_base, _)) = attack_target {
                common::update_look_at_entity_by_step(base, target_base, LOOK_STEP);
            }

            // TODO: If collided horizontal and no path, then jump

            if base.rand.next_float() < 0.8 && (base.in_water || base.in_lava) {
                living.jumping = true;
            }

            return;  // Do not fallback to living AI

        } else {
            // trace!("entity #{id}, forget path because 1% chance")
        }

    }

    // If we can't run a path finding AI, fallback to the default immobile AI.
    living.path = None;
    tick_living_ai(world, id, entity);

}

/// Tick a slime entity AI.
/// 
/// REF: EntitySlime::updatePlayerActionState
fn tick_slime_ai(world: &mut World, id: u32, entity: &mut Entity) {

    /// Look step for slime: 10/20 deg
    const LOOK_STEP: Vec2 = Vec2::new(0.17453292519943295, 0.3490658503988659);
    
    if tick_natural_despawn(world, id, entity) {
        return;
    }

    let_expect!(Entity(base, BaseKind::Living(living, LivingKind::Slime(slime))) = entity);

    // Searching the closest player entities behind 16.0 blocks.
    let closest_player = common::find_closest_player_entity(world, base.pos, 16.0);
    if let Some((_, Entity(closest_base, _), _)) = closest_player {
        common::update_look_at_entity_by_step(base, closest_base, LOOK_STEP);
    }

    let mut set_jumping = false;
    if base.on_ground {
        slime.jump_remaining_time = slime.jump_remaining_time.saturating_sub(1);
        if slime.jump_remaining_time == 0 {
            set_jumping = true;
        }
    }

    if set_jumping {
        
        slime.jump_remaining_time = base.rand.next_int_bounded(20) as u32 + 10;

        if closest_player.is_some() {
            slime.jump_remaining_time /= 3;
        }

        living.jumping = true;
        living.accel_strafing = 1.0 - base.rand.next_float() * 2.0;
        living.accel_forward = slime.size as f32 + 1.0;

    } else {
        living.jumping = false;
        if base.on_ground {
            living.accel_strafing = 0.0;
            living.accel_forward = 0.0;
        }
    }

}

/// Tick a ghast entity AI.
/// 
/// REF: EntityGhast::updatePlayerActionState
fn tick_ghast_ai(world: &mut World, id: u32, entity: &mut Entity) {

    // Maximum distance to shoot a player, beyond this the ghast just follow its vel.
    const SHOT_MAX_DIST_SQUARED: f64 = 64.0 * 64.0;

    if tick_natural_despawn(world, id, entity) {
        return;
    }

    let_expect!(Entity(base, BaseKind::Living(living, LivingKind::Ghast(ghast))) = entity);

    // If we are too close or too far, change the waypoint.
    let dist = (ghast.waypoint - base.pos).length();
    if dist < 1.0 || dist > 60.0 {
        ghast.waypoint = base.pos + ((base.rand.next_float_vec() * 2.0 - 1.0) * 16.0).as_dvec3();
    }

    // Check if the ghast can reach the waypoint...
    ghast.waypoint_check_time = ghast.waypoint_check_time.saturating_sub(1);
    if ghast.waypoint_check_time == 0 {

        ghast.waypoint_check_time = base.rand.next_int_bounded(5) as u8 + 2;

        let delta = ghast.waypoint - base.pos;
        let dist = delta.length();
        let delta_norm = delta / dist;

        if delta_norm.is_finite() {
            
            // If the norm is finite then we check that we'll not collide.
            let mut traversable = true;
            let mut bb = base.bb;
            for _ in 1..dist.ceil() as usize {
                bb += delta_norm;
                if world.iter_blocks_boxes_colliding(bb).next().is_some() {
                    traversable = false;
                    break;
                }
            }

            // If traversable we accelerate toward the waypoint. If not we reset.
            if traversable {
                base.vel += delta_norm * 0.1;
            } else {
                ghast.waypoint = base.pos;
            }

        }

    }

    // Try to get the target entity if still alive.
    let mut target_entity = living.attack_target
        .and_then(|target_id| world.get_entity(target_id));

    // If we have a target entity, decrement countdown.
    if target_entity.is_some() {
        ghast.attack_target_time = ghast.attack_target_time.saturating_sub(1);
    }

    // Only then we search for the closest player if required.
    if target_entity.is_none() || ghast.attack_target_time == 0 {
        if let Some((closest_id, closest_entity, _)) = common::find_closest_player_entity(world, base.pos, 100.0) {
            living.attack_target = Some(closest_id);
            target_entity = Some(closest_entity);
            ghast.attack_target_time = 20;
        } else {
            living.attack_target = None;
            target_entity = None;
        }
    }

    // These two booleans are used to choose or not to look toward velocity and to
    // cool down attach timer.
    let mut look_vel = true;
    let mut next_attack_time = living.attack_time.saturating_sub(1);

    if let Some(Entity(target_base, _)) = target_entity {
        if target_base.pos.distance_squared(base.pos) < SHOT_MAX_DIST_SQUARED {
            
            look_vel = false;

            // PARITY: Notchian implementation use an equivalent form but not using
            // the bounding box in itself to compute the center.
            let center = base.bb.center();
            let delta = target_base.bb.center() - center;
            base.look.x = -f64::atan2(delta.x, delta.z) as f32;

            // Charge the attack only if we see the player.
            if common::can_eye_track(world, base, target_base) {
                // PARITY: Notchian implementation doesn't use the living's attack 
                // time but we use it here, so this is slightly different logic from
                // the original impl, which use negative numbers.
                next_attack_time = living.attack_time.saturating_add(1);
                if living.attack_time == 60 {
                    
                    next_attack_time = 0;

                    let fireball = Fireball::new_with(|throw_base, throw_projectile, throw_fireball| {

                        let dir = delta + throw_base.rand.next_gaussian_vec() * 0.4;
                        let dir = dir.normalize_or_zero();
    
                        throw_base.pos = center + dir * DVec3::new(4.0, 0.0, 4.0);
                        throw_base.look = base.look;
                        throw_fireball.accel = dir * 0.1;
                        throw_projectile.owner_id = Some(id);
            
                    });
    
                    world.spawn_entity(fireball);
                    
                }
            }

        }
    }

    if look_vel {
        base.look.x = -f64::atan2(base.vel.x, base.vel.z) as f32;
    }

    // Send the proper event.
    let was_charged = living.attack_time > 50;
    let charged = next_attack_time > 50;
    if was_charged != charged {
        world.push_event(Event::Entity { 
            id, 
            inner: EntityEvent::Metadata
        });
    }

    living.attack_time = next_attack_time;

}

/// Tick a squid entity AI.
/// 
/// REF: EntitySquid::updatePlayerActionState
fn tick_squid_ai(world: &mut World, id: u32, entity: &mut Entity) {

    if tick_natural_despawn(world, id, entity) {
        return;
    }

    let_expect!(Entity(base, BaseKind::Living(_living, LivingKind::Squid(_squid))) = entity);

    if base.rand.next_int_bounded(50) == 0 || !base.in_water || false /* not yet accelerated */ {
        
        // PARITY: The Notchian implementation uses other variables to control the 
        // acceleration, but here we try to reuse the existing properties. We just pick a 
        // random look, and we know that the acceleration is always 0.2 in the direction.

        base.look.x = base.rand.next_float() * std::f32::consts::TAU;
        base.look.y = base.rand.next_float() * 0.46365 * 2.0 - 0.46365;

    }

}

/// Internal function to handle the entity despawning range of entities, which is 128 
/// blocks away from the closest player. This functions return true if the entity is
/// has been removed for being too far or too old.
fn tick_natural_despawn(world: &mut World, id: u32, entity: &mut Entity) -> bool {

    // Only living entities can naturally despawned.
    let Entity(base, BaseKind::Living(living, living_kind)) = entity else {
        return false;
    };

    // Can't despawn persistent entities.
    if living.artificial {
        return false;
    }

    // We don't despawn natural wolf that are tamed.
    if let LivingKind::Wolf(wolf) = living_kind {
        if wolf.owner.is_some() {
            return false;
        }
    }

    // Increment the interaction time, mobs that are in high brightness locations have
    // faster increment.
    living.wander_time = living.wander_time.saturating_add(1);
    if living_kind.entity_kind().category() == EntityCategory::Mob {
        if common::get_entity_light(world, base).brightness() > 0.5 {
            living.wander_time = living.wander_time.saturating_add(2);
        }
    }

    // We only despawn if there are player in the server, but the entity is not in range.
    if world.get_entity_player_count() == 0 {
        return false;
    }

    if let Some((_, _, dist)) = common::find_closest_player_entity(world, base.pos, 128.0) {
        if dist < 32.0 {
            living.wander_time = 0;
            false
        } else if living.wander_time > 600 && base.rand.next_int_bounded(800) == 0 {
            // The entity has not interacted with player in long time, randomly despawn.
            world.remove_entity(id, "random wandering despawn");
            true
        } else {
            false
        }
    } else {
        // No player in 128 range, despawn this natural entity entity.
        world.remove_entity(id, "no close player despawn");
        true
    }

}
