//! Nachrichten zwischen Client und Server über die WebSocket-Verbindung. Alles ist aus Sicht des Empfängers
//! formuliert: [`Side::Player`] ist die eigene Seite, [`Side::Enemy`] die des Gegners. Karten werden über ihre Id
//! übertragen, Fehler als [`ErrorCode`]; formuliert wird im Client.

use serde::{Deserialize, Serialize};

use crate::admirals::{AdmiralId, PowerId};
use crate::board::{BoardSnapshot, LaneType, Side, Sides, SlotRef, SlotTarget};
use crate::cards::{CommandId, ShipId};

/// Ein Schiff von der Hand, das auf einen eigenen Slot gelegt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Deploy {
    pub hand_index: usize,
    pub target: SlotTarget,
}

/// Der Befehl im Zug, per Hand-Index. `target` liegt auf der eigenen Seite, außer bei Befehlen mit Ziel auf der
/// Gegnerseite ([`CommandId::targets_enemy_side`]). Verlegt der Befehl sein Ziel, ist `destination` der Slot-Index in
/// der Ziel-Lane ([`CommandId::destination_lane`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandPlan {
    pub hand_index: usize,
    pub target: Option<SlotTarget>,
    pub destination: Option<usize>,
}

/// Grundmanöver: ein eigenes Schiff, das schon auf dem Brett steht, in den freien Slot `destination` der anderen
/// oder derselben Lane; dafür feuert es in dieser Runde nicht. Die Mirage darf auch in einen belegten eigenen Slot
/// (Phasentausch), dann feuern beide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Maneuver {
    pub ship: SlotTarget,
    pub destination: SlotTarget,
}

/// Die Admiralsfähigkeit im Zug; was `target` sein darf, sagt [`PowerId::target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PowerPlan {
    pub target: SlotTarget,
}

/// Der Zug einer Runde: bis zu `max_deploys` Schiffe und 1 Befehl, jeweils per Hand-Index, dazu entweder ein
/// Grundmanöver oder die Admiralsfähigkeit (mit Freier Hand beides). Eigene Ziele beziehen sich auf die Stellung vor
/// dem Manöver: Ein Befehl auf das manövrierte Schiff folgt ihm. Ein Slot der Gegnerseite bleibt, was er ist:
/// Getroffen wird, wer nach allen Bewegungen dort steht. Im Kampf feuert nur, wer einen Feuerbefehl hat.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TurnPlan {
    pub ships: Vec<Deploy>,
    pub command: Option<CommandPlan>,
    pub maneuver: Option<Maneuver>,
    /// Runde 1: der Slot, auf dem das eigene Flaggschiff landet (Pflicht); danach immer `None`.
    pub flagship: Option<SlotTarget>,
    /// Die Admiralsfähigkeit statt des Grundmanövers.
    pub power: Option<PowerPlan>,
    /// Pendulum: der Slot des gegnerischen Schiffs (Stellung beim Planen), das sie markiert.
    pub mark: Option<SlotTarget>,
    /// Feuerbefehle: je eigenem Kampfschiff höchstens einer; ein Schiff ohne Feuerbefehl feuert nicht.
    pub fire: Vec<FireOrder>,
}

/// Ein Feuerbefehl: `ship` ist ein eigenes Kampfschiff (Slot beim Planen, auch eines, das in dieser Runde landet,
/// nicht das Schiff des Grundmanövers); der Befehl folgt ihm, wenn es sich bewegt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FireOrder {
    pub ship: SlotTarget,
    pub target: FireTarget,
}

/// Das Ziel eines Feuerbefehls auf der Gegnerseite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireTarget {
    /// Spaltenfeuer (Slot-Index, auch eine leere Spalte): Die Spalte bleibt. Der Schuss trifft das vorderste Schiff,
    /// das dort noch steht, erst Strike, dann Command; steht dort keins mehr, verpufft er.
    Column(usize),
    /// Schiffsfeuer (Slot des Schiffs beim Planen, auch eines gedeckten): Der Schuss folgt dem Schiff und verpufft,
    /// wenn es dann gedeckt, schon zerstört oder nicht mehr auf dem Brett ist.
    Ship(SlotTarget),
}

