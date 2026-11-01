//! Die WebSocket-Verbindung zum Server in einem eigenen Thread, ohne Bevy: blockierendes `tungstenite` mit kurzem
//! Lese-Timeout, damit derselbe Thread zwischendurch die ausgehenden Nachrichten schreiben kann. Mit dem Rest des
//! Spiels redet er über zwei Kanäle; fällt die [`Connection`], schließt er den Socket und endet.

use std::{io::ErrorKind, net::TcpStream, thread, time::Duration};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use tungstenite::{
    ClientRequestBuilder, HandshakeError, Message, WebSocket, error::UrlError, http::Uri,
};
use voidfleet_core::protocol::{ClientMessage, ServerMessage};

/// So lange wartet ein Lesen höchstens, bevor der Thread nach ausgehenden Nachrichten schaut.
const POLL: Duration = Duration::from_millis(20);

/// Was der Verbindungs-Thread meldet.
#[derive(Debug)]
pub enum Incoming {
    Message(ServerMessage),
    /// Die Verbindung ist zu, mit dem Grund, falls sie nicht regulär endete; danach kommt nichts mehr.
    Closed(Option<String>),
}

/// Eine Verbindung zum Server; der Thread dahinter läuft, bis sie fällt oder der Server schließt.
#[derive(Debug)]
pub struct Connection {
    outgoing: Sender<ClientMessage>,
    incoming: Receiver<Incoming>,
}

impl Connection {
    /// Verbindet sich im Hintergrund mit `url` (`ws://host:port/ws?…`); schlägt das fehl, meldet der Kanal
    /// [`Incoming::Closed`].
    pub fn open(url: String) -> Self {
        let (outgoing, outbox) = crossbeam_channel::unbounded();
        let (inbox, incoming) = crossbeam_channel::unbounded();
        thread::Builder::new()
            .name("voidfleet-net".into())
            .spawn(move || {
                let error = run(&url, &outbox, &inbox)
                    .err()
                    .map(|error| error.to_string());
                // Ist die Gegenseite schon weg, interessiert das Ende niemanden mehr.
                inbox.send(Incoming::Closed(error)).ok();
            })
            .expect("Thread der Verbindung startet");
        Connection { outgoing, incoming }
    }

    /// Schickt eine Nachricht; ist die Verbindung schon zu, geht sie ins Leere.
    pub fn send(&self, message: ClientMessage) {
        self.outgoing.send(message).ok();
    }

    /// Alles, was seit dem letzten Aufruf angekommen ist.
    pub fn receive(&self) -> impl Iterator<Item = Incoming> + '_ {
        self.incoming.try_iter()
    }

    /// Wartet höchstens `timeout` auf die nächste Meldung (für Tests und Werkzeuge ohne Bildtakt).
    pub fn receive_timeout(&self, timeout: Duration) -> Option<Incoming> {
        self.incoming.recv_timeout(timeout).ok()
    }
}

/// Verbindet, schreibt Ausgehendes und liest Eingehendes, bis einer der beiden Seiten schließt.
fn run(
    url: &str,
    outbox: &Receiver<ClientMessage>,
    inbox: &Sender<Incoming>,
) -> tungstenite::Result<()> {
    let mut socket = connect(url)?;
    loop {
        loop {
            match outbox.try_recv() {
                Ok(message) => {
                    let json = serde_json::to_string(&message)
                        .expect("Client-Nachrichten sind serialisierbar");
                    socket.send(Message::text(json))?;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    socket.close(None)?;
                    return Ok(());
                }
            }
        }
        match socket.read() {
            Ok(Message::Text(text)) => match serde_json::from_str(&text) {
                Ok(message) => {
                    if inbox.send(Incoming::Message(message)).is_err() {
                        return Ok(());
                    }
                }
                Err(error) => bevy::log::warn!("Unlesbare Nachricht vom Server: {error}"),
            },
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(tungstenite::Error::ConnectionClosed) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

/// Öffnet den TCP-Strom, führt den Handshake und setzt danach den Lese-Timeout.
fn connect(url: &str) -> tungstenite::Result<WebSocket<TcpStream>> {
    let uri: Uri = url.parse()?;
    let host = uri
        .host()
        .ok_or(tungstenite::Error::Url(UrlError::NoHostName))?
        .to_owned();
    let port = uri.port_u16().unwrap_or(80);
    let stream = TcpStream::connect((host.as_str(), port))?;
    let (socket, _) = tungstenite::client(ClientRequestBuilder::new(uri), stream).map_err(
        |error| match error {
            HandshakeError::Failure(error) => error,
            HandshakeError::Interrupted(_) => unreachable!("der Strom blockiert beim Handshake"),
        },
    )?;
    socket.get_ref().set_read_timeout(Some(POLL))?;
    Ok(socket)
}
