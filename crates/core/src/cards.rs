//! Das Befehlsdeck: 26 Karten, die nur der Spieler spielt (Entwurf Abschnitt 8). Jede Karte hat eine aufgewertete
//! Fassung: Angriffskarten und Schild- oder Reparaturwerte +2, alle anderen kosten einen Befehlspunkt weniger.
//! Namen und Effekttexte liegen im Client.

use serde::{Deserialize, Serialize};

use crate::admirals::AdmiralId;

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

/// Wärmesucher auf ein ausweichendes Ziel: 3 mehr (5 → 8).
pub const HEAT_SEEKER_EVADING_BONUS: i32 = 3;
/// Gnadenstoß auf ein Ziel im Notfall: 5 mehr (3 → 8).
pub const COUP_DE_GRACE_EMERGENCY_BONUS: i32 = 5;
/// Standhalten: Hülle, die jeder Schütze verliert, der das Flaggschiff trifft.
pub const HOLD_THE_LINE_RETALIATION: i32 = 2;
/// Überladung: Schaden am Flaggschiff zum Zugende.
pub const OVERCHARGE_SELF_DAMAGE: i32 = 3;
/// Plündern: so viele Karten ziehen.
pub const SCAVENGE_DRAW: usize = 2;
/// Aufwertung von Schaden, Schild und Reparatur.
pub const UPGRADE_VALUE: i32 = 2;

impl CardId {
    pub fn card_type(self) -> CardType {
        use CardId::*;
        match self {
            Torpedo | OrbitalStrike | VolleyFire | Barrage | HeatSeeker | CoupDeGrace
            | Bombardment | CollisionCourse | Firebomb => CardType::Attack,
            EmergencyShields | HoldTheLine | AblativeArmor | Overcharge | PowerSurge
            | RepairCrew => CardType::Buff,
            EmpBlast | TargetingJammer | RadioSilence | TractorBeam => CardType::Debuff,
            MicroJump | EvasiveAction | Countermeasures | Scavenge | RedAlert => CardType::Tactic,
            FireControlDoctrine | BattleStations => CardType::Doctrine,
        }
    }

    pub fn target(self) -> CardTarget {
        use CardId::*;
        match self {
            Torpedo | OrbitalStrike | HeatSeeker | CoupDeGrace | CollisionCourse | Firebomb
            | EmpBlast | RadioSilence | TractorBeam => CardTarget::Enemy,
            EvasiveAction => CardTarget::Column,
            _ => CardTarget::None,
        }
    }

    /// Ob die Aufwertung den Wert erhöht (Angriff, Schild, Reparatur) statt die Kosten zu senken.
    fn upgrade_raises_value(self) -> bool {
        self.card_type() == CardType::Attack
            || matches!(
                self,
                CardId::EmergencyShields
                    | CardId::HoldTheLine
                    | CardId::RepairCrew
                    | CardId::BattleStations
            )
    }

    /// Kosten in Befehlspunkten.
    pub fn cost(self, upgraded: bool) -> u32 {
        use CardId::*;
        let base: u32 = match self {
            OrbitalStrike | CollisionCourse => 3,
            Torpedo | VolleyFire | HeatSeeker | CoupDeGrace | Bombardment | AblativeArmor
            | PowerSurge | EmpBlast | RadioSilence | TractorBeam | MicroJump | RedAlert
            | FireControlDoctrine | BattleStations => 2,
            _ => 1,
        };
        base.saturating_sub((upgraded && !self.upgrade_raises_value()) as u32)
    }

    /// Der Hauptwert der Karte: Schaden (Rammkurs: Zuschlag auf das Doppelte der ATK), Schild, Reparatur, ATK-Bonus,
    /// Panzerung; 0 bei Karten ohne Zahl.
    pub fn value(self, upgraded: bool) -> i32 {
        use CardId::*;
        let base = match self {
            Torpedo => 6,
            OrbitalStrike => 7,
            VolleyFire => 2,
            Barrage => 4,
            HeatSeeker => 5,
            CoupDeGrace => 3,
            Bombardment => 3,
            Firebomb => 2,
            EmergencyShields | HoldTheLine | Overcharge => 5,
            RepairCrew => 4,
            PowerSurge => 3,
            BattleStations => 2,
            AblativeArmor | FireControlDoctrine => 1,
            _ => 0,
        };
        base + if upgraded && self.upgrade_raises_value() {
            UPGRADE_VALUE
        } else {
            0
        }
    }
}

/// Zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zweimal die Kennkarte.
pub fn starter_deck(admiral: AdmiralId) -> Vec<DeckCard> {
    use CardId::*;
    let signature = admiral.signature_card();
    [
        Torpedo,
        Torpedo,
        Torpedo,
        EmergencyShields,
        EmergencyShields,
        EmergencyShields,
        RepairCrew,
        Scavenge,
        signature,
        signature,
    ]
    .map(DeckCard::new)
    .to_vec()
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
