//! A Minecraft beta 1.7.3 server in Rust.

use std::sync::{Arc, Mutex};
use autogen::autogen::{connect, ReducerEvent, StdbChunk};
use spacetimedb_sdk::subscribe;
use spacetimedb_sdk::table::TableType;
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;

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
    println!("Received chunk: {}, {}", chunk.x, chunk.z);
    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().worlds[0].world.set_chunk(chunk.x, chunk.z, Arc::new(chunk.chunk.clone().into()));
}

/// Entrypoint!
pub fn main() {

    init_tracing();

    ctrlc::set_handler(|| RUNNING.store(false, Ordering::Relaxed)).unwrap();
    StdbChunk::on_insert(on_chunk_inserted);
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
        let mut s = SERVER.lock().unwrap();
        s.as_mut().unwrap().tick_padded().unwrap();
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
