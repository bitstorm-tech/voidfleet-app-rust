//! Wiedergabe der Ereignisse einer Aktion. Vorerst minimal: Sie schreibt je Ereignis eine Log-Zeile, kündigt eine neue
//! Runde an, lässt den Bordfunk melden (Abschuss, Hülle kritisch, Sieg, Niederlage) und meldet sofort
//! [`PlaybackFinished`]. Die erzählte
//! Wiedergabe mit Kamera, Salven und Bannern (Entwurf Abschnitt 9) baut auf demselben Vertrag auf.
//!
//! Vertrag mit dem Controller: Nach jeder Aktion mit Ereignissen schaltet er die Szene auf
//! [`crate::scene::sync::BoardMode::Playback`] und sendet [`PlayEvents`]; die Szene zeigt dann noch den Stand von vor
//! der Aktion. Am Ende meldet die Wiedergabe [`PlaybackFinished`], der Controller schaltet zurück auf die Planung, und
//! [`crate::scene::sync`] gleicht die Szene dem neuen Zustand an. [`SkipPlayback`] bricht eine laufende Wiedergabe ab.

use bevy::prelude::*;
use voidfleet_core::{
    battle::BattleResult,
    events::{BattleEvent, UnitRef},
};

use crate::{
    audio::{RadioCue, Say},
    game::Battle,
    i18n::Locale,
    ui::{AppendLog, ShowRoundBanner, round_log::describe_event},
};

/// Spielt die Ereignisse einer Aktion ab (in Ausführungsreihenfolge).
#[derive(Message, Clone, Debug)]
pub struct PlayEvents(pub Vec<BattleEvent>);

/// Die Ereignisse sind abgespielt (oder übersprungen).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct PlaybackFinished;

/// Bricht die laufende Wiedergabe ab.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct SkipPlayback;

pub struct PlaybackPlugin;

impl Plugin for PlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayEvents>()
            .add_message::<PlaybackFinished>()
            .add_message::<SkipPlayback>()
            .add_systems(Update, play.run_if(on_message::<PlayEvents>));
    }
}

/// Der Funkspruch zu einem Ereignis.
fn radio_cue(event: &BattleEvent) -> Option<RadioCue> {
    match event {
        BattleEvent::Destroyed {
            target: UnitRef::Enemy { .. },
            ..
        } => Some(RadioCue::EnemyDestroyed),
        BattleEvent::Emergency {
            target: UnitRef::Flagship,
            active: true,
        } => Some(RadioCue::HullCritical),
        BattleEvent::BattleEnded { result } => Some(match result {
            BattleResult::Won => RadioCue::Victory,
            BattleResult::Lost => RadioCue::Defeat,
        }),
        _ => None,
    }
}

fn play(
    mut plays: MessageReader<PlayEvents>,
    battle: Option<Res<Battle>>,
    locale: Res<Locale>,
    mut logs: MessageWriter<AppendLog>,
    mut say: MessageWriter<Say>,
    mut round_banner: MessageWriter<ShowRoundBanner>,
    mut finished: MessageWriter<PlaybackFinished>,
) {
    let Some(battle) = battle else {
        plays.clear();
        return;
    };
    let admiral = battle.flagship.admiral;
    for PlayEvents(events) in plays.read() {
        let lines = events
            .iter()
            .map(|event| describe_event(&**locale, event, admiral))
            .collect();
        logs.write(AppendLog(lines));
        for event in events {
            if let BattleEvent::RoundStarted { round } = event {
                round_banner.write(ShowRoundBanner(*round));
            }
        }
        // Der wichtigste Spruch zuletzt: Der Funk lässt Unwichtigeres warten.
        let mut cues: Vec<RadioCue> = events.iter().filter_map(radio_cue).collect();
        cues.sort_by_key(|cue| cue.priority());
        cues.dedup();
        if let Some(&cue) = cues.last() {
            say.write(Say::new(cue));
        }
        finished.write(PlaybackFinished);
    }
}
