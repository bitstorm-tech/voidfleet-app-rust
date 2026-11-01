//! Verbindung zum Spielserver (Port von `net/connection.ts` samt dem Serverstart der Electron-App).
//!
//! - [`launch`]: Kommandozeile ([`Launch`]) und Beitrittsdaten ([`Endpoint`]: Match-Id, Token je Start).
//! - [`connection`]: der WebSocket in einem eigenen Thread ([`Connection`]), ohne Bevy.
//!
//! Das Plugin verbindet sich beim Start: ohne Argumente mit dem eingebetteten Server
//! ([`voidfleet_server::server::spawn_local`]) gegen den Computergegner, sonst mit dem entfernten Server. Die Match-Id
//! steht im Log. Ausgehendes schreibt man als [`ToServer`]; Eingehendes kommt als [`FromServer`], erst wenn die
//! Assets geladen sind ([`AppState::Match`]), bis dahin wartet es im Kanal. Fällt die Verbindung, kommt einmal
//! [`ConnectionLost`].

pub mod connection;
pub mod launch;

use bevy::prelude::*;
use voidfleet_core::protocol::{ClientMessage, ServerMessage};

pub use connection::{Connection, Incoming};
pub use launch::{Endpoint, Launch, USAGE};

use crate::assets::AppState;

/// Eine Nachricht an den Server.
#[derive(Message, Clone, Debug)]
pub struct ToServer(pub ClientMessage);

/// Eine Nachricht vom Server.
#[derive(Message, Clone, Debug)]
pub struct FromServer(pub ServerMessage);

/// Die Verbindung zum Server ist verloren (oder kam nie zustande).
#[derive(Message, Clone, Debug)]
pub struct ConnectionLost;

/// Stand der Verbindung, für die Statuszeile.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Connecting,
    Connected,
    Disconnected,
}

/// Die offene Verbindung samt Beitrittsdaten.
#[derive(Resource, Debug)]
pub struct Server {
    pub endpoint: Endpoint,
    connection: Connection,
}

impl Server {
    /// Legt auf demselben Server ein neues Match gegen den Computer an; die alte Verbindung schließt.
    pub fn start_new_match(&mut self) {
        self.endpoint = Endpoint::new(self.endpoint.server.clone(), None, true);
        info!("Neues Match {}", self.endpoint.match_id);
        self.connection = Connection::open(self.endpoint.url());
    }
}

/// Reihenfolge im Bild: Erst kommt an, was der Server geschickt hat, dann arbeiten die Systeme damit.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReceiveSet;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Launch>()
            .init_resource::<ConnectionStatus>()
            .add_message::<ToServer>()
            .add_message::<FromServer>()
            .add_message::<ConnectionLost>()
            .add_systems(Startup, connect)
            .add_systems(
                Update,
                (
                    receive
                        .in_set(ReceiveSet)
                        .run_if(in_state(AppState::Match))
                        .run_if(resource_exists::<Server>),
                    send.run_if(resource_exists::<Server>),
                ),
            );
    }
}

/// Startet bei Bedarf den eingebetteten Server und verbindet sich.
fn connect(launch: Res<Launch>, mut status: ResMut<ConnectionStatus>, mut commands: Commands) {
    let endpoint = match launch.clone() {
        Launch::Local => match voidfleet_server::server::spawn_local() {
            Ok(address) => Endpoint::new(format!("ws://{address}"), None, true),
            Err(error) => {
                error!("Eingebetteter Server startet nicht: {error}");
                *status = ConnectionStatus::Disconnected;
                return;
            }
        },
        Launch::Remote {
            server,
            match_id,
            bot,
        } => Endpoint::new(server, match_id, bot),
    };
    info!(
        "Match-Id {} auf {}{}",
        endpoint.match_id,
        endpoint.server,
        if endpoint.bot {
            " (gegen den Computer)"
        } else {
            ""
        }
    );
    let connection = Connection::open(endpoint.url());
    commands.insert_resource(Server {
        endpoint,
        connection,
    });
}

fn send(server: Res<Server>, mut outgoing: MessageReader<ToServer>) {
    for ToServer(message) in outgoing.read() {
        server.connection.send(message.clone());
    }
}

fn receive(
    server: Res<Server>,
    mut status: ResMut<ConnectionStatus>,
    mut messages: MessageWriter<FromServer>,
    mut lost: MessageWriter<ConnectionLost>,
) {
    for incoming in server.connection.receive() {
        match incoming {
            Incoming::Message(message) => {
                status.set_if_neq(ConnectionStatus::Connected);
                messages.write(FromServer(message));
            }
            Incoming::Closed(reason) => {
                match reason {
                    Some(reason) => error!("Verbindung zum Server verloren: {reason}"),
                    None => warn!("Der Server hat die Verbindung geschlossen"),
                }
                *status = ConnectionStatus::Disconnected;
                lost.write(ConnectionLost);
            }
        }
    }
}
