//! Das Befehlsdeck: 48 Karten, die nur der Spieler spielt (`docs/karten.md`). Art, Ziel, Kosten, Wert und Zweitwert
//! jeder Karte samt ihrer aufgewerteten Fassung und ihre Texte stehen in `assets/data/cards/<id>.json`, die Startdecks
//! in `admirals.json`. Was die Texte bedeuten, weiß der Client.

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
    ShieldBash,
    Bulwark,
    FlakBreaker,
    BastionDoctrine,
    Onslaught,
    BreakthroughVolley,
    ArmorPiercer,
    AssaultDoctrine,
    TimeWindow,
    PrecisionLock,
    ChronoStrike,
    ClockworkDoctrine,
    Wildfire,
    Accelerant,
    MeltdownSalvo,
    EmberDoctrine,
    EmergencyPower,
    AutocannonSalvo,
    TargetPainter,
    StandbyFire,
    ChainCommand,
    BridgeCoordination,
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

    pub fn extra(self) -> i32 {
        self.id.extra(self.upgraded)
    }
}

impl CardId {
    /// Alle 48 Befehlskarten des Spiels.
    pub const ALL: [CardId; 48] = {
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
            ShieldBash,
            Bulwark,
            FlakBreaker,
            BastionDoctrine,
            Onslaught,
            BreakthroughVolley,
            ArmorPiercer,
            AssaultDoctrine,
            TimeWindow,
            PrecisionLock,
            ChronoStrike,
            ClockworkDoctrine,
            Wildfire,
            Accelerant,
            MeltdownSalvo,
            EmberDoctrine,
            EmergencyPower,
            AutocannonSalvo,
            TargetPainter,
            StandbyFire,
            ChainCommand,
            BridgeCoordination,
        ]
    };

    fn data(self) -> &'static CardData {
        &data().cards[&self]
    }

    /// Die Datei der Karte in `assets/data`: `cards/<id>.json`.
    pub fn file(self) -> String {
        let id = serde_json::to_value(self).expect("Id als JSON");
        format!("cards/{}.json", id.as_str().expect("Id als Text"))
    }

    pub fn card_type(self) -> CardType {
        self.data().card_type
    }

    pub fn target(self) -> CardTarget {
        self.data().target
    }

    /// Schlüsselwort **Einmalig**: Nach dem Spielen bis zum Gefechtsende aus dem Spiel.
    pub const fn exhaust(self) -> bool {
        matches!(self, CardId::MicroJump | CardId::EmergencyPower)
    }

    /// Schlüsselwort **Behalten**: Bleibt in der Auflösung auf der Hand, statt abgeworfen zu werden.
    pub const fn retain(self) -> bool {
        matches!(
            self,
            CardId::Bulwark | CardId::TimeWindow | CardId::PrecisionLock | CardId::StandbyFire
        )
    }

    /// Ob die Karte einen Zweitwert (`extra`) in JSON hat.
    pub const fn has_extra(self) -> bool {
        matches!(
            self,
            CardId::HeatSeeker
                | CardId::CoupDeGrace
                | CardId::HoldTheLine
                | CardId::Overcharge
                | CardId::Scavenge
                | CardId::FlakBreaker
                | CardId::BastionDoctrine
                | CardId::BreakthroughVolley
                | CardId::ArmorPiercer
                | CardId::ClockworkDoctrine
                | CardId::MeltdownSalvo
                | CardId::EmberDoctrine
                | CardId::EmergencyPower
                | CardId::AutocannonSalvo
                | CardId::ChainCommand
        )
    }

    /// Kosten in Befehlspunkten.
    pub fn cost(self, upgraded: bool) -> u32 {
        let card = self.data();
        card.upgraded.cost.filter(|_| upgraded).unwrap_or(card.cost)
    }

    /// Der Zweitwert einer Karte aus [`EXTRA_CARDS`]; 0 bei allen anderen.
    pub fn extra(self, upgraded: bool) -> i32 {
        let card = self.data();
        card.upgraded
            .extra
            .filter(|_| upgraded)
            .or(card.extra)
            .unwrap_or(0)
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

/// Die Karten mit einem Zweitwert ([`CardId::extra`]).
pub const EXTRA_CARDS: [CardId; 15] = [
    CardId::HeatSeeker,
    CardId::CoupDeGrace,
    CardId::HoldTheLine,
    CardId::Overcharge,
    CardId::Scavenge,
    CardId::FlakBreaker,
    CardId::BastionDoctrine,
    CardId::BreakthroughVolley,
    CardId::ArmorPiercer,
    CardId::ClockworkDoctrine,
    CardId::MeltdownSalvo,
    CardId::EmberDoctrine,
    CardId::EmergencyPower,
    CardId::AutocannonSalvo,
    CardId::ChainCommand,
];

/// Die 28 neutralen Karten des gemeinsamen Sockels (für alle Admirale verfügbar).
pub const NEUTRAL_POOL: [CardId; 28] = {
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
        EmergencyShields,
        AblativeArmor,
        PowerSurge,
        RepairCrew,
        EmpBlast,
        TargetingJammer,
        RadioSilence,
        TractorBeam,
        EvasiveAction,
        Countermeasures,
        Scavenge,
        RedAlert,
        FireControlDoctrine,
        BattleStations,
        EmergencyPower,
        AutocannonSalvo,
        TargetPainter,
        StandbyFire,
        ChainCommand,
        BridgeCoordination,
    ]
};

/// Die 5 Karten des Admiral-eigenen Pools (Kennkarte + 4 Admiral-Karten).
pub const fn admiral_pool(admiral: AdmiralId) -> [CardId; 5] {
    use CardId::*;
    match admiral {
        AdmiralId::Osei => [
            HoldTheLine,
            ShieldBash,
            Bulwark,
            FlakBreaker,
            BastionDoctrine,
        ],
        AdmiralId::Vance => [
            Overcharge,
            Onslaught,
            BreakthroughVolley,
            ArmorPiercer,
            AssaultDoctrine,
        ],
        AdmiralId::Kerr => [
            MicroJump,
            TimeWindow,
            PrecisionLock,
            ChronoStrike,
            ClockworkDoctrine,
        ],
        AdmiralId::Reyes => [Firebomb, Wildfire, Accelerant, MeltdownSalvo, EmberDoctrine],
    }
}
