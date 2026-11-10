//! Die Auswahl als kleine Zustandsmaschine ohne Bevy: was ein Klick je Zustand bedeutet, welche Slots gültige Ziele
//! sind, was die Vorschau unter dem Zeiger zeigt und wo der Geist des Flaggschiffs steht.
//!
//! Karten der Hand und Stationen folgen derselben Grammatik: Was kein Ziel braucht, wirkt sofort beim Klick; was eins
//! braucht (Gegner oder eigene Spalte), ist danach gewählt, die gültigen Ziele leuchten, und der Klick auf eins führt
//! es aus. Ein zweiter Klick auf das Gewählte, ein Klick daneben (auch Rechtsklick und Escape im Controller) hebt die
//! Wahl auf. Ohne Wahl tut ein Klick aufs Brett nichts.

use voidfleet_core::{
    admirals::Station,
    battle::{Battle, RuleError, Target},
    board::COLUMNS,
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
    /// Eine Station, die auf ihr Ziel wartet.
    Station(Station),
}

/// Worauf der Spieler geklickt hat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Card(usize),
    Station(Station),
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

/// Das Gewählte, Karte oder Station, mit einem Code: welches Ziel es braucht, was es dort träfe, welche Aktion es ist.
impl Selection {
    pub fn target_kind(self, battle: &Battle) -> CardTarget {
        match self {
            Self::Idle => CardTarget::None,
            Self::Card(index) => battle.hand[index].id.target(),
            Self::Station(station) => battle.station_target(station),
        }
    }

    /// Die Treffer auf einer Kopie; `None`, wenn das Ziel nicht gilt.
    fn preview(self, battle: &Battle, target: Target) -> Option<Vec<Hit>> {
        match self {
            Self::Idle => None,
            Self::Card(index) => battle.preview_card(index, target).ok(),
            Self::Station(station) => battle.preview_station(station, target).ok(),
        }
    }

    fn action(self, target: Target) -> Option<PlayerAction> {
        match self {
            Self::Idle => None,
            Self::Card(hand_index) => Some(PlayerAction::PlayCard { hand_index, target }),
            Self::Station(station) => Some(PlayerAction::RunStation { station, target }),
        }
    }

    /// Ohne Ziel wirkt das Gewählte sofort, sonst wartet es.
    fn choose(self, battle: &Battle) -> Outcome {
        match self.target_kind(battle) {
            CardTarget::None => self
                .action(Target::None)
                .map_or(Outcome::Nothing, Outcome::Act),
            _ => Outcome::Select(self),
        }
    }

    /// Das Ziel, das ein Slot für das Gewählte wäre (geprüft wird erst über die Vorschau bzw. beim Ausführen).
    fn target_at(self, battle: &Battle, slot: SlotRef) -> Option<Target> {
        match (self.target_kind(battle), slot.side) {
            (CardTarget::Enemy, Side::Enemy) => enemy_at(battle, slot).map(Target::Enemy),
            (CardTarget::Column, Side::Player) => Some(Target::Column(slot.column)),
            _ => None,
        }
    }
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
    match pick {
        Pick::Miss => Outcome::Select(Selection::Idle),
        Pick::Card(index) if selection == Selection::Card(index) => {
            Outcome::Select(Selection::Idle)
        }
        Pick::Station(station) if selection == Selection::Station(station) => {
            Outcome::Select(Selection::Idle)
        }
        Pick::Card(index) => match battle.hand.get(index) {
            None => Outcome::Nothing,
            Some(card) if card.cost() > battle.points => {
                Outcome::Reject(RuleError::NotEnoughPoints)
            }
            Some(_) => Selection::Card(index).choose(battle),
        },
        Pick::Station(station) => match battle.station_open(station) {
            Err(error) => Outcome::Reject(error),
            Ok(()) => Selection::Station(station).choose(battle),
        },
        Pick::Slot(slot) => selection
            .target_at(battle, slot)
            .and_then(|target| selection.action(target))
            .map_or(Outcome::Nothing, Outcome::Act),
    }
}

/// Gültige Ziele des Gewählten: Gegner bzw. eigene Spalten (nie die, in der das Flaggschiff steht), für die die
/// Vorschau gelingt.
pub fn highlights(battle: &Battle, selection: Selection) -> Vec<SlotRef> {
    let valid = |target| selection.preview(battle, target).is_some();
    match selection.target_kind(battle) {
        CardTarget::Enemy => battle
            .enemies
            .iter()
            .filter(|enemy| valid(Target::Enemy(enemy.uid)))
            .map(|enemy| SlotRef::enemy(enemy.slot))
            .collect(),
        CardTarget::Column => (0..COLUMNS)
            .filter(|&column| valid(Target::Column(column)))
            .map(SlotRef::player)
            .collect(),
        CardTarget::None => Vec::new(),
    }
}

