//! Voidfleet-Client in Bevy: Port des TypeScript-Clients (Babylon.js, DOM-UI, Web Audio).
//!
//! Aufbau: je Bereich ein Plugin, zusammengefasst in [`ClientPlugin`].
//!
//! - [`settings`] — Spieler-Einstellungen als Ressource, gespeichert als `settings.json`, Fenstermodus.
//! - [`assets`] — Ladeliste aller Assets, Fortschritt und die Zustände [`AppState`] und [`LoadingPhase`].
//! - [`data`] — Kataloge der Assets (Schiffsmodelle, Kartenbilder, Porträts, Hintergründe, Sounds, Bordfunk).
//! - [`palette`] — alle Farben mit Bedeutung.
//! - [`scene`] — die 3D-Szene: Kamera mit ihren zwei Ansichten, Brett, Schiffe, Planungs-Overlays, Picking, Abgleich
//!   mit dem Spielzustand, Effekte und Animationen der Wiedergabe.
//! - [`audio`] — Mixer, Effekte, Musik, synthetisierte UI-Klänge und Bordfunk über Kira.
//! - [`ui`] — Oberfläche in `bevy_ui`: Karten, Hand, HUD, Gegnerpanel, Rundenlog, Dialoge, Ladebildschirm.
//! - [`i18n`] — Texte in Englisch und Deutsch ([`i18n::Locale`]).
//! - [`net`] — Kommandozeile, eingebetteter Server und WebSocket-Verbindung.
//! - [`game`] — Spielzustand, Zugplanung und Zielvorschau; übernimmt die Nachrichten des Servers.
//! - [`controller`] — verbindet Server, Zustand, Szene und Oberfläche: Auswahl, Klicks, Hervorhebung, HUD, Bordfunk.
//! - [`playback`] — Rundenwiedergabe: spielt die aufgedeckten Ereignisse einer Runde auf dem Brett ab.
//! - [`dev`] — Rauchtest für Läufe ohne Spieler (`VOIDFLEET_SMOKE_*`) und Vorschau-Szenen (`VOIDFLEET_DEV_SCENE`).

// Bevy-Systeme nehmen ihre Ressourcen und Abfragen als Parameter; viele davon und verschachtelte Abfragetypen sind dort
// üblich (so empfiehlt es auch Bevy).
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod assets;
pub mod audio;
pub mod controller;
pub mod data;
pub mod dev;
pub mod game;
pub mod i18n;
pub mod net;
pub mod palette;
pub mod playback;
pub mod scene;
pub mod settings;
pub mod ui;

use bevy::{
    asset::AssetPlugin,
    gltf::{GltfPlugin, convert_coordinates::GltfConvertCoordinates},
    log::{DEFAULT_FILTER, LogPlugin},
    prelude::*,
    window::WindowPlugin,
};

use std::path::PathBuf;

pub use assets::{AppState, LoadingPhase};

/// Die Assets im Repository: beim Entwickeln (`cargo run`, ein Binary aus `target/`) und in den Tests.
pub const REPO_ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// Wo die Assets liegen: `$BEVY_ASSET_ROOT/assets`, sonst `assets/` neben dem Binary oder in `Contents/Resources`
/// eines macOS-Bundles, sonst die des Repositorys ([`REPO_ASSETS`]).
fn asset_root() -> PathBuf {
    if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
        return PathBuf::from(root).join("assets");
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let dir = exe.parent()?;
            [dir.join("assets"), dir.join("../Resources/assets")]
                .into_iter()
                .find(|candidate| candidate.is_dir())
        })
        .unwrap_or_else(|| PathBuf::from(REPO_ASSETS))
}

/// Das ganze Spiel: Bevys Standard-Plugins samt Fenster und glTF-Einstellungen und je Bereich ein eigenes Plugin.
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voidfleet".into(),
                        name: Some("voidfleet".into()),
                        ..default()
                    }),
                    ..default()
                })
                // Die MP3s tragen ID3-Kopfdaten, über die Symphonia bei jeder Datei warnt; beim Streamen
                // der Musik meldet es außerdem harmlos das Bit-Reservoir des ersten Frames.
                .set(LogPlugin {
                    filter: format!(
                        "{DEFAULT_FILTER},symphonia_core=error,symphonia_bundle_mp3=error"
                    ),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root().to_string_lossy().into_owned(),
                    ..default()
                })
                // Die Schiffe zeigen mit der Nase nach glTF +Z; gedreht zeigt sie nach Bevys `Transform::forward` (−Z).
                .set(GltfPlugin {
                    convert_coordinates: GltfConvertCoordinates {
                        rotate_scene_entity: true,
                        rotate_meshes: false,
                    },
                    ..default()
                }),
        )
        .insert_resource(ClearColor(palette::BACKGROUND))
        .add_plugins((
            settings::SettingsPlugin,
            i18n::I18nPlugin,
            net::NetPlugin,
            game::GamePlugin,
            assets::AssetsPlugin,
            scene::ScenePlugin,
            audio::SoundPlugin,
            ui::InterfacePlugin,
            playback::PlaybackPlugin,
            controller::ControllerPlugin,
            dev::DevPlugin,
        ));
    }
}
