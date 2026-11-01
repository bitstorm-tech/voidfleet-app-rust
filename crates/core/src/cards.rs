//! Kartendefinitionen, portiert aus den Godot-Ressourcen (`data/cards/*.tres`).
//! Karten werden über ihre Id übertragen; Namen und Effekttexte sind Übersetzungen und liegen im Client.

use serde::{Deserialize, Serialize};

use crate::board::LaneType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipClass {
    Fighter,
    Cruiser,
    Battleship,
    Support,
}

/// Befehlstypen in Auflösungsreihenfolge ([`COMMAND_ORDER`]); die Ordnung des Enums ist diese Reihenfolge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Reposition,
    Tactic,
    Buff,
    Special,
    Debuff,
    Attack,
}

/// Befehle wirken gleichzeitig. Damit kein Spieler im Vorteil ist, laufen sie nach Typ statt nach Spieler: erst
/// Positionen, dann Verstärkung, dann Schwächung, zuletzt Schaden. Zwei Befehle gleichen Typs betreffen nie dieselben
/// Schiffe; nur Rammstöße treffen beide Seiten und wirken deshalb gleichzeitig, und Funkstille entfernt nur die Marken
/// der Gegenseite.
pub const COMMAND_ORDER: [CommandType; 6] = [
    CommandType::Reposition,
    CommandType::Tactic,
    CommandType::Buff,
    CommandType::Special,
    CommandType::Debuff,
    CommandType::Attack,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode {
    None,
    AutoHighestHp,
    AutoLowestHp,
    AutoHighestAtk,
    AutoAll,
    AllFriendly,
    AllFriendlyStrike,
    AllEnemyStrike,
    AllEnemyCommand,
    AutoMoved,
    AutoArrived,
    AutoUnpaired,
    EnemyPower,
    ChooseFriendly,
    ChooseEnemy,
    ChooseEnemySlot,
    ChooseEnemyColumn,
}

/// Wohin ein verlegender Befehl sein Ziel bringt: in die andere Lane oder innerhalb derselben (Ausweichmanöver).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Moves {
    Lane,
    Within,
}

/// Die Schiffe: Deck-Schiffe, Tokens (entstehen erst auf dem Brett) und Flaggschiffe der Admirale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipId {
    HorizonCruiser,
    IronDreadnought,
    PlasmaFrigate,
    RepairDrone,
    ScrapRaider,
    SignalBooster,
    TitanCarrier,
    VoidInterceptor,
    // Tokens
    Drone,
    Decoy,
    // Flaggschiffe
    Bastion,
    Vanguard,
    Hive,
    Mirage,
    Pendulum,
}

/// Die Werte einer Schiffskarte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShipCard {
    pub id: ShipId,
    pub class: ShipClass,
    pub hp: i32,
    pub atk: i32,
    pub shield: i32,
}

/// Die Schiffe im Deck.
pub const SHIPS: [ShipId; 8] = [
    ShipId::HorizonCruiser,
    ShipId::IronDreadnought,
    ShipId::PlasmaFrigate,
    ShipId::RepairDrone,
    ShipId::ScrapRaider,
    ShipId::SignalBooster,
    ShipId::TitanCarrier,
    ShipId::VoidInterceptor,
];

/// Schiffe, die nicht im Deck liegen, sondern erst auf dem Brett entstehen: die Drohne (Titan Carrier, Hive) und das
/// Trugbild der Admiralsfähigkeit des Phantoms (steht eine Runde, greift nicht an, gibt Deckung).
pub const TOKENS: [ShipId; 2] = [ShipId::Drone, ShipId::Decoy];

/// Flaggschiffe: liegen in keinem Deck; jeder Admiral bringt seins mit und legt es in Runde 1 zusätzlich.
pub const FLAGSHIPS: [ShipId; 5] = [
    ShipId::Bastion,
    ShipId::Vanguard,
    ShipId::Hive,
    ShipId::Mirage,
    ShipId::Pendulum,
];

