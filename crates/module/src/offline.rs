//! Offline player data.

use glam::{DVec3, Vec2};
use mc173_module::dvec3::StdbDVec3;
use mc173_module::vec2::StdbVec2;
use spacetimedb::spacetimedb;

/// An offline player defines the saved data of a player that is not connected.
#[derive(Debug)]
#[spacetimedb(table(public))]
pub struct StdbOfflinePlayer {
    // The entity ID of the player
    pub entity_id: u32,
    pub username: String,
    /// World name.
    pub world: String,
    /// Last saved position of the player.
    pub pos: StdbDVec3,
    /// Last saved look of the player.
    pub look: StdbVec2,
}
