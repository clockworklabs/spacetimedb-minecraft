//! Inventory data structure storing item stacks.

use std::ops::Range;
use spacetimedb::{query, spacetimedb};
use crate::item::ItemStack;
use crate::item;

#[spacetimedb(table(public))]
pub struct StdbHandSlot {
    #[primarykey]
    pub player_id: u32,
    pub slot_id: u32,
}

#[spacetimedb(table(public))]
pub struct StdbInventory {
    // The inventory ID can be one of the following things:
    // - player_id: it's the inventory for the player
    // - entity_id: it's the inventory for whatever entity (StorageChests, Furnaces, Dispensers)
    #[primarykey]
    #[autoinc]
    pub inventory_id: u32,
    pub size: u32,
}

#[spacetimedb(table(public))]
#[spacetimedb(index(btree, inventory_id, index))]
pub struct StdbItemStack {
    #[primarykey]
    #[autoinc]
    pub stack_id: u32,
    pub inventory_id: u32,
    pub index: u32,
    pub stack: ItemStack
}

impl StdbInventory {
    /// Get the item stack at the given index.
    pub fn get(inventory_id: u32, index: u32) -> ItemStack {
        let inventory = StdbInventory::filter_by_inventory_id(&inventory_id).expect(
            format!("Could not find inventory with id: {}", inventory_id).as_str());
        if index >= inventory.size {
            panic!("Requested inventory slot outside of inventory: inventory_id={} index={}", inventory_id, index);
        }

        match query!(|stack: StdbItemStack| stack.inventory_id == inventory_id && stack.index == index).next() {
            None => {
                ItemStack {
                    id: 0,
                    size: 0,
                    damage: 0,
                }
            }, Some(stack) => {
                stack.stack
            }
        }
    }

    /// Set the item stack at the given index.
    pub fn set(inventory_id: u32, index: u32, stack: ItemStack) {
        let inventory = StdbInventory::filter_by_inventory_id(&inventory_id).expect(
            format!("Could not find inventory with id: {}", inventory_id).as_str());
        if index >= inventory.size {
            panic!("Requested inventory slot outside of inventory: inventory_id={} index={}",
                   inventory_id, index);
        }

        let existing_stack = query!(|item: StdbItemStack| item.inventory_id == inventory_id && item.index == index).next();
        match existing_stack {
            None => {
                // Insert a new item stack
                StdbItemStack::insert(StdbItemStack {
                    stack_id: 0,
                    inventory_id,
                    index,
                    stack,
                }).expect("Insert item stack violated unique constraint");
            }
            Some(existing_stack) => {
                // Update the existing stack
                StdbItemStack::update_by_stack_id(&existing_stack.stack_id, StdbItemStack {
                    stack_id: existing_stack.stack_id,
                    inventory_id,
                    index,
                    stack,
                });
            }
        }
    }

    /// Add an item to the inventory, starting by the first slots.
    /// 
    /// The given item stack is modified according to the amount of items actually added 
    /// to the inventory, its size will be set to zero if fully consumed.
    pub fn push_front(inventory_id: u32, stack: &mut ItemStack) {
        let inventory = StdbInventory::filter_by_inventory_id(&inventory_id).expect(
            format!("Could not find inventory with id: {}", inventory_id).as_str());
        Self::push(inventory_id, stack, 0..inventory.size, false);
    }

    /// Add an item to the inventory, starting from the last slots.
    /// 
    /// The given item stack is modified according to the amount of items actually added 
    /// to the inventory, its size will be set to zero if fully consumed.
    pub fn push_back(inventory_id: u32, stack: &mut ItemStack) {
        let inventory = StdbInventory::filter_by_inventory_id(&inventory_id).expect(
            format!("Inventory with inventory id not found: {}", inventory_id).as_str());
        Self::push(inventory_id, stack, 0..inventory.size, true);
    }

    /// Same as [`push_front`](Self::push_front), but this work in a slice of inventory.
    pub fn push_front_in(inventory_id: u32, stack: &mut ItemStack, range: Range<u32>) {
        Self::push(inventory_id, stack, range, false);
    }

