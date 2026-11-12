//! Laden beim Start (Port von `preload.ts`): Alle Assets werden geladen, bevor das Match beginnt, damit im Spiel nichts
//! nachgeladen wird. Danach stehen in der Generalprobe ([`LoadingPhase::Rehearsal`]) einmal alle Schiffe und Wracks
//! hinter dem Ladebildschirm, damit Bevy ihre Pipelines übersetzt, bevor sie gebraucht werden.
//!
//! Jeder Katalog meldet seine Handles bei [`Preload`] an; [`LoadingProgress`] zeigt der Ladebildschirm.

use bevy::{
    asset::{RenderAssetUsages, UntypedAssetId},
    image::ImageLoaderSettings,
    prelude::*,
};

use crate::data::DataPlugin;

/// Ablauf der App: erst laden, dann das Match.
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Loading,
    Match,
}

/// Abschnitte des Ladens: erst die Dateien, dann die Generalprobe.
#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::Loading)]
pub enum LoadingPhase {
    #[default]
    Files,
    Rehearsal,
}

/// Anteil der Generalprobe am Fortschrittsbalken; den Rest teilen sich die Dateien.
const REHEARSAL_SHARE: f32 = 0.1;
/// So viele Bilder steht die Generalprobe, bis alle Pipelines übersetzt sind (Bevy übersetzt im Hintergrund).
const REHEARSAL_FRAMES: u32 = 30;

/// Alle Assets, auf die der Ladebildschirm wartet. Die Handles bleiben hier, damit nichts wieder entladen wird.
#[derive(Resource, Default)]
pub struct Preload(Vec<UntypedHandle>);

/// Lädt ein Bild, das nur die Grafikkarte braucht (Karten, Porträts, Hintergründe): Die Kopie im Hauptspeicher fällt nach
/// dem Hochladen weg, Größe und Format bleiben lesbar.
pub fn gpu_image(server: &AssetServer, path: String) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
        })
        .load(path)
}

impl Preload {
    /// Meldet ein Asset an und gibt den Handle zurück.
    pub fn track<A: Asset>(&mut self, handle: Handle<A>) -> Handle<A> {
        self.0.push(handle.clone().untyped());
        handle
    }

    fn ids(&self) -> impl Iterator<Item = UntypedAssetId> + '_ {
        self.0.iter().map(UntypedHandle::id)
    }
}

/// Fortschritt des Ladens (0–1) und ob eine Datei nicht geladen werden konnte.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct LoadingProgress {
    pub fraction: f32,
    pub failed: bool,
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<LoadingPhase>()
            .init_resource::<Preload>()
            .init_resource::<LoadingProgress>()
            .add_plugins(DataPlugin)
            .add_systems(Update, track_files.run_if(in_state(LoadingPhase::Files)))
            .add_systems(Update, rehearse.run_if(in_state(LoadingPhase::Rehearsal)));
    }
}

fn track_files(
    server: Res<AssetServer>,
    preload: Res<Preload>,
    mut progress: ResMut<LoadingProgress>,
    mut phase: ResMut<NextState<LoadingPhase>>,
) {
    if progress.failed {
        return;
    }
    let mut loaded = 0;
    for id in preload.ids() {
        if server.load_state(id).is_failed()
            || server.recursive_dependency_load_state(id).is_failed()
        {
            error!("Asset nicht geladen: {:?}", server.get_path(id));
            progress.failed = true;
            return;
        }
        if server.is_loaded_with_dependencies(id) {
            loaded += 1;
        }
    }
    let total = preload.0.len().max(1);
    progress.set_if_neq(LoadingProgress {
        fraction: loaded as f32 / total as f32 * (1.0 - REHEARSAL_SHARE),
        failed: false,
    });
    if loaded == preload.0.len() {
        phase.set(LoadingPhase::Rehearsal);
    }
}

fn rehearse(
    mut frames: Local<u32>,
    mut progress: ResMut<LoadingProgress>,
    mut state: ResMut<NextState<AppState>>,
) {
    *frames += 1;
    progress.fraction = 1.0 - REHEARSAL_SHARE
        + REHEARSAL_SHARE * (*frames as f32 / REHEARSAL_FRAMES as f32).min(1.0);
    if *frames >= REHEARSAL_FRAMES {
        state.set(AppState::Match);
    }
}
