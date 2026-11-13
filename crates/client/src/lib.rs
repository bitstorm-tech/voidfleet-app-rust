//! Voidfleet-Client in Bevy: das PvE-Gefecht aus `voidfleet_core` mit 3D-Szene, Oberfläche und Ton.
//!
//! Aufbau: je Bereich ein Plugin, zusammengefasst in [`ClientPlugin`].
//!
//! - [`settings`] — Spieler-Einstellungen als Ressource, gespeichert als `settings.json`, Fenstermodus.
//! - [`assets`] — Ladeliste aller Assets, Fortschritt und die Zustände [`AppState`] und [`LoadingPhase`].
//! - [`data`] — Kataloge der Assets (Schiffsmodelle, Kartenbilder, Porträts, Hintergründe, Sounds, Bordfunk).
//! - [`palette`] — alle Farben mit Bedeutung.
//! - [`scene`] — die 3D-Szene: Kamera mit ihren zwei Ansichten, Brett, Schiffe, Statusanzeigen, Picking, Abgleich
//!   mit dem Gefecht, Effekte und Animationen der Wiedergabe.
//! - [`audio`] — Mixer, Effekte, Musik, synthetisierte UI-Klänge und Bordfunk über Kira.
//! - [`ui`] — Oberfläche in `bevy_ui`: Karten, Hand, Stationen, HUD, Rundenlog, Dialoge, Ladebildschirm.
//! - [`i18n`] — Texte in Englisch und Deutsch ([`i18n::Locale`]).
//! - [`game`] — das Gefecht als Ressource; führt die Aktionen des Spielers am Regelwerk aus.
//! - [`run`] — der Run als Ressource: Bildschirm, Wahlen, Gefechte der Knoten, Speicherstand `run.json`.
//! - [`controller`] — verbindet Gefecht, Szene und Oberfläche: Auswahl, Klicks, Hervorhebung, Vorschau, Musik.
//! - [`playback`] — Wiedergabe der Ereignisse einer Aktion (vorerst nur Log und Bordfunk).
//! - [`editor`] — der Karteneditor ohne Oberfläche: Sternenkarten bearbeiten, prüfen, schreiben, probespielen.
//! - [`dev`] — Rauchtest für Läufe ohne Spieler (`VOIDFLEET_SMOKE_*`) und Vorschau-Szenen (`VOIDFLEET_DEV_SCENE`).

// Bevy-Systeme nehmen ihre Ressourcen und Abfragen als Parameter; viele davon und verschachtelte Abfragetypen sind dort
// üblich (so empfiehlt es auch Bevy).
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod assets;
pub mod audio;
pub mod card_editor;
pub mod controller;
pub mod data;
pub mod dev;
pub mod editor;
pub mod game;
pub mod i18n;
pub mod palette;
pub mod playback;
pub mod run;
pub mod scene;
pub mod settings;
pub mod ui;

use bevy::{
    asset::AssetPlugin,
    gltf::{GltfPlugin, convert_coordinates::GltfConvertCoordinates},
    log::{DEFAULT_FILTER, LogPlugin},
    pbr::PbrPlugin,
    prelude::*,
    window::WindowPlugin,
};

use std::path::PathBuf;

pub use assets::{AppState, LoadingPhase};

/// Die Assets im Repository: beim Entwickeln (`cargo run`, ein Binary aus `target/`) und in den Tests.
pub const REPO_ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// Wo die Assets liegen: `$BEVY_ASSET_ROOT/assets`, sonst `assets/` neben dem Binary oder in `Contents/Resources`
/// eines macOS-Bundles, sonst die des Repositorys ([`REPO_ASSETS`]).
pub(crate) fn asset_root() -> PathBuf {
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
                        // Eigene App-ID, damit eine Fensterregel z. B. Agent-Starts auf einen Monitor legen kann.
                        name: Some(
                            std::env::var("VOIDFLEET_APP_ID")
                                .unwrap_or_else(|_| "voidfleet".into()),
                        ),
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
                })
                // Die Modelle bekommen ihre Materialien von `scene::toon` statt als `StandardMaterial`.
                .set(PbrPlugin {
                    gltf_enable_standard_materials: false,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(palette::BACKGROUND))
        .add_plugins((
            settings::SettingsPlugin,
            i18n::I18nPlugin,
            game::GamePlugin,
            run::RunPlugin,
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
