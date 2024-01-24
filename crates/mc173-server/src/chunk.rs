//! Chunk tracking.

use std::collections::{HashMap, VecDeque};
use std::time::{Instant, Duration};
use std::sync::Arc;

use glam::IVec3;

use mc173::chunk::{Chunk, self};
use mc173::world::World;

use crate::proto::{OutPacket, self};
use crate::player::ServerPlayer;


/// This data structure contains all chunk trackers for a world. It can be used to 
/// efficiently send block changes to clients.
#[derive(Debug)]
pub struct ChunkTrackers {
    /// Inner mapping from chunk coordinates to tracker.
    inner: HashMap<(i32, i32), ChunkTracker>,
    /// Queue of chunks to be saved, this queue should be sorted by 
    scheduled_saves: VecDeque<(i32, i32, Instant)>,
}

impl ChunkTrackers {

    /// Construct a new chunk tracker map.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            scheduled_saves: VecDeque::new(),
        }
    }

    /// Notify the tracker of a block change to be sent later to players. This will also
    /// mark the chunk dirty.
    pub fn set_block(&mut self, pos: IVec3, block: u8, metadata: u8) {
        
        let (cx, cz) = chunk::calc_chunk_pos_unchecked(pos);
        let tracker = self.inner.entry((cx, cz)).or_default();

        let local_pos = ChunkLocalPos {
            x: (pos.x as u32 & 0b1111) as u8,
            y: (pos.y as u32 & 0b1111111) as u8,
            z: (pos.z as u32 & 0b1111) as u8,
        };

        tracker.set_block(local_pos, block, metadata);

    }

    /// Mark a chunk dirty, to be saved later.
    pub fn set_dirty(&mut self, cx: i32, cz: i32) {
        let tracker = self.inner.entry((cx, cz)).or_default();
        if let Some(instant) = tracker.set_dirty() {
            self.schedule_save(cx, cz, instant);
        }
    }

    /// Internal method to schedule a save in the future at given timestamp, this will
    /// be sorted into the scheduled save queue.
    fn schedule_save(&mut self, cx: i32, cz: i32, instant: Instant) {
        
        // Search the correct sorted (asc) index to insert the given time.
        let index = self.scheduled_saves
            .binary_search_by_key(&instant, |&(_ ,_, i)| i)
            .unwrap_or_else(|index| index);

        self.scheduled_saves.insert(index, (cx, cz, instant));

    }

    /// Update the given player list to send new block changes into account. The given
    /// world is used to get the chunk if it is full of changes.
    pub fn update_players(&mut self, players: &[ServerPlayer], world: &World) {
        for (&(cx, cz), tracker) in &mut self.inner {
            tracker.update_players(cx, cz, players, world);
        }
    }

    /// Get the next chunk to save, if any.
    pub fn next_save(&mut self) -> Option<(i32, i32)> {
        let &(_, _, instant) = self.scheduled_saves.front()?;
        if Instant::now() >= instant {
            let (cx, cz, _) = self.scheduled_saves.pop_front().unwrap();
            self.inner.get_mut(&(cx, cz)).unwrap().dirty = false;
            Some((cx, cz))
        } else {
            None
        }
    }

    /// Force drain the internal save queue, ignoring cool downs.
    pub fn drain_save(&mut self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.scheduled_saves.drain(..).map(|(cx, cz, _)| {
            self.inner.get_mut(&(cx, cz)).unwrap().dirty = false;
            (cx, cz)
        })
    }

}

/// This structure tracks a chunk and record every block set in the chunk, this is used
/// to track blocks being set.
#[derive(Debug, Default)]
struct ChunkTracker {
    /// A list of block set in this chunk, if the number of set blocks go above a given
    /// threshold, the vector can be cleared and `set_blocks_full` set to true in order
    /// to resend only the modified range.
    set_blocks: Vec<ChunkSetBlock>,
    /// Set to true when the whole chunk area can be resent instead of all blocks one by
    /// one.
    set_blocks_full: bool,
    /// The minimum position where blocks have been set in the chunk (inclusive).
    set_blocks_min: ChunkLocalPos,
    /// The maximum position where blocks have been set in the chunk (inclusive).
    set_blocks_max: ChunkLocalPos,
    /// This represent the number of dirty notifications to this chunk.
    dirty: bool,
    /// Current save interval for this chunk, may increase of decrease.
    save_interval: Duration,
    /// Last save interval used for scheduling.
    last_save: Option<Instant>,
}

/// A position structure to store chunk-local coordinates to save space.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct ChunkLocalPos {
    x: u8,
    y: u8,
    z: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ChunkSetBlock {
    pos: ChunkLocalPos,
    block: u8,
    metadata: u8,
}

impl ChunkTracker {

    /// Internally register the given set block, depending on the internal state the 
    /// change may be discarded and the whole modified area may be resent instead.
    fn set_block(&mut self, pos: ChunkLocalPos, block: u8, metadata: u8) {

        // This is the Notchian implementation threshold.
        const FULL_THRESHOLD: usize = 10;

        if !self.set_blocks_full {
            // If the number of set blocks go above a threshold, then we abort and set
            // the full state.
            if self.set_blocks.len() >= FULL_THRESHOLD {
                self.set_blocks_full = true;
                self.set_blocks.clear(); // Can be cleared because useless now.
            } else {
                self.set_blocks.push(ChunkSetBlock { pos, block, metadata });
                // If the list was previously empty, we set min/max to initial pos.
                if self.set_blocks.len() == 1 {
                    self.set_blocks_min = pos;
                    self.set_blocks_max = pos;
                    return;
                }
            }
        }

        self.set_blocks_min.x = self.set_blocks_min.x.min(pos.x);
        self.set_blocks_min.y = self.set_blocks_min.y.min(pos.y);
        self.set_blocks_min.z = self.set_blocks_min.z.min(pos.z);
        
        self.set_blocks_max.x = self.set_blocks_max.x.max(pos.x);
        self.set_blocks_max.y = self.set_blocks_max.y.max(pos.y);
        self.set_blocks_max.z = self.set_blocks_max.z.max(pos.z);

    }

