//! A thread-based world storage manager with chunk generation support for non-existing
//! chunks. The current implementation use a single worker for region or features 
//! generation and many workers for terrain generation.

use std::collections::HashSet;
use std::collections::hash_map::Entry;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use std::sync::Arc;
use std::thread;
use std::io;

use crossbeam_channel::TryRecvError;
use crossbeam_channel::unbounded;
use crossbeam_channel::{select, bounded, Sender, Receiver, RecvError};

use crate::serde::nbt::NbtError;
use crate::serde::nbt::NbtParseError;
use crate::serde::region::{RegionDir, RegionError};
use crate::world::{ChunkSnapshot, World};
use crate::gen::ChunkGenerator;
use crate::world::Dimension;
use crate::chunk::Chunk;


const POPULATED_NEG_NEG: u8 = 0b0001;
const POPULATED_POS_NEG: u8 = 0b0010;
const POPULATED_NEG_POS: u8 = 0b0100;
const POPULATED_POS_POS: u8 = 0b1000;
const POPULATED_ALL: u8     = 0b1111;
const POPULATED_NEG_X: u8   = POPULATED_NEG_NEG | POPULATED_NEG_POS;
const POPULATED_POS_X: u8   = POPULATED_POS_POS | POPULATED_POS_NEG;
const POPULATED_NEG_Z: u8   = POPULATED_NEG_NEG | POPULATED_POS_NEG;
const POPULATED_POS_Z: u8   = POPULATED_POS_POS | POPULATED_NEG_POS;


/// This structure is a handle around a chunk storage.
pub struct ChunkStorage {
    /// Request sender to storage worker.
    storage_request_sender: Sender<StorageRequest>,
    /// Reply receiver from storage worker.
    storage_reply_receiver: Receiver<ChunkStorageReply>,
    /// Set of requested chunk loads.
    request_load: HashSet<(i32, i32)>,
    /// Set of requested chunk saves.
    request_save: HashSet<(i32, i32)>,
}

/// The storage worker is the entry point where commands arrives, it dispatch terrain
/// generation if needed in order to later 
struct StorageWorker<G: ChunkGenerator> {
    /// The shared generator.
    generator: Arc<G>,
    /// The non-shared state of the generator.
    state: G::State,
    /// An internal world used to generate features after terrain generation of chunks.
    world: World,
    /// Populated status of chunks.
    chunks_populated: HashMap<(i32, i32), u8>,
    /// The region directory to try loading required chunks.
    region_dir: RegionDir,
    /// Request receiver from the handle.
    storage_request_receiver: Receiver<StorageRequest>,
    /// Reply sender to the handle.
    storage_reply_sender: Sender<ChunkStorageReply>,
    /// Request sender to the terrain worker.
    terrain_request_sender: Sender<TerrainRequest>,
    /// Reply receiver from the handle.
    terrain_reply_receiver: Receiver<TerrainReply>,
    /// Internal statistics tracker.
    stats: Arc<Stats>,
}

/// The chunk worker is responsible of generating the biomes and terrain.
struct TerrainWorker<G: ChunkGenerator> {
    /// The shared generator.
    generator: Arc<G>,
    /// The non-shared state of the generator.
    state: G::State,
    /// Request receiver from storage worker.
    terrain_request_receiver: Receiver<TerrainRequest>,
    /// Reply sender to storage worker.
    terrain_reply_sender: Sender<TerrainReply>,
    /// Internal statistics tracker.
    stats: Arc<Stats>,
}

/// Internal statistics about performance of chunk generation and request to load times.
#[derive(Debug, Default)]
struct Stats {
    /// Total duration of gen_terrain, in μs.
    gen_terrain_duration: AtomicU64,
    /// Number of samples added to 'gen_terrain_duration'.
    gen_terrain_count: AtomicU64,
    /// Total duration of gen_features, in μs.
    gen_features_duration: AtomicU64,
    /// Number of samples added to 'gen_features_duration'.
    gen_features_count: AtomicU64,
}

impl ChunkStorage {