/// Ein Schiff an einer Stelle des Bretts; die Karte steht dabei, weil der Slot danach leer sein kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShipRef {
    pub side: Side,
    pub lane: LaneType,
    pub slot_index: usize,
    pub card_id: ShipId,
}

impl ShipRef {
    pub const fn new(at: SlotRef, card_id: ShipId) -> Self {
        ShipRef {
            side: at.side,
            lane: at.lane,
            slot_index: at.slot_index,
            card_id,
        }
    }

    /// Der Slot des Schiffs.
    pub const fn slot(self) -> SlotRef {
        SlotRef::new(self.side, self.lane, self.slot_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchResult {
    Won,
    Lost,
    Draw,
}

/// Was man vom Gegner weiß: ob er abgegeben hat, wie viele Karten er hält und welche Befehle er schon gespielt hat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpponentView {
    /// Sein Admiral; `None`, solange noch nicht beide gewählt haben (die Wahl ist verdeckt).
    pub admiral: Option<AdmiralId>,
    pub submitted: bool,
    pub ship_hand_size: usize,
    pub command_hand_size: usize,
    pub ship_deck_size: usize,
    pub command_deck_size: usize,
    /// Der Computergegner sitzt auf diesem Platz.
    pub bot: bool,
    /// Seine aufgedeckten Befehle in der Reihenfolge, in der er sie gespielt hat.
    pub played_commands: Vec<CommandId>,
}

/// Der Nachschub nach jeder Runde: bis zu 3 Schiffe und 3 Befehle aus den Decks, davon nimmt der Spieler je eines auf
/// die Hand. Hat ein eigener Scrap Raider Schrott geborgen, ist der erste Befehl die Schrottverwertung.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DraftOffer {
    pub ships: Vec<ShipId>,
    pub commands: Vec<CommandId>,
}

/// Die Wahl aus dem Nachschub: Indizes in [`DraftOffer::ships`] und [`DraftOffer::commands`]; jeder ist Pflicht,
/// sobald es in seiner Reihe etwas zu wählen gibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DraftPick {
    pub ship: Option<usize>,
    pub command: Option<usize>,
}

/// Die Sicht eines Spielers auf das Match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub match_id: String,
    pub round: u32,
    /// Der eigene Admiral; `None`, solange noch nicht gewählt.
    pub admiral: Option<AdmiralId>,
    pub board: BoardSnapshot,
    /// So viele Drohnen hat die Hive je Seite in diesem Match schon gestartet.
    pub hive_launches: Sides<u32>,
    pub ship_hand: Vec<ShipId>,
    pub command_hand: Vec<CommandId>,
    pub ship_deck_size: usize,
    pub command_deck_size: usize,
    /// Der abgegebene Zug dieser Runde; `None`, solange noch geplant wird.
    pub submitted_plan: Option<TurnPlan>,
    /// Der Nachschub nach der letzten Runde, solange nicht gewählt; erst danach wird geplant.
    pub draft: Option<DraftOffer>,
    /// Der Gegner, sobald er beigetreten ist.
    pub opponent: Option<OpponentView>,
    /// Gesetzt, sobald das Match entschieden ist; danach gibt es keine Züge mehr.
    pub result: Option<MatchResult>,
    /// Das Match endete, weil ein Spieler aufgegeben hat; wer, folgt aus `result`.
    pub surrendered: bool,
}

/// HP und Schild eines Schiffs nach einem Ereignis, damit der Client die Runde Schritt für Schritt zeigen kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShipStats {
    pub hp: i32,
    pub shield: i32,
}

