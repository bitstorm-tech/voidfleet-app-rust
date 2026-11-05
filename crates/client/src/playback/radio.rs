//! Welcher Funkspruch zu welchem Ereignis gehört, an einer Stelle; neue Aufnahmen kommen hier dazu. Gesprochen wird
//! er im Takt des Ereignisses; die Warteschlange des Bordfunks ([`crate::audio::radio`]) ordnet nach Priorität.

use voidfleet_core::{
    battle::BattleResult,
    events::{BattleEvent, UnitRef},
};

use crate::audio::RadioCue;

/// Der Funkspruch zu einem Ereignis.
pub fn radio_cue(event: &BattleEvent) -> Option<RadioCue> {
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
