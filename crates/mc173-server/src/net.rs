//! Packet server for threaded decoding and encoding of packets.

use std::io::{self, Read, Write, Cursor};
use std::net::{SocketAddr, Shutdown};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::thread;
use std::fmt;

use crossbeam_channel::{bounded, Sender, Receiver, TryRecvError};

use mio::{Poll, Events, Interest, Token};
use mio::net::{TcpListener, TcpStream};
use mio::event::Event;


/// A server-bound packet (received and processed by the server).
pub trait InPacket: Sized {
    /// Read the packet from the writer.
    fn read(read: &mut impl Read) -> io::Result<Self>;
}

/// A client-bound packet (received and processed by the client).
pub trait OutPacket {
    /// Write the packet to the given writer.
    fn write(&self, write: &mut impl Write) -> io::Result<()>;
}


/// A packet server backed by a background thread that do all the hard processing.
/// 
/// To kill the server, every handle of it should be dropped.
#[derive(Debug, Clone)]
pub struct Network<I, O> {
    /// This channels allows sending commands to the thread.
    commands_sender: Sender<ThreadCommand<O>>,
    /// This channels allows received events from the thread.
    events_receiver: Receiver<ThreadEvent<I>>,
}

impl<I, O> Network<I, O>
where
    I: InPacket + Send + 'static,
    O: OutPacket + Send + 'static,
{

    pub fn bind(addr: SocketAddr) -> io::Result<Self> {

        let poll = Poll::new()?;
        let mut listener = TcpListener::bind(addr)?;
        poll.registry().register(&mut listener, LISTENER_TOKEN, Interest::READABLE)?;

        // TODO: Adapt channel sizes depending on number of players, maybe unbounded 
        // queue could be better for our case.

        let (
            commands_sender,
            commands_receiver
        ) = bounded(1000);

        let (
            events_sender,
            events_receiver
        ) = bounded(1000);

        // The poll thread.
        let poll_commands_sender = commands_sender.clone();
        
        thread::Builder::new()
            .name("Packet Poll Thread".to_string())
            .spawn(move || {
                PollThread::<I, O> {
                    commands_sender: poll_commands_sender,
                    events_sender,
                    listener,
                    poll,
                    next_token: CLIENT_FIRST_TOKEN,
                    clients: HashMap::new(),
                }.run();
            }).unwrap();

        // The command thread.
        thread::Builder::new()
            .name("Packet Command Thread".to_string())
            .spawn(move || {
                CommandThread::<O> {
                    commands_receiver,
                    clients: HashMap::new(),
                }.run();
            }).unwrap();

        Ok(Self {
            commands_sender,
            events_receiver,
        })

    }

    /// Poll events from this packet server. If an I/O error is returned, the error is
    /// critical and the 
    pub fn poll(&self) -> io::Result<Option<NetworkEvent<I>>> {
        loop { // A loop to ignore channel check.
            return Ok(Some(match self.events_receiver.try_recv() {
                Ok(ThreadEvent::ChannelCheck) => continue,
                Ok(ThreadEvent::Accept { token }) => NetworkEvent::Accept {
                    client: NetworkClient(token)
                },
                Ok(ThreadEvent::Lost { token, error }) => NetworkEvent::Lost {
                    client: NetworkClient(token),
                    error,
                },
                Ok(ThreadEvent::Packet { token, packet }) => NetworkEvent::Packet {
                    client: NetworkClient(token), 
                    packet,
                },
                // Critical error, this should be the last event of the channel before 
                // disconnection.
                Ok(ThreadEvent::Error { error }) => return Err(error), 
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => 
                    return Err(new_io_abort_error("previous error made this server unusable")),
            }));
        }
    }

    pub fn send(&self, client: NetworkClient, packet: O) {
        // NOTE: Commands channel can never disconnect if a handle exists.
        self.commands_sender.try_send(ThreadCommand::SingleClientPacket { 
            token: client.0, 
            packet
        }).expect("commands channel is full");
    }

    pub fn disconnect(&self, client: NetworkClient) {
        // NOTE: Commands channel can never disconnect if a handle exists.
        self.commands_sender.try_send(ThreadCommand::DisconnectClient {
            token: client.0
        }).expect("commands channel is full");
    }

}

