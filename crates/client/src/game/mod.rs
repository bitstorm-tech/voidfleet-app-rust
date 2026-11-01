//! Das Match aus Sicht des Clients, ohne Darstellung.
//!
//! - [`state`]: Spielzustand und Zugplanung ([`Match`], Ressource).
//! - [`preview`]: Zielvorschau der Runde mit dem eigenen Plan ([`preview_round`]).
//!
//! Das Plugin übernimmt die Nachrichten des Servers in einer Warteschlange ([`ServerInbox`]), höchstens eine je Bild:
//! `State` aktualisiert [`Match`] und meldet [`StateChanged`], `Round` beendet die Runde und meldet [`RoundResolved`]
//! mit den Ereignissen, `Error` meldet [`ServerError`]. Wer eine Runde abspielt, hält die Warteschlange beim
//! [`RoundResolved`] an ([`ServerInbox::hold`]) und gibt sie danach frei; erst dann kommt der neue Zustand, und die
//! Szene gleicht sich ihm an. Abgeben und Wählen gehen als [`crate::net::ToServer`] hinaus.

pub mod preview;
pub mod state;

use std::collections::VecDeque;

use bevy::prelude::*;
use voidfleet_core::protocol::{ErrorCode, RoundEvent, ServerMessage};

pub use preview::{PreviewShot, RoundPreview, ShotOutcome, preview_round};
pub use state::{
    Arrival, CommandChange, FireOrderCount, ManeuverChange, Match, ShipPick, TurnPartState,
    TurnProgress,
};

use crate::{
    i18n::Locale,
    net::{FromServer, ReceiveSet},
};

/// [`Match`] hat einen neuen Zustand vom Server übernommen.
#[derive(Message, Clone, Copy, Debug)]
pub struct StateChanged;

/// Die Runde `round` ist aufgelöst; `events` aus eigener Sicht in Auflösungsreihenfolge. Der Plan in [`Match`] ist
/// schon verbraucht, Brett und Hände sind noch die von vor der Runde, bis der nächste `State` kommt.
#[derive(Message, Clone, Debug)]
pub struct RoundResolved {
    pub round: u32,
    pub events: Vec<RoundEvent>,
}

/// Der Server hat eine Nachricht abgelehnt; der Text steht in [`Locale`] (`error`).
#[derive(Message, Clone, Copy, Debug)]
pub struct ServerError(pub ErrorCode);

/// Server-Nachrichten, die noch nicht übernommen sind.
#[derive(Resource, Debug, Default)]
pub struct ServerInbox {
    queue: VecDeque<ServerMessage>,
    held: bool,
}

impl ServerInbox {
    /// Hält die Warteschlange an, etwa solange eine Runde abgespielt wird.
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Gibt sie wieder frei.
    pub fn release(&mut self) {
        self.held = false;
    }
}

/// Reihenfolge im Bild: Nach [`ReceiveSet`] übernimmt das Plugin die nächste Nachricht; wer auf seine Messages
/// reagiert, läuft danach.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ApplySet;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Match>()
            .init_resource::<ServerInbox>()
            .add_message::<StateChanged>()
            .add_message::<RoundResolved>()
            .add_message::<ServerError>()
            .configure_sets(Update, ApplySet.after(ReceiveSet))
            .add_systems(Update, (enqueue, apply_next).chain().in_set(ApplySet));
    }
}

fn enqueue(mut incoming: MessageReader<FromServer>, mut inbox: ResMut<ServerInbox>) {
    inbox
        .queue
        .extend(incoming.read().map(|FromServer(message)| message.clone()));
}

/// Übernimmt die nächste Nachricht, solange die Warteschlange nicht angehalten ist.
fn apply_next(
    mut inbox: ResMut<ServerInbox>,
    mut game: ResMut<Match>,
    locale: Res<Locale>,
    mut changed: MessageWriter<StateChanged>,
    mut resolved: MessageWriter<RoundResolved>,
    mut errors: MessageWriter<ServerError>,
) {
    if inbox.held {
        return;
    }
    let Some(message) = inbox.queue.pop_front() else {
        return;
    };
    match message {
        ServerMessage::State { view } => {
            game.apply(*view);
            info!(
                "Zustand: Runde {}, Admiral {:?}, Gegner {}, Hand {:?} / {:?}",
                game.round,
                game.admiral,
                match &game.opponent {
                    Some(opponent) if opponent.bot => "Computer",
                    Some(_) => "Mitspieler",
                    None => "fehlt",
                },
                game.ship_hand,
                game.command_hand,
            );
            changed.write(StateChanged);
        }
        ServerMessage::Round { round, events } => {
            info!("Runde {round} aufgelöst: {} Ereignisse", events.len());
            game.end_round();
            resolved.write(RoundResolved { round, events });
        }
        ServerMessage::Error { code } => {
            warn!("Server lehnt ab: {}", locale.error(code));
            errors.write(ServerError(code));
        }
    }
}