    /// Create a new chunk storage backed by the given terrain workers count.
    pub fn new<P, G>(region_dir: P, generator: G, terrain_workers: usize) -> Self
    where
        P: Into<PathBuf>,
        G: ChunkGenerator + Sync + Send + 'static,
    {

        // This channel is unbounded because it may cause deadlock.
        let (
            storage_request_sender,
            storage_request_receiver,
        ) = unbounded();

        // The bound on the reply channel is used to block the storage worker if 
        // the consumer cannot keep up, this has the downside of blocking the whole
        // storage worker and therefore preventing new requests to come.
        let (
            storage_reply_sender,
            storage_reply_receiver,
        ) = bounded(100);

        let (
            terrain_request_sender,
            terrain_request_receiver,
        ) = unbounded();

        let (
            terrain_reply_sender,
            terrain_reply_receiver,
        ) = bounded(100 * terrain_workers);

        let region_dir: PathBuf = region_dir.into();
        let generator = Arc::new(generator);
        let stats = Arc::new(Stats::default());
        
        for i in 0..terrain_workers {

            let worker_generator = Arc::clone(&generator);
            let terrain_request_receiver = terrain_request_receiver.clone();
            let terrain_reply_sender = terrain_reply_sender.clone();
            let worker_stats = Arc::clone(&stats);

            thread::Builder::new()
                .name(format!("Chunk Terrain Worker #{i}"))
                .spawn(move || TerrainWorker {
                    generator: worker_generator,
                    state: G::State::default(),
                    terrain_request_receiver,
                    terrain_reply_sender,
                    stats: worker_stats,
                }.run())
                .unwrap();

        }

        thread::Builder::new()
            .name(format!("Chunk Storage Worker"))
            .spawn(move || StorageWorker {
                generator,
                state: G::State::default(),
                world: World::new(Dimension::Overworld), // Not relevant in worker.
                chunks_populated: HashMap::new(),
                region_dir: RegionDir::new(region_dir),
                storage_request_receiver,
                storage_reply_sender,
                terrain_request_sender,
                terrain_reply_receiver,
                stats,
            }.run())
            .unwrap();

        Self {
            storage_request_sender,
            storage_reply_receiver,
            request_load: HashSet::new(),
            request_save: HashSet::new(),
        }

    }

    /// Request loading of a chunk, that will later be returned by polling this storage.
    pub fn request_load(&mut self, cx: i32, cz: i32) {
        self.request_load.insert((cx, cz));
        self.storage_request_sender.send(StorageRequest::Load { cx, cz })
            .expect("worker should not disconnect while this handle exists");
    }

    /// Request saving of the given chunk snapshot.
    pub fn request_save(&mut self, snapshot: ChunkSnapshot) {
        self.request_save.insert((snapshot.cx, snapshot.cz));
        self.storage_request_sender.send(StorageRequest::Save { snapshot })
            .expect("worker should not disconnect while this handle exists");
    }

    /// Poll without blocking this storage for new reply to requested load and save.
    /// This function returns none if there is not new reply to poll.
    pub fn poll(&mut self) -> Option<ChunkStorageReply> {
        match self.storage_reply_receiver.try_recv() {
            Ok(reply) => {
                match reply {
                    ChunkStorageReply::Load { cx, cz, .. } => self.request_load.remove(&(cx, cz)),
                    ChunkStorageReply::Save { cx, cz, .. } => self.request_save.remove(&(cx, cz)),
                };
                Some(reply)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("worker should not disconnect while this handle exists"),
        }
    }

    /// Number of requested chunk loads pending.
    #[inline]
    pub fn request_load_count(&self) -> usize {
        self.request_load.len()
    }

    /// Number of requested chunk saves pending.
    #[inline]
    pub fn request_save_count(&self) -> usize {
        self.request_save.len()
    }

}

impl<G: ChunkGenerator> StorageWorker<G> {

    fn run(mut self) {
        while let Ok(true) = self.handle() { }
    }

    /// Handle a channel message, return Ok(true) to continue and Ok(false) to stop
    /// thread, this is used to stop if any channel error happens.
    fn handle(&mut self) -> Result<bool, RecvError> {
        Ok(select! {
            recv(self.storage_request_receiver) -> request => 
                self.handle_storage_request(request?),
            recv(self.terrain_reply_receiver) -> reply => 
                self.receive_terrain_reply(reply?),
        })
    }

    fn handle_storage_request(&mut self, request: StorageRequest) -> bool {
        match request {
            StorageRequest::Load { cx, cz } => 
                self.load_or_gen(cx, cz),
            StorageRequest::Save { snapshot } => 
                self.save(&snapshot),
        }
    }

