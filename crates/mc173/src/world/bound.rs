//! Block overlay and colliding bounding box extension for world. This module also 
//! provides methods to ray trace and find colliding boxes in a world.

use std::ops::Add;

use glam::{IVec3, DVec3};

use crate::block_entity::BlockEntity;
use crate::geom::{BoundingBox, Face};
use crate::block;

use super::World;


const PIXEL: f64 = 1.0 / 16.0;
const PIXEL_2: f64 = 2.0 / 16.0;
const PIXEL_3: f64 = 3.0 / 16.0;


/// Methods related to block collisions and overlay bounds and also for ray tracing.
impl World {

    /// Get the colliding boxes for a block, the colliding box will be offset to the 
    /// block's position as needed. Not to confuse with overlay boxes, which are just used
    /// to client side placement rendering, and used server-side to compute ray tracing 
    /// when using items such as bucket.
    pub fn iter_block_colliding_boxes(&self, pos: IVec3, id: u8, metadata: u8) -> BlockCollidingBoxesIter {
        
        // Moving piston is a special case because we inherit the block collisions.
        if id == block::PISTON_MOVING {
            if let Some(BlockEntity::Piston(piston)) = self.get_block_entity(pos) {
                    
                let progress = if piston.extending {
                    1.0 - piston.progress
                } else {
                    piston.progress
                };

                return BlockCollidingBoxesIter { 
                    offset: pos.as_dvec3() - piston.face.delta().as_dvec3() * progress as f64, 
                    id, 
                    metadata, 
                    index: 0
                };
                
            }
        }

        BlockCollidingBoxesIter { 
            offset: pos.as_dvec3(), 
            id, 
            metadata, 
            index: 0 
        }

    }

    /// Get the colliding box for a block, this returns a single bounding box that is an
    /// union between all boxes returned by [`iter_block_colliding_boxes`] iterator.
    /// 
    /// [`iter_block_colliding_boxes`]: Self::iter_block_colliding_boxes
    pub fn get_block_colliding_box(&self, pos: IVec3, id: u8, metadata: u8) -> Option<BoundingBox> {
        let mut iter = self.iter_block_colliding_boxes(pos, id, metadata);
        let mut bb = iter.next()?;
        while let Some(other) = iter.next() {
            bb |= other;
        }
        Some(bb)
    }

    /// Get the exclusion box of a block, this function doesn't take the block metadata.
    /// 
    /// PARITY: The Notchian implementation is terrible because it uses the colliding box
    /// for the exclusion box but with the metadata of the block currently at the 
    /// position, so we fix this in this implementation by just returning a full block
    /// for blocks that usually depends on metadata (such as doors, trapdoors).
    pub fn get_block_exclusion_box(&self, pos: IVec3, id: u8) -> Option<BoundingBox> {

        let bb = match id {
            block::BED => Face::NegY.extrude(0.0, 9.0 / 16.0),
            block::CAKE => Face::NegY.extrude(PIXEL, 0.5),
            block::CACTUS => Face::NegY.extrude(PIXEL, 1.0),
            block::SOULSAND => Face::NegY.extrude(0.0, 1.0 - PIXEL_2),
            block::AIR |
            block::LEVER |
            block::BUTTON |
            block::PORTAL |
            block::WOOD_PRESSURE_PLATE |
            block::STONE_PRESSURE_PLATE |
            block::RAIL |
            block::POWERED_RAIL |
            block::DETECTOR_RAIL |
            block::SIGN |
            block::WALL_SIGN |
            block::WHEAT |
            block::DANDELION |
            block::POPPY |
            block::RED_MUSHROOM |
            block::BROWN_MUSHROOM |
            block::DEAD_BUSH |
            block::SAPLING |
            block::TALL_GRASS |
            block::WATER_MOVING |
            block::WATER_STILL |
            block::LAVA_MOVING |
            block::LAVA_STILL |
            block::REDSTONE |
            block::SUGAR_CANES |
            block::TORCH |
            block::REDSTONE_TORCH |
            block::REDSTONE_TORCH_LIT |
            block::COBWEB => return None,
            _ => BoundingBox::CUBE
        };

        Some(bb + pos.as_dvec3())

    }

