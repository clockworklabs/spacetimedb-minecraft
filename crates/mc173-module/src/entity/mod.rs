//! Entities structures and logic implementation.

use glam::{DVec3, Vec2, IVec3};

use crate::block::material::Material;
use crate::util::default as def;
use crate::geom::BoundingBox;
use crate::rand::JavaRandom;
use crate::item::ItemStack;
use crate::world::World;
use crate::block;

pub mod common;

mod tick;
mod tick_state;
mod tick_ai;
mod tick_attack;

use tick_state::tick_state;
use tick_ai::tick_ai;
use tick_attack::tick_attack;


/// Kind of entity, without actual data. This enumeration can be used to construct a
/// real entity instance with default values, to be modified later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Item,
    Painting,
    Boat,
    Minecart,
    Bobber,
    LightningBolt,
    FallingBlock,
    Tnt,
    Arrow,
    Egg,
    Fireball,
    Snowball,
    Human,
    Ghast,
    Slime,
    Pig,
    Chicken,
    Cow,
    Sheep,
    Squid,
    Wolf,
    Creeper,
    Giant,
    PigZombie,
    Skeleton,
    Spider,
    Zombie,
}

/// Category of entity enumeration, this defines various common properties for groups of
/// entities, such as natural spawning properties. 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCategory {
    /// All animal entities.
    Animal = 0,
    /// Water animal entities.
    WaterAnimal = 1,
    /// Mob entities.
    Mob = 2,
    /// All remaining entities.
    Other = 3,
}

/// Base type that contains all entity types, this is composed of the entity base data,
/// which is common to all entities, and the base kind that is the first sub division in
/// entities. Each subdivision in the entity family tree is composed of the family's
/// common data as the first tuple element, and the kind of entity as the second element.
#[derive(Debug, Clone)]
pub struct Entity(pub Base, pub BaseKind);

/// Kind of base entity.
#[derive(Debug, Clone)]
pub enum BaseKind {
    Item(Item),
    Painting(Painting),
    Boat(Boat),
    Minecart(Minecart),
    LightningBolt(LightningBolt),
    FallingBlock(FallingBlock),
    Tnt(Tnt),
    Projectile(Projectile, ProjectileKind),
    Living(Living, LivingKind),
}

/// Kind of projectile entity.
#[derive(Debug, Clone)]
pub enum ProjectileKind {
    Arrow(Arrow),
    Egg(Egg),
    Fireball(Fireball),
    Snowball(Snowball),
    Bobber(Bobber),
}

/// Kind of living entity, this include animals and mobs.
#[derive(Debug, Clone)]
pub enum LivingKind {
    // Not categorized
    Human(Human),
    Ghast(Ghast),
    Slime(Slime),
    // Animal
    Pig(Pig),
    Chicken(Chicken),
    Cow(Cow),
    Sheep(Sheep),
    Squid(Squid),
    Wolf(Wolf),
    // Mob
    Creeper(Creeper),
    Giant(Giant),
    PigZombie(PigZombie),
    Skeleton(Skeleton),
    Spider(Spider),
    Zombie(Zombie),
}

