//! Teilt die Ereignisse einer Aktion in Schritte. Ein Schritt beginnt mit einem Kopf (Karte, Station, Aktion einer
//! Einheit, Rundenbeginn, Durchbruch …) und nimmt alle Folgen bis zum nächsten Kopf mit (Treffer, Brände, Zerstörung,
//! Nachrücker, Absichten). Gleichartiges ohne Entscheidungswert läuft zusammen: Verstärkung, die gemeinsam eintrifft, Einheiten,
//! die nacheinander ausweichen, aufladen oder Schilde heben, aussetzende Einheiten und die Brände der Auflösung.

use std::mem::{Discriminant, discriminant};

use voidfleet_core::{enemies::Action, events::BattleEvent};

/// Wie ein Ereignis einen Schritt beginnt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Head {
    /// Immer ein eigener Schritt.
    Alone,
    /// Läuft mit unmittelbar vorangehenden Köpfen derselben Art zusammen.
    Bundle(Discriminant<BattleEvent>, Option<Discriminant<Action>>),
}

/// Der Kopf eines Ereignisses; `None` für Folgen. `previous` unterscheidet Nachrücker (eine Folge der Zerstörung) von
/// Verstärkung.
fn head(event: &BattleEvent, previous: Option<&BattleEvent>) -> Option<Head> {
    let bundle = |action: Option<Action>| {
        Some(Head::Bundle(
            discriminant(event),
            action.map(|action| discriminant(&action)),
        ))
    };
    match event {
        BattleEvent::CardPlayed { .. }
        | BattleEvent::StationTriggered { .. }
        | BattleEvent::SelfDamage { .. }
        | BattleEvent::Returned
        | BattleEvent::RoundStarted { .. }
        | BattleEvent::Breach
        | BattleEvent::BattleEnded { .. } => Some(Head::Alone),
        BattleEvent::EnemyAction { action, .. } => match action {
            Action::Evade | Action::Charge | Action::Shields(_) => bundle(Some(*action)),
            _ => Some(Head::Alone),
        },
        BattleEvent::Idle { .. }
        | BattleEvent::BurnTick { .. }
        | BattleEvent::Regenerated { .. } => bundle(None),
        BattleEvent::Arrived { .. } => match previous {
            Some(
                BattleEvent::Destroyed { .. }
                | BattleEvent::Cannibalized { .. }
                | BattleEvent::Emergency { .. }
                | BattleEvent::Breach,
            ) => None,
            _ => bundle(None),
        },
        _ => None,
    }
}

/// Ob ein Ereignis eine neue Kette innerhalb eines gebündelten Schritts beginnt (jeder Kopf, auch gebündelt).
pub fn starts_chain(event: &BattleEvent, previous: Option<&BattleEvent>) -> bool {
    head(event, previous).is_some()
}

/// Teilt die Ereignisse in Schritte.
pub fn group_steps(events: &[BattleEvent]) -> Vec<Vec<BattleEvent>> {
    let mut steps: Vec<(Option<Head>, Vec<BattleEvent>)> = Vec::new();
    let mut previous = None;
    for event in events {
        match (head(event, previous), steps.last_mut()) {
            (None, Some((_, step))) => step.push(event.clone()),
            (Some(Head::Bundle(..)), Some((current, step)))
                if *current == head(event, previous) =>
            {
                step.push(event.clone())
            }
            (key, _) => steps.push((key, vec![event.clone()])),
        }
        previous = Some(event);
    }
    steps.into_iter().map(|(_, step)| step).collect()
}

/// Teilt einen Schritt in seine Ketten: je Kopf eine (ein gebündelter Schritt hat mehrere).
pub fn chains(step: &[BattleEvent]) -> Vec<&[BattleEvent]> {
    let mut starts: Vec<usize> = (0..step.len())
        .filter(|&i| i == 0 || starts_chain(&step[i], Some(&step[i - 1])))
        .collect();
    starts.push(step.len());
    starts.windows(2).map(|w| &step[w[0]..w[1]]).collect()
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        admirals::Station,
        board::{EnemySlot, Row},
        enemies::EnemyId,
        events::{Breakdown, DestroyCause, UnitRef, UnitStats},
    };

    use super::*;

    fn unit(uid: u32) -> UnitRef {
        UnitRef::Enemy {
            uid,
            id: EnemyId::Raiders,
            slot: EnemySlot::new(Row::Front, uid as usize % 3),
        }
    }

    fn act(uid: u32, action: Action) -> BattleEvent {
        BattleEvent::EnemyAction {
            unit: unit(uid),
            action,
        }
    }

    const STATS: UnitStats = UnitStats { hull: 4, shield: 0 };

    #[test]
    fn bundles_alike_and_keeps_consequences() {
        let hit = BattleEvent::Hit {
            source: UnitRef::Flagship,
            target: unit(1),
            breakdown: Breakdown::default(),
            after: STATS,
        };
        let events = vec![
            BattleEvent::StationTriggered {
                station: Station::Weapons,
            },
            hit.clone(),
            BattleEvent::Destroyed {
                target: unit(1),
                cause: DestroyCause::Shot,
            },
            // Nachrücker: Folge der Zerstörung.
            BattleEvent::Arrived { unit: unit(4) },
            // Verstärkung im Gegnerzug: zwei zusammen.
            BattleEvent::Arrived { unit: unit(5) },
            BattleEvent::Arrived { unit: unit(6) },
            act(2, Action::Evade),
            act(3, Action::Evade),
            act(4, Action::Fire),
            hit,
            act(5, Action::Evade),
            BattleEvent::BurnTick {
                target: unit(2),
                amount: 2,
                after: STATS,
            },
            BattleEvent::BurnTick {
                target: unit(3),
                amount: 2,
                after: STATS,
            },
            BattleEvent::RoundStarted { round: 2 },
            BattleEvent::Drawn { cards: Vec::new() },
        ];
        let lengths: Vec<usize> = group_steps(&events).iter().map(Vec::len).collect();
        assert_eq!(lengths, [4, 2, 2, 2, 1, 2, 2]);
        let steps = group_steps(&events);
        assert_eq!(chains(&steps[1]).len(), 2);
        assert_eq!(chains(&steps[0]).len(), 1);
    }

    #[test]
    fn different_actions_stay_apart() {
        let events = [
            act(1, Action::Evade),
            act(2, Action::Charge),
            act(3, Action::Fire),
            act(4, Action::Fire),
        ];
        assert_eq!(group_steps(&events).len(), 4);
    }
}