/// Was ein Befehl an einem Schiff bewirkt (`ship` im Ereignis [`RoundEvent::Effect`], Stellung vor der Wirkung).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandEffect {
    Damage {
        amount: i32,
    },
    Attack {
        delta: i32,
    },
    /// `permanent`: Ablativpanzerung, verfällt nicht am Rundenende.
    Shield {
        amount: i32,
        permanent: bool,
    },
    Disable,
    Heal {
        amount: i32,
    },
    /// Verlegt das Schiff, auch ein gegnerisches (Traktorstrahl).
    Move {
        to: SlotTarget,
    },
    /// Kurzsprung: Das Schiff verlässt das Brett bis zum Cleanup ([`RoundEvent::Return`]).
    Jump,
    /// Ablenkung: zieht diese Runde das Feuer auf sich.
    Taunt,
    /// Drohnenstaffel: `ship` ist die gerade gestartete Drohne.
    Spawn,
    /// Rammkurs: `ship` rammt `target`; `dealt` ist der Schaden am Ziel, `taken` der am Rammer, beide Nennwerte vor
    /// dem Schild; `after` im Ereignis gilt dem Rammer, `target_after` dem Ziel.
    Ram {
        target: ShipRef,
        dealt: i32,
        taken: i32,
        target_after: ShipStats,
    },
    /// Funkstille: Die gegnerische Admiralsfähigkeit `power` auf diesem Schiff fällt weg; beim Trugbild verschwindet es.
    Jam {
        power: PowerId,
    },
}

/// Was eine Schiffs-Fähigkeit an einem Schiff bewirkt; `Salvage`: Der Scrap Raider birgt Schrott.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AbilityEffect {
    Heal { amount: i32 },
    Damage { amount: i32 },
    Shield { amount: i32 },
    Salvage,
}

/// Warum ein Schuss verpufft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Miss {
    /// Spaltenfeuer: In der Spalte steht kein Schiff mehr; `cleared`: Dort ist in dieser Runde eins gefallen, sonst
    /// war sie leer.
    Column { column: usize, cleared: bool },
    /// Schiffsfeuer (auch Drohnen im Schwarmangriff): Das Ziel ist schon zerstört, nicht mehr auf dem Brett (`None`)
    /// oder gedeckt (`covered`).
    Target {
        target: Option<ShipRef>,
        covered: bool,
    },
}

