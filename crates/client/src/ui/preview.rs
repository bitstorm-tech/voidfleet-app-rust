//! Dev-Ansicht der Oberfläche ohne Server: Mit `VOIDFLEET_UI_PREVIEW=<runde>` hält der Client die Nachrichten des
//! Servers an und füllt das Match mit einer Beispiel-Hand (Stapel, eingeplantes Schiff, gewählter Befehl), Admiral
//! Kerr samt Pendulum (Zielerfassung), einem Gegner und dem HUD-Stand. Ab Runde 8 zeigt die Kopfzeile den
//! Overdrive, in Runde 1 liegt das Flaggschiff vorn auf der Hand.
//!
//! Mit `VOIDFLEET_UI_PREVIEW_SHOT=<pfad>` spielt sie einen kurzen Ablauf und schreibt Screenshots nach
//! `<pfad>-1.png` (Hand), `-2.png` (Banner, Bordfunk, Log), `-3.png` (Rundenbanner), `-4.png` (Karte unter dem
//! Zeiger) und beendet die App.
//!
//! Mit `VOIDFLEET_UI_PREVIEW_DIALOGS=<pfad>` geht sie stattdessen die Dialoge durch und schreibt `<pfad>-<name>.png`:
//! Admiralswahl, Nachschub, die drei Tabs der Einstellungen, die Kapitel der Regeln mit Illustrationen, die Rückfrage
//! zum Aufgeben, das Ergebnis und sein Abzeichen. Die Einstellungen bleiben dabei unberührt.
//!
//! ```sh
//! VOIDFLEET_UI_PREVIEW=8 VOIDFLEET_UI_PREVIEW_SHOT=$TMPDIR/ui cargo run -p voidfleet-client
//! VOIDFLEET_UI_PREVIEW=3 VOIDFLEET_UI_PREVIEW_DIALOGS=$TMPDIR/dialog cargo run -p voidfleet-client
//! ```

use bevy::{
    camera::NormalizedRenderTarget,
    picking::pointer::{Location, PointerAction, PointerId, PointerInput},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    time::Stopwatch,
    ui_widgets::Activate,
    window::{PrimaryWindow, WindowRef},
};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::{CommandId, ShipId},
    protocol::{DraftOffer, MatchResult, OpponentView, RoundEvent, ShipRef, ShipStats},
};

use super::{
    AppendLog, HudAction, HudState, Selected, ShowBanner, ShowRoundBanner,
    button::Key,
    card::{CardFace, CardView},
    dialogs::{
        Dialog, OpenDialog, result_screen::BoardButton, rules::ChapterLink,
        settings_dialog::ActiveTab,
    },
    hud::LogButton,
    round_log::describe_event,
    theme::Modal,
};
use crate::{
    assets::AppState,
    audio::{RadioCue, RadioTransmission, Transmission},
    game::{Match, ServerInbox, ShipPick},
    i18n::{Locale, RulesChapter, SettingsTab},
};

#[derive(Resource)]
struct Preview {
    round: u32,
    shot: Option<String>,
    /// Pfad der Screenshots der Dialoge.
    dialogs: Option<String>,
    /// Zeit seit Beginn des Matches.
    clock: Stopwatch,
    /// Der nächste Schritt des Ablaufs.
    step: usize,
}

/// Füllt das Match mit dem Beispiel.
fn sample_match(round: u32) -> Match {
    let mut game = Match::default();
    game.round = round;
    game.admiral = Some(AdmiralId::Kerr);
    game.ship_hand = vec![
        ShipId::VoidInterceptor,
        ShipId::PlasmaFrigate,
        ShipId::VoidInterceptor,
        ShipId::TitanCarrier,
        ShipId::RepairDrone,
    ];
    game.command_hand = vec![
        CommandId::Torpedo,
        CommandId::EmpBlast,
        CommandId::Torpedo,
        CommandId::Torpedo,
        CommandId::ShieldGrid,
        CommandId::MicroJump,
    ];
    game.ship_deck_size = 9;
    game.command_deck_size = 17;
    game.opponent = Some(OpponentView {
        admiral: Some(AdmiralId::Vance),
        submitted: true,
        ship_hand_size: 3,
        command_hand_size: 4,
        ship_deck_size: 10,
        command_deck_size: 18,
        bot: true,
        played_commands: Vec::new(),
    });
    if round > 1 {
        let board = &mut game.board;
        board.put(
            SlotRef::new(Side::Player, LaneType::Command, 2),
            ShipId::Pendulum,
        );
        board.put(
            SlotRef::new(Side::Player, LaneType::Strike, 1),
            ShipId::HorizonCruiser,
        );
        board.put(
            SlotRef::new(Side::Enemy, LaneType::Strike, 1),
            ShipId::VoidInterceptor,
        );
        board.put(
            SlotRef::new(Side::Enemy, LaneType::Command, 2),
            ShipId::Vanguard,
        );
    }
    game.plan_ship(ShipPick::Hand(1), SlotTarget::new(LaneType::Strike, 3));
    game
}

