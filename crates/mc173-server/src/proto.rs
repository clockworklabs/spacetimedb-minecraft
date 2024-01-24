//! Minecraft Beta 1.7.3 network protocol definition.

use std::io::{Read, self, Write};
use std::fmt::Arguments;
use std::sync::Arc;

use glam::{DVec3, Vec2, IVec3};

use byteorder::{ReadBytesExt, WriteBytesExt};

use mc173::io::{ReadJavaExt, WriteJavaExt};
use mc173::item::ItemStack;

use crate::net;

/// Type alias for Minecraft protocol server.
pub type Network = net::Network<InPacket, OutPacket>;
pub type NetworkEvent = net::NetworkEvent<InPacket>;
pub type NetworkClient = net::NetworkClient;

/// A packet received by the server (server-bound).
#[derive(Debug, Clone)]
pub enum InPacket {
    /// Used for TCP keep alive.
    KeepAlive,
    /// A login request from the client.
    Login(InLoginPacket),
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
    Position(PositionPacket),
    /// The client's player is rotating but not moving.
    Look(LookPacket),
    /// The client's player is moving and rotating.
    PositionLook(PositionLookPacket),
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
    Position(PositionPacket),
    /// Legal to send but not made in practice.
    Look(LookPacket),
    /// Set the client's player position and look.
    PositionLook(PositionLookPacket),
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
#[derive(Debug, Clone)]
pub struct InLoginPacket {
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
#[derive(Debug, Clone)]
pub struct PositionPacket {
    pub pos: DVec3,
    pub stance: f64,
    pub on_ground: bool,
}

/// Packet 12
#[derive(Debug, Clone)]
pub struct LookPacket {
    pub look: Vec2,
    pub on_ground: bool,
}

/// Packet 13
#[derive(Debug, Clone)]
pub struct PositionLookPacket {
    pub pos: DVec3,
    pub stance: f64,
    pub look: Vec2,
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


impl net::InPacket for InPacket {

    fn read(read: &mut impl Read) -> io::Result<Self> {
        Ok(match read.read_u8()? {
            0 => InPacket::KeepAlive,
            1 => {

                let packet = InPacket::Login(InLoginPacket {
                    protocol_version: read.read_java_int()?, 
                    username: read.read_java_string16(16)?,
                });

                // Unused when client connects to server.
                let _map_seed = read.read_java_long()?;
                let _dimension = read.read_java_byte()?;

                packet

            }
            2 => InPacket::Handshake(InHandshakePacket {
                username: read.read_java_string16(16)?
            }),
            3 => InPacket::Chat(ChatPacket { 
                message: read.read_java_string16(119)?,
            }),
            7 => InPacket::Interact(InteractPacket {
                player_entity_id: read.read_java_int()? as u32,
                target_entity_id: read.read_java_int()? as u32,
                left_click: read.read_java_boolean()?,
            }),
            9 => InPacket::Respawn(RespawnPacket {
                dimension: read.read_java_byte()?,
            }),
            10 => InPacket::Flying(FlyingPacket {
                on_ground: read.read_java_boolean()?,
            }),
            11 => {
                let x = read.read_java_double()?;
                let y = read.read_java_double()?;
                let stance = read.read_java_double()?;
                let z = read.read_java_double()?;
                let on_ground = read.read_java_boolean()?;
                InPacket::Position(PositionPacket {
                    pos: DVec3::new(x, y, z),
                    stance,
                    on_ground,
                })
            }
            12 => {
                let yaw = read.read_java_float()?;
                let pitch = read.read_java_float()?;
                let on_ground = read.read_java_boolean()?;
                InPacket::Look(LookPacket {
                    look: Vec2::new(yaw, pitch), 
                    on_ground,
                })
            }
            13 => {
                let x = read.read_java_double()?;
                let y = read.read_java_double()?;
                let stance = read.read_java_double()?;
                let z = read.read_java_double()?;
                let yaw = read.read_java_float()?;
                let pitch = read.read_java_float()?;
                let on_ground = read.read_java_boolean()?;
                InPacket::PositionLook(PositionLookPacket {
                    pos: DVec3::new(x, y, z),
                    look: Vec2::new(yaw, pitch),
                    stance,
                    on_ground,
                })
            }
            14 => InPacket::BreakBlock(BreakBlockPacket {
                status: read.read_java_byte()? as u8,
                x: read.read_java_int()?,
                y: read.read_java_byte()?,
                z: read.read_java_int()?,
                face: read.read_java_byte()? as u8,
            }),
            15 => InPacket::PlaceBlock(PlaceBlockPacket {
                x: read.read_java_int()?,
                y: read.read_java_byte()?,
                z: read.read_java_int()?,
                direction: read.read_java_byte()? as u8,
                stack: read_item_stack(read)?,
            }),
            16 => InPacket::HandSlot(HandSlotPacket {
                slot: read.read_java_short()?,
            }),
            18 => InPacket::Animation(AnimationPacket {
                entity_id: read.read_java_int()? as u32,
                animate: read.read_java_byte()? as u8,
            }),
            19 => InPacket::Action(ActionPacket {
                entity_id: read.read_java_int()? as u32,
                state: read.read_java_byte()? as u8,
            }),
            // 27 => return Err(new_todo_packet_err("position??")),
            101 => InPacket::WindowClose(WindowClosePacket {
                window_id: read.read_java_byte()? as u8,
            }),
            102 => InPacket::WindowClick(WindowClickPacket {
                window_id: read.read_java_byte()? as u8,
                slot: read.read_java_short()?,
                right_click: read.read_java_boolean()?,
                transaction_id: read.read_java_short()? as u16,
                shift_click: read.read_java_boolean()?,
                stack: read_item_stack(read)?,
            }),
            106 => InPacket::WindowTransaction(WindowTransactionPacket {
                window_id: read.read_java_byte()? as u8,
                transaction_id: read.read_java_short()? as u16,
                accepted: read.read_java_boolean()?,
            }),
            130 => InPacket::UpdateSign(UpdateSignPacket {
                x: read.read_java_int()?,
                y: read.read_java_short()?,
                z: read.read_java_int()?,
                lines: Box::new([
                    read.read_java_string16(15)?,
                    read.read_java_string16(15)?,
                    read.read_java_string16(15)?,
                    read.read_java_string16(15)?,
                ]),
            }),
            255 => InPacket::Disconnect(DisconnectPacket {
                reason: read.read_java_string16(100)?,
            }),
            id => return Err(new_invalid_packet_err(format_args!("unknown id {id}"))),
        })
    }

}

impl net::OutPacket for OutPacket {

