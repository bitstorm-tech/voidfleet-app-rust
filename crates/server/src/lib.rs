//! Der Spielserver von Voidfleet: autoritative Matches im Speicher, verbunden über WebSockets.
//!
//! - [`deck`]: gemischte Nachziehstapel.
//! - [`game`]: das Match (Sitzplätze, Admiralswahl, Zugprüfung, Auflösung, Nachschub, Sicht je Spieler).
//! - [`bot`]: die Schnittstelle zum Computergegner.
//! - [`server`]: HTTP/WebSocket und Räume; [`serve`] für eine eigene Runtime, [`spawn_local`] eingebettet im Client.

pub mod bot;
pub mod deck;
pub mod game;
pub mod server;

pub use game::{BotView, Match, Outcome, Seat};
pub use server::{router, serve, spawn_local};
