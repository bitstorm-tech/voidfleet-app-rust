//! Ton über [Kira](kira) (Port von `src/audio/`): ein Mixer mit je einem Bus für Musik, Effekte und Bordfunk, deren
//! Lautstärke aus den Einstellungen kommt, und einem Limiter dahinter, damit viele gleichzeitige Explosionen nicht
//! übersteuern. Solange der Bordfunk spricht, weicht die Musik zurück.
//!
//! - [`effects`] — Soundeffekte: [`PlaySound`] für einmalige, [`LoopingSound`] als Komponente für Schleifen
//!   (Triebwerke), Varianten, Verstimmung, Stereopanorama nach der Lage im Bild, höchstens 4 Stimmen je Sound.
//! - [`music`] — Hintergrundmusik in zwei Stimmungen ([`MusicMood`] als Ressource [`Music`]) mit Überblendung.
//! - [`ui_sounds`] — Klänge der Oberfläche, im Code synthetisiert ([`PlayUiSound`]).
//! - [`radio`] — Bordfunk mit Warteschlange, Prioritäten und Funkstrecke ([`Say`], [`SilenceRadio`],
//!   [`RadioTransmission`]).
//!
//! Solange Animationen übersprungen werden ([`crate::scene::animation::AnimationClock::skip`]), schweigen Effekte und Funk.
//! Ohne Audiogerät läuft das Spiel stumm weiter: Dann fehlt die Ressource [`Mixer`], und die Systeme ruhen.

pub mod effects;
pub mod music;
pub mod radio;
pub mod sample;
pub mod ui_sounds;

use std::time::Duration;

use bevy::{platform::collections::HashMap, prelude::*};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
    effect::compressor::CompressorBuilder,
    track::{MainTrackBuilder, TrackBuilder, TrackHandle},
};

pub use effects::{LoopingSound, PlaySound, StopSounds};
pub use music::Music;
pub use radio::{RadioTransmission, Say, SilenceRadio, Transmission};
pub use sample::{MusicTrack, Sample};
pub use ui_sounds::{PlayUiSound, UiSound};

pub use crate::data::{radio::RadioCue, sounds::MusicMood, sounds::SoundId};
use crate::{assets::Preload, settings::Settings};

/// So weit weicht die Musik zurück, solange der Bordfunk spricht.
const MUSIC_DUCK: f32 = 0.45;

/// Der Mixer: Audiogerät und Busse.
#[derive(Resource)]
pub struct Mixer {
    /// Hält das Audiogerät offen, solange der Mixer lebt.
    _manager: AudioManager,
    /// Bus der Musik (Lautstärke aus den Einstellungen) und darunter die Spur, die beim Funk zurückweicht.
    music: TrackHandle,
    music_duck: TrackHandle,
    effects: TrackHandle,
    /// Unter den Effekten: Klänge der Oberfläche durch einen Tiefpass.
    ui: TrackHandle,
    /// Bordfunk samt Funkstrecke.
    voice: TrackHandle,
}

impl Mixer {
    fn new() -> Result<Self, BevyError> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings {
            main_track_builder: MainTrackBuilder::new().with_effect(
                CompressorBuilder::new()
                    .threshold(-6.0)
                    .ratio(20.0)
                    .attack_duration(Duration::from_millis(3))
                    .release_duration(Duration::from_millis(250)),
            ),
            ..default()
        })?;
        let mut music = manager.add_sub_track(TrackBuilder::new())?;
        let music_duck = music.add_sub_track(TrackBuilder::new())?;
        let mut effects = manager.add_sub_track(TrackBuilder::new())?;
        let ui = effects.add_sub_track(ui_sounds::track())?;
        let voice = manager.add_sub_track(radio::transmitter())?;
        Ok(Self {
            _manager: manager,
            music,
            music_duck,
            effects,
            ui,
            voice,
        })
    }

    /// Setzt die Lautstärke der Busse; beim Ziehen am Regler gleitet sie, statt zu knacken.
    fn set_volumes(&mut self, settings: &Settings) {
        let tween = tween(0.05);
        self.music
            .set_volume(bus_gain(settings.music_volume), tween);
        self.effects
            .set_volume(bus_gain(settings.effects_volume), tween);
        self.voice
            .set_volume(bus_gain(settings.voice_volume), tween);
    }

    /// Senkt die Musik zügig ab, solange der Bordfunk spricht, und lässt sie danach langsam zurückkommen.
    fn duck_music(&mut self, active: bool) {
        let (level, seconds) = if active {
            (MUSIC_DUCK, 0.25)
        } else {
            (1.0, 1.5)
        };
        self.music_duck.set_volume(decibels(level), tween(seconds));
    }
}

