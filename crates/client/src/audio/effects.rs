//! Soundeffekte (Port von `audio/sounds.ts`): Jedes Abspielen wählt eine zufällige Variante (nie zweimal dieselbe
//! hintereinander), verstimmt sie leicht und setzt erst nach dem stillen Vorlauf ein. Die Stereoposition folgt dem
//! Geschehen im Bild, in jeder Kameraansicht. Höchstens 4 Stimmen je Sound: Eine Salve mit acht Einschlägen wäre sonst
//! nur laut. Solange Animationen übersprungen werden, bleibt es still.
//!
//! Einmalige Effekte schickt man als [`PlaySound`]; Schleifen (Triebwerke) hängen als [`LoopingSound`] an einer Entity
//! mit Lage, folgen ihr im Panorama und blenden aus, sobald die Komponente oder die Entity verschwindet.

use bevy::{platform::collections::HashMap, prelude::*, transform::TransformSystems};
use kira::{
    Panning, PlaybackRate,
    sound::{PlaybackState, static_sound::StaticSoundHandle},
};
use rand::Rng;

use super::{AudioLibrary, Mixer, Sample, SoundId, decibels, pick_variant, tween};
use crate::scene::{
    animation::AnimationClock,
    camera::{MainCamera, screen_side},
};

/// Ab diesem Anteil der halben Bildbreite liegt ein Sound ganz außen im Stereobild; ganz hart schwenkt nichts.
const PAN_EDGE: f32 = 0.8;
const MAX_PAN: f32 = 0.8;
/// Zufällige Verstimmung als Anteil der Abspielrate, damit wiederholte Samples nicht mechanisch klingen.
const DETUNE: f64 = 0.06;
/// Gleichzeitige Stimmen je Sound, Schleifen eingeschlossen.
const MAX_VOICES: usize = 4;
/// Sekunden: Schleifen setzen sanft ein, Stoppen blendet aus, das Panorama gleitet nach.
const LOOP_FADE_IN: f32 = 0.15;
const STOP_FADE: f32 = 0.12;
const PAN_GLIDE: f32 = 0.05;

/// Spielt einen Effekt einmal ab.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlaySound {
    pub id: SoundId,
    /// Wo es geschieht (Weltpunkt); ohne Angabe klingt es aus der Mitte.
    pub at: Option<Vec3>,
    /// Faktor auf die Lautstärke des Sounds.
    pub volume: f32,
    /// Abspielrate: unter 1 tiefer und länger, darüber höher und kürzer.
    pub rate: f32,
    /// Diese Variante statt einer zufälligen.
    pub variant: Option<usize>,
}

impl PlaySound {
    pub fn new(id: SoundId) -> Self {
        Self {
            id,
            at: None,
            volume: 1.0,
            rate: 1.0,
            variant: None,
        }
    }

    pub fn at(self, at: Vec3) -> Self {
        Self {
            at: Some(at),
            ..self
        }
    }

    pub fn volume(self, volume: f32) -> Self {
        Self { volume, ..self }
    }

    pub fn rate(self, rate: f32) -> Self {
        Self { rate, ..self }
    }

    pub fn variant(self, variant: usize) -> Self {
        Self {
            variant: Some(variant),
            ..self
        }
    }
}

/// Ein Effekt in Schleife an dieser Entity (Lage aus ihrer `GlobalTransform`); verschwindet die Komponente oder die
/// Entity, blendet er aus.
#[derive(Component, Clone, Copy, Debug)]
pub struct LoopingSound {
    pub id: SoundId,
    pub volume: f32,
    pub rate: f32,
}

impl LoopingSound {
    pub fn new(id: SoundId) -> Self {
        Self {
            id,
            volume: 1.0,
            rate: 1.0,
        }
    }
}

/// Blendet alle laufenden Effekte aus, etwa wenn der Spieler die Runde überspringt.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct StopSounds;

/// Die laufende Stimme einer [`LoopingSound`].
#[derive(Component)]
struct LoopVoice {
    id: SoundId,
    handle: StaticSoundHandle,
}

/// Laufende einmalige Effekte je Sound und die zuletzt gewählte Variante.
#[derive(Resource, Default)]
struct Voices {
    active: HashMap<SoundId, Vec<StaticSoundHandle>>,
    last_variant: HashMap<SoundId, usize>,
}

/// Was abgespielt werden soll.
struct Voice {
    id: SoundId,
    pan: f32,
    volume: f32,
    rate: f32,
    variant: Option<usize>,
    looped: bool,
}

impl Voices {
    /// Noch laufende einmalige Stimmen eines Sounds.
    fn running(&mut self, id: SoundId) -> usize {
        let active = self.active.entry(id).or_default();
        active.retain(|voice| voice.state() != PlaybackState::Stopped);
        active.len()
    }

