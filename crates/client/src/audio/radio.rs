//! Bordfunk (Port von `audio/radio.ts`): kurze Funksprüche der eigenen Flotte zu den Momenten, auf die es ankommt —
//! Torpedo im Anflug, Abschuss, Overdrive, Sieg. Ausgelöst mit [`Say`]; jeden gesendeten Spruch (und das Ende bei
//! Abbruch) meldet [`RadioTransmission`] für den Untertitel.
//!
//! Es spricht immer nur einer: Ein Spruch, der bei belegtem Funk ankommt, wartet höchstens 2 s und nur, wenn er
//! wichtiger ist als der, der schon wartet; sonst entfällt er. Derselbe Spruch wiederholt sich je nach Priorität
//! frühestens nach 60 bzw. 15 s (Priorität 3 immer). Gesendet wird über eine Funkstrecke — Bandpass, Präsenz und etwas
//! Sättigung auf der Spur des Bordfunks, dazu Rauschen, ein Piepen beim Drücken und ein Rauschstoß beim Loslassen der
//! Sprechtaste; die Musik weicht solange zurück. Mit Lautstärke 0 schweigt der Bordfunk, ebenso solange Animationen
//! übersprungen werden; [`SilenceRadio`] bricht ab.

use std::time::Duration;

use bevy::{platform::collections::HashMap, prelude::*};
use kira::{
    Decibels, Frame, StartTime,
    effect::{
        distortion::{DistortionBuilder, DistortionKind},
        eq_filter::{EqFilterBuilder, EqFilterKind},
        filter::{FilterBuilder, FilterMode},
    },
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
    track::TrackBuilder,
};
use rand::Rng;

use super::{AudioLibrary, Mixer, RadioCue, Sample, decibels, pick_variant, tween};
use crate::{scene::tween::AnimationClock, settings::Settings};

/// So lange (Sekunden) darf ein Spruch auf den freien Funk warten, und die Pause zwischen zwei Sprüchen.
const MAX_WAIT: f64 = 2.0;
const GAP: f64 = 0.3;
/// Vom Drücken der Sprechtaste bis zur Stimme und von ihrem Ende bis zum Loslassen.
const KEY_UP: f64 = 0.12;
const TAIL: f64 = 0.18;
const VOICE_LEVEL: f32 = 0.7;
const HISS_LEVEL: f32 = 0.012;
const CARRIER_RATE: u32 = 48_000;

/// Ein Funkspruch, sobald der Funk frei ist; `variant` wählt eine bestimmte Aufnahme und übergeht die Sperre gegen
/// Wiederholungen.
#[derive(Message, Clone, Copy, Debug)]
pub struct Say {
    pub cue: RadioCue,
    pub variant: Option<usize>,
}

impl Say {
    pub fn new(cue: RadioCue) -> Self {
        Self { cue, variant: None }
    }
}

/// Bricht den laufenden Spruch ab und verwirft den wartenden, etwa wenn die Runde übersprungen wird.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct SilenceRadio;

/// Ein gesendeter Spruch; `duration` in Sekunden, Sprechtaste eingeschlossen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transmission {
    pub cue: RadioCue,
    pub variant: usize,
    pub duration: f32,
}

/// Meldet jeden Spruch, sobald er gesendet wird, und `None`, wenn der Funk abbricht.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct RadioTransmission(pub Option<Transmission>);

/// Die Funkstrecke auf der Spur des Bordfunks: Alles, was gesendet wird, klingt wie über ein Funkgerät.
pub(super) fn transmitter() -> TrackBuilder {
    TrackBuilder::new()
        .with_effect(
            FilterBuilder::new()
                .mode(FilterMode::HighPass)
                .cutoff(320.0),
        )
        .with_effect(
            FilterBuilder::new()
                .mode(FilterMode::LowPass)
                .cutoff(3600.0),
        )
        .with_effect(EqFilterBuilder::new(
            EqFilterKind::Bell,
            2000.0,
            Decibels(5.0),
            0.9,
        ))
        .with_effect(
            DistortionBuilder::new()
                .kind(DistortionKind::SoftClip)
                .drive(Decibels(4.0)),
        )
}

struct OnAir {
    voice: StaticSoundHandle,
    carrier: StaticSoundHandle,
    /// Ab hier (echte Zeit in Sekunden) ist der Funk wieder frei.
    free_at: f64,
}

struct Waiting {
    say: Say,
    since: f64,
}

#[derive(Resource, Default)]
struct Radio {
    on_air: Option<OnAir>,
    waiting: Option<Waiting>,
    last_said: HashMap<RadioCue, f64>,
    last_variant: HashMap<RadioCue, usize>,
}

pub(super) struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Radio>()
            .add_message::<Say>()
            .add_message::<SilenceRadio>()
            .add_message::<RadioTransmission>()
            .add_systems(Update, run_radio.run_if(resource_exists::<Mixer>));
    }
}

/// Frühestens nach so vielen Sekunden derselbe Spruch noch einmal.
fn repeat_after(cue: RadioCue) -> f64 {
    match cue.priority() {
        1 => 60.0,
        2 => 15.0,
        _ => 0.0,
    }
}

