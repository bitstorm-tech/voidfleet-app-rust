//! Der Controller verbindet Gefecht, Szene und Oberfläche. Die Szene spiegelt das Gefecht nur
//! ([`crate::scene::sync`]); der Controller legt fest, was gewählt ist, was leuchtet, was unter dem Zeiger als Vorschau
//! steht und was das HUD sagt.
//!
//! - [`selection`]: die Auswahl als Zustandsmaschine ([`Selection`]), gültige Ziele und die Vorschau.
//!
//! Ablauf je Bild, nach dem Gefecht ([`GameSystems`]) und vor dem Abgleich der Szene ([`SyncSystems`]):
//!
//! 1. [`ControllerSystems::Game`] — ein neues Gefecht setzt Auswahl, Musik und Log zurück und lässt den Bordfunk
//!    melden. Ereignisse einer Aktion übergeben die Szene der Wiedergabe ([`BoardMode::Playback`], [`PlayEvents`]);
//!    meldet sie [`PlaybackFinished`], gehört die Szene wieder der Planung. Abgelehnte Aktionen erscheinen als Banner.
//! 2. [`ControllerSystems::Input`] — je Eingabeart ein System: Karten der Hand, Slots, Klick daneben (zweimal schnell:
//!    Vollbild), Rechtsklick und Escape brechen ab, Escape ohne Auswahl öffnet die Einstellungen, Leertaste
//!    überspringt, Stationen-Feld und Aktionsknöpfe. Admiralswahl, Ergebnis und Run laufen über [`crate::run`].
//! 3. [`ControllerSystems::Refresh`] — [`Highlights`], [`BoardFocus`] und [`HudState`] aus Gefecht, Auswahl und Zeiger.
//!
//! Dev-Ansichten, die Brett und HUD selbst stellen, setzen [`Scripted`]; dann ruht der Controller.

pub mod selection;

use bevy::{picking::hover::Hovered, platform::collections::HashSet, prelude::*};

pub use selection::{Outcome, Pick, Selection};

use crate::{
    assets::AppState,
    audio::{Music, MusicMood, PlayUiSound, RadioCue, Say, UiSound},
    game::{Battle, BattleEvents, BattleStarted, GameSystems, PlayerAction, RuleRejected},
    i18n::Locale,
    playback::{PlayEvents, PlaybackFinished, SkipPlayback},
    scene::{
        background::ChangeBackground,
        board::{BoardHover, Highlights},
        picking::{BoardMissed, SlotPressed},
        sync::{BoardFocus, BoardMode, SyncSystems},
    },
    ui::{
        ActionPressed, AppendLog, ClearLog, HandCardPressed, HudAction, HudState, ShowBanner,
        StationPressed,
        dialogs::{Dialog, DialogSystems, OpenDialog},
        fullscreen::ToggleFullscreen,
        stations::StationButton,
        theme::Modal,
    },
};

/// Zwei Klicks daneben innerhalb dieser Zeit sind ein Doppelklick (Vollbild).
const DOUBLE_CLICK: f64 = 0.3;

/// Dev-Ansichten stellen Gefecht, Brett und HUD selbst; der Controller ruht.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Scripted;

/// Reihenfolge der Systeme des Controllers.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControllerSystems {
    Game,
    Input,
    Refresh,
}

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        use ControllerSystems::{Game, Input, Refresh};
        app.init_resource::<Selection>()
            .configure_sets(
                Update,
                (Game, Input, Refresh)
                    .chain()
                    .after(GameSystems)
                    .before(SyncSystems)
                    .run_if(in_state(AppState::Match))
                    .run_if(not(resource_exists::<Scripted>)),
            )
            .add_systems(
                Update,
                (
                    (
                        begin_battle.run_if(on_message::<BattleStarted>),
                        start_playback.run_if(on_message::<BattleEvents>),
                        finish_playback.run_if(on_message::<PlaybackFinished>),
                        report_rejections.run_if(on_message::<RuleRejected>),
                    )
                        .in_set(Game),
                    (
                        press_hand_cards.run_if(on_message::<HandCardPressed>),
                        press_slots.run_if(on_message::<SlotPressed>),
                        miss_board.run_if(on_message::<BoardMissed>),
                        press_stations.run_if(on_message::<StationPressed>),
                        press_action.run_if(on_message::<ActionPressed>),
                        cancel_on_right_click,
                        escape.before(DialogSystems),
                        skip_on_space.run_if(in_state(BoardMode::Playback)),
                    )
                        .in_set(Input),
                    (
                        update_highlights.run_if(planning_changed),
                        update_focus.run_if(focus_changed),
                        update_hud.run_if(planning_changed),
                    )
                        .chain()
                        .in_set(Refresh),
                ),
            );
    }
}