    fn write(&self, write: &mut impl Write) -> io::Result<()> {

        // println!("Encode packet: {self:?}");
        
        match self {
            OutPacket::KeepAlive => write.write_u8(0)?,
            OutPacket::Login(packet) => {
                write.write_u8(1)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_string16("")?; // No username it sent to the client.
                write.write_java_long(packet.random_seed)?;
                write.write_java_byte(packet.dimension)?;
            }
            OutPacket::Handshake(packet) => {
                write.write_u8(2)?;
                write.write_java_string16(&packet.server)?;
            }
            OutPacket::Chat(packet) => {
                write.write_u8(3)?;
                write.write_java_string16(&packet.message[..packet.message.len().min(199)])?;
            }
            OutPacket::UpdateTime(packet) => {
                write.write_u8(4)?;
                write.write_java_long(packet.time as i64)?;
            }
            OutPacket::PlayerInventory(packet) => {
                write.write_u8(5)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_short(packet.slot)?;
                if let Some(item) = packet.stack {
                    write.write_java_short(item.id as i16)?;
                    write.write_java_short(item.damage as i16)?;
                } else {
                    write.write_java_short(-1)?;
                    write.write_java_short(0)?;
                }
            }
            OutPacket::SpawnPosition(packet)=> {
                write.write_u8(6)?;
                write.write_java_int(packet.pos.x)?;
                write.write_java_int(packet.pos.y)?;
                write.write_java_int(packet.pos.z)?;
            }
            OutPacket::UpdateHealth(packet) => {
                write.write_u8(8)?;
                write.write_java_short(packet.health)?;
            }
            OutPacket::Respawn(packet) => {
                write.write_u8(9)?;
                write.write_java_byte(packet.dimension)?;
            }
            OutPacket::Flying(packet) => {
                write.write_u8(10)?;
                write.write_java_boolean(packet.on_ground)?;
            }
            OutPacket::Position(packet) => {
                write.write_u8(11)?;
                write.write_java_double(packet.pos.x)?;
                write.write_java_double(packet.pos.y)?;
                write.write_java_double(packet.stance)?;
                write.write_java_double(packet.pos.z)?;
                write.write_java_boolean(packet.on_ground)?;
            }
            OutPacket::Look(packet) => {
                write.write_u8(12)?;
                write.write_java_float(packet.look.x)?;
                write.write_java_float(packet.look.y)?;
                write.write_java_boolean(packet.on_ground)?;
            }
            OutPacket::PositionLook(packet) => {
                write.write_u8(13)?;
                write.write_java_double(packet.pos.x)?;
                write.write_java_double(packet.pos.y)?;
                write.write_java_double(packet.stance)?;
                write.write_java_double(packet.pos.z)?;
                write.write_java_float(packet.look.x)?;
                write.write_java_float(packet.look.y)?;
                write.write_java_boolean(packet.on_ground)?;
            }
            OutPacket::PlayerSleep(packet) => {
                write.write_u8(17)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.unused)?;
                write.write_java_int(packet.x)?;
                write.write_java_byte(packet.y)?;
                write.write_java_int(packet.z)?;
            }
            OutPacket::EntityAnimation(packet) => {
                write.write_u8(18)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.animate as i8)?;
            }
            OutPacket::HumanSpawn(packet) => {
                write.write_u8(20)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_string16(&packet.username)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.yaw)?;
                write.write_java_byte(packet.pitch)?;
                write.write_java_short(packet.current_item as i16)?;
            }
            OutPacket::ItemSpawn(packet) => {
                write.write_u8(21)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_short(packet.stack.id as i16)?;
                write.write_java_byte(packet.stack.size as i8)?;
                write.write_java_short(packet.stack.damage as i16)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.vx)?;
                write.write_java_byte(packet.vy)?;
                write.write_java_byte(packet.vz)?;
            }
            OutPacket::EntityPickup(packet) => {
                write.write_u8(22)?;
                write.write_java_int(packet.picked_entity_id as i32)?;
                write.write_java_int(packet.entity_id as i32)?;
            }
            OutPacket::ObjectSpawn(packet) => {
                write.write_u8(23)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.kind as i8)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                // PARITY: Notchian client use the owner entity id here.
                write.write_java_int(packet.velocity.is_some() as i32)?;
                if let Some((vx, vy, vz)) = packet.velocity {
                    write.write_java_short(vx)?;
                    write.write_java_short(vy)?;
                    write.write_java_short(vz)?;
                }
            }
            OutPacket::MobSpawn(packet) => {
                write.write_u8(24)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.kind as i8)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.yaw)?;
                write.write_java_byte(packet.pitch)?;
                write_metadata_list(write, &packet.metadata)?;
            }
            OutPacket::PaintingSpawn(packet) => {
                write.write_u8(25)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_string16(&packet.title)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_int(packet.direction)?;
            }
            OutPacket::EntityVelocity(packet) => {
                write.write_u8(28)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_short(packet.vx)?;
                write.write_java_short(packet.vy)?;
                write.write_java_short(packet.vz)?;
            }
            OutPacket::EntityKill(packet) => {
                write.write_u8(29)?;
                write.write_java_int(packet.entity_id as i32)?;
            }
            OutPacket::Entity(packet) => {
                write.write_u8(30)?;
                write.write_java_int(packet.entity_id as i32)?;
            }
            OutPacket::EntityMove(packet) => {
                write.write_u8(31)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.dx)?;
                write.write_java_byte(packet.dy)?;
                write.write_java_byte(packet.dz)?;
            }
            OutPacket::EntityLook(packet) => {
                write.write_u8(32)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.yaw)?;
                write.write_java_byte(packet.pitch)?;
            }
            OutPacket::EntityMoveAndLook(packet) => {
                write.write_u8(33)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.dx)?;
                write.write_java_byte(packet.dy)?;
                write.write_java_byte(packet.dz)?;
                write.write_java_byte(packet.yaw)?;
                write.write_java_byte(packet.pitch)?;
            }
            OutPacket::EntityPositionAndLook(packet) => {
                write.write_u8(34)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.yaw)?;
                write.write_java_byte(packet.pitch)?;
            }
            OutPacket::EntityStatus(packet) => {
                write.write_u8(38)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_byte(packet.status as i8)?;
            }
            OutPacket::EntityRide(packet) => {
                write.write_u8(39)?;
                write.write_java_int(packet.entity_id as i32)?;
                write.write_java_int(packet.vehicle_entity_id as i32)?;
            }
            OutPacket::EntityMetadata(packet) => {
                write.write_u8(40)?;
                write.write_java_int(packet.entity_id as i32)?;
                write_metadata_list(write, &packet.metadata)?;
            }
            OutPacket::ChunkState(packet) => {
                write.write_u8(50)?;
                write.write_java_int(packet.cx)?;
                write.write_java_int(packet.cz)?;
                write.write_java_boolean(packet.init)?;
            }
            OutPacket::ChunkData(packet) => {
                write.write_u8(51)?;
                write.write_java_int(packet.x)?;
                write.write_java_short(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte((packet.x_size - 1) as i8)?;
                write.write_java_byte((packet.y_size - 1) as i8)?;
                write.write_java_byte((packet.z_size - 1) as i8)?;
                write.write_java_int(packet.compressed_data.len() as i32)?;
                write.write_all(&packet.compressed_data)?;
            }
            OutPacket::ChunkBlockSet(packet) => {
                write.write_u8(52)?;
                write.write_java_int(packet.cx)?;
                write.write_java_int(packet.cz)?;
                write.write_java_short(packet.blocks.len() as i16)?;
                for block in &packet.blocks[..] {
                    let raw_pos = ((block.x as u16 & 15) << 12) | ((block.z as u16 & 15) << 8) | (block.y as u16 & 255);
                    write.write_java_short(raw_pos as i16)?;
                }
                for block in &packet.blocks[..] {
                    write.write_u8(block.block)?;
                }
                for block in &packet.blocks[..] {
                    write.write_u8(block.metadata)?;
                }
            }
            OutPacket::BlockSet(packet) => {
                write.write_u8(53)?;
                write.write_java_int(packet.x)?;
                write.write_java_byte(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.block as i8)?;
                write.write_java_byte(packet.metadata as i8)?;
            }
            OutPacket::BlockAction(packet) => {
                write.write_u8(54)?;
                write.write_java_int(packet.x)?;
                write.write_java_short(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_byte(packet.data0)?;
                write.write_java_byte(packet.data1)?;
            }
            OutPacket::Explosion(packet) => {
                write.write_u8(60)?;
                write.write_java_double(packet.x)?;
                write.write_java_double(packet.y)?;
                write.write_java_double(packet.z)?;
                write.write_java_float(packet.size)?;
                write.write_java_int(packet.blocks.len() as i32)?;
                for &(dx, dy, dz) in &packet.blocks {
                    write.write_java_byte(dx)?;
                    write.write_java_byte(dy)?;
                    write.write_java_byte(dz)?;
                }
            }
            OutPacket::EffectPlay(packet) => {
                write.write_u8(61)?;
                write.write_java_int(packet.effect_id as i32)?;
                write.write_java_int(packet.x)?;
                write.write_java_byte(packet.y)?;
                write.write_java_int(packet.z)?;
                write.write_java_int(packet.effect_data as i32)?;
            }
            OutPacket::Notification(packet) => {
                write.write_u8(70)?;
                write.write_java_byte(packet.reason as i8)?;
            }
            OutPacket::LightningBolt(packet) => {
                write.write_u8(71)?;
                write.write_java_boolean(true)?;
                write.write_java_int(packet.x)?;
                write.write_java_int(packet.y)?;
                write.write_java_int(packet.z)?;
            }
            OutPacket::WindowOpen(packet) => {
                write.write_u8(100)?;
                write.write_java_byte(packet.window_id as i8)?;
                write.write_java_byte(packet.inventory_type as i8)?;
                write.write_java_string8(&packet.title)?;
                write.write_java_byte(packet.slots_count as i8)?;
            }
            OutPacket::WindowClose(packet) => {
                write.write_u8(101)?;
                write.write_java_byte(packet.window_id as i8)?;
            }
            OutPacket::WindowSetItem(packet) => {
                write.write_u8(103)?;
                write.write_java_byte(packet.window_id as i8)?;
                write.write_java_short(packet.slot)?;
                write_item_stack(write, packet.stack)?;
            }
            OutPacket::WindowItems(packet) => {
                write.write_u8(104)?;
                write.write_java_byte(packet.window_id as i8)?;
                write.write_java_short(packet.stacks.len() as i16)?;
                for &item_stack in &packet.stacks {
                    write_item_stack(write, item_stack)?;
                }
            }
            OutPacket::WindowProgressBar(packet) => {
                write.write_u8(105)?;
                write.write_java_byte(packet.window_id as i8)?;
                write.write_java_short(packet.bar_id as i16)?;
                write.write_java_short(packet.value)?;
            }
            OutPacket::WindowTransaction(packet) => {
                write.write_u8(106)?;
                write.write_java_byte(packet.window_id as i8)?;
                write.write_java_short(packet.transaction_id as i16)?;
                write.write_java_boolean(packet.accepted)?;
            }
            OutPacket::UpdateSign(packet) => {
                write.write_u8(130)?;
                write.write_java_int(packet.x)?;
                write.write_java_short(packet.y)?;
                write.write_java_int(packet.z)?;
                for line in packet.lines.iter() {
                    write.write_java_string16(&line)?;
                }
            }
            OutPacket::ItemData(packet) => {
                write.write_u8(131)?;
                write.write_java_short(packet.id as i16)?;
                write.write_java_short(packet.damage as i16)?;
                
                let len = u8::try_from(packet.data.len())
                    .map_err(|_| new_invalid_packet_err(format_args!("too much item data")))?;

                write.write_u8(len)?;
                write.write_all(&packet.data)?;
            }
            OutPacket::StatisticIncrement(packet) => {
                write.write_u8(200)?;
                write.write_java_int(packet.statistic_id as i32)?;
                write.write_java_byte(packet.amount)?;
            }
            OutPacket::Disconnect(packet) => {
                write.write_u8(255)?;
                write.write_java_string16(&packet.reason)?;
            }
        }

        Ok(())

    }

}


