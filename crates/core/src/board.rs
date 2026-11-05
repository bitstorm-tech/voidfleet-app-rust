//! Das Brett: Die Gegnerseite hat zwei Reihen (Front und Back) mit je drei Spalten und höchstens vier Einheiten, die
//! Spielerseite drei Spalten, in deren einer das Flaggschiff steht.

use serde::{Deserialize, Serialize};

pub const COLUMNS: usize = 3;
pub const MAX_ENEMIES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Row {
    Front,
    Back,
}

/// Ein Slot der Gegnerseite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnemySlot {
    pub row: Row,
    pub column: usize,
}

impl EnemySlot {
    pub const fn new(row: Row, column: usize) -> Self {
        Self { row, column }
    }

    /// Alle Slots in Handlungs- und Ankunftsreihenfolge: Front vor Back, links nach rechts.
    pub const ALL: [EnemySlot; 2 * COLUMNS] = [
        EnemySlot::new(Row::Front, 0),
        EnemySlot::new(Row::Front, 1),
        EnemySlot::new(Row::Front, 2),
        EnemySlot::new(Row::Back, 0),
        EnemySlot::new(Row::Back, 1),
        EnemySlot::new(Row::Back, 2),
    ];

    /// Der Slot, der eine Back-Einheit deckt.
    pub fn front(self) -> Option<EnemySlot> {
        (self.row == Row::Back).then_some(EnemySlot::new(Row::Front, self.column))
    }

    /// Die Reihenfolge im Brett ([`EnemySlot::ALL`]).
    pub fn order(self) -> usize {
        self.row as usize * COLUMNS + self.column
    }
}
