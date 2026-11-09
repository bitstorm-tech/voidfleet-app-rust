//! Die vier Admirale (`docs/admirale.md`): Flaggschiff mit Hülle und ATK, Waffe, Passiv, Admiralsstation und
//! Kennkarte. Die Passiva wertet das Gefecht aus: Vorstoß (Vance), Uhrwerk (Kerr, zeigt der Client), Glutkern
//! (Reyes), dazu die Waffen Railgun (Kerr) und Plasmawerfer (Reyes).

use serde::{Deserialize, Serialize};

use crate::cards::CardId;

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

/// Schild-Station: so viel schluckt sie im folgenden Gegnerzug, Osei (Wächterschild) mehr.
pub const SHIELD_STATION: i32 = 8;
pub const GUARDIAN_SHIELD: i32 = 12;
/// Schadenskontrolle: Hülle zurück.
pub const DAMAGE_CONTROL_REPAIR: i32 = 5;
/// Vorstoß: höchstens so viel ATK.
pub const MAX_MOMENTUM: i32 = 4;
/// Gegenfeuer: Schild und Hülle, die jeder Schütze verliert, der die Bastion trifft.
pub const COUNTERFIRE_SHIELD: i32 = 6;
pub const COUNTERFIRE_DAMAGE: i32 = 3;
/// Fokusfeuer: Mehrschaden je gegnerischem Treffer im folgenden Gegnerzug.
pub const FOCUS_FIRE_VULNERABILITY: i32 = 2;
/// Nachsetzen: Zuschlag für Angriffskarten auf brennende Ziele.
pub const FOLLOW_UP_CARD_BONUS: i32 = 2;

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
        match self {
            AdmiralId::Osei => 70,
            AdmiralId::Vance => 50,
            AdmiralId::Kerr | AdmiralId::Reyes => 56,
        }
    }

    pub fn attack(self) -> i32 {
        match self {
            AdmiralId::Osei => 7,
            AdmiralId::Vance => 11,
            AdmiralId::Kerr => 8,
            AdmiralId::Reyes => 9,
        }
    }

    pub fn signature_card(self) -> CardId {
        match self {
            AdmiralId::Osei => CardId::HoldTheLine,
            AdmiralId::Vance => CardId::Overcharge,
            AdmiralId::Kerr => CardId::MicroJump,
            AdmiralId::Reyes => CardId::Firebomb,
        }
    }

    pub fn station(self) -> AdmiralStation {
        match self {
            AdmiralId::Osei => AdmiralStation::Counterfire,
            AdmiralId::Vance => AdmiralStation::FocusFire,
            AdmiralId::Kerr => AdmiralStation::Stasis,
            AdmiralId::Reyes => AdmiralStation::FollowUp,
        }
    }

    pub fn shield_station(self) -> i32 {
        if self == AdmiralId::Osei {
            GUARDIAN_SHIELD
        } else {
            SHIELD_STATION
        }
    }
}
