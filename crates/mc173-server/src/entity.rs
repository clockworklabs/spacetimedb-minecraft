//! Entity tracking.

use std::ops::{Mul, Div};

use glam::{DVec3, Vec2, IVec3};

use mc173::entity::{self as e, Entity, EntityKind, BaseKind, ProjectileKind, LivingKind};
use mc173::world::World;
use mc173::block;

use crate::proto::{OutPacket, self};
use crate::player::ServerPlayer;
use crate::config;


/// This structure tracks every entity spawned in the world and save their previous 
/// position/look (and motion for some entities). It handle allows sending the right
/// packets to the right players when these properties are changed.
#[derive(Debug)]
pub struct EntityTracker {
    /// The entity id.
    id: u32,
    /// Maximum tracking distance for this type of entity.
    distance: u16,
    /// Update interval for this type of entity.
    interval: u16,
    /// Internal time of this entity tracker, this is reset to 0 when reaching interval.
    time: u16,
    /// This countdown is reset when the absolute position is sent, if the absolute 
    /// position has not been sent for 400 ticks (20 seconds), it's resent.
    absolute_countdown_time: u16,
    /// True when the velocity must be sent when changing.
    vel_enable: bool,
    /// Last known position of the entity.
    pos: (i32, i32, i32),
    /// Last known velocity of the entity.
    vel: (i16, i16, i16),
    /// Last known look of the entity.
    look: (i8, i8),
    /// Last encoded position sent to clients.
    sent_pos: (i32, i32, i32),
    /// Last encoded velocity sent to clients.
    sent_vel: (i16, i16, i16),
    /// Last encoded look sent to clients.
    sent_look: (i8, i8),
}

impl EntityTracker {

    /// Construct an entity tracker for the given entity with its id. The tracker 
    /// configuration will be different depending on the entity kind, and the initial
    /// position and look are encoded.
    pub fn new(id: u32, entity: &Entity) -> Self {

        let (distance, interval, vel_enable) = match entity.kind() {
            EntityKind::Human => (512, 2, false),
            EntityKind::Bobber => (64, 5, true),
            EntityKind::Arrow => (64, 20, false),
            EntityKind::Fireball => (64, 1, false), // Notchian use 10 ticks
            EntityKind::Snowball => (64, 10, false),
            EntityKind::Egg => (64, 10, false),
            EntityKind::Item => (64, 5, true), // Notchian use 20 ticks
            EntityKind::Minecart => (160, 5, true),
            EntityKind::Boat => (160, 5, true),
            EntityKind::Squid => (160, 3, true),
            EntityKind::Tnt => (160, 10, true),
            EntityKind::FallingBlock => (160, 20, true),
            EntityKind::Painting => (160, 0, false),
            // All remaining animals and mobs.
            _ => (160, 3, true)
        };

        let mut tracker = Self {
            id,
            distance,
            interval,
            time: 0,
            absolute_countdown_time: 0,
            vel_enable,
            pos: (0, 0, 0),
            look: (0, 0),
            vel: (0, 0, 0),
            sent_pos: (0, 0, 0),
            sent_vel: (0, 0, 0),
            sent_look: (0, 0),
        };

        // If fast entity tracking is enabled and interval is not disabled, set interval
        // to 1 tick.
        if config::fast_entity() && tracker.interval != 0 {
            tracker.interval = 1;
        }
        
        tracker.set_pos(entity.0.pos);
        tracker.set_look(entity.0.look);
        tracker.set_vel(entity.0.vel);
        tracker.sent_pos = tracker.pos;
        tracker.sent_look = tracker.look;
        tracker.sent_vel = tracker.vel;
        tracker

    } 

    /// Update the last known position of this tracked entity.
    pub fn set_pos(&mut self, pos: DVec3) {
        let scaled = pos.mul(32.0).floor().as_ivec3();
        self.pos = (scaled.x, scaled.y, scaled.z);
    }

    /// Update the last known look of this tracked entity.
    pub fn set_look(&mut self, look: Vec2) {
        // Rebase 0..2PI to 0..256. 
        let scaled = look.mul(256.0).div(std::f32::consts::TAU);
        // We can cast to i8, this will take the low 8 bits and wrap around.
        // We need to cast to i32 first because float to int cast is saturated by default.
        self.look = (scaled.x as i32 as i8, scaled.y as i32 as i8);
    }