/// Der Regler wirkt quadratisch, so fühlen sich seine Schritte gleichmäßiger an als linear.
fn bus_gain(volume: f32) -> Decibels {
    decibels(volume * volume)
}

/// Amplitude → Dezibel; ab −60 dB schweigt Kira.
pub(crate) fn decibels(amplitude: f32) -> Decibels {
    if amplitude <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * amplitude.log10())
    }
}

/// Linearer Übergang über so viele Sekunden.
pub(crate) fn tween(seconds: f32) -> Tween {
    Tween {
        duration: Duration::from_secs_f32(seconds),
        ..default()
    }
}

/// Zufällig eine Variante, aber nie zweimal dieselbe hintereinander; `last` merkt sich die letzte je Id.
pub(crate) fn pick_variant<K: Eq + std::hash::Hash>(
    last: &mut HashMap<K, usize>,
    id: K,
    count: usize,
) -> usize {
    use rand::Rng;
    let previous = last.get(&id).copied();
    let choices: Vec<usize> = (0..count)
        .filter(|&i| count == 1 || Some(i) != previous)
        .collect();
    let index = choices[rand::rng().random_range(0..choices.len())];
    last.insert(id, index);
    index
}

/// Alle Aufnahmen, beim Start geladen.
#[derive(Resource)]
pub struct AudioLibrary {
    pub effects: HashMap<SoundId, Vec<Handle<Sample>>>,
    pub radio: HashMap<RadioCue, Vec<Handle<Sample>>>,
    pub music: HashMap<MusicMood, Handle<MusicTrack>>,
}

impl FromWorld for AudioLibrary {
    fn from_world(world: &mut World) -> Self {
        fn samples(
            server: &AssetServer,
            preload: &mut Preload,
            paths: impl Iterator<Item = String>,
        ) -> Vec<Handle<Sample>> {
            paths.map(|path| preload.track(server.load(path))).collect()
        }
        let server = world.resource::<AssetServer>().clone();
        let mut preload = world.resource_mut::<Preload>();
        let effects = SoundId::ALL
            .into_iter()
            .map(|id| (id, samples(&server, &mut preload, id.paths())))
            .collect();
        let radio = RadioCue::ALL
            .iter()
            .map(|&cue| (cue, samples(&server, &mut preload, cue.paths())))
            .collect();
        let music = MusicMood::ALL
            .into_iter()
            .map(|mood| (mood, preload.track(server.load(mood.path()))))
            .collect();
        Self {
            effects,
            radio,
            music,
        }
    }
}

/// Der Ton des Spiels.
pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Sample>()
            .init_asset::<MusicTrack>()
            .init_asset_loader::<sample::SampleLoader>()
            .init_asset_loader::<sample::MusicLoader>()
            // Erst wenn die Loader stehen.
            .init_resource::<AudioLibrary>()
            .add_systems(Startup, start_mixer)
            .add_systems(
                Update,
                apply_volumes
                    .run_if(resource_exists::<Mixer>.and_then(resource_changed::<Settings>)),
            )
            .add_plugins((
                effects::EffectsPlugin,
                music::MusicPlugin,
                ui_sounds::UiSoundsPlugin,
                radio::RadioPlugin,
            ));
    }
}

fn start_mixer(mut commands: Commands, settings: Res<Settings>) {
    match Mixer::new() {
        Ok(mut mixer) => {
            mixer.set_volumes(&settings);
            commands.insert_resource(mixer);
        }
        Err(error) => warn!("Kein Ton: {error}"),
    }
}

fn apply_volumes(mut mixer: ResMut<Mixer>, settings: Res<Settings>) {
    mixer.set_volumes(&settings);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_never_repeat() {
        let mut last = HashMap::default();
        let mut previous = pick_variant(&mut last, "x", 3);
        for _ in 0..50 {
            let next = pick_variant(&mut last, "x", 3);
            assert_ne!(next, previous);
            previous = next;
        }
        assert_eq!(pick_variant(&mut last, "y", 1), 0);
        assert_eq!(pick_variant(&mut last, "y", 1), 0);
    }

    #[test]
    fn decibel_conversion() {
        assert_eq!(decibels(1.0), Decibels::IDENTITY);
        assert_eq!(decibels(0.0), Decibels::SILENCE);
        assert!((decibels(0.5).0 + 6.0206).abs() < 1e-3);
    }
}
