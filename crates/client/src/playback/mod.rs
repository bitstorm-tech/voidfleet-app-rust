//! Die erzählte Wiedergabe des Gefechts (`docs/inszenierung.md`): Jede Aktion läuft sichtbar ab,
//! bevor die Szene den neuen Zustand übernimmt. Der Spielerzug zeigt Schuss, Einschlag und Zahl und gibt sofort zurück;
//! `end_turn` erzählt Zugende, Gegnerzug und Auflösung Ereignis für Ereignis.
//!
//! - [`steps`]: Ereignisse → Schritte (Kopf und Folgen, Gleichartiges gebündelt).
//! - [`plan`]: Schritt → [`script::Script`] aus Takten.
//! - [`script`]: Takte als Daten und ihre Bühne; je Bild höchstens ein Takt, erst wenn die Aufträge des vorigen fertig
//!   sind.
//! - [`radio`]: welcher Funkspruch zu welchem Ereignis gehört.
//!
//! Vertrag mit dem Controller: Nach jeder Aktion mit Ereignissen schaltet er die Szene auf
//! [`crate::scene::sync::BoardMode::Playback`] und sendet [`PlayEvents`]; die Szene zeigt dann noch den Stand von vor
//! der Aktion. Am Ende meldet die Wiedergabe [`PlaybackFinished`], der Controller schaltet zurück auf die Planung, und
//! [`crate::scene::sync`] gleicht die Szene dem neuen Zustand an (auch nach dem Überspringen). [`SkipPlayback`] springt
//! ans Ende: Animationen enden sofort, Effekte, Banner, Schwebetexte und Funk verstummen, die übrigen Schritte landen
//! nur noch im Log.

mod plan;
mod radio;
mod script;
mod steps;

use std::collections::VecDeque;

use bevy::prelude::*;
use voidfleet_core::{admirals::AdmiralId, events::BattleEvent};

pub use self::radio::radio_cue;
use self::{
    plan::Planner,
    script::{Script, run_scripts},
    steps::group_steps,
};
use crate::{
    audio::{SilenceRadio, StopSounds},
    game::Battle,
    i18n::{Locale, Messages},
    scene::{
        animation::{AnimationClock, AnimationScope, AnimationSystems},
        camera::CameraCommand,
        floating_text::FloatingText,
    },
    ui::{AppendLog, HideBanner, round_log::describe_event},
};

/// Spielt die Ereignisse einer Aktion ab (in Ausführungsreihenfolge).
#[derive(Message, Clone, Debug)]
pub struct PlayEvents(pub Vec<BattleEvent>);

/// Die Ereignisse sind abgespielt (oder übersprungen).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct PlaybackFinished;

/// Springt in der laufenden Wiedergabe ans Ende.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct SkipPlayback;

/// Die laufende Wiedergabe; nur vorhanden, solange abgespielt wird.
#[derive(Resource, Debug)]
pub(crate) struct Playback {
    /// Die Schritte, die noch nicht begonnen haben.
    steps: VecDeque<Vec<BattleEvent>>,
    skipped: bool,
}

impl Playback {
    /// Übersprungen: Die übrigen Schritte kommen nur noch als Log-Zeilen.
    fn skip_rest(&mut self, locale: &dyn Messages, admiral: AdmiralId) -> Vec<String> {
        self.steps
            .drain(..)
            .flatten()
            .map(|event| describe_event(locale, &event, admiral))
            .collect()
    }
}

pub struct PlaybackPlugin;

impl Plugin for PlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayEvents>()
            .add_message::<PlaybackFinished>()
            .add_message::<SkipPlayback>()
            .add_systems(
                Update,
                (
                    start.run_if(on_message::<PlayEvents>),
                    skip.run_if(on_message::<SkipPlayback>)
                        .run_if(resource_exists::<Playback>),
                    (run_scripts, drive)
                        .chain()
                        .run_if(resource_exists::<Playback>),
                )
                    .chain()
                    // Was ein Takt beginnt, läuft noch im selben Bild an.
                    .before(AnimationSystems::Advance),
            );
    }
}

fn start(
    mut commands: Commands,
    mut plays: MessageReader<PlayEvents>,
    mut clock: ResMut<AnimationClock>,
) {
    for PlayEvents(events) in plays.read() {
        clock.skip = false;
        commands.insert_resource(Playback {
            steps: group_steps(events).into(),
            skipped: false,
        });
    }
}

/// Animationen springen ans Ende, Sounds und Funk verstummen, Banner und Schwebetexte verschwinden.
fn skip(
    mut commands: Commands,
    mut playback: ResMut<Playback>,
    mut clock: ResMut<AnimationClock>,
    mut stop: MessageWriter<StopSounds>,
    mut silence: MessageWriter<SilenceRadio>,
    mut banner: MessageWriter<HideBanner>,
) {
    playback.skipped = true;
    clock.skip = true;
    stop.write(StopSounds);
    silence.write(SilenceRadio);
    banner.write(HideBanner);
    commands.despawn_all::<With<FloatingText>>();
}

/// Beginnt den nächsten Schritt, sobald keiner mehr läuft; übersprungen kommen die übrigen nur ins Log. Nach dem letzten
/// ist die Wiedergabe fertig.
fn drive(
    mut commands: Commands,
    mut playback: ResMut<Playback>,
    scripts: Query<(), With<Script>>,
    battle: Option<Res<Battle>>,
    locale: Res<Locale>,
    mut clock: ResMut<AnimationClock>,
    mut logs: MessageWriter<AppendLog>,
    mut camera: MessageWriter<CameraCommand>,
    mut finished: MessageWriter<PlaybackFinished>,
) {
    if !scripts.is_empty() {
        return;
    }
    let Some(admiral) = battle.map(|battle| battle.flagship.admiral) else {
        playback.steps.clear();
        return finish(&mut commands, &mut clock, &mut camera, &mut finished);
    };
    if playback.skipped {
        let lines = playback.skip_rest(&**locale, admiral);
        if !lines.is_empty() {
            logs.write(AppendLog(lines));
        }
    }
    let planner = Planner {
        locale: &**locale,
        admiral,
    };
    if let Some(step) = playback.steps.pop_front() {
        commands.spawn((Name::new("Wiedergabe"), planner.step(&step)));
    } else {
        finish(&mut commands, &mut clock, &mut camera, &mut finished);
    }
}

fn finish(
    commands: &mut Commands,
    clock: &mut AnimationClock,
    camera: &mut MessageWriter<CameraCommand>,
    finished: &mut MessageWriter<PlaybackFinished>,
) {
    clock.scope = AnimationScope::General;
    clock.skip = false;
    camera.write(CameraCommand::Lean(None));
    finished.write(PlaybackFinished);
    commands.remove_resource::<Playback>();
}

#[cfg(test)]
mod tests {
    use voidfleet_core::events::UnitStats;

    use super::*;
    use crate::settings::Language;

    /// Überspringen: Die übrigen Schritte landen vollständig im Log, nichts bleibt offen.
    #[test]
    fn skip_logs_the_rest() {
        let events = vec![
            BattleEvent::SelfDamage {
                amount: 3,
                after: UnitStats { hull: 5, shield: 0 },
            },
            BattleEvent::RoundStarted { round: 3 },
            BattleEvent::Shuffled,
        ];
        let mut playback = Playback {
            steps: group_steps(&events).into(),
            skipped: true,
        };
        let locale = Locale::new(Language::En);
        let lines = playback.skip_rest(&*locale, AdmiralId::Osei);
        assert_eq!(lines.len(), events.len());
        assert!(playback.steps.is_empty());
    }
}
