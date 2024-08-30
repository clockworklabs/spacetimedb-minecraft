//! Minecraft Beta 1.7.3 network protocol definition.

use std::io::{Read, self, Write};
use std::fmt::Arguments;
use std::sync::Arc;

use glam::{DVec3, Vec2, IVec3};
use spacetimedb::SpacetimeType;
use mc173_module::dvec3::StdbDVec3;
use mc173_module::item::ItemStack;
use mc173_module::i32vec3::StdbI32Vec3;
use mc173_module::vec2::StdbVec2;

/// Type alias for Minecraft protocol server.
// pub type Network = net::Network<InPacket, OutPacket>;
// pub type NetworkEvent = net::NetworkEvent<InPacket>;
// pub type NetworkClient = net::NetworkClient;

/// A packet received by the server (server-bound).
#[derive(Debug, Clone)]
pub enum InPacket {
    /// Used for TCP keep alive.
    KeepAlive,
    /// A login request from the client.
    Login(StdbInLoginPacket),
    /// Sent by the client to handshake.
    Handshake(InHandshakePacket),
    /// A chat message.
    Chat(ChatPacket),
    /// The client's player interact with an entity.
    Interact(InteractPacket),
    /// The client's player want to respawn after being dead.
    Respawn(RespawnPacket),
    /// The client's player is not moving/rotating.
    Flying(FlyingPacket),
    /// The client's player is moving but not rotating.
    Position(StdbPositionPacket),
    /// The client's player is rotating but not moving.
    Look(StdbLookPacket),
    /// The client's player is moving and rotating.
    PositionLook(StdbPositionLookPacket),
    /// The client's player break a block.
    BreakBlock(BreakBlockPacket),
    /// The client's player place a block.
    PlaceBlock(PlaceBlockPacket),
    /// The client's player change its hand item.
    HandSlot(HandSlotPacket),
    /// The client's player has an animation, vanilla client usually only send swing arm.
    Animation(AnimationPacket),
    /// The player is making an action, like (un)crouch or leave bed.
    Action(ActionPacket),
    /// The client is closing a window.
    WindowClose(WindowClosePacket),
    /// The client clicked a window.
    WindowClick(WindowClickPacket),
    /// Answer to a server transaction rejection.
    WindowTransaction(WindowTransactionPacket),
    /// Sent when a player click the "Done" button after placing a sign.
    UpdateSign(UpdateSignPacket),
    /// Sent when the player disconnect from the server.
    Disconnect(DisconnectPacket),
}