/// Ob der Spieler gerade handeln darf: ein Gefecht läuft, keine Wiedergabe, nicht entschieden.
fn can_act(battle: Option<&Battle>, mode: BoardMode) -> bool {
    mode == BoardMode::Planning && battle.is_some_and(|battle| battle.result.is_none())
}

// --- Gefecht ---

/// Ein neues Gefecht: Auswahl und Log leer, Musik an, der Bordfunk meldet.
fn begin_battle(
    battle: Res<Battle>,
    locale: Res<Locale>,
    mut selection: ResMut<Selection>,
    mut music: ResMut<Music>,
    mut mode: ResMut<NextState<BoardMode>>,
    mut clear: MessageWriter<ClearLog>,
    mut log: MessageWriter<AppendLog>,
    mut say: MessageWriter<Say>,
    mut background: MessageWriter<ChangeBackground>,
) {
    selection.set_if_neq(Selection::Idle);
    music.set_if_neq(Music(Some(MusicMood::Battle)));
    mode.set(BoardMode::Planning);
    clear.write(ClearLog);
    log.write(AppendLog(vec![
        locale.log_battle_started(&locale.admiral_title(battle.flagship.admiral)),
        locale.log_round_header(battle.round),
    ]));
    say.write(Say::new(RadioCue::BattleStart));
    background.write(ChangeBackground);
}

/// Eine Aktion hat Ereignisse: Die Szene gehört der Wiedergabe, bis sie fertig ist. Nach Gefechtsende verstummt die
/// Musik.
fn start_playback(
    mut events: MessageReader<BattleEvents>,
    battle: Res<Battle>,
    mut mode: ResMut<NextState<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut music: ResMut<Music>,
    mut play: MessageWriter<PlayEvents>,
) {
    for BattleEvents(events) in events.read() {
        mode.set(BoardMode::Playback);
        play.write(PlayEvents(events.clone()));
    }
    selection.set_if_neq(Selection::Idle);
    if battle.result.is_some() {
        music.set_if_neq(Music(None));
    }
}

fn finish_playback(
    mut finished: MessageReader<PlaybackFinished>,
    mut mode: ResMut<NextState<BoardMode>>,
) {
    finished.clear();
    mode.set(BoardMode::Planning);
}

/// Abgelehnte Aktionen: kurz als Banner, dazu ein Fehlerklang.
fn report_rejections(
    mut rejected: MessageReader<RuleRejected>,
    locale: Res<Locale>,
    mut banner: MessageWriter<ShowBanner>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if let Some(&RuleRejected(error)) = rejected.read().last() {
        banner.write(ShowBanner {
            title: locale.rule_error(error).to_owned(),
            subtitle: None,
            admiral: None,
        });
        sounds.write(PlayUiSound(UiSound::Denied));
    }
}

// --- Eingaben ---

/// Lässt die Auswahl einen Klick verarbeiten: Aktion ans Gefecht, neue Auswahl oder Ablehnung.
fn handle_click(
    pick: Pick,
    battle: Option<&Battle>,
    mode: BoardMode,
    selection: &mut Selection,
    actions: &mut MessageWriter<PlayerAction>,
    rejected: &mut MessageWriter<RuleRejected>,
    sounds: &mut MessageWriter<PlayUiSound>,
) {
    let Some(battle) = battle.filter(|_| can_act(battle, mode)) else {
        return;
    };
    match selection::click(battle, *selection, pick) {
        Outcome::Select(next) => {
            if next != *selection {
                sounds.write(PlayUiSound(if next == Selection::Idle {
                    UiSound::Cancel
                } else {
                    UiSound::Select
                }));
            }
            *selection = next;
        }
        Outcome::Act(action) => {
            *selection = Selection::Idle;
            sounds.write(PlayUiSound(UiSound::Place));
            actions.write(action);
        }
        Outcome::Reject(error) => {
            rejected.write(RuleRejected(error));
        }
        Outcome::Nothing => {}
    }
}

