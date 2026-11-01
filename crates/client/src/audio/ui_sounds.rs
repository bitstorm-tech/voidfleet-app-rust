//! Klänge der Oberfläche, im Code synthetisiert statt aus Dateien (Port von `audio/uiSounds.ts`): kurze Töne und
//! gefiltertes Rauschen mit schnellen Hüllkurven, alle durch einen Tiefpass, damit nichts schrill wird. Die Klänge
//! entstehen einmal beim Start als Sample-Puffer; abgespielt wird mit [`PlayUiSound`].

use std::f32::consts::{PI, TAU};

use bevy::{platform::collections::HashMap, prelude::*};
use kira::{
    Frame,
    effect::filter::{FilterBuilder, FilterMode},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
    track::TrackBuilder,
};
use rand::Rng;

use super::Mixer;

const SAMPLE_RATE: u32 = 48_000;

/// Bedeutung eines Klangs der Oberfläche.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiSound {
    /// Knopf-Klick und Auswahl in Dialogen.
    Click,
    /// Überfahren einer Karte.
    Hover,
    /// Karte aufnehmen: heller Aufwärts-Blip.
    Select,
    /// Schiff oder Ziel einplanen: tiefer und voller als die Auswahl.
    Place,
    /// Rücknahme.
    Cancel,
    /// Zug beenden: drei aufsteigende Töne, der letzte klingt am längsten.
    Confirm,
    /// Geht nicht, verpufft: zweimal ein tiefes Brummen.
    Denied,
    /// Eine Karte fliegt auf die Hand.
    Draw,
    /// Neue Runde: ein weicher, anschwellender Akkord.
    Round,
}

impl UiSound {
    pub const ALL: [Self; 9] = [
        Self::Click,
        Self::Hover,
        Self::Select,
        Self::Place,
        Self::Cancel,
        Self::Confirm,
        Self::Denied,
        Self::Draw,
        Self::Round,
    ];

    /// Rechnet den Klang.
    fn synthesize(self) -> Vec<f32> {
        let mut out = Synth::default();
        match self {
            Self::Click => out.tone(0.0, Wave::Sine, 2000.0, 1200.0, 0.04, 0.25),
            Self::Hover => out.tone(0.0, Wave::Sine, 2800.0, 2500.0, 0.03, 0.05),
            Self::Select => {
                out.tone(0.0, Wave::Triangle, 880.0, 1320.0, 0.09, 0.2);
                out.tone(0.0, Wave::Sine, 1760.0, 2640.0, 0.09, 0.05);
            }
            Self::Place => {
                out.tone(0.0, Wave::Triangle, 520.0, 780.0, 0.11, 0.26);
                out.tone(0.0, Wave::Sine, 1040.0, 1560.0, 0.11, 0.06);
            }
            Self::Cancel => out.tone(0.0, Wave::Triangle, 880.0, 520.0, 0.12, 0.2),
            Self::Confirm => {
                out.tone(0.0, Wave::Triangle, 660.0, 660.0, 0.18, 0.17);
                out.tone(0.07, Wave::Triangle, 880.0, 880.0, 0.26, 0.17);
                out.tone(0.14, Wave::Triangle, 1320.0, 1320.0, 0.34, 0.17);
            }
            Self::Denied => {
                out.tone(0.0, Wave::Square, 150.0, 140.0, 0.08, 0.08);
                out.tone(0.11, Wave::Square, 150.0, 140.0, 0.11, 0.08);
            }
            Self::Draw => out.swish(800.0, 4000.0, 0.2, 0.15),
            Self::Round => {
                out.tone_with_attack(0.0, Wave::Sine, 220.0, 220.0, 1.2, 0.16, 0.25);
                out.tone_with_attack(0.0, Wave::Sine, 330.0, 330.0, 1.2, 0.1, 0.3);
                out.tone_with_attack(0.1, Wave::Sine, 880.0, 880.0, 0.9, 0.04, 0.2);
            }
        }
        out.0
    }
}

/// Spielt einen Klang der Oberfläche.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayUiSound(pub UiSound);

/// Die gerechneten Klänge.
#[derive(Resource)]
struct UiSounds(HashMap<UiSound, StaticSoundData>);

impl Default for UiSounds {
    fn default() -> Self {
        let sounds = UiSound::ALL.into_iter().map(|sound| {
            let frames: Vec<Frame> = sound
                .synthesize()
                .into_iter()
                .map(Frame::from_mono)
                .collect();
            let data = StaticSoundData {
                sample_rate: SAMPLE_RATE,
                frames: frames.into(),
                settings: StaticSoundSettings::default(),
                slice: None,
            };
            (sound, data)
        });
        Self(sounds.collect())
    }
}

