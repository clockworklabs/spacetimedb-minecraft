//! Moving piston block entity.

use glam::IVec3;

use crate::world::World;
use crate::geom::Face;


#[derive(Debug, Clone)]
pub struct PistonBlockEntity {
    /// The block id of the moving piston block.
    pub block: u8,
    /// The block metadata of the moving piston block.
    pub metadata: u8,
    /// Face toward the block is moving.
    pub face: Face,
    /// Progress of the move animation.
    pub progress: f32,
    /// True when the piston is extending, false when retracting.
    pub extending: bool,
}

impl Default for PistonBlockEntity {
    fn default() -> Self {
        Self { 
            block: 0, 
            metadata: 0, 
            face: Face::PosY,
            progress: 0.0,
            extending: false,
        }
    }
}

impl PistonBlockEntity {

    pub fn tick(&mut self, world: &mut World, pos: IVec3) {
        let _ = (world, pos);
        // TODO:
    }

}