fn press_hand_cards(
    mut pressed: MessageReader<HandCardPressed>,
    battle: Option<Res<Battle>>,
    mode: Res<State<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut actions: MessageWriter<PlayerAction>,
    mut rejected: MessageWriter<RuleRejected>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    for &HandCardPressed(index) in pressed.read() {
        handle_click(
            Pick::Card(index),
            battle.as_deref(),
            *mode.get(),
            &mut selection,
            &mut actions,
            &mut rejected,
            &mut sounds,
        );
    }
}

fn press_slots(
    mut pressed: MessageReader<SlotPressed>,
    battle: Option<Res<Battle>>,
    mode: Res<State<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut actions: MessageWriter<PlayerAction>,
    mut rejected: MessageWriter<RuleRejected>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    for &SlotPressed(slot) in pressed.read() {
        handle_click(
            Pick::Slot(slot),
            battle.as_deref(),
            *mode.get(),
            &mut selection,
            &mut actions,
            &mut rejected,
            &mut sounds,
        );
    }
}

/// Ein Klick daneben bricht die Auswahl ab; ein Doppelklick auf eine freie Stelle schaltet das Vollbild um.
fn miss_board(
    mut missed: MessageReader<BoardMissed>,
    time: Res<Time<Real>>,
    mut last: Local<Option<f64>>,
    mut selection: ResMut<Selection>,
    mut fullscreen: MessageWriter<ToggleFullscreen>,
) {
    for _ in missed.read() {
        selection.set_if_neq(Selection::Idle);
        let now = time.elapsed_secs_f64();
        if last.is_some_and(|then| now - then < DOUBLE_CLICK) {
            fullscreen.write(ToggleFullscreen);
            *last = None;
        } else {
            *last = Some(now);
        }
    }
}

/// Eine Station im Stationen-Feld: Sie läuft sofort (wie eine gespielte Karte), eine Kartenwahl geht.
fn press_stations(
    mut pressed: MessageReader<StationPressed>,
    battle: Option<Res<Battle>>,
    mode: Res<State<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut actions: MessageWriter<PlayerAction>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    for &StationPressed(station) in pressed.read() {
        if can_act(battle.as_deref(), *mode.get()) {
            selection.set_if_neq(Selection::Idle);
            // Eine Ablehnung meldet sich mit ihrem eigenen Ton.
            if battle
                .as_deref()
                .is_some_and(|battle| battle.can_run(station).is_ok())
            {
                sounds.write(PlayUiSound(UiSound::Place));
            }
            actions.write(PlayerAction::RunStation(station));
        }
    }
}

/// Die Aktionsknöpfe: Zug beenden, Wiedergabe überspringen.
fn press_action(
    mut pressed: MessageReader<ActionPressed>,
    battle: Option<Res<Battle>>,
    mode: Res<State<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut actions: MessageWriter<PlayerAction>,
    mut skip: MessageWriter<SkipPlayback>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    for &ActionPressed(action) in pressed.read() {
        match action {
            HudAction::Skip => {
                skip.write(SkipPlayback);
            }
            _ if !can_act(battle.as_deref(), *mode.get()) => {}
            HudAction::EndTurn => {
                selection.set_if_neq(Selection::Idle);
                sounds.write(PlayUiSound(UiSound::Confirm));
                actions.write(PlayerAction::EndTurn);
            }
        }
    }
}

/// Ein Rechtsklick bricht eine angefangene Auswahl ab, außer bei offenem Dialog.
fn cancel_on_right_click(
    mouse: Res<ButtonInput<MouseButton>>,
    modal: Query<(), With<Modal>>,
    mut selection: ResMut<Selection>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if mouse.just_pressed(MouseButton::Right) && modal.is_empty() && *selection != Selection::Idle {
        *selection = Selection::Idle;
        sounds.write(PlayUiSound(UiSound::Cancel));
    }
}

