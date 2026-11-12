//! Beschreibt jedes Ereignis des Gefechts in einer Log-Zeile. Die Zeilen hängt die Wiedergabe mit [`super::AppendLog`]
//! ans Log.

use voidfleet_core::{
    admirals::AdmiralId,
    battle::Target,
    events::{BattleEvent, UnitRef},
};

use crate::i18n::Messages;

/// Eine Einheit im Log: das Flaggschiff beim Namen, ein Gegner mit Slot („Wachkreuzer (Front Mitte)“).
pub fn unit_name(locale: &Messages, unit: UnitRef, admiral: AdmiralId) -> String {
    match unit {
        UnitRef::Flagship => locale.flagship(admiral.flagship()).to_owned(),
        UnitRef::Enemy { id, slot, .. } => {
            format!("{} ({})", locale.enemy(id), locale.slot_name(slot))
        }
    }
}

/// Eine Zeile je Ereignis; `admiral` benennt das Flaggschiff.
pub fn describe_event(locale: &Messages, event: &BattleEvent, admiral: AdmiralId) -> String {
    let unit = |unit: UnitRef| unit_name(locale, unit, admiral);
    match event {
        BattleEvent::RoundStarted { round } => locale.log_round_header(*round),
        BattleEvent::CardPlayed { card, target } => {
            let target = match *target {
                Target::None => None,
                // Das Ziel steht im Treffer danach; hier genügt der Name.
                Target::Enemy(_) => None,
                Target::Column(column) => Some(locale.column_name(column).to_owned()),
            };
            locale.log_card(&locale.card(*card).name, target.as_deref())
        }
        BattleEvent::StationTriggered { station } => {
            locale.log_station(locale.station(*station, admiral))
        }
        BattleEvent::Hit {
            source,
            target,
            breakdown,
            ..
        } => locale.log_hit(&unit(*source), &unit(*target), &locale.breakdown(breakdown)),
        BattleEvent::Missed { source, reason } => locale.log_missed(&unit(*source), *reason),
        BattleEvent::Shielded { target, amount, .. } => {
            locale.log_shielded(&unit(*target), *amount)
        }
        BattleEvent::Healed { target, amount, .. } => locale.log_healed(&unit(*target), *amount),
        BattleEvent::Ignited { target } => locale.log_ignited(&unit(*target)),
        BattleEvent::Extinguished { target } => locale.log_extinguished(&unit(*target)),
        BattleEvent::Emergency { target, active } => locale.log_emergency(&unit(*target), *active),
        BattleEvent::Destroyed { target, cause } => locale.log_destroyed(&unit(*target), *cause),
        BattleEvent::FireshipSpent { unit: spent } => locale.log_fireship_spent(&unit(*spent)),
        BattleEvent::Rammed { target } => locale.log_rammed(&unit(*target)),
        BattleEvent::Jumped => locale.log_jumped().to_owned(),
        BattleEvent::Returned => locale.log_returned().to_owned(),
        BattleEvent::Maneuvered { from, to } => {
            locale.log_maneuvered(locale.column_name(*from), locale.column_name(*to))
        }
        BattleEvent::Tractored { unit: pulled, from } => {
            locale.log_tractored(&unit(*pulled), &locale.slot_name(*from))
        }
        BattleEvent::Arrived { unit: arrived } => locale.log_arrived(&unit(*arrived)),
        BattleEvent::EnemyAction {
            unit: acting,
            action,
        } => locale.log_enemy_action(&unit(*acting), &locale.action(*action)),
        BattleEvent::Idle { unit: idle, reason } => locale.log_idle(&unit(*idle), *reason),
        BattleEvent::Counterfire { target, amount, .. } => {
            locale.log_counterfire(&unit(*target), *amount)
        }
        BattleEvent::HoldTheLine { target, amount, .. } => {
            locale.log_hold_the_line(&unit(*target), *amount)
        }
        BattleEvent::SelfDamage { amount, .. } => locale.log_self_damage(*amount),
        BattleEvent::BurnTick { target, amount, .. } => locale.log_burn(&unit(*target), *amount),
        BattleEvent::Cannibalized {
            unit: king, amount, ..
        } => locale.log_cannibalized(&unit(*king), *amount),
        BattleEvent::EmergencyGenerator { .. } => locale.log_emergency_generator().to_owned(),
        BattleEvent::Shuffled => locale.log_shuffled().to_owned(),
        BattleEvent::Drawn { cards } => locale.log_drawn(cards.len()),
        BattleEvent::Intents {
            unit: announcing,
            intents,
        } => {
            let [first, second] = intents.map(|intent| locale.intent(intent));
            locale.log_intents(&unit(*announcing), &format!("{first} → {second}"))
        }
        BattleEvent::Clock { bonus } => locale.log_clock(*bonus),
        BattleEvent::BattleEnded { result } => locale.log_battle_ended(*result).to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        admirals::Station,
        battle::BattleResult,
        board::{EnemySlot, Row},
        cards::{CardId, DeckCard},
        enemies::{Action, EnemyId},
        events::{Breakdown, DestroyCause, IdleReason, Intent, MissReason, UnitStats},
    };

    use super::*;
    use crate::{
        i18n::{LANGUAGES, Locale},
        settings::Language,
    };

    /// Jede Variante von [`BattleEvent`] einmal.
    fn all_events() -> Vec<BattleEvent> {
        let enemy = UnitRef::Enemy {
            uid: 1,
            id: EnemyId::GuardCruiser,
            slot: EnemySlot::new(Row::Front, 1),
        };
        let stats = UnitStats {
            hull: 10,
            shield: 2,
        };
        let intent = Intent {
            action: Action::Fire,
            damage: 4,
        };
        vec![
            BattleEvent::RoundStarted { round: 2 },
            BattleEvent::CardPlayed {
                card: DeckCard::new(CardId::Torpedo),
                target: Target::Enemy(1),
            },
            BattleEvent::StationTriggered {
                station: Station::Admiral,
            },
            BattleEvent::Hit {
                source: UnitRef::Flagship,
                target: enemy,
                breakdown: Breakdown {
                    base: 7,
                    hull: 7,
                    ..Breakdown::default()
                },
                after: stats,
            },
            BattleEvent::Missed {
                source: enemy,
                reason: MissReason::Maneuvered,
            },
            BattleEvent::Shielded {
                target: enemy,
                amount: 6,
                after: stats,
            },
            BattleEvent::Healed {
                target: UnitRef::Flagship,
                amount: 5,
                after: stats,
            },
            BattleEvent::Ignited { target: enemy },
            BattleEvent::Extinguished {
                target: UnitRef::Flagship,
            },
            BattleEvent::Emergency {
                target: enemy,
                active: true,
            },
            BattleEvent::Destroyed {
                target: enemy,
                cause: DestroyCause::BurnedOut,
            },
            BattleEvent::FireshipSpent { unit: enemy },
            BattleEvent::Rammed { target: enemy },
            BattleEvent::Jumped,
            BattleEvent::Returned,
            BattleEvent::Maneuvered { from: 1, to: 2 },
            BattleEvent::Tractored {
                unit: enemy,
                from: EnemySlot::new(Row::Back, 0),
            },
            BattleEvent::Arrived { unit: enemy },
            BattleEvent::EnemyAction {
                unit: enemy,
                action: Action::ColumnFire {
                    columns: [true, false, true],
                },
            },
            BattleEvent::Idle {
                unit: enemy,
                reason: IdleReason::Stasis,
            },
            BattleEvent::Counterfire {
                target: enemy,
                amount: 3,
                after: stats,
            },
            BattleEvent::HoldTheLine {
                target: enemy,
                amount: 2,
                after: stats,
            },
            BattleEvent::SelfDamage {
                amount: 3,
                after: stats,
            },
            BattleEvent::BurnTick {
                target: enemy,
                amount: 2,
                after: stats,
            },
            BattleEvent::Cannibalized {
                unit: enemy,
                amount: 8,
                after: stats,
            },
            BattleEvent::EmergencyGenerator { after: stats },
            BattleEvent::Shuffled,
            BattleEvent::Drawn {
                cards: vec![DeckCard::new(CardId::Scavenge)],
            },
            BattleEvent::Intents {
                unit: enemy,
                intents: [intent, intent],
            },
            BattleEvent::Clock { bonus: 1 },
            BattleEvent::BattleEnded {
                result: BattleResult::Won,
            },
        ]
    }

    #[test]
    fn every_event_has_a_line() {
        for language in LANGUAGES {
            let locale = Locale::new(language);
            for event in all_events() {
                let line = describe_event(&locale, &event, AdmiralId::Reyes);
                assert!(!line.trim().is_empty(), "{event:?} in {language:?}");
            }
        }
        let de = Locale::new(Language::De);
        assert_eq!(
            describe_event(&de, &all_events()[3], AdmiralId::Osei),
            "Bastion trifft Wachkreuzer (Front Mitte): 7"
        );
    }
}
