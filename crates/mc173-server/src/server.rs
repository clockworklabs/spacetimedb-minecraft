//! The network server managing connected players and dispatching incoming packets.

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::io;

use glam::{DVec3, Vec2};

use tracing::{warn, info};
use crate::autogen::{stdb_handle_accept, stdb_handle_lost, StdbClientState, StdbConnectionStatus, StdbEntity, StdbInLoginPacket, StdbServerPlayer, StdbServerWorld, StdbTime, StdbWeather, StdbWorld};
use crate::{autogen, config};
use crate::player::ServerPlayer;
use crate::proto::{self, Network, NetworkEvent, NetworkClient, InPacket, OutPacket};


/// Target tick duration. Currently 20 TPS, so 50 ms/tick.
pub const TICK_DURATION: Duration = Duration::from_millis(50);


/// This structure manages a whole server and its clients, dispatching incoming packets
/// to correct handlers.
pub struct Server {
    /// Packet server handle.
    pub net: Network,
    /// Clients of this server, these structures track the network state of each client.
    pub clients: HashMap<u64, NetworkClient>,
    //// Worlds list.
    // pub worlds: Vec<ServerWorld>,
    //// Offline players
    // offline_players: HashMap<String, OfflinePlayer>,
}

impl Server {

    /// Bind this server's TCP listener to the given address.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {

        info!("server bound to {addr}");

