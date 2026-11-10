//! Der Ereignisstrom eines Gefechts in Ausführungsreihenfolge: Daraus animiert der Client die Wiedergabe und schreibt
//! das Log. Einheiten stehen als [`UnitRef`] samt Id und Slot darin, weil der Slot danach leer sein kann; Treffer
//! tragen ihre Rechnung ([`Breakdown`]) und die Werte danach ([`UnitStats`]).

use serde::{Deserialize, Serialize};

use crate::admirals::Station;
use crate::battle::{BattleResult, Target};
use crate::board::EnemySlot;
use crate::cards::DeckCard;
use crate::enemies::{Action, EnemyId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitRef {
    Flagship,
    Enemy {
        uid: u32,
        id: EnemyId,
        slot: EnemySlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitStats {
    pub hull: i32,
    pub shield: i32,
}

/// Wie der Schaden eines Treffers entstand, in der Reihenfolge der Abzüge: Gegenmaßnahmen und Ausweichen halbieren,
/// Deckung leitet einen Teil an den Vordermann (eigener Treffer mit diesem Teil als `base`), Schilde schlucken,
/// Panzerung zieht ab, der Rest geht auf die Hülle. `lethal`: Die Hülle steht danach bei 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakdown {
    pub base: i32,
    pub evaded: i32,
    pub armor: i32,
    pub covered: Option<(u32, i32)>,
    pub shielded: i32,
    pub countermeasures: i32,
    pub hull: i32,
    pub lethal: bool,
}

/// Ein Treffer der Vorschau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub target: UnitRef,
    pub breakdown: Breakdown,
}

/// Eine angesagte Aktion samt ihrem Schaden am Flaggschiff (vor dessen Abzügen), live gerechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub action: Action,
    pub damage: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissReason {
    /// Das Flaggschiff ist im Kurzsprung.
    Jumped,
    /// Das Flaggschiff steht nicht in der Spalte des Spaltenfeuers.
    Maneuvered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestroyCause {
    Shot,
    BurnedOut,
    Rammed,
    /// Gegenfeuer oder Standhalten.
    Retaliation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleReason {
    Stasis,
    Silenced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEvent {
    RoundStarted {
        round: u32,
    },
    CardPlayed {
        card: DeckCard,
        target: Target,
    },
    StationTriggered {
        station: Station,
    },
    Hit {
        source: UnitRef,
        target: UnitRef,
        breakdown: Breakdown,
        after: UnitStats,
    },
    Missed {
        source: UnitRef,
        reason: MissReason,
    },
    Shielded {
        target: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    Healed {
        target: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    Ignited {
        target: UnitRef,
    },
    Extinguished {
        target: UnitRef,
    },
    Emergency {
        target: UnitRef,
        active: bool,
    },
    Destroyed {
        target: UnitRef,
        cause: DestroyCause,
    },
    /// Der Brander hat gerammt und ist weg.
    FireshipSpent {
        unit: UnitRef,
    },
    /// Das Flaggschiff rammt (Karte Rammkurs); es folgen die Treffer an beiden.
    Rammed {
        target: UnitRef,
    },
    Jumped,
    Returned,
    Maneuvered {
        from: usize,
        to: usize,
    },
    /// Traktorstrahl: `unit` steht schon im neuen Slot.
    Tractored {
        unit: UnitRef,
        from: EnemySlot,
    },
    /// Verstärkung oder Nachrücker trifft ein; es folgen ihre Absichten.
    Arrived {
        unit: UnitRef,
    },
    /// Eine Einheit handelt nach ihrer Ansage; bei Feuer folgt `Hit` oder `Missed`.
    EnemyAction {
        unit: UnitRef,
        action: Action,
    },
    Idle {
        unit: UnitRef,
        reason: IdleReason,
    },
    Counterfire {
        target: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    HoldTheLine {
        target: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    /// Überladung am Zugende.
    SelfDamage {
        amount: i32,
        after: UnitStats,
    },
    BurnTick {
        target: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    Cannibalized {
        unit: UnitRef,
        amount: i32,
        after: UnitStats,
    },
    /// Das Notaggregat hält das Flaggschiff bei 1 Hülle.
    EmergencyGenerator {
        after: UnitStats,
    },
    Shuffled,
    Drawn {
        cards: Vec<DeckCard>,
    },
    Intents {
        unit: UnitRef,
        intents: [Intent; 2],
    },
    /// Gefechtsuhr: Zuschlag auf die ATK aller Gegner in dieser Runde.
    Clock {
        bonus: i32,
    },
    BattleEnded {
        result: BattleResult,
    },
}