/// The base data common to all entities.
#[derive(Debug, Clone, Default)]
pub struct Base {
    /// Tell if this entity is persistent or not. A persistent entity is saved with its
    /// chunk, but non-persistent entities are no saved. For example, all player entities
    /// are typically non-persistent because these are not real entities. Some entities
    /// cannot be persistent as they are not supported by the Notchian serialization.
    pub persistent: bool,
    /// The last size that was used when recomputing the bounding box based on the 
    /// position, we keep it in order to check that the bounding box don't shift too far
    /// from it because of rounding errors, and also to keep the height center. This is
    /// updated with the bounding box by `tick_base` method when entity isn't coherent.
    pub size: Size,
    /// The bounding box is defining the actual position from the size of the entity, the 
    /// actual position of the entity is derived from it. This is recomputed with the size
    /// by `tick_base` method when entity isn't coherent.
    pub bb: BoundingBox,
    /// The current entity position, it is derived from the bounding box and size, it can
    /// be forced by setting it and then calling `resize` on entity.
    pub pos: DVec3,
    /// True if an entity pos event should be sent after update.
    /// The current entity velocity.
    pub vel: DVec3,
    /// Yaw a pitch angles of this entity's look. These are in radians with no range 
    /// guarantee, although this will often be normalized in 2pi range. The yaw angle
    /// in Minecraft is set to zero when pointing toward PosZ, and then rotate clockwise
    /// to NegX, NegZ and then PosX.
    /// 
    /// Yaw is X and pitch is Y.
    pub look: Vec2,
    /// Lifetime of the entity since it was spawned in the world, it increase at every
    /// world tick.
    pub lifetime: u32,
    /// Height of the eyes, this is an Y offset from the position.
    pub eye_height: f32,
    /// Set to true when the entity is able to pickup surrounding items and arrows on
    /// ground, if so a pickup event is triggered, but the item or arrow is not actually
    /// picked up, it's up to the event listener to decide. Disabled by default.
    pub can_pickup: bool,
    /// No clip is used to disable collision check when moving the entity, if no clip is
    /// false, then the entity will be constrained by bounding box in its way.
    pub no_clip: bool,
    /// Is this entity currently on ground.
    pub on_ground: bool,
    /// Is this entity in water.
    pub in_water: bool,
    /// Is this entity in lava.
    pub in_lava: bool,
    /// Total fall distance, will be used upon contact to calculate damages to deal.
    pub fall_distance: f32,
    /// Remaining fire ticks.
    pub fire_time: u32,
    /// Remaining air ticks to breathe.
    pub air_time: u32,
    /// A list of hurts to apply to the entity.
    pub hurt: Vec<Hurt>,
    /// If this entity is ridden, this contains its entity id.
    pub rider_id: Option<u32>,
    /// If this entity has thrown a bobber for fishing, this contains its entity id.
    pub bobber_id: Option<u32>,
    /// The random number generator used for this entity.
    pub rand: JavaRandom,
}

/// Hurt data to apply on the next tick to the entity.
#[derive(Debug, Clone, Default)]
pub struct Hurt {
    /// The damage to deal.
    pub damage: u16,
    /// When damage is dealt, this optionally contains the entity id at the origin of the
    /// hit in order to apply knock back to the entity if needed.
    pub origin_id: Option<u32>,
}

/// The data common to all living entities.
#[derive(Debug, Clone, Default)]
pub struct Living {
    /// Set to true if an entity is artificial, as opposed to natural. If not artificial,
    /// an entity is despawned when too far from the closest player (maximum distance of 
    /// 128.0 blocks).
    pub artificial: bool,
    /// The health.
    pub health: u16,
    /// The last damage inflicted to the entity during `hurt_time`, this is used to only
    /// damage for the maximum damage inflicted while `hurt_time` is not zero.
    pub hurt_last_damage: u16,
    /// Hurt countdown, read `hurt_damage` documentation.
    pub hurt_time: u16,
    /// TBD.
    pub attack_time: u16,
    /// The death timer, increasing each tick when no health, after 20 ticks the entity
    /// is definitely removed from the world.
    pub death_time: u16,
    /// The strafing acceleration.
    pub accel_strafing: f32,
    /// The forward acceleration.
    pub accel_forward: f32,
    /// Velocity of the look's yaw axis.
    pub yaw_velocity: f32,
    /// True if this entity is trying to jump.
    pub jumping: bool,
    /// If this entity is looking at another one.
    pub look_target: Option<LookTarget>,
    /// If this entity is attacking another one.
    pub attack_target: Option<u32>,
    /// The path this creature needs to follow.
    pub path: Option<Path>,
    /// This timer is used on entities that are wandering too far from players or that
    /// take hurt damages. This is only used on entities that are AI ticked and on non
    /// persistent living entities. When this time reaches 600 and there are players in
    /// the 128.0 block distance, then this entity has 1/800 chance of despawning.
    pub wander_time: u16,
}

