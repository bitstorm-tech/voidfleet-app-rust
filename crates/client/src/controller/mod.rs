//! Der Match-Controller (Port von `matchController.ts`): verbindet Server, Spielzustand, Szene und Oberfläche. Die
//! Szene spiegelt den Zustand nur ([`crate::scene::sync`]); der Controller legt fest, was gewählt ist, was leuchtet,
//! was unter dem Zeiger als Vorschau steht und was das HUD sagt.
//!
//! - [`planner`]: die Auswahl als Zustandsmaschine ([`Selection`]) und was ein Klick je Zustand bedeutet.
//! - [`status`]: die Statuszeile.
//!
//! Ablauf je Bild, nach der Übernahme der Server-Nachricht ([`ApplySet`]) und vor dem Abgleich der Szene
//! ([`SyncSystems`]):
//!
//! 1. [`ControllerSystems::Server`] — ein neuer Zustand setzt die Auswahl zurück, stellt die Musik, kündigt eine neue
//!    Runde an und lässt den Bordfunk melden (Gefechtsbeginn, Overdrive, letztes Kriegsschiff, Gegner hat abgegeben,
//!    kurz nach der Fanfare der Ausgang). Eine aufgelöste Runde hält die Warteschlange an und übergibt die Szene der
//!    Wiedergabe ([`BoardMode::Playback`], [`PlayRound`]); ist sie fertig ([`PlaybackFinished`]), gibt er die
//!    Warteschlange frei und schaltet auf [`BoardMode::Planning`], sobald der neue Zustand da ist.
//!    Fehler des Servers landen im Log.
//! 2. [`ControllerSystems::Input`] — je Eingabeart ein System: Karten der Hand, Fähigkeit, Zielerfassung, Slots,
//!    Spaltenpfeile, Klick daneben (zweimal schnell: Vollbild), Rechtsklick, Escape (vor den Dialogen), Leertaste und
//!    Aktionsknopf, die Wahl aus Admiralswahl und Nachschub, Aufgeben und neues Match.
//! 3. [`ControllerSystems::Refresh`] — [`Highlights`], [`BoardFocus`] und [`HudState`] aus Zustand, Auswahl und Zeiger.
//!
//! Dev-Ansichten, die Brett und HUD selbst stellen, setzen [`Scripted`]; dann ruht der Controller.

pub mod planner;
pub mod status;

use std::time::Duration;

use bevy::{ecs::system::SystemParam, prelude::*};
use voidfleet_core::{
    board::overdrive_bonus,
    protocol::{ClientMessage, MatchResult},
};

pub use planner::{Escape, Notice, Notices, Planner, Selection};
use status::{StatusView, status_line};

use crate::{
    assets::AppState,
    audio::{Music, MusicMood, PlayUiSound, RadioCue, Say, UiSound},
    game::{ApplySet, Match, RoundResolved, ServerError, ServerInbox, StateChanged, preview_round},
    i18n::Locale,
    net::{ConnectionLost, ConnectionStatus, Server, ToServer},
    playback::{PlayRound, PlaybackFinished, SkipPlayback},
    scene::{
        background::ChangeBackground,
        board::{BoardHover, Highlights},
        picking::{BoardMissed, ColumnPressed, SlotPressed},
        sync::{BoardFocus, BoardMode, SyncSystems},
    },
    ui::{
        ActionPressed, AppendLog, HandCardPressed, HudAction, HudState, MarkPressed, PowerPressed,
        ShowRoundBanner,
        dialogs::{
            AdmiralChosen, Dialog, DialogSystems, DraftChosen, NewMatch, OpenDialog,
            SurrenderConfirmed,
        },
        fullscreen::ToggleFullscreen,
        hand::HandBar,
        theme::Modal,
    },
};

/// Den Ausgang meldet der Bordfunk kurz nach dem Einsatz der Fanfare.
const RESULT_REPORT_DELAY: Duration = Duration::from_millis(1500);
/// Zwei Klicks daneben innerhalb dieser Zeit sind ein Doppelklick (Vollbild).
const DOUBLE_CLICK: f64 = 0.3;

