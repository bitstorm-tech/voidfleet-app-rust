//! Die Gegner (`docs/gegner.md`). Werte und Muster stehen in `assets/data/enemies.json`; jedes Muster ist eine feste
//! Aktionsfolge, die das Gefecht Schritt für Schritt ansagt ([`EnemyId::action`]); was eine Aktion im Gegnerzug
//! anrichtet, rechnet `battle`.

use serde::{Deserialize, Serialize};

use crate::board::COLUMNS;
use crate::data::{EnemyData, Step, data};

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
/// Spalten, Rammkurs macht festen Schaden und verbraucht den Brander, Schilde gelten bis zum nächsten Gegnerzug,
/// Aufladen verdoppelt den nächsten Schuss, Zielerfassung ist die Aura des Spotters, Riegel gibt allen gegnerischen
/// Einheiten Schild.
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

impl EnemyId {
    pub const ALL: [EnemyId; 11] = {
        use EnemyId::*;
        [
            Raiders,
            Fireship,
            TorpedoWing,
            GuardCruiser,
            PlasmaCruiser,
            Spotter,
            Besieger,
            Carrier,
            ScrapKing,
            Warden,
            Seal,
        ]
    };

    fn data(self) -> &'static EnemyData {
        &data().enemies[&self]
    }

    pub fn class(self) -> EnemyClass {
        self.data().class
    }

    /// Unterstützer machen selbst keinen Schaden; bleiben nur sie übrig, ist das Gefecht gewonnen.
    pub fn support(self) -> bool {
        self == EnemyId::Spotter
    }

    /// Hülle vor der Sektor-Skalierung.
    pub fn hull(self) -> i32 {
        self.data().hull
    }

    pub fn attack(self) -> i32 {
        self.data().attack
    }

    /// Fester Abzug je Treffer.
    pub fn armor(self) -> i32 {
        self.data().armor
    }

    /// Die Aktion im Schritt `step` des Musters. `column` ist die Spalte des Flaggschiffs bei der Ansage;
    /// breiteres Spaltenfeuer nimmt erst die linke Nachbarspalte dazu (aus Spalte 0 die rechte), dann die dritte.
    /// Die Reaktion der Plünderer (Ausweichen nach einem Treffer) setzt das Gefecht.
    pub fn action(self, step: u32, column: usize) -> Action {
        let pattern = &self.data().pattern;
        let last = pattern.len() - 1;
        // Ein Muster, das mit dem Rammkurs endet, läuft nicht von vorn: Der Brander rammt einmal und ist weg.
        let index = if pattern[last] == Step::Ram {
            (step as usize).min(last)
        } else {
            step as usize % pattern.len()
        };
        match pattern[index] {
            Step::Fire => Action::Fire,
            Step::ColumnFire(width) => {
                let neighbour = if column == 0 { 1 } else { column - 1 };
                let order = [column, neighbour, COLUMNS - column - neighbour];
                let mut columns = [false; COLUMNS];
                for &hit in &order[..width] {
                    columns[hit] = true;
                }
                Action::ColumnFire { columns }
            }
            Step::Ram => Action::Ram,
            Step::Shields(amount) => Action::Shields(amount),
            Step::Evade => Action::Evade,
            Step::Charge => Action::Charge,
            Step::TargetLock => Action::TargetLock,
            Step::Call(unit) => Action::Call(unit),
            Step::Bolt => Action::Bolt,
        }
    }
}
