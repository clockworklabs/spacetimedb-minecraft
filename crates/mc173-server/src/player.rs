//! Server player tracker.

use std::collections::HashSet;

use glam::{DVec3, Vec2, IVec3};

use tracing::warn;

use mc173::world::{World, BlockEntityStorage, BlockEntityEvent, Event, BlockEntityProgress, EntityEvent};
use mc173::world::interact::Interaction;

use mc173::entity::{self as e, Entity, Hurt, BaseKind, LivingKind};
use mc173::block_entity::BlockEntity;
use mc173::item::{self, ItemStack};
use mc173::{block, chunk};

use mc173::inventory::InventoryHandle;
use mc173::craft::CraftTracker;
use mc173::geom::Face;

use crate::proto::{self, Network, NetworkClient, OutPacket, InPacket};
use crate::command::{self, CommandContext};
use crate::chunk::new_chunk_data_packet;
use crate::world::ServerWorldState;
use crate::offline::OfflinePlayer;


/// A server player is an actual 
pub struct ServerPlayer {
    /// The network handle for the network server.
    net: Network,
    /// The network client used to send packets through the network to that player.
    pub client: NetworkClient,
    /// The entity id this player is controlling.
    pub entity_id: u32, 
    /// The username of that player.
    pub username: String,
    /// Last position sent by the client.
    pub pos: DVec3,
    /// Last look sent by the client.
    pub look: Vec2,
    /// Set of chunks that are already sent to the player.
    pub tracked_chunks: HashSet<(i32, i32)>,
    /// Set of tracked entities by this player, all entity ids in this set are considered
    /// known and rendered by the client, when the entity will disappear, a kill packet
    /// should be sent.
    pub tracked_entities: HashSet<u32>,
    /// The main player inventory including the hotbar in the first 9 slots.
    main_inv: Box<[ItemStack; 36]>,
    /// The armor player inventory.
    armor_inv: Box<[ItemStack; 4]>,
    /// The item stacks for the 3x3 crafting grid. Also support the 2x2 as top left slots.
    craft_inv: Box<[ItemStack; 9]>,
    /// The item stack in the cursor of the client's using a window.
    cursor_stack: ItemStack,
    /// The slot current selected for the hand. Must be in range 0..9.
    hand_slot: u8,
    /// The total number of windows that have been opened by this player, this is also 
    /// used to generate a unique window id. This id should never be zero because it is
    /// reserved for the player inventory.
    window_count: u32,
    /// The current window opened on the client side. Note that the player inventory is
    /// not registered here while opened because we can't know when it is. However we 
    /// know that its window id is always 0.
    window: Window,
    /// This crafting tracker is used to update the current craft being constructed by 
    /// the player in the current window (player inventory or crafting table interface).
    craft_tracker: CraftTracker,
    /// If the player is breaking a block, this record the breaking state.
    breaking_block: Option<BreakingBlock>,
}

/// Describe an opened window and how to handle clicks into it.
#[derive(Debug, Default)]
struct Window {
    /// The unique id of the currently opened window.
    id: u8,
    /// Specialization kind of window.
    kind: WindowKind,
}

/// Describe a kind of opened window on the client side.
#[derive(Debug, Default)]
enum WindowKind {
    /// The player inventory is the default window that is always opened if no other 
    /// window is opened, it also always has the id 0, it contains the armor and craft
    /// matrix.
    #[default]
    Player,
    /// The client-side has a crafting table window opened on the given block pos.
    CraftingTable {
        pos: IVec3,
    },
    /// The client-side has a chest window opened referencing the listed block entities.
    Chest {
        pos: Vec<IVec3>,
    },
    /// The client-side has a furnace window onto the given block entity.
    Furnace {
        pos: IVec3,
    },
    /// The client-side has a dispenser window onto the given block entity.
    Dispenser {
        pos: IVec3,
    }
}

/// State of a player breaking a block.
struct BreakingBlock {
    /// The start time of this block breaking.
    start_time: u64,
    /// The position of the block.
    pos: IVec3,
    /// The block id.
    id: u8,
}

impl ServerPlayer {

    /// Construct a new player with a configured network, an associated entity id and with
    /// initial position and look given from its offline serialized data.
    pub fn new(net: &Network, client: NetworkClient, entity_id: u32, username: String, offline: &OfflinePlayer) -> Self {
        Self {
            net: net.clone(),
            client,
            entity_id,
            username,
            pos: offline.pos,
            look: offline.look,
            tracked_chunks: HashSet::new(),
            tracked_entities: HashSet::new(),
            main_inv: Box::new([ItemStack::EMPTY; 36]),
            armor_inv: Box::new([ItemStack::EMPTY; 4]),
            craft_inv: Box::new([ItemStack::EMPTY; 9]),
            cursor_stack: ItemStack::EMPTY,
            hand_slot: 0,
            window_count: 0,
            window: Window::default(),
            craft_tracker: CraftTracker::default(),
            breaking_block: None,
        }
    }

    /// Send a packet to this player.
    pub fn send(&self, packet: OutPacket) {
        // println!("[NET] Sending packet {packet:?}");
        self.net.send(self.client, packet);
    }

    /// Send a chat message to this player.
    pub fn send_chat(&self, message: String) {
        self.send(OutPacket::Chat(proto::ChatPacket { message }));
    }

    /// Handle an incoming packet from this player.
    pub fn handle(&mut self, world: &mut World, state: &mut ServerWorldState, packet: InPacket) {
        
        match packet {
            InPacket::KeepAlive => {}
            InPacket::Flying(_) => {}, // Ignore because it doesn't update anything.
            InPacket::Disconnect(_) =>
                self.handle_disconnect(),
            InPacket::Chat(packet) =>
                self.handle_chat(world, state, packet.message),
            InPacket::Position(packet) => 
                self.handle_position(world, packet),
            InPacket::Look(packet) => 
                self.handle_look(world, packet),
            InPacket::PositionLook(packet) => 
                self.handle_position_look(world, packet),
            InPacket::BreakBlock(packet) =>
                self.handle_break_block(world, packet),
            InPacket::PlaceBlock(packet) =>
                self.handle_place_block(world, packet),
            InPacket::HandSlot(packet) =>
                self.handle_hand_slot(world, packet.slot),
            InPacket::WindowClick(packet) =>
                self.handle_window_click(world, packet),
            InPacket::WindowClose(packet) =>
                self.handle_window_close(world, packet),
            InPacket::Animation(packet) =>
                self.handle_animation(world, packet),
            InPacket::Interact(packet) =>
                self.handle_interact(world, packet),
            InPacket::Action(packet) =>
                self.handle_action(world, packet),
            _ => warn!("unhandled packet from #{}: {packet:?}", self.client.id())
        }

    }