/// Dev-Ansichten stellen Spielzustand, Brett und HUD selbst; der Controller ruht.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Scripted;

/// Reihenfolge der Systeme des Controllers.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControllerSystems {
    Server,
    Input,
    Refresh,
}

/// Was beim letzten Zustand galt, für die Meldungen des Bordfunks und die Ankündigung einer neuen Runde.
#[derive(Resource, Clone, Copy, Debug, Default)]
struct Reported {
    round: u32,
    opponent_admiral: bool,
    opponent_submitted: bool,
    result: Option<MatchResult>,
    /// Eigene Kriegsschiffe auf Brett und Hand, für die Meldung, dass nur noch eines übrig ist.
    warships: usize,
}

impl Reported {
    fn of(game: &Match) -> Self {
        Self {
            round: game.round,
            opponent_admiral: game
                .opponent
                .as_ref()
                .is_some_and(|opponent| opponent.admiral.is_some()),
            opponent_submitted: game
                .opponent
                .as_ref()
                .is_some_and(|opponent| opponent.submitted),
            result: game.result,
            warships: game.warships(),
        }
    }
}

/// Die Wiedergabe ist fertig, die Warteschlange frei; die Szene wartet auf den neuen Zustand.
#[derive(Resource, Debug, Default)]
struct AwaitingState;

/// Der Funkspruch zum Ausgang, sobald die Fanfare eingesetzt hat.
#[derive(Resource, Debug)]
struct ResultReport {
    timer: Timer,
    cue: RadioCue,
}

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        use ControllerSystems::{Input, Refresh, Server};
        app.init_resource::<Selection>()
            .init_resource::<Notices>()
            .init_resource::<Reported>()
            .configure_sets(
                Update,
                (Server, Input, Refresh)
                    .chain()
                    .after(ApplySet)
                    .before(SyncSystems)
                    .run_if(in_state(AppState::Match))
                    .run_if(not(resource_exists::<Scripted>)),
            )
            .add_systems(
                Update,
                (
                    (
                        follow_state.run_if(on_message::<StateChanged>),
                        start_playback.run_if(on_message::<RoundResolved>),
                        finish_playback.run_if(on_message::<PlaybackFinished>),
                        resume_planning
                            .run_if(resource_exists::<AwaitingState>)
                            .run_if(
                                on_message::<StateChanged>.or_eager(on_message::<ConnectionLost>),
                            ),
                        lose_connection.run_if(on_message::<ConnectionLost>),
                        log_errors.run_if(on_message::<ServerError>),
                        report_result.run_if(resource_exists::<ResultReport>),
                    )
                        .in_set(Server),
                    (
                        press_hand_cards.run_if(on_message::<HandCardPressed>),
                        press_power.run_if(on_message::<PowerPressed>),
                        press_mark.run_if(on_message::<MarkPressed>),
                        press_slots.run_if(on_message::<SlotPressed>),
                        press_columns.run_if(on_message::<ColumnPressed>),
                        miss_board.run_if(on_message::<BoardMissed>),
                        cancel_on_right_click,
                        escape.before(DialogSystems),
                        skip_on_space.run_if(in_state(BoardMode::Playback)),
                        press_action.run_if(on_message::<ActionPressed>),
                        forward_choices,
                        start_new_match.run_if(on_message::<NewMatch>),
                    )
                        .in_set(Input),
                    (
                        update_highlights.run_if(planning_changed),
                        update_focus.run_if(focus_changed),
                        update_hud.run_if(focus_changed),
                        hide_hand.run_if(resource_changed::<Match>),
                    )
                        .chain()
                        .in_set(Refresh),
                ),
            );
    }
}

/// Ob gerade geplant werden darf: verbunden, keine Wiedergabe, beide Admirale gewählt, der Zug dieser Runde offen, der
/// Nachschub gewählt und das Match nicht entschieden.
fn can_plan(game: &Match, connection: ConnectionStatus, mode: BoardMode) -> bool {
    connection == ConnectionStatus::Connected && mode == BoardMode::Planning && game.turn_open()
}