/// Was in einer Runde aufgedeckt und aufgelöst wurde, in Auflösungsreihenfolge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoundEvent {
    /// Auch das Flaggschiff in Runde 1.
    Deploy {
        side: Side,
        card_id: ShipId,
        target: SlotTarget,
    },
    Pass {
        side: Side,
    },
    /// Die Admiralsfähigkeit, gleich nach den Deploys: `target` ist das gewählte Schiff bzw. der Slot des Trugbilds.
    Power {
        side: Side,
        power: PowerId,
        target: SlotRef,
    },
    /// Trugbild: erscheint nach `Power` im gewählten Strike-Slot und verschwindet am Rundenende (`Vanish`).
    Decoy {
        ship: ShipRef,
    },
    /// Pendulum: markiert nach den Deploys ein gegnerisches Schiff; die Railgun feuert in der nächsten Runde.
    Mark {
        side: Side,
        ship: ShipRef,
    },
    /// Grundmanöver: `ship` ist die Stelle davor; das Schiff feuert in dieser Runde nicht.
    Maneuver {
        side: Side,
        ship: ShipRef,
        to: SlotTarget,
    },
    /// Phasentausch statt `Maneuver`: Die Mirage (`a`) und das Schiff `b` tauschen die Plätze (Stellung davor).
    Swap {
        side: Side,
        a: ShipRef,
        b: ShipRef,
    },
    /// `target` ist der Slot nach dem Manöver, beim Torpedo der gewählte Slot der Gegnerseite, beim Durchschlag der
    /// Strike-Slot der Slot-Nummer, bei Wärmesucher und Traktorstrahl der Slot des erfassten Schiffs beim Aufdecken
    /// (`None`, wenn es nicht mehr auf dem Brett steht).
    Command {
        side: Side,
        card_id: CommandId,
        target: Option<SlotRef>,
    },
    Effect {
        side: Side,
        card_id: CommandId,
        ship: ShipRef,
        effect: CommandEffect,
        after: ShipStats,
    },
    /// Plündern und Gegenmaßnahmen ohne gegnerischen Angriff: was auf der Hand gelandet ist.
    Draw {
        side: Side,
        card_id: CommandId,
        ships: u32,
        commands: u32,
    },
    NoEffect {
        side: Side,
        card_id: CommandId,
    },
    /// Gegenmaßnahmen von `side`: Der Angriffs- oder Schwächungsbefehl `countered` der Gegenseite verpufft.
    Countered {
        side: Side,
        card_id: CommandId,
        countered: CommandId,
    },
    /// Befehl, der den Zug selbst verändert hat (Schnellverlegung, Freie Hand).
    Rule {
        side: Side,
        card_id: CommandId,
    },
    /// Torpedo oder Durchschlag in einen Slot, in dem nach allen Bewegungen kein Schiff steht.
    Miss {
        side: Side,
        card_id: CommandId,
        target: SlotRef,
    },
    /// Fähigkeit eines Schiffs (`source`) an einem Schiff: Plasma Frigate, Repair Drone, Scrap Raider, Wächterschild.
    Ability {
        source: ShipRef,
        ship: ShipRef,
        effect: AbilityEffect,
        after: ShipStats,
    },
    /// Drohnenstart: Titan Carrier beim Deploy, Hive am Rundenende.
    Spawn {
        source: ShipRef,
        ship: ShipRef,
    },
    /// Railgun der Pendulum zu Kampfbeginn auf das markierte Schiff; `blocked`, wenn es gedeckt war.
    Railgun {
        source: ShipRef,
        target: ShipRef,
        amount: i32,
        after: ShipStats,
        blocked: bool,
    },
    /// Taktvorsprung: Das Schiff fiel durch einen Schuss, der vor allen anderen kam, und feuert nicht mehr.
    Silenced {
        ship: ShipRef,
    },
    /// Ein Schuss im Kampf; `focus`/`swarm`/`head_start`: verstärkt, gelenkt bzw. beschleunigt von der
    /// Admiralsfähigkeit; `through`: Spaltenfeuer, das durchging; `taunt`: Die Ablenkung hat ihn auf sich gezogen.
    Attack {
        attacker: ShipRef,
        target: ShipRef,
        amount: i32,
        after: ShipStats,
        focus: bool,
        swarm: bool,
        head_start: bool,
        through: bool,
        taunt: bool,
    },
    /// Ein Schuss, der verpufft.
    Fizzle {
        attacker: ShipRef,
        head_start: bool,
        miss: Miss,
    },
    /// Gegenfeuer nach dem Kampf: `ship` hat `source` getroffen und nimmt dafür Schaden.
    Counterfire {
        source: ShipRef,
        ship: ShipRef,
        amount: i32,
        after: ShipStats,
    },
    SelfDamage {
        ship: ShipRef,
        amount: i32,
        after: ShipStats,
    },
    Destroyed {
        ship: ShipRef,
    },
    /// Kurzsprung: Das Schiff kehrt zu Beginn des Cleanups in seinen Slot zurück.
    Return {
        ship: ShipRef,
        after: ShipStats,
    },
    /// Das Trugbild verschwindet am Rundenende (sofern es nicht zerstört wurde).
    Vanish {
        ship: ShipRef,
    },
}

/// Nachrichten vom Client. Die Verbindung selbst ist der Beitritt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Wählt den eigenen Admiral, einmal und vor Runde 1.
    ChooseAdmiral {
        admiral: AdmiralId,
    },
    /// Wählt aus dem Nachschub, bevor der Zug der Runde abgegeben werden kann.
    Draft {
        pick: DraftPick,
    },
    Submit {
        plan: TurnPlan,
    },
    /// Setzt den Computergegner auf den freien Platz.
    AddBot,
    /// Macht seinen Platz für einen Mitspieler frei, solange der Spieler seinen Admiral noch nicht gewählt hat.
    RemoveBot,
    /// Gibt das Match auf; der Gegner gewinnt sofort.
    Surrender,
}

