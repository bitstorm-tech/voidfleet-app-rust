//! Das Regelwerk von Voidfleet, gemeinsam für Server, Computergegner und Client.
//!
//! - [`cards`]: Schiffs- und Befehlskarten samt Werten.
//! - [`admirals`]: Admirale, ihre Flaggschiffe und Fähigkeiten.
//! - [`board`]: das Spielbrett mit Platzierungsregeln und dem Live-Zustand der Schiffe.
//! - [`powers`]: Ziele der Admiralsfähigkeiten und der Markierung der Pendulum.
//! - [`protocol`]: Nachrichten zwischen Client und Server, Ereignisse einer Runde.
//! - [`resolution`]: die Auflösung einer Runde.

pub mod admirals;
pub mod board;
pub mod cards;
pub mod powers;
pub mod protocol;
pub mod resolution;
