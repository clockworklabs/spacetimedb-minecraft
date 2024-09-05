//! A Minecraft beta 1.7.3 server in Rust.

use clap::{Arg, Command};
use glam::IVec3;
use lazy_static::lazy_static;
use spacetimedb_sdk::identity::Identity;
use spacetimedb_sdk::reducer::Status;
use spacetimedb_sdk::table::TableType;
use spacetimedb_sdk::table::TableWithPrimaryKey;
use spacetimedb_sdk::{subscribe, Address, on_subscription_applied, log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;
use crate::autogen::{connect, on_handle_look, on_handle_position, on_handle_position_look, on_stdb_handle_login, ReducerEvent, StdbChunk, StdbChunkView, StdbEntity, StdbEntityTracker, StdbEntityView, StdbHuman, StdbInLoginPacket, StdbLookPacket, StdbPositionLookPacket, StdbPositionPacket, StdbServerPlayer, StdbSetBlockEvent, StdbWeather};
use crate::player::ServerPlayer;
use crate::proto::{InLoginPacket, OutPacket};
use crate::server::Server;
use crate::types::Event;
use crate::autogen::Weather;

// The common configuration of the server.
pub mod config;

// The network modules, net is generic and proto is the implementation for b1.7.3.
pub mod net;
pub mod proto;

// This modules use each others, this is usually a bad design but here this was too huge
// for a single module and it will be easier to maintain like this.
pub mod command;
mod block;
mod item;
mod geom;
mod craft;

// This module link the previous ones to make a fully functional, multi-world server.
pub mod server;
pub mod io;
mod types;
mod player;
mod world;
mod convert;
mod autogen;
mod chunk;

/// Storing true while the server should run.
static RUNNING: AtomicBool = AtomicBool::new(true);
static READY: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref SERVER: Arc<Mutex<Option<server::Server>>> = Arc::new(Mutex::new(None));
}

fn on_weather_updated(old_weather: &StdbWeather, new_weather: &StdbWeather, _reducer_event: Option<&ReducerEvent>) {
    println!("Received new weather!");
    let mut s = SERVER.lock().unwrap();
    let mut server = s.as_mut().unwrap();

    for player in StdbServerPlayer::iter() {
        let entity = StdbEntity::find_by_entity_id(player.entity_id).unwrap();
        if entity.dimension_id != new_weather.dimension_id {
            continue;
        }

        ServerPlayer::send(server, player.connection_id, OutPacket::Notification(proto::NotificationPacket {
            reason: if new_weather.weather == Weather::Clear { 2 } else { 1 },
        }));
    }
}

fn on_chunk_inserted(chunk: &StdbChunk, _reducer_event: Option<&ReducerEvent>) {
    println!("Received chunk inserted!");
    let mut s = SERVER.lock().unwrap();
    let server = s.as_ref().unwrap();

    // Who needs this chunk?
    for view in StdbChunkView::filter_by_chunk_id(chunk.chunk_id) {
        let player = StdbServerPlayer::find_by_entity_id(view.observer_id).unwrap();
        chunk.send_full(server, player.connection_id);
    }
}

fn on_chunk_update(
    chunk_old: &StdbChunk,
    chunk: &StdbChunk,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("Received chunk update!");
    // TODO(jdetter): reimpl this
    // let mut s = SERVER.lock().unwrap();
    // let mut server = s.as_mut().unwrap();
    // server.worlds[0].world.set_chunk(
    //     chunk.x,
    //     chunk.z,
    //     Arc::new(chunk.chunk.clone().into()),
    // );
    // server.worlds[0].state.chunk_trackers.flush_chunk(chunk.x, chunk.z);
}

fn on_set_block_event_insert(event: &StdbSetBlockEvent, _reducer_event: Option<&ReducerEvent>) {
    let mut s = SERVER.lock().unwrap();
    let mut server = s.as_mut().unwrap();
    // TODO(jdetter): reimpl this
    // server.worlds[0]
    //     .world
    //     .push_set_block_event(event.clone());
}

// fn on_chunk_event(event: &StdbChunkEvent, _reducer_event: Option<&ReducerEvent>) {
//     let mut s = SERVER.lock().unwrap();
//     let mut server = s.as_mut().unwrap();
//     server.worlds[0]
//         .world
//         .push_chunk_event(event.clone());
// }