        Ok(Self {
            net: Network::bind(addr)?,
            clients: HashMap::<u64, NetworkClient>::new(),
            // worlds: vec![
            //     ServerWorld::new("overworld"),
            // ],
            // offline_players: HashMap::new(),
        })

    }

    // /// Force save this server and block waiting for all resources to be saved.
    // pub fn save(&mut self) {
    //
    //     for world in &mut self.worlds {
    //         world.save();
    //     }
    //
    // }



    /// Run a single tick on the server network and worlds.
    pub fn tick(&mut self) -> io::Result<()> {

        self.tick_net()?;

        // for world in &mut self.worlds {
        //     world.tick();
        // }

        Ok(())

    }

    /// Tick the network and accept incoming events.
    fn tick_net(&mut self) -> io::Result<()> {

        // Poll all network events.
        while let Some(event) = self.net.poll()? {
            match event {
                NetworkEvent::Accept { client } => 
                    self.handle_accept(client),
                NetworkEvent::Lost { client, error } => 
                    self.handle_lost(client, error),
                NetworkEvent::Packet { client, packet } => 
                    self.handle_packet(client, packet),
            }
        }

        Ok(())

    }

    /// Handle new client accepted by the network.
    fn handle_accept(&mut self, client: NetworkClient) {
        info!("accept client #{}", client.id());
        self.clients.insert(client.id(), client);
        stdb_handle_accept(client.id());
    }

    /// Handle a lost client.
    fn handle_lost(&mut self, client: NetworkClient, error: Option<io::Error>) {
        info!("lost client #{}: {:?}", client.id(), error);
        stdb_handle_lost(client.id(), true);
        // if let StdbClientState::Playing(playing_state) = StdbConnectionStatus::find_by_connection_id(client.id()) {
        //     // If the client was playing, remove it from its world.
        //     let world = &mut self.worlds[playing_state.world_index];
        //     if let Some(swapped_player) = world.handle_player_leave(player_index, true) {
        //         // If a player has been swapped in place of the removed one, update the
        //         // swapped one to point to its new index (and same world).
        //         let state = self.clients.get_mut(&swapped_player.client)
        //             .expect("swapped player should be existing");
        //         *state = ClientState::Playing { world_index, player_index };
        //     }
        // }
    }

    fn handle_packet(&mut self, client: NetworkClient, packet: InPacket) {

        let packet_str = format!("{packet:?}");
        if !packet_str.starts_with("Position") && !packet_str.starts_with("Flying") && !packet_str.starts_with("Look") {
            println!("[{client:?}] Packet: {packet:?}");
        }

        let status = StdbConnectionStatus::find_by_connection_id(client.id()).unwrap_or(
            StdbConnectionStatus {
                connection_id: 0,
                status: StdbClientState::Handshaking,
            }
        );

        // match *self.clients.get(&client).unwrap() {
        match status.status {
            StdbClientState::Handshaking => {
                self.handle_handshaking(client, packet);
            }
            StdbClientState::Playing(playing) => {
                // let world = &mut self.worlds[playing.dimension_id];
                // let player = &mut world.players[playing.player_index];
                ServerPlayer::handle(&self, client.id(), packet);
            }
        }
    }

    /// Handle a packet for a client that is in handshaking state.
    fn handle_handshaking(&mut self, client: NetworkClient, packet: InPacket) {
        match packet {
            InPacket::KeepAlive => {}
            InPacket::Handshake(_) => 
                self.handle_handshake(client),
            InPacket::Login(packet) =>
                self.handle_login(client, packet).unwrap(),
            _ => self.send_disconnect(client, format!("Invalid packet: {packet:?}"))
        }
    }

    /// Handle a handshake from a client that is still handshaking, there is no 
    /// restriction.
    fn handle_handshake(&mut self, client: NetworkClient) {
        self.net.send(client, OutPacket::Handshake(proto::OutHandshakePacket {
            server: "-".to_string(),
        }));
    }

    /// Handle a login after handshake.
    fn handle_login(&mut self, client: NetworkClient, packet: proto::InLoginPacket) -> Result<(), String> {
        if packet.protocol_version != 14 {
            self.send_disconnect(client, format!("Protocol version mismatch!"));
            return Err("Protocol version mismatch!".to_string());
        }

        // let spawn_pos = config::SPAWN_POS;

        // Get the offline player, if not existing we create a new one with the 
        // let offline_player = self.offline_players.entry(packet.username.clone())
        //     .or_insert_with(|| {
        //         let spawn_world = &self.worlds[0];
        //         OfflinePlayer {
        //             world: spawn_world.state.name.clone(),
        //             pos: spawn_pos,
        //             look: Vec2::ZERO,
        //         }
        //     });

        // let (world_index, world) = self.worlds.iter_mut()
        //     .enumerate()
        //     .filter(|(_, world)| world.state.name == offline_player.world)
        //     .next()
        //     .expect("invalid offline player world name");

        // let entity = e::Human::new_with(|base, living, player| {
        //     base.pos = offline_player.pos;
        //     base.look = offline_player.look;
        //     base.persistent = false;
        //     base.can_pickup = true;
        //     living.artificial = true;
        //     living.health = 200;  // FIXME: Lot of HP for testing.
        //     player.username = packet.username.clone();
        // });

        autogen::stdb_handle_login(
            client.id(),
            StdbInLoginPacket {
                protocol_version: packet.protocol_version,
                username: packet.username,
            }
        );

        Ok(())
    }

    pub fn handle_login_result(&mut self, connection_id: u64) {
        let new_player = StdbServerPlayer::find_by_connection_id(connection_id);
        if new_player.is_none() {
            return;
        }
        let new_player = new_player.unwrap();
        let client = self.clients.get(&connection_id).unwrap().clone();
        // let world = self.worlds.get_mut(0).unwrap();
        let entity = StdbEntity::find_by_entity_id(new_player.entity_id.clone()).unwrap();
        let world = StdbWorld::find_by_dimension_id(entity.dimension_id).unwrap();
        let server_world = StdbServerWorld::find_by_dimension_id(entity.dimension_id).unwrap();
        let dvec3 : DVec3 = new_player.spawn_pos.clone().into();

        // NOTE(jdetter): I think this doesn't need to happen anymore
        // world.spawn_entity(entity);
        // world.world.set_entity_player(entity_id, true);

        // Confirm the login by sending same packet in response.
        self.net.send(client, OutPacket::Login(proto::OutLoginPacket {
            entity_id: entity.entity_id,
            random_seed: server_world.seed,
            dimension: world.dimension_id as i8,
        }));

        // The standard server sends the spawn position just after login response.
        self.net.send(client, OutPacket::SpawnPosition(proto::SpawnPositionPacket {
            pos: dvec3.as_ivec3(),
        }));

        // Send the initial position for the client.
        self.net.send(client, OutPacket::PositionLook(proto::PositionLookPacket {
            pos: entity.pos.clone().into(),
            // TODO: try putting the camera at the player's feet
            stance: entity.pos.y + 1.62,
            look: entity.look.into(),
            on_ground: false,
        }));

        // Time must be sent once at login to conclude the login phase.
        self.net.send(client, OutPacket::UpdateTime(proto::UpdateTimePacket {
            // time: world.get_time(),
            time: StdbTime::find_by_id(0).unwrap().time
        }));

        if StdbWeather::find_by_dimension_id(entity.dimension_id).unwrap().weather != autogen::Weather::Clear {
        // if world.world.get_weather() != Weather::Clear {
            self.net.send(client, OutPacket::Notification(proto::NotificationPacket {
                reason: 1,
            }));
        }

        // NOTE(jdetter): This is done in stdb
        // Get the offline player, if not existing we create a new one with the
        // let offline_player = self.offline_players.entry(new_player.username.clone())
        //     .or_insert_with(|| {
        //         let spawn_world = &self.worlds[0];
        //         OfflinePlayer {
        //             world: spawn_world.state.name.clone(),
        //             pos: new_player.spawn_pos.clone().into(),
        //             look: Vec2::ZERO,
        //         }
        //     });

        // NOTE(jdetter): This is done in stdb
        // let (world_index, world) = self.worlds.iter_mut()
        //     .enumerate()
        //     .filter(|(_, world)| world.state.name == offline_player.world)
        //     .next()
        //     .expect("invalid offline player world name");

        // TODO(jdetter): Add support for this when we add entities
        // let entity = e::Human::new_with(|base, living, player| {
        //     base.pos = offline_player.pos;
        //     base.look = offline_player.look;
        //     base.persistent = false;
        //     base.can_pickup = true;
        //     living.artificial = true;
        //     living.health = 200;  // FIXME: Lot of HP for testing.
        //     player.username = new_player.username.clone();
        // });

        // NOTE(jdetter): This is done in stdb
        // Finally insert the player tracker.
        // let server_player = ServerPlayer::new(&self.net, client, entity.entity_id, new_player.username.clone());
        // let player_index = world.handle_player_join(server_player);

        // Replace the previous state with a playing state containing the world and
        // player indices, used to get to the player instance.
        // let previous_state = self.clients.insert(client, ClientState::Playing {
        //     world_index: 0,
        //     player_index,
        // });

        // Just a sanity check...
        // debug_assert_eq!(previous_state, Some(ClientState::Handshaking));

        // TODO: Broadcast chat joining chat message.
    }

    /// Send disconnect (a.k.a. kick) to a client.
    fn send_disconnect(&mut self, client: NetworkClient, reason: String) {
        self.net.send(client, OutPacket::Disconnect(proto::DisconnectPacket {
            reason,
        }))
    }

}

/// Track state of a network client in the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientState {
    /// This client is not yet connected to the world.
    Handshaking,
    /// This client is actually playing into a world.
    Playing {
        /// Index of the world this player is in.
        world_index: usize,
        /// Index of the player within the server world.
        player_index: usize,
    }
}