    /// Startet eine Stimme; `None`, wenn das Sample fehlt oder Kira sie nicht annimmt.
    fn start(
        &mut self,
        mixer: &mut Mixer,
        library: &AudioLibrary,
        samples: &Assets<Sample>,
        voice: &Voice,
    ) -> Option<StaticSoundHandle> {
        let variants = &library.effects[&voice.id];
        let variant = voice
            .variant
            .unwrap_or_else(|| pick_variant(&mut self.last_variant, voice.id, variants.len()));
        let sample = samples.get(&variants[variant])?;
        let level = sample.gain * voice.id.volume() * voice.volume;
        let detune = 1.0 + (rand::rng().random::<f64>() - 0.5) * DETUNE;
        let mut data = sample
            .data
            .start_position(sample.onset)
            .playback_rate(PlaybackRate(f64::from(voice.rate) * detune))
            .panning(Panning(voice.pan))
            .volume(decibels(level));
        if voice.looped {
            data = data
                .loop_region(sample.onset..)
                .fade_in_tween(tween(LOOP_FADE_IN));
        }
        mixer
            .effects
            .play(data)
            .inspect_err(|error| warn!("Sound {:?} nicht abgespielt: {error}", voice.id))
            .ok()
    }
}

pub(super) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Voices>()
            .add_message::<PlaySound>()
            .add_message::<StopSounds>()
            .add_observer(stop_loop)
            // Nach `TransformSystems::Propagate`, damit auch die Schleife einer eben gespawnten Entity dort im Panorama
            // beginnt, wo sie im Bild steht.
            .add_systems(
                PostUpdate,
                (
                    play_sounds,
                    start_loops,
                    follow_loops,
                    stop_sounds.run_if(on_message::<StopSounds>),
                )
                    .chain()
                    .after(TransformSystems::Propagate)
                    .run_if(resource_exists::<Mixer>),
            );
    }
}

type MainCameraView<'w, 's> =
    Option<Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>>;

/// Lage eines Weltpunkts im Stereobild.
fn pan_at(camera: &MainCameraView, at: Option<Vec3>) -> f32 {
    let side = at
        .zip(camera.as_deref())
        .and_then(|(at, (camera, transform))| screen_side(camera, transform, at));
    side.map_or(0.0, |side| MAX_PAN * (side / PAN_EDGE).clamp(-1.0, 1.0))
}

fn play_sounds(
    mut messages: MessageReader<PlaySound>,
    mut mixer: ResMut<Mixer>,
    mut voices: ResMut<Voices>,
    library: Res<AudioLibrary>,
    samples: Res<Assets<Sample>>,
    clock: Res<AnimationClock>,
    loops: Query<&LoopVoice>,
    camera: MainCameraView,
) {
    for request in messages.read() {
        let looping = loops.iter().filter(|voice| voice.id == request.id).count();
        if clock.skip || voices.running(request.id) + looping >= MAX_VOICES {
            continue;
        }
        let voice = Voice {
            id: request.id,
            pan: pan_at(&camera, request.at),
            volume: request.volume,
            rate: request.rate,
            variant: request.variant,
            looped: false,
        };
        if let Some(handle) = voices.start(&mut mixer, &library, &samples, &voice) {
            voices.active.entry(request.id).or_default().push(handle);
        }
    }
}

/// Startet die Schleifen neuer [`LoopingSound`]s (auch nach Änderung der Komponente).
fn start_loops(
    mut commands: Commands,
    mut added: Query<
        (
            Entity,
            &LoopingSound,
            &GlobalTransform,
            Option<&mut LoopVoice>,
        ),
        Changed<LoopingSound>,
    >,
    mut mixer: ResMut<Mixer>,
    mut voices: ResMut<Voices>,
    library: Res<AudioLibrary>,
    samples: Res<Assets<Sample>>,
    clock: Res<AnimationClock>,
    camera: MainCameraView,
) {
    for (entity, sound, transform, previous) in &mut added {
        if let Some(mut previous) = previous {
            previous.handle.stop(tween(STOP_FADE));
        }
        if clock.skip {
            continue;
        }
        let voice = Voice {
            id: sound.id,
            pan: pan_at(&camera, Some(transform.translation())),
            volume: sound.volume,
            rate: sound.rate,
            variant: None,
            looped: true,
        };
        if let Some(handle) = voices.start(&mut mixer, &library, &samples, &voice) {
            commands.entity(entity).insert(LoopVoice {
                id: sound.id,
                handle,
            });
        }
    }
}

/// Schwenkt die Schleifen sanft dorthin, wo ihre Entity im Bild steht.
fn follow_loops(mut loops: Query<(&GlobalTransform, &mut LoopVoice)>, camera: MainCameraView) {
    for (transform, mut voice) in &mut loops {
        let pan = pan_at(&camera, Some(transform.translation()));
        voice.handle.set_panning(Panning(pan), tween(PAN_GLIDE));
    }
}

/// Blendet die Schleife aus, sobald ihre Komponente oder Entity verschwindet.
fn stop_loop(
    removed: On<Remove<LoopingSound>>,
    mut commands: Commands,
    mut voices: Query<&mut LoopVoice>,
) {
    if let Ok(mut voice) = voices.get_mut(removed.entity) {
        voice.handle.stop(tween(STOP_FADE));
        if let Ok(mut entity) = commands.get_entity(removed.entity) {
            entity.try_remove::<LoopVoice>();
        }
    }
}

fn stop_sounds(mut voices: ResMut<Voices>, mut loops: Query<&mut LoopVoice>) {
    for voice in voices.active.values_mut().flatten() {
        voice.stop(tween(STOP_FADE));
    }
    for mut voice in &mut loops {
        voice.handle.stop(tween(STOP_FADE));
    }
}
