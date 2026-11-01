//! Dev-Ansicht der Oberfläche ohne Server: Mit `VOIDFLEET_UI_PREVIEW=<runde>` hält der Client die Nachrichten des
//! Servers an und füllt das Match mit einer Beispiel-Hand (Stapel, eingeplantes Schiff, gewählter Befehl), Admiral
//! Kerr samt Pendulum (Zielerfassung), einem Gegner und dem HUD-Stand. Ab Runde 8 zeigt die Kopfzeile den
//! Overdrive, in Runde 1 liegt das Flaggschiff vorn auf der Hand.
//!
//! Mit `VOIDFLEET_UI_PREVIEW_SHOT=<pfad>` spielt sie einen kurzen Ablauf und schreibt Screenshots nach
//! `<pfad>-1.png` (Hand), `-2.png` (Banner, Bordfunk, Log), `-3.png` (Rundenbanner), `-4.png` (Karte unter dem
//! Zeiger) und beendet die App.
//!
//! ```sh
//! VOIDFLEET_UI_PREVIEW=8 VOIDFLEET_UI_PREVIEW_SHOT=$TMPDIR/ui cargo run -p voidfleet-client
//! ```

use bevy::{
    camera::NormalizedRenderTarget,
    picking::pointer::{Location, PointerAction, PointerId, PointerInput},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    ui_widgets::Activate,
    window::{PrimaryWindow, WindowRef},
};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::{CommandId, ShipId},
    protocol::{OpponentView, RoundEvent, ShipRef, ShipStats},
};

use super::{
    AppendLog, HudAction, HudState, Selected, ShowBanner, ShowRoundBanner,
    card::{CardFace, CardView},
    hud::LogButton,
    round_log::describe_event,
};
use crate::{
    assets::AppState,
    audio::{RadioCue, RadioTransmission, Transmission},
    game::{Match, ServerInbox, ShipPick},
    i18n::Locale,
};

#[derive(Resource)]
struct Preview {
    round: u32,
    shot: Option<String>,
    /// Sekunden seit Beginn des Matches.
    elapsed: f32,
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
#[allow(clippy::too_many_arguments)]
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
    preview.elapsed += time.delta_secs();
    let screenshot = |commands: &mut Commands, index: usize| {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{shot}-{index}.png")));
    };
    let steps: [f32; 8] = [3.0, 3.2, 3.6, 4.4, 5.0, 5.4, 6.0, 7.0];
    let Some(&at) = steps.get(preview.step) else {
        return;
    };
    if preview.elapsed < at {
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
            elapsed: 0.0,
            step: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Match), fill)
        .add_systems(Update, script.run_if(in_state(AppState::Match)));
    }
}
