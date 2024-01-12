//! Tick entity attack from AI.

use glam::{Vec3Swizzles, DVec3};

use crate::entity::{Hurt, Arrow};
use crate::world::{World, Event, EntityEvent};

use super::{Entity, BaseKind, LivingKind};
use super::common::{self, let_expect};


/// Tick an attack from the entity to its targeted entity. The targeted entity id is given
/// as argument and the entity is guaranteed to be present in the world as living entity.
/// 
/// REF: EntityCreature::attackEntity
pub(super) fn tick_attack(world: &mut World, id: u32, entity: &mut Entity, target_id: u32, dist_squared: f64, eye_track: bool, should_strafe: &mut bool) {
    match entity {
        Entity(_, BaseKind::Living(_, LivingKind::Spider(_))) => tick_spider_attack(world, id, entity, target_id, dist_squared, eye_track, should_strafe),
        Entity(_, BaseKind::Living(_, LivingKind::Creeper(_))) => tick_creeper_attack(world, id, entity, target_id, dist_squared, eye_track, should_strafe),
        Entity(_, BaseKind::Living(_, LivingKind::Skeleton(_))) => tick_skeleton_attack(world, id, entity, target_id, dist_squared, eye_track, should_strafe),
        Entity(_, BaseKind::Living(_, _)) => tick_mob_attack(world, id, entity, target_id, dist_squared, eye_track, should_strafe),
        _ => unreachable!("expected a living entity for this function")
    }
}

/// REF: EntityMob::attackEntity
fn tick_mob_attack(world: &mut World, id: u32, entity: &mut Entity, target_id: u32, dist_squared: f64, eye_track: bool, _should_strafe: &mut bool) {

    /// Maximum distance for the mob to attack.
    const MAX_DIST_SQUARED: f64 = 2.0 * 2.0;

    let_expect!(Entity(base, BaseKind::Living(living, living_kind)) = entity);

    living.attack_time = living.attack_time.saturating_sub(1);
    if eye_track && living.attack_time == 0 && dist_squared < MAX_DIST_SQUARED {

        let Some(Entity(target_base, BaseKind::Living(_, _))) = world.get_entity_mut(target_id) else {
            panic!("target entity should exists");
        };

        if base.bb.intersects_y(target_base.bb) {
        
            let attack_damage = match living_kind {
                LivingKind::Giant(_) => 50,
                LivingKind::PigZombie(_) => 5,
                LivingKind::Zombie(_) => 5,
                _ => 2,
            };

            living.attack_time = 20;

            target_base.hurt.push(Hurt {
                damage: attack_damage,
                origin_id: Some(id),
            });

        }

    }

}

/// REF: EntitySpider::attackEntity
fn tick_spider_attack(world: &mut World, id: u32, entity: &mut Entity, target_id: u32, dist_squared: f64, eye_track: bool, should_strafe: &mut bool) {

    /// Minimum distance from a player to trigger a climb of the spider.
    const MIN_DIST_SQUARED: f64 = 2.0 * 2.0;
    /// Maximum distance from a player to trigger a climb of the spider.
    const MAX_DIST_SQUARED: f64 = 6.0 * 6.0;

    let_expect!(Entity(base, BaseKind::Living(living, LivingKind::Spider(_))) = entity);
    
    // If the brightness has changed, there if 1% chance to loose target.
    if common::get_entity_light(world, base).brightness() > 0.5 && base.rand.next_int_bounded(100) == 0 {
        // Loose target because it's too bright.
        living.attack_target = None;
    } else if dist_squared > MIN_DIST_SQUARED && dist_squared < MAX_DIST_SQUARED && base.rand.next_int_bounded(10) == 0 {
        // If the target is in certain range, there is 10% chance of climbing.
        if base.on_ground {

            // Unwrap should be safe because target id should exists at this point.
            let Entity(target_base, _) = world.get_entity(target_id).unwrap();

            let delta = target_base.pos.xz() - base.pos.xz();
            let h_dist = delta.length();
            let h_vel = delta / h_dist * 0.5 * 0.8 + base.vel.xz() * 0.2;
            base.vel = DVec3::new(h_vel.x, 0.4, h_vel.y);

        }
    } else {
        // Fallthrough to direct attack logic...
        tick_mob_attack(world, id, entity, target_id, dist_squared, eye_track, should_strafe)
    }

}