fn run_radio(
    mut says: MessageReader<Say>,
    mut silence: MessageReader<SilenceRadio>,
    mut transmissions: MessageWriter<RadioTransmission>,
    mut radio: ResMut<Radio>,
    mut mixer: ResMut<Mixer>,
    library: Res<AudioLibrary>,
    samples: Res<Assets<Sample>>,
    settings: Res<Settings>,
    clock: Res<AnimationClock>,
    time: Res<Time<Real>>,
) {
    let now = time.elapsed_secs_f64();
    if silence.read().count() > 0 {
        radio.waiting = None;
        if let Some(mut on_air) = radio.on_air.take() {
            on_air.voice.stop(tween(0.1));
            on_air.carrier.stop(tween(0.1));
            mixer.duck_music(false);
            transmissions.write(RadioTransmission(None));
        }
    }
    let audible = settings.voice_volume > 0.0 && !clock.skip;
    for say in says.read() {
        if !audible {
            continue;
        }
        let last = radio.last_said.get(&say.cue).copied();
        if say.variant.is_none() && last.is_some_and(|last| now - last < repeat_after(say.cue)) {
            continue;
        }
        if radio.on_air.is_none() {
            radio.transmit(
                *say,
                now,
                &mut mixer,
                &library,
                &samples,
                &mut transmissions,
            );
        } else if radio
            .waiting
            .as_ref()
            .is_none_or(|waiting| say.cue.priority() > waiting.say.cue.priority())
        {
            radio.waiting = Some(Waiting {
                say: *say,
                since: now,
            });
        }
    }
    // Der Funk ist frei: Ein wartender Spruch kommt dran, solange er noch zum Geschehen passt.
    if radio
        .on_air
        .as_ref()
        .is_some_and(|on_air| now >= on_air.free_at)
    {
        radio.on_air = None;
        match radio.waiting.take() {
            Some(waiting) if audible && now - waiting.since <= MAX_WAIT => {
                radio.transmit(
                    waiting.say,
                    now,
                    &mut mixer,
                    &library,
                    &samples,
                    &mut transmissions,
                );
            }
            _ => mixer.duck_music(false),
        }
    }
}

impl Radio {
    fn transmit(
        &mut self,
        say: Say,
        now: f64,
        mixer: &mut Mixer,
        library: &AudioLibrary,
        samples: &Assets<Sample>,
        transmissions: &mut MessageWriter<RadioTransmission>,
    ) {
        let recordings = &library.radio[&say.cue];
        let variant = say
            .variant
            .unwrap_or_else(|| pick_variant(&mut self.last_variant, say.cue, recordings.len()));
        let Some(sample) = samples.get(&recordings[variant]) else {
            return;
        };
        let end = KEY_UP + sample.duration();
        let voice = sample
            .data
            .start_position(sample.onset)
            .start_time(StartTime::Delayed(Duration::from_secs_f64(KEY_UP)))
            .volume(decibels(sample.gain * VOICE_LEVEL));
        let played = mixer
            .voice
            .play(carrier(end))
            .and_then(|carrier| Ok((carrier, mixer.voice.play(voice)?)));
        let (carrier, voice) = match played {
            Ok(handles) => handles,
            Err(error) => {
                warn!("Funkspruch {:?} nicht gesendet: {error}", say.cue);
                return;
            }
        };
        let duration = end + TAIL;
        self.on_air = Some(OnAir {
            voice,
            carrier,
            free_at: now + duration + GAP,
        });
        self.last_said.insert(say.cue, now);
        mixer.duck_music(true);
        transmissions.write(RadioTransmission(Some(Transmission {
            cue: say.cue,
            variant,
            duration: duration as f32,
        })));
    }
}

/// Träger eines Spruchs, dessen Stimme bei `end` Sekunden endet: Rauschstoß beim Drücken, leises Rauschen unter der
/// Stimme, Rauschstoß beim Loslassen und das Piepen der Sprechtaste.
fn carrier(end: f64) -> StaticSoundData {
    let end = end as f32;
    let tail = TAIL as f32;
    let length = ((end + tail) * CARRIER_RATE as f32) as usize;
    let mut rng = rand::rng();
    let frames: Vec<Frame> = (0..length)
        .map(|i| {
            let t = i as f32 / CARRIER_RATE as f32;
            let hiss = if t < 0.07 {
                0.25 * (HISS_LEVEL / 0.25).powf(t / 0.07)
            } else if t < end {
                HISS_LEVEL
            } else if t < end + 0.02 {
                HISS_LEVEL + (0.22 - HISS_LEVEL) * (t - end) / 0.02
            } else {
                0.22 * (0.0001_f32 / 0.22).powf((t - end - 0.02) / (tail - 0.02))
            };
            let beep = match t {
                t if t < 0.03 => 0.0,
                t if t < 0.035 => 0.12 * (t - 0.03) / 0.005,
                t if t < 0.085 => 0.12,
                t if t < 0.09 => 0.12 * (0.09 - t) / 0.005,
                _ => 0.0,
            };
            let value = hiss * rng.random_range(-1.0..1.0)
                + beep * (std::f32::consts::TAU * 1500.0 * t).sin();
            Frame::from_mono(value)
        })
        .collect();
    StaticSoundData {
        sample_rate: CARRIER_RATE,
        frames: frames.into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_spans_key_up_to_release() {
        let data = carrier(1.0);
        let seconds = data.duration().as_secs_f64();
        assert!((seconds - (1.0 + TAIL)).abs() < 1e-3, "{seconds}");
        let peak = data
            .frames
            .iter()
            .fold(0.0_f32, |max, frame| max.max(frame.left.abs()));
        assert!(peak <= 0.37, "{peak}");
    }

    #[test]
    fn repeats_by_priority() {
        assert_eq!(repeat_after(RadioCue::OrdersSent), 60.0);
        assert_eq!(repeat_after(RadioCue::EnemyReady), 15.0);
        assert_eq!(repeat_after(RadioCue::ShipLost), 0.0);
    }
}