    /// Update the last known velocity of this entity.
    pub fn set_vel(&mut self, vel: DVec3) {
        // The Notchian client clamps the input velocity, this ensure that the scaled 
        // vector is in i16 range or integers.
        let scaled = vel.clamp(DVec3::splat(-3.9), DVec3::splat(3.9)).mul(8000.0).as_ivec3();
        self.vel = (scaled.x as i16, scaled.y as i16, scaled.z as i16);
    }

    /// Tick this entity tracker and update players if needed. Only the players that
    /// already track this entity will be updated if relevant.
    pub fn tick_and_update_players(&mut self, players: &[ServerPlayer]) {

        // If the interval is 0, then entity should not be updated after being created.
        if self.interval == 0 {
            return;
        }

        if config::fast_entity() {
            self.absolute_countdown_time += 20;
        } else {
            self.absolute_countdown_time += 1;
        }

        self.time += 1;

        if self.time >= self.interval {
            self.time = 0;
            self.update_players(players);
        }

    }

    /// Update this tracker to determine which move packet to send and to which players.
    pub fn update_players(&mut self, players: &[ServerPlayer]) {

        let mut send_pos = true;
        let send_look = self.look.0.abs_diff(self.sent_look.0) >= 8 || self.look.1.abs_diff(self.sent_look.1) >= 8;

        // Check if the delta can be sent with a move packet.
        let dx = i8::try_from(self.pos.0 - self.sent_pos.0).ok();
        let dy = i8::try_from(self.pos.1 - self.sent_pos.1).ok();
        let dz = i8::try_from(self.pos.2 - self.sent_pos.2).ok();

        let mut move_packet = None;
        let forced_position = self.absolute_countdown_time > 400;

        if let (false, Some(dx), Some(dy), Some(dz)) = (forced_position, dx, dy, dz) {

            // We don't send position if delta is too small.
            send_pos = dx.unsigned_abs() >= 8 || dy.unsigned_abs() >= 8 || dz.unsigned_abs() >= 8;

            if send_pos && send_look {
                move_packet = Some(OutPacket::EntityMoveAndLook(proto::EntityMoveAndLookPacket {
                    entity_id: self.id,
                    dx,
                    dy,
                    dz,
                    yaw: self.look.0,
                    pitch: self.look.1,
                }))
            } else if send_pos {
                move_packet = Some(OutPacket::EntityMove(proto::EntityMovePacket {
                    entity_id: self.id,
                    dx,
                    dy,
                    dz,
                }))
            } else if send_look {
                move_packet = Some(OutPacket::EntityLook(proto::EntityLookPacket {
                    entity_id: self.id,
                    yaw: self.look.0,
                    pitch: self.look.1,
                }))
            }

        } else {
            self.absolute_countdown_time = 0;
            move_packet = Some(OutPacket::EntityPositionAndLook(proto::EntityPositionAndLookPacket {
                entity_id: self.id,
                x: self.pos.0,
                y: self.pos.1,
                z: self.pos.2,
                yaw: self.look.0,
                pitch: self.look.1,
            }));
        }

        if send_pos {
            self.sent_pos = self.pos;
        }

        if send_look {
            self.sent_look = self.look;
        }

        if let Some(packet) = move_packet {
            for player in players {
                if player.tracked_entities.contains(&self.id) {
                    player.send(packet.clone());
                }
            }
        }

        // If velocity tracking is enabled...
        if self.vel_enable {

            // We differ from the Notchian server because we don't check for the distance.
            let dvx = self.vel.0 as i32 - self.sent_vel.0 as i32;
            let dvy = self.vel.1 as i32 - self.sent_vel.1 as i32;
            let dvz = self.vel.2 as i32 - self.sent_vel.2 as i32;
            // If any axis velocity change by 0.0125 (100 when encoded *8000).
            if dvx.abs() > 100 || dvy.abs() > 100 || dvz.abs() > 100 {
                
                for player in players {
                    if player.tracked_entities.contains(&self.id) {
                        player.send(OutPacket::EntityVelocity(proto::EntityVelocityPacket {
                            entity_id: self.id,
                            vx: self.vel.0,
                            vy: self.vel.1,
                            vz: self.vel.2,
                        }));
                    }
                }
        
                self.sent_vel = self.vel;

            }
            
        }

    }

    /// Update players to track or untrack this entity. 
    /// See [`update_tracking_player`](Self::update_tracking_player).
    pub fn update_tracking_players(&self, players: &mut [ServerPlayer], world: &World) {
        for player in players {
            self.update_tracking_player(player, world);
        }
    }

