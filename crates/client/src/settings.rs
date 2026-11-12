//! Spieler-Einstellungen (Port von `settings.ts` samt dem Fenstermodus der Electron-App).
//!
//! [`Settings`] ist eine Ressource, gespeichert als `settings.json` im Konfigurationsordner des Systems (unter macOS
//! `~/Library/Application Support/dev.voidfleet.Voidfleet`; getrennt vom Ordner der Electron-App, deren Kamera-Zoom in
//! Babylon-Koordinaten steht). Wer etwas ändert, schreibt die Ressource; gespeichert wird danach
//! automatisch, und wer darauf reagieren muss, prüft `Res<Settings>::is_changed()` bzw. `resource_changed::<Settings>`.
//! Fehlende Felder in der Datei nehmen ihren Standardwert.

use std::{fs, path::PathBuf};

use bevy::{
    prelude::*,
    render::{render_resource::TextureFormat, renderer::RenderAdapter},
    window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode as BevyWindowMode},
};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Animationsgeschwindigkeit als Streckungsfaktor aller Animationen; „Normal“ spielt die Dauern aus dem Code etwas
/// gedehnt ab, „Schnell“ und „Sehr schnell“ beschleunigen deutlich.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
    VeryFast,
}

impl AnimationSpeed {
    /// Streckungsfaktor der Dauern.
    pub const fn time_scale(self) -> f32 {
        match self {
            Self::Slow => 1.6,
            Self::Normal => 1.25,
            Self::Fast => 0.6,
            Self::VeryFast => 0.35,
        }
    }
}

/// Fenstermodus der Desktop-App, in dem sie beim nächsten Start öffnet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowMode {
    Windowed,
    #[default]
    Maximized,
    Fullscreen,
}

/// Kantenglättung der 3D-Szene (MSAA), der Wert ist die Zahl der Abtastpunkte je Pixel. Mehr glättet dünne Teile
/// besser und kostet Grafikspeicher: Bild und Tiefe der Szene wachsen mit der Zahl.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Antialiasing {
    Off = 1,
    X2 = 2,
    #[default]
    X4 = 4,
    X8 = 8,
}

impl Antialiasing {
    pub const ALL: [Self; 4] = [Self::Off, Self::X2, Self::X4, Self::X8];

    /// Diese Stufe oder die nächstkleinere, die die Grafikkarte für Bild und Tiefe der Szene kann: 2- und 8-fach gibt
    /// es nicht überall, und eine Stufe, die fehlt, beendete das Spiel mit einem Renderfehler.
    pub fn usable(self, adapter: &RenderAdapter) -> Self {
        let supported = |level: Self| {
            [TextureFormat::Rgba16Float, TextureFormat::Depth32Float]
                .into_iter()
                .all(|format| {
                    adapter
                        .get_texture_format_features(format)
                        .flags
                        .sample_count_supported(level as u32)
                })
        };
        Self::ALL
            .into_iter()
            .rev()
            .find(|&level| level <= self && supported(level))
            .unwrap_or(Self::Off)
    }
}

/// Sprache der Oberfläche; ohne Wahl gilt die Systemsprache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    En,
    De,
}

/// Wählbare Mauszeiger (`assets/cursors/cursor_<id>.svg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CursorId {
    #[default]
    Arrow,
    Fighter,
    Delta,
    Flagship,
    Needle,
    Shard,
    Formation,
    Holo,
    Thruster,
    Crosshair,
}

/// Zuletzt eingestellte Kamera; das nächste Match beginnt dort.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    /// Stand zwischen den Ansichten: 0 von der Seite, 1 von hinten oben.
    pub view: f32,
    /// Versatz durch Zoom und Verschieben aus der Ruhelage (Weltkoordinaten in Bevy, siehe [`crate::scene::coords`]).
    pub zoom: [f32; 3],
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            view: 1.0,
            zoom: [0.0; 3],
        }
    }
}

/// Eine Datei im Konfigurationsordner des Systems (Einstellungen, Speicherstand des Runs).
pub fn config_file(name: &str) -> Option<PathBuf> {
    ProjectDirs::from("dev", "voidfleet", "Voidfleet").map(|dirs| dirs.config_dir().join(name))
}