/// A packet to send to a client (client-bound).
#[derive(Debug, Clone)]
pub enum OutPacket {
    /// Used for TCP keep alive.
    KeepAlive,
    /// Answered by the server to a client's login request, if successful.
    Login(OutLoginPacket),
    /// Answered by the server when the client wants to handshake.
    Handshake(OutHandshakePacket),
    /// A chat message sent to the client.
    Chat(ChatPacket),
    /// Update the world's time of the client.
    UpdateTime(UpdateTimePacket),
    /// Sent after a player spawn packet to setup each of the 5 slots (held item and 
    /// armor slots) with the items.
    PlayerInventory(PlayerInventoryPacket),
    /// Set the spawn position for the compass to point to.
    SpawnPosition(SpawnPositionPacket),
    /// Update the client's player health.
    UpdateHealth(UpdateHealthPacket),
    /// Sent to the client when the player has been successfully respawned.
    Respawn(RespawnPacket),
    /// Legal to send but not made in practice.
    Flying(FlyingPacket),
    /// Legal to send but not made in practice.
    Position(StdbPositionPacket),
    /// Legal to send but not made in practice.
    Look(StdbLookPacket),
    /// Set the client's player position and look.
    PositionLook(StdbPositionLookPacket),
    /// Set a given player to sleep in a bed.
    PlayerSleep(PlayerSleepPacket),
    /// An entity play an animation.
    EntityAnimation(AnimationPacket),
    /// A player entity to spawn.
    HumanSpawn(HumanSpawnPacket),
    /// An item entity to spawn.
    ItemSpawn(ItemSpawnPacket),
    /// An entity has picked up an entity on ground.
    EntityPickup(EntityPickupPacket),
    /// An object entity to spawn.
    ObjectSpawn(ObjectSpawnPacket),
    /// A mob entity to spawn.
    MobSpawn(MobSpawnPacket),
    /// A painting entity to spawn.
    PaintingSpawn(PaintingSpawnPacket),
    /// Update an entity velocity.
    EntityVelocity(EntityVelocityPacket),
    /// Kill an entity.
    EntityKill(EntityKillPacket),
    /// Base packet for subsequent entity packets, this packet alone is not sent by the
    /// vanilla server.
    Entity(EntityPacket),
    /// Move an entity by a given offset.
    EntityMove(EntityMovePacket),
    /// Set an entity' look.
    EntityLook(EntityLookPacket),
    /// Move an entity by a given offset and set its look.
    EntityMoveAndLook(EntityMoveAndLookPacket),
    /// Teleport an entity to a position and set its look.
    EntityPositionAndLook(EntityPositionAndLookPacket),
    /// Not fully understood.
    EntityStatus(EntityStatusPacket),
    /// Make an entity ride another one.
    EntityRide(EntityRidePacket),
    /// Modify an entity's metadata.
    EntityMetadata(EntityMetadataPacket),
    /// Notify the client of a chunk initialization or deletion, this is required before
    /// sending blocks and chunk data.
    ChunkState(ChunkStatePacket),
    /// A bulk send of chunk data.
    ChunkData(ChunkDataPacket),
    /// Many block changed at the same time.
    ChunkBlockSet(ChunkBlockSetPacket),
    /// A single block changed.
    BlockSet(BlockSetPacket),
    /// An action to apply to a block, currently only note block and pistons.
    BlockAction(BlockActionPacket),
    /// Sent when an explosion happen, from TNT or creeper.
    Explosion(ExplosionPacket),
    /// Play various effect on the client.
    EffectPlay(EffectPlayPacket),
    /// Various state notification, such as raining begin/end and invalid bed to sleep.
    Notification(NotificationPacket),
    /// Spawn a lightning bold.
    LightningBolt(LightningBoltPacket),
    /// Force the client to open a window.
    WindowOpen(WindowOpenPacket),
    /// Force the client to quit a window (when a chest is destroyed).
    WindowClose(WindowClosePacket),
    /// Change a slot in a window.
    WindowSetItem(WindowSetItemPacket),
    /// Set all items in a window.
    WindowItems(WindowItemsPacket),
    /// Set a progress bar in a window (for furnaces).
    WindowProgressBar(WindowProgressBarPacket),
    /// Information about a window transaction to the client.
    WindowTransaction(WindowTransactionPacket),
    /// A sign is discovered or is created.
    UpdateSign(UpdateSignPacket),
    /// Complex item data.
    ItemData(ItemDataPacket),
    /// Increment a statistic by a given amount.
    StatisticIncrement(StatisticIncrementPacket),
    /// Sent to a client to force disconnect it from the server.
    Disconnect(DisconnectPacket),
}

/// Packet 1 (server-bound)
#[derive(SpacetimeType, Debug, Clone)]
pub struct StdbInLoginPacket {
    /// Current protocol version, should be 14 for this version.
    pub protocol_version: i32,
    /// The username of the player that connects.
    pub username: String,
}

/// Packet 1 (client-bound)
#[derive(Debug, Clone)]
pub struct OutLoginPacket {
    /// The entity id of the player being connected.
    pub entity_id: u32,
    /// A random seed sent to the player. The client use this to recompute the biomes
    /// on their side in order to show the right foliage color and also for weather.
    pub random_seed: i64,
    /// The dimension the player is connected to.
    pub dimension: i8,
}

/// Packet 2 (server-bound)
#[derive(Debug, Clone)]
pub struct InHandshakePacket {
    /// Username of the player trying to connect.
    pub username: String,
}

/// Packet 2 (client-bound)
#[derive(Debug, Clone)]
pub struct OutHandshakePacket {
    /// Server identifier that accepted the player handshake. This equals '-' in 
    /// offline mode.
    pub server: String,
}

/// Packet 3
#[derive(Debug, Clone)]
pub struct ChatPacket {
    pub message: String,
}

/// Packet 4
#[derive(Debug, Clone)]
pub struct UpdateTimePacket {
    /// The world time (in game ticks).
    pub time: u64,
}

/// Packet 5
#[derive(Debug, Clone)]
pub struct PlayerInventoryPacket {
    pub entity_id: u32,
    pub slot: i16,
    pub stack: Option<ItemStack>,
}

