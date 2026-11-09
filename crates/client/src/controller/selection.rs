//! Die Auswahl als kleine Zustandsmaschine ohne Bevy: was ein Klick je Zustand bedeutet, welche Slots gültige Ziele
//! sind und was die Vorschau unter dem Zeiger zeigt.
//!
//! - Nichts gewählt: Ein Klick auf einen Gegner setzt das Ziel der Stationen (Waffen, Fokusfeuer, Stillstand), solange
//!   eine davon in dieser Runde noch laufen darf; ein Klick auf eine eigene Spalte setzt die Zielspalte des Manövers,
//!   solange es noch laufen darf. Ausgelöst werden Stationen im Stationen-Feld.
//! - Eine Karte gewählt, die ein Ziel braucht: Der Klick auf ein gültiges Ziel spielt sie. Karten ohne Ziel wirken
//!   sofort beim Klick auf die Hand.

use voidfleet_core::{
    admirals::Station,
    battle::{Battle, RuleError, Target},
    cards::CardTarget,
    events::{Hit, UnitRef},
};

use crate::{
    game::PlayerAction,
    scene::coords::{Side, SlotRef},
};

/// Was gerade gewählt ist.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    Idle,
    /// Hand-Index einer Karte, die auf ihr Ziel wartet.
    Card(usize),
}

/// Worauf der Spieler geklickt hat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Card(usize),
    Slot(SlotRef),
    /// Daneben, ins Weltall.
    Miss,
}

/// Was ein Klick bewirkt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Die Auswahl wechselt.
    Select(Selection),
    /// Eine Aktion geht ans Gefecht; die Auswahl ist danach leer.
    Act(PlayerAction),
    /// Geht nicht, die Auswahl bleibt.
    Reject(RuleError),
    Nothing,
}

/// Ob eine Station, die ein gegnerisches Ziel braucht, in dieser Runde noch laufen darf.
pub fn station_needs_target(battle: &Battle) -> bool {
    [Station::Weapons, Station::Admiral]
        .into_iter()
        .any(|station| {
            battle.flagship.needs_target(station) && battle.station_open(station).is_ok()
        })
}

/// Ob das Manöver in dieser Runde noch laufen darf.
fn maneuver_open(battle: &Battle) -> bool {
    battle.station_open(Station::Maneuver).is_ok()
}

/// Die Einheit auf einem Slot der Gegnerseite.
fn enemy_at(battle: &Battle, slot: SlotRef) -> Option<u32> {
    battle
        .enemies
        .iter()
        .find(|enemy| SlotRef::enemy(enemy.slot) == slot)
        .map(|enemy| enemy.uid)
}

/// Was ein Klick bedeutet.
pub fn click(battle: &Battle, selection: Selection, pick: Pick) -> Outcome {
    match (pick, selection) {
        (Pick::Miss, _) => Outcome::Select(Selection::Idle),
        (Pick::Card(index), Selection::Card(selected)) if index == selected => {
            Outcome::Select(Selection::Idle)
        }
        (Pick::Card(index), _) => {
            let Some(card) = battle.hand.get(index) else {
                return Outcome::Nothing;
            };
            if card.cost() > battle.points {
                return Outcome::Reject(RuleError::NotEnoughPoints);
            }
            match card.id.target() {
                CardTarget::None => Outcome::Act(PlayerAction::PlayCard {
                    hand_index: index,
                    target: Target::None,
                }),
                _ => Outcome::Select(Selection::Card(index)),
            }
        }
        (Pick::Slot(slot), Selection::Card(index)) => {
            let target = match (battle.hand[index].id.target(), slot) {
                (CardTarget::Enemy, _) => enemy_at(battle, slot).map(Target::Enemy),
                (
                    CardTarget::Column,
                    SlotRef {
                        side: Side::Player,
                        column,
                        ..
                    },
                ) => Some(Target::Column(column)),
                _ => None,
            };
            target.map_or(Outcome::Nothing, |target| {
                Outcome::Act(PlayerAction::PlayCard {
                    hand_index: index,
                    target,
                })
            })
        }
        (Pick::Slot(slot), Selection::Idle) => match enemy_at(battle, slot) {
            Some(uid) if station_needs_target(battle) && battle.flagship.target != Some(uid) => {
                Outcome::Act(PlayerAction::SetTarget(uid))
            }
            Some(_) => Outcome::Nothing,
            None if slot.side == Side::Enemy => Outcome::Nothing,
            None if maneuver_open(battle) && slot.column != battle.flagship.column => {
                Outcome::Act(PlayerAction::SetColumn(slot.column))
            }
            None => Outcome::Nothing,
        },
    }
}