    fn receive_terrain_reply(&mut self, reply: TerrainReply) -> bool {
        match reply {
            TerrainReply::Load { cx, cz, chunk } => 
                self.insert_terrain(cx, cz, chunk),
        }
    }

    /// Internal function to try loading a chunk from region file, if the chunk is not
    /// found, its generation is requested to terrain workers. But if a critical error
    /// is returned by the region file then an error is returned. This avoid overwriting
    /// the chunk later and ruining a possibly recoverable error.
    fn load_or_gen(&mut self, cx: i32, cz: i32) -> bool {
        match self.try_load(cx, cz) {
            Err(err) => {
                // Immediately send error, we don't want to load the chunk if there is
                // an error in the region file, in order to avoid overwriting the error.
                self.storage_reply_sender.send(ChunkStorageReply::Load { cx, cz, res: Err(err) }).is_ok()
            }
            Ok(Some(snapshot)) => {
                // Immediately send the loaded chunk.
                self.storage_reply_sender.send(ChunkStorageReply::Load { cx, cz, res: Ok(snapshot) }).is_ok()
            }
            Ok(None) => {
                // The chunk has not been found in region files, generate it.
                self.request_full(cx, cz);
                true
            }
        }
    }

    /// Try loading a chunk from region file.
    fn try_load(&mut self, cx: i32, cz: i32) -> Result<Option<ChunkSnapshot>, StorageError> {

        // Get the region file but do not create it if not already existing, returning
        // unsupported if not existing.
        let region = match self.region_dir.ensure_region(cx, cz, false) {
            Ok(region) => region,
            Err(RegionError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(err) => return Err(StorageError::Region(err))
        };
        
        // Read the chunk, if it is empty then we return unsupported because we don't
        // have the chunk but it's not really an error.
        let reader = match region.read_chunk(cx, cz) {
            Ok(chunk) => chunk,
            Err(RegionError::EmptyChunk) => return Ok(None),
            Err(err) => return Err(StorageError::Region(err))
        };

        let root_tag = crate::serde::nbt::from_reader(reader)?;
        let mut snapshot = crate::serde::chunk::from_nbt(&root_tag)?;
        let chunk = Arc::get_mut(&mut snapshot.chunk).unwrap();
        
        // Biomes are not serialized in the chunk NBT, so we need to generate it on each
        // chunk load because it may be used for natural entity spawn.
        self.generator.gen_biomes(cx, cz, chunk, &mut self.state);

        Ok(Some(snapshot))

    }

    /// Request full generation of a chunk to terrain workers, in order to fully generate
    /// a chunk, its terrain must be generated along with all of its corner being 
    /// populated by features.
    fn request_full(&mut self, cx: i32, cz: i32) {

        // If the requested chunk already exists but is not fully populated, we only
        // request terrain chunks that are in the missing corners.
        let populated = self.chunks_populated.get(&(cx, cz)).copied().unwrap_or(0);
        assert_ne!(populated, POPULATED_ALL);

        let mut min_cx = cx;
        let mut min_cz = cz;
        let mut max_cx = cx;
        let mut max_cz = cz;

        // Only generate terrain for chunks on corners that are not yet populated.
        if populated & POPULATED_NEG_X != POPULATED_NEG_X {
            min_cx -= 1;
        }
        if populated & POPULATED_POS_X != POPULATED_POS_X {
            max_cx += 1;
        }
        if populated & POPULATED_NEG_Z != POPULATED_NEG_Z {
            min_cz -= 1;
        }
        if populated & POPULATED_POS_Z != POPULATED_POS_Z {
            max_cz += 1;
        }

        // For each chunk that needs to be loaded, we check if its terrain already exists,
        // if not existing then we generate it.
        for terrain_cx in min_cx..=max_cx {
            for terrain_cz in min_cz..=max_cz {
                // If the chunk has not terrain or is not fully populated...
                if let Entry::Vacant(v) = self.chunks_populated.entry((terrain_cx, terrain_cz)) {
                    // Send the request to one of the terrain worker.
                    self.terrain_request_sender.send(TerrainRequest::Load { 
                        cx: terrain_cx, 
                        cz: terrain_cz,
                    }).expect("terrain worker should not disconnect while this worker exists");
                    // Insert 0 as populated, this marks the thread as already requested.
                    v.insert(0);
                }
            }
        }

    }

    /// Insert a terrain chunk that have just been returned by a terrain worker.
    fn insert_terrain(&mut self, cx: i32, cz: i32, chunk: Arc<Chunk>) -> bool {
        
        // Get the current state and check its coherency.
        let populated = self.chunks_populated.get_mut(&(cx, cz))
            .expect("chunk state should be present if terrain has been requested");
        assert_eq!(*populated, 0, "requested terrain chunk should have no populated corner");
        assert!(!self.world.contains_chunk(cx, cz), "requested terrain chunk is already present");

        // Set the chunk in the world.
        self.world.set_chunk(cx, cz, chunk);

        // For each chunk around the current chunk, check if it exists. Component order 
        // is [X][Z]. Using this temporary array avoids too much calls to contains_chunk.
        let mut contains = [[false; 3]; 3];
        contains[1][1] = true;  // We know that our center chunk exists.
        
        // Check all chunks around in order to populate them if needed.
        for dcx in 0..3 {
            for dcz in 0..3 {
                // If the chunk is not the current one (that we know existing). If the 
                // chunk is contained in the world, it also implies that it has a state
                // in the local "chunks_state" map.
                if (dcx, dcz) != (1, 1) {
                    if self.world.contains_chunk(cx + dcx as i32 - 1, cz + dcz as i32 - 1) {
                        // NOTE: Array access should be heavily optimized by compiler.
                        contains[dcx][dcz] = true;
                    }
                }
            }
        }
        
        // for dcz in 0..3 {
        //     print!(" {:03} | ", cz + dcz as i32 - 1);
        //     for dcx in 0..3 {
        //         if contains[dcx][dcz] {
        //             print!("X ");
        //         } else {
        //             print!("  ");
        //         }
        //     }
        //     println!("|");
        // }

        // New populated mask to apply to each chunk. Using this intermediate array 
        // allows us to avoid much calls to "chunks_populated.get_mut".
        let mut new_populated = [[0u8; 3]; 3];

        // Now, for each neg/neg corner chunks (4 possible chunks), check if a 2x2 chunk
        // grid is present for populating.
        for dcx in 0..2 {
            for dcz in 0..2 {

                let mut neighbor_count = 0;
                for neighbor_dcx in 0..2 {
                    for neighbor_dcz in 0..2 {
                        if contains[dcx + neighbor_dcx][dcz + neighbor_dcz] {
                            neighbor_count += 1;
                        }
                    }
                }

                // If that corner contains 4 chunks, we can generate features for the 
                // current chunk, if not we check the next chunk.
                if neighbor_count != 4 {
                    continue;
                }

                let current_cx = cx + dcx as i32 - 1;
                let current_cz = cz + dcz as i32 - 1;

                let start = Instant::now();
                self.generator.gen_features(current_cx, current_cz, &mut self.world, &mut self.state);
                let duration = start.elapsed();
                self.stats.gen_features_duration.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
                self.stats.gen_features_count.fetch_add(1, Ordering::Relaxed);

                new_populated[dcx    ][dcz    ] |= POPULATED_POS_POS;
                new_populated[dcx + 1][dcz    ] |= POPULATED_NEG_POS;
                new_populated[dcx    ][dcz + 1] |= POPULATED_POS_NEG;
                new_populated[dcx + 1][dcz + 1] |= POPULATED_NEG_NEG;

            }
        }

        // Finally update all populated state for each chunk.
        for dcx in 0..3 {
            for dcz in 0..3 {
                let populated_mask = new_populated[dcx][dcz];
                if populated_mask != 0 {
                    
                    let current_cx = cx + dcx as i32 - 1;
                    let current_cz = cz + dcz as i32 - 1;
                    let populated = self.chunks_populated.get_mut(&(current_cx, current_cz))
                        .expect("chunk should be existing at this point");

                    *populated |= populated_mask;

                    // After this, we check if the chunk has been fully populated, if so
                    // we can remove its snapshot and finally return it!
                    if *populated & POPULATED_ALL == POPULATED_ALL {

                        // Remove the populated status to keep coherency because we'll 
                        // remove the chunk from the world.
                        self.chunks_populated.remove(&(current_cx, current_cz));

                        let snapshot = self.world.remove_chunk_snapshot(current_cx, current_cz)
                            .expect("chunk should be existing and snapshot possible");

                        // Immediately save the chunk into its region file!
                        if !self.save(&snapshot) {
                            return false
                        }

                        // Finally return the chunk snapshot!
                        if self.storage_reply_sender.send(ChunkStorageReply::Load { cx, cz, res: Ok(snapshot) }).is_err() {
                            // Directly abort to stop the thread because the handle is dropped.
                            return false;
                        }

                    }

                }
            }
        }

        // // NOTE: Technically this code is atomically wrong because duration and count are
        // // not synchronized, but we don't ware for now.
        // let gen_terrain_duration = self.stats.gen_terrain_duration.load(Ordering::Relaxed) as f32 / 1000000.0;
        // let gen_terrain_count = self.stats.gen_terrain_count.load(Ordering::Relaxed);
        // let gen_features_duration = self.stats.gen_features_duration.load(Ordering::Relaxed) as f32 / 1000000.0;
        // let gen_features_count = self.stats.gen_features_count.load(Ordering::Relaxed);
        // println!("gen_terrain_duration: {} ms (samples: {})", gen_terrain_duration * 1000.0, gen_terrain_count);
        // println!("gen_features_duration: {} ms (samples: {})", gen_features_duration * 1000.0, gen_features_count);

        true

    }

    /// Save a chunk snapshot. Returning false if the reply channel is broken.
    fn save(&mut self, snapshot: &ChunkSnapshot) -> bool {

        let (cx, cz) = (snapshot.cx, snapshot.cz);

        match self.try_save(snapshot) {
            Err(err) => {
                // Immediately send the save error.
                self.storage_reply_sender.send(ChunkStorageReply::Save { cx, cz, res: Err(err) }).is_ok()
            }
            Ok(()) => {
                self.storage_reply_sender.send(ChunkStorageReply::Save { cx, cz, res: Ok(()) }).is_ok()
            }
        }

    }

    /// Save a chunk snapshot and return result about success.
    fn try_save(&mut self, snapshot: &ChunkSnapshot) -> Result<(), StorageError> {

        let (cx, cz) = (snapshot.cx, snapshot.cz);
        let region = self.region_dir.ensure_region(cx, cz, true)?;

        let mut writer = region.write_chunk(cx, cz);
        let root_tag = crate::serde::chunk::to_nbt(snapshot);
        crate::serde::nbt::to_writer(&mut writer, &root_tag)?;
        writer.flush_chunk()?;

        Ok(())

    }

}

impl<G: ChunkGenerator> TerrainWorker<G> {

