//! Eine Partie gegen den Computer ohne Spieler, für den Rauchtest (`VOIDFLEET_SMOKE_PLAY=<runden>`): Sie klickt wie
//! ein Spieler — über dieselben Messages, die Picking, Hand, Aktionsknopf und Dialoge schicken — und prüft so den
//! Controller Ende zu Ende. Admiralin Kerr wählen, in Runde 1 Flaggschiff und zwei Schiffe legen, danach je ein
//! Schiff, den ersten Befehl auf ein hervorgehobenes Ziel, jedem Schützen ein Ziel geben (abwechselnd Spaltenfeuer
//! und Schiffsfeuer), abgeben, Nachschub wählen. Nach `rounds` Runden oder dem Ende des Matches beendet sie die App.
//!
//! Mit `VOIDFLEET_SMOKE_SCREENSHOT=<pfad>.png` entsteht an jeder Station ein Screenshot `<stamm>-<station>.png`:
//! Admiralswahl, Zielen in Runde 1 und 2, fertiger Plan je Runde, Wiedergabe, Nachschub, Ende.

use std::path::PathBuf;

use bevy::{
    diagnostic::FrameCount,
    ecs::system::SystemParam,
    platform::collections::HashSet,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{SLOT_COUNT, Side, SlotRef},
    protocol::DraftPick,
};

use crate::{
    controller::Selection,
    game::{Match, ShipPick},
    scene::{
        board::{Highlights, all_slots},
        picking::{BoardMissed, ColumnPressed, SlotPressed},
        sync::BoardMode,
    },
    ui::{
        ActionPressed, HandCard, HandCardPressed, HudAction, HudState,
        dialogs::{AdmiralChosen, DraftChosen},
    },
};

/// Alle so viele Bilder ein Klick: Zeit für Hervorhebung, Geister und Screenshots dazwischen.
const STEP: u32 = 12;
/// So viele Schritte nach dem Ende, bis die App schließt (Ergebnis-Overlay, letzter Screenshot).
const LINGER: u32 = 12;

#[derive(Resource, Debug)]
pub(super) struct AutoPlay {
    rounds: u32,
    screenshot: Option<PathBuf>,
    taken: HashSet<String>,
    /// Runde, für die die folgenden Merker gelten.
    round: u32,
    admiral_sent: bool,
    drafted: bool,
    /// Schiffe der Hand, die schon versucht wurden (gelegt oder ohne freien Slot).
    tried_ships: Vec<usize>,
    tried_command: bool,
    submitted: bool,
    /// Wie viele Feuerbefehle diese Runde vergeben wurden (für den Wechsel von Spalten- und Schiffsfeuer).
    aimed: usize,
    finished: Option<u32>,
}

impl AutoPlay {
    pub(super) fn new(rounds: u32, screenshot: Option<PathBuf>) -> Self {
        Self {
            rounds,
            screenshot,
            taken: HashSet::default(),
            round: 0,
            admiral_sent: false,
            drafted: false,
            tried_ships: Vec::new(),
            tried_command: false,
            submitted: false,
            aimed: 0,
            finished: None,
        }
    }

    /// Ein Screenshot je Station.
    fn shoot(&mut self, commands: &mut Commands, station: String) {
        let Some(path) = &self.screenshot else {
            return;
        };
        if !self.taken.insert(station.clone()) {
            return;
        }
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let file = path.with_file_name(format!("{stem}-{station}.png"));
        info!("Rauchtest: Screenshot {}", file.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(file));
    }

    /// Eine neue Runde setzt die Merker zurück.
    fn enter_round(&mut self, round: u32) {
        if self.round != round {
            *self = Self {
                rounds: self.rounds,
                screenshot: self.screenshot.take(),
                taken: std::mem::take(&mut self.taken),
                admiral_sent: self.admiral_sent,
                finished: self.finished,
                round,
                ..Self::new(0, None)
            };
        }
    }
}

/// Was ein Spieler klicken kann.
#[derive(SystemParam)]
pub(super) struct Clicks<'w> {
    hand: MessageWriter<'w, HandCardPressed>,
    slots: MessageWriter<'w, SlotPressed>,
    columns: MessageWriter<'w, ColumnPressed>,
    missed: MessageWriter<'w, BoardMissed>,
    action: MessageWriter<'w, ActionPressed>,
    admiral: MessageWriter<'w, AdmiralChosen>,
    draft: MessageWriter<'w, DraftChosen>,
}