/// Fehler werden als Code übertragen, der Client formuliert sie in seiner Sprache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    MatchFull,
    MatchOver,
    MatchStarted,
    OpponentMissing,
    AdmiralRequired,
    AdmiralAlreadyChosen,
    OpponentChoosing,
    DraftPending,
    DraftUnavailable,
    DraftInvalid,
    FlagshipRequired,
    FlagshipNotAllowed,
    PowerAndManeuver,
    PowerTargetInvalid,
    MarkUnavailable,
    MarkTargetInvalid,
    SwapNotAllowed,
    AlreadySubmitted,
    ShipNotInHand,
    TooManyShips,
    SlotUnavailable,
    CommandNotInHand,
    TargetRequired,
    TargetNotAllowed,
    TargetNotOwnShip,
    TargetWrongLane,
    TargetNotEnemySlot,
    TargetNotEnemyShip,
    TargetWrongShip,
    DestinationRequired,
    DestinationNotAllowed,
    DestinationUnavailable,
    ManeuverNotOwnShip,
    ManeuverDestinationUnavailable,
    ManeuverConflict,
    FireShipInvalid,
    FireTargetInvalid,
}

/// Nachrichten vom Server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    State { view: Box<PlayerView> },
    Round { round: u32, events: Vec<RoundEvent> },
    Error { code: ErrorCode },
}

/// Spiegelt einen Wert für den zweiten Spieler: Jede Seite, auch in verschachtelten Schiffen, wechselt.
pub trait Mirror {
    fn mirrored(self) -> Self;
}

impl Mirror for Side {
    fn mirrored(self) -> Self {
        self.opposite()
    }
}

impl Mirror for SlotRef {
    fn mirrored(self) -> Self {
        SlotRef {
            side: self.side.opposite(),
            ..self
        }
    }
}

impl Mirror for ShipRef {
    fn mirrored(self) -> Self {
        ShipRef {
            side: self.side.opposite(),
            ..self
        }
    }
}

impl<T: Mirror> Mirror for Option<T> {
    fn mirrored(self) -> Self {
        self.map(Mirror::mirrored)
    }
}

impl Mirror for CommandEffect {
    fn mirrored(self) -> Self {
        match self {
            CommandEffect::Ram {
                target,
                dealt,
                taken,
                target_after,
            } => CommandEffect::Ram {
                target: target.mirrored(),
                dealt,
                taken,
                target_after,
            },
            other => other,
        }
    }
}

impl Mirror for Miss {
    fn mirrored(self) -> Self {
        match self {
            Miss::Target { target, covered } => Miss::Target {
                target: target.mirrored(),
                covered,
            },
            column => column,
        }
    }
}

impl Mirror for RoundEvent {
    fn mirrored(mut self) -> Self {
        use RoundEvent as E;
        fn flip<T: Mirror + Copy>(value: &mut T) {
            *value = value.mirrored();
        }
        match &mut self {
            E::Deploy { side, .. }
            | E::Pass { side }
            | E::Draw { side, .. }
            | E::NoEffect { side, .. }
            | E::Countered { side, .. }
            | E::Rule { side, .. } => flip(side),
            E::Power { side, target, .. } | E::Miss { side, target, .. } => {
                flip(side);
                flip(target);
            }
            E::Command { side, target, .. } => {
                flip(side);
                flip(target);
            }
            E::Mark { side, ship } | E::Maneuver { side, ship, .. } => {
                flip(side);
                flip(ship);
            }
            E::Swap { side, a, b } => {
                flip(side);
                flip(a);
                flip(b);
            }
            E::Effect {
                side, ship, effect, ..
            } => {
                flip(side);
                flip(ship);
                flip(effect);
            }
            E::Decoy { ship }
            | E::Silenced { ship }
            | E::SelfDamage { ship, .. }
            | E::Destroyed { ship }
            | E::Return { ship, .. }
            | E::Vanish { ship } => flip(ship),
            E::Ability { source, ship, .. }
            | E::Spawn { source, ship }
            | E::Counterfire { source, ship, .. }
            | E::Railgun {
                source,
                target: ship,
                ..
            }
            | E::Attack {
                attacker: source,
                target: ship,
                ..
            } => {
                flip(source);
                flip(ship);
            }
            E::Fizzle { attacker, miss, .. } => {
                flip(attacker);
                flip(miss);
            }
        }
        self
    }
}