/// REF: EntityCreeper::attackEntity
fn tick_creeper_attack(world: &mut World, id: u32, entity: &mut Entity, _target_id: u32, dist_squared: f64, eye_track: bool, _should_strafe: &mut bool) {

    /// Minimum distance from a player to trigger a climb of the spider.
    const IDLE_MAX_DIST_SQUARED: f64 = 3.0 * 3.0;
    /// Maximum distance from a player to trigger a climb of the spider.
    const IGNITED_MAX_DIST_SQUARED: f64 = 7.0 * 7.0;

    let_expect!(Entity(base, BaseKind::Living(_, LivingKind::Creeper(creeper))) = entity);

    // Check if the creeper should be ignited depending on its current state.
    let ignited = 
        eye_track &&
        (creeper.ignited_time.is_none() && dist_squared < IDLE_MAX_DIST_SQUARED) || 
        (creeper.ignited_time.is_some() && dist_squared < IGNITED_MAX_DIST_SQUARED);

    if ignited {

        if creeper.ignited_time.is_none() {
            world.push_event(Event::Entity { id, inner: EntityEvent::Metadata });
        }

        let ignited_time = creeper.ignited_time.unwrap_or(0) + 1;
        creeper.ignited_time = Some(ignited_time);

        if ignited_time >= 30 {

            // Kill the creeper and return none in order to loose focus on the entity.
            world.remove_entity(id, "creeper explode");
            
            if creeper.powered {
                world.explode(base.pos, 6.0, false, Some(id));
            } else {
                world.explode(base.pos, 3.0, false, Some(id));
            }

        }

    } else {

        if creeper.ignited_time.is_some() {
            world.push_event(Event::Entity { id, inner: EntityEvent::Metadata });
            creeper.ignited_time = None;
        }

    }

}

/// REF: EntitySkeleton::attackEntity
fn tick_skeleton_attack(world: &mut World, id: u32, entity: &mut Entity, target_id: u32, dist_squared: f64, eye_track: bool, should_strafe: &mut bool) {
        
    const MAX_DIST_SQUARED: f64 = 10.0 * 10.0;
    
    if eye_track && dist_squared < MAX_DIST_SQUARED {

        let_expect!(Entity(base, BaseKind::Living(living, LivingKind::Skeleton(_))) = entity);
        let Entity(target_base, _) = world.get_entity(target_id).unwrap();

        living.attack_time = living.attack_time.saturating_sub(1);
        if living.attack_time == 0 {

            living.attack_time = 30;

            let eye_pos = common::calc_eye_pos(base);
            let target_eye_pos = common::calc_eye_pos(target_base);

            let arrow = Arrow::new_with(|arrow_base, arrow_projectile, arrow| {

                let mut dir = target_eye_pos - eye_pos;
                dir.y += dir.xz().length() * 0.2;
                let dir = dir.normalize_or_zero();

                arrow_base.pos = eye_pos + dir * DVec3::new(1.0, 0.0, 1.0);
                arrow_base.look = base.look;

                arrow_base.vel = dir;
                arrow_base.vel += arrow_base.rand.next_gaussian_vec() * 0.0075 * 12.0;
                arrow_base.vel *= 0.6;
    
                arrow_projectile.owner_id = Some(id);
                arrow.from_player = false;
    
            });

            world.spawn_entity(arrow);

        }

        // TODO: Look toward target
        *should_strafe = true;

    }

}