    fn run(mut self) {
        // Run while the channel is existing, so while associated `StorageWorker` exists.
        while let Ok(request) = self.terrain_request_receiver.recv() {
            match request {
                TerrainRequest::Load { cx, cz } => {

                    let mut chunk = Chunk::new();
                    let chunk_access = Arc::get_mut(&mut chunk).unwrap();
                    
                    let start = Instant::now();
                    self.generator.gen_terrain(cx, cz, chunk_access, &mut self.state);
                    let duration = start.elapsed();
                    self.stats.gen_terrain_duration.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
                    self.stats.gen_terrain_count.fetch_add(1, Ordering::Relaxed);
    
                    // If the channel is disconnected, abort to stop thread.
                    if self.terrain_reply_sender.send(TerrainReply::Load { cx, cz, chunk }).is_err() {
                        break;
                    }

                }
            }
        }
    }

}


enum StorageRequest {
    Load { cx: i32, cz: i32 },
    Save { snapshot: ChunkSnapshot },
}

/// A reply from the storage for a previously requested chunk loading or saving.
pub enum ChunkStorageReply {
    Load { cx: i32, cz: i32, res: Result<ChunkSnapshot, StorageError> },
    Save { cx: i32, cz: i32, res: Result<(), StorageError> },
}

enum TerrainRequest {
    Load { cx: i32, cz: i32 },
}

enum TerrainReply {
    Load { cx: i32, cz: i32, chunk: Arc<Chunk> }
}


/// Error type used together with `RegionResult` for every call on region file methods.
#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("region: {0}")]
    Region(#[from] RegionError),
    #[error("nbt: {0}")]
    Nbt(#[from] NbtError),
    #[error("nbt parse: {0}")]
    NbtParse(#[from] NbtParseError),
}