    /// Update a player to track or untrack this entity. The correct packet is sent if
    /// the entity needs to appear or disappear on the client side.
    pub fn update_tracking_player(&self, player: &mut ServerPlayer, world: &World) {
        
        // A player cannot track its own entity.
        if player.entity_id == self.id {
            return;
        }

        let delta = player.pos - IVec3::new(self.pos.0, self.pos.1, self.pos.2).as_dvec3() / 32.0;
        if delta.x.abs() <= self.distance as f64 && delta.z.abs() <= self.distance as f64 {
            if player.tracked_entities.insert(self.id) {
                self.spawn_entity(player, world);
            }
        } else if player.tracked_entities.remove(&self.id) {
            self.kill_entity(player);
        }

    }

    /// Force untrack this entity to this player if the player is already tracking it.
    pub fn untrack_player(&self, player: &mut ServerPlayer) {
        if player.tracked_entities.remove(&self.id) {
            self.kill_entity(player);
        }
    }

    /// Force untrack this entity to all given players, it applies only to players that
    /// were already tracking the entity.
    pub fn untrack_players(&self, players: &mut [ServerPlayer]) {
        for player in players {
            self.untrack_player(player);
        }
    }

    /// Spawn the entity on the player side.
    pub fn spawn_entity(&self, player: &ServerPlayer, world: &World) {

        // NOTE: Silently ignore dead if the entity is dead, it will be killed later.
        let Some(entity) = world.get_entity(self.id) else { return };
        let metadata = self.make_entity_metadata(entity);

        let Entity(base, base_kind) = entity;

        match base_kind {
            BaseKind::Item(item) => self.spawn_entity_item(player, base, item),
            BaseKind::Painting(_) => todo!(),  // TODO:
            BaseKind::Boat(_) => self.spawn_entity_object(player, 1, false),
            BaseKind::Minecart(e::Minecart::Normal) => self.spawn_entity_object(player, 10, false),
            BaseKind::Minecart(e::Minecart::Chest { .. }) => self.spawn_entity_object(player, 11, false),
            BaseKind::Minecart(e::Minecart::Furnace { .. }) => self.spawn_entity_object(player, 12, false),
            BaseKind::LightningBolt(_) => (),
            BaseKind::FallingBlock(falling_block) => {
                // NOTE: We use sand for any block id that is unsupported.
                match falling_block.block_id {
                    block::GRAVEL => self.spawn_entity_object(player, 71, false),
                    _ => self.spawn_entity_object(player, 70, false),
                }
            }
            BaseKind::Tnt(_) => self.spawn_entity_object(player, 50, false),
            BaseKind::Projectile(_, projectile_kind) => {
                match projectile_kind {
                    ProjectileKind::Arrow(_) => self.spawn_entity_object(player, 60, true),
                    ProjectileKind::Egg(_) => self.spawn_entity_object(player, 62, true),
                    ProjectileKind::Fireball(_) => self.spawn_entity_object(player, 63, true),
                    ProjectileKind::Snowball(_) => self.spawn_entity_object(player, 61, true),
                    ProjectileKind::Bobber(_) => self.spawn_entity_object(player, 90, true),
                }
            }
            BaseKind::Living(_, living_kind) => {
                match living_kind {
                    LivingKind::Human(pl) => self.spawn_entity_human(player, pl, metadata),
                    LivingKind::Ghast(_) => self.spawn_entity_mob(player, 56, metadata),
                    LivingKind::Slime(_) => self.spawn_entity_mob(player, 55, metadata),
                    LivingKind::Pig(_) => self.spawn_entity_mob(player, 90, metadata),
                    LivingKind::Chicken(_) => self.spawn_entity_mob(player, 93, metadata),
                    LivingKind::Cow(_) => self.spawn_entity_mob(player, 92, metadata),
                    LivingKind::Sheep(_) => self.spawn_entity_mob(player, 91, metadata),
                    LivingKind::Squid(_) => self.spawn_entity_mob(player, 94, metadata),
                    LivingKind::Wolf(_) => self.spawn_entity_mob(player, 95, metadata),
                    LivingKind::Creeper(_) => self.spawn_entity_mob(player, 50, metadata),
                    LivingKind::Giant(_) => self.spawn_entity_mob(player, 53, metadata),
                    LivingKind::PigZombie(_) => self.spawn_entity_mob(player, 57, metadata),
                    LivingKind::Skeleton(_) => self.spawn_entity_mob(player, 51, metadata),
                    LivingKind::Spider(_) => self.spawn_entity_mob(player, 52, metadata),
                    LivingKind::Zombie(_) => self.spawn_entity_mob(player, 54, metadata),
                }
            }
        }

    }

