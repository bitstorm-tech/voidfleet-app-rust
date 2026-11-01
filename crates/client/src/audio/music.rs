//! Hintergrundmusik in zwei Stimmungen (Port von `audio/music.ts`): ruhig, solange geplant wird, angespannt, während
//! eine Runde abgespielt wird. Wer die Stimmung wechseln will, schreibt die Ressource [`Music`]; ein Wechsel blendet
//! über, und jede Stimmung spielt dort weiter, wo sie zuletzt aufgehört hat. Beim ersten Mal erklingt ein Stück mit
//! seinem Einklang; vor dem Ausklang blendet es in den Anfang seines Kernteils über, so läuft es ohne Lücke endlos.
//! Die Stücke liegen komprimiert im Speicher und werden beim Abspielen gestreamt.

use std::io::Cursor;

use bevy::{platform::collections::HashMap, prelude::*};
use kira::sound::{
    FromFileError,
    streaming::{StreamingSoundData, StreamingSoundHandle},
};

use super::{AudioLibrary, Mixer, MusicMood, MusicTrack, tween};

/// Überblendung in Sekunden: beim Wechsel der Stimmung und am Ende des Kernteils.
const MOOD_FADE: f32 = 2.5;
const LOOP_FADE: f32 = 4.0;

/// Gewünschte Stimmung der Musik; `None` blendet sie aus.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Music(pub Option<MusicMood>);

struct Playing {
    mood: MusicMood,
    handle: StreamingSoundHandle<FromFileError>,
}

#[derive(Resource, Default)]
struct MusicPlayer {
    current: Option<Playing>,
    /// Wo jede Stimmung zuletzt aufgehört hat (Sekunden).
    resume_at: HashMap<MusicMood, f64>,
}

impl MusicPlayer {
    /// Blendet das laufende Stück aus und `mood` ab `from` ein, ohne Angabe dort, wo die Stimmung aufgehört hat.
    fn crossfade(
        &mut self,
        mixer: &mut Mixer,
        library: &AudioLibrary,
        tracks: &Assets<MusicTrack>,
        mood: Option<MusicMood>,
        fade: f32,
        from: Option<f64>,
    ) {
        if let Some(mut playing) = self.current.take() {
            self.resume_at
                .insert(playing.mood, playing.handle.position());
            playing.handle.stop(tween(fade));
        }
        let Some(mood) = mood else {
            return;
        };
        let Some(track) = tracks.get(&library.music[&mood]) else {
            return;
        };
        let from = from.unwrap_or_else(|| self.resume_at.get(&mood).copied().unwrap_or(0.0));
        let started = StreamingSoundData::from_cursor(Cursor::new(track.bytes.clone()))
            .map(|data| data.start_position(from).fade_in_tween(tween(fade)))
            .map_err(BevyError::from)
            .and_then(|data| Ok(mixer.music_duck.play(data)?));
        match started {
            Ok(handle) => self.current = Some(Playing { mood, handle }),
            Err(error) => warn!("Musik {mood:?} nicht abgespielt: {error}"),
        }
    }
}

pub(super) struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Music>()
            .init_resource::<MusicPlayer>()
            .add_systems(
                Update,
                (change_mood.run_if(resource_changed::<Music>), loop_music)
                    .chain()
                    .run_if(resource_exists::<Mixer>),
            );
    }
}

/// Blendet in die gewünschte Stimmung über, wenn sie sich geändert hat und eine andere ist als die laufende.
fn change_mood(
    music: Res<Music>,
    mut player: ResMut<MusicPlayer>,
    mut mixer: ResMut<Mixer>,
    library: Res<AudioLibrary>,
    tracks: Res<Assets<MusicTrack>>,
) {
    if music.0 != player.current.as_ref().map(|playing| playing.mood) {
        player.crossfade(&mut mixer, &library, &tracks, music.0, MOOD_FADE, None);
    }
}

/// Blendet vor dem Ausklang in den Anfang des Kernteils über.
fn loop_music(
    mut player: ResMut<MusicPlayer>,
    mut mixer: ResMut<Mixer>,
    library: Res<AudioLibrary>,
    tracks: Res<Assets<MusicTrack>>,
) {
    let Some(playing) = &player.current else {
        return;
    };
    let mood = playing.mood;
    let (loop_start, loop_end) = mood.core();
    if playing.handle.position() > loop_end - f64::from(LOOP_FADE) {
        player.crossfade(
            &mut mixer,
            &library,
            &tracks,
            Some(mood),
            LOOP_FADE,
            Some(loop_start),
        );
    }
}