/// Packet 6
#[derive(Debug, Clone)]
pub struct SpawnPositionPacket {
    /// The spawn position.
    pub pos: IVec3,
}

/// Packet 7
#[derive(Debug, Clone)]
pub struct InteractPacket {
    pub player_entity_id: u32,
    pub target_entity_id: u32,
    pub left_click: bool,
}

/// Packet 8
#[derive(Debug, Clone)]
pub struct UpdateHealthPacket {
    pub health: i16,
}

/// Packet 9
#[derive(Debug, Clone)]
pub struct RespawnPacket {
    pub dimension: i8,
}

/// Packet 10
#[derive(Debug, Clone)]
pub struct FlyingPacket {
    pub on_ground: bool,
}

/// Packet 11
#[derive(Debug, Clone, SpacetimeType)]
pub struct StdbPositionPacket {
    pub pos: StdbDVec3,
    pub stance: f64,
    pub on_ground: bool,
}

/// Packet 12
#[derive(Debug, Clone, SpacetimeType)]
pub struct StdbLookPacket {
    pub look: StdbVec2,
    pub on_ground: bool,
}

/// Packet 13
#[derive(Debug, Clone, SpacetimeType)]
pub struct StdbPositionLookPacket {
    pub pos: StdbDVec3,
    pub stance: f64,
    pub look: StdbVec2,
    pub on_ground: bool,
}

/// Packet 14
#[derive(Debug, Clone)]
pub struct BreakBlockPacket {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub face: u8,
    pub status: u8,
}

/// Packet 15
#[derive(Debug, Clone)]
pub struct PlaceBlockPacket {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub direction: u8,
    pub stack: Option<ItemStack>,
}

/// Packet 16
#[derive(Debug, Clone)]
pub struct HandSlotPacket {
    pub slot: i16,
}

/// Packet 17
#[derive(Debug, Clone)]
pub struct PlayerSleepPacket {
    pub entity_id: u32,
    pub unused: i8,
    pub x: i32,
    pub y: i8,
    pub z: i32,
}

/// Packet 18
#[derive(Debug, Clone)]
pub struct AnimationPacket {
    pub entity_id: u32,
    pub animate: u8,
}

/// Packet 19
#[derive(Debug, Clone)]
pub struct ActionPacket {
    pub entity_id: u32,
    /// The Notchian implementation support the following states:
    /// - 1: The player is sneaking
    /// - 2: The player is no longer sneaking
    /// - 3: The player wants to wake up from bed
    pub state: u8,
}

/// Packet 20
#[derive(Debug, Clone)]
pub struct HumanSpawnPacket {
    pub entity_id: u32,
    pub username: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
    pub current_item: u16,
}

/// Packet 21
#[derive(Debug, Clone)]
pub struct ItemSpawnPacket {
    pub entity_id: u32,
    pub stack: ItemStack,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub vx: i8,
    pub vy: i8,
    pub vz: i8,
}

/// Packet 22
#[derive(Debug, Clone)]
pub struct EntityPickupPacket {
    /// The entity id of the entity that picked up the item.
    pub entity_id: u32,
    /// The entity id of the entity that have been picked up.
    pub picked_entity_id: u32,
}

/// Packet 23
#[derive(Debug, Clone)]
pub struct ObjectSpawnPacket {
    pub entity_id: u32,
    /// Supported by Notchian impl:
    /// - 01: Boat
    /// - 10: Normal minecart
    /// - 11: Chest minecart
    /// - 12: Furnace minecart
    /// - 50: Tnt
    /// - 60: Arrow
    /// - 61: Snowball
    /// - 62: Egg
    /// - 63: Fireball
    /// - 70: Falling sand
    /// - 71: Falling gravel
    /// - 90: Fishing rod hook
    pub kind: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// For fireball and arrow.
    pub velocity: Option<(i16, i16, i16)>,
}

/// Packet 24
#[derive(Debug, Clone)]
pub struct MobSpawnPacket {
    pub entity_id: u32,
    pub kind: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
    pub metadata: Vec<Metadata>,
}

/// Packet 25
#[derive(Debug, Clone)]
pub struct PaintingSpawnPacket {
    pub entity_id: u32,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub direction: i32,
}

