//! Asset-Typen des Tons: [`Sample`] (Effekte und Funksprüche, ganz dekodiert, auf gleiche Lautheit gebracht) und
//! [`MusicTrack`] (Musik, komprimiert im Speicher und beim Abspielen gestreamt). Beide kommen aus MP3-Dateien; welcher
//! Loader gilt, entscheidet der Typ des Handles.

use std::{io::Cursor, sync::Arc};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use kira::sound::static_sound::StaticSoundData;

/// Ziel der Normalisierung: Effektivwert der lautesten 50 ms.
const TARGET_LOUDNESS: f32 = 0.25;
const LOUDNESS_WINDOW: f32 = 0.05;
/// Ab diesem Pegel beginnt der Sound; ein paar Millisekunden davor setzt das Abspielen ein.
const ONSET_LEVEL: f32 = 0.01;
const ONSET_MARGIN: f64 = 0.005;

/// Ein dekodierter Sound samt Verstärkung auf die gemeinsame Lautheit und stillem Vorlauf.
#[derive(Asset, TypePath)]
pub struct Sample {
    pub data: StaticSoundData,
    /// Verstärkung (Amplitude) auf die gemeinsame Lautheit, ohne dass die Spitze übersteuert.
    pub gain: f32,
    /// Stiller Vorlauf in Sekunden; abgespielt wird ab hier, damit der Ton genau mit dem Bild einsetzt.
    pub onset: f64,
}

impl Sample {
    /// Länge ab dem Einsatz in Sekunden.
    pub fn duration(&self) -> f64 {
        self.data.duration().as_secs_f64() - self.onset
    }
}

/// Ein Musikstück, komprimiert im Speicher; [`kira::sound::streaming::StreamingSoundData`] dekodiert es beim Abspielen.
#[derive(Asset, TypePath)]
pub struct MusicTrack {
    pub bytes: Arc<[u8]>,
}

#[derive(Default, TypePath)]
pub(super) struct SampleLoader;

impl AssetLoader for SampleLoader {
    type Asset = Sample;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Sample, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let data = StaticSoundData::from_cursor(Cursor::new(bytes))?;
        let (gain, onset) = analyse(&data);
        Ok(Sample { data, gain, onset })
    }

    fn extensions(&self) -> &[&str] {
        &["mp3"]
    }
}

#[derive(Default, TypePath)]
pub(super) struct MusicLoader;

impl AssetLoader for MusicLoader {
    type Asset = MusicTrack;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<MusicTrack, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(MusicTrack {
            bytes: bytes.into(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["mp3"]
    }
}

/// Verstärkung auf [`TARGET_LOUDNESS`] (ohne dass die Spitze übersteuert) und Länge des stillen Vorlaufs.
fn analyse(data: &StaticSoundData) -> (f32, f64) {
    let frames = &data.frames;
    let window = ((LOUDNESS_WINDOW * data.sample_rate as f32).round() as usize)
        .clamp(1, frames.len().max(1));
    let power: Vec<f32> = frames
        .iter()
        .map(|frame| (frame.left * frame.left + frame.right * frame.right) / 2.0)
        .collect();
    let mut sum = 0.0_f64;
    let mut loudest = 0.0_f64;
    for (i, value) in power.iter().enumerate() {
        sum += f64::from(*value)
            - if i >= window {
                f64::from(power[i - window])
            } else {
                0.0
            };
        loudest = loudest.max(sum);
    }
    let level = |i: usize| frames[i].left.abs().max(frames[i].right.abs());
    let peak = (0..frames.len()).map(level).fold(0.0_f32, f32::max);
    let rms = (loudest / window as f64).sqrt() as f32;
    let first = (0..frames.len())
        .find(|&i| level(i) > ONSET_LEVEL)
        .unwrap_or(0);
    let gain = if rms > 0.0 {
        (TARGET_LOUDNESS / rms).min(1.0 / peak)
    } else {
        0.0
    };
    let onset = (first as f64 / f64::from(data.sample_rate) - ONSET_MARGIN).max(0.0);
    (gain, onset)
}

#[cfg(test)]
mod tests {
    use kira::{Frame, sound::static_sound::StaticSoundSettings};

    use super::*;

    /// Ein Sinus mit Amplitude 0,5 nach 0,1 s Stille: Effektivwert 0,354, also Verstärkung 0,25 / 0,354; der Einsatz
    /// liegt kurz vor 0,1 s.
    #[test]
    fn normalises_and_finds_onset() {
        let rate = 1000;
        let frames: Vec<Frame> = (0..1000)
            .map(|i| {
                let value = if i < 100 {
                    0.0
                } else {
                    0.5 * (i as f32 * 0.3).sin()
                };
                Frame::from_mono(value)
            })
            .collect();
        let data = StaticSoundData {
            sample_rate: rate,
            frames: frames.into(),
            settings: StaticSoundSettings::default(),
            slice: None,
        };
        let (gain, onset) = analyse(&data);
        assert!(
            (gain - 0.25 / (0.5 / 2.0_f32.sqrt())).abs() < 0.02,
            "{gain}"
        );
        assert!((0.09..0.1).contains(&onset), "{onset}");
    }
}