/// Return an invalid data io error with specific message.
fn new_invalid_packet_err(format: Arguments) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("invalid packet: {format}"))
}

fn read_item_stack(read: &mut impl Read) -> io::Result<Option<ItemStack>> {
    let id = read.read_java_short()?;
    Ok(if id >= 0 {
        Some(ItemStack {
            id: id as u16,
            size: read.read_java_byte()? as u8 as u16,
            damage: read.read_java_short()? as u16,
        })
    } else {
        None
    })
}

fn write_item_stack(write: &mut impl Write, item_stack: Option<ItemStack>) -> io::Result<()> {
    if let Some(item_stack) = item_stack {
        write.write_java_short(item_stack.id as i16)?;  // TODO: Do not overflow!
        write.write_java_byte(item_stack.size as i8)?;  // TODO: Do not overflow!
        write.write_java_short(item_stack.damage as i16)
    } else {
        write.write_java_short(-1)
    }
}

fn write_metadata(write: &mut impl Write, metadata: &Metadata) -> io::Result<()> {
    
    let kind_index: u8 = match metadata.kind {
        MetadataKind::Byte(_) => 0,
        MetadataKind::Short(_) => 1,
        MetadataKind::Int(_) => 2,
        MetadataKind::Float(_) => 3,
        MetadataKind::String(_) => 4,
        MetadataKind::ItemStack(_) => 5,
        MetadataKind::Position(_) => 6,
    };

    write.write_u8((kind_index << 5) | (metadata.id & 31))?;

    match metadata.kind {
        MetadataKind::Byte(n) => write.write_java_byte(n),
        MetadataKind::Short(n) => write.write_java_short(n),
        MetadataKind::Int(n) => write.write_java_int(n),
        MetadataKind::Float(n) => write.write_java_float(n),
        MetadataKind::String(ref s) => write.write_java_string16(&s),
        MetadataKind::ItemStack(i) => {
            write.write_java_short(i.id as i16)?;
            write.write_java_byte(i.size as i8)?;
            write.write_java_short(i.damage as i16)
        }
        MetadataKind::Position(pos) => {
            write.write_java_int(pos.x)?;
            write.write_java_int(pos.y)?;
            write.write_java_int(pos.z)
        }
    }

}

fn write_metadata_list(write: &mut impl Write, list: &[Metadata]) -> io::Result<()> {
    for metadata in list {
        write_metadata(write, metadata)?;
    }
    write.write_java_byte(127)
}