/// Was ein Klick zum Planen braucht.
#[derive(SystemParam)]
struct Desk<'w> {
    game: ResMut<'w, Match>,
    selection: ResMut<'w, Selection>,
    notices: ResMut<'w, Notices>,
    connection: Res<'w, ConnectionStatus>,
    mode: Res<'w, State<BoardMode>>,
    sounds: MessageWriter<'w, PlayUiSound>,
}

impl Desk<'_> {
    /// Lässt den Planer einen Klick verarbeiten und spielt den Klang dazu.
    fn act(&mut self, click: impl FnOnce(&mut Planner) -> Option<UiSound>) {
        let can_plan = can_plan(&self.game, *self.connection, *self.mode.get());
        let mut planner = Planner {
            game: &mut self.game,
            selection: &mut self.selection,
            notices: &mut self.notices,
            can_plan,
        };
        if let Some(sound) = click(&mut planner) {
            self.sounds.write(PlayUiSound(sound));
        }
    }
}

// --- Server ---

/// Ein neuer Zustand: Auswahl und Hinweise gehen, die Musik folgt der Phase, eine neue Runde wird angekündigt, und der
/// Bordfunk meldet, was sich zwischen den Runden getan hat.
fn follow_state(
    mut commands: Commands,
    game: Res<Match>,
    mut reported: ResMut<Reported>,
    mut selection: ResMut<Selection>,
    mut notices: ResMut<Notices>,
    mut music: ResMut<Music>,
    mut round_banner: MessageWriter<ShowRoundBanner>,
    mut say: MessageWriter<Say>,
) {
    let before = *reported;
    *reported = Reported::of(&game);
    selection.set_if_neq(Selection::Idle);
    notices.set_if_neq(Notices::default());
    music.set_if_neq(Music(game.result.is_none().then_some(MusicMood::Calm)));
    if let Some(result) = game.result {
        if before.result.is_none() {
            commands.insert_resource(ResultReport {
                timer: Timer::new(RESULT_REPORT_DELAY, TimerMode::Once),
                cue: result_cue(result, game.surrendered),
            });
        }
        return;
    }
    // Runde 1 versteht sich von selbst, ebenso der erste Zustand nach dem Start.
    let new_round = before.round > 0 && game.round > before.round;
    if new_round {
        round_banner.write(ShowRoundBanner(game.round));
    }
    let now = *reported;
    if !before.opponent_admiral && now.opponent_admiral && game.round == 1 {
        say.write(Say::new(RadioCue::BattleStart));
    }
    let overdrive = overdrive_bonus(game.round);
    if new_round && overdrive > overdrive_bonus(game.round - 1) {
        say.write(Say::new(if overdrive == 1 {
            RadioCue::Overdrive
        } else {
            RadioCue::OverdriveUp
        }));
    }
    if now.warships == 1 && before.warships > 1 {
        say.write(Say::new(RadioCue::LastShip));
    }
    if !before.opponent_submitted && now.opponent_submitted && !game.submitted {
        say.write(Say::new(RadioCue::EnemyReady));
    }
}

/// Der Funkspruch zum Ausgang; nach einer Aufgabe meldet er den Rückzug.
fn result_cue(result: MatchResult, surrendered: bool) -> RadioCue {
    match (result, surrendered) {
        (MatchResult::Draw, _) => RadioCue::Draw,
        (MatchResult::Won, false) => RadioCue::Victory,
        (MatchResult::Won, true) => RadioCue::EnemySurrender,
        (MatchResult::Lost, false) => RadioCue::Defeat,
        (MatchResult::Lost, true) => RadioCue::Retreat,
    }
}

fn report_result(
    mut commands: Commands,
    time: Res<Time>,
    mut report: ResMut<ResultReport>,
    mut say: MessageWriter<Say>,
) {
    if report.timer.tick(time.delta()).just_finished() {
        say.write(Say::new(report.cue));
        commands.remove_resource::<ResultReport>();
    }
}