/// Der erste hervorgehobene Slot, eigene vor gegnerischen.
fn first_highlighted(highlights: &Highlights) -> Option<SlotRef> {
    all_slots().find(|slot| highlights.slots.contains(slot))
}

pub(super) fn autoplay(
    frame: Res<FrameCount>,
    mut commands: Commands,
    mut play: ResMut<AutoPlay>,
    game: Res<Match>,
    selection: Res<Selection>,
    hud: Res<HudState>,
    highlights: Res<Highlights>,
    mode: Res<State<BoardMode>>,
    mut clicks: Clicks,
    mut exit: MessageWriter<AppExit>,
) {
    if !frame.0.is_multiple_of(STEP) {
        return;
    }
    if let Some(steps) = play.finished {
        if steps == 0 {
            info!("Rauchtest: Partie beendet in Runde {}", game.round);
            exit.write(AppExit::Success);
        }
        play.finished = Some(steps.saturating_sub(1));
        return;
    }
    let round = game.round;
    if *mode.get() == BoardMode::Playback {
        play.shoot(&mut commands, format!("playback-r{round}"));
        return;
    }
    if game.result.is_some() || (round > play.rounds && hud.planning) {
        play.shoot(&mut commands, "end".into());
        play.finished = Some(LINGER);
        return;
    }
    if round > 0 && game.admiral.is_none() {
        if !play.admiral_sent {
            play.shoot(&mut commands, "admiral".into());
            play.admiral_sent = true;
            clicks.admiral.write(AdmiralChosen(AdmiralId::Kerr));
        }
        return;
    }
    play.enter_round(round);
    if let Some(draft) = &game.draft {
        if !play.drafted {
            play.shoot(&mut commands, format!("draft-r{round}"));
            play.drafted = true;
            clicks.draft.write(DraftChosen(DraftPick {
                ship: (!draft.ships.is_empty()).then_some(0),
                command: (!draft.commands.is_empty()).then_some(0),
            }));
        }
        return;
    }
    if !hud.planning {
        return;
    }
    match *selection {
        Selection::Ship(_) | Selection::Command { .. } => match first_highlighted(&highlights) {
            Some(slot) => {
                clicks.slots.write(SlotPressed(slot));
            }
            None => {
                clicks.missed.write(BoardMissed);
            }
        },
        Selection::Unit(unit) => {
            if game.is_armed(unit.target()) {
                clicks.missed.write(BoardMissed);
                return;
            }
            play.shoot(&mut commands, format!("aim-r{round}"));
            let enemy = all_slots()
                .find(|slot| slot.side == Side::Enemy && highlights.slots.contains(slot));
            match enemy.filter(|_| play.aimed % 2 == 1) {
                Some(target) => {
                    clicks.slots.write(SlotPressed(target));
                }
                None => {
                    clicks
                        .columns
                        .write(ColumnPressed((unit.slot_index + play.aimed) % SLOT_COUNT));
                }
            }
            play.aimed += 1;
        }
        Selection::Power | Selection::Mark => {
            clicks.missed.write(BoardMissed);
        }
        Selection::Idle => idle_click(&mut commands, &mut play, &game, &mut clicks),
    }
}

/// Ohne Auswahl: Flaggschiff, Schiffe, Befehl, Feuerbefehle, dann abgeben.
fn idle_click(commands: &mut Commands, play: &mut AutoPlay, game: &Match, clicks: &mut Clicks) {
    if game.flagship_missing() {
        clicks
            .hand
            .write(HandCardPressed(HandCard::Ship(ShipPick::Flagship)));
        return;
    }
    let next_ship = (game.deploys_left() > 0)
        .then(|| (0..game.ship_hand.len()).find(|index| !play.tried_ships.contains(index)))
        .flatten();
    if let Some(index) = next_ship {
        play.tried_ships.push(index);
        clicks
            .hand
            .write(HandCardPressed(HandCard::Ship(ShipPick::Hand(index))));
        return;
    }
    if !play.tried_command && !game.command_hand.is_empty() {
        play.tried_command = true;
        clicks.hand.write(HandCardPressed(HandCard::Command(0)));
        return;
    }
    if let Some(shooter) = game
        .shooters()
        .into_iter()
        .find(|&ship| !game.is_armed(ship))
    {
        clicks.slots.write(SlotPressed(shooter.on(Side::Player)));
        return;
    }
    if !play.submitted {
        play.shoot(commands, format!("plan-r{}", play.round));
        play.submitted = true;
        clicks.action.write(ActionPressed(HudAction::EndTurn));
    }
}
