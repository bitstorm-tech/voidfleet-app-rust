//! Welcher Funkspruch zu welchem Schritt gehört, an einer Stelle; neue Aufnahmen kommen hier dazu. Je Schritt höchstens
//! einer, der wichtigste (bei Gleichstand der erste); gesprochen wird er zu Beginn des Schritts, die Warteschlange des
//! Bordfunks ([`crate::audio::radio`]) ordnet nach Priorität und sperrt Wiederholungen.
//!
//! Die Brücke meldet Gefahr, Brände, Notfall und eigene Manöver, die Geschützleitung Waffen und Treffer, der abgehörte
//! Gegnerfunk Notfall, Aufladen, Verstärkung und den Moment, in dem ein Verband bricht. Zum Durchbruch gehören zwei
//! Sprüche: die Gegenflotte, als das Siegel fällt (hier), und die Brücke, als das Fremde eintrifft (den zweiten sagt die
//! Wiedergabe selbst an, [`super::plan`]). Das Fremde und seine Brut funken nicht: Abgehörter Gegnerfunk schweigt bei
//! ihnen.

use voidfleet_core::{
    admirals::{AdmiralId, Station},
    battle::BattleResult,
    cards::CardId,
    enemies::{Action, EnemyId},
    events::{BattleEvent, UnitRef},
};

use crate::audio::RadioCue;

/// So viel Hüllenschaden macht ein eigener Treffer, damit die Geschützleitung „Volltreffer“ meldet.
const DIRECT_HIT: i32 = 12;

/// Der Funkspruch zu einem Schritt.
pub fn radio_cue(step: &[BattleEvent], admiral: AdmiralId) -> Option<RadioCue> {
    let kills = step
        .iter()
        .filter(|event| {
            matches!(
                event,
                BattleEvent::Destroyed { target, .. } if human(*target)
            )
        })
        .count();
    let breaking = (kills >= 2).then_some(RadioCue::EnemyBreaking);
    breaking
        .into_iter()
        .chain(step.iter().filter_map(|event| event_cue(event, admiral)))
        .reduce(|best, cue| {
            if cue.priority() > best.priority() {
                cue
            } else {
                best
            }
        })
}

/// Eine gegnerische Einheit, deren Funk sich abhören lässt: alle außer dem Fremden und seiner Brut.
fn human(unit: UnitRef) -> bool {
    match unit {
        UnitRef::Enemy { id, .. } => !matches!(id, EnemyId::UnknownContact | EnemyId::Brood),
        UnitRef::Flagship => false,
    }
}