/// Gültige Ziele: Gegner für Karten mit Ziel und für Zielstationen, eigene Spalten für Ausweichmanöver und Manöver
/// (nie die Spalte, in der das Flaggschiff steht).
pub fn highlights(battle: &Battle, selection: Selection) -> Vec<SlotRef> {
    if battle.result.is_some() {
        return Vec::new();
    }
    let enemies = || battle.enemies.iter().map(|enemy| (enemy.uid, enemy.slot));
    let columns =
        || (0..voidfleet_core::board::COLUMNS).filter(|&column| column != battle.flagship.column);
    match selection {
        Selection::Card(index) => {
            let valid = |target| battle.preview_card(index, target).is_ok();
            match battle.hand[index].id.target() {
                CardTarget::Enemy => enemies()
                    .filter(|&(uid, _)| valid(Target::Enemy(uid)))
                    .map(|(_, slot)| SlotRef::enemy(slot))
                    .collect(),
                CardTarget::Column => columns()
                    .filter(|&column| valid(Target::Column(column)))
                    .map(SlotRef::player)
                    .collect(),
                CardTarget::None => Vec::new(),
            }
        }
        Selection::Idle => {
            let mut slots: Vec<SlotRef> = Vec::new();
            if station_needs_target(battle) {
                slots.extend(enemies().map(|(_, slot)| SlotRef::enemy(slot)));
            }
            if maneuver_open(battle) {
                slots.extend(columns().map(SlotRef::player));
            }
            slots
        }
    }
}

/// Die Vorschau unter dem Zeiger: die Treffer der gewählten Karte bzw. der Waffen (solange sie noch laufen dürfen) auf
/// den Gegner unter dem Zeiger, samt dessen Kennung. Gerechnet auf einer Kopie, also genau wie die Ausführung.
pub fn preview(
    battle: &Battle,
    selection: Selection,
    hover: Option<SlotRef>,
) -> Option<(u32, Vec<Hit>)> {
    let uid = enemy_at(battle, hover?)?;
    let hits = match selection {
        Selection::Card(index) => battle.preview_card(index, Target::Enemy(uid)).ok()?,
        Selection::Idle => {
            let mut copy = battle.clone();
            copy.set_target(uid).ok()?;
            copy.preview_station(Station::Weapons).ok()?
        }
    };
    Some((uid, hits))
}

/// Die Vorschau einer Station unter dem Zeiger im Stationen-Feld, mit dem eingestellten Ziel bzw. der eingestellten
/// Spalte, samt der Kennung des ersten Getroffenen; ohne Treffer keine.
pub fn station_preview(battle: &Battle, station: Station) -> Option<(u32, Vec<Hit>)> {
    let hits = battle.preview_station(station).ok()?;
    Some((unit_uid(hits.first()?.target), hits))
}

