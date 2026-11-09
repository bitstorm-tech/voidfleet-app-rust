//! Spielt den Run für den Rauchtest (`VOIDFLEET_SMOKE_PLAY=<runden>`) über dieselben Messages wie ein Spieler: auf der
//! Karte den ersten wählbaren Knoten, bei der Beute die erste Karte bzw. das erste Modul, im Ereignis die erste
//! bezahlbare Entscheidung, in der Werft Reparieren, den Markt sofort verlassen; im Gefecht je Runde Waffen auf den
//! ersten Gegner auslösen, die erste bezahlbare Karte ohne Ziel spielen, Zug beenden, nach dem Ergebnis „Weiter“. Vor
//! jedem Schritt wartet es, bis die Szene ruht. Je Station (Karte, Beute, Ereignis, Werft, Markt, Beginn eines
//! Gefechts) schreibt es einen Screenshot `<stamm>-<nr>-<station>.png` neben `VOIDFLEET_SMOKE_SCREENSHOT`. Es endet
//! mit dem Run oder nach `runden` gespielten Runden mit einem letzten Screenshot.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use voidfleet_core::{
    admirals::Station,
    battle::Target,
    cards::CardTarget,
    run::{EventOption, PILOT_PRICE, Stage},
};

use super::exit_when_captured;
use crate::{
    game::{Battle, LeaveBattle, PlayerAction},
    run::{ActiveRun, RunAction, RunFlow, Screen},
    scene::sync::BoardMode,
};

/// So viele Bilder ruht die Szene vor jedem Schritt, damit Overlays stehen, Karten eingeflogen und Balken gesetzt sind.
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
    played: u32,
    screenshot: Option<PathBuf>,
    shots: u32,
    step: Step,
    wait: u32,
    /// Die Station ist schon im Bild; erst im nächsten Schritt wird gehandelt.
    shot: bool,
    done: bool,
}

impl AutoPlay {
    pub(super) fn new(rounds: u32, screenshot: Option<PathBuf>) -> Self {
        Self {
            rounds,
            played: 0,
            screenshot,
            shots: 0,
            step: Step::Weapons,
            wait: SETTLE_FRAMES,
            shot: false,
            done: false,
        }
    }

    /// Schreibt einen Screenshot dieser Station; der letzte beendet die App.
    fn shoot(&mut self, commands: &mut Commands, name: &str, last: bool) {
        let Some(path) = &self.screenshot else {
            return;
        };
        self.shots += 1;
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let mut shot = commands.spawn(Screenshot::primary_window());
        shot.observe(save_to_disk(
            path.with_file_name(format!("{stem}-{:03}-{name}.png", self.shots)),
        ));
        if last {
            shot.observe(exit_when_captured);
        }
    }

    /// Fotografiert die Station einmal und meldet `true`; gehandelt wird erst im nächsten Schritt, denn der Screenshot
    /// entsteht ein paar Bilder später und zeigte sonst schon die Folge der Wahl.
    fn snap(&mut self, commands: &mut Commands, name: &str) -> bool {
        self.shot = !self.shot;
        if self.shot {
            self.shoot(commands, name, false);
        }
        self.shot
    }

    fn finish(&mut self, commands: &mut Commands, exit: &mut MessageWriter<AppExit>) {
        self.done = true;
        if self.screenshot.is_some() {
            self.shoot(commands, "end", true);
        } else {
            exit.write(AppExit::Success);
        }
    }
}

pub(super) fn autoplay(
    mut commands: Commands,
    mut play: ResMut<AutoPlay>,
    screen: Res<Screen>,
    run: Option<Res<ActiveRun>>,
    battle: Option<Res<Battle>>,
    mode: Option<Res<State<BoardMode>>>,
    mut actions: MessageWriter<PlayerAction>,
    mut run_actions: MessageWriter<RunAction>,
    mut flow: MessageWriter<RunFlow>,
    mut leave: MessageWriter<LeaveBattle>,
    mut exit: MessageWriter<AppExit>,
) {
    if play.done {
        return;
    }
    if screen.is_changed() {
        play.wait = SETTLE_FRAMES;
    }
    let Some(run) = run else {
        return;
    };
    if battle.is_some() && mode.is_none_or(|mode| *mode.get() != BoardMode::Planning) {
        return;
    }
    if play.wait > 0 {
        play.wait -= 1;
        return;
    }
    play.wait = SETTLE_FRAMES;
    if play.played >= play.rounds {
        info!("Rauchtest: Ende nach {} Runden", play.played);
        play.finish(&mut commands, &mut exit);
        return;
    }
    if let Some(battle) = battle {
        if battle.result.is_some() {
            info!(
                "Rauchtest: Gefecht {:?} in Runde {}",
                battle.result, battle.round
            );
            leave.write(LeaveBattle);
            play.step = Step::Weapons;
            return;
        }
        if battle.round == 1 && play.step == Step::Weapons && play.snap(&mut commands, "battle") {
            return;
        }
        fight(&battle, &mut play, &mut actions);
        return;
    }
    match *screen {
        Screen::Map => {
            if play.snap(&mut commands, "map") {
                return;
            }
            run_actions.write(RunAction::Advance(run.choices()[0]));
        }
        Screen::Loot => {
            if play.snap(&mut commands, "loot") {
                return;
            }
            let offers = matches!(&run.stage, Stage::Loot(loot) if !loot.cards.is_empty() || !loot.modules.is_empty());
            run_actions.write(RunAction::TakeLoot(offers.then_some(0)));
        }
        Screen::Event => {
            if play.snap(&mut commands, "event") {
                return;
            }
            if let Stage::Event(event) = run.stage {
                let index = event
                    .options()
                    .iter()
                    .position(|&option| option != EventOption::PayPilot || run.scrap >= PILOT_PRICE)
                    .unwrap_or(0);
                run_actions.write(RunAction::Choose(index));
            }
        }
        Screen::EventOutcome => {
            if play.snap(&mut commands, "outcome") {
                return;
            }
            flow.write(RunFlow::Dismiss);
        }
        Screen::Shipyard => {
            if play.snap(&mut commands, "shipyard") {
                return;
            }
            run_actions.write(RunAction::Repair);
        }
        Screen::Market => {
            if play.snap(&mut commands, "market") {
                return;
            }
            run_actions.write(RunAction::LeaveMarket);
        }
        Screen::Ended => {
            info!("Rauchtest: Run vorbei, {:?}", run.result);
            play.finish(&mut commands, &mut exit);
        }
        Screen::Menu | Screen::Admirals | Screen::Battle | Screen::Models => {}
    }
}

/// Ein Schritt im Gefecht.
fn fight(battle: &Battle, play: &mut AutoPlay, actions: &mut MessageWriter<PlayerAction>) {
    match play.step {
        Step::Weapons => {
            if let Some(enemy) = battle.enemies.first() {
                actions.write(PlayerAction::SetTarget(enemy.uid));
                actions.write(PlayerAction::RunStation(Station::Weapons));
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
            play.played += 1;
            play.step = Step::Weapons;
        }
    }
}