    /// Get the overlay box of the block, this overlay is what should be shown client-side
    /// around the block and where the player can click. Unlike colliding boxes, there is
    /// only one overlay box per block.
    /// 
    /// **Note that** liquid blocks returns no box.
    pub fn get_block_overlay_box(&self, pos: IVec3, id: u8, metadata: u8) -> Option<BoundingBox> {

        let bb = match id {
            block::BED => Face::NegY.extrude(0.0, 9.0 / 16.0),
            block::CAKE => BoundingBox::new((1 + metadata * 2) as f64 / 16.0, 0.0, PIXEL, 1.0 - PIXEL, 0.5, 1.0 - PIXEL),
            block::CACTUS => Face::NegY.extrude(PIXEL, 1.0),
            block::REDSTONE => Face::NegY.extrude(0.0, PIXEL),
            block::WOOD_DOOR | 
            block::IRON_DOOR => block::door::get_actual_face(metadata).extrude(0.0, PIXEL_3),
            block::LEVER => {
                let (face, _) = block::lever::get_face(metadata)?;
                if face == Face::NegY {
                    face.extrude(0.25, 0.6)
                } else {
                    face.extrude(5.0 / 16.0, 6.0 / 16.0).inflate(DVec3::new(0.0, PIXEL_2, 0.0))
                }
            }
            block::BUTTON => {
                let face = block::button::get_face(metadata)?;
                let active = block::button::is_active(metadata);
                face.extrude(0.6, if active { PIXEL } else { PIXEL_2 })
                    .inflate(DVec3::new(0.0, -PIXEL, 0.0))
            }
            block::PISTON |
            block::STICKY_PISTON => {
                if block::piston::is_extended(metadata) {
                    block::piston::get_face(metadata)?.extrude(0.0, 12.0 / 16.0)
                } else {
                    BoundingBox::CUBE
                }
            }
            block::PISTON_EXT => block::piston::get_face(metadata)?.extrude(0.0, 0.25),
            block::PISTON_MOVING => return None,  // TODO: Use block entity.
            block::PORTAL => return None,  // TODO: Use surrounding portals to determine
            block::WOOD_PRESSURE_PLATE |
            block::STONE_PRESSURE_PLATE => {
                Face::NegY.extrude(PIXEL, if metadata == 1 { PIXEL / 2.0 } else { PIXEL })
            }
            block::RAIL |
            block::POWERED_RAIL |
            block::DETECTOR_RAIL => {
                // TODO: Use proper metadata functions when implementing rails.
                Face::NegY.extrude(0.0, if metadata >= 2 && metadata <= 5 { 10.0 / 16.0 } else { PIXEL_2 })
            }
            block::SIGN |
            block::WALL_SIGN => return None,  // TODO:
            block::SNOW => {
                let layers = metadata & 7;
                Face::NegY.extrude(0.0, 2.0 * (1.0 + layers as f64) / 16.0)
            }
            block::TRAPDOOR => {
                if block::trapdoor::is_open(metadata) {
                    block::trapdoor::get_face(metadata).extrude(0.0, PIXEL_3)
                } else {
                    Face::NegY.extrude(0.0, PIXEL_3)
                }
            }
            block::WHEAT => Face::NegY.extrude(0.0, 0.25),
            block::DANDELION |
            block::POPPY => Face::NegY.extrude(0.3, 0.6),
            block::RED_MUSHROOM |
            block::BROWN_MUSHROOM => Face::NegY.extrude(0.3, 0.4),
            block::DEAD_BUSH |
            block::SAPLING |
            block::TALL_GRASS => Face::NegY.extrude(0.1, 0.8),
            block::SUGAR_CANES => Face::NegY.extrude(PIXEL_2, 1.0),
            block::WATER_MOVING |
            block::WATER_STILL |
            block::LAVA_MOVING |
            block::LAVA_STILL |
            block::AIR => return None,
            _ => BoundingBox::CUBE,
        };

        Some(bb + pos.as_dvec3())

    }

