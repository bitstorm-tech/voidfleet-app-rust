//! Die vier Admirale (`docs/admirale.md`): Flaggschiff mit Hülle und ATK, Waffe, Passiv, Admiralsstation und
//! Kennkarte; die Zahlen und das Startdeck stehen in `assets/data/admirals.json`, die der Stationen in `rules.json`.
//! Die Passiva wertet das Gefecht aus: Vorstoß (Vance), Uhrwerk (Kerr, zeigt der Client), Glutkern
//! (Reyes), dazu die Waffen Railgun (Kerr) und Plasmawerfer (Reyes).

use serde::{Deserialize, Serialize};

use crate::cards::CardId;
use crate::data::data;

/// Osei, die Mauer; Vance, der Hammer; Kerr, der Uhrmacher; Reyes, die Glut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmiralId {
    Osei,
    Vance,
    Kerr,
    Reyes,
}

pub const ADMIRAL_IDS: [AdmiralId; 4] = [
    AdmiralId::Osei,
    AdmiralId::Vance,
    AdmiralId::Kerr,
    AdmiralId::Reyes,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagshipId {
    Bastion,
    Vanguard,
    Pendulum,
    Pyre,
}

/// Die Stationen des Flaggschiffs; genau eine je Runde (mit Alarmstufe Rot zwei).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Station {
    Weapons,
    Shields,
    DamageControl,
    Maneuver,
    Admiral,
}

/// Gegenfeuer (Osei), Fokusfeuer (Vance), Stillstand (Kerr), Nachsetzen (Reyes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmiralStation {
    Counterfire,
    FocusFire,
    Stasis,
    FollowUp,
}

impl AdmiralId {
    pub fn flagship(self) -> FlagshipId {
        match self {
            AdmiralId::Osei => FlagshipId::Bastion,
            AdmiralId::Vance => FlagshipId::Vanguard,
            AdmiralId::Kerr => FlagshipId::Pendulum,
            AdmiralId::Reyes => FlagshipId::Pyre,
        }
    }

    pub fn hull(self) -> i32 {
        data().admirals[&self].hull
    }

    pub fn attack(self) -> i32 {
        data().admirals[&self].attack
    }

    pub fn signature_card(self) -> CardId {
        data().admirals[&self].signature_card
    }

    pub fn station(self) -> AdmiralStation {
        match self {
            AdmiralId::Osei => AdmiralStation::Counterfire,
            AdmiralId::Vance => AdmiralStation::FocusFire,
            AdmiralId::Kerr => AdmiralStation::Stasis,
            AdmiralId::Reyes => AdmiralStation::FollowUp,
        }
    }

    /// Schild-Station: so viel schluckt sie im folgenden Gegnerzug (Oseis Wächterschild mehr).
    pub fn shield_station(self) -> i32 {
        data().admirals[&self].shield_station
    }
}
