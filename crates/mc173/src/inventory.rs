//! Inventory data structure storing item stacks.

use std::ops::Range;

use crate::item::ItemStack;
use crate::item;


/// An inventory handle is used to assists item insertion into inventory. It also record
/// stack indices that have changed and therefore allows selective events.
pub struct InventoryHandle<'a> {
    inv: &'a mut [ItemStack],
    changes: u64,
}

impl<'a> InventoryHandle<'a> {

    /// Construct a new inventory handle to a slice of item stacks. This functions panics
    /// if the given slice is bigger than 64 stacks.
    pub fn new(inv: &'a mut [ItemStack]) -> Self {
        assert!(inv.len() <= 64);
        Self {
            inv,
            changes: 0,
        }
    }

    /// Get the item stack at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> ItemStack {
        self.inv[index]
    }

    /// Set the item stack at the given index.
    #[inline]
    pub fn set(&mut self, index: usize, stack: ItemStack) {
        if self.inv[index] != stack {
            self.inv[index] = stack;
            self.changes |= 1 << index;
        }
    }

    /// Add an item to the inventory, starting by the first slots.
    /// 
    /// The given item stack is modified according to the amount of items actually added 
    /// to the inventory, its size will be set to zero if fully consumed.
    pub fn push_front(&mut self, stack: &mut ItemStack) {
        self.push(stack, 0..self.inv.len(), false);
    }

    /// Add an item to the inventory, starting from the last slots.
    /// 
    /// The given item stack is modified according to the amount of items actually added 
    /// to the inventory, its size will be set to zero if fully consumed.
    pub fn push_back(&mut self, stack: &mut ItemStack) {
        self.push(stack, 0..self.inv.len(), true);
    }

    /// Same as [`push_front`](Self::push_front), but this work in a slice of inventory.
    pub fn push_front_in(&mut self, stack: &mut ItemStack, range: Range<usize>) {
        self.push(stack, range, false);
    }

    /// Same as [`push_back`](Self::push_back), but this work in a slice of inventory.
    pub fn push_back_in(&mut self, stack: &mut ItemStack, range: Range<usize>) {
        self.push(stack, range, true);
    }

    /// Add an item to the inventory. The given item stack is modified according to the
    /// amount of items actually added to the inventory, its size will be set to zero if
    /// fully consumed.
    fn push(&mut self, stack: &mut ItemStack, range: Range<usize>, back: bool) {

        // Do nothing if stack size is 0 or the item is air.
        if stack.is_empty() {
            return;
        }

        let item = item::from_id(stack.id);

        // Only accumulate of stack size is greater than 1.
        if item.max_stack_size > 1 {

            let mut range = range.clone();
            while let Some(index) = if back { range.next_back() } else { range.next() } {
                let slot = &mut self.inv[index];
                // If the slot is of the same item and has space left in the stack size.
                if slot.size != 0 && slot.id == stack.id && slot.damage == stack.damage && slot.size < item.max_stack_size {
                    let available = item.max_stack_size - slot.size;
                    let to_add = available.min(stack.size);
                    slot.size += to_add;
                    stack.size -= to_add;
                    // NOTE: We requires that size must be less than 64, so the index fit
                    // in the 64 bits of changes integer.
                    self.changes |= 1 << index;
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
            let slot = &mut self.inv[index];
            if slot.is_empty() {
                // We found an empty slot, insert the whole remaining stack size.
                *slot = *stack;
                stack.size = 0;
                self.changes |= 1 << index;
                return;
            }
        }
        
    }

    /// Test if the given item can be pushed in this inventory. If true is returned, a
    /// call to `push_*` function is guaranteed to fully consume the stack.
    pub fn can_push(&self, mut stack: ItemStack) -> bool {

        // Do nothing if stack size is 0 or the item is air.
        if stack.is_empty() {
            return true;
        }

        let item = item::from_id(stack.id);

        for slot in &self.inv[..] {
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

    /// Consume the equivalent of the given item stack, returning true if successful.
    pub fn consume(&mut self, stack: ItemStack) -> bool {
        
        for (index, slot) in self.inv.iter_mut().enumerate() {
            if slot.id == stack.id && slot.damage == stack.damage && slot.size >= stack.size {
                slot.size -= stack.size;
                self.changes |= 1 << index;
                return true;
            }
        }
        
        false

    }

    /// Get an iterator for changes that happened in this inventory.
    pub fn iter_changes(&self) -> ChangesIter {
        ChangesIter {
            changes: self.changes,
            count: 0,
        }
    }

}


/// An iterator of changes that happened to an inventory.
pub struct ChangesIter {
    changes: u64,
    count: u8,
}

impl Iterator for ChangesIter {

    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        
        while self.count < 64 {
            let ret = ((self.changes & 1) != 0).then_some(self.count as usize);
            self.changes >>= 1;
            self.count += 1;
            if let Some(ret) = ret {
                return Some(ret);
            }
        }

        None

    }

}