    /// Iterate over all blocks that are in the bounding box area, this doesn't check for
    /// actual collision with the block's bounding box, it just return all potential 
    /// blocks in the bounding box' area.
    pub fn iter_blocks_in_box(&self, bb: BoundingBox) -> impl Iterator<Item = (IVec3, u8, u8)> + '_ {
        let min = bb.min.floor().as_ivec3();
        let max = bb.max.add(1.0).floor().as_ivec3();
        self.iter_blocks_in(min, max)
    }

    /// Iterate over all bounding boxes in the given area.
    /// *Min is inclusive and max is exclusive.*
    pub fn iter_blocks_boxes_in(&self, min: IVec3, max: IVec3) -> impl Iterator<Item = BoundingBox> + '_ {
        self.iter_blocks_in(min, max).flat_map(|(pos, id, metadata)| {
            self.iter_block_colliding_boxes(pos, id, metadata)
        })
    }

    /// Iterate over all bounding boxes in the given area that are colliding with the 
    /// given one.
    pub fn iter_blocks_boxes_colliding(&self, bb: BoundingBox) -> impl Iterator<Item = BoundingBox> + '_ {
        let min = bb.min.floor().as_ivec3();
        let max = bb.max.add(1.0).floor().as_ivec3();
        self.iter_blocks_boxes_in(min, max)
            .filter(move |block_bb| block_bb.intersects(bb))
    }

    /// Ray trace from an origin point and return the first colliding blocks, either 
    /// entity or block. The fluid argument is used to hit the fluid **source** blocks or
    /// not. The overlay argument is used to select the block overlay box instead of the
    /// block bound box.
    pub fn ray_trace_blocks(&self, origin: DVec3, ray: DVec3, kind: RayTraceKind) -> Option<RayTraceHit> {
        
        let ray_norm = ray.normalize();

        let mut pos = origin;
        let mut block_pos = pos.floor().as_ivec3();
        let stop_pos = origin.add(ray).floor().as_ivec3();

        // Break when an invalid chunk is encountered.
        while let Some((block, metadata)) = self.get_block(block_pos) {

            let bb = match kind {
                RayTraceKind::Colliding => 
                    self.get_block_colliding_box(block_pos, block, metadata),
                RayTraceKind::OverlayWithFluid 
                if matches!(block, block::WATER_MOVING | block::WATER_STILL | block::LAVA_MOVING | block::LAVA_STILL) => 
                    block::fluid::is_source(metadata)
                        .then_some(BoundingBox::CUBE + block_pos.as_dvec3()),
                RayTraceKind::Overlay | RayTraceKind::OverlayWithFluid => 
                    self.get_block_overlay_box(block_pos, block, metadata),
            };

            if let Some(bb) = bb {
                if let Some((new_ray, face)) = bb.calc_ray_trace(origin, ray) {
                    return Some(RayTraceHit {
                        ray: new_ray,
                        pos: block_pos,
                        block,
                        metadata,
                        face,
                    });
                }
            }

            // Reached the last block position, just break!
            if block_pos == stop_pos {
                break;
            }

            // Binary search algorithm of the next adjacent block to check.
            let mut tmp_norm = ray_norm;
            let mut tmp_norm_div_count = 0u8;
            let mut next_block_pos;

            'a: loop {

                pos += tmp_norm;
                next_block_pos = pos.floor().as_ivec3();

                // If we reached another block, tmp norm is divided by two in order to
                // converge toward the nearest block.
                if next_block_pos != block_pos {
                    
                    // If the tmp norm has already been divided 7 times, the norm length
                    // should be 0.015625 and we don't want to go lower.
                    if tmp_norm_div_count >= 6 {
                        break 'a;
                    }

                    tmp_norm_div_count += 1;
                    tmp_norm /= 2.0;

                }

                // The next pos is different, check if it is on a face, or 
                while next_block_pos != block_pos {

                    // We check the delta between current block pos and the next one, we 
                    // check if this new pos is on a face of the current pos.
                    let pos_delta = (next_block_pos - block_pos).abs();

                    // Manhattan distance == 1 means we are on a face, use this pos for 
                    // the next ray trace test.
                    if pos_delta.x + pos_delta.y + pos_delta.z == 1 {
                        break 'a;
                    }

                    // Go backward and try finding a block nearer our current pos.
                    pos -= tmp_norm;
                    next_block_pos = pos.floor().as_ivec3();

                }

            }

            block_pos = next_block_pos;

        }

        None

    }

}


/// Internal iterator implementation for bounding boxes of a block with metadata, we must
/// use an iterator because some blocks have multiple bounding boxes.
pub struct BlockCollidingBoxesIter {
    /// The offset to apply the the colliding box.
    offset: DVec3,
    /// The block id.
    id: u8,
    /// The block metadata.
    metadata: u8,
    /// The index of the bounding box, for example stairs have 2 bounding boxes.
    index: u8,
}

impl Iterator for BlockCollidingBoxesIter {

    type Item = BoundingBox;