    /// Just disconnect itself, this will produce a lost event from the network.
    fn handle_disconnect(&mut self) {
        self.net.disconnect(self.client);
    }

    /// Handle a chat message packet.
    fn handle_chat(&mut self, world: &mut World, state: &mut ServerWorldState, message: String) {
        if message.starts_with('/') {
            let parts = message[1..].split_whitespace().collect::<Vec<_>>();
            command::handle_command(CommandContext {
                parts: &parts,
                world,
                state,
                player: self,
            });
        }
    }

    /// Handle a position packet.
    fn handle_position(&mut self, world: &mut World, packet: proto::PositionPacket) {
        self.handle_position_look_inner(world, Some(packet.pos), None, packet.on_ground);
    }

    /// Handle a look packet.
    fn handle_look(&mut self, world: &mut World, packet: proto::LookPacket) {
        self.handle_position_look_inner(world, None, Some(packet.look), packet.on_ground);
    }

    /// Handle a position and look packet.
    fn handle_position_look(&mut self, world: &mut World, packet: proto::PositionLookPacket) {
        self.handle_position_look_inner(world, Some(packet.pos), Some(packet.look), packet.on_ground);
    }

    fn handle_position_look_inner(&mut self, world: &mut World, pos: Option<DVec3>, look: Option<Vec2>, on_ground: bool) {

        let entity = world.get_entity_mut(self.entity_id).expect("incoherent player entity");
        entity.0.on_ground = on_ground;

        if let Some(pos) = pos {
            self.pos = pos;
            entity.teleport(pos);
        }

        if let Some(look) = look {
            self.look = Vec2::new(look.x.to_radians(), look.y.to_radians());
            entity.0.look = self.look;
        }

        if pos.is_some() {
            world.push_event(Event::Entity { id: self.entity_id, inner: EntityEvent::Position { pos: self.pos } });
            self.update_chunks(world);
        }

        if look.is_some() {
            world.push_event(Event::Entity { id: self.entity_id, inner: EntityEvent::Look { look: self.look } });
        }

    }

    /// Handle a break block packet.
    fn handle_break_block(&mut self, world: &mut World, packet: proto::BreakBlockPacket) {
        
        let face = match packet.face {
            0 => Face::NegY,
            1 => Face::PosY,
            2 => Face::NegZ,
            3 => Face::PosZ,
            4 => Face::NegX,
            5 => Face::PosX,
            _ => return,
        };

        let Some(entity) = world.get_entity_mut(self.entity_id) else { return };
        let pos = IVec3::new(packet.x, packet.y as i32, packet.z);

        tracing::trace!("packet: {packet:?}");
        // TODO: Use server time for breaking blocks.

        let in_water = entity.0.in_water;
        let on_ground = entity.0.on_ground;
        let mut stack = self.main_inv[self.hand_slot as usize];

        if packet.status == 0 {

            // Special case to extinguish fire.
            if world.is_block(pos + face.delta(), block::FIRE) {
                world.set_block_notify(pos + face.delta(), block::AIR, 0);
            }

            // We ignore any interaction result for the left click (break block) to
            // avoid opening an inventory when breaking a container.
            // NOTE: Interact before 'get_block': relevant for redstone_ore lit.
            world.interact_block(pos);

            // Start breaking a block, ignore if the position is invalid.
            if let Some((id, _)) = world.get_block(pos) {
                
                let break_duration = world.get_break_duration(stack.id, id, in_water, on_ground);
                if break_duration.is_infinite() {
                    // Do nothing, the block is unbreakable.
                } else if break_duration == 0.0 {
                    world.break_block(pos);
                } else {
                    self.breaking_block = Some(BreakingBlock {
                        start_time: world.get_time(), // + (break_duration * 0.7) as u64,
                        pos,
                        id,
                    });
                }

            }

        } else if packet.status == 2 {
            // Block breaking should be finished.
            if let Some(state) = self.breaking_block.take() {
                if state.pos == pos && world.is_block(pos, state.id) {
                    let break_duration = world.get_break_duration(stack.id, state.id, in_water, on_ground);
                    let min_time = state.start_time + (break_duration * 0.7) as u64;
                    if world.get_time() >= min_time {
                        world.break_block(pos);
                    } else {
                        warn!("from {}, incoherent break time, expected {min_time} but got {}", self.username, world.get_time());
                    }
                } else {
                    warn!("from {}, incoherent break position, expected  {}, got {}", self.username, pos, state.pos);
                }
            }
        } else if packet.status == 4 {
            // Drop the selected item.

            if !stack.is_empty() {
                
                stack.size -= 1;
                self.main_inv[self.hand_slot as usize] = stack.to_non_empty().unwrap_or_default();
                
                self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                    window_id: 0,
                    slot: 36 + self.hand_slot as i16,
                    stack: stack.to_non_empty(),
                }));

