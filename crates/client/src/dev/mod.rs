//! Rauchtest für Läufe ohne Spieler: Mit `VOIDFLEET_SMOKE_FRAMES=<n>` beginnt die App nach dem Laden einen Run mit
//! Admiral Osei (fester Seed) und fliegt zum ersten Gefecht, schreibt `n` Bilder nach dem Start einen Screenshot nach `VOIDFLEET_SMOKE_SCREENSHOT` (kleine
//! `n` zeigen den Ladebildschirm) und endet, sobald er auf der Platte liegt; ohne Screenshot-Pfad endet sie nach `n`
//! Bildern. `VOIDFLEET_SMOKE_TOGGLE=1` fährt zu Beginn zur anderen Kameraansicht. Mit
//! `VOIDFLEET_SMOKE_SHOTS=<bild>,<bild>,…` entsteht statt des einen Screenshots je genanntem Bild einer
//! (`<stamm>-<bild>.png` neben `VOIDFLEET_SMOKE_SCREENSHOT`), die App endet nach dem letzten. Gezählt wird mit
//! [`FrameCount`].
//!
//! `VOIDFLEET_SMOKE_PLAY=<runden>` spielt stattdessen den Run ab der Sternenkarte über dieselben Messages wie ein
//! Spieler ([`autoplay`]), mit Screenshots an jeder Station (`<stamm>-<nr>-<station>.png` neben
//! `VOIDFLEET_SMOKE_SCREENSHOT`). Rauchtest und Vorschau-Szenen speichern keinen Run ([`RunSave`] ohne Pfad).
//!
//! `VOIDFLEET_DEV_SCENE` startet statt des Gefechts eine Vorschau-Szene ([`scenes`]), auch ohne Rauchtest. Ohne die
//! Variablen tut das Plugin nichts.
//!
//! ```sh
//! VOIDFLEET_SMOKE_FRAMES=300 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/voidfleet.png cargo run -p voidfleet-client
//! VOIDFLEET_SMOKE_FRAMES=200000 VOIDFLEET_SMOKE_PLAY=6 VOIDFLEET_SMOKE_SCREENSHOT=/tmp/play.png cargo run -p voidfleet-client
//! VOIDFLEET_DEV_SCENE=hit cargo run -p voidfleet-client
//! ```

mod autoplay;
mod report;
mod scenes;

use std::path::PathBuf;

use bevy::{
    diagnostic::FrameCount,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
};
use voidfleet_core::admirals::AdmiralId;

use crate::{
    assets::AppState,
    run::{ActiveRun, RunSave, new_run},
    scene::camera::CameraCommand,
};

/// Seed der Runs von Rauchtest und Vorschau-Szenen, damit die Bilder vergleichbar bleiben.
const DEV_SEED: u64 = 7;

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
        let staged = if let Some(scene) = scenes::DevScene::from_env() {
            scenes::plugin(app, scene);
            app.insert_resource(RunSave(None));
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
        app.insert_resource(RunSave(None));
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
                .add_systems(OnEnter(AppState::Match), start_run)
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
        app.add_systems(OnEnter(AppState::Match), toggle_view);
        if !staged {
            app.add_systems(OnEnter(AppState::Match), start_battle);
        }
    }
}

/// Fährt zu Beginn zur anderen Ansicht, wenn der Rauchtest es will.
fn toggle_view(smoke: Res<Smoke>, mut camera: MessageWriter<CameraCommand>) {
    if smoke.toggle {
        camera.write(CameraCommand::ToggleView);
    }
}

/// Ein Run mit festem Seed; mit `to_battle` schon am ersten Knoten (Reihe 1 ist immer ein Gefecht).
fn dev_run(admiral: AdmiralId, to_battle: bool) -> ActiveRun {
    let mut run = new_run(admiral, DEV_SEED);
    if to_battle {
        let column = run.choices()[0];
        run.advance(column).expect("Reihe 1 ist wählbar");
    }
    run
}

/// Beginnt einen Run mit Admiral Osei auf der Sternenkarte.
fn start_run(mut commands: Commands) {
    commands.insert_resource(dev_run(AdmiralId::Osei, false));
}

/// Beginnt einen Run mit Admiral Osei am ersten Gefecht.
fn start_battle(mut commands: Commands) {
    commands.insert_resource(dev_run(AdmiralId::Osei, true));
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
pub(crate) fn exit_when_captured(_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
