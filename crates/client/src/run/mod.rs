//! Der Run aus Sicht des Clients (`docs/run.md`): die Ressource [`ActiveRun`] um [`voidfleet_core::run::Run`],
//! was gerade zu sehen ist ([`Screen`]) und der Speicherstand ([`save`]). Die Regeln stehen im Core; hier wird nur
//! gerufen, gespeichert und das Gefecht eines Knotens gestartet.
//!
//! Ablauf: Menü und Overlays melden [`RunFlow`] (neuer Run, weiter, Ergebnis gesehen, abbrechen) und [`RunAction`]
//! (alles, was den Run ändert); [`RunSystems`] führt sie aus, vor dem Gefecht ([`GameSystems`]). Steht ein Gefecht an
//! und läuft keins, geht [`StartBattle`] mit `run.start_battle()` hinaus. Meldet die Wiedergabe nach Gefechtsende
//! [`PlaybackFinished`], übernimmt `finish_battle` das Gefecht; „Weiter“ im Ergebnis ([`LeaveBattle`]) führt dann zur
//! Beute oder zum Run-Ende. Nach jeder Änderung von [`ActiveRun`] wird gespeichert, nach dem Ende gelöscht; beim Start
//! geht es ohne Hauptmenü mit dem Speicherstand weiter.

pub mod save;

use bevy::prelude::*;
use voidfleet_core::{
    admirals::AdmiralId,
    run::{EventOption, Gain, Run, RunError, Stage},
};

pub use save::RunSave;

use crate::{
    assets::AppState,
    audio::{Music, MusicMood, PlayUiSound, UiSound},
    game::{Battle, GameSystems, LeaveBattle, StartBattle},
    i18n::Locale,
    playback::PlaybackFinished,
    ui::{ShowBanner, dialogs::AdmiralChosen},
};

/// Der laufende Run; fehlt im Hauptmenü und in der Admiralswahl.
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct ActiveRun(pub Run);

/// Was den Run ändert; jede Wahl auf Karte, Beute, Ereignis, Werft und Markt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction {
    Advance(usize),
    TakeLoot(Option<usize>),
    Choose(usize),
    Repair,
    Upgrade(usize),
    Remove(usize),
    BuyCard(usize),
    BuyModule(usize),
    BuyRemoval(usize),
    BuyRepair,
    LeaveMarket,
}

/// Der Weg um den Run herum.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunFlow {
    /// „Neuer Run“: ein laufender Run endet, die Admiralswahl öffnet.
    NewRun,
    /// „Weiter“ im Hauptmenü: der Speicherstand.
    Continue,
    /// Das Ergebnis eines Ereignisses ist gesehen.
    Dismiss,
    /// Run abbrechen: Speicherstand weg, zurück ins Hauptmenü.
    Abandon,
}

/// Was eine Entscheidung im Ereignis gebracht hat, zum Zeigen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventOutcome {
    pub option: EventOption,
    pub gain: Gain,
    /// Änderung der Hülle und des Schrotts.
    pub hull: i32,
    pub scrap: i64,
}

/// Was außer dem Run bestimmt, was zu sehen ist.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunView {
    /// Die Admiralswahl ist offen.
    pub choosing: bool,
    /// Das Ergebnis des letzten Ereignisses, bis es gesehen ist.
    pub outcome: Option<EventOutcome>,
    /// Die Modellschau ist offen (Einstellungen → Dev); sie liegt über allem, auch über einem laufenden Run.
    pub models: bool,
}

/// Was gerade zu sehen ist.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Menu,
    Admirals,
    Map,
    Loot,
    Event,
    EventOutcome,
    Shipyard,
    Market,
    /// Ein Gefecht läuft (oder wird gerade gestartet), samt seinem Ergebnis.
    Battle,
    /// Der Run ist gewonnen oder verloren.
    Ended,
    /// Die Modellschau ([`crate::dev::viewer`]).
    Models,
}

/// Der Bildschirm zu Run, laufendem Gefecht und Ansicht.
pub fn screen(run: Option<&Run>, battle: bool, view: &RunView) -> Screen {
    if view.models {
        return Screen::Models;
    }
    let Some(run) = run else {
        return if view.choosing {
            Screen::Admirals
        } else {
            Screen::Menu
        };
    };
    if battle {
        return Screen::Battle;
    }
    if run.result.is_some() {
        return Screen::Ended;
    }
    if view.outcome.is_some() {
        return Screen::EventOutcome;
    }
    match run.stage {
        Stage::Map => Screen::Map,
        Stage::Battle { .. } => Screen::Battle,
        Stage::Loot(_) => Screen::Loot,
        Stage::Event(_) => Screen::Event,
        Stage::Shipyard => Screen::Shipyard,
        Stage::Market(_) => Screen::Market,
    }
}