    fn next(&mut self) -> Option<Self::Item> {

        let metadata = self.metadata;
        let index = self.index;
        self.index += 1;

        let bb = match (index, self.id) {
            (0, block::CACTUS) => BoundingBox::new(PIXEL, 0.0, PIXEL, 1.0 - PIXEL, 1.0, 1.0 - PIXEL),
            (0, block::CAKE) => BoundingBox::new((1 + metadata * 2) as f64 / 16.0, 0.0, PIXEL, 1.0 - PIXEL, 0.5, 1.0 - PIXEL),
            (0, block::FENCE) => BoundingBox::new(0.0, 0.0, 0.0, 1.0, 1.5, 1.0),
            (0, block::SOULSAND) => BoundingBox::new(0.0, 0.0, 0.0, 1.0, 1.0 - PIXEL_2, 1.0),
            (0, block::BED) => BoundingBox::new(0.0, 0.0, 0.0, 1.0, 9.0 / 16.0, 1.0),
            (0, block::REPEATER | block::REPEATER_LIT) => BoundingBox::new(0.0, 0.0, 0.0, 1.0, PIXEL_2, 1.0),
            (0, block::WOOD_DOOR | block::IRON_DOOR) => block::door::get_actual_face(metadata).extrude(0.0, PIXEL_3),
            (0, block::LADDER) => return block::ladder::get_face(metadata).map(|face| face.extrude(0.0, PIXEL_2)),
            (0, block::SNOW) => {
                let layers = metadata & 7;
                if layers >= 3 {
                    BoundingBox::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
                } else {
                    return None
                }
            }
            (0, block::TRAPDOOR) => {
                if block::trapdoor::is_open(metadata) {
                    block::trapdoor::get_face(metadata).extrude(0.0, PIXEL_3)
                } else {
                    Face::NegY.extrude(0.0, PIXEL_3)
                }
            }
            (0, block::PISTON_MOVING) => return None,
            (0, block::PISTON_EXT) => {
                // The extension plate first.
                block::piston::get_face(metadata)?.extrude(0.0, 0.25)
            }
            (1, block::PISTON_EXT) => {
                // The extension rod second.
                let face = block::piston::get_face(metadata)?;
                face.extrude(6.0 / 16.0, 0.75) - face.delta().as_dvec3() * 0.25
            }
            (0, block::WOOD_STAIR | block::COBBLESTONE_STAIR | block::SLAB) => {
                // Slab and stair bottom piece.
                BoundingBox::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
            }
            (1, block::WOOD_STAIR | block::COBBLESTONE_STAIR) => {
                // The stair top piece (the bottom)
                block::stair::get_face(metadata).extrude(0.0, 0.5)
            }
            (0, block::AIR |
                block::FIRE |
                block::DANDELION | block::POPPY |
                block::WHEAT |
                block::DEAD_BUSH |
                block::RED_MUSHROOM | block::BROWN_MUSHROOM |
                block::TALL_GRASS |
                block::SUGAR_CANES |
                block::WATER_MOVING | block::WATER_STILL |
                block::LAVA_MOVING | block::LAVA_STILL |
                block::PORTAL |
                block::WOOD_PRESSURE_PLATE |
                block::STONE_PRESSURE_PLATE |
                block::RAIL | block::POWERED_RAIL | block::DETECTOR_RAIL |
                block::REDSTONE |
                block::BUTTON |
                block::LEVER |
                block::SIGN | block::WALL_SIGN |
                block::TORCH | block::REDSTONE_TORCH | block::REDSTONE_TORCH_LIT |
                block::COBWEB) => return None,
            // All blocks have a cube bounding box by default.
            (0, _) => BoundingBox::CUBE,
            // After index 1, defaults to None in order to stop iterator.
            _ => return None
        };

        Some(bb + self.offset)

    }

}


/// Describe the kind of ray tracing to make, this describe how blocks are collided.
pub enum RayTraceKind {
    /// The ray trace will be on block colliding boxes.
    Colliding,
    /// The ray trace will be on block overlay boxes.
    Overlay,
    /// The ray trace will be on block overlay boxes including fluid sources.
    OverlayWithFluid,
}

/// Result of a ray trace that hit a block.
#[derive(Debug, Clone)]
pub struct RayTraceHit {
    /// The ray vector that stop on the block.
    pub ray: DVec3,
    /// The position of the block.
    pub pos: IVec3,
    /// The block.
    pub block: u8,
    /// The block metadata.
    pub metadata: u8,
    /// The face of the block.
    pub face: Face,
}
