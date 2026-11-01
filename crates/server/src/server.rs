//! HTTP/WebSocket: verbindet die Spieler unter `/ws?match=<id>&token=<token>` mit ihren Räumen (Match samt den
//! Verbindungen beider Sitze). Matches leben nur im Speicher und entstehen beim ersten Beitritt; wer den Match-Link
//! öffnet, sitzt mit drin. Mit `&bot` legt der erste Spieler das Match gleich gegen den Computergegner an.

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use voidfleet_core::protocol::{ClientMessage, ErrorCode, ServerMessage, TurnPlan};

use crate::game::{Match, Seat};

/// Eine offene Verbindung eines Sitzes: Was über `outbox` geht, schreibt ihre Task auf den Socket.
struct Connection {
    /// Unterscheidet die Verbindungen eines Sitzes, wenn ein zweiter Tab den Platz übernimmt.
    id: u64,
    outbox: UnboundedSender<Message>,
}

impl Connection {
    fn send(&self, message: &ServerMessage) {
        let json = serde_json::to_string(message).expect("Servernachrichten sind serialisierbar");
        // Ist die Verbindung schon zu, geht die Nachricht ins Leere.
        self.outbox.send(Message::text(json)).ok();
    }

    fn close(&self) {
        self.outbox.send(Message::Close(None)).ok();
    }
}

/// Ein Match samt den Verbindungen beider Sitze.
struct Room {
    game: Match,
    connections: [Option<Connection>; 2],
}

impl Room {
    fn new(id: &str) -> Self {
        Room {
            game: Match::new(id),
            connections: [None, None],
        }
    }

    fn send(&self, seat: Seat, message: &ServerMessage) {
        if let Some(connection) = &self.connections[seat.index()] {
            connection.send(message);
        }
    }

    fn broadcast_state(&self) {
        for seat in self.game.seats() {
            let view = Box::new(self.game.view(seat));
            self.send(seat, &ServerMessage::State { view });
        }
    }

    /// Setzt die Verbindung auf ihren Sitz; eine ältere Verbindung desselben Tokens (zweiter Tab) wird geschlossen.
    fn join(&mut self, token: &str, bot: bool, connection: Connection) -> Result<Seat, ErrorCode> {
        let seat = self.game.join(token)?;
        if bot && self.game.seats().count() == 1 {
            self.game.add_bot()?;
        }
        if let Some(old) = self.connections[seat.index()].replace(connection) {
            old.close();
        }
        self.broadcast_state();
        Ok(seat)
    }

    /// Gibt den Sitz frei, wenn diese Verbindung noch die aktuelle ist.
    fn leave(&mut self, seat: Seat, id: u64) {
        let slot = &mut self.connections[seat.index()];
        if slot.as_ref().is_some_and(|connection| connection.id == id) {
            *slot = None;
        }
    }

    /// Führt die Nachricht aus: bei Erfolg bekommen beide Sitze ihre neue Sicht, sonst der Absender den Fehler.
    fn handle(&mut self, seat: Seat, message: ClientMessage) {
        let result = match message {
            ClientMessage::ChooseAdmiral { admiral } => self.game.choose_admiral(seat, admiral),
            ClientMessage::Draft { pick } => self.game.draft(seat, pick),
            ClientMessage::Submit { plan } => self.submit(seat, plan),
            ClientMessage::Surrender => self.game.surrender(seat),
            ClientMessage::AddBot => self.game.add_bot().map(|_| ()),
            ClientMessage::RemoveBot => self.game.remove_bot(),
        };
        match result {
            Ok(()) => self.broadcast_state(),
            Err(code) => self.send(seat, &ServerMessage::Error { code }),
        }
    }

    /// Nimmt den Zug an und löst die Runde auf, sobald beide abgegeben haben; jeder bekommt ihre Ereignisse aus
    /// seiner Sicht.
    fn submit(&mut self, seat: Seat, plan: TurnPlan) -> Result<(), ErrorCode> {
        self.game.submit(seat, plan)?;
        if self.game.is_round_complete() {
            let round = self.game.round();
            let events = self.game.resolve_round();
            for seat in self.game.seats() {
                let events = self.game.events_for(seat, &events);
                self.send(seat, &ServerMessage::Round { round, events });
            }
        }
        Ok(())
    }
}

/// Alle Räume; jeder Raum hat sein eigenes Schloss, damit Matches sich nicht gegenseitig aufhalten.
#[derive(Clone, Default)]
struct Rooms {
    rooms: Arc<Mutex<HashMap<String, Arc<Mutex<Room>>>>>,
    next_connection: Arc<AtomicU64>,
}

