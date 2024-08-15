//! A Minecraft beta 1.7.3 server in Rust.

use autogen::autogen::{connect, on_handle_break_block, ReducerEvent, StdbBreakBlockPacket, StdbChunk, StdbChunkEvent, StdbSetBlockEvent, StdbWeather};
use clap::{Arg, Command};
use glam::IVec3;
use lazy_static::lazy_static;
use spacetimedb_sdk::identity::Identity;
use spacetimedb_sdk::reducer::Status;
use spacetimedb_sdk::table::TableType;
use spacetimedb_sdk::table::TableWithPrimaryKey;
use spacetimedb_sdk::{subscribe, Address, on_subscription_applied};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;
use mc173::world::Event;

// The common configuration of the server.
pub mod config;

// The network modules, net is generic and proto is the implementation for b1.7.3.
pub mod net;
pub mod proto;

// This modules use each others, this is usually a bad design but here this was too huge
// for a single module and it will be easier to maintain like this.
pub mod chunk;
pub mod command;
pub mod entity;
pub mod offline;
pub mod player;
pub mod world;

// This module link the previous ones to make a fully functional, multi-world server.
pub mod server;

/// Storing true while the server should run.
static RUNNING: AtomicBool = AtomicBool::new(true);
static READY: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref SERVER: Arc<Mutex<Option<server::Server>>> = Arc::new(Mutex::new(None));
}

fn on_weather_updated(old_weather: &StdbWeather, new_weather: &StdbWeather, _reducer_event: Option<&ReducerEvent>) {
    println!("Received new weather!");
    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().worlds[0].world.push_event(Event::Weather { prev: old_weather.weather.clone().into(), new: new_weather.weather.clone().into() });
}

fn on_chunk_inserted(chunk: &StdbChunk, _reducer_event: Option<&ReducerEvent>) {
    println!("Received chunk inserted!");
    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().worlds[0].world.set_chunk(
        chunk.x,
        chunk.z,
        Arc::new(chunk.chunk.clone().into()),
    );
}

fn on_chunk_update(
    chunk_old: &StdbChunk,
    chunk: &StdbChunk,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("Received chunk update!");
    let mut s = SERVER.lock().unwrap();
    let mut server = s.as_mut().unwrap();
    server.worlds[0].world.set_chunk(
        chunk.x,
        chunk.z,
        Arc::new(chunk.chunk.clone().into()),
    );
    server.worlds[0].state.chunk_trackers.flush_chunk(chunk.x, chunk.z);
}

fn on_set_block_event_insert(event: &StdbSetBlockEvent, _reducer_event: Option<&ReducerEvent>) {
    let mut s = SERVER.lock().unwrap();
    let mut server = s.as_mut().unwrap();
    server.worlds[0]
        .world
        .push_set_block_event(event.clone());
}

fn on_chunk_event(event: &StdbChunkEvent, _reducer_event: Option<&ReducerEvent>) {
    let mut s = SERVER.lock().unwrap();
    let mut server = s.as_mut().unwrap();
    server.worlds[0]
        .world
        .push_chunk_event(event.clone());
}

fn on_subscription_applied_callback() {
    READY.store(true, Ordering::Relaxed);
    println!("Initial subscription!")
}

/// Entrypoint!
pub fn main() {
    let command = Command::new("mc173-server")
        .help_expected(true)
        .arg(
            Arg::new("module")
                .long("module")
                .short('m')
                .required(true)
                .help("The module name to connect to"),
        )
        .arg(
            Arg::new("server")
                .long("server")
                .short('s')
                .required(true)
                .help("The remote server to connect to"),
        );

    let result = command.try_get_matches().unwrap();
    let server = result
        .get_one::<String>("server")
        .unwrap();
    let module = result
        .get_one::<String>("module")
        .unwrap();

    init_tracing();
    // ctrlc::set_handler(|| RUNNING.store(false, Ordering::Relaxed)).unwrap();
    StdbChunk::on_insert(on_chunk_inserted);
    StdbChunk::on_update(on_chunk_update);
    StdbWeather::on_update(on_weather_updated);
    on_subscription_applied(on_subscription_applied_callback);
    StdbSetBlockEvent::on_insert(on_set_block_event_insert);
    connect(server, module, None).expect("Failed to connect");
    subscribe(&["SELECT * FROM StdbChunk", "SELECT * FROM StdbTime", "SELECT * FROM StdbWeather"]).unwrap();
    println!("Connected to SpacetimeDB");

    {
        let mut s = SERVER.lock().unwrap();
        *s = Some(server::Server::bind("127.0.0.1:25565".parse().unwrap()).unwrap());
    }

    while !READY.load(Ordering::Relaxed) { std::thread::sleep(Duration::from_millis(10));}

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
            warn!(
                "tick take too long {:?}, expected {:?}",
                elapsed,
                crate::server::TICK_DURATION
            );
        }
    }

    let mut s = SERVER.lock().unwrap();
    s.as_mut().unwrap().save();
}

/// Initialize tracing to output into the console.
fn init_tracing() {
    use tracing_flame::FlameLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let (flame_layer, _) = FlameLayer::with_file("./tracing.folded").unwrap();
    let flame_layer = flame_layer.with_file_and_line(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(flame_layer)
        .init();
}