    /// Update the given players by sending them the correct packets to update the player
    /// client side. If the chunk is full of set blocks then the whole area is resent, 
    /// else only individual changes are sent to the players loading the chunk.
    /// 
    /// Once this function has updated all players, all modifications are removed.
    fn update_players(&mut self, cx: i32, cz: i32, players: &[ServerPlayer], world: &World) {

        if self.set_blocks_full {

            let chunk = world.get_chunk(cx, cz).expect("chunk has been removed");
            
            let from = IVec3 { 
                x: cx * 16 + self.set_blocks_min.x as i32, 
                y: self.set_blocks_min.y as i32, 
                z: cz * 16 + self.set_blocks_min.z as i32,
            };

            let size = IVec3 { 
                x: (self.set_blocks_max.x - self.set_blocks_min.x + 1) as i32, 
                y: (self.set_blocks_max.y - self.set_blocks_min.y + 1) as i32, 
                z: (self.set_blocks_max.z - self.set_blocks_min.z + 1) as i32, 
            };

            // trace!("sending partial chunk data for {cx}/{cz}, from {from}, size {size}");

            let packet = OutPacket::ChunkData(new_chunk_data_packet(chunk, from, size));
            for player in players {
                if player.tracked_chunks.contains(&(cx, cz)) {
                    player.send(packet.clone());
                }
            }

        } else if self.set_blocks.len() == 1 {

            let set_block = self.set_blocks[0];
            // trace!("sending single block for {cx}/{cz}, at {:?}", set_block.pos);
            
            for player in players {
                if player.tracked_chunks.contains(&(cx, cz)) {
                    player.send(OutPacket::BlockSet(proto::BlockSetPacket {
                        x: cx * 16 + set_block.pos.x as i32,
                        y: set_block.pos.y as i8,
                        z: cz * 16 + set_block.pos.z as i32,
                        block: set_block.block,
                        metadata: set_block.metadata,
                    }));
                }
            }

        } else if !self.set_blocks.is_empty() {

            let set_blocks = self.set_blocks.iter()
                .map(|set_block| proto::ChunkBlockSet {
                    x: set_block.pos.x,
                    y: set_block.pos.y,
                    z: set_block.pos.z,
                    block: set_block.block,
                    metadata: set_block.metadata,
                })
                .collect();

            let packet = OutPacket::ChunkBlockSet(proto::ChunkBlockSetPacket {
                cx,
                cz,
                blocks: Arc::new(set_blocks),
            });

            // trace!("sending multi block for {cx}/{cz}, count {}", self.set_blocks.len());

            for player in players {
                if player.tracked_chunks.contains(&(cx, cz)) {
                    player.send(packet.clone());
                }
            }

        }

        self.set_blocks_full = false;
        self.set_blocks.clear();

    }

    /// Mark this chunk as dirty, and return some instant if a save should be scheduled
    /// in the future for this chunk.
    fn set_dirty(&mut self) -> Option<Instant> {

        const MAX_INTERVAL: Duration = Duration::from_secs(60);
        const MIN_INTERVAL: Duration = Duration::from_secs(4);
        const INTERVAL_STEP: Duration = Duration::from_secs(4);

        // A save has already been scheduled.
        if self.dirty {
            return None;
        }

        self.dirty = true;

        let now = Instant::now();

        // If a save has already happened in the past.
        if let Some(last_save) = self.last_save {
            // We subtract the interval base in order to possibly reach zero duration and
            // therefore be equal to the initial interval of zero and increase interval.
            let elapsed = now.saturating_duration_since(last_save);
            if elapsed > self.save_interval {
                // If the time elapsed since last scheduled save is greater than interval,
                // then we reduce the interval.
                self.save_interval = self.save_interval.saturating_sub(INTERVAL_STEP);
            } else {
                // If the time elapsed is less than or equal to current interval, we 
                // increase the save interval.
                self.save_interval = self.save_interval.saturating_add(INTERVAL_STEP).min(MAX_INTERVAL);
            }
        }

        if self.save_interval < MIN_INTERVAL {
            self.save_interval = MIN_INTERVAL;
        }

        // Finally compute, store and return next save instant.
        let next_save = now + self.save_interval;
        self.last_save = Some(next_save);
        Some(next_save)

    }

}


/// Create a new chunk data packet for the given chunk. This only works for a single 
/// chunk and the given coordinate should be part of that chunk. The two arguments "from"
/// and "to" are inclusive but might be modified to include more blocks if ths reduces
/// computation.
pub fn new_chunk_data_packet(chunk: &Chunk, mut from: IVec3, mut size: IVec3) -> proto::ChunkDataPacket {

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    debug_assert!(size.x != 0 && size.y != 0 && size.z != 0);
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    chunk.write_data(&mut encoder, &mut from, &mut size).unwrap();

    debug_assert!(size.x != 0 && size.y != 0 && size.z != 0);
    
    proto::ChunkDataPacket {
        x: from.x,
        y: from.y as i16, 
        z: from.z, 
        x_size: size.x as u8, 
        y_size: size.y as u8, 
        z_size: size.z as u8,
        compressed_data: Arc::new(encoder.finish().unwrap()),
    }
    
}