                self.drop_stack(world, stack.with_size(1), false);

            }

        }

    }

    /// Handle a place block packet.
    fn handle_place_block(&mut self, world: &mut World, packet: proto::PlaceBlockPacket) {
        
        let face = match packet.direction {
            0 => Some(Face::NegY),
            1 => Some(Face::PosY),
            2 => Some(Face::NegZ),
            3 => Some(Face::PosZ),
            4 => Some(Face::NegX),
            5 => Some(Face::PosX),
            0xFF => None,
            _ => return,
        };

        let pos = IVec3 {
            x: packet.x,
            y: packet.y as i32,
            z: packet.z,
        };

        let mut inv = InventoryHandle::new(&mut self.main_inv[..]);
        let inv_index = self.hand_slot as usize;

        // Check if the player is reasonably near the block.
        if face.is_none() || self.pos.distance_squared(pos.as_dvec3() + 0.5) < 64.0 {
            // The real action depends on 
            if let Some(face) = face {
                match world.interact_block(pos) {
                    Interaction::None => {
                        // No interaction, use the item at that block.
                        world.use_stack(&mut inv, inv_index, pos, face, self.entity_id);
                    }
                    Interaction::CraftingTable { pos } => {
                        return self.open_window(world, WindowKind::CraftingTable { pos });
                    }
                    Interaction::Chest { pos } => {
                        return self.open_window(world, WindowKind::Chest { pos });
                    }
                    Interaction::Furnace { pos } => {
                        return self.open_window(world, WindowKind::Furnace { pos });
                    }
                    Interaction::Dispenser { pos } => {
                        return self.open_window(world, WindowKind::Dispenser { pos });
                    }
                    Interaction::Handled => {}
                }
            } else {
                world.use_raw_stack(&mut inv, inv_index, self.entity_id);
            }
        }

        for index in inv.iter_changes() {
            self.send_main_inv_item(index);
        }

    }

    /// Handle a hand slot packet.
    fn handle_hand_slot(&mut self, world: &mut World, slot: i16) {
        if slot >= 0 && slot < 9 {

            // If the previous item was a fishing rod, then we ensure that the bobber id
            // is unset from the player's entity, so that the bobber will be removed.
            let prev_stack = self.main_inv[self.hand_slot as usize];
            if prev_stack.size != 0 && prev_stack.id == item::FISHING_ROD {
                if prev_stack.id == item::FISHING_ROD {

                    let Entity(base, _) = world.get_entity_mut(self.entity_id).expect("incoherent player entity");
                    base.bobber_id = None;
                    
                }
            }

            self.hand_slot = slot as u8;

        } else {
            warn!("from {}, invalid hand slot: {slot}", self.username);
        }
    }

    /// Handle a window click packet.
    fn handle_window_click(&mut self, world: &mut World, packet: proto::WindowClickPacket) {

        // Holding the target slot's item stack.
        let mut cursor_stack = self.cursor_stack;
        let slot_stack;
        let slot_notify;

        // Check coherency of server/client windows.
        if self.window.id != packet.window_id {
            warn!("from {}, incoherent window id, expected {}, got {} from client", self.username, self.window.id, packet.window_id);
            return;
        }

        if packet.slot == -999 {
            slot_stack = ItemStack::EMPTY;
            slot_notify = SlotNotify::None;
            if !cursor_stack.is_empty() {

                let mut drop_stack = cursor_stack;
                if packet.right_click {
                    drop_stack = drop_stack.with_size(1);
                }

                cursor_stack.size -= drop_stack.size;
                self.drop_stack(world, drop_stack, false);

            }
        } else if packet.shift_click {
            
            if packet.slot < 0 {
                return;
            }

            // TODO: This whole branch should be reworked to use a similar approach to
            // regular clicks. One idea would be to have some kind of "SlotTransfer"
            // structure that references targets for transfers, like current "SlotHandle".

            let slot = packet.slot as usize;
            
            // Try to get the main slot, if any.
            let main_slot = match self.window.kind {
                WindowKind::Player => slot.checked_sub(9),
                WindowKind::CraftingTable { .. } => slot.checked_sub(10),
                WindowKind::Chest { ref pos } => slot.checked_sub(pos.len() * 27),
                WindowKind::Furnace { .. } => slot.checked_sub(3),
                WindowKind::Dispenser { .. } => slot.checked_sub(9),
            };

            // From the slot number, we get the index in the main inventory stacks.
            // The the main slot is set by invalid, just abort.
            let main_index = match main_slot {
                Some(n @ 0..=26) => Some(n + 9),
                Some(n @ 27..=35) => Some(n - 27),
                Some(_) => return,
                _ => None
            };
            
            // Create a handle to the main inventory.
            let mut main_inv = InventoryHandle::new(&mut self.main_inv[..]);

            // Each window kind has a different handling of shift click...
            match self.window.kind {
                WindowKind::Player => {
                    if let Some(main_index) = main_index {
                        // Between hotbar and inventory...
                        slot_stack = main_inv.get(main_index);
                        let mut stack = slot_stack;
                        main_inv.push_front_in(&mut stack, if main_index < 9 { 9..36 } else { 0..9 });
                        main_inv.set(main_index, stack);
                        slot_notify = SlotNotify::None;
                    } else if slot == 0 {

                        // Craft result
                        if let Some(mut result_stack) = self.craft_tracker.recipe() {
                            slot_stack = result_stack;
                            if main_inv.can_push(result_stack) {

                                self.craft_tracker.consume(&mut self.craft_inv);

                                main_inv.push_back_in(&mut result_stack, 0..9);
                                main_inv.push_back_in(&mut result_stack, 9..36);
                                assert!(result_stack.is_empty());

                                slot_notify = SlotNotify::Craft { 
                                    mapping: Some(&[1, 2, -1, 3, 4, -1, -1, -1, -1]), 
                                    modified: true,
                                };

                            } else {
                                slot_notify = SlotNotify::None;
                            }
                        } else {
                            slot_stack = ItemStack::EMPTY;
                            slot_notify = SlotNotify::None;
                        }

                    } else if slot >= 1 && slot <= 4 {

                        // Craft matrix
                        let stack = match slot {
                            1 | 2 => &mut self.craft_inv[slot - 1],
                            3 | 4 => &mut self.craft_inv[slot],
                            _ => unreachable!()
                        };

                        slot_stack = *stack;
                        main_inv.push_front_in(stack, 9..36);
                        main_inv.push_front_in(stack, 0..9);

                        slot_notify = SlotNotify::Craft { 
                            mapping: None, 
                            modified: true,
                        };

                    } else {
                        // Armor
                        let stack = &mut self.armor_inv[slot - 5];
                        slot_stack = *stack;
                        main_inv.push_front_in(stack, 9..36);
                        main_inv.push_front_in(stack, 0..9);
                        slot_notify = SlotNotify::None;
                    }
                }
                WindowKind::CraftingTable { .. } => {

                    if let Some(main_index) = main_index {
                        // Between hotbar and inventory...
                        slot_stack = main_inv.get(main_index);
                        let mut stack = slot_stack;
                        main_inv.push_front_in(&mut stack, if main_index < 9 { 9..36 } else { 0..9 });
                        main_inv.set(main_index, stack);
                        slot_notify = SlotNotify::None;
                    } else if slot == 0 {

                        // Craft result
                        if let Some(mut result_stack) = self.craft_tracker.recipe() {
                            slot_stack = result_stack;
                            if main_inv.can_push(result_stack) {

                                self.craft_tracker.consume(&mut self.craft_inv);

                                main_inv.push_back_in(&mut result_stack, 0..9);
                                main_inv.push_back_in(&mut result_stack, 9..36);
                                assert!(result_stack.is_empty());

                                slot_notify = SlotNotify::Craft { 
                                    mapping: Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9]), 
                                    modified: true,
                                };

                            } else {
                                slot_notify = SlotNotify::None;
                            }
                        } else {
                            slot_stack = ItemStack::EMPTY;
                            slot_notify = SlotNotify::None;
                        }

                    } else {

                        // Craft matrix
                        let stack = &mut self.craft_inv[slot - 1];

                        slot_stack = *stack;
                        main_inv.push_front_in(stack, 9..36);
                        main_inv.push_front_in(stack, 0..9);

                        slot_notify = SlotNotify::Craft { 
                            mapping: None, 
                            modified: true,
                        };

                    }

                }
                WindowKind::Chest { ref pos } => {

                    if let Some(main_index) = main_index {

                        // From hotbar/inventory to chest.
                        slot_stack = main_inv.get(main_index);
                        let mut stack = slot_stack;

                        // Temporarily swap events out to avoid borrowing issues.
                        let mut events = world.swap_events(None);

                        // We try to insert 
                        for &pos in pos {
                            if let Some(BlockEntity::Chest(chest)) = world.get_block_entity_mut(pos) {

                                let mut chest_inv = InventoryHandle::new(&mut chest.inv[..]);
                                chest_inv.push_front(&mut stack);

                                // Push all changes in the chest inventory as world event.
                                if let Some(events) = &mut events {
                                    for index in chest_inv.iter_changes() {
                                        events.push(Event::BlockEntity { 
                                            pos, 
                                            inner: BlockEntityEvent::Storage { 
                                                storage: BlockEntityStorage::Standard(index as u8), 
                                                stack: chest_inv.get(index),
                                            },
                                        });
                                    }
                                }

                                if stack.is_empty() {
                                    break;
                                }

                            }
                        }

                        main_inv.set(main_index, stack);

                        // Swap events back in.
                        world.swap_events(events);
                        // No notify because we handled the events for all chest entities.
                        slot_notify = SlotNotify::None;

                    } else {

                        // From the chest to hotbar/inventory

                        let pos = pos[slot / 27];
                        let Some(BlockEntity::Chest(chest)) = world.get_block_entity_mut(pos) else { 
                            return;
                        };

                        let index = slot % 27;
                        let stack = &mut chest.inv[index];
                        slot_stack = *stack;
                        if !stack.is_empty() {
                            main_inv.push_back_in(stack, 0..9);
                            main_inv.push_back_in(stack, 9..36);
                        }

                        slot_notify = SlotNotify::BlockEntityStorageEvent { 
                            pos, 
                            storage: BlockEntityStorage::Standard(index as u8), 
                            stack: Some(*stack),
                        };

                    }
                    
                }
                WindowKind::Furnace { pos } => {

                    if let Some(main_index) = main_index {
                        // Between hotbar and inventory...
                        slot_stack = main_inv.get(main_index);
                        let mut stack = slot_stack;
                        main_inv.push_front_in(&mut stack, if main_index < 9 { 9..36 } else { 0..9 });
                        main_inv.set(main_index, stack);
                        slot_notify = SlotNotify::None;
                    } else {

                        // From furnace to inventory
                        let Some(BlockEntity::Furnace(furnace)) = world.get_block_entity_mut(pos) else { 
                            return;
                        };

                        let (stack, storage) = match slot {
                            0 => (&mut furnace.input_stack, BlockEntityStorage::FurnaceInput),
                            1 => (&mut furnace.fuel_stack, BlockEntityStorage::FurnaceFuel),
                            2 => (&mut furnace.output_stack, BlockEntityStorage::FurnaceOutput),
                            _ => unreachable!()
                        };

                        slot_stack = *stack;
                        main_inv.push_front_in(stack, 9..36);
                        main_inv.push_front_in(stack, 0..9);

                        slot_notify = SlotNotify::BlockEntityStorageEvent { 
                            pos, 
                            storage, 
                            stack: Some(*stack),
                        };

                    }

                }
                WindowKind::Dispenser { pos } => {

                    // No shift click possible with dispenser, but we check coherency.
                    if let Some(main_index) = main_index {
                        slot_stack = main_inv.get(main_index);
                    } else if let Some(BlockEntity::Dispenser(dispenser)) = world.get_block_entity_mut(pos) {
                        slot_stack = dispenser.inv[slot];
                    } else {
                        // No dispenser block entity??
                        return;
                    }
                    
                    slot_notify = SlotNotify::None;

                }
            }

        } else {

            let slot_handle = self.make_window_slot_handle(world, packet.slot);
            let Some(mut slot_handle) = slot_handle else {
                warn!("from {}, cannot find a handle for slot {} in window {}", self.username, packet.slot, packet.window_id);
                return;
            };

            slot_stack = slot_handle.get_stack();
            let slot_access = slot_handle.get_access();

            if slot_stack.is_empty() {
                if !cursor_stack.is_empty() && slot_access.can_drop(cursor_stack) {
                    
                    let drop_size = if packet.right_click { 1 } else { cursor_stack.size };
                    let drop_size = drop_size.min(slot_handle.max_stack_size());
                    
                    slot_handle.set_stack(cursor_stack.with_size(drop_size));
                    cursor_stack.size -= drop_size;

                }
            } else if cursor_stack.is_empty() {

                // Here the slot is not empty, but the cursor is.
                
                // NOTE: Splitting is equivalent of taking and then drop (half), we check 
                // if the slot would accept that drop by checking validity.
                cursor_stack = slot_stack;
                if packet.right_click && slot_access.can_drop(cursor_stack) {
                    cursor_stack.size = (cursor_stack.size + 1) / 2;
                }

                let mut new_slot_stack = slot_stack;
                new_slot_stack.size -= cursor_stack.size;
                if new_slot_stack.size == 0 {
                    slot_handle.set_stack(ItemStack::EMPTY);
                } else {
                    slot_handle.set_stack(new_slot_stack);
                }

            } else if slot_access.can_drop(cursor_stack) {

                // Here the slot and the cursor are not empty, we check if we can
                // drop some item if compatible, or swap if not.

                let cursor_item = item::from_id(cursor_stack.id);

                if (slot_stack.id, slot_stack.damage) != (cursor_stack.id, cursor_stack.damage) {
                    // Not the same item, we just swap with hand.
                    if cursor_stack.size <= slot_handle.max_stack_size() {
                        slot_handle.set_stack(cursor_stack);
                        cursor_stack = slot_stack;
                    }
                } else {
                    // Same item, just drop some into the existing stack.
                    let max_stack_size = cursor_item.max_stack_size.min(slot_handle.max_stack_size());
                    // Only drop if the stack is not full.
                    if slot_stack.size < max_stack_size {
                        
                        let drop_size = if packet.right_click { 1 } else { cursor_stack.size };
                        let drop_size = drop_size.min(max_stack_size - slot_stack.size);
                        cursor_stack.size -= drop_size;

                        let mut new_slot_stack = slot_stack;
                        new_slot_stack.size += drop_size;
                        slot_handle.set_stack(new_slot_stack);

                    }
                }

            } else if let SlotAccess::Pickup(min_size) = slot_access {

                // This last case is when the slot and the cursor are not empty, but we
                // can't drop the cursor into the slot, in such case we try to pick item.

                if (slot_stack.id, slot_stack.damage) == (cursor_stack.id, cursor_stack.damage) {
                    let cursor_item = item::from_id(cursor_stack.id);
                    if cursor_stack.size < cursor_item.max_stack_size {
                        let available_size = cursor_item.max_stack_size - cursor_stack.size;
                        if available_size >= min_size {
                            let pick_size = slot_stack.size.min(available_size);
                            cursor_stack.size += pick_size;
                            let new_slot_stack = slot_stack.with_size(slot_stack.size - pick_size);
                            slot_handle.set_stack(new_slot_stack.to_non_empty().unwrap_or_default());
                        }
                    }
                }

            }

            slot_notify = slot_handle.notify;

        }

        // Handle notification if the slot has changed.
        match slot_notify {
            SlotNotify::Craft { 
                mapping, 
                modified: true,
            } => {

                self.craft_tracker.update(&self.craft_inv);
                
                self.net.send(self.client, OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                    window_id: packet.window_id,
                    slot: 0,
                    stack: self.craft_tracker.recipe(),
                }));

                if let Some(mapping) = mapping {
                    for (index, &slot) in mapping.iter().enumerate() {
                        if slot >= 0 {
                            self.net.send(self.client, OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                                window_id: packet.window_id,
                                slot,
                                stack: self.craft_inv[index].to_non_empty(),
                            }));
                        }
                    }
                }

            }
            SlotNotify::BlockEntityStorageEvent { 
                pos,
                storage, 
                stack: Some(stack),
            } => {
                world.push_event(Event::BlockEntity { 
                    pos, 
                    inner: BlockEntityEvent::Storage { 
                        storage, 
                        stack,
                    },
                });
            }
            _ => {}
        }
            
        // Answer with a transaction packet that is accepted if the packet's stack is
        // the same as the server's slot stack.
        let accepted = slot_stack.to_non_empty() == packet.stack;
        self.send(OutPacket::WindowTransaction(proto::WindowTransactionPacket {
            window_id: packet.window_id,
            transaction_id: packet.transaction_id,
            accepted,
        }));

        if !accepted {
            warn!("from {}, incoherent item at {} in window {}", self.username, packet.slot, packet.window_id);
        }

        if cursor_stack != self.cursor_stack || !accepted {

            // Send the new cursor item.
            if cursor_stack.size == 0 {
                cursor_stack = ItemStack::EMPTY;
            }

            self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket { 
                window_id: 0xFF,
                slot: -1,
                stack: cursor_stack.to_non_empty(),
            }));

            self.cursor_stack = cursor_stack;

        }

    }

    /// Handle a window close packet, it just forget the current window.
    fn handle_window_close(&mut self, world: &mut World, packet: proto::WindowClosePacket) {
        self.close_window(world, Some(packet.window_id), false);
    }

    fn handle_animation(&mut self, _world: &mut World, _packet: proto::AnimationPacket) {
        // TODO: Send animation to other players.
    }

    /// Handle an entity interaction.
    fn handle_interact(&mut self, world: &mut World, packet: proto::InteractPacket) {
        
        if self.entity_id != packet.player_entity_id {
            warn!("from {}, incoherent interact entity: {}, expected: {}", self.username, packet.player_entity_id, self.entity_id);
        }

        let Some(Entity(target_base, _)) = world.get_entity_mut(packet.target_entity_id) else {
            warn!("from {}, incoherent interact entity target: {}", self.username, packet.target_entity_id);
            return;
        };

        if self.pos.distance_squared(target_base.pos) >= 36.0 {
            warn!("from {}, incoherent interact entity distance", self.username);
            return;
        }

        let hand_stack = self.main_inv[self.hand_slot as usize];

        if packet.left_click {

            // TODO: Critical damage if vel.y < 0

            let damage = item::attack::get_base_damage(hand_stack.id);
            target_base.hurt.push(Hurt {
                damage,
                origin_id: Some(self.entity_id),
            });

        } else {
            
        }

    }

    /// Handle an action packet from the player.
    fn handle_action(&mut self, world: &mut World, packet: proto::ActionPacket) {

        if self.entity_id != packet.entity_id {
            warn!("from {}, incoherent player entity: {}, expected: {}", self.username, packet.entity_id, self.entity_id);
        }

        // A player action is only relevant on human entities, ignore if the player is 
        // bound to any other entity kind.
        let Some(Entity(_, BaseKind::Living(_, LivingKind::Human(human)))) = world.get_entity_mut(self.entity_id) else {
            return;
        };

        match packet.state {
            1 | 2 => {
                human.sneaking = packet.state == 1;
                world.push_event(Event::Entity { id: self.entity_id, inner: EntityEvent::Metadata });
            }
            3 => todo!("wake up..."),
            _ => warn!("from {}, invalid action state: {}", self.username, packet.state)
        }

    }

    /// Open the given window kind on client-side by sending appropriate packet. A new
    /// window id is automatically associated to that window.
    fn open_window(&mut self, world: &mut World, kind: WindowKind) {
        
        // Close any already opened window.
        self.close_window(world, None, true);

        // NOTE: We should never get a window id of 0 because it is the player inventory.
        let window_id = (self.window_count % 100 + 1) as u8;
        self.window_count += 1;
        
        match kind {
            WindowKind::Player => panic!("cannot force open the player window"),
            WindowKind::CraftingTable { .. } => {
                self.send(OutPacket::WindowOpen(proto::WindowOpenPacket {
                    window_id,
                    inventory_type: 1,
                    title: "Crafting".to_string(),
                    slots_count: 9,
                }));
            }
            WindowKind::Chest { ref pos } => {

                self.send(OutPacket::WindowOpen(proto::WindowOpenPacket {
                    window_id,
                    inventory_type: 0,
                    title: if pos.len() <= 1 { "Chest" } else { "Large Chest" }.to_string(),
                    slots_count: (pos.len() * 27) as u8,  // TODO: Checked cast
                }));

                let mut stacks = Vec::new();

                for &pos in pos {
                    if let Some(BlockEntity::Chest(chest)) = world.get_block_entity(pos) {
                        stacks.extend(chest.inv.iter().map(|stack| stack.to_non_empty()));
                    } else {
                        stacks.extend(std::iter::repeat(None).take(27));
                    }
                }

                self.send(OutPacket::WindowItems(proto::WindowItemsPacket {
                    window_id,
                    stacks,
                }));

            }
            WindowKind::Furnace { pos } => {

                self.send(OutPacket::WindowOpen(proto::WindowOpenPacket {
                    window_id,
                    inventory_type: 2,
                    title: format!("Furnace"),
                    slots_count: 3,
                }));
                
                if let Some(BlockEntity::Furnace(furnace)) = world.get_block_entity(pos) {

                    self.send(OutPacket::WindowProgressBar(proto::WindowProgressBarPacket {
                        window_id,
                        bar_id: 0,
                        value: furnace.smelt_ticks as i16,
                    }));

                    self.send(OutPacket::WindowProgressBar(proto::WindowProgressBarPacket {
                        window_id,
                        bar_id: 1,
                        value: furnace.burn_remaining_ticks as i16,
                    }));

                    self.send(OutPacket::WindowProgressBar(proto::WindowProgressBarPacket {
                        window_id,
                        bar_id: 2,
                        value: furnace.burn_max_ticks as i16,
                    }));
    
                    self.send(OutPacket::WindowItems(proto::WindowItemsPacket {
                        window_id,
                        stacks: vec![
                            furnace.input_stack.to_non_empty(),
                            furnace.fuel_stack.to_non_empty(),
                            furnace.output_stack.to_non_empty()
                        ],
                    }));

                }

            }
            WindowKind::Dispenser { pos } => {

                self.send(OutPacket::WindowOpen(proto::WindowOpenPacket {
                    window_id,
                    inventory_type: 3,
                    title: format!("Dispenser"),
                    slots_count: 9,
                }));

                if let Some(BlockEntity::Dispenser(dispenser)) = world.get_block_entity(pos) {
                    self.send(OutPacket::WindowItems(proto::WindowItemsPacket {
                        window_id,
                        stacks: dispenser.inv.iter().map(|stack| stack.to_non_empty()).collect()
                    }));
                }

            }
        };

        self.window.id = window_id;
        self.window.kind = kind;

    }

    /// Close the current window opened by the player. If the window id argument is 
    /// provided, then this will only work if the current server-side window is matching.
    /// The send boolean indicates if a window close packet must also be sent.
    fn close_window(&mut self, world: &mut World, window_id: Option<u8>, send: bool) {
    
        if let Some(window_id) = window_id {
            if self.window.id != window_id {
                return;
            }
        }

        // For any closed inventory, we drop the cursor stack and crafting matrix.
        let mut drop_stacks = Vec::new();
        drop_stacks.extend(self.cursor_stack.take_non_empty());
        for stack in self.craft_inv.iter_mut() {
            drop_stacks.extend(stack.take_non_empty());
        }

        for drop_stack in drop_stacks {
            self.drop_stack(world, drop_stack, false);
        }

        // Closing the player inventory so we clear the crafting matrix.
        if self.window.id == 0 {
            for slot in 1..=4 {
                self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket { 
                    window_id: 0,
                    slot,
                    stack: None,
                }));
            }
        }

        // Reset to the default window.
        self.window.id = 0;
        self.window.kind = WindowKind::Player;

        if send {
            self.send(OutPacket::WindowClose(proto::WindowClosePacket {
                window_id: self.window.id,
            }));
        }

    }

    /// Internal function to create a window slot handle specifically for a player main
    /// inventory slot, the offset of the first player inventory slot is also given.
    fn make_player_window_slot_handle(&mut self, slot: i16, offset: i16) -> Option<SlotHandle<'_>> {

        let index = match slot - offset {
            0..=26 => slot - offset + 9,
            27..=35 => slot - offset - 27,
            _ => return None,
        } as usize;

        Some(SlotHandle {
            kind: SlotKind::Standard { 
                stack: &mut self.main_inv[index],
                access: SlotAccess::PickupDrop, 
                max_size: 64,
            },
            notify: SlotNotify::None
        })

    }

    /// Internal function to create a window slot handle. This handle is temporary and
    /// own two mutable reference to the player itself and the world, it can only work
    /// on the given slot.
    fn make_window_slot_handle<'a>(&'a mut self, world: &'a mut World, slot: i16) -> Option<SlotHandle<'a>> {

        // This avoid temporary cast issues afterward, even if we keep the signed type.
        if slot < 0 {
            return None;
        }

        Some(match self.window.kind {
            WindowKind::Player => {
                match slot {
                    0 => SlotHandle {
                        kind: SlotKind::CraftingResult { 
                            craft_inv: &mut self.craft_inv, 
                            craft_tracker: &mut self.craft_tracker,
                        },
                        notify: SlotNotify::Craft { 
                            mapping: Some(&[1, 2, -1, 3, 4, -1, -1, -1, -1]),
                            modified: false,
                        },
                    },
                    1..=4 => SlotHandle { 
                        kind: SlotKind::Standard { 
                            stack: &mut self.craft_inv[match slot {
                                1 => 0,
                                2 => 1,
                                3 => 3,
                                4 => 4,
                                _ => unreachable!()
                            }], 
                            access: SlotAccess::PickupDrop,
                            max_size: 64,
                        },
                        notify: SlotNotify::Craft {
                            mapping: None,
                            modified: false,
                        },
                    },
                    5..=8 => SlotHandle { 
                        kind: SlotKind::Standard { 
                            stack: &mut self.armor_inv[slot as usize - 5], 
                            access: match slot {
                                5 => SlotAccess::ArmorHelmet,
                                6 => SlotAccess::ArmorChestplate,
                                7 => SlotAccess::ArmorLeggings,
                                8 => SlotAccess::ArmorBoots,
                                _ => unreachable!(),
                            }, max_size: 1,
                        }, 
                        notify: SlotNotify::None,
                    },
                    _ => self.make_player_window_slot_handle(slot, 9)?
                }
            }
            WindowKind::CraftingTable { .. } => {
                match slot {
                    0 => SlotHandle {
                        kind: SlotKind::CraftingResult { 
                            craft_inv: &mut self.craft_inv, 
                            craft_tracker: &mut self.craft_tracker,
                        },
                        notify: SlotNotify::Craft {
                            mapping: Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9]),
                            modified: false,
                        },
                    },
                    1..=9 => SlotHandle { 
                        kind: SlotKind::Standard { 
                            stack: &mut self.craft_inv[slot as usize - 1], 
                            access: SlotAccess::PickupDrop,
                            max_size: 64,
                        },
                        notify: SlotNotify::Craft {
                            mapping: None,
                            modified: false,
                        },
                    },
                    _ => self.make_player_window_slot_handle(slot, 10)?
                }
            }
            WindowKind::Chest { ref pos } => {

                if let Some(&pos) = pos.get(slot as usize / 27) {
                    
                    // Get the chest tile entity corresponding to the clicked slot,
                    // if not found we just ignore.
                    let Some(BlockEntity::Chest(chest)) = world.get_block_entity_mut(pos) else {
                        return None
                    };

                    let index = slot as usize % 27;

                    SlotHandle {
                        kind: SlotKind::Standard { 
                            stack: &mut chest.inv[index],
                            access: SlotAccess::PickupDrop,
                            max_size: 64,
                        },
                        notify: SlotNotify::BlockEntityStorageEvent {
                            pos,
                            storage: BlockEntityStorage::Standard(index as u8),
                            stack: None,
                        },
                    }

                } else {
                    self.make_player_window_slot_handle(slot, pos.len() as i16 * 27)?
                }

            }
            WindowKind::Furnace { pos } => {

                if slot <= 2 {

                    let Some(BlockEntity::Furnace(furnace)) = world.get_block_entity_mut(pos) else {
                        return None
                    };

                    let (stack, access, storage) = match slot {
                        0 => (&mut furnace.input_stack, SlotAccess::PickupDrop, BlockEntityStorage::FurnaceInput),
                        1 => (&mut furnace.fuel_stack, SlotAccess::PickupDrop, BlockEntityStorage::FurnaceFuel),
                        2 => (&mut furnace.output_stack, SlotAccess::Pickup(1), BlockEntityStorage::FurnaceOutput),
                        _ => unreachable!()
                    };

                    SlotHandle {
                        kind: SlotKind::Standard { 
                            stack,
                            access, 
                            max_size: 64,
                        },
                        notify: SlotNotify::BlockEntityStorageEvent { 
                            pos, 
                            storage, 
                            stack: None,
                        },
                    }

                } else {
                    self.make_player_window_slot_handle(slot, 3)?
                }

            }
            WindowKind::Dispenser { pos } => {

                if slot < 9 {

                    let Some(BlockEntity::Dispenser(dispenser)) = world.get_block_entity_mut(pos) else {
                        return None
                    };

                    SlotHandle {
                        kind: SlotKind::Standard { 
                            stack: &mut dispenser.inv[slot as usize], 
                            access: SlotAccess::PickupDrop,
                            max_size: 64,
                        },
                        notify: SlotNotify::BlockEntityStorageEvent { 
                            pos, 
                            storage: BlockEntityStorage::Standard(slot as u8), 
                            stack: None,
                        },
                    }

                } else {
                    self.make_player_window_slot_handle(slot, 9)?
                }

            }
        })

    }

    /// Send the main inventory item at given index to the client.
    fn send_main_inv_item(&self, index: usize) {

        let slot = match index {
            0..=8 => 36 + index,
            _ => index,
        };

        self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
            window_id: 0,
            slot: slot as i16,
            stack: self.main_inv[index].to_non_empty(),
        }));

    }

    /// Drop an item from the player's entity, items are drop in front of the player, but
    /// the `on_ground` argument can be set to true in order to drop item on the ground.
    pub fn drop_stack(&mut self, world: &mut World, stack: ItemStack, on_ground: bool) {

        let Entity(origin_base, _) = world.get_entity_mut(self.entity_id).expect("incoherent player entity");
        
        let entity = e::Item::new_with(|base, item| {

            base.persistent = true;
            base.pos = origin_base.pos;
            base.pos.y += 1.3;  // TODO: Adjust depending on eye height.

            if on_ground {

                let rand_drop_speed = origin_base.rand.next_float() * 0.5;
                let rand_yaw = origin_base.rand.next_float() * std::f32::consts::TAU;

                base.vel.x = (rand_yaw.sin() * rand_drop_speed) as f64;
                base.vel.z = (rand_yaw.cos() * rand_drop_speed) as f64;
                base.vel.y = 0.2;

            } else {

                let drop_speed = 0.3;
                let rand_yaw = base.rand.next_float() * std::f32::consts::TAU;
                let rand_drop_speed = base.rand.next_float() * 0.02;
                let rand_vel_y = (base.rand.next_float() - base.rand.next_float()) * 0.1;

                base.vel.x = (-origin_base.look.x.sin() * origin_base.look.y.cos() * drop_speed) as f64;
                base.vel.z = (origin_base.look.x.cos() * origin_base.look.y.cos() * drop_speed) as f64;
                base.vel.y = (-origin_base.look.y.sin() * drop_speed + 0.1) as f64;
                base.vel.x += (rand_yaw.cos() * rand_drop_speed) as f64;
                base.vel.z += (rand_yaw.sin() * rand_drop_speed) as f64;
                base.vel.y += rand_vel_y as f64;

            }

            item.frozen_time = 40;
            item.stack = stack;
            
        });
        
        world.spawn_entity(entity);

    }

    /// Update the chunks sent to this player.
    pub fn update_chunks(&mut self, world: &World) {

        let (ocx, ocz) = chunk::calc_entity_chunk_pos(self.pos);
        let view_range = 3;

        for cx in (ocx - view_range)..(ocx + view_range) {
            for cz in (ocz - view_range)..(ocz + view_range) {

                if let Some(chunk) = world.get_chunk(cx, cz) {
                    if self.tracked_chunks.insert((cx, cz)) {

                        self.send(OutPacket::ChunkState(proto::ChunkStatePacket {
                            cx, cz, init: true
                        }));

                        let from = IVec3 {
                            x: cx * 16,
                            y: 0,
                            z: cz * 16,
                        };

                        let size = IVec3 { 
                            x: 16, 
                            y: 128, 
                            z: 16,
                        };

                        self.send(OutPacket::ChunkData(new_chunk_data_packet(chunk, from, size)));

                    }
                }

            }
        }

    }

    /// Make this player pickup an item stack, the stack and its size is modified 
    /// regarding the amount actually picked up.
    pub fn pickup_stack(&mut self, stack: &mut ItemStack) {
        
        let mut inv = InventoryHandle::new(&mut self.main_inv[..]);
        inv.push_front(stack);

        // Update the associated slots in the player inventory.
        for index in inv.iter_changes() {
            self.send_main_inv_item(index);
        }

    }

    /// For the given block position, close any window that may be linked to it. This is
    /// usually called when the block entity or crafting table is removed.
    pub fn close_block_window(&mut self, world: &mut World, target_pos: IVec3) {

        let contains = match self.window.kind {
            WindowKind::Player => false,
            WindowKind::Furnace { pos } |
            WindowKind::Dispenser { pos } |
            WindowKind::CraftingTable { pos } => 
                pos == target_pos,
            WindowKind::Chest { ref pos } => 
                pos.iter().any(|&pos| pos == target_pos),
        };

        if contains {
            self.close_window(world, None, true);
        }

    }

    /// If this player has a window opened for the given position, this will update the
    /// displayed storage according to the given storage event.
    pub fn update_block_window_storage(&mut self, target_pos: IVec3, storage: BlockEntityStorage, stack: ItemStack) {
        
        match self.window.kind {
            WindowKind::Chest { ref pos } => {
                if let Some(row) = pos.iter().position(|&pos| pos == target_pos) {

                    if let BlockEntityStorage::Standard(index) = storage {
                        self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                            window_id: self.window.id,
                            slot: row as i16 * 27 + index as i16,
                            stack: stack.to_non_empty(),
                        }));
                    }
                    
                }
            }
            WindowKind::Furnace { pos } => {
                if pos == target_pos {

                    let slot = match storage {
                        BlockEntityStorage::FurnaceInput => 0,
                        BlockEntityStorage::FurnaceFuel => 1,
                        BlockEntityStorage::FurnaceOutput => 2,
                        _ => return,
                    };

                    self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                        window_id: self.window.id,
                        slot,
                        stack: stack.to_non_empty(),
                    }));

                }
            }
            WindowKind::Dispenser { pos } => {
                if pos == target_pos {
                    if let BlockEntityStorage::Standard(index) = storage {

                        self.send(OutPacket::WindowSetItem(proto::WindowSetItemPacket {
                            window_id: self.window.id,
                            slot: index as i16,
                            stack: stack.to_non_empty(),
                        }));

                    }
                }
            }
            _ => {}  // Not handled.
        }
    }

    /// If this player has a window opened for the given position, this will update the
    /// displayed storage according to the given storage event.
    pub fn update_block_window_progress(&mut self, target_pos: IVec3, progress: BlockEntityProgress, value: u16) {
        
        if let WindowKind::Furnace { pos } = self.window.kind {
            if pos == target_pos {

                let bar_id = match progress {
                    BlockEntityProgress::FurnaceSmeltTime => 0,
                    BlockEntityProgress::FurnaceBurnRemainingTime => 1,
                    BlockEntityProgress::FurnaceBurnMaxTime => 2,
                };

                self.send(OutPacket::WindowProgressBar(proto::WindowProgressBarPacket {
                    window_id: self.window.id,
                    bar_id,
                    value: value as i16,
                }));

            }
        }

    }

}

