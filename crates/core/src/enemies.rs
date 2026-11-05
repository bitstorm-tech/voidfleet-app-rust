//! Die Gegner (Entwurf Abschnitt 12): Werte und Muster. Jedes Muster ist eine feste Aktionsfolge, die das Gefecht
//! Schritt für Schritt ansagt ([`EnemyId::action`]); was eine Aktion im Gegnerzug anrichtet, rechnet `battle`.

use serde::{Deserialize, Serialize};

use crate::board::COLUMNS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyId {
    /// Plünderer-Staffel.
    Raiders,
    /// Brander.
    Fireship,
    TorpedoWing,
    /// Wachkreuzer.
    GuardCruiser,
    PlasmaCruiser,
    Spotter,
    /// Belagerer.
    Besieger,
    /// Trägerschiff.
    Carrier,
    /// Schrottkönig, Boss von Sektor 1.
    ScrapKing,
    /// Wächter, Boss von Sektor 2.
    Warden,
    /// Siegel, Flaggschiff von Kommandantin Vey, Boss von Sektor 3.
    Seal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyClass {
    Fighter,
    Cruiser,
    Battleship,
    Flagship,
}

/// Eine angesagte Aktion. Zielfeuer folgt dem Flaggschiff, Spaltenfeuer trifft die bei der Ansage bestimmten
/// Spalten, Rammkurs macht feste 10 und verbraucht den Brander, Schilde gelten bis zum nächsten Gegnerzug,
/// Aufladen verdoppelt den nächsten Schuss, Zielerfassung ist die Aura des Spotters, Riegel gibt allen gegnerischen
/// Einheiten Schild 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Fire,
    ColumnFire { columns: [bool; COLUMNS] },
    Ram,
    Shields(i32),
    Evade,
    Charge,
    TargetLock,
    Call(EnemyId),
    Bolt,
}

pub const RAM_DAMAGE: i32 = 10;
pub const BOLT_SHIELD: i32 = 4;
pub const CANNIBALIZE_HEAL: i32 = 8;

impl EnemyId {
    pub fn class(self) -> EnemyClass {
        use EnemyId::*;
        match self {
            Raiders | Fireship | TorpedoWing => EnemyClass::Fighter,
            GuardCruiser | PlasmaCruiser | Spotter => EnemyClass::Cruiser,
            Besieger | Carrier | ScrapKing | Warden => EnemyClass::Battleship,
            Seal => EnemyClass::Flagship,
        }
    }

    /// Hülle vor der Sektor-Skalierung.
    pub fn hull(self) -> i32 {
        use EnemyId::*;
        match self {
            Raiders | TorpedoWing => 8,
            Fireship => 6,
            GuardCruiser => 16,
            PlasmaCruiser => 14,
            Spotter => 12,
            Besieger => 30,
            Carrier => 26,
            ScrapKing => 60,
            Warden => 90,
            Seal => 50,
        }
    }

    pub fn attack(self) -> i32 {
        use EnemyId::*;
        match self {
            Raiders | Fireship | Spotter => 2,
            TorpedoWing | GuardCruiser | PlasmaCruiser => 4,
            Carrier => 5,
            ScrapKing => 7,
            Besieger | Seal => 8,
            Warden => 15,
        }
    }

    pub fn armor(self) -> i32 {
        if self == EnemyId::Besieger { 2 } else { 0 }
    }

    /// Die Aktion im Schritt `step` des Musters. `column` ist die Spalte des Flaggschiffs bei der Ansage; der Wächter
    /// nimmt eine Nachbarspalte dazu. Die Reaktion der Plünderer (Ausweichen nach einem Treffer) setzt das Gefecht.
    pub fn action(self, step: u32, column: usize) -> Action {
        use Action::*;
        let mut columns = [false; COLUMNS];
        columns[column] = true;
        let cycle: &[Action] = match self {
            EnemyId::Raiders | EnemyId::PlasmaCruiser => &[Fire],
            EnemyId::Fireship => &[Evade, Evade, Ram],
            EnemyId::TorpedoWing => &[ColumnFire { columns }, Evade],
            EnemyId::GuardCruiser => &[Fire, Fire, Shields(6)],
            EnemyId::Spotter => &[TargetLock],
            EnemyId::Besieger => &[Charge, ColumnFire { columns }, Shields(10)],
            EnemyId::Carrier => &[Fire, Call(EnemyId::Raiders)],
            EnemyId::ScrapKing => &[Fire, Fire, Call(EnemyId::Raiders)],
            EnemyId::Warden => {
                columns[if column == 0 { 1 } else { column - 1 }] = true;
                &[Charge, Charge, ColumnFire { columns }, Shields(15)]
            }
            EnemyId::Seal => &[Fire, Bolt],
        };
        // Der Brander rammt nur einmal und ist danach weg.
        let step = if self == EnemyId::Fireship {
            step.min(2)
        } else {
            step
        };
        cycle[step as usize % cycle.len()]
    }
}