/// Escape bricht eine Auswahl ab, sonst öffnet es die Einstellungen. Ein offener Dialog geht vor: Den schließt Escape
/// selbst.
fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    modal: Query<(), With<Modal>>,
    mut selection: ResMut<Selection>,
    mut open: MessageWriter<OpenDialog>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if !keys.just_pressed(KeyCode::Escape) || !modal.is_empty() {
        return;
    }
    if *selection == Selection::Idle {
        open.write(OpenDialog(Dialog::Settings));
    } else {
        *selection = Selection::Idle;
        sounds.write(PlayUiSound(UiSound::Cancel));
    }
}

/// Die Leertaste überspringt die laufende Wiedergabe.
fn skip_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    modal: Query<(), With<Modal>>,
    mut skip: MessageWriter<SkipPlayback>,
) {
    if keys.just_pressed(KeyCode::Space) && modal.is_empty() {
        skip.write(SkipPlayback);
    }
}

// --- Darstellung ---

/// Ob sich etwas geändert hat, wovon Hervorhebung und HUD abhängen. Bedingungen werden auch außerhalb des Gefechts
/// ausgewertet; dort fehlen [`Battle`] und [`BoardMode`].
fn planning_changed(
    battle: Option<Res<Battle>>,
    selection: Res<Selection>,
    mode: Option<Res<State<BoardMode>>>,
) -> bool {
    battle.is_none_or(|battle| battle.is_changed())
        || selection.is_changed()
        || mode.is_some_and(|mode| mode.is_changed())
}

/// Dazu der Zeiger, auf dem Brett und auf den Stationen.
fn focus_changed(
    battle: Option<Res<Battle>>,
    selection: Res<Selection>,
    mode: Option<Res<State<BoardMode>>>,
    hover: Res<BoardHover>,
    stations: Query<(), (Changed<Hovered>, With<StationButton>)>,
) -> bool {
    planning_changed(battle, selection, mode) || hover.is_changed() || !stations.is_empty()
}

fn update_highlights(
    battle: Option<Res<Battle>>,
    selection: Res<Selection>,
    mode: Res<State<BoardMode>>,
    mut highlights: ResMut<Highlights>,
) {
    let next = match battle.as_deref() {
        Some(battle) if can_act(Some(battle), *mode.get()) => {
            // Pads nur beim Manövrieren des Spielers; Gegner als Ziel zeigen ihre Schiffe.
            let slots: HashSet<_> = selection::highlights(battle, *selection)
                .into_iter()
                .filter(|slot| slot.side == crate::scene::coords::Side::Player)
                .collect();
            Highlights {
                grid: !slots.is_empty(),
                slots,
            }
        }
        _ => Highlights::default(),
    };
    highlights.set_if_neq(next);
}

/// Unter dem Zeiger die Treffer der gewählten Karte bzw. der Waffen; zeigt er auf eine Station, deren Treffer.
fn update_focus(
    battle: Option<Res<Battle>>,
    selection: Res<Selection>,
    mode: Res<State<BoardMode>>,
    hover: Res<BoardHover>,
    stations: Query<(&Hovered, &StationButton)>,
    mut focus: ResMut<BoardFocus>,
) {
    let station = stations
        .iter()
        .find_map(|(hovered, &StationButton(station))| hovered.get().then_some(station));
    let preview = battle
        .as_deref()
        .filter(|battle| can_act(Some(battle), *mode.get()))
        .and_then(|battle| match station {
            Some(station) => selection::station_preview(battle, station),
            None => selection::preview(battle, *selection, hover.slot),
        });
    focus.set_if_neq(BoardFocus { preview });
}

fn update_hud(
    battle: Option<Res<Battle>>,
    selection: Res<Selection>,
    mode: Res<State<BoardMode>>,
    mut hud: ResMut<HudState>,
) {
    hud.set_if_neq(HudState {
        planning: can_act(battle.as_deref(), *mode.get()),
        playing_back: *mode.get() == BoardMode::Playback,
        selected: match *selection {
            Selection::Card(index) => Some(index),
            Selection::Idle => None,
        },
    });
}