    /// Same as [`push_back`](Self::push_back), but this work in a slice of inventory.
    pub fn push_back_in(inventory_id: u32, stack: &mut ItemStack, range: Range<u32>) {
        Self::push(inventory_id, stack, range, true);
    }

    /// Add an item to the inventory. The given item stack is modified according to the
    /// amount of items actually added to the inventory, its size will be set to zero if
    /// fully consumed.
    fn push(inventory_id: u32, stack: &mut ItemStack, range: Range<u32>, back: bool) {
        // Do nothing if stack size is 0 or the item is air.
        if stack.is_empty() {
            return;
        }

        let item = item::from_id(stack.id);

        // Only accumulate of stack size is greater than 1.
        if item.max_stack_size > 1 {

            let mut range = range.clone();
            while let Some(index) = if back { range.next_back() } else { range.next() } {
                let mut slot = Self::get(inventory_id, index);
                // If the slot is of the same item and has space left in the stack size.
                if slot.size != 0 && slot.id == stack.id && slot.damage == stack.damage && slot.size < item.max_stack_size {
                    let available = item.max_stack_size - slot.size;
                    let to_add = available.min(stack.size);
                    slot.size += to_add;
                    // Immediately update this slot in stdb
                    Self::set(inventory_id, index, slot);
                    stack.size -= to_add;
                    // NOTE: We requires that size must be less than 64, so the index fit
                    // in the 64 bits of changes integer.
                    // TODO(jdetter): minecraft optimizes inventory updates by only sending the changes
                    // self.changes |= 1 << index;
                    if stack.size == 0 {
                        return;
                    }
                }
            }

        }

        // If we land here, some items are remaining to insert in the empty slots.
        // We can also land here if the item has damage value. We search empty slots.
        let mut range = range.clone();
        while let Some(index) = if back { range.next_back() } else { range.next() } {
            let mut slot = Self::get(inventory_id, index);
            if slot.is_empty() {
                Self::set(inventory_id, index, stack.clone());
                // We found an empty slot, insert the whole remaining stack size.
                // *slot = *stack;
                stack.size = 0;
                // self.changes |= 1 << index;
                return;
            }
        }
        
    }

    /// Test if the given item can be pushed in this inventory. If true is returned, a
    /// call to `push_*` function is guaranteed to fully consume the stack.
    pub fn can_push(inventory_id: u32, mut stack: ItemStack) -> bool {

        // Do nothing if stack size is 0 or the item is air.
        if stack.is_empty() {
            return true;
        }

        let item = item::from_id(stack.id);
        let mut slots = Self::get_inventory_vec(inventory_id);

        for slot in slots {
            if slot.is_empty() {
                return true;
            } else if slot.size != 0 && slot.id == stack.id && slot.damage == stack.damage && slot.size < item.max_stack_size {
                let available = item.max_stack_size - slot.size;
                let to_add = available.min(stack.size);
                stack.size -= to_add;
                if stack.size == 0 {
                    return true;
                }
            }
        }

        false

    }

    fn get_inventory_vec(inventory_id: u32) -> Vec<ItemStack> {
        let inventory = StdbInventory::filter_by_inventory_id(&inventory_id).expect(
            format!("Failed to find inventory with inventory id: {}", inventory_id).as_str()
        );
        let mut slots = Vec::<ItemStack>::new();
        for index in 0..inventory.size {
            let slot = query!(|stack: StdbItemStack| stack.inventory_id == inventory_id && stack.index == index).next();
            if slot.is_none() {
                slots.push(ItemStack {
                    id: 0,
                    size: 0,
                    damage: 0,
                });
            } else {
                slots.push(slot.unwrap().stack);
            }
        }

        slots
    }

    /// Consume the equivalent of the given item stack, returning true if successful.
    pub fn consume(inventory_id: u32, stack: ItemStack) -> bool {
        let mut slots = Self::get_inventory_vec(inventory_id);

        for (index, slot) in slots.iter_mut().enumerate() {
            if slot.id == stack.id && slot.damage == stack.damage && slot.size >= stack.size {
                slot.size -= stack.size;
                Self::set(inventory_id, index as u32, slot.clone());
                // self.changes |= 1 << index;
                return true;
            }
        }
        
        false

    }
}