impl ShipId {
    /// Die Werte der Karte.
    pub const fn card(self) -> ShipCard {
        use ShipClass::*;
        let (class, hp, atk, shield) = match self {
            Self::HorizonCruiser => (Cruiser, 8, 2, 0),
            Self::IronDreadnought => (Battleship, 12, 2, 2),
            Self::PlasmaFrigate => (Cruiser, 6, 3, 0),
            Self::RepairDrone => (Support, 5, 0, 0),
            Self::ScrapRaider => (Fighter, 6, 2, 0),
            Self::SignalBooster => (Support, 5, 0, 0),
            Self::TitanCarrier => (Battleship, 11, 1, 0),
            Self::VoidInterceptor => (Fighter, 5, 3, 0),
            Self::Drone => (Fighter, 1, 1, 0),
            Self::Decoy => (Support, 1, 0, 0),
            Self::Bastion => (Battleship, 20, 1, 0),
            Self::Vanguard => (Cruiser, 16, 3, 0),
            Self::Hive => (Battleship, 18, 1, 0),
            Self::Mirage => (Fighter, 21, 3, 0),
            Self::Pendulum => (Battleship, 20, 2, 0),
        };
        ShipCard {
            id: self,
            class,
            hp,
            atk,
            shield,
        }
    }

    pub const fn hp(self) -> i32 {
        self.card().hp
    }

    pub const fn atk(self) -> i32 {
        self.card().atk
    }

    /// Ob das Schiff ein Token ist ([`TOKENS`]): Es liegt in keinem Deck und zählt weder zur Lane- noch zur Flottengrenze.
    pub const fn is_token(self) -> bool {
        matches!(self, Self::Drone | Self::Decoy)
    }

    /// Ob das Schiff ein Flaggschiff ist ([`FLAGSHIPS`]): Fällt es, hat seine Seite verloren.
    pub const fn is_flagship(self) -> bool {
        matches!(
            self,
            Self::Bastion | Self::Vanguard | Self::Hive | Self::Mirage | Self::Pendulum
        )
    }

    /// Kopien im Nachziehstapel: jedes Deck-Schiff zweimal, Tokens und Flaggschiffe nie.
    pub const fn deck_copies(self) -> usize {
        if self.is_token() || self.is_flagship() {
            0
        } else {
            2
        }
    }
}

/// Die Befehle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandId {
    EmergencyShields,
    EmpBlast,
    FlankingManeuver,
    Intercept,
    OrbitalStrike,
    Overcharge,
    PowerSurge,
    Recall,
    Scavenge,
    TargetingJammer,
    VolleyFire,
    Torpedo,
    Barrage,
    // Umgruppierung
    EvasiveAction,
    MicroJump,
    // Taktik
    RapidDeployment,
    DrawFire,
    // Verstärkung
    ShieldGrid,
    AblativeArmor,
    CoveringFire,
    DroneSquadron,
    // Spezial
    CollisionCourse,
    Countermeasures,
    CarteBlanche,
    // Schwächung
    RadioSilence,
    Minefield,
    TractorBeam,
    // Angriff
    HeatSeeker,
    CoupDeGrace,
    PiercingShot,
    Bombardment,
    Crossfire,
    /// Liegt in keinem Deck: kommt nur über den Nachschub, wenn ein eigener Scrap Raider Schrott geborgen hat.
    ScrapSalvage,
}

/// Die Werte einer Befehlskarte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandCard {
    pub id: CommandId,
    pub command_type: CommandType,
    pub target_mode: TargetMode,
    /// Bei Zielwahl eines eigenen Schiffs: nur Schiffe dieser Lane sind gültige Ziele.
    pub target_lane: Option<LaneType>,
    /// Bei Zielwahl eines Schiffs: nie ein Flaggschiff (Rammkurs, Traktorstrahl).
    pub not_flagship: bool,
    /// Bei Zielwahl eines eigenen Schiffs: nur Schiffe dieser Karte (Schrottverwertung).
    pub target_ship: Option<ShipId>,
    /// Verlegt das Zielschiff in einen frei gewählten Slot.
    pub moves: Option<Moves>,
}

