//! Rauchtest für Läufe ohne Spieler: Mit `VOIDFLEET_SMOKE_FRAMES=<n>` stellt das Match ein paar Schiffe, Geister und
//! ein auseinandergerücktes, glühendes Wrack auf das Brett, wählt beim ersten Zustand vom Server Admiral Osei (der
//! Log zeigt beide Zustände), schreibt `n` Bilder nach dem Start einen Screenshot nach
//! `VOIDFLEET_SMOKE_SCREENSHOT` (sofern gesetzt; kleine `n` zeigen den Ladebildschirm) und beendet die App.
//! `VOIDFLEET_SMOKE_TOGGLE=1` fährt zu Beginn des Matches zur anderen Kameraansicht. Ohne die Variable tut das Plugin
//! nichts.
//!
//! ```sh
//! VOIDFLEET_SMOKE_FRAMES=240 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/voidfleet.png cargo run -p voidfleet-client
//! ```

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side},
    cards::ShipId,
    protocol::ClientMessage,
};

use crate::{
    assets::AppState,
    data::ship_models::ShipModels,
    game::{Match, StateChanged},
    net::ToServer,
    scene::{
        camera::{BoardLayout, CameraCommand},
        coords::facing,
        ship::{BOARD_SCALE, BreakApart, ToonState, WreckReady, spawn_ship},
    },
};

/// Wie viele Bilder nach dem Screenshot noch laufen, bis er auf der Platte liegt.
const SCREENSHOT_MARGIN: u32 = 20;

#[derive(Resource)]
struct Smoke {
    frames: u32,
    screenshot: Option<PathBuf>,
    toggle: bool,
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        let Some(frames) = std::env::var("VOIDFLEET_SMOKE_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok())
        else {
            return;
        };
        app.insert_resource(Smoke {
            frames,
            screenshot: std::env::var_os("VOIDFLEET_SMOKE_SCREENSHOT").map(PathBuf::from),
            toggle: std::env::var("VOIDFLEET_SMOKE_TOGGLE").is_ok_and(|value| value == "1"),
        })
        .add_systems(OnEnter(AppState::Match), stage_demo)
        .add_observer(spread_wreck)
        .add_systems(
            Update,
            (run_smoke, choose_admiral.run_if(on_message::<StateChanged>)),
        );
    }
}

/// Je Seite ein paar Schiffe, eines davon als Geist, und ein Wrack mit glühenden Bruchkanten.
fn stage_demo(
    mut commands: Commands,
    models: Res<ShipModels>,
    layout: Res<BoardLayout>,
    smoke: Res<Smoke>,
    mut camera: MessageWriter<CameraCommand>,
) {
    if smoke.toggle {
        camera.write(CameraCommand::ToggleView);
    }
    let fleet = [
        (
            Side::Player,
            LaneType::Strike,
            1,
            ShipId::IronDreadnought,
            false,
        ),
        (
            Side::Player,
            LaneType::Strike,
            3,
            ShipId::PlasmaFrigate,
            false,
        ),
        (Side::Player, LaneType::Command, 2, ShipId::Bastion, false),
        (
            Side::Player,
            LaneType::Command,
            4,
            ShipId::RepairDrone,
            true,
        ),
        (
            Side::Enemy,
            LaneType::Strike,
            1,
            ShipId::VoidInterceptor,
            false,
        ),
        (Side::Enemy, LaneType::Strike, 2, ShipId::Decoy, false),
        (
            Side::Enemy,
            LaneType::Strike,
            4,
            ShipId::TitanCarrier,
            false,
        ),
        (Side::Enemy, LaneType::Command, 3, ShipId::Mirage, false),
        (
            Side::Enemy,
            LaneType::Command,
            1,
            ShipId::ScrapRaider,
            false,
        ),
    ];
    for (side, lane, slot, ship, ghost) in fleet {
        let entity = spawn_ship(&mut commands, &models, ship, ghost);
        commands.entity(entity).insert((
            Transform::from_translation(layout.slot_position(side, lane, slot) + Vec3::Y * 0.5)
                .looking_to(Vec3::X * facing(side), Vec3::Y)
                .with_scale(Vec3::splat(BOARD_SCALE)),
            ToonState::facing(side),
        ));
    }
    let wreck = spawn_ship(&mut commands, &models, ShipId::HorizonCruiser, false);
    commands.entity(wreck).insert((
        Transform::from_translation(
            layout.slot_position(Side::Enemy, LaneType::Command, 5) + Vec3::Y * 0.5,
        )
        .looking_to(Vec3::NEG_X, Vec3::Y)
        .with_scale(Vec3::splat(BOARD_SCALE)),
        ToonState {
            heat: Some(LinearRgba::rgb(2.0, 0.9, 0.3)),
            ..ToonState::facing(Side::Enemy)
        },
    ));
    commands.trigger(BreakApart { entity: wreck });
}

/// Rückt die Bruchstücke des Wracks auseinander, damit die glühenden Bruchflächen zu sehen sind.
fn spread_wreck(ready: On<WreckReady>, mut pieces: Query<&mut Transform>) {
    for piece in &ready.pieces {
        if let Ok(mut transform) = pieces.get_mut(*piece) {
            transform.translation *= 1.8;
        }
    }
}

/// Wählt einen Admiral, solange noch keiner gewählt ist.
fn choose_admiral(game: Res<Match>, mut outgoing: MessageWriter<ToServer>) {
    if game.admiral.is_none() {
        outgoing.write(ToServer(ClientMessage::ChooseAdmiral {
            admiral: AdmiralId::Osei,
        }));
    }
}

fn run_smoke(
    mut frame: Local<u32>,
    smoke: Res<Smoke>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == smoke.frames
        && let Some(path) = &smoke.screenshot
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    }
    if *frame >= smoke.frames + SCREENSHOT_MARGIN {
        exit.write(AppExit::Success);
    }
}
