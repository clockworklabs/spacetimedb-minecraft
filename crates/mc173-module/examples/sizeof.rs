//! This example is just used internally to debug structures sizes.

use std::mem::size_of;

pub fn main() {

    println!("mc173::chunk::Chunk: {}", size_of::<mc173::chunk::Chunk>());
    println!("mc173::world::World: {}", size_of::<mc173::world::World>());
    println!("mc173::entity::Entity: {}", size_of::<mc173::entity::Entity>());

}