/// Die Runde ist aufgelöst: Die Warteschlange wartet, bis sie abgespielt ist; eine offene Auswahl muss weichen (wer als
/// Zweiter abgibt, bekommt die Runde ohne Zwischenstand).
fn start_playback(
    mut resolved: MessageReader<RoundResolved>,
    mut inbox: ResMut<ServerInbox>,
    mut mode: ResMut<NextState<BoardMode>>,
    mut selection: ResMut<Selection>,
    mut music: ResMut<Music>,
    mut play: MessageWriter<PlayRound>,
) {
    for RoundResolved { round, events } in resolved.read() {
        inbox.hold();
        mode.set(BoardMode::Playback);
        selection.set_if_neq(Selection::Idle);
        music.set_if_neq(Music(Some(MusicMood::Battle)));
        play.write(PlayRound {
            round: *round,
            events: events.clone(),
        });
    }
}

/// Die Wiedergabe ist fertig: Der neue Zustand darf kommen. Die Szene gehört erst der Planung, wenn er da ist
/// ([`resume_planning`]); sonst gliche sie sich noch dem Brett von vor der Runde an.
fn finish_playback(
    mut commands: Commands,
    mut finished: MessageReader<PlaybackFinished>,
    mut inbox: ResMut<ServerInbox>,
) {
    finished.clear();
    inbox.release();
    commands.init_resource::<AwaitingState>();
}

/// Der Zustand nach der Wiedergabe ist da (oder die Verbindung weg, dann kommt keiner mehr): zurück zur Planung.
fn resume_planning(mut commands: Commands, mut mode: ResMut<NextState<BoardMode>>) {
    commands.remove_resource::<AwaitingState>();
    mode.set(BoardMode::Planning);
}

fn lose_connection(
    mut lost: MessageReader<ConnectionLost>,
    mut selection: ResMut<Selection>,
    mut notices: ResMut<Notices>,
) {
    lost.clear();
    selection.set_if_neq(Selection::Idle);
    notices.set_if_neq(Notices::default());
}

fn log_errors(
    mut errors: MessageReader<ServerError>,
    locale: Res<Locale>,
    mut log: MessageWriter<AppendLog>,
) {
    for ServerError(code) in errors.read() {
        log.write(AppendLog(vec![format!("Server: {}", locale.error(*code))]));
    }
}

// --- Eingaben ---

fn press_hand_cards(mut pressed: MessageReader<HandCardPressed>, mut desk: Desk) {
    for &HandCardPressed(card) in pressed.read() {
        desk.act(|planner| Some(planner.press_hand_card(card)));
    }
}

fn press_power(mut pressed: MessageReader<PowerPressed>, mut desk: Desk) {
    for _ in pressed.read() {
        desk.act(|planner| Some(planner.press_power()));
    }
}

fn press_mark(mut pressed: MessageReader<MarkPressed>, mut desk: Desk) {
    for _ in pressed.read() {
        desk.act(|planner| Some(planner.press_mark()));
    }
}

fn press_slots(mut pressed: MessageReader<SlotPressed>, mut desk: Desk) {
    for &SlotPressed(slot) in pressed.read() {
        desk.act(|planner| planner.press_slot(slot));
    }
}

fn press_columns(mut pressed: MessageReader<ColumnPressed>, mut desk: Desk) {
    for &ColumnPressed(column) in pressed.read() {
        desk.act(|planner| planner.press_column(column));
    }
}

/// Ein Klick daneben bricht die Auswahl ab; ein Doppelklick auf eine freie Stelle schaltet das Vollbild um.
fn miss_board(
    mut missed: MessageReader<BoardMissed>,
    time: Res<Time<Real>>,
    mut last: Local<Option<f64>>,
    mut desk: Desk,
    mut fullscreen: MessageWriter<ToggleFullscreen>,
) {
    for _ in missed.read() {
        desk.act(|planner| planner.miss());
        let now = time.elapsed_secs_f64();
        if last.is_some_and(|then| now - then < DOUBLE_CLICK) {
            fullscreen.write(ToggleFullscreen);
            *last = None;
        } else {
            *last = Some(now);
        }
    }
}