/// Die Kennung einer Einheit in der Szene: Gegner nach `uid`, das Flaggschiff 0.
pub fn unit_uid(unit: UnitRef) -> u32 {
    match unit {
        UnitRef::Flagship => 0,
        UnitRef::Enemy { uid, .. } => uid,
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        admirals::AdmiralId,
        cards::{CardId, DeckCard},
    };

    use super::*;
    use crate::game::fixture_battle;

    fn battle() -> Battle {
        let mut battle = fixture_battle(AdmiralId::Kerr);
        battle.hand = [
            CardId::Torpedo,
            CardId::EmergencyShields,
            CardId::EvasiveAction,
            CardId::OrbitalStrike,
        ]
        .map(DeckCard::new)
        .to_vec();
        battle
    }

    fn enemy_slot(battle: &Battle, index: usize) -> (u32, SlotRef) {
        let enemy = &battle.enemies[index];
        (enemy.uid, SlotRef::enemy(enemy.slot))
    }

    #[test]
    fn cards_without_target_play_at_once() {
        let battle = battle();
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Card(1)),
            Outcome::Act(PlayerAction::PlayCard {
                hand_index: 1,
                target: Target::None
            })
        );
    }

    #[test]
    fn targeted_card_waits_for_enemy() {
        let battle = battle();
        let (uid, slot) = enemy_slot(&battle, 0);
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Card(0)),
            Outcome::Select(Selection::Card(0))
        );
        assert_eq!(
            click(&battle, Selection::Card(0), Pick::Card(0)),
            Outcome::Select(Selection::Idle)
        );
        assert_eq!(
            click(&battle, Selection::Card(0), Pick::Slot(SlotRef::player(0))),
            Outcome::Nothing
        );
        assert_eq!(
            click(&battle, Selection::Card(0), Pick::Slot(slot)),
            Outcome::Act(PlayerAction::PlayCard {
                hand_index: 0,
                target: Target::Enemy(uid)
            })
        );
        assert_eq!(
            click(&battle, Selection::Card(0), Pick::Miss),
            Outcome::Select(Selection::Idle)
        );
        let highlighted = highlights(&battle, Selection::Card(0));
        assert_eq!(highlighted.len(), battle.enemies.len());
    }

    #[test]
    fn column_card_and_unaffordable_card() {
        let mut battle = battle();
        assert_eq!(
            click(&battle, Selection::Card(2), Pick::Slot(SlotRef::player(0))),
            Outcome::Act(PlayerAction::PlayCard {
                hand_index: 2,
                target: Target::Column(0)
            })
        );
        assert_eq!(
            highlights(&battle, Selection::Card(2)),
            vec![SlotRef::player(0), SlotRef::player(2)]
        );
        battle.points = 2;
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Card(3)),
            Outcome::Reject(RuleError::NotEnoughPoints)
        );
    }

    #[test]
    fn idle_clicks_set_target_and_column() {
        let mut battle = battle();
        let (uid, slot) = enemy_slot(&battle, 1);
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Slot(slot)),
            Outcome::Act(PlayerAction::SetTarget(uid))
        );
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Slot(SlotRef::player(2))),
            Outcome::Act(PlayerAction::SetColumn(2))
        );
        assert_eq!(
            highlights(&battle, Selection::Idle).len(),
            battle.enemies.len() + 2
        );
        battle.set_target(uid).unwrap();
        battle.run_station(Station::Weapons).unwrap();
        assert_eq!(
            click(
                &battle,
                Selection::Idle,
                Pick::Slot(enemy_slot(&battle, 0).1)
            ),
            Outcome::Nothing
        );
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Slot(SlotRef::player(2))),
            Outcome::Nothing
        );
        assert!(highlights(&battle, Selection::Idle).is_empty());
        // Mit Alarmstufe Rot sind Stillstand (braucht ein Ziel) und Manöver noch offen.
        battle.flagship.station_slots = 2;
        assert!(station_needs_target(&battle));
        assert!(
            highlights(&battle, Selection::Idle)
                .ends_with(&[SlotRef::player(0), SlotRef::player(2)])
        );
    }

    #[test]
    fn preview_shows_station_and_card_hits() {
        let battle = battle();
        let (uid, slot) = enemy_slot(&battle, 0);
        let (focus, hits) = preview(&battle, Selection::Idle, Some(slot)).unwrap();
        assert_eq!(focus, uid);
        assert_eq!(unit_uid(hits[0].target), uid);
        assert_eq!(hits[0].breakdown.base, battle.flagship_attack());
        let (_, hits) = preview(&battle, Selection::Card(0), Some(slot)).unwrap();
        assert!(hits.iter().any(|hit| unit_uid(hit.target) == uid));
        assert!(preview(&battle, Selection::Idle, Some(SlotRef::player(0))).is_none());
        assert!(station_preview(&battle, Station::Weapons).is_none());
        let mut battle = battle;
        battle.set_target(uid).unwrap();
        let (focus, hits) = station_preview(&battle, Station::Weapons).unwrap();
        assert_eq!(focus, uid);
        assert_eq!(hits[0].breakdown.base, battle.flagship_attack());
        battle.run_station(Station::Weapons).unwrap();
        assert!(preview(&battle, Selection::Idle, Some(slot)).is_none());
    }
}
