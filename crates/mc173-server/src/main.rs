//! A Minecraft beta 1.7.3 server in Rust.

use std::io;
use std::sync::{Arc, Mutex};
use autogen::autogen::{connect, on_set_block, ReducerEvent, StdbChunk};
use spacetimedb_sdk::{Address, subscribe};
use spacetimedb_sdk::table::TableType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use glam::IVec3;
use lazy_static::lazy_static;
use spacetimedb_sdk::identity::Identity;
use spacetimedb_sdk::reducer::Status;
use tracing::warn;
use spacetimedb_sdk::table::TableWithPrimaryKey;

// The common configuration of the server.
pub mod config;

// The network modules, net is generic and proto is the implementation for b1.7.3.
pub mod net;
pub mod proto;

// This modules use each others, this is usually a bad design but here this was too huge
// for a single module and it will be easier to maintain like this.  
pub mod world;
pub mod chunk;
pub mod entity;
pub mod offline;
pub mod player;
pub mod command;

// This module link the previous ones to make a fully functional, multi-world server.
pub mod server;

/// Storing true while the server should run.
static RUNNING: AtomicBool = AtomicBool::new(true);

lazy_static! {
    static ref SERVER: Arc<Mutex<Option<server::Server>>> = Arc::new(Mutex::new(None));
}

fn on_chunk_inserted(chunk: &StdbChunk, _reducer_event: Option<&ReducerEvent>) {
    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().worlds[0].world.set_chunk(chunk.x, chunk.z, Arc::new(chunk.chunk.clone().into()));
}

fn on_chunk_update(chunk_old: &StdbChunk, chunk: &StdbChunk, _reducer_event: Option<&ReducerEvent>) {
    println!("On update called!");
    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().worlds[0].world.set_chunk(chunk.x, chunk.z, Arc::new(chunk.chunk.clone().into()));

}

fn on_block_set(_sender_id: &Identity, _sender_address: Option<Address>,
                status: &Status, pos_x: &i32, pos_y: &i32, pos_z: &i32, id: &u8, metadata: &u8) {
    let mut s = SERVER.lock().unwrap();
    let pos = IVec3::new(*pos_x, *pos_y, *pos_z);
    s.as_mut().unwrap().worlds[0].world.notify_block_2(pos, *id, *metadata);
}

/// Entrypoint!
pub fn main() {

    init_tracing();

    ctrlc::set_handler(|| RUNNING.store(false, Ordering::Relaxed)).unwrap();
    StdbChunk::on_insert(on_chunk_inserted);
    StdbChunk::on_update(on_chunk_update);
    on_set_block(on_block_set);
    connect(
        "ws://localhost:3000",
        "spacetimedb-minecraft",
        None,
    ).expect("Failed to connect");
    subscribe(&["SELECT * FROM StdbChunk;"]).unwrap();
    println!("Connected to SpacetimeDB");

    {
        let mut s = SERVER.lock().unwrap();
        *s = Some(server::Server::bind("127.0.0.1:25565".parse().unwrap()).unwrap());
    }

    while RUNNING.load(Ordering::Relaxed) {
        let start = Instant::now();
        {
            let mut s = SERVER.lock().unwrap();
            let server = s.as_mut().unwrap();
            server.tick().unwrap();
        }

        let elapsed = start.elapsed();

        if let Some(missing) = crate::server::TICK_DURATION.checked_sub(elapsed) {
            std::thread::sleep(missing);
        } else {
            warn!("tick take too long {:?}, expected {:?}", elapsed, crate::server::TICK_DURATION);
        }
    }

    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().save();
}

/// Initialize tracing to output into the console.
fn init_tracing() {

    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::EnvFilter;
    use tracing_flame::FlameLayer;

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false);

    let (flame_layer, _) = FlameLayer::with_file("./tracing.folded").unwrap();
    let flame_layer = flame_layer.with_file_and_line(false);
    
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(flame_layer)
        .init();

}