/// Ein Rechtsklick bricht eine angefangene Auswahl ab, außer bei offenem Dialog.
fn cancel_on_right_click(
    mouse: Res<ButtonInput<MouseButton>>,
    modal: Query<(), With<Modal>>,
    mut desk: Desk,
) {
    if mouse.just_pressed(MouseButton::Right) && modal.is_empty() {
        desk.act(|planner| planner.cancel());
    }
}

/// Escape nimmt zurück, was zurückzunehmen ist, sonst öffnet es die Einstellungen. Ein offener Dialog geht vor: Den
/// schließt Escape selbst.
fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    modal: Query<(), With<Modal>>,
    mut desk: Desk,
    mut open: MessageWriter<OpenDialog>,
) {
    if !keys.just_pressed(KeyCode::Escape) || !modal.is_empty() {
        return;
    }
    let mut escaped = Escape::OpenSettings;
    desk.act(|planner| {
        escaped = planner.escape();
        (escaped == Escape::TookBack).then_some(UiSound::Cancel)
    });
    if escaped == Escape::OpenSettings {
        open.write(OpenDialog(Dialog::Settings));
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

/// Der Aktionsknopf: Zug beenden, Wiedergabe überspringen, Computergegner holen oder seinen Platz freimachen.
fn press_action(
    mut pressed: MessageReader<ActionPressed>,
    game: Res<Match>,
    connection: Res<ConnectionStatus>,
    mode: Res<State<BoardMode>>,
    mut outgoing: MessageWriter<ToServer>,
    mut skip: MessageWriter<SkipPlayback>,
    mut sounds: MessageWriter<PlayUiSound>,
    mut say: MessageWriter<Say>,
) {
    for &ActionPressed(action) in pressed.read() {
        match action {
            HudAction::EndTurn => {
                let ready = can_plan(&game, *connection, *mode.get())
                    && game.opponent.is_some()
                    && !game.flagship_missing();
                if ready {
                    outgoing.write(ToServer(ClientMessage::Submit {
                        plan: game.to_turn_plan(),
                    }));
                    sounds.write(PlayUiSound(UiSound::Confirm));
                    say.write(Say::new(RadioCue::OrdersSent));
                }
            }
            HudAction::Skip => {
                skip.write(SkipPlayback);
            }
            HudAction::PlayComputer => {
                outgoing.write(ToServer(ClientMessage::AddBot));
            }
            HudAction::InvitePlayer => {
                outgoing.write(ToServer(ClientMessage::RemoveBot));
            }
        }
    }
}

/// Admiralswahl, Nachschub und Aufgabe gehen an den Server.
fn forward_choices(
    mut admirals: MessageReader<AdmiralChosen>,
    mut drafts: MessageReader<DraftChosen>,
    mut surrenders: MessageReader<SurrenderConfirmed>,
    mut outgoing: MessageWriter<ToServer>,
) {
    let admirals = admirals
        .read()
        .map(|&AdmiralChosen(admiral)| ClientMessage::ChooseAdmiral { admiral });
    let drafts = drafts
        .read()
        .map(|&DraftChosen(pick)| ClientMessage::Draft { pick });
    let surrenders = surrenders.read().map(|_| ClientMessage::Surrender);
    outgoing.write_batch(admirals.chain(drafts).chain(surrenders).map(ToServer));
}

/// „Neues Match“ wechselt nur die Verbindung und den Hintergrund; der erste Zustand des neuen Matches überschreibt
/// Brett, Hände, Admirale und Ergebnis, die Admiralswahl erscheint wieder.
fn start_new_match(
    mut new_match: MessageReader<NewMatch>,
    mut server: ResMut<Server>,
    mut connection: ResMut<ConnectionStatus>,
    mut reported: ResMut<Reported>,
    mut selection: ResMut<Selection>,
    mut background: MessageWriter<ChangeBackground>,
) {
    new_match.clear();
    server.start_new_match();
    *connection = ConnectionStatus::Connecting;
    *reported = Reported::default();
    selection.set_if_neq(Selection::Idle);
    background.write(ChangeBackground);
}

// --- Darstellung ---

/// Ob sich etwas geändert hat, wovon Hervorhebung und HUD abhängen.
/// Bedingungen werden auch außerhalb des Matches ausgewertet; dort fehlt [`BoardMode`].
fn planning_changed(
    game: Res<Match>,
    selection: Res<Selection>,
    connection: Res<ConnectionStatus>,
    mode: Option<Res<State<BoardMode>>>,
) -> bool {
    game.is_changed()
        || selection.is_changed()
        || connection.is_changed()
        || mode.is_some_and(|mode| mode.is_changed())
}

/// Dazu der Zeiger, die Hinweise und die Sprache.
fn focus_changed(
    game: Res<Match>,
    selection: Res<Selection>,
    connection: Res<ConnectionStatus>,
    mode: Option<Res<State<BoardMode>>>,
    hover: Res<BoardHover>,
    notices: Res<Notices>,
    locale: Res<Locale>,
) -> bool {
    planning_changed(game, selection, connection, mode)
        || hover.is_changed()
        || notices.is_changed()
        || locale.is_changed()
}

fn update_highlights(
    game: Res<Match>,
    selection: Res<Selection>,
    mode: Res<State<BoardMode>>,
    mut highlights: ResMut<Highlights>,
) {
    let next = match mode.get() {
        BoardMode::Planning => planner::highlights(&game, *selection),
        BoardMode::Playback => Highlights::default(),
    };
    highlights.set_if_neq(next);
}

/// Unter dem Zeiger der Zug nach dem Klick, dazu der gewählte Schütze.
fn update_focus(
    game: Res<Match>,
    selection: Res<Selection>,
    hover: Res<BoardHover>,
    mut focus: ResMut<BoardFocus>,
) {
    *focus = BoardFocus {
        hypothetical: planner::hypothetical(&game, *selection, *hover),
        shooter: planner::shooter(&game, *selection).map(|slot| slot.target()),
    };
}

/// Statuszeile, Aktionsknopf, ob geplant wird und was gewählt ist.
fn update_hud(
    game: Res<Match>,
    selection: Res<Selection>,
    notices: Res<Notices>,
    hover: Res<BoardHover>,
    focus: Res<BoardFocus>,
    connection: Res<ConnectionStatus>,
    mode: Res<State<BoardMode>>,
    server: Option<Res<Server>>,
    locale: Res<Locale>,
    mut hud: ResMut<HudState>,
) {
    let connected = *connection == ConnectionStatus::Connected;
    let playing_back = *mode.get() == BoardMode::Playback;
    let can_plan = can_plan(&game, *connection, *mode.get());
    let shown = focus.hypothetical.as_ref().unwrap_or(&game);
    let preview = can_plan.then(|| preview_round(shown, shown.plan()));
    let view = StatusView {
        game: &game,
        selection: *selection,
        notices: &notices.0,
        hover: *hover,
        preview: preview.as_ref(),
        locale: &**locale,
    };
    let match_id = server
        .as_ref()
        .map_or("", |server| server.endpoint.match_id.as_str());
    // Neue Matches gehen gegen den Computergegner; bis zur Admiralswahl macht der Knopf seinen Platz für einen
    // Mitspieler frei, und solange keiner da ist, holt er ihn zurück.
    let open = connected && game.round > 0 && game.result.is_none();
    let action = if playing_back {
        HudAction::Skip
    } else if connected && game.round > 0 && game.opponent.is_none() {
        HudAction::PlayComputer
    } else if open
        && game.admiral.is_none()
        && game.opponent.as_ref().is_some_and(|opponent| opponent.bot)
    {
        HudAction::InvitePlayer
    } else {
        HudAction::EndTurn
    };
    let has_opponent = game.opponent.is_some();
    hud.set_if_neq(HudState {
        status: status_line(*connection, playing_back, match_id, &view),
        action,
        action_enabled: action != HudAction::EndTurn
            || can_plan && has_opponent && !game.flagship_missing(),
        planning: can_plan && has_opponent,
        selected: selection.selected(),
    });
}

/// Nach Matchende geht die Hand.
fn hide_hand(game: Res<Match>, mut hand: Query<&mut Visibility, With<HandBar>>) {
    let visibility = if game.result.is_some() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut shown in &mut hand {
        shown.set_if_neq(visibility);
    }
}