/// Alle Befehle, auch die Schrottverwertung, die in keinem Deck liegt.
pub const COMMANDS: [CommandId; 33] = {
    use CommandId::*;
    [
        EmergencyShields,
        EmpBlast,
        FlankingManeuver,
        Intercept,
        OrbitalStrike,
        Overcharge,
        PowerSurge,
        Recall,
        Scavenge,
        TargetingJammer,
        VolleyFire,
        Torpedo,
        Barrage,
        EvasiveAction,
        MicroJump,
        RapidDeployment,
        DrawFire,
        ShieldGrid,
        AblativeArmor,
        CoveringFire,
        DroneSquadron,
        CollisionCourse,
        Countermeasures,
        CarteBlanche,
        RadioSilence,
        Minefield,
        TractorBeam,
        HeatSeeker,
        CoupDeGrace,
        PiercingShot,
        Bombardment,
        Crossfire,
        ScrapSalvage,
    ]
};

impl CommandId {
    /// Die Werte der Karte.
    pub const fn card(self) -> CommandCard {
        use CommandType as T;
        use TargetMode as M;
        let mut card = CommandCard {
            id: self,
            command_type: T::Special,
            target_mode: M::None,
            target_lane: None,
            not_flagship: false,
            target_ship: None,
            moves: None,
        };
        (card.command_type, card.target_mode) = match self {
            Self::EmergencyShields => (T::Buff, M::ChooseFriendly),
            Self::EmpBlast => (T::Debuff, M::AutoHighestAtk),
            Self::FlankingManeuver => (T::Tactic, M::ChooseFriendly),
            Self::Intercept => (T::Reposition, M::ChooseFriendly),
            Self::OrbitalStrike => (T::Attack, M::AutoHighestHp),
            Self::Overcharge => (T::Special, M::ChooseFriendly),
            Self::PowerSurge => (T::Buff, M::AllFriendlyStrike),
            Self::Recall => (T::Reposition, M::ChooseFriendly),
            Self::Scavenge => (T::Tactic, M::None),
            Self::TargetingJammer => (T::Debuff, M::AllEnemyStrike),
            Self::VolleyFire => (T::Attack, M::AutoAll),
            Self::Torpedo => (T::Attack, M::ChooseEnemySlot),
            Self::Barrage => (T::Debuff, M::AutoMoved),
            Self::EvasiveAction => (T::Reposition, M::ChooseFriendly),
            Self::MicroJump => (T::Reposition, M::ChooseFriendly),
            Self::RapidDeployment => (T::Tactic, M::None),
            Self::DrawFire => (T::Tactic, M::ChooseFriendly),
            Self::ShieldGrid => (T::Buff, M::AllFriendly),
            Self::AblativeArmor => (T::Buff, M::ChooseFriendly),
            Self::CoveringFire => (T::Buff, M::ChooseFriendly),
            Self::DroneSquadron => (T::Buff, M::None),
            Self::CollisionCourse => (T::Special, M::ChooseFriendly),
            Self::Countermeasures => (T::Special, M::None),
            Self::CarteBlanche => (T::Special, M::None),
            Self::RadioSilence => (T::Debuff, M::EnemyPower),
            Self::Minefield => (T::Debuff, M::AutoArrived),
            Self::TractorBeam => (T::Debuff, M::ChooseEnemy),
            Self::HeatSeeker => (T::Attack, M::ChooseEnemy),
            Self::CoupDeGrace => (T::Attack, M::AutoLowestHp),
            Self::PiercingShot => (T::Attack, M::ChooseEnemyColumn),
            Self::Bombardment => (T::Attack, M::AllEnemyCommand),
            Self::Crossfire => (T::Attack, M::AutoUnpaired),
            Self::ScrapSalvage => (T::Buff, M::ChooseFriendly),
        };
        match self {
            Self::FlankingManeuver => card.moves = Some(Moves::Lane),
            Self::Intercept => {
                card.target_lane = Some(LaneType::Command);
                card.moves = Some(Moves::Lane);
            }
            Self::Recall => {
                card.target_lane = Some(LaneType::Strike);
                card.moves = Some(Moves::Lane);
            }
            Self::EvasiveAction => card.moves = Some(Moves::Within),
            Self::CollisionCourse => {
                card.target_lane = Some(LaneType::Strike);
                card.not_flagship = true;
            }
            Self::TractorBeam => card.not_flagship = true,
            Self::ScrapSalvage => card.target_ship = Some(ShipId::ScrapRaider),
            _ => {}
        }
        card
    }