/// The data common to all projectile entities.
#[derive(Debug, Clone, Default)]
pub struct Projectile {
    /// The state of the projectile, none when in air, set to block/metadata when in.
    pub state: Option<ProjectileHit>,
    /// This is the number of ticks the projectile has been in its current state.
    pub state_time: u16,
    /// Some entity id if this projectile was thrown by an entity, this is used to avoid
    /// hitting the owner.
    pub owner_id: Option<u32>,
    /// Current shaking of the projectile.
    pub shake: u8,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct ProjectileHit {
    /// The block position the projectile is in.
    pub pos: IVec3,
    /// The block the projectile is in.
    pub block: u8,
    /// The block metadata the projectile is in.
    pub metadata: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    /// The item stack represented by this entity.
    pub stack: ItemStack,
    /// The item health.
    pub health: u16,
    /// Remaining time for this item to be picked up by entities that have `can_pickup`.
    pub frozen_time: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Painting {
    /// Block position of this painting.
    pub block_pos: IVec3,
    /// Orientation of this painting at block position.
    pub orientation: PaintingOrientation,
    /// The art of the painting, which define its size.
    pub art: PaintingArt,
    /// This timer is used to repeatedly check if the painting is at a valid position.
    pub check_valid_time: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaintingOrientation {
    #[default]
    NegX,
    PosX,
    NegZ,
    PosZ,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaintingArt {
    #[default]
    Kebab,
    Aztec,
    Alban,
    Aztec2,
    Bomb,
    Plant,
    Wasteland,
    Pool,
    Courbet,
    Sea,
    Sunset,
    Creebet,
    Wanderer,
    Graham,
    Match,
    Bust,
    Stage,
    Void,
    SkullAndRoses,
    Fighters,
    Pointer,
    Pigscene,
    BurningSkull,
    Skeleton,
    DonkeyKong,
}

#[derive(Debug, Clone, Default)]
pub struct Boat { }

#[derive(Debug, Clone, Default)]
pub enum Minecart { 
    /// A normal minecart for living entity transportation.
    #[default]
    Normal,
    /// A chest minecart for storing a single chest of items.
    Chest {
        /// The inventory storing the items.
        inv: Box<[ItemStack; 27]>,
    },
    /// A furnace minecart that push when fueled.
    Furnace {
        push_x: f64,
        push_z: f64,
        /// Remaining fuel amount.
        fuel: u32,
    }
}

#[derive(Debug, Clone, Default)]
pub struct Bobber { 
    /// Some entity id if this bobber is attached to an entity instead of floating in 
    /// water.
    pub attached_id: Option<u32>,
    /// The remaining time for the bobber to be caught and have a chance of getting a 
    /// fish.
    pub catch_time: u16,
}

#[derive(Debug, Clone, Default)]
pub struct LightningBolt { }

#[derive(Debug, Clone, Default)]
pub struct FallingBlock {
    /// Number of ticks since this block is falling.
    pub fall_time: u32,
    /// The falling block id.
    pub block_id: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Tnt {
    pub fuse_time: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Arrow {
    /// Set to true for arrows that are sent by players and therefore can be picked up.
    pub from_player: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Egg { }

#[derive(Debug, Clone, Default)]
pub struct Fireball {
    /// Acceleration to that fireball.
    pub accel: DVec3,
}

#[derive(Debug, Clone, Default)]
pub struct Snowball { }

#[derive(Debug, Clone, Default)]
pub struct Human {
    /// The player username.
    pub username: String,
    /// True when the player is sleeping.
    pub sleeping: bool,
    /// True when the player is sneaking.
    pub sneaking: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Ghast {
    /// The ghast waypoint defaults to zero.
    pub waypoint: DVec3,
    /// Remaining time before changing the target waypoint of the ghast.
    pub waypoint_check_time: u8,
    /// Remaining time before searching an attack target again.
    pub attack_target_time: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Slime {
    /// Size of the slime, this is a bit different because here the size is initially 
    /// at 0 and this is equivalent to 1 in Notchian implementation.
    pub size: u8,
    /// Remaining time before jumping.
    pub jump_remaining_time: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Pig {
    /// True when the pig has a saddle.
    pub saddle: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Chicken {
    /// Ticks remaining until this chicken lays an egg.
    pub next_egg_ticks: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Cow { }

#[derive(Debug, Clone, Default)]
pub struct Sheep {
    pub sheared: bool,
    pub color: u8, // TODO: Color enumeration.
}

#[derive(Debug, Clone, Default)]
pub struct Squid {
    /// Animation progress for the squid.
    pub animation: f32,
    /// Speed of the animation.
    pub animation_speed: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Wolf {
    pub angry: bool,
    pub sitting: bool,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Creeper { 
    /// True when the creeper is powered.
    pub powered: bool,
    /// Set to some time when the creeper is ignited.
    pub ignited_time: Option<u16>
}

#[derive(Debug, Clone, Default)]
pub struct Giant { }

#[derive(Debug, Clone, Default)]
pub struct PigZombie { 
    pub anger: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Skeleton { }

#[derive(Debug, Clone, Default)]
pub struct Spider { }

#[derive(Debug, Clone, Default)]
pub struct Zombie { }


/// Size of an entity, used to update each entity bounding box prior to ticking if 
/// relevant.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Size {
    /// Width of the bounding box, centered on the X/Z coordinates.
    pub width: f32,
    /// Height of the bounding box.
    pub height: f32,
    /// Define the center of the bounding box on Y axis.
    pub center: f32,
}

impl Size {

    /// New size with the Y position at the bottom center of the bounding box.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, center: 0.0 }
    }

    /// New size with the Y position at the center of the bounding box.
    pub fn new_centered(width: f32, height: f32) -> Self {
        Self { width, height, center: height / 2.0 }
    }

}

/// Define a target for an entity to look at.
#[derive(Debug, Clone, Default)]
pub struct LookTarget {
    /// The entity id to look at.
    pub entity_id: u32,
    /// Ticks remaining before stop looking at it.
    pub remaining_time: u32,
}

/// A result of the path finder.
#[derive(Debug, Clone)]
pub struct Path {
    pub points: Vec<IVec3>,
    pub index: usize,
}

impl From<Vec<IVec3>> for Path {
    fn from(points: Vec<IVec3>) -> Self {
        Self { points, index: 0 }
    }
}

impl From<IVec3> for Path {
    fn from(value: IVec3) -> Self {
        Self { points: vec![value], index: 0 }
    }
}

impl Path {

    /// Return the current path position.
    pub fn point(&self) -> Option<IVec3> {
        self.points.get(self.index).copied()
    }

    /// Advanced the path by one point.
    pub fn advance(&mut self) {
        self.index += 1;
    }
    
}


impl Entity {

    /// Get the kind of entity from this instance.
    pub fn kind(&self) -> EntityKind {
        self.1.entity_kind()
    }

    /// Get the category of entity from this instance.
    #[inline]
    pub fn category(&self) -> EntityCategory {
        self.kind().category()
    }

    /// This this entity from its id in a world.
    /// 
    /// **This is really important to no change the entity kind when ticking the 
    /// function.**
    pub fn tick(&mut self, world: &mut World, id: u32) {
        tick::tick(world, id, self);
    }

    /// Recompute this entity's size and recompute the bounding box from its position.
    pub fn resize(&mut self) {

        let Entity(base, base_kind) = self;

        // Calculate the new size from the entity properties.
        base.size = match base_kind {
            BaseKind::Item(_) => Size::new_centered(0.25, 0.25),
            BaseKind::Painting(_) => Size::new(0.5, 0.5),
            BaseKind::Boat(_) => Size::new_centered(1.5, 0.6),
            BaseKind::Minecart(_) => Size::new_centered(0.98, 0.7),
            BaseKind::LightningBolt(_) => Size::new(0.0, 0.0),
            BaseKind::FallingBlock(_) => Size::new_centered(0.98, 0.98),
            BaseKind::Tnt(_) => Size::new_centered(0.98, 0.98),
            BaseKind::Projectile(_, ProjectileKind::Arrow(_)) => Size::new(0.5, 0.5),
            BaseKind::Projectile(_, ProjectileKind::Egg(_)) => Size::new(0.5, 0.5),
            BaseKind::Projectile(_, ProjectileKind::Fireball(_)) => Size::new(1.0, 1.0),
            BaseKind::Projectile(_, ProjectileKind::Snowball(_)) => Size::new(0.5, 0.5),
            BaseKind::Projectile(_, ProjectileKind::Bobber(_)) => Size::new(0.25, 0.25),
            BaseKind::Living(_, LivingKind::Human(player)) => {
                if player.sleeping {
                    Size::new(0.2, 0.2)
                } else {
                    Size::new(0.6, 1.8)
                }
            }
            BaseKind::Living(_, LivingKind::Ghast(_)) => Size::new(4.0, 4.0),
            BaseKind::Living(_, LivingKind::Slime(slime)) => {
                let factor = slime.size as f32 + 1.0;
                Size::new(0.6 * factor, 0.6 * factor)
            }
            BaseKind::Living(_, LivingKind::Pig(_)) => Size::new(0.9, 0.9),
            BaseKind::Living(_, LivingKind::Chicken(_)) => Size::new(0.3, 0.4),
            BaseKind::Living(_, LivingKind::Cow(_)) => Size::new(0.9, 1.3),
            BaseKind::Living(_, LivingKind::Sheep(_)) =>Size::new(0.9, 1.3),
            BaseKind::Living(_, LivingKind::Squid(_)) => Size::new(0.95, 0.95),
            BaseKind::Living(_, LivingKind::Wolf(_)) => Size::new(0.8, 0.8),
            BaseKind::Living(_, LivingKind::Creeper(_)) => Size::new(0.6, 1.8),
            BaseKind::Living(_, LivingKind::Giant(_)) => Size::new(3.6, 10.8),
            BaseKind::Living(_, LivingKind::PigZombie(_)) => Size::new(0.6, 1.8),
            BaseKind::Living(_, LivingKind::Skeleton(_)) => Size::new(0.6, 1.8),
            BaseKind::Living(_, LivingKind::Spider(_)) => Size::new(1.4, 0.9),
            BaseKind::Living(_, LivingKind::Zombie(_)) => Size::new(0.6, 1.8),
        };

        // Calculate new eyes height.
        base.eye_height = match base_kind {
            BaseKind::Living(_, LivingKind::Human(_)) => 1.62,
            BaseKind::Living(_, LivingKind::Wolf(_)) => base.size.height * 0.8,
            BaseKind::Living(_, _) => base.size.height * 0.85,
            _ => 0.0,
        };

        // Finally update the bounding box.
        common::update_bounding_box_from_pos(base);

    }

    /// Teleport the entity to a specific position, this function keep the bounding box
    /// synchronized with the position.
    pub fn teleport(&mut self, pos: DVec3) {
        let Entity(base, _) = self;
        base.pos = pos;
        common::update_bounding_box_from_pos(base);
    }

    /// Return true if the entity can naturally spawn at its current position (with
    /// synchronized bounding box) in the given world. The entity is mutated because its
    /// RNG may be used.
    pub fn can_natural_spawn(&mut self, world: &World) -> bool {

        let Entity(base, BaseKind::Living(_, living_kind)) = self else {
            // Non-living entities cannot naturally spawn.
            return false;
        };

        let kind = living_kind.entity_kind();
        let block_pos = IVec3 {
            x: base.bb.center_x().floor() as i32,
            y: base.bb.min.y.floor() as i32,
            z: base.bb.center_z().floor() as i32,
        };

        let category = kind.category();

        if category == EntityCategory::Animal {
            
            // Animals can only spawn on grass blocks.
            if !world.is_block(block_pos - IVec3::Y, block::GRASS) {
                return false;
            }

            // Animals requires a light level of at least 9.
            if world.get_light(block_pos).max() <= 8 {
                return false;
            }

        } else if category == EntityCategory::Mob {

            let light = world.get_light(block_pos);

            // Lower chance of spawn if there is sky light.
            if light.sky as i32 > base.rand.next_int_bounded(32) {
                return false;
            }

            // Random spawning chance when light is under 8.
            if light.max_real() as i32 > base.rand.next_int_bounded(8) {
                return false;
            }

        }

        if category != EntityCategory::Other {
            let weight_func = common::path_weight_func(living_kind);
            if weight_func(world, block_pos) < 0.0 {
                return false;
            }
        }

        // Any hard entity colliding prevent spawning.
        if world.has_entity_colliding(base.bb, true) {
            return false;
        }

        if category != EntityCategory::WaterAnimal {
            
            // Any block colliding prevent spawning.
            if world.iter_blocks_boxes_colliding(base.bb).next().is_some() {
                return false;
            }

            // Any colliding fluid block prevent spawning.
            if world.iter_blocks_in_box(base.bb).any(|(_pos, block, _)| block::material::is_fluid(block)) {
                return false;
            }

        }

        true

    }

    /// Initialize this entity for natural spawn, for example this randomize the slime
    /// size or sheep color or make a spider with jokey.
    pub fn init_natural_spawn(&mut self, _world: &mut World) {

        let Entity(base, BaseKind::Living(_, living_kind)) = self else {
            // Non-living entities cannot naturally spawn.
            return;
        };

        match living_kind {
            LivingKind::Slime(slime) => {
                slime.size = 1 << base.rand.next_int_bounded(3) as u8;
                self.resize();
            }
            LivingKind::Sheep(sheep) => {
                let rand = base.rand.next_int_bounded(100) as u8;
                sheep.color = match rand {
                    0..=4 => 15,
                    5..=9 => 7,
                    10..=14 => 8,
                    15..=17 => 12,
                    _ if base.rand.next_int_bounded(500) == 0 => 6,
                    _ => 0,
                };
            }
            _ => {}
        }

    }

}

impl BaseKind {

    /// Get the generic entity kind from this base entity kind.
    pub fn entity_kind(&self) -> EntityKind {
        match self {
            BaseKind::Item(_) => EntityKind::Item,
            BaseKind::Painting(_) => EntityKind::Painting,
            BaseKind::Boat(_) => EntityKind::Boat,
            BaseKind::Minecart(_) => EntityKind::Minecart,
            BaseKind::LightningBolt(_) => EntityKind::LightningBolt,
            BaseKind::FallingBlock(_) => EntityKind::FallingBlock,
            BaseKind::Tnt(_) => EntityKind::Tnt,
            BaseKind::Projectile(_, kind) => kind.entity_kind(),
            BaseKind::Living(_, kind) => kind.entity_kind(),
        }
    }

}

impl LivingKind {

    /// Get the generic entity kind from this living entity kind.
    pub fn entity_kind(&self) -> EntityKind {
        match self {
            LivingKind::Human(_) => EntityKind::Human,
            LivingKind::Ghast(_) => EntityKind::Ghast,
            LivingKind::Slime(_) => EntityKind::Slime,
            LivingKind::Pig(_) => EntityKind::Pig,
            LivingKind::Chicken(_) => EntityKind::Chicken,
            LivingKind::Cow(_) => EntityKind::Cow,
            LivingKind::Sheep(_) => EntityKind::Sheep,
            LivingKind::Squid(_) => EntityKind::Squid,
            LivingKind::Wolf(_) => EntityKind::Wolf,
            LivingKind::Creeper(_) => EntityKind::Creeper,
            LivingKind::Giant(_) => EntityKind::Giant,
            LivingKind::PigZombie(_) => EntityKind::PigZombie,
            LivingKind::Skeleton(_) => EntityKind::Skeleton,
            LivingKind::Spider(_) => EntityKind::Spider,
            LivingKind::Zombie(_) => EntityKind::Zombie,
        }
    }

}

impl ProjectileKind {

    /// Get the generic entity kind from this projectile entity kind.
    pub fn entity_kind(&self) -> EntityKind {
        match self {
            ProjectileKind::Arrow(_) => EntityKind::Arrow,
            ProjectileKind::Egg(_) => EntityKind::Egg,
            ProjectileKind::Fireball(_) => EntityKind::Fireball,
            ProjectileKind::Snowball(_) => EntityKind::Snowball,
            ProjectileKind::Bobber(_) => EntityKind::Bobber,
        }
    }

}

impl EntityKind {

    /// Create a new default entity instance from the given type.
    pub fn new_default(self, pos: DVec3) -> Box<Entity> {
        match self {
            EntityKind::Item => Item::new_default(pos),
            EntityKind::Painting => Painting::new_default(pos),
            EntityKind::Boat => Boat::new_default(pos),
            EntityKind::Minecart => Minecart::new_default(pos),
            EntityKind::Bobber => Bobber::new_default(pos),
            EntityKind::LightningBolt => LightningBolt::new_default(pos),
            EntityKind::FallingBlock => FallingBlock::new_default(pos),
            EntityKind::Tnt => Tnt::new_default(pos),
            EntityKind::Arrow => Arrow::new_default(pos),
            EntityKind::Egg => Egg::new_default(pos),
            EntityKind::Fireball => Fireball::new_default(pos),
            EntityKind::Snowball => Snowball::new_default(pos),
            EntityKind::Human => Human::new_default(pos),
            EntityKind::Ghast => Ghast::new_default(pos),
            EntityKind::Slime => Slime::new_default(pos),
            EntityKind::Pig => Pig::new_default(pos),
            EntityKind::Chicken => Chicken::new_default(pos),
            EntityKind::Cow => Cow::new_default(pos),
            EntityKind::Sheep => Sheep::new_default(pos),
            EntityKind::Squid => Squid::new_default(pos),
            EntityKind::Wolf => Wolf::new_default(pos),
            EntityKind::Creeper => Creeper::new_default(pos),
            EntityKind::Giant => Giant::new_default(pos),
            EntityKind::PigZombie => PigZombie::new_default(pos),
            EntityKind::Skeleton => Skeleton::new_default(pos),
            EntityKind::Spider => Spider::new_default(pos),
            EntityKind::Zombie => Zombie::new_default(pos),
        }
    }

    /// Return true if this entity kind is hard, hard entities prevent block placing and
    /// entity spawning when colliding.
    #[inline]
    pub fn is_hard(self) -> bool {
        match self {
            EntityKind::Item |
            EntityKind::Bobber |
            EntityKind::LightningBolt |
            EntityKind::Arrow |
            EntityKind::Egg |
            EntityKind::Fireball |
            EntityKind::Snowball => false,
            _ => true
        }
    }

    /// Get the category of this entity kind.
    pub fn category(self) -> EntityCategory {
        match self {
            EntityKind::Pig |
            EntityKind::Chicken |
            EntityKind::Cow |
            EntityKind::Sheep |
            EntityKind::Wolf => EntityCategory::Animal,
            EntityKind::Squid => EntityCategory::WaterAnimal,
            EntityKind::Creeper |
            EntityKind::Giant |
            EntityKind::PigZombie |
            EntityKind::Skeleton |
            EntityKind::Spider |
            EntityKind::Zombie |
            EntityKind::Slime => EntityCategory::Mob,
            _ => EntityCategory::Other
        }
    }

    /// Returns the maximum number of entities of that kind that can be spawned at once
    /// when natural spawning in a single chunk.
    pub fn natural_spawn_max_chunk_count(self) -> usize {
        match self {
            EntityKind::Ghast => 1,
            EntityKind::Wolf => 8,
            _ => 4,
        }
    }

}

impl EntityCategory {

    pub const ALL: [Self; 4] = [Self::Animal, Self::WaterAnimal, Self::Mob, Self::Other];
    
    /// Returns the maximum number of entities of this category before preventing more
    /// natural spawning. This number will be multiplied by the number of spawn-able
    /// chunks and then by 256 (16x16 chunks). So this is the maximum count of entities
    /// per 16x16 chunks loaded.
    pub fn natural_spawn_max_world_count(self) -> usize {
        match self {
            EntityCategory::Animal => 15,
            EntityCategory::WaterAnimal => 5,
            EntityCategory::Mob => 70,
            EntityCategory::Other => 0,
        }
    }

    /// Returns the material this entity is able to spawn in, this is a preliminary check.
    pub fn natural_spawn_material(self) -> Material {
        match self {
            EntityCategory::Animal => Material::Air,
            EntityCategory::WaterAnimal => Material::Water,
            EntityCategory::Mob => Material::Air,
            EntityCategory::Other => Material::Air,
        }
    }

}


macro_rules! impl_new_with {
    ( Base: $( $kind:ident $($def:expr)? ),* ) => {
        
        $(impl $kind {

            /// Create a new instance of this entity type and initialize the entity with
            /// a closure, the entity is then resized to initialize its bounding box.
            #[inline]
            pub fn new_with(func: impl FnOnce(&mut Base, &mut $kind)) -> Box<Entity> {
                let mut entity = Box::new(Entity(def(), BaseKind::$kind(def())));
                let Entity(base, BaseKind::$kind(this)) = &mut *entity else { unreachable!() };
                $( ($def)(base, this); )?
                func(base, this);
                entity.resize();
                entity
            }

            /// Create a new instance of this entity at the given position, the entity is
            /// then resized to initialize its bounding box.
            pub fn new_default(pos: DVec3) -> Box<Entity> {
                Self::new_with(|base, _| base.pos = pos)
            }

        })*

    };
    ( Living: $( $kind:ident ($def_health:expr) $($def:expr)?),* ) => {
        
        $(impl $kind {
            
            /// Create a new instance of this entity type and initialize the entity with
            /// a closure, the entity is then resized to initialize its bounding box.
            #[inline]
            pub fn new_with(func: impl FnOnce(&mut Base, &mut Living, &mut $kind)) -> Box<Entity> {
                let mut entity = Box::new(Entity(def(), BaseKind::Living(def(), LivingKind::$kind(def()))));
                let Entity(base, BaseKind::Living(living, LivingKind::$kind(this))) = &mut *entity else { unreachable!() };
                living.health = $def_health;
                $( ($def)(base, living, this); )?
                func(base, living, this);
                entity.resize();
                entity
            }

            /// Create a new instance of this entity at the given position, the entity is
            /// then resized to initialize its bounding box.
            pub fn new_default(pos: DVec3) -> Box<Entity> {
                Self::new_with(|base, _, _| base.pos = pos)
            }

        })*

    };
    ( Projectile: $( $kind:ident ),* ) => {
        
        $(impl $kind {
            
            /// Create a new instance of this entity type and initialize the entity with
            /// a closure, the entity is then resized to initialize its bounding box.
            #[inline]
            pub fn new_with(func: impl FnOnce(&mut Base, &mut Projectile, &mut $kind)) -> Box<Entity> {
                let mut entity = Box::new(Entity(def(), BaseKind::Projectile(def(), ProjectileKind::$kind(def()))));
                let Entity(base, BaseKind::Projectile(projectile, ProjectileKind::$kind(this))) = &mut *entity else { unreachable!() };
                func(base, projectile, this);
                entity.resize();
                entity
            }

            /// Create a new instance of this entity at the given position, the entity is
            /// then resized to initialize its bounding box.
            pub fn new_default(pos: DVec3) -> Box<Entity> {
                Self::new_with(|base, _, _| base.pos = pos)
            }

        })*

    };
}

impl_new_with!(Base: 
    Item |_: &mut Base, this: &mut Item| { 
        this.health = 5; 
        this.stack = ItemStack::new_block(block::STONE, 0);
    },
    Painting, 
    Boat, 
    Minecart, 
    LightningBolt, 
    FallingBlock |_: &mut Base, this: &mut FallingBlock| {
        this.block_id = block::SAND;
    }, 
    Tnt);

impl_new_with!(Living: 
    Human(20),
    Ghast(10),
    Slime(1),
    Pig(10),
    Chicken(4),
    Cow(10),
    Sheep(10),
    Squid(10),
    Wolf(8),
    Creeper(20),
    Giant(200),
    PigZombie(20),
    Skeleton(20),
    Spider(20),
    Zombie(20));
    
impl_new_with!(Projectile: 
    Arrow,
    Egg,
    Fireball,
    Snowball,
    Bobber);