fn event_cue(event: &BattleEvent, admiral: AdmiralId) -> Option<RadioCue> {
    use BattleEvent as E;
    use UnitRef::{Enemy, Flagship};
    Some(match *event {
        E::BattleEnded { result } => match result {
            BattleResult::Won => RadioCue::Victory,
            BattleResult::Lost => RadioCue::Defeat,
        },
        E::Destroyed {
            target: Enemy { .. },
            ..
        } => RadioCue::EnemyDestroyed,
        E::Emergency {
            target: Flagship,
            active: true,
        } => RadioCue::HullCritical,
        E::Emergency {
            target,
            active: true,
        } if human(target) => RadioCue::EnemyPanic,
        E::Breach => RadioCue::SealBroken,
        E::Ignited { target: Flagship } => RadioCue::FireOnboard,
        E::Ignited {
            target: Enemy { .. },
        } => RadioCue::EnemyBurning,
        E::Extinguished { target: Flagship } => RadioCue::FireOut,
        E::EmergencyGenerator { .. } => RadioCue::EmergencyGenerator,
        E::Jumped => RadioCue::MicroJump,
        E::Returned => RadioCue::JumpReturn,
        E::Maneuvered { .. } => RadioCue::Maneuver,
        E::Arrived {
            unit:
                Enemy {
                    id: EnemyId::UnknownContact,
                    ..
                },
        } => RadioCue::UnknownContact,
        E::Arrived { unit } if human(unit) => RadioCue::ReinforcementsSighted,
        E::Counterfire { .. } => RadioCue::Counterfire,
        E::StationTriggered { station } => match station {
            Station::Shields => RadioCue::ShieldsUp,
            Station::DamageControl => RadioCue::Repairs,
            Station::Weapons => match admiral {
                AdmiralId::Kerr => RadioCue::RailgunFire,
                AdmiralId::Reyes => RadioCue::PlasmaFire,
                AdmiralId::Osei | AdmiralId::Vance => RadioCue::TargetLocked,
            },
            Station::Maneuver | Station::Admiral => return None,
        },
        E::CardPlayed { card, .. } => match card.id {
            CardId::Torpedo => RadioCue::TorpedoAway,
            CardId::OrbitalStrike => RadioCue::OrbitalStrike,
            CardId::RepairCrew => RadioCue::Repairs,
            CardId::MicroJump => RadioCue::MicroJump,
            _ => return None,
        },
        E::Hit {
            source: Flagship,
            target: Enemy { .. },
            breakdown,
            ..
        } if breakdown.hull >= DIRECT_HIT => RadioCue::DirectHit,
        E::EnemyAction { unit, action } => match action {
            Action::Fire | Action::ColumnFire { .. } => RadioCue::IncomingFire,
            Action::Charge if human(unit) => RadioCue::EnemyCharging,
            Action::Call(_) if human(unit) => RadioCue::EnemyCallsReinforcements,
            _ => return None,
        },
        // Angesagt für den nächsten Gegnerzug: Schaden am Flaggschiff heißt, es steht in der Schusslinie.
        E::Intents { unit, intents } => {
            intents
                .iter()
                .filter(|intent| intent.damage > 0)
                .find_map(|intent| match intent.action {
                    Action::ColumnFire { .. } => Some(RadioCue::ColumnFireIncoming),
                    // Der Spruch nennt den Brander; die Brut rammt ohne Meldung.
                    Action::Ram if human(unit) => Some(RadioCue::RamIncoming),
                    _ => None,
                })?
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        battle::Target,
        board::{EnemySlot, Row},
        cards::DeckCard,
        enemies::EnemyId,
        events::{Breakdown, DestroyCause, Intent, UnitStats},
    };

    use super::*;

    const ENEMY: UnitRef = UnitRef::Enemy {
        uid: 1,
        id: EnemyId::Raiders,
        slot: EnemySlot::new(Row::Front, 0),
    };
    const CONTACT: UnitRef = UnitRef::Enemy {
        uid: 9,
        id: EnemyId::UnknownContact,
        slot: EnemySlot::new(Row::Front, 1),
    };
    const STATS: UnitStats = UnitStats { hull: 1, shield: 0 };

    fn kill(uid: u32) -> BattleEvent {
        BattleEvent::Destroyed {
            target: UnitRef::Enemy {
                uid,
                id: EnemyId::Raiders,
                slot: EnemySlot::new(Row::Front, uid as usize % 3),
            },
            cause: DestroyCause::Shot,
        }
    }

    fn station(station: Station) -> BattleEvent {
        BattleEvent::StationTriggered { station }
    }

    fn card(id: CardId) -> BattleEvent {
        BattleEvent::CardPlayed {
            card: DeckCard::new(id),
            target: Target::None,
        }
    }

    fn action(action: Action) -> BattleEvent {
        BattleEvent::EnemyAction {
            unit: ENEMY,
            action,
        }
    }

    fn intents(action: Action, damage: i32) -> BattleEvent {
        let intent = Intent { action, damage };
        BattleEvent::Intents {
            unit: ENEMY,
            intents: [intent, intent],
        }
    }

    /// Jeder Funkspruch außer dem Gefechtsbeginn (den sagt der Controller) hat einen Auslöser.
    #[test]
    fn every_cue_has_a_trigger() {
        use BattleEvent as E;
        let column = Action::ColumnFire {
            columns: [true, false, false],
        };
        let triggers: Vec<(Vec<BattleEvent>, AdmiralId, RadioCue)> = vec![
            (
                vec![E::BattleEnded {
                    result: BattleResult::Won,
                }],
                AdmiralId::Osei,
                RadioCue::Victory,
            ),
            (
                vec![E::BattleEnded {
                    result: BattleResult::Lost,
                }],
                AdmiralId::Osei,
                RadioCue::Defeat,
            ),
            (vec![kill(1)], AdmiralId::Osei, RadioCue::EnemyDestroyed),
            (
                vec![kill(1), kill(2)],
                AdmiralId::Osei,
                RadioCue::EnemyBreaking,
            ),
            (
                vec![E::Emergency {
                    target: UnitRef::Flagship,
                    active: true,
                }],
                AdmiralId::Osei,
                RadioCue::HullCritical,
            ),
            (
                vec![E::Emergency {
                    target: ENEMY,
                    active: true,
                }],
                AdmiralId::Osei,
                RadioCue::EnemyPanic,
            ),
            (
                vec![E::Ignited {
                    target: UnitRef::Flagship,
                }],
                AdmiralId::Osei,
                RadioCue::FireOnboard,
            ),
            (
                vec![E::Ignited { target: ENEMY }],
                AdmiralId::Osei,
                RadioCue::EnemyBurning,
            ),
            (
                vec![E::Extinguished {
                    target: UnitRef::Flagship,
                }],
                AdmiralId::Osei,
                RadioCue::FireOut,
            ),
            (
                vec![E::EmergencyGenerator { after: STATS }],
                AdmiralId::Osei,
                RadioCue::EmergencyGenerator,
            ),
            (vec![E::Jumped], AdmiralId::Osei, RadioCue::MicroJump),
            (vec![E::Returned], AdmiralId::Osei, RadioCue::JumpReturn),
            (
                vec![E::Maneuvered { from: 0, to: 1 }],
                AdmiralId::Osei,
                RadioCue::Maneuver,
            ),
            (
                vec![E::Arrived { unit: ENEMY }],
                AdmiralId::Osei,
                RadioCue::ReinforcementsSighted,
            ),
            (
                vec![E::Counterfire {
                    target: ENEMY,
                    amount: 3,
                    after: STATS,
                }],
                AdmiralId::Osei,
                RadioCue::Counterfire,
            ),
            (
                vec![station(Station::Shields)],
                AdmiralId::Osei,
                RadioCue::ShieldsUp,
            ),
            (
                vec![station(Station::DamageControl)],
                AdmiralId::Osei,
                RadioCue::Repairs,
            ),
            (
                vec![station(Station::Weapons)],
                AdmiralId::Vance,
                RadioCue::TargetLocked,
            ),
            (
                vec![station(Station::Weapons)],
                AdmiralId::Kerr,
                RadioCue::RailgunFire,
            ),
            (
                vec![station(Station::Weapons)],
                AdmiralId::Reyes,
                RadioCue::PlasmaFire,
            ),
            (
                vec![card(CardId::Torpedo)],
                AdmiralId::Osei,
                RadioCue::TorpedoAway,
            ),
            (
                vec![card(CardId::OrbitalStrike)],
                AdmiralId::Osei,
                RadioCue::OrbitalStrike,
            ),
            (
                vec![E::Hit {
                    source: UnitRef::Flagship,
                    target: ENEMY,
                    breakdown: Breakdown {
                        hull: DIRECT_HIT,
                        ..Breakdown::default()
                    },
                    after: STATS,
                }],
                AdmiralId::Osei,
                RadioCue::DirectHit,
            ),
            (
                vec![action(Action::Fire)],
                AdmiralId::Osei,
                RadioCue::IncomingFire,
            ),
            (
                vec![action(Action::Charge)],
                AdmiralId::Osei,
                RadioCue::EnemyCharging,
            ),
            (
                vec![action(Action::Call(EnemyId::Raiders))],
                AdmiralId::Osei,
                RadioCue::EnemyCallsReinforcements,
            ),
            (
                vec![intents(column, 6)],
                AdmiralId::Osei,
                RadioCue::ColumnFireIncoming,
            ),
            (
                vec![intents(Action::Ram, 10)],
                AdmiralId::Osei,
                RadioCue::RamIncoming,
            ),
            (vec![E::Breach], AdmiralId::Osei, RadioCue::SealBroken),
            (
                vec![E::Arrived { unit: CONTACT }],
                AdmiralId::Osei,
                RadioCue::UnknownContact,
            ),
        ];
        for (step, admiral, cue) in &triggers {
            assert_eq!(radio_cue(step, *admiral), Some(*cue), "{step:?}");
        }
        for &cue in RadioCue::ALL {
            assert!(
                cue == RadioCue::BattleStart || triggers.iter().any(|(_, _, c)| *c == cue),
                "{cue:?}"
            );
        }
        // Spaltenfeuer auf eine andere Spalte schweigt.
        assert_eq!(radio_cue(&[intents(column, 0)], AdmiralId::Osei), None);
    }

    /// Das Fremde funkt nicht: Lädt es auf, ruft es Brut oder gerät es in den Notfall, bleibt der Gegnerfunk still;
    /// sein Feuer meldet die Brücke wie jedes andere. Im Schritt des Durchbruchs spricht die Gegenflotte.
    #[test]
    fn the_unknown_contact_is_silent() {
        use BattleEvent as E;
        let act = |action| E::EnemyAction {
            unit: CONTACT,
            action,
        };
        let panic = E::Emergency {
            target: CONTACT,
            active: true,
        };
        for event in [
            act(Action::Charge),
            act(Action::Call(EnemyId::Brood)),
            panic,
        ] {
            assert_eq!(
                radio_cue(std::slice::from_ref(&event), AdmiralId::Osei),
                None,
                "{event:?}"
            );
        }
        assert_eq!(
            radio_cue(&[act(Action::Fire)], AdmiralId::Osei),
            Some(RadioCue::IncomingFire)
        );
        let breach = [E::Breach, kill(1), kill(2), E::Arrived { unit: CONTACT }];
        assert_eq!(
            radio_cue(&breach, AdmiralId::Osei),
            Some(RadioCue::SealBroken)
        );
    }

    /// Je Schritt der wichtigste: Der Sieg geht dem Abschuss vor.
    #[test]
    fn picks_the_most_important() {
        let won = BattleEvent::BattleEnded {
            result: BattleResult::Won,
        };
        assert_eq!(
            radio_cue(&[kill(1), won], AdmiralId::Osei),
            Some(RadioCue::Victory)
        );
    }
}
