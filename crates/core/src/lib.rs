//! Das Regelwerk von Voidfleet: ein Flaggschiff gegen die gegnerische Flotte (`docs/PVE_DESIGN.md`).
//!
//! - [`board`]: Reihen, Spalten und Slots.
//! - [`cards`]: das Befehlsdeck samt Aufwertungen und Startdecks.
//! - [`admirals`]: Admirale, Flaggschiffe und Stationen.
//! - [`enemies`]: Gegner, ihre Werte und Muster.
//! - [`modules`]: die Module des Runs.
//! - [`battle`]: das Gefecht, die eine Schnittstelle für den Client.
//! - [`events`]: der Ereignisstrom eines Gefechts.
//! - [`run`]: der Run: Sternenkarte, Verbände, Beute, Werft, Schwarzmarkt und Ereignisse.

pub mod admirals;
pub mod battle;
pub mod board;
pub mod cards;
pub mod enemies;
pub mod events;
pub mod modules;
pub mod run;
