//! Offline player data.

use glam::{DVec3, Vec2};


/// An offline player defines the saved data of a player that is not connected.
#[derive(Debug)]
pub struct OfflinePlayer {
    /// World name.
    pub world: String,
    /// Last saved position of the player.
    pub pos: DVec3,
    /// Last saved look of the player.
    pub look: Vec2,
}
