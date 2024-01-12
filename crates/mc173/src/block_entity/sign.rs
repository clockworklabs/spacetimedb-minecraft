//! Sign block entity.


#[derive(Debug, Clone, Default)]
pub struct SignBlockEntity {
    /// Text line of this sign block.
    pub lines: Box<[String; 4]>,
}
