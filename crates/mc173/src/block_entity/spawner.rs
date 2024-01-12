//! Spawner block entity.

use glam::{IVec3, DVec3};

use tracing::trace;

use crate::entity::{EntityKind, Entity};
use crate::geom::BoundingBox;
use crate::world::World;


#[derive(Debug, Clone)]
pub struct SpawnerBlockEntity {
    /// Remaining ticks to spawn the entity.
    pub remaining_time: u16,
    /// Kind of entity.
    pub entity_kind: EntityKind,
}

impl Default for SpawnerBlockEntity {

    #[inline]
    fn default() -> Self {
        Self { 
            remaining_time: 20,
            entity_kind: EntityKind::Pig,
        }
    }
    
}

impl SpawnerBlockEntity {

    /// Tick the furnace block entity.
    pub fn tick(&mut self, world: &mut World, pos: IVec3) {

        /// Maximum distance for a player to load the spawner.
        const LOAD_DIST_SQUARED: f64 = 16.0 * 16.0;

        let center = pos.as_dvec3() + 0.5;
        let loaded = world.iter_entities()
            .filter(|(_, entity)| entity.kind() == EntityKind::Human)
            .any(|(_, Entity(base, _))| base.pos.distance_squared(center) < LOAD_DIST_SQUARED);

        if !loaded {
            return;
        }

        if self.remaining_time > 0 {
            self.remaining_time -= 1;
            return;
        }
        
        self.remaining_time = 200 + world.get_rand_mut().next_int_bounded(600) as u16;
        trace!("spawner {pos}, reached spawn time, next time in: {}", self.remaining_time);

        // Count the number of entities of the spawner type in its box.
        let bb = BoundingBox::CUBE + pos.as_dvec3();
        let mut same_count = world.iter_entities_colliding(bb.inflate(DVec3::new(8.0, 4.0, 8.0)))
            .filter(|(_, entity)| entity.kind() == self.entity_kind)
            .count();

        trace!("spawner {pos}, same entity count: {same_count}");

        for _ in 0..4 {

            // If more than 5 entities of the same type exists, abort.
            if same_count > 5 {
                break;
            }

            let rand = world.get_rand_mut();
            let pos = pos.as_dvec3() + DVec3 {
                x: (rand.next_double() - rand.next_double()) * 4.0,
                y: (rand.next_int_bounded(3) - 1) as f64,
                z: (rand.next_double() - rand.next_double()) * 4.0,
            };

            let mut entity = self.entity_kind.new_default(pos);
            entity.0.look.x = rand.next_float();

            if entity.can_natural_spawn(world) {
                world.spawn_entity(entity);
                same_count += 1;
            }

        }

    }

}