    pub const fn command_type(self) -> CommandType {
        self.card().command_type
    }

    pub const fn target_mode(self) -> TargetMode {
        self.card().target_mode
    }

    /// Befehle mit Zielwahl: ein eigenes Schiff, ein gegnerisches Schiff oder ein Slot der Gegnerseite.
    pub const fn needs_target(self) -> bool {
        matches!(self.target_mode(), TargetMode::ChooseFriendly) || self.targets_enemy_side()
    }

    /// Ob das Ziel auf der Gegnerseite liegt: ein gegnerisches Schiff oder ein Slot der Gegnerseite.
    pub const fn targets_enemy_side(self) -> bool {
        self.targets_enemy_ship() || self.targets_enemy_slot()
    }

    /// Wärmesucher, Traktorstrahl: Ziel ist ein gegnerisches Schiff, das beim Planen auf dem Brett steht. Der Befehl
    /// folgt ihm, wenn es sich in der Runde bewegt.
    pub const fn targets_enemy_ship(self) -> bool {
        matches!(self.target_mode(), TargetMode::ChooseEnemy)
    }

    /// Torpedo, Durchschlag: Ziel ist ein Slot der Gegnerseite, belegt oder leer; getroffen wird, wer nach allen
    /// Bewegungen dort steht. Beim Durchschlag zählt nur die Slot-Nummer ([`Self::targets_enemy_column`]).
    pub const fn targets_enemy_slot(self) -> bool {
        matches!(self.target_mode(), TargetMode::ChooseEnemySlot) || self.targets_enemy_column()
    }

    /// Durchschlag: Das Ziel ist eine Slot-Nummer der Gegnerseite, also ihr Strike- und ihr Command-Slot.
    pub const fn targets_enemy_column(self) -> bool {
        matches!(self.target_mode(), TargetMode::ChooseEnemyColumn)
    }

    /// Ob ein Ziel in dieser Lane für den Befehl gültig ist.
    pub fn can_target_lane(self, lane: LaneType) -> bool {
        self.needs_target() && self.card().target_lane.is_none_or(|only| only == lane)
    }

    /// Ob der Befehl dieses Schiff zum Ziel haben darf: manche nie ein Flaggschiff, manche nur eine Karte.
    pub fn can_target_ship(self, ship: ShipId) -> bool {
        let card = self.card();
        !(card.not_flagship && ship.is_flagship())
            && card.target_ship.is_none_or(|only| only == ship)
    }

    /// Die Lane, in die ein verlegender Befehl sein Ziel bringt: die andere oder (Ausweichmanöver) dieselbe.
    pub fn destination_lane(self, from: LaneType) -> LaneType {
        match self.card().moves {
            Some(Moves::Within) => from,
            _ => from.other(),
        }
    }

    /// Kopien im Nachziehstapel: Torpedo und Sperrfeuer zweimal, jeder andere Befehl einmal, die Schrottverwertung nie.
    pub const fn deck_copies(self) -> usize {
        match self {
            Self::ScrapSalvage => 0,
            Self::Torpedo | Self::Barrage => 2,
            _ => 1,
        }
    }
}

/// Schnellverlegung: Mit diesem Befehl darf in der Runde ein Schiff mehr gelegt werden.
pub fn extra_deploys(command: Option<CommandId>) -> usize {
    usize::from(command == Some(CommandId::RapidDeployment))
}

/// Freie Hand: Mit diesem Befehl gibt es in der Runde das Grundmanöver und die Admiralsfähigkeit.
pub fn allows_maneuver_and_power(command: Option<CommandId>) -> bool {
    command == Some(CommandId::CarteBlanche)
}
