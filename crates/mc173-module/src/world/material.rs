//! Various shortcut methods to directly check block materials.

use glam::IVec3;

use crate::block::material::Material;
use crate::block;
use crate::chunk_cache::ChunkCache;
use super::StdbWorld;


/// Shortcut methods for directly querying and checking a block material and properties.
impl StdbWorld {

    /// Get the block material at given position, defaults to air if no chunk.
    pub fn get_block_material(&self, pos: IVec3, cache: &mut ChunkCache) -> Material {
        self.get_block(pos, cache).map(|(id, _)| block::material::get_material(id)).unwrap_or_default()
    }

    /// Return true if the block at given position can be replaced.
    pub fn is_block_replaceable(&self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        if let Some((id, _)) = self.get_block(pos, cache) {
            block::material::get_material(id).is_replaceable()
        } else {
            false
        }
    }

    /// Return true if the block at position is opaque.
    /// 
    /// FIXME: A lot of calls to this function should instead be for "normal_cube". This
    /// is not exactly the same properties in the Notchian implementation.
    pub fn is_block_opaque_cube(&self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        if let Some((id, _)) = self.get_block(pos, cache) {
            block::material::is_opaque_cube(id)
        } else {
            false
        }
    }

    /// Return true if the block at position is material solid.
    pub fn is_block_solid(&self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        if let Some((id, _)) = self.get_block(pos, cache) {
            block::material::get_material(id).is_solid()
        } else {
            false
        }
    }

    /// Return true if the block at position is air.
    #[inline]
    pub fn is_block_air(&self, pos: IVec3, cache: &mut ChunkCache) -> bool {
        if let Some((id, _)) = self.get_block(pos, cache) {
            id == block::AIR
        } else {
            true
        }
    }

    /// Return true if the block at position is the given one. 
    #[inline]
    pub fn is_block(&self, pos: IVec3, id: u8, cache: &mut ChunkCache) -> bool {
        if let Some((pos_id, _)) = self.get_block(pos, cache) {
            pos_id == id
        } else {
            false  // TODO: id == block::AIR ? because non existing position are air
        }
    }

}