/// Spur der Klänge unter dem Effekt-Bus: ein Tiefpass, damit nichts schrill wird.
pub(super) fn track() -> TrackBuilder {
    TrackBuilder::new().with_effect(
        FilterBuilder::new()
            .mode(FilterMode::LowPass)
            .cutoff(3500.0),
    )
}

pub(super) struct UiSoundsPlugin;

impl Plugin for UiSoundsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSounds>()
            .add_message::<PlayUiSound>()
            .add_systems(Update, play_ui_sounds.run_if(resource_exists::<Mixer>));
    }
}

fn play_ui_sounds(
    mut messages: MessageReader<PlayUiSound>,
    mut mixer: ResMut<Mixer>,
    sounds: Res<UiSounds>,
) {
    for PlayUiSound(sound) in messages.read() {
        if let Err(error) = mixer.ui.play(sounds.0[sound].clone()) {
            warn!("UI-Klang {sound:?} nicht abgespielt: {error}");
        }
    }
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Triangle,
    Square,
}

impl Wave {
    fn sample(self, phase: f32) -> f32 {
        match self {
            Self::Sine => phase.sin(),
            Self::Triangle => 2.0 / PI * phase.sin().asin(),
            Self::Square => phase.sin().signum(),
        }
    }
}

/// Ein Puffer, in den Töne und Rauschen addiert werden (mono).
#[derive(Default)]
struct Synth(Vec<f32>);

impl Synth {
    fn add(&mut self, start: f32, samples: impl Iterator<Item = f32>) {
        let offset = (start * SAMPLE_RATE as f32) as usize;
        for (i, value) in samples.enumerate() {
            if self.0.len() <= offset + i {
                self.0.resize(offset + i + 1, 0.0);
            }
            self.0[offset + i] += value;
        }
    }

    fn tone(&mut self, start: f32, wave: Wave, from: f32, to: f32, duration: f32, volume: f32) {
        self.tone_with_attack(start, wave, from, to, duration, volume, 0.004);
    }

    /// Ein Ton, der in `duration` Sekunden von `from` nach `to` Hz gleitet und dabei verklingt.
    fn tone_with_attack(
        &mut self,
        start: f32,
        wave: Wave,
        from: f32,
        to: f32,
        duration: f32,
        volume: f32,
        attack: f32,
    ) {
        let mut phase = 0.0_f32;
        let samples = samples(duration).map(move |t| {
            let frequency = from * (to / from).powf(t / duration);
            phase = (phase + TAU * frequency / SAMPLE_RATE as f32) % TAU;
            wave.sample(phase) * envelope(t, attack, volume, duration)
        });
        self.add(start, samples);
    }

    /// Rauschen durch einen Bandpass, der von `from` nach `to` Hz wandert.
    fn swish(&mut self, from: f32, to: f32, duration: f32, volume: f32) {
        let mut rng = rand::rng();
        let mut filter = Bandpass::default();
        let samples: Vec<f32> = samples(duration)
            .map(|t| {
                let frequency = from * (to / from).powf(t / duration);
                filter.process(rng.random_range(-1.0..1.0), frequency, 1.5)
                    * envelope(t, 0.03, volume, duration)
            })
            .collect();
        self.add(0.0, samples.into_iter());
    }
}

/// Zeitpunkte (Sekunden) aller Samples über `duration`.
fn samples(duration: f32) -> impl Iterator<Item = f32> {
    (0..(duration * SAMPLE_RATE as f32) as usize).map(|i| i as f32 / SAMPLE_RATE as f32)
}

/// Linear an in `attack`, dann exponentiell aus bis `duration` (wie `exponentialRampToValueAtTime` auf 0,0001).
fn envelope(t: f32, attack: f32, peak: f32, duration: f32) -> f32 {
    if t < attack {
        peak * t / attack
    } else {
        peak * (0.0001 / peak).powf((t - attack) / (duration - attack))
    }
}

/// Biquad-Bandpass (RBJ, konstante Spitze 0 dB) mit Frequenz je Sample.
#[derive(Default)]
struct Bandpass {
    x: [f32; 2],
    y: [f32; 2],
}

impl Bandpass {
    fn process(&mut self, input: f32, frequency: f32, q: f32) -> f32 {
        let omega = TAU * frequency / SAMPLE_RATE as f32;
        let alpha = omega.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let output = (alpha * input - alpha * self.x[1] + 2.0 * omega.cos() * self.y[0]
            - (1.0 - alpha) * self.y[1])
            / a0;
        self.x = [input, self.x[0]];
        self.y = [output, self.y[0]];
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_short_and_bounded() {
        for sound in UiSound::ALL {
            let samples = sound.synthesize();
            let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
            let peak = samples.iter().fold(0.0_f32, |max, v| max.max(v.abs()));
            assert!(seconds > 0.02 && seconds < 1.4, "{sound:?}: {seconds} s");
            assert!(peak > 0.01 && peak < 1.0, "{sound:?}: Spitze {peak}");
        }
    }
}