/// Packet 28
#[derive(Debug, Clone)]
pub struct EntityVelocityPacket {
    pub entity_id: u32,
    pub vx: i16,
    pub vy: i16,
    pub vz: i16,
}

/// Packet 29
#[derive(Debug, Clone)]
pub struct EntityKillPacket {
    pub entity_id: u32,
}

/// Packet 30
#[derive(Debug, Clone)]
pub struct EntityPacket {
    pub entity_id: u32,
}

/// Packet 31
#[derive(Debug, Clone)]
pub struct EntityMovePacket {
    pub entity_id: u32,
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

/// Packet 32
#[derive(Debug, Clone)]
pub struct EntityLookPacket {
    pub entity_id: u32,
    pub yaw: i8,
    pub pitch: i8,
}

/// Packet 33
#[derive(Debug, Clone)]
pub struct EntityMoveAndLookPacket {
    pub entity_id: u32,
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
    pub yaw: i8,
    pub pitch: i8,
}

/// Packet 34
#[derive(Debug, Clone)]
pub struct EntityPositionAndLookPacket {
    pub entity_id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
}

/// Packet 38
#[derive(Debug, Clone)]
pub struct EntityStatusPacket {
    pub entity_id: u32,
    /// Statuses supported by the Notchian client:
    /// - 2 (Living): Attack animation on the entity
    /// - 3 (Living): Death animation on the entity
    /// - 6 (Wolf): Make smoke particles
    /// - 7 (Wolf): Make hearts particles
    /// - 8 (Wolf): Make wolf shaking
    pub status: u8,
}

/// Packet 39
#[derive(Debug, Clone)]
pub struct EntityRidePacket {
    pub entity_id: u32,
    pub vehicle_entity_id: u32,
}

/// Packet 40
#[derive(Debug, Clone)]
pub struct EntityMetadataPacket {
    pub entity_id: u32,
    pub metadata: Vec<Metadata>,
}

/// Packet 50
#[derive(Debug, Clone)]
pub struct ChunkStatePacket {
    pub cx: i32,
    pub cz: i32,
    pub init: bool,
}

/// Packet 51
#[derive(Debug, Clone)]
pub struct ChunkDataPacket {
    pub x: i32,
    pub y: i16,
    pub z: i32,
    pub x_size: u8,
    pub y_size: u8,
    pub z_size: u8,
    pub compressed_data: Arc<Vec<u8>>,
}

/// Packet 52
#[derive(Debug, Clone)]
pub struct ChunkBlockSetPacket {
    pub cx: i32,
    pub cz: i32,
    pub blocks: Arc<Vec<ChunkBlockSet>>,
}

/// Represent a block change local to a chunk.
#[derive(Debug, Clone)]
pub struct ChunkBlockSet {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub block: u8,
    pub metadata: u8,
}

/// Packet 53
#[derive(Debug, Clone)]
pub struct BlockSetPacket {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub block: u8,
    pub metadata: u8,
}

/// Packet 54
#[derive(Debug, Clone)]
pub struct BlockActionPacket {
    pub x: i32,
    pub y: i16,
    pub z: i32,
    pub data0: i8,
    pub data1: i8,
}

/// Packet 60
#[derive(Debug, Clone)]
pub struct ExplosionPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub size: f32,
    pub blocks: Vec<(i8, i8, i8)>,
}

/// Packet 61
#[derive(Debug, Clone)]
pub struct EffectPlayPacket {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    /// The effect id, the Notchian client support the following effects:
    /// - 1000: Play sound 'random.click' with pitch 1.0
    /// - 1001: Play sound 'random.click' with pitch 1.2
    /// - 1002: Play sound 'random.bow' with pitch 1.2
    /// - 1003: Play sound randomly between 'random.door_open' and 'random.door_close' 
    ///         with random uniform pitch between 0.9 and 1.0
    /// - 1004: Play sound 'random.fizz' with volume 0.5 and random pitch
    /// - 1005: Play record sound, the record item id is given in effect data
    /// - 2000: Spawn smoke particles, the radius is given in effect data with two bits 
    ///         for X and Z axis, like this: `0bZZXX`
    /// - 2001: Play and show block break sound and particles, the block id is given in
    ///         effect data.
    pub effect_id: u32,
    pub effect_data: u32,
}