/// A pointer to a slot in an inventory.
struct SlotHandle<'a> {
    /// True if the client is able to drop item into this stack, if not then it can only
    /// pickup the item stack.
    kind: SlotKind<'a>,
    notify: SlotNotify,
}

/// Represent a major slot kind.
enum SlotKind<'a> {
    /// A standard slot referencing a single item stack.
    Standard {
        /// The stack referenced by this slot handle.
        stack: &'a mut ItemStack,
        /// The access kind to this slot.
        access: SlotAccess,
        /// The maximum stack size this slot can accept.
        max_size: u16,
    },
    /// The slot represent a crafting result.
    CraftingResult {
        /// The crafting grid item stacks.
        craft_inv: &'a mut [ItemStack; 9],
        /// The crafting tracker for the player.
        craft_tracker: &'a mut CraftTracker,
    },
}

/// Represent the kind of drop rule to apply to this slot.
#[derive(Clone, Copy)]
enum SlotAccess {
    /// The cursor is able to pickup and drop items into this slot. 
    PickupDrop,
    /// The cursor isn't able to drop items into this slot, it can only pickup. The field
    /// gives the minimum number of items that can be picked up at the same time. 
    /// Typically used for crafting because only a full recipe result can be picked up.
    Pickup(u16),
    /// This slot only accepts helmet armor items.
    ArmorHelmet,
    /// This slot only accepts chestplate armor items.
    ArmorChestplate,
    /// This slot only accepts leggings armor items.
    ArmorLeggings,
    /// This slot only accepts boots armor items.
    ArmorBoots,
}