fn setup(mut inbox: ResMut<ServerInbox>) {
    inbox.hold();
}

/// Setzt Spielzustand und HUD, sobald das Match beginnt.
fn fill(
    mut game: ResMut<Match>,
    mut hud: ResMut<HudState>,
    preview: Res<Preview>,
    locale: Res<Locale>,
) {
    *game = sample_match(preview.round);
    *hud = HudState {
        status: locale.status_idle().into(),
        action: HudAction::EndTurn,
        action_enabled: true,
        planning: true,
        selected: Some(Selected::Command(0)),
    };
}

/// Ein Schuss und ein Abschuss als Beispiel fürs Log.
fn sample_events() -> Vec<RoundEvent> {
    let interceptor = ShipRef::new(
        SlotRef::new(Side::Player, LaneType::Strike, 1),
        ShipId::VoidInterceptor,
    );
    let target = ShipRef::new(
        SlotRef::new(Side::Enemy, LaneType::Strike, 1),
        ShipId::Drone,
    );
    vec![
        RoundEvent::Attack {
            attacker: interceptor,
            target,
            amount: 3,
            after: ShipStats::default(),
            focus: true,
            swarm: false,
            head_start: false,
            through: false,
            taunt: false,
        },
        RoundEvent::Destroyed { ship: target },
    ]
}

/// Der Ablauf mit Screenshots: Hand, dann Banner samt Bordfunk und Log, dann das Rundenbanner, dann eine Karte unter
/// dem Zeiger.
fn script(
    mut commands: Commands,
    time: Res<Time>,
    mut preview: ResMut<Preview>,
    locale: Res<Locale>,
    mut banner: MessageWriter<ShowBanner>,
    mut round_banner: MessageWriter<ShowRoundBanner>,
    mut log: MessageWriter<AppendLog>,
    mut radio: MessageWriter<RadioTransmission>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(shot) = preview.shot.clone() else {
        return;
    };
    preview.clock.tick(time.delta());
    let screenshot = |commands: &mut Commands, index: usize| {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{shot}-{index}.png")));
    };
    let steps: [f32; 8] = [3.0, 3.2, 3.6, 4.4, 5.0, 5.4, 6.0, 7.0];
    let Some(&at) = steps.get(preview.step) else {
        return;
    };
    if preview.clock.elapsed_secs() < at {
        return;
    }
    match preview.step {
        0 => screenshot(&mut commands, 1),
        1 => {
            banner.write(ShowBanner {
                title: locale.banner_command(
                    locale.log_who(Side::Enemy),
                    locale.command_name(CommandId::Torpedo),
                ),
                subtitle: Some(locale.banner_miss("Strike 2")),
                side: Side::Enemy,
                admiral: Some(AdmiralId::Vance),
            });
            radio.write(RadioTransmission(Some(Transmission {
                cue: RadioCue::TorpedoIncoming,
                variant: 0,
                duration: 3.0,
            })));
            let mut lines = vec![locale.log_round_header(preview.round)];
            lines.extend(
                sample_events()
                    .iter()
                    .map(|event| describe_event(&**locale, event)),
            );
            log.write(AppendLog(lines));
            commands.run_system_cached(open_log);
        }
        2 => screenshot(&mut commands, 2),
        3 => {
            round_banner.write(ShowRoundBanner(preview.round));
        }
        4 => screenshot(&mut commands, 3),
        5 => commands.run_system_cached(point_at_card),
        6 => screenshot(&mut commands, 4),
        _ => {
            exit.write(AppExit::Success);
        }
    }
    preview.step += 1;
}

