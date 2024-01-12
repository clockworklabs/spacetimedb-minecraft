//! Dispenser block entity.

use crate::rand::JavaRandom;
use crate::item::ItemStack;


#[derive(Debug, Clone, Default)]
pub struct DispenserBlockEntity {
    /// The inventory of the dispenser.
    pub inv: Box<[ItemStack; 9]>,
    /// The dispenser has its own RNG.
    pub rand: JavaRandom,
}

impl DispenserBlockEntity {

    /// Randomly pick a non-empty stack in this dispenser, returning its index if any,
    /// none if there are only empty stacks in the inventory. 
    pub fn pick_random_index(&mut self) -> Option<usize> {

        let mut bound = 0;
        let mut selected_index = None;

        for (index, stack) in self.inv.iter_mut().enumerate() {
            if !stack.is_empty() {
                bound += 1;
                if self.rand.next_int_bounded(bound) == 0 {
                    selected_index = Some(index);
                }
            }
        }

        selected_index

    }
    
}