/// Packet 70
#[derive(Debug, Clone)]
pub struct NotificationPacket {
    /// The Notchian client understand 3 different values for this:
    /// - 0: Impossible to sleep in a bed
    /// - 1: Start raining
    /// - 2: Stop raining
    pub reason: u8,
}

/// Packet 71
#[derive(Debug, Clone)]
pub struct LightningBoltPacket {
    pub entity_id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Packet 100
#[derive(Debug, Clone)]
pub struct WindowOpenPacket {
    pub window_id: u8,
    /// Inventory type:
    /// - 0: Chest
    /// - 1: Crafting table
    /// - 2: Furnace
    /// - 3: Dispenser
    pub inventory_type: u8,
    /// The title is only actually used with chest window.
    pub title: String,
    /// The slots count in the window, only used with chest window.
    pub slots_count: u8,
}

/// Packet 101
#[derive(Debug, Clone)]
pub struct WindowClosePacket {
    pub window_id: u8,
}

/// Packet 102
#[derive(Debug, Clone)]
pub struct WindowClickPacket {
    pub window_id: u8,
    pub slot: i16,
    pub right_click: bool,
    pub shift_click: bool,
    pub transaction_id: u16,
    pub stack: Option<ItemStack>,
}

/// Packet 103
#[derive(Debug, Clone)]
pub struct WindowSetItemPacket {
    /// If `window_id = 0xFF` and `slot = -1`, this is the window cursor.
    pub window_id: u8,
    /// If `window_id = 0xFF` and `slot = -1`, this is the window cursor.
    pub slot: i16,
    pub stack: Option<ItemStack>,
}

/// Packet 104
#[derive(Debug, Clone)]
pub struct WindowItemsPacket {
    pub window_id: u8,
    pub stacks: Vec<Option<ItemStack>>,
}

/// Packet 105
#[derive(Debug, Clone)]
pub struct WindowProgressBarPacket {
    pub window_id: u8,
    /// When used for furnace:
    /// - 0: cook time
    /// - 1: burn time
    /// - 2: max burn time
    pub bar_id: u16,
    pub value: i16,
}

/// Packet 106
#[derive(Debug, Clone)]
pub struct WindowTransactionPacket {
    pub window_id: u8,
    pub transaction_id: u16,
    pub accepted: bool,
}

/// Packet 130
#[derive(Debug, Clone)]
pub struct UpdateSignPacket {
    pub x: i32,
    pub y: i16,
    pub z: i32,
    pub lines: Box<[String; 4]>,
}

/// Packet 131
#[derive(Debug, Clone)]
pub struct ItemDataPacket {
    pub id: u16,
    pub damage: u16,
    pub data: Vec<u8>,
}

/// Packet 200
#[derive(Debug, Clone)]
pub struct StatisticIncrementPacket {
    pub statistic_id: u32,
    pub amount: i8,
}

/// Packet 255
#[derive(Debug, Clone)]
pub struct DisconnectPacket {
    /// The reason for being kicked or disconnection.
    pub reason: String,
}

/// A metadata for entity.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub id: u8,
    pub kind: MetadataKind,
}

#[derive(Debug, Clone)]
pub enum MetadataKind {
    Byte(i8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    ItemStack(ItemStack),
    Position(IVec3),
}

impl Metadata {

    #[inline]
    pub fn new_byte(id: u8, value: i8) -> Self {
        Self { id, kind: MetadataKind::Byte(value) }
    }

    #[inline]
    pub fn new_short(id: u8, value: i16) -> Self {
        Self { id, kind: MetadataKind::Short(value) }
    }

    #[inline]
    pub fn new_int(id: u8, value: i32) -> Self {
        Self { id, kind: MetadataKind::Int(value) }
    }

    #[inline]
    pub fn new_float(id: u8, value: f32) -> Self {
        Self { id, kind: MetadataKind::Float(value) }
    }

    #[inline]
    pub fn new_item_stack(id: u8, value: ItemStack) -> Self {
        Self { id, kind: MetadataKind::ItemStack(value) }
    }

    #[inline]
    pub fn new_position(id: u8, value: IVec3) -> Self {
        Self { id, kind: MetadataKind::Position(value) }
    }

}


/// Return an invalid data io error with specific message.
fn new_invalid_packet_err(format: Arguments) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("invalid packet: {format}"))
}