/// A handle to a client produced by a packet server. This handle can be used with a
/// server to send packets to a client.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkClient(Token);

impl fmt::Debug for NetworkClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NetworkClient").field(&self.0.0).finish()
    }
}

impl NetworkClient {

    /// Return the numeric id of this network client, many used for displaying purpose.
    #[inline]
    pub fn id(self) -> u64 {
        self.0.0 as u64
    }

}

/// An event of the packet
#[derive(Debug)]
pub enum NetworkEvent<I> {
    /// A client 
    Accept {
        /// The client handle that was accepted.
        client: NetworkClient,
    },
    Lost {
        /// The client handle that was lost.
        client: NetworkClient,
        /// Some error if that caused the client to be lost, no error means that the
        /// client was just kicked from the server or closed the connection itself.
        error: Option<io::Error>,
    },
    /// A packet was received from a client.
    Packet {
        /// The client handle that received the packet.
        client: NetworkClient,
        /// Received packet.
        packet: I,
    },
}


/// Internal polling token used for the listening socket.
const LISTENER_TOKEN: Token = Token(0);
/// First token associated to a client.
const CLIENT_FIRST_TOKEN: Token = Token(1);
/// Size of internal buffers for incoming client's data.
const BUF_SIZE: usize = 1024;


/// Shared immutable client state.
struct SharedClient {
    /// The client's stream, this stream is behind a read/write lock because most of the
    /// time it will be accessed immutably, because reading/writing from/to the stream
    /// don't requires mutability, the only moment it will be accessed mutably is for
    /// deregister it from poll instance, when closing client.
    stream: RwLock<TcpStream>,
}

/// Internal thread for polling the TCP listener and client events. Polling is done it
/// its own thread because it blocks until events are received, but we also need to block
/// for incoming commands, this would require a sort of *select* between poll events
/// and channel commands, but we can't do that.
struct PollThread<I, O> {
    /// Commands sent to the command thread, to register and deregister 
    commands_sender: Sender<ThreadCommand<O>>,
    /// Events sent to the handle.
    events_sender: Sender<ThreadEvent<I>>,
    /// The inner TCP listener.
    listener: TcpListener,
    /// The poll used for event listening TCP events.
    poll: Poll,
    /// The next token to associate to a client.
    next_token: Token,
    /// All clients.
    clients: HashMap<Token, PollClient>,
}

/// Internal structure for storing client
struct PollClient {
    /// Shared client state.
    shared: Arc<SharedClient>,
    /// Internal buffer to temporarily stores incoming client's data.
    buf: Box<[u8; BUF_SIZE]>,
    /// Cursor in the receiving buffer.
    buf_cursor: usize,
}

impl<I: InPacket, O: OutPacket> PollThread<I, O> {

    fn run(mut self) {
        
        let mut events = Events::with_capacity(100);
        
        // While events channel is not disconnected.
        while self.events_sender.send(ThreadEvent::ChannelCheck).is_ok() {
            if let Err(e) = self.poll(&mut events) {
                // NOTE: We ignore if the channel is disconnected, we terminate anyway.
                let _ = self.events_sender.send(ThreadEvent::Error { error: e });
                return;
            }
        }

    }

    /// Internal function just to make error try in common.
    fn poll(&mut self, events: &mut Events) -> io::Result<bool> {

        // NOTE: We use 1 second timeout in order to regularly check channel.
        self.poll.poll(events, Some(Duration::from_secs(1)))?;

        for event in events.iter() {
            
            let run = match event.token() {
                LISTENER_TOKEN => self.handle_listener()?,
                _ => self.handle_client(event),
            };

            // If any event break the thread, immediately abort.
            if !run {
                return Ok(false);
            }

        }

        // No error, just continue the thread.
        Ok(true)

    }

