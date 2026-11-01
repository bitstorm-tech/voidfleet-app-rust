//! Rundenwiedergabe (Port von `roundPlayback.ts` und `radioCues.ts`): spielt die aufgedeckten Ereignisse einer Runde
//! auf dem Brett ab — Deploy, Admiralsfähigkeiten, Zielerfassung, Manöver, Befehle, Kampf und Cleanup nacheinander,
//! gleichartige Ereignisse jeweils gleichzeitig ([`steps`]), mit Bannern, Log, Schwebetexten, Sounds und Bordfunk
//! ([`radio_cues`]).
//!
//! Vertrag mit dem Controller: Er hält beim [`crate::game::RoundResolved`] die Warteschlange des Servers an, schaltet
//! auf [`crate::scene::sync::BoardMode::Playback`] und sendet [`PlayRound`]. Die Wiedergabe meldet am Ende
//! [`PlaybackFinished`]; dann stehen alle überlebenden Schiffe auf ihren neuen Slots ([`OnSlot`]), damit der Abgleich
//! sie stehen lässt, und der Controller gibt die Warteschlange frei und schaltet zurück auf die Planung, sobald der
//! neue Zustand da ist.
//! [`SkipPlayback`] bricht ab: Animationen springen ans Ende, Sounds, Funk, Banner und Schwebetexte verstummen, die
//! restlichen Schritte landen nur noch im Log.
//!
//! Ablauf als Daten: [`Playback`] hält die Schritte; ist keiner mehr im Gang, übersetzt [`drive`] den nächsten mit
//! dem [`plan::Planner`] in ein [`script::Script`] aus Takten, das [`script::run_scripts`] Takt für Takt abspielt,
//! jeweils sobald die Animationen des vorigen fertig sind. Die Szene läuft dabei dem Spielzustand voraus; den Endzustand
//! übernimmt danach der Server-Snapshot — auch nach dem Überspringen.

mod plan;
pub mod radio_cues;
mod script;
pub mod steps;

use std::collections::VecDeque;

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::{board::SlotRef, protocol::RoundEvent};

use self::{
    plan::Planner,
    script::{Script, run_scripts},
    steps::group_steps,
};
use crate::{
    audio::{SilenceRadio, StopSounds},
    i18n::Locale,
    scene::{
        animation::AnimationSystems,
        animation::{AnimationClock, AnimationScope},
        board::OnSlot,
        camera::BoardLayout,
        floating_text::FloatingText,
        ship::{BOARD_SCALE, Ship},
    },
    ui::{AppendLog, HideBanner, round_log::describe_event},
};

/// Spielt die Ereignisse der Runde `round` ab (aus eigener Sicht, in Auflösungsreihenfolge).
#[derive(Message, Clone, Debug)]
pub struct PlayRound {
    pub round: u32,
    pub events: Vec<RoundEvent>,
}

/// Die Runde `round` ist abgespielt (oder übersprungen); die Schiffe stehen auf ihren neuen Slots.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlaybackFinished {
    pub round: u32,
}

/// Bricht die laufende Wiedergabe ab.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct SkipPlayback;

/// Die laufende Wiedergabe; nur vorhanden, solange eine Runde abgespielt wird.
#[derive(Resource, Debug)]
pub struct Playback {
    round: u32,
    /// Die Schritte, die noch nicht begonnen haben.
    steps: VecDeque<Vec<RoundEvent>>,
    skipped: bool,
    /// Schiffe im Kurzsprung nach ihrem Slot: Sie haben das Brett verlassen und kehren im Cleanup zurück.
    jumped: HashMap<SlotRef, Entity>,
}

pub struct PlaybackPlugin;

impl Plugin for PlaybackPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayRound>()
            .add_message::<PlaybackFinished>()
            .add_message::<SkipPlayback>()
            .add_systems(
                Update,
                (
                    start,
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
    mut rounds: MessageReader<PlayRound>,
    locale: Res<Locale>,
    mut clock: ResMut<AnimationClock>,
    mut logs: MessageWriter<AppendLog>,
) {
    for PlayRound { round, events } in rounds.read() {
        logs.write(AppendLog(vec![locale.log_round_header(*round)]));
        clock.skip = false;
        commands.insert_resource(Playback {
            round: *round,
            steps: group_steps(events).into(),
            skipped: false,
            jumped: HashMap::default(),
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

/// Beginnt den nächsten Schritt, sobald keiner mehr im Gang ist; übersprungen kommen die übrigen nur ins Log. Am Ende
/// kehren Schiffe aus dem Kurzsprung, die noch fort sind, auf ihren Slot zurück.
fn drive(
    mut commands: Commands,
    mut playback: ResMut<Playback>,
    scripts: Query<(), With<Script>>,
    ships: Query<(&Ship, &OnSlot)>,
    layout: Res<BoardLayout>,
    locale: Res<Locale>,
    mut clock: ResMut<AnimationClock>,
    mut logs: MessageWriter<AppendLog>,
    mut finished: MessageWriter<PlaybackFinished>,
) {
    if !scripts.is_empty() {
        return;
    }
    if playback.skipped {
        let lines = playback
            .steps
            .drain(..)
            .flatten()
            .map(|event| describe_event(&**locale, &event))
            .collect::<Vec<_>>();
        if !lines.is_empty() {
            logs.write(AppendLog(lines));
        }
    }
    if let Some(step) = playback.steps.pop_front() {
        let occupied = ships
            .iter()
            .filter(|(ship, _)| !ship.ghost)
            .map(|(ship, on)| (on.0, ship.id))
            .collect();
        let planner = Planner {
            layout: &layout,
            locale: &**locale,
            ships: &occupied,
        };
        commands.spawn((Name::new("Rundenwiedergabe"), planner.step(&step)));
        return;
    }
    for (slot, ship) in playback.jumped.drain() {
        commands.entity(ship).insert((
            OnSlot(slot),
            Visibility::Inherited,
            Transform::from_scale(Vec3::splat(BOARD_SCALE)),
        ));
    }
    clock.scope = AnimationScope::General;
    clock.skip = false;
    finished.write(PlaybackFinished {
        round: playback.round,
    });
    commands.remove_resource::<Playback>();
}