/// Die Vorschau unter dem Zeiger: die Treffer des Gewählten auf den Gegner unter dem Zeiger, samt dessen Kennung.
/// Gerechnet auf einer Kopie, also genau wie die Ausführung.
pub fn preview(
    battle: &Battle,
    selection: Selection,
    hover: Option<SlotRef>,
) -> Option<(u32, Vec<Hit>)> {
    let uid = enemy_at(battle, hover?)?;
    Some((uid, selection.preview(battle, Target::Enemy(uid))?))
}

/// Wo der Geist des Flaggschiffs steht: in der eigenen Spalte unter dem Zeiger, wenn das Gewählte eine Spalte braucht
/// und sie gilt.
pub fn ghost(battle: &Battle, selection: Selection, hover: Option<SlotRef>) -> Option<SlotRef> {
    let target @ Target::Column(column) = selection.target_at(battle, hover?)? else {
        return None;
    };
    selection.preview(battle, target)?;
    Some(SlotRef::player(column))
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
    fn targeted_station_waits_for_enemy() {
        let battle = battle();
        let (uid, slot) = enemy_slot(&battle, 1);
        let weapons = Selection::Station(Station::Weapons);
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Slot(slot)),
            Outcome::Nothing
        );
        assert!(highlights(&battle, Selection::Idle).is_empty());
        assert_eq!(
            click(&battle, Selection::Card(0), Pick::Station(Station::Weapons)),
            Outcome::Select(weapons)
        );
        assert_eq!(highlights(&battle, weapons).len(), battle.enemies.len());
        assert_eq!(
            click(&battle, weapons, Pick::Slot(SlotRef::player(0))),
            Outcome::Nothing
        );
        assert_eq!(
            click(&battle, weapons, Pick::Slot(slot)),
            Outcome::Act(PlayerAction::RunStation {
                station: Station::Weapons,
                target: Target::Enemy(uid)
            })
        );
        assert_eq!(
            click(&battle, weapons, Pick::Station(Station::Weapons)),
            Outcome::Select(Selection::Idle)
        );
        let (focus, hits) = preview(&battle, weapons, Some(slot)).unwrap();
        assert_eq!(focus, uid);
        assert_eq!(hits[0].breakdown.base, battle.flagship_attack());
    }

    #[test]
    fn maneuver_waits_for_own_column() {
        let battle = battle();
        let maneuver = Selection::Station(Station::Maneuver);
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Station(Station::Maneuver)),
            Outcome::Select(maneuver)
        );
        assert_eq!(
            highlights(&battle, maneuver),
            vec![SlotRef::player(0), SlotRef::player(2)]
        );
        assert_eq!(
            click(&battle, maneuver, Pick::Slot(SlotRef::player(2))),
            Outcome::Act(PlayerAction::RunStation {
                station: Station::Maneuver,
                target: Target::Column(2)
            })
        );
        assert_eq!(
            ghost(&battle, maneuver, Some(SlotRef::player(2))),
            Some(SlotRef::player(2))
        );
        assert_eq!(ghost(&battle, maneuver, Some(SlotRef::player(1))), None);
        assert_eq!(
            ghost(&battle, Selection::Idle, Some(SlotRef::player(2))),
            None
        );
        assert_eq!(
            ghost(&battle, Selection::Card(2), Some(SlotRef::player(0))),
            Some(SlotRef::player(0))
        );
    }

    #[test]
    fn station_without_target_runs_at_once_and_only_while_open() {
        let mut battle = battle();
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Station(Station::Shields)),
            Outcome::Act(PlayerAction::RunStation {
                station: Station::Shields,
                target: Target::None
            })
        );
        battle.run_station(Station::Shields, Target::None).unwrap();
        assert_eq!(
            click(&battle, Selection::Idle, Pick::Station(Station::Weapons)),
            Outcome::Reject(RuleError::NoStationSlot)
        );
        assert!(highlights(&battle, Selection::Station(Station::Weapons)).is_empty());
    }

    #[test]
    fn preview_shows_card_hits() {
        let battle = battle();
        let (uid, slot) = enemy_slot(&battle, 0);
        let (_, hits) = preview(&battle, Selection::Card(0), Some(slot)).unwrap();
        assert!(hits.iter().any(|hit| unit_uid(hit.target) == uid));
        assert!(preview(&battle, Selection::Idle, Some(slot)).is_none());
        assert!(preview(&battle, Selection::Card(0), Some(SlotRef::player(0))).is_none());
    }
}
