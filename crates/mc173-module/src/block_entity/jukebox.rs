//! Sign block entity.


#[derive(Debug, Clone, Default)]
pub struct JukeboxBlockEntity {
    /// The record currently playing in the jukebox.
    pub record: u32,
}
