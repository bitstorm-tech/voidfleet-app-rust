//! Die Admirale: Zu Matchbeginn wählt jeder Spieler verdeckt einen. Jeder bringt sein Flaggschiff mit (eine
//! Schiffskarte außerhalb der Decks mit eigener Fähigkeit), das er in Runde 1 zusätzlich zu seinen Schiffen legt, und
//! hat eine Admiralsfähigkeit, die er jede Runde statt des Grundmanövers einsetzen darf. Wessen Flaggschiff fällt, hat
//! verloren.

use serde::{Deserialize, Serialize};

use crate::cards::ShipId;

/// Osei, die Mauer: Bastion (Wächterschild) und Gegenfeuer. Vance, der Hammer: Vanguard (Speerspitze) und
/// Fokusfeuer. Halvorsen, der Schwarm: Hive (Brutkammer) und Schwarmangriff. Moreau, das Phantom: Mirage
/// (Phasentausch) und Trugbild. Kerr, der Uhrmacher: Pendulum (Railgun) und Taktvorsprung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmiralId {
    Osei,
    Vance,
    Halvorsen,
    Moreau,
    Kerr,
}

pub const ADMIRAL_IDS: [AdmiralId; 5] = [
    AdmiralId::Osei,
    AdmiralId::Vance,
    AdmiralId::Halvorsen,
    AdmiralId::Moreau,
    AdmiralId::Kerr,
];

/// Die Admiralsfähigkeiten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerId {
    Counterfire,
    FocusFire,
    SwarmStrike,
    Decoy,
    HeadStart,
}

/// Wohin eine Admiralsfähigkeit zielt (gültig zur Stellung beim Planen). Die Marke einer Fähigkeit folgt ihrem
/// Schiff, wenn es sich in der Runde bewegt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerTarget {
    /// Ein eigenes Schiff, auch eines, das in dieser Runde gelegt wird (Gegenfeuer).
    OwnShip,
    /// Ebenso, aber nur ein Kampfschiff mit ATK > 0 (Taktvorsprung).
    OwnCombatShip,
    /// Ein gegnerisches Schiff, auch ein gedecktes (Fokusfeuer).
    EnemyShip,
    /// Ein gegnerisches Schiff außer dem Flaggschiff, auch ein gedecktes (Schwarmangriff).
    EnemyShipNotFlagship,
    /// Ein freier eigener Strike-Slot, in dem in dieser Runde nichts anderes landet (Trugbild).
    FreeOwnStrikeSlot,
}

/// Ein Admiral mit Flaggschiff und Fähigkeit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Admiral {
    pub id: AdmiralId,
    pub flagship: ShipId,
    pub power: PowerId,
}

impl AdmiralId {
    pub const fn admiral(self) -> Admiral {
        let (flagship, power) = match self {
            Self::Osei => (ShipId::Bastion, PowerId::Counterfire),
            Self::Vance => (ShipId::Vanguard, PowerId::FocusFire),
            Self::Halvorsen => (ShipId::Hive, PowerId::SwarmStrike),
            Self::Moreau => (ShipId::Mirage, PowerId::Decoy),
            Self::Kerr => (ShipId::Pendulum, PowerId::HeadStart),
        };
        Admiral {
            id: self,
            flagship,
            power,
        }
    }

    pub const fn flagship(self) -> ShipId {
        self.admiral().flagship
    }

    pub const fn power(self) -> PowerId {
        self.admiral().power
    }
}

impl PowerId {
    /// Wohin die Fähigkeit zielt.
    pub const fn target(self) -> PowerTarget {
        match self {
            Self::Counterfire => PowerTarget::OwnShip,
            Self::FocusFire => PowerTarget::EnemyShip,
            Self::SwarmStrike => PowerTarget::EnemyShipNotFlagship,
            Self::Decoy => PowerTarget::FreeOwnStrikeSlot,
            Self::HeadStart => PowerTarget::OwnCombatShip,
        }
    }

    /// Ob die Fähigkeit ein Schiff der Gegenseite zum Ziel hat; sonst liegt ihr Ziel auf der eigenen Seite.
    pub const fn targets_enemy(self) -> bool {
        matches!(
            self.target(),
            PowerTarget::EnemyShip | PowerTarget::EnemyShipNotFlagship
        )
    }
}

/// Die Zahlen der Flaggschiff- und Admiralsfähigkeiten, abgestimmt per Bot-Simulation; der Regel-Dialog nennt sie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityValues {
    /// Bastion (Wächterschild): Schild für sie, jede Runde …
    pub guardian_shield: i32,
    /// … und für das Schiff im Strike-Slot vor ihr.
    pub guardian_shield_front: i32,
    /// Vanguard (Speerspitze): ATK-Bonus des Kampfschiffs im Strike-Slot vor ihr.
    pub spearhead: i32,
    /// Hive (Brutkammer): höchstens so viele Drohnen gleichzeitig auf ihrer Seite …
    pub max_drones: usize,
    /// … und so viele startet sie im ganzen Match.
    pub hive_budget: u32,
    /// Pendulum (Railgun): Schaden auf das markierte Schiff, wenn es zu Kampfbeginn ungedeckt ist.
    pub railgun: i32,
    /// Gegenfeuer: Schaden je Treffer am Schützen (Flaggschiffe ausgenommen).
    pub counterfire: i32,
    /// Fokusfeuer: zusätzlicher Schaden je Treffer auf das Ziel.
    pub focus_fire: i32,
    /// Schwarmangriff: zusätzlicher Schaden je Drohnentreffer auf das Ziel.
    pub swarm_strike: i32,
}

pub const ABILITY_VALUES: AbilityValues = AbilityValues {
    guardian_shield: 2,
    guardian_shield_front: 1,
    spearhead: 1,
    max_drones: 2,
    hive_budget: 8,
    railgun: 5,
    counterfire: 1,
    focus_fire: 1,
    swarm_strike: 2,
};