/// Type of notification that will be triggered when the slot gets modified.
enum SlotNotify {
    /// The modification of the slot has no effect.
    None,
    /// The modification of the slot requires the crafting matrix to be resent.
    /// This should only be used for craft matrix windows, where the craft result is in
    /// slot 0.
    Craft {
        /// For each craft inventory stack a client slot number. If not present, this 
        /// means that the crafting matrix should not be updated. If the slot should not
        /// be sent to the client, then the value must be negative.
        mapping: Option<&'static [i16; 9]>,
        /// True if the craft result should be updated from matrix and resent.
        modified: bool,
    },
    /// A block entity storage event need to be pushed to the world.
    BlockEntityStorageEvent {
        /// The position of the block entity.
        pos: IVec3,
        /// The index of the inventory stack that is modified.
        storage: BlockEntityStorage,
        /// If the stack is actually modified, this is the new item stack at the index.
        stack: Option<ItemStack>,
    }
}

impl<'a> SlotHandle<'a> {

    /// Get the maximum stack size for that slot.
    fn max_stack_size(&self) -> u16 {
        match self.kind {
            SlotKind::Standard { max_size, .. } => max_size,
            SlotKind::CraftingResult { .. } => 64,
        }
    }

    /// Get the access rule to this slot.
    fn get_access(&self) -> SlotAccess {
        match self.kind {
            SlotKind::Standard { access, .. } => access,
            SlotKind::CraftingResult { ref craft_tracker, .. } => 
                SlotAccess::Pickup(craft_tracker.recipe().map(|stack| stack.size).unwrap_or(0)),
        }
    }