    /// Internal function to handle a readable polling event from the TCP listener stream.
    fn handle_listener(&mut self) -> io::Result<bool> {

        loop {

            let mut stream = match self.listener.accept() {
                Ok((stream, _addr)) => stream,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                Err(e) => return Err(e),
            };

            // Get a new unique token and register events on this stream.
            let token = self.next_token;
            self.next_token = Token(token.0.checked_add(1).expect("out of client token"));
            self.poll.registry().register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)?;

            let shared = Arc::new(SharedClient {
                stream: RwLock::new(stream),
            });

            // NOTE: Blocking send because this would have no sense to continue if the
            // command thread is not aware of the new client.
            self.commands_sender.send(ThreadCommand::NewClient { token, shared: Arc::clone(&shared) })
                .expect("commands channel should not be disconnected while this poll thread exists");

            // NOTE: Blocking send is intentional.
            if self.events_sender.send(ThreadEvent::Accept { token }).is_err() {
                // If the events channel is disconnected, stop thread.
                return Ok(false);
            }

            self.clients.insert(token, PollClient {
                shared, 
                buf: Box::new([0; BUF_SIZE]),
                buf_cursor: 0
            });

        }

    }

    /// Internal function to handle a polling event from a client. This function doesn't
    /// generate errors, if errors happen they are pushed as client events. 
    /// The function returns true if the thread should continue.
    fn handle_client(&mut self, event: &Event) -> bool {

        let token = event.token();

        if event.is_read_closed() || event.is_write_closed() {
            // If any of the stream side is closed, send a command to force the command
            // thread to forget about the client. This can also happen if the client is
            // disconnected prior to such event (following an error for example), in this
            // case the event will be triggered by the command thread, so the following
            // answer will just be ignored.
            self.handle_client_close(token, Some(new_io_abort_error("client side closed")))
        } else if event.is_readable() {
            // Try reading the client, if an error happen we directly ends the client.
            match self.handle_client_read(token) {
                Err(e) => self.handle_client_close(token, Some(e)),
                Ok(run) => run
            }
        } else {
            // No interesting event, just continue thread.
            true
        }

    }

    /// Handle a readable client event. This return false if the global loop should stop.
    /// **If this internal function return an I/O error, it should be considered critical
    /// and the client should be closed. If no error, the returned boolean indicates if
    /// the thread should continue.**
    fn handle_client_read(&mut self, token: Token) -> io::Result<bool> {

        // Just ignore no longer existing clients.
        let Some(client) = self.clients.get_mut(&token) else { return Ok(true) };
        let stream = client.shared.stream.read().expect("poisoned");
        let mut stream = &*stream;

        loop {
            match stream.read(&mut client.buf[client.buf_cursor..]) {
                Ok(0) => break,
                Ok(len) => client.buf_cursor += len,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        loop {

            // TODO: Handle packet not found in a fully filled buffer.
            let buf = &client.buf[..client.buf_cursor];

            if buf.len() == 0 {
                return Ok(true);
            }

            let mut cursor = Cursor::new(buf);
            let packet = I::read(&mut cursor)?;

            // If the channel was disconnect, return Ok(false) to stop the thread, because
            // all handles have been dropped.
            if self.events_sender.send(ThreadEvent::Packet { token, packet }).is_err() {
                return Ok(false);
            }

            let read_length = cursor.position() as usize;
            drop(cursor);

            // Remove the buffer part that we successfully read.
            client.buf.copy_within(read_length..client.buf_cursor, 0);
            client.buf_cursor -= read_length;

        }

    }

    /// Internal function to actually close and forget a client (if not already the case).
    /// This function returns true if the thread should continue to run.
    fn handle_client_close(&mut self, token: Token, error: Option<io::Error>) -> bool {
        
        // Just ignore no longer existing clients.
        let Some(client) = self.clients.remove(&token) else { return true; };

        // We block until we can write the stream, blocking is not a problem here because
        // there are only too possible accessor for the stream: this poll thread and the
        // command thread. We are the poll thread trying to write, and if the command
        // thread is currently reading it (and therefore blocking it) it will end really
        // soon, when it will finish writing a packet.
        let mut stream = client.shared.stream.write().expect("poisoned");

        // NOTE: Shutting down this stream will trigger events in the PollThread and 
        // deregister the event.
        let _ = stream.shutdown(Shutdown::Both);
        let _ = self.poll.registry().deregister(&mut *stream);

        // NOTE: Blocking intentionally (read same comment above).
        self.commands_sender.send(ThreadCommand::LostClient { token })
            .expect("commands channel should not be disconnected while this poll thread exists");

        // NOTE: We use blocking send, because there is no point continuing if we can no
        // longer send events, just wait for handles to process events.
        // NOTE: We also return false (stop thread) if the channel is disconnected (that
        // would mean all handles are gone).
        self.events_sender.send(ThreadEvent::Lost { token, error }).is_ok()

    }

}

/// Internal command thread. This thread stores all clients and their buffers, and 
/// handles all the overhead of encoding and decoding packets. It terminates when all
/// command senders are gone (all handles and the poll thread, so the poll thread must
/// terminate in order to terminate this one).
struct CommandThread<O> {
    /// This channel allows receiving commands from server and client handles.
    commands_receiver: Receiver<ThreadCommand<O>>,
    /// Connected clients, mapped to their polling token.
    clients: HashMap<Token, Arc<SharedClient>>,
}

impl<O: OutPacket> CommandThread<O> {

    /// Run the thread until termination or critical error.
    fn run(mut self) {
        // This receive commands while there is any sender.
        while let Ok(command) = self.commands_receiver.recv() {
            match command {
                ThreadCommand::NewClient { token, shared } => {
                    self.clients.insert(token, shared);
                }
                ThreadCommand::LostClient { token } => {
                    self.clients.remove(&token).expect("client already lost");
                }
                ThreadCommand::DisconnectClient { token } => {
                    self.handle_client_disconnect(token);
                }
                ThreadCommand::SingleClientPacket { token, packet } => {
                    self.handle_client_send(token, packet);
                }
            }
        }
    }

    fn handle_client_disconnect(&mut self, token: Token) {
        // Just ignore no longer existing clients.
        let Some(client) = self.clients.get(&token) else { return };
        let stream = client.stream.read().expect("poisoned");
        // This shutdown should be seen by the poll thread, and therefore properly
        // shutdown and deregister, and a `ThreadCommand::LostClient` should come back
        // to this command thread.
        let _ = stream.shutdown(Shutdown::Both);
    }

    /// Internal function to send a packet to the given client. If an error is returned,
    /// it should be considered critical for the client, and the client should be closed.
    fn handle_client_send(&mut self, token: Token, packet: O) {
        // Just ignore no longer existing clients.
        let Some(client) = self.clients.get(&token) else { return };
        let stream = client.stream.read().expect("poisoned");
        // NOTE: For now we ignore I/O errors because we can't send them to handle.
        let _ = packet.write(&mut &*stream);
    }

}

enum ThreadCommand<O> {
    /// Sent by the poll thread when a new client has been accepted.
    NewClient {
        token: Token,
        shared: Arc<SharedClient>,
    },
    /// Sent by the poll thread when a client should be forget by the command thread
    LostClient {
        token: Token,
    },
    /// Sent by handles to force disconnect a client.
    DisconnectClient {
        token: Token,
    },
    /// Send a single packet to a client.
    SingleClientPacket {
        token: Token,
        packet: O,
    }
}

enum ThreadEvent<I> {
    /// Internal event to check if the channel is still connected.
    ChannelCheck,
    /// A client was accepted and can receive and send packets from now on.
    Accept {
        token: Token,
    },
    Lost {
        token: Token,
        /// Some error if that caused the client to be lost, no error means that the
        /// client was just kicked from the server or closed the connection itself.
        error: Option<io::Error>,
    },
    /// A packet was received from a client.
    Packet {
        token: Token,
        packet: I,
    },
    /// An I/O error that caused the background thread to crash, it's not recoverable.
    Error {
        error: io::Error,
    }
}


fn new_io_abort_error(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionAborted, message)
}