/// Führt eine Wahl am Run aus; eine Entscheidung im Ereignis liefert, was sie gebracht hat.
pub fn apply(run: &mut Run, action: RunAction) -> Result<Option<EventOutcome>, RunError> {
    match action {
        RunAction::Advance(column) => run.advance(column),
        RunAction::TakeLoot(index) => run.take_loot(index),
        RunAction::Choose(index) => {
            let Stage::Event(event) = run.stage else {
                return Err(RunError::WrongStage);
            };
            let option = *event.options().get(index).ok_or(RunError::OutOfRange)?;
            let (hull, scrap) = (run.hull, run.scrap);
            let gain = run.choose(index)?;
            return Ok(Some(EventOutcome {
                option,
                gain,
                hull: run.hull - hull,
                scrap: i64::from(run.scrap) - i64::from(scrap),
            }));
        }
        RunAction::Repair => run.repair_at_shipyard(),
        RunAction::Upgrade(index) => run.upgrade_at_shipyard(index),
        RunAction::Remove(index) => run.remove_at_shipyard(index),
        RunAction::BuyCard(index) => run.buy_card(index),
        RunAction::BuyModule(index) => run.buy_module(index),
        RunAction::BuyRemoval(index) => run.buy_removal(index),
        RunAction::BuyRepair => run.buy_repair(),
        RunAction::LeaveMarket => run.leave_market(),
    }
    .map(|()| None)
}

/// Seed eines neuen Runs aus der Systemzeit.
fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos() as u64)
}

/// Hier läuft der Run, vor dem Gefecht.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunSystems;

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunSave>()
            .init_resource::<RunView>()
            .init_resource::<Screen>()
            .add_message::<RunAction>()
            .add_message::<RunFlow>()
            // Beim Start geht es mit dem Speicherstand weiter, ohne Hauptmenü; ohne Speicherstand bleibt es.
            .add_systems(
                OnEnter(AppState::Match),
                |mut flow: MessageWriter<RunFlow>| {
                    flow.write(RunFlow::Continue);
                },
            )
            .configure_sets(
                Update,
                RunSystems
                    .before(GameSystems)
                    .run_if(in_state(AppState::Match)),
            )
            .add_systems(
                Update,
                (
                    follow_flow,
                    run_actions.run_if(on_message::<RunAction>),
                    finish_battle.run_if(on_message::<PlaybackFinished>),
                    start_battle,
                    save_run.run_if(resource_exists_and_changed::<ActiveRun>),
                    update_screen,
                )
                    .chain()
                    .in_set(RunSystems),
            );
    }
}

fn follow_flow(
    mut commands: Commands,
    mut flow: MessageReader<RunFlow>,
    mut admirals: MessageReader<AdmiralChosen>,
    save: Res<RunSave>,
    mut view: ResMut<RunView>,
    mut leave: MessageWriter<LeaveBattle>,
) {
    for &step in flow.read() {
        match step {
            RunFlow::NewRun | RunFlow::Abandon => {
                commands.remove_resource::<ActiveRun>();
                save.clear();
                leave.write(LeaveBattle);
                *view = RunView {
                    choosing: step == RunFlow::NewRun,
                    ..RunView::default()
                };
            }
            RunFlow::Continue => {
                if let Some(run) = save.load() {
                    info!(
                        "Run geladen: Sektor {}, Reihe {}",
                        run.sector,
                        run.path.len()
                    );
                    commands.insert_resource(ActiveRun(run));
                }
            }
            RunFlow::Dismiss => view.outcome = None,
        }
    }
    if let Some(&AdmiralChosen(admiral)) = admirals.read().last() {
        commands.insert_resource(new_run(admiral, time_seed()));
        view.choosing = false;
    }
}

/// Ein neuer Run.
pub fn new_run(admiral: AdmiralId, seed: u64) -> ActiveRun {
    info!("Neuer Run: {admiral:?}, Seed {seed}");
    ActiveRun(Run::new(admiral, seed))
}

/// Führt die Wahlen dieses Bilds aus; nach dem Boss kündigt ein Banner den neuen Sektor an.
fn run_actions(
    mut actions: MessageReader<RunAction>,
    run: Option<ResMut<ActiveRun>>,
    locale: Res<Locale>,
    mut view: ResMut<RunView>,
    mut banner: MessageWriter<ShowBanner>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let Some(mut run) = run else {
        actions.clear();
        return;
    };
    for &action in actions.read() {
        let sector = run.sector;
        match apply(&mut run, action) {
            Ok(outcome) => {
                if outcome.is_some() {
                    view.outcome = outcome;
                }
                if run.sector != sector {
                    banner.write(ShowBanner {
                        title: locale.map_title(run.sector),
                        subtitle: Some(locale.sector_banner_hint().to_owned()),
                        admiral: None,
                    });
                }
            }
            Err(error) => {
                warn!("Run lehnt ab: {action:?} → {error:?}");
                sounds.write(PlayUiSound(UiSound::Denied));
            }
        }
    }
}