/// Fährt den Zeiger über die zweite Schiffskarte, als käme die Bewegung von der Maus.
fn point_at_card(
    cards: Query<(&CardView, &UiGlobalTransform, &ComputedNode)>,
    window: Single<Entity, With<PrimaryWindow>>,
    mut input: MessageWriter<PointerInput>,
) {
    let Some((_, transform, node)) = cards
        .iter()
        .filter(|(view, ..)| matches!(view.card, CardFace::Ship(_)))
        .nth(1)
    else {
        return;
    };
    let Some(target) = WindowRef::Primary
        .normalize(Some(*window))
        .map(NormalizedRenderTarget::Window)
    else {
        return;
    };
    let position = transform.translation * node.inverse_scale_factor();
    input.write(PointerInput::new(
        PointerId::Mouse,
        Location { target, position },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
}

/// Klappt das Log auf, als hätte jemand den Knopf gedrückt.
fn open_log(mut commands: Commands, button: Single<Entity, With<LogButton>>) {
    commands.trigger(Activate { entity: *button });
}

/// Ein Schritt durch die Dialoge.
#[derive(Clone, Copy, Debug)]
enum DialogStep {
    /// Der eigene Admiral fehlt noch: die Admiralswahl.
    Admiral,
    /// Ein Nachschub mit Schrottverwertung.
    Draft,
    Open(Dialog),
    Tab(SettingsTab),
    Chapter(RulesChapter),
    /// Gewonnen gegen Vance, offene Dialoge zu.
    Result,
    /// „Brett ansehen“: nur noch das Abzeichen.
    Badge,
    Shot(&'static str),
}

const DIALOG_STEPS: [DialogStep; 32] = [
    DialogStep::Admiral,
    DialogStep::Shot("admiral"),
    DialogStep::Draft,
    DialogStep::Shot("draft"),
    DialogStep::Open(Dialog::Settings),
    DialogStep::Shot("settings-game"),
    DialogStep::Tab(SettingsTab::Video),
    DialogStep::Shot("settings-video"),
    DialogStep::Tab(SettingsTab::Sound),
    DialogStep::Shot("settings-sound"),
    DialogStep::Open(Dialog::Rules),
    DialogStep::Shot("rules-goal"),
    DialogStep::Chapter(RulesChapter::Turn),
    DialogStep::Shot("rules-turn"),
    DialogStep::Chapter(RulesChapter::Roster),
    DialogStep::Shot("rules-roster"),
    DialogStep::Chapter(RulesChapter::Board),
    DialogStep::Shot("rules-board"),
    DialogStep::Chapter(RulesChapter::Resolution),
    DialogStep::Shot("rules-resolution"),
    DialogStep::Chapter(RulesChapter::Combat),
    DialogStep::Shot("rules-combat"),
    DialogStep::Chapter(RulesChapter::Ships),
    DialogStep::Shot("rules-ships"),
    DialogStep::Chapter(RulesChapter::Commands),
    DialogStep::Shot("rules-commands"),
    DialogStep::Open(Dialog::Surrender),
    DialogStep::Shot("surrender"),
    DialogStep::Result,
    DialogStep::Shot("result"),
    DialogStep::Badge,
    DialogStep::Shot("badge"),
];

/// Geht die Dialoge durch, je Schritt eine Sekunde, und beendet danach die App.
fn dialog_script(
    mut commands: Commands,
    time: Res<Time>,
    mut preview: ResMut<Preview>,
    mut game: ResMut<Match>,
    mut open: MessageWriter<OpenDialog>,
    mut tab: ResMut<ActiveTab>,
    links: Query<(Entity, &ChapterLink)>,
    modals: Query<Entity, With<Modal>>,
    board_button: Query<Entity, (With<BoardButton>, With<Key>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = preview.dialogs.clone() else {
        return;
    };
    preview.clock.tick(time.delta());
    let at = 3.0 + preview.step as f32;
    if preview.clock.elapsed_secs() < at {
        return;
    }
    let Some(&step) = DIALOG_STEPS.get(preview.step) else {
        exit.write(AppExit::Success);
        return;
    };
    preview.step += 1;
    match step {
        DialogStep::Admiral => {
            game.admiral = None;
            game.round = 1;
            if let Some(opponent) = &mut game.opponent {
                opponent.admiral = None;
            }
        }
        DialogStep::Draft => {
            game.admiral = Some(AdmiralId::Kerr);
            game.round = 3;
            game.draft = Some(DraftOffer {
                ships: vec![
                    ShipId::PlasmaFrigate,
                    ShipId::ScrapRaider,
                    ShipId::HorizonCruiser,
                ],
                commands: vec![
                    CommandId::ScrapSalvage,
                    CommandId::Torpedo,
                    CommandId::ShieldGrid,
                ],
            });
        }
        DialogStep::Open(dialog) => {
            game.draft = None;
            open.write(OpenDialog(dialog));
        }
        DialogStep::Tab(next) => tab.0 = next,
        DialogStep::Chapter(chapter) => {
            if let Some((link, _)) = links.iter().find(|(_, link)| link.0 == chapter) {
                commands.trigger(Activate { entity: link });
            }
        }
        DialogStep::Result => {
            for modal in &modals {
                commands.entity(modal).despawn();
            }
            if let Some(opponent) = &mut game.opponent {
                opponent.admiral = Some(AdmiralId::Vance);
            }
            game.round = 9;
            game.result = Some(MatchResult::Won);
        }
        DialogStep::Badge => {
            for button in &board_button {
                commands.trigger(Activate { entity: button });
            }
        }
        DialogStep::Shot(name) => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{path}-{name}.png")));
        }
    }
}

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        let Some(round) = std::env::var("VOIDFLEET_UI_PREVIEW")
            .ok()
            .and_then(|value| value.parse().ok())
        else {
            return;
        };
        app.insert_resource(Preview {
            round,
            shot: std::env::var("VOIDFLEET_UI_PREVIEW_SHOT").ok(),
            dialogs: std::env::var("VOIDFLEET_UI_PREVIEW_DIALOGS").ok(),
            clock: Stopwatch::new(),
            step: 0,
        })
        .insert_resource(crate::controller::Scripted)
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Match), fill)
        .add_systems(
            Update,
            (script, dialog_script).run_if(in_state(AppState::Match)),
        );
    }
}