impl Rooms {
    /// Der Raum des Matches; beim ersten Beitritt entsteht er.
    fn room(&self, match_id: &str) -> Arc<Mutex<Room>> {
        let mut rooms = lock(&self.rooms);
        Arc::clone(
            rooms
                .entry(match_id.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(Room::new(match_id)))),
        )
    }
}

/// Ein Schloss, das ein Panik in einem anderen Handler nicht unbrauchbar macht: Der Zustand dahinter ist jeweils nach
/// einer ganzen Nachricht konsistent.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Deserialize)]
struct JoinParams {
    #[serde(rename = "match")]
    match_id: Option<String>,
    token: Option<String>,
    /// Vorhanden (auch ohne Wert): Der erste Spieler spielt gegen den Computergegner.
    bot: Option<String>,
}

async fn upgrade(
    State(rooms): State<Rooms>,
    Query(params): Query<JoinParams>,
    ws: WebSocketUpgrade,
) -> Response {
    let (Some(match_id), Some(token)) = (params.match_id, params.token) else {
        return (StatusCode::BAD_REQUEST, "match und token fehlen").into_response();
    };
    let bot = params.bot.is_some();
    ws.on_upgrade(move |socket| connect(socket, rooms, match_id, token, bot))
}

/// Eine Verbindung von Beitritt bis Abbruch: Eingehende Nachrichten führt der Raum aus, ausgehende schreibt eine
/// eigene Task.
async fn connect(socket: WebSocket, rooms: Rooms, match_id: String, token: String, bot: bool) {
    let (sink, mut stream) = socket.split();
    let (outbox, inbox) = mpsc::unbounded_channel();
    tokio::spawn(write(sink, inbox));

    let room = rooms.room(&match_id);
    let id = rooms.next_connection.fetch_add(1, Ordering::Relaxed);
    let connection = Connection {
        id,
        outbox: outbox.clone(),
    };
    let joined = lock(&room).join(&token, bot, connection);
    let seat = match joined {
        Ok(seat) => seat,
        Err(code) => {
            let refused = Connection { id, outbox };
            refused.send(&ServerMessage::Error { code });
            refused.close();
            return;
        }
    };
    drop(outbox);

    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                // Gibt ein Spieler gegen den Computergegner ab, plant der Bot seinen Zug gleich mit; das rechnet und
                // läuft deshalb nicht auf einem Async-Worker.
                Ok(message) => {
                    let room = Arc::clone(&room);
                    tokio::task::spawn_blocking(move || lock(&room).handle(seat, message))
                        .await
                        .expect("die Nachricht wird ohne Panik ausgeführt");
                }
                Err(error) => {
                    eprintln!("Unlesbare Nachricht von Sitz {seat:?} in {match_id}: {error}")
                }
            },
            Message::Close(_) => break,
            _ => {}
        }
    }
    lock(&room).leave(seat, id);
}

/// Schreibt die Nachrichten der Verbindung auf den Socket, bis sie geschlossen wird oder niemand mehr sendet.
async fn write(mut sink: SplitSink<WebSocket, Message>, mut inbox: UnboundedReceiver<Message>) {
    while let Some(message) = inbox.recv().await {
        let closing = matches!(message, Message::Close(_));
        if sink.send(message).await.is_err() || closing {
            break;
        }
    }
}

/// Der Router des Servers: nur der WebSocket unter `/ws`.
pub fn router() -> Router {
    Router::new()
        .route("/ws", get(upgrade))
        .with_state(Rooms::default())
}

/// Bedient Verbindungen auf `listener`, bis der Prozess endet.
pub async fn serve(listener: TcpListener) -> io::Result<()> {
    axum::serve(listener, router()).await
}

/// Startet den Server eingebettet, etwa im nativen Client: in einem eigenen Thread mit eigener Tokio-Runtime, nur auf
/// `127.0.0.1` (nicht im Netz erreichbar) und einem freien Port. Gibt die Adresse zurück, sobald er Verbindungen
/// annimmt; er läuft, bis der Prozess endet.
pub fn spawn_local() -> io::Result<SocketAddr> {
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("voidfleet-server")
        .build()?;
    thread::Builder::new()
        .name("voidfleet-server".into())
        .spawn(move || {
            let served = runtime.block_on(async { serve(TcpListener::from_std(listener)?).await });
            if let Err(error) = served {
                eprintln!("Eingebetteter Voidfleet-Server beendet: {error}");
            }
        })?;
    Ok(address)
}
