//! Voidfleet-Client in Bevy: Port des TypeScript-Clients (Babylon.js, DOM-UI, Web Audio).
//!
//! Aufbau: je Bereich ein Plugin, zusammengefasst in [`ClientPlugin`].
//!
//! - [`settings`] — Spieler-Einstellungen als Ressource, gespeichert als `settings.json`, Fenstermodus.
//! - [`assets`] — Ladeliste aller Assets, Fortschritt und die Zustände [`AppState`] und [`LoadingPhase`].
//! - [`data`] — Kataloge der Assets (Schiffsmodelle, Kartenbilder, Porträts, Hintergründe, Sounds, Bordfunk).
//! - [`palette`] — alle Farben mit Bedeutung.
//! - [`scene`] — Kamera mit ihren zwei Ansichten, Hintergrund, Tweens, Toon-Rendering, Schiffe und Wracks.
//! - [`audio`] — Mixer, Effekte, Musik, synthetisierte UI-Klänge und Bordfunk über Kira.
//! - [`ui`] — Schriften, Fenster-Icon und Ladebildschirm.
//! - [`i18n`] — Texte in Englisch und Deutsch ([`i18n::Locale`]).
//! - [`net`] — Kommandozeile, eingebetteter Server und WebSocket-Verbindung.
//! - [`game`] — Spielzustand, Zugplanung und Zielvorschau; übernimmt die Nachrichten des Servers.
//! - [`controller`] — verbindet Server, Zustand, Szene und Oberfläche: Auswahl, Klicks, Hervorhebung, HUD, Bordfunk.
//! - [`playback`] — Rundenwiedergabe: spielt die aufgedeckten Ereignisse einer Runde auf dem Brett ab.
//! - [`dev`] — Rauchtest für Läufe ohne Spieler (`VOIDFLEET_SMOKE_FRAMES`).

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

pub use assets::{AppState, LoadingPhase};

/// Ordner der Assets, relativ zum Paket `crates/client` (Bevy liest beim Start über Cargo `CARGO_MANIFEST_DIR`).
const ASSET_ROOT: &str = "../../assets";

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
                // Die MP3s tragen ID3-Kopfdaten, über die Symphonia bei jeder Datei warnt.
                .set(LogPlugin {
                    filter: format!("{DEFAULT_FILTER},symphonia_core=error"),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: ASSET_ROOT.into(),
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