    /// Get the stack in this slot.
    fn get_stack(&mut self) -> ItemStack {
        match &self.kind {
            SlotKind::Standard { stack, .. } => **stack,
            SlotKind::CraftingResult { craft_tracker, .. } =>
                craft_tracker.recipe().unwrap_or_default()
        }
    }

    /// Set the stack in this slot, called if `is_valid` previously returned `true`, if
    /// the latter return `false`, this function can only be called with `EMPTY` stack.
    /// 
    /// This function also push the slot changes that happened into `slot_changes` of the
    /// server player temporary vector.
    fn set_stack(&mut self, new_stack: ItemStack) {

        match &mut self.kind {
            SlotKind::Standard { stack, .. } => {
                **stack = new_stack;
            }
            SlotKind::CraftingResult { 
                craft_inv, 
                craft_tracker,
            } => {
                craft_tracker.consume(*craft_inv);
            }
        }

        match &mut self.notify {
            SlotNotify::None => {}
            SlotNotify::Craft { modified, .. } => *modified = true,
            SlotNotify::BlockEntityStorageEvent { stack, .. } => *stack = Some(new_stack),
        }

    }

}

impl SlotAccess {

    fn can_drop(self, stack: ItemStack) -> bool {
        match self {
            SlotAccess::PickupDrop => true,
            SlotAccess::Pickup(_) => false,
            SlotAccess::ArmorHelmet => matches!(stack.id, 
                item::LEATHER_HELMET | 
                item::GOLD_HELMET | 
                item::CHAIN_HELMET | 
                item::IRON_HELMET | 
                item::DIAMOND_HELMET) || stack.id == block::PUMPKIN as u16,
            SlotAccess::ArmorChestplate => matches!(stack.id, 
                item::LEATHER_CHESTPLATE | 
                item::GOLD_CHESTPLATE | 
                item::CHAIN_CHESTPLATE | 
                item::IRON_CHESTPLATE | 
                item::DIAMOND_CHESTPLATE),
            SlotAccess::ArmorLeggings => matches!(stack.id, 
                item::LEATHER_LEGGINGS | 
                item::GOLD_LEGGINGS | 
                item::CHAIN_LEGGINGS | 
                item::IRON_LEGGINGS | 
                item::DIAMOND_LEGGINGS),
            SlotAccess::ArmorBoots => matches!(stack.id, 
                item::LEATHER_BOOTS | 
                item::GOLD_BOOTS | 
                item::CHAIN_BOOTS | 
                item::IRON_BOOTS | 
                item::DIAMOND_BOOTS),
        }
    }

}
