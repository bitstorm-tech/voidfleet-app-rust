//! Rauchtest für Läufe ohne Spieler: Mit `VOIDFLEET_SMOKE_FRAMES=<n>` stellt das Match ein paar Schiffe, Geister und
//! ein auseinandergerücktes, glühendes Wrack auf das Brett, wählt beim ersten Zustand vom Server Admiral Osei (der
//! Log zeigt beide Zustände), schreibt `n` Bilder nach dem Start einen Screenshot nach
//! `VOIDFLEET_SMOKE_SCREENSHOT` (sofern gesetzt; kleine `n` zeigen den Ladebildschirm) und beendet die App.
//! `VOIDFLEET_SMOKE_TOGGLE=1` fährt zu Beginn des Matches zur anderen Kameraansicht. Mit
//! `VOIDFLEET_SMOKE_SHOTS=<bild>,<bild>,…` entsteht statt des einen Screenshots je genanntem Bild einer
//! (`<stamm>-<bild>.png` neben `VOIDFLEET_SMOKE_SCREENSHOT`).
//!
//! `VOIDFLEET_DEV_SCENE` startet statt des Matches eine Vorschau-Szene ([`scenes`]), auch ohne Rauchtest. Ohne beide
//! Variablen tut das Plugin nichts.
//!
//! ```sh
//! VOIDFLEET_SMOKE_FRAMES=240 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/voidfleet.png cargo run -p voidfleet-client
//! VOIDFLEET_DEV_SCENE=hit cargo run -p voidfleet-client
//! ```

mod scenes;

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
    /// Bilder für eine Screenshot-Serie; leer: ein Screenshot nach `frames` Bildern.
    shots: Vec<u32>,
    toggle: bool,
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        let scene = scenes::DevScene::from_env();
        if let Some(scene) = scene {
            scenes::plugin(app, scene);
        }
        let Some(frames) = std::env::var("VOIDFLEET_SMOKE_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok())
        else {
            return;
        };
        let shots = std::env::var("VOIDFLEET_SMOKE_SHOTS")
            .map(|shots| {
                shots
                    .split(',')
                    .filter_map(|shot| shot.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        app.insert_resource(Smoke {
            frames,
            screenshot: std::env::var_os("VOIDFLEET_SMOKE_SCREENSHOT").map(PathBuf::from),
            shots,
            toggle: std::env::var("VOIDFLEET_SMOKE_TOGGLE").is_ok_and(|value| value == "1"),
        })
        .add_systems(Update, run_smoke);
        if scene.is_none() {
            app.add_systems(OnEnter(AppState::Match), stage_demo)
                .add_observer(spread_wreck)
                .add_systems(Update, choose_admiral.run_if(on_message::<StateChanged>));
        } else {
            app.add_systems(OnEnter(AppState::Match), toggle_view);
        }
    }
}

/// Fährt zu Beginn zur anderen Ansicht, wenn der Rauchtest es will.
fn toggle_view(smoke: Res<Smoke>, mut camera: MessageWriter<CameraCommand>) {
    if smoke.toggle {
        camera.write(CameraCommand::ToggleView);
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
    if let Some(path) = &smoke.screenshot {
        let path = if smoke.shots.is_empty() {
            (*frame == smoke.frames).then(|| path.clone())
        } else {
            smoke.shots.contains(&*frame).then(|| {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                path.with_file_name(format!("{stem}-{frame}.png", frame = *frame))
            })
        };
        if let Some(path) = path {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
    }
    if *frame >= smoke.frames + SCREENSHOT_MARGIN {
        exit.write(AppExit::Success);
    }
}