/// Die Einstellungen des Spielers.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Lautstärke von Musik, Effekten und Bordfunk, jeweils 0..1; Bordfunk auf 0 schaltet ihn samt Untertiteln ab.
    pub music_volume: f32,
    pub effects_volume: f32,
    pub voice_volume: f32,
    /// Schüsse im Kampf.
    pub attack_speed: AnimationSpeed,
    /// Flüge der Schiffe: Deploy-Einflug und Lane-Wechsel.
    pub movement_speed: AnimationSpeed,
    /// Alle übrigen Animationen.
    pub animation_speed: AnimationSpeed,
    pub camera: CameraSettings,
    pub cursor: CursorId,
    /// Kantenlänge des Mauszeigers in logischen Pixeln (Punkten), siehe [`crate::ui::cursor`].
    pub cursor_size: f32,
    pub window_mode: WindowMode,
    /// Gilt für die Kamera der Szene, siehe [`crate::scene::camera`].
    pub antialiasing: Antialiasing,
    /// FPS-Anzeige oben links, siehe [`crate::ui::fps`].
    pub show_fps: bool,
    /// Bildaufbau im Takt des Monitors (`PresentMode::Fifo`), sonst ungebremst.
    pub vsync: bool,
    pub language: Option<Language>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music_volume: 0.6,
            effects_volume: 0.8,
            voice_volume: 0.8,
            attack_speed: AnimationSpeed::Normal,
            movement_speed: AnimationSpeed::Normal,
            animation_speed: AnimationSpeed::Normal,
            camera: CameraSettings::default(),
            cursor: CursorId::Arrow,
            cursor_size: 32.0,
            window_mode: WindowMode::Maximized,
            antialiasing: Antialiasing::X4,
            show_fps: false,
            vsync: true,
            language: None,
        }
    }
}

impl Settings {
    /// Pfad der Datei im Konfigurationsordner des Systems.
    pub fn path() -> Option<PathBuf> {
        config_file("settings.json")
    }

    /// Liest die gespeicherten Einstellungen; ohne Datei (erster Start) gelten die Standardwerte.
    pub fn load() -> Self {
        let Some(json) = Self::path().and_then(|path| fs::read_to_string(path).ok()) else {
            return Self::default();
        };
        serde_json::from_str(&json).unwrap_or_else(|error| {
            warn!("Einstellungen unlesbar, nehme Standardwerte: {error}");
            Self::default()
        })
    }

    fn save(&self) -> std::io::Result<()> {
        let path =
            Self::path().ok_or_else(|| std::io::Error::other("kein Konfigurationsordner"))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

/// Lädt die Einstellungen beim Start, speichert jede Änderung und setzt Fenstermodus und VSync.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings::load()).add_systems(
            Update,
            (
                save_settings
                    .run_if(resource_changed::<Settings>.and_then(not(resource_added::<Settings>))),
                (apply_window_mode, apply_vsync).run_if(resource_changed::<Settings>),
            ),
        );
    }
}

fn save_settings(settings: Res<Settings>) {
    if let Err(error) = settings.save() {
        error!("Einstellungen nicht gespeichert: {error}");
    }
}

/// Setzt den Fenstermodus beim Start und bei jeder Änderung der Einstellung.
fn apply_window_mode(
    settings: Res<Settings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<Option<WindowMode>>,
) {
    let mode = settings.window_mode;
    if *applied == Some(mode) {
        return;
    }
    *applied = Some(mode);
    match mode {
        WindowMode::Windowed => {
            window.mode = BevyWindowMode::Windowed;
            window.set_maximized(false);
        }
        WindowMode::Maximized => {
            window.mode = BevyWindowMode::Windowed;
            window.set_maximized(true);
        }
        WindowMode::Fullscreen => {
            window.mode = BevyWindowMode::BorderlessFullscreen(MonitorSelection::Current);
        }
    }
}

/// Setzt VSync beim Start und bei jeder Änderung der Einstellung.
fn apply_vsync(settings: Res<Settings>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    let mode = if settings.vsync {
        PresentMode::Fifo
    } else {
        PresentMode::AutoNoVsync
    };
    if window.present_mode != mode {
        window.present_mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Die Datei schreibt Schlüssel wie die Web-Fassung (`musicVolume`, `veryFast` …), fehlende nehmen ihren Standard.
    #[test]
    fn json_round_trip() {
        let settings = Settings {
            attack_speed: AnimationSpeed::VeryFast,
            antialiasing: Antialiasing::X2,
            language: Some(Language::De),
            ..default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"attackSpeed\":\"veryFast\""));
        assert!(json.contains("\"antialiasing\":\"x2\""));
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), settings);
        let partial: Settings = serde_json::from_str(r#"{"musicVolume":0.1}"#).unwrap();
        assert_eq!(partial.music_volume, 0.1);
        assert_eq!(partial.window_mode, WindowMode::Maximized);
        assert_eq!(partial.antialiasing, Antialiasing::X4);
    }
}
