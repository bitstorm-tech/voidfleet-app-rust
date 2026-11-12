//! Das Befehlsdeck: 26 Karten, die nur der Spieler spielt (`docs/karten.md`). Art, Ziel, Kosten und Wert jeder
//! Karte samt ihrer aufgewerteten Fassung stehen in `assets/data/cards.json`, die Startdecks in `admirals.json`.
//! Namen und Effekttexte liegen im Client.

use serde::{Deserialize, Serialize};

use crate::admirals::AdmiralId;
use crate::data::{CardData, data};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardId {
    Torpedo,
    OrbitalStrike,
    VolleyFire,
    Barrage,
    HeatSeeker,
    CoupDeGrace,
    Bombardment,
    CollisionCourse,
    Firebomb,
    EmergencyShields,
    HoldTheLine,
    AblativeArmor,
    Overcharge,
    PowerSurge,
    RepairCrew,
    EmpBlast,
    TargetingJammer,
    RadioSilence,
    TractorBeam,
    MicroJump,
    EvasiveAction,
    Countermeasures,
    Scavenge,
    RedAlert,
    FireControlDoctrine,
    BattleStations,
}

/// Angriff, Verstärkung, Schwächung, Taktik, Doktrin (bleibt bis zum Gefechtsende liegen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Attack,
    Buff,
    Debuff,
    Tactic,
    Doctrine,
}

/// Worauf eine Karte zielt: nichts, eine gegnerische Einheit oder eine Spalte der Spielerseite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardTarget {
    None,
    Enemy,
    Column,
}

/// Eine Karte im Deck des Runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckCard {
    pub id: CardId,
    pub upgraded: bool,
}

impl DeckCard {
    pub const fn new(id: CardId) -> Self {
        Self {
            id,
            upgraded: false,
        }
    }

    pub fn cost(self) -> u32 {
        self.id.cost(self.upgraded)
    }

    pub fn value(self) -> i32 {
        self.id.value(self.upgraded)
    }
}

impl CardId {
    fn data(self) -> &'static CardData {
        &data().cards[&self]
    }

    pub fn card_type(self) -> CardType {
        self.data().card_type
    }

    pub fn target(self) -> CardTarget {
        self.data().target
    }

    /// Kosten in Befehlspunkten.
    pub fn cost(self, upgraded: bool) -> u32 {
        let card = self.data();
        card.upgraded.cost.filter(|_| upgraded).unwrap_or(card.cost)
    }

    /// Der Hauptwert der Karte: Schaden (Rammkurs: Zuschlag auf das Doppelte der ATK), Schild, Reparatur, ATK-Bonus,
    /// Panzerung; 0 bei Karten ohne Zahl.
    pub fn value(self, upgraded: bool) -> i32 {
        let card = self.data();
        card.upgraded
            .value
            .filter(|_| upgraded)
            .unwrap_or(card.value)
    }
}

/// Das Startdeck des Admirals.
pub fn starter_deck(admiral: AdmiralId) -> Vec<DeckCard> {
    let deck = &data().admirals[&admiral].deck;
    deck.iter().copied().map(DeckCard::new).collect()
}

/// Alle Karten, aus denen die Beute kommt (im Prototyp ein gemeinsamer Pool).
pub const LOOT_POOL: [CardId; 26] = {
    use CardId::*;
    [
        Torpedo,
        OrbitalStrike,
        VolleyFire,
        Barrage,
        HeatSeeker,
        CoupDeGrace,
        Bombardment,
        CollisionCourse,
        Firebomb,
        EmergencyShields,
        HoldTheLine,
        AblativeArmor,
        Overcharge,
        PowerSurge,
        RepairCrew,
        EmpBlast,
        TargetingJammer,
        RadioSilence,
        TractorBeam,
        MicroJump,
        EvasiveAction,
        Countermeasures,
        Scavenge,
        RedAlert,
        FireControlDoctrine,
        BattleStations,
    ]
};