fn on_entity_view_inserted(
    new_view: &StdbEntityView,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("New entity tracked! observer_id: {} target_id: {}",
    new_view.observer_id, new_view.target_id);
    let mut s = SERVER.lock().unwrap();
    let server = s.as_mut().unwrap();
    stdb_spawn_entity_human(server, new_view.observer_id, new_view.target_id);
}
pub fn stdb_spawn_entity_human(server: &Server, player_observer_id: u32, human_target_id: u32) {
    let observer = StdbServerPlayer::find_by_entity_id(player_observer_id).unwrap();
    let tracker = StdbEntityTracker::find_by_entity_id(human_target_id).unwrap();
    let client = server.clients.get(&observer.connection_id).unwrap();
    let human = StdbHuman::find_by_entity_id(human_target_id).unwrap();
    let metadata = vec![
        proto::Metadata::new_byte(0, (human.sneaking as i8) << 1),
    ];
    server.net.send(client.clone(), OutPacket::HumanSpawn(proto::HumanSpawnPacket {
        entity_id: human.entity_id,
        username: human.username.clone(),
        x: tracker.sent_pos.x,
        y: tracker.sent_pos.y,
        z: tracker.sent_pos.z,
        yaw: tracker.sent_look.x,
        pitch: tracker.sent_look.y,
        // Is this the item they're holding?
        current_item: 0, // TODO:
    }));

    server.net.send(client.clone(), OutPacket::EntityMetadata(proto::EntityMetadataPacket {
        entity_id: human.entity_id,
        metadata,
    }));

}

fn on_entity_view_deleted(
    new_view: &StdbEntityView,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("Entity no longer tracked! observer_id: {} target_id: {}",
             new_view.observer_id, new_view.target_id);
    let mut s = SERVER.lock().unwrap();
    let server = s.as_mut().unwrap();
    stdb_kill_entity(server, new_view.observer_id, new_view.target_id);
}

/// Kill the entity on the player side.
pub fn stdb_kill_entity(server: &Server, player_observer_id: u32, human_target_id: u32) {
    let observer = StdbServerPlayer::find_by_entity_id(player_observer_id).unwrap();
    let client = server.clients.get(&observer.connection_id).unwrap().clone();

    server.net.send(client, OutPacket::EntityKill(proto::EntityKillPacket {
        entity_id: human_target_id
    }));
}

fn on_chunk_view_inserted(
    new_view: &StdbChunkView,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("New chunk tracked! observer_id: {} chunk_id: {}",
             new_view.observer_id, new_view.chunk_id);
    let mut s = SERVER.lock().unwrap();
    let server = s.as_mut().unwrap();
    let chunk = StdbChunk::find_by_chunk_id(new_view.chunk_id).unwrap();
    let player = StdbServerPlayer::find_by_entity_id(new_view.observer_id).unwrap();
    chunk.send_full(server, player.connection_id);
}

fn on_chunk_view_deleted(
    new_view: &StdbChunkView,
    _reducer_event: Option<&ReducerEvent>,
) {
    println!("Chunk no longer tracked! observer_id: {} chunk_id: {}",
             new_view.observer_id, new_view.chunk_id);

    // TODO(jdetter): I think there's nothing to do here, it appears that minecraft doesn't care
    //  when you go out of range of a chunk.
    // let mut s = SERVER.lock().unwrap();
    // let server = s.as_mut().unwrap();
    // stdb_kill_entity(server, new_view.observer_id, new_view.chunk_id);
}

fn on_handle_login_callback(ident: &Identity, _: Option<Address>, _: &Status, connection_id: &u64, packet: &StdbInLoginPacket) {
    let mut s = SERVER.lock().unwrap();
    let server = s.as_mut().unwrap();
    server.handle_login_result(connection_id.clone());
}

fn on_handle_position_callback(ident: &Identity, _addr: Option<Address>, status: &Status,
                               entity_id: &u32, packet: &StdbPositionPacket) {

}

fn on_handle_position_look_callback(ident: &Identity, _addr: Option<Address>, status: &Status,
                                    entity_id: &u32, packet: &StdbPositionLookPacket) {

}

fn on_handle_look_callback(ident: &Identity, _addr: Option<Address>, status: &Status,
                           entity_id: &u32, packet: &StdbLookPacket) {

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
    // StdbServerPlayer::on_insert(on_stdb_server_player_inserted);
    on_handle_position(on_handle_position_callback);
    on_handle_position_look(on_handle_position_look_callback);
    on_handle_look(on_handle_look_callback);
    on_stdb_handle_login(on_handle_login_callback);
    StdbEntityView::on_insert(on_entity_view_inserted);
    StdbEntityView::on_delete(on_entity_view_deleted);
    StdbChunkView::on_insert(on_chunk_view_inserted);
    StdbChunkView::on_delete(on_chunk_view_deleted);
    connect(server, module, None).expect("Failed to connect");
    subscribe(&["SELECT * FROM *"]).unwrap();
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

    // let mut s = SERVER.lock().unwrap();
    // s.as_mut().unwrap().save();
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
