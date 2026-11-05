//! Spielt das Gefecht für den Rauchtest (`VOIDFLEET_SMOKE_PLAY=<runden>`) über dieselben Messages wie ein Spieler:
//! je Runde Waffen auf den ersten Gegner auslösen, die erste bezahlbare Karte ohne Ziel spielen, Zug beenden. Vor jedem
//! Schritt wartet es, bis die Szene ruht, und schreibt einen Screenshot (`<stamm>-<runde>-<schritt>.png` neben
//! `VOIDFLEET_SMOKE_SCREENSHOT`). Es endet nach dem Gefecht oder nach `runden` Runden mit einem letzten Screenshot.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use voidfleet_core::{admirals::Station, battle::Target, cards::CardTarget};

use super::exit_when_captured;
use crate::{
    game::{Battle, PlayerAction},
    scene::sync::BoardMode,
};

/// So viele Bilder ruht die Szene vor jedem Schritt, damit Karten eingeflogen und Balken gesetzt sind.
const SETTLE_FRAMES: u32 = 45;

/// Die Schritte einer Runde.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Weapons,
    Card,
    EndTurn,
}

#[derive(Resource)]
pub(super) struct AutoPlay {
    rounds: u32,
    screenshot: Option<PathBuf>,
    step: Step,
    wait: u32,
    done: bool,
}

impl AutoPlay {
    pub(super) fn new(rounds: u32, screenshot: Option<PathBuf>) -> Self {
        Self {
            rounds,
            screenshot,
            step: Step::Weapons,
            wait: SETTLE_FRAMES,
            done: false,
        }
    }

    /// Schreibt einen Screenshot dieser Station; der letzte beendet die App.
    fn shoot(&self, commands: &mut Commands, name: &str, last: bool) {
        let Some(path) = &self.screenshot else {
            return;
        };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let mut shot = commands.spawn(Screenshot::primary_window());
        shot.observe(save_to_disk(
            path.with_file_name(format!("{stem}-{name}.png")),
        ));
        if last {
            shot.observe(exit_when_captured);
        }
    }
}

pub(super) fn autoplay(
    mut commands: Commands,
    mut play: ResMut<AutoPlay>,
    battle: Option<Res<Battle>>,
    mode: Option<Res<State<BoardMode>>>,
    mut actions: MessageWriter<PlayerAction>,
    mut exit: MessageWriter<AppExit>,
) {
    let (Some(battle), Some(mode)) = (battle, mode) else {
        return;
    };
    if play.done || *mode.get() != BoardMode::Planning {
        return;
    }
    if play.wait > 0 {
        play.wait -= 1;
        return;
    }
    play.wait = SETTLE_FRAMES;
    if battle.result.is_some() || battle.round > play.rounds {
        play.done = true;
        info!(
            "Rauchtest: Ende in Runde {}, Ergebnis {:?}",
            battle.round, battle.result
        );
        if play.screenshot.is_some() {
            play.shoot(&mut commands, "end", true);
        } else {
            exit.write(AppExit::Success);
        }
        return;
    }
    let name = format!("{}-{:?}", battle.round, play.step).to_lowercase();
    play.shoot(&mut commands, &name, false);
    match play.step {
        Step::Weapons => {
            if let Some(enemy) = battle.enemies.first() {
                actions.write(PlayerAction::SetStation(Station::Weapons));
                actions.write(PlayerAction::SetTarget(enemy.uid));
                actions.write(PlayerAction::TriggerStation);
            }
            play.step = Step::Card;
        }
        Step::Card => {
            let playable = battle.hand.iter().position(|card| {
                card.id.target() == CardTarget::None && card.cost() <= battle.points
            });
            if let Some(hand_index) = playable {
                actions.write(PlayerAction::PlayCard {
                    hand_index,
                    target: Target::None,
                });
            }
            play.step = Step::EndTurn;
        }
        Step::EndTurn => {
            actions.write(PlayerAction::EndTurn);
            play.step = Step::Weapons;
        }
    }
}
