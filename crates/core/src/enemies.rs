//! Die Gegner (`docs/gegner.md`). Jede Einheit eines Verbands trägt ihre Werte und ihr Muster in der Kartendatei des
//! Sektors ([`Unit`]); `assets/data/enemies_defaults.json` ist nur die Vorlage, mit der der Editor eine neue Einheit
//! füllt. Jedes Muster ist eine feste Aktionsfolge, die das Gefecht Schritt für Schritt ansagt ([`action`]); was eine
//! Aktion im Gegnerzug anrichtet, rechnet `battle`.

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
}

/// Eine Einheit eines Verbands mit allen Werten, wie sie in der Kartendatei des Sektors steht; nur dort stehen die
/// Werte, mit denen sie kämpft. `armor` ist der feste Abzug je Treffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: EnemyId,
    pub hull: i32,
    pub attack: i32,
    pub armor: i32,
    pub pattern: Vec<Step>,
}

impl From<EnemyId> for Unit {
    /// Die Vorlage aus `enemies_defaults.json`.
    fn from(id: EnemyId) -> Self {
        let data = id.data();
        Unit {
            id,
            hull: data.hull,
            attack: data.attack,
            armor: data.armor,
            pattern: data.pattern.clone(),
        }
    }
}

impl Unit {
    /// Hülle mindestens 1, ATK und Panzerung nicht negativ, ein gültiges Muster.
    pub fn valid(&self) -> bool {
        self.hull > 0 && self.attack >= 0 && self.armor >= 0 && valid_pattern(&self.pattern)
    }
}

/// Ein Muster hat Schritte, Spaltenfeuer trifft 1 bis [`COLUMNS`] Spalten, Schilde sind positiv, eine gerufene
/// Einheit ist gültig.
pub fn valid_pattern(pattern: &[Step]) -> bool {
    !pattern.is_empty()
        && pattern.iter().all(|step| match step {
            Step::ColumnFire(width) => (1..=COLUMNS).contains(width),
            Step::Shields(amount) => *amount > 0,
            Step::Call(unit) => unit.valid(),
            _ => true,
        })
}

/// Der Index des Schritts `step` im Muster: Es läuft von vorn, außer es endet mit dem Rammkurs: Der Brander rammt
/// einmal und ist weg.
pub fn step_index(pattern: &[Step], step: u32) -> usize {
    let last = pattern.len() - 1;
    if pattern[last] == Step::Ram {
        (step as usize).min(last)
    } else {
        step as usize % pattern.len()
    }
}

/// Die Aktion im Schritt `step` des Musters. `column` ist die Spalte des Flaggschiffs bei der Ansage;
/// breiteres Spaltenfeuer nimmt erst die linke Nachbarspalte dazu (aus Spalte 0 die rechte), dann die dritte.
/// Die Reaktion der Plünderer (Ausweichen nach einem Treffer) setzt das Gefecht.
pub fn action(pattern: &[Step], step: u32, column: usize) -> Action {
    match pattern[step_index(pattern, step)] {
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
        Step::Call(ref unit) => Action::Call(unit.id),
        Step::Bolt => Action::Bolt,
    }
}