/// Nach dem Gefecht, sobald die Wiedergabe fertig ist: Der Run übernimmt Hülle, Schrott und Beute.
fn finish_battle(
    mut finished: MessageReader<PlaybackFinished>,
    battle: Option<Res<Battle>>,
    run: Option<ResMut<ActiveRun>>,
) {
    finished.clear();
    let (Some(battle), Some(mut run)) = (battle, run) else {
        return;
    };
    if battle.result.is_some() && matches!(run.stage, Stage::Battle { .. }) {
        let loot = run.finish_battle(&battle);
        info!("Gefecht beendet: {:?}, Beute {loot:?}", battle.result);
    }
}

/// Steht ein Gefecht an und läuft keins, beginnt es (auch nach dem Laden: derselbe Seed, dieselbe Formation).
fn start_battle(
    run: Option<Res<ActiveRun>>,
    battle: Option<Res<Battle>>,
    mut start: MessageWriter<StartBattle>,
) {
    if let Some(run) = run
        && battle.is_none()
        && run.result.is_none()
        && matches!(run.stage, Stage::Battle { .. })
    {
        start.write(StartBattle(Box::new(run.start_battle())));
    }
}

fn save_run(run: Res<ActiveRun>, save: Res<RunSave>) {
    if run.result.is_some() {
        save.clear();
    } else if let Err(error) = save.store(&run) {
        error!("Run nicht gespeichert: {error}");
    }
}

/// Hält [`Screen`] aktuell; außerhalb des Gefechts spielt die ruhige Musik.
fn update_screen(
    run: Option<Res<ActiveRun>>,
    battle: Option<Res<Battle>>,
    view: Res<RunView>,
    mut current: ResMut<Screen>,
    mut music: ResMut<Music>,
) {
    let next = screen(run.as_deref().map(|run| &run.0), battle.is_some(), &view);
    if *current != next {
        info!("Bildschirm: {next:?}");
        *current = next;
        if next != Screen::Battle {
            music.set_if_neq(Music(Some(MusicMood::Calm)));
        }
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{battle::BattleResult, run::NodeKind};

    use super::*;

    fn run() -> Run {
        Run::new(AdmiralId::Osei, 3)
    }

    #[test]
    fn screens_around_the_run() {
        let view = RunView::default();
        assert_eq!(screen(None, false, &view), Screen::Menu);
        let choosing = RunView {
            choosing: true,
            ..view
        };
        assert_eq!(screen(None, false, &choosing), Screen::Admirals);
        let mut run = run();
        assert_eq!(screen(Some(&run), false, &view), Screen::Map);
        run.result = Some(BattleResult::Lost);
        assert_eq!(screen(Some(&run), false, &view), Screen::Ended);
        assert_eq!(screen(Some(&run), true, &view), Screen::Battle);
        let models = RunView {
            models: true,
            ..view
        };
        assert_eq!(screen(Some(&run), true, &models), Screen::Models);
    }

    /// Erste Reihe: immer Gefechte. Sieg → Beute → Karte, die nächste Reihe ist wählbar.
    #[test]
    fn battle_loot_map() {
        let mut run = run();
        let view = RunView::default();
        let column = run.choices()[0];
        assert_eq!(run.map[0][column].kind, NodeKind::Battle);
        apply(&mut run, RunAction::Advance(column)).unwrap();
        assert_eq!(screen(Some(&run), false, &view), Screen::Battle);
        assert_eq!(
            apply(&mut run, RunAction::Advance(column)),
            Err(RunError::WrongStage)
        );
        let mut battle = run.start_battle();
        battle.result = Some(BattleResult::Won);
        let loot = run.finish_battle(&battle);
        assert_eq!(loot.cards.len(), 3);
        assert_eq!(screen(Some(&run), false, &view), Screen::Loot);
        apply(&mut run, RunAction::TakeLoot(Some(0))).unwrap();
        assert_eq!(run.deck.last().unwrap().id, loot.cards[0]);
        assert_eq!(screen(Some(&run), false, &view), Screen::Map);
        assert_eq!(run.choices(), run.map[0][column].next);
    }

    /// Ein Ereignis meldet, was es gebracht hat; das Ergebnis steht vor der Karte bzw. dem Kartenangebot.
    #[test]
    fn event_outcome() {
        let mut run = run();
        run.path = vec![0];
        run.stage = Stage::Event(voidfleet_core::run::EventId::Resolute);
        let outcome = apply(&mut run, RunAction::Choose(0)).unwrap().unwrap();
        assert_eq!(outcome.option, EventOption::Salvage);
        assert_eq!(
            outcome.scrap,
            i64::from(voidfleet_core::run::RESOLUTE_SCRAP)
        );
        let view = RunView {
            outcome: Some(outcome),
            ..RunView::default()
        };
        assert_eq!(screen(Some(&run), false, &view), Screen::EventOutcome);
        run.stage = Stage::Event(voidfleet_core::run::EventId::Pilot);
        let outcome = apply(&mut run, RunAction::Choose(1)).unwrap().unwrap();
        assert_eq!(outcome.hull, -voidfleet_core::run::PILOT_DAMAGE);
        assert_eq!(screen(Some(&run), false, &RunView::default()), Screen::Loot);
    }
}
