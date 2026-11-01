//! Rauchtest für Läufe ohne Spieler: Mit `VOIDFLEET_SMOKE_FRAMES=<n>` stellt das Match ein paar Schiffe, Geister und
//! ein auseinandergerücktes, glühendes Wrack auf das Brett, wählt beim ersten Zustand vom Server Admiral Osei (der
//! Log zeigt beide Zustände), schreibt `n` Bilder nach dem Start einen Screenshot nach
//! `VOIDFLEET_SMOKE_SCREENSHOT` (kleine `n` zeigen den Ladebildschirm) und beendet die App, sobald er auf der Platte
//! liegt; ohne Screenshot-Pfad endet sie nach `n` Bildern. `VOIDFLEET_SMOKE_TOGGLE=1` fährt zu Beginn des Matches zur
//! anderen Kameraansicht. Mit `VOIDFLEET_SMOKE_SHOTS=<bild>,<bild>,…` entsteht statt des einen Screenshots je
//! genanntem Bild einer (`<stamm>-<bild>.png` neben `VOIDFLEET_SMOKE_SCREENSHOT`), die App endet nach dem letzten.
//! Gezählt wird mit [`FrameCount`].
//!
//! `VOIDFLEET_SMOKE_PLAY=<runden>` spielt stattdessen eine Partie gegen den Computer über den Controller ([`autoplay`]),
//! mit Screenshots an jeder Station (`<stamm>-<station>.png` neben `VOIDFLEET_SMOKE_SCREENSHOT`).
//!
//! `VOIDFLEET_DEV_SCENE` startet statt des Matches eine Vorschau-Szene ([`scenes`], die Rundenwiedergabe mit
//! `commands`: [`commands`]), auch ohne Rauchtest. Ohne beide Variablen tut das Plugin nichts.
//!
//! ```sh
//! VOIDFLEET_SMOKE_FRAMES=240 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/voidfleet.png cargo run -p voidfleet-client
//! VOIDFLEET_SMOKE_FRAMES=6000 VOIDFLEET_SMOKE_PLAY=3 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/play.png cargo run -p voidfleet-client
//! VOIDFLEET_DEV_SCENE=hit cargo run -p voidfleet-client
//! ```

mod autoplay;
mod commands;
mod scenes;

use std::path::PathBuf;

use bevy::{
    diagnostic::FrameCount,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
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
    game::StateChanged,
    net::ToServer,
    scene::{
        camera::{BoardLayout, CameraCommand},
        coords::facing,
        ship::{BOARD_SCALE, BreakApart, ToonState, WreckReady, spawn_ship},
    },
};

#[derive(Resource)]
struct Smoke {
    /// Ohne Screenshots endet der Rauchtest nach so vielen Bildern.
    frames: u32,
    /// Die Screenshots mit ihrem Bild; nach dem letzten endet der Rauchtest.
    shots: Vec<(u32, PathBuf)>,
    toggle: bool,
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        let staged = if commands::requested() {
            commands::plugin(app);
            true
        } else if let Some(scene) = scenes::DevScene::from_env() {
            scenes::plugin(app, scene);
            true
        } else {
            false
        };
        let Some(frames) = std::env::var("VOIDFLEET_SMOKE_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok())
        else {
            return;
        };
        let screenshot = std::env::var_os("VOIDFLEET_SMOKE_SCREENSHOT").map(PathBuf::from);
        let play = std::env::var("VOIDFLEET_SMOKE_PLAY")
            .ok()
            .and_then(|rounds| rounds.parse().ok());
        if let Some(rounds) = play {
            app.insert_resource(autoplay::AutoPlay::new(rounds, screenshot))
                .insert_resource(Smoke {
                    frames,
                    shots: Vec::new(),
                    toggle: false,
                })
                .add_systems(
                    Update,
                    (
                        run_smoke,
                        autoplay::autoplay.run_if(in_state(AppState::Match)),
                    ),
                );
            return;
        }
        let series: Vec<u32> = std::env::var("VOIDFLEET_SMOKE_SHOTS")
            .map(|shots| {
                shots
                    .split(',')
                    .filter_map(|shot| shot.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        let shots = match screenshot {
            None => Vec::new(),
            Some(path) if series.is_empty() => vec![(frames, path)],
            Some(path) => {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                series
                    .into_iter()
                    .map(|frame| (frame, path.with_file_name(format!("{stem}-{frame}.png"))))
                    .collect()
            }
        };
        app.insert_resource(Smoke {
            frames,
            shots,
            toggle: std::env::var("VOIDFLEET_SMOKE_TOGGLE").is_ok_and(|value| value == "1"),
        })
        .add_systems(Update, run_smoke);
        if !staged {
            app.add_systems(OnEnter(AppState::Match), stage_demo)
                .add_observer(spread_wreck)
                .add_systems(
                    Update,
                    choose_admiral.run_if(on_message::<StateChanged>.and_then(run_once)),
                );
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

/// Wählt beim ersten Zustand vom Server (noch ohne Admiral) Admiral Osei.
fn choose_admiral(mut outgoing: MessageWriter<ToServer>) {
    outgoing.write(ToServer(ClientMessage::ChooseAdmiral {
        admiral: AdmiralId::Osei,
    }));
}

/// Schreibt jeden Screenshot in seinem Bild; nach dem letzten endet die App, ohne Screenshots nach `frames` Bildern.
fn run_smoke(
    frame: Res<FrameCount>,
    smoke: Res<Smoke>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(last) = smoke.shots.iter().map(|&(at, _)| at).max() else {
        if frame.0 >= smoke.frames {
            exit.write(AppExit::Success);
        }
        return;
    };
    for (at, path) in smoke.shots.iter().filter(|(at, _)| *at == frame.0) {
        let mut shot = commands.spawn(Screenshot::primary_window());
        shot.observe(save_to_disk(path.clone()));
        if *at == last {
            shot.observe(exit_when_captured);
        }
    }
}

/// Beendet die App nach dem Bild, in dem der Screenshot ankommt; `save_to_disk` schreibt ihn im selben Ereignis.
fn exit_when_captured(_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