    fn spawn_entity_human(&self, player: &ServerPlayer, human: &e::Human, metadata: Vec<proto::Metadata>) {
        
        player.send(OutPacket::HumanSpawn(proto::HumanSpawnPacket {
            entity_id: self.id,
            username: human.username.clone(),
            x: self.sent_pos.0, 
            y: self.sent_pos.1, 
            z: self.sent_pos.2, 
            yaw: self.sent_look.0,
            pitch: self.sent_look.1,
            current_item: 0, // TODO:
        }));

        player.send(OutPacket::EntityMetadata(proto::EntityMetadataPacket { 
            entity_id: self.id,
            metadata,
        }));

    }

    fn spawn_entity_item(&self, player: &ServerPlayer, base: &e::Base, item: &e::Item) {
        let vel = base.vel.mul(128.0).as_ivec3();
        player.send(OutPacket::ItemSpawn(proto::ItemSpawnPacket { 
            entity_id: self.id, 
            stack: item.stack, 
            x: self.sent_pos.0, 
            y: self.sent_pos.1, 
            z: self.sent_pos.2, 
            vx: vel.x as i8,
            vy: vel.y as i8,
            vz: vel.z as i8,
        }));
    }

    fn spawn_entity_object(&self, player: &ServerPlayer, kind: u8, vel: bool) {
        player.send(OutPacket::ObjectSpawn(proto::ObjectSpawnPacket {
            entity_id: self.id,
            kind,
            x: self.sent_pos.0, 
            y: self.sent_pos.1, 
            z: self.sent_pos.2, 
            velocity: vel.then(|| self.sent_vel)
        }));
    }

    fn spawn_entity_mob(&self, player: &ServerPlayer, kind: u8, metadata: Vec<proto::Metadata>) {
        player.send(OutPacket::MobSpawn(proto::MobSpawnPacket {
            entity_id: self.id,
            kind,
            x: self.sent_pos.0, 
            y: self.sent_pos.1, 
            z: self.sent_pos.2, 
            yaw: self.sent_look.0,
            pitch: self.sent_look.1,
            metadata,
        }));
    }

    /// Kill the entity on the player side.
    pub fn kill_entity(&self, player: &ServerPlayer) {
        player.send(OutPacket::EntityKill(proto::EntityKillPacket { 
            entity_id: self.id
        }));
    }

    /// Update an entity metadata on player side.
    pub fn update_entity(&self, player: &ServerPlayer, world: &World) {

        // NOTE: Silently ignore dead if the entity is dead, it will be killed later.
        let Some(entity) = world.get_entity(self.id) else { return };
        let metadata = self.make_entity_metadata(entity);

        player.send(OutPacket::EntityMetadata(proto::EntityMetadataPacket { 
            entity_id: self.id,
            metadata,
        }));

    }

    /// Internal method to generate an entity metadata vector.
    #[inline(always)]
    fn make_entity_metadata(&self, Entity(_, base_kind): &Entity) -> Vec<proto::Metadata> {
        match base_kind {
            BaseKind::Living(living, living_kind) => {
                match living_kind {
                    LivingKind::Human(human) => vec![
                        proto::Metadata::new_byte(0, (human.sneaking as i8) << 1),
                    ],
                    LivingKind::Ghast(_) => vec![
                        proto::Metadata::new_byte(16, (living.attack_time > 50) as _),
                    ],
                    LivingKind::Slime(slime) => vec![
                        proto::Metadata::new_byte(16, (slime.size as i8).saturating_add(1)),
                    ],
                    LivingKind::Pig(pig) => vec![
                        proto::Metadata::new_byte(16, pig.saddle as _),
                    ],
                    LivingKind::Sheep(sheep) => vec![
                        proto::Metadata::new_byte(16, 
                            ((sheep.sheared as i8) << 4) | 
                            ((sheep.color as i8) & 15)),
                    ],
                    LivingKind::Wolf(wolf) => vec![
                        proto::Metadata::new_byte(16, 
                            ((wolf.sitting as i8) << 0) |
                            ((wolf.angry as i8) << 1) |
                            ((wolf.owner.is_some() as i8) << 2))
                    ],
                    LivingKind::Creeper(creeper) => vec![
                        proto::Metadata::new_byte(16, if creeper.ignited_time.is_some() { 1 } else { -1 }),
                        proto::Metadata::new_byte(17, creeper.powered as _),
                    ],
                    _ => vec![]
                }
            }
            _ => vec![]
        }
    }

}
