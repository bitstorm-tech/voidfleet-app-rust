//! Das Spielbrett: zwei Lanes pro Seite, Strike vorne, Command hinten. Kennt als einzige Stelle die
//! Platzierungsregeln; die Slots tragen den Live-Zustand der Schiffe (HP, Schild, Rundenmodifikatoren).
//! [`Side::Player`] ist immer die Seite des Betrachters; der Server spiegelt das Brett für den zweiten Spieler.

use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use crate::admirals::{ABILITY_VALUES, PowerId};
use crate::cards::{CommandId, ShipId, extra_deploys};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Player,
    Enemy,
}

impl Side {
    /// Beide Seiten in fester Reihenfolge.
    pub const ALL: [Side; 2] = [Side::Player, Side::Enemy];

    pub const fn opposite(self) -> Side {
        match self {
            Side::Player => Side::Enemy,
            Side::Enemy => Side::Player,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneType {
    Strike,
    Command,
}

impl LaneType {
    /// Strike vor Command, die Reihenfolge von [`Board::ships`].
    pub const ALL: [LaneType; 2] = [LaneType::Strike, LaneType::Command];

    pub const fn other(self) -> LaneType {
        match self {
            LaneType::Strike => LaneType::Command,
            LaneType::Command => LaneType::Strike,
        }
    }
}

/// Je ein Wert für beide Seiten, indizierbar mit [`Side`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sides<T> {
    pub player: T,
    pub enemy: T,
}

impl<T> Sides<T> {
    /// Vertauscht die Seiten (Sicht des anderen Spielers).
    pub fn mirrored(self) -> Self {
        Sides {
            player: self.enemy,
            enemy: self.player,
        }
    }
}

impl<T> Index<Side> for Sides<T> {
    type Output = T;

    fn index(&self, side: Side) -> &T {
        match side {
            Side::Player => &self.player,
            Side::Enemy => &self.enemy,
        }
    }
}

impl<T> IndexMut<Side> for Sides<T> {
    fn index_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Player => &mut self.player,
            Side::Enemy => &mut self.enemy,
        }
    }
}

/// Je ein Wert für beide Lanes einer Seite, indizierbar mit [`LaneType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Lanes<T> {
    pub strike: T,
    pub command: T,
}

impl<T> Index<LaneType> for Lanes<T> {
    type Output = T;

    fn index(&self, lane: LaneType) -> &T {
        match lane {
            LaneType::Strike => &self.strike,
            LaneType::Command => &self.command,
        }
    }
}

impl<T> IndexMut<LaneType> for Lanes<T> {
    fn index_mut(&mut self, lane: LaneType) -> &mut T {
        match lane {
            LaneType::Strike => &mut self.strike,
            LaneType::Command => &mut self.command,
        }
    }
}

/// Ein Slot auf einer Seite des Bretts, die sich aus dem Zusammenhang ergibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotTarget {
    pub lane: LaneType,
    pub slot_index: usize,
}

impl SlotTarget {
    pub const fn new(lane: LaneType, slot_index: usize) -> Self {
        SlotTarget { lane, slot_index }
    }

    /// Derselbe Slot auf der Seite `side`.
    pub const fn on(self, side: Side) -> SlotRef {
        SlotRef {
            side,
            lane: self.lane,
            slot_index: self.slot_index,
        }
    }
}

/// Ein Slot samt Seite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotRef {
    pub side: Side,
    pub lane: LaneType,
    pub slot_index: usize,
}

impl SlotRef {
    pub const fn new(side: Side, lane: LaneType, slot_index: usize) -> Self {
        SlotRef {
            side,
            lane,
            slot_index,
        }
    }

    /// Der Slot ohne Seite.
    pub const fn target(self) -> SlotTarget {
        SlotTarget {
            lane: self.lane,
            slot_index: self.slot_index,
        }
    }

    /// Der Slot gleicher Nummer in der Lane `lane` derselben Seite.
    pub const fn in_lane(self, lane: LaneType) -> SlotRef {
        SlotRef { lane, ..self }
    }
}

/// Alle Slots einer Lane in Index-Reihenfolge.
pub fn lane_slots(side: Side, lane: LaneType) -> impl Iterator<Item = SlotRef> {
    (0..SLOT_COUNT).map(move |index| SlotRef::new(side, lane, index))
}

/// Alle Slots einer Seite, Strike vor Command, jeweils nach Index.
pub fn side_slots(side: Side) -> impl Iterator<Item = SlotRef> {
    LaneType::ALL
        .into_iter()
        .flat_map(move |lane| lane_slots(side, lane))
}

/// Serialisierbarer Zustand eines Schiffs zwischen den Runden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotState {
    pub card_id: ShipId,
    pub hp: i32,
    pub shield: i32,
    /// Dauerhafter ATK-Bonus des Scrap Raiders ([`Ship::scrap_bonus`]).
    pub scrap_bonus: i32,
    /// Von der gegnerischen Pendulum markiert: Ihre Railgun feuert in der nächsten Runde auf dieses Schiff.
    pub marked: bool,
}

/// Serialisierbarer Brettzustand, je Seite und Lane die Slots in Index-Reihenfolge.
pub type BoardSnapshot = Sides<Lanes<[Option<SlotState>; SLOT_COUNT]>>;

/// Ein gebundener Feuerbefehl ([`Ship::aim`]): Spaltenfeuer auf eine Spalte der Gegnerseite oder Schiffsfeuer auf
/// die Kennung ([`Ship::uid`]) eines gegnerischen Schiffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aim {
    Column(usize),
    Ship(u32),
}

/// Eine eingeplante Bewegung: der Slot des Schiffs und der Slot, in den es geht (beim Phasentausch belegt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlannedMove {
    pub from: SlotRef,
    pub to: SlotRef,
}

pub const SLOT_COUNT: usize = 6;
pub const MAX_SHIPS_PER_LANE: usize = 4;
pub const MAX_SHIPS_PER_SIDE: usize = 6;

/// Aufstellung: In Runde 1 darf jede Seite zwei Schiffe legen, danach eines; mit Schnellverlegung eines mehr.
pub fn max_deploys(round: u32, command: Option<CommandId>) -> usize {
    (if round == 1 { 2 } else { 1 }) + extra_deploys(command)
}

/// Ab dieser Runde bekommen alle Kampfschiffe Overdrive, damit Partien nicht im Patt enden.
const OVERDRIVE_START: u32 = 8;
/// Alle so viele Runden steigt der Overdrive um 1 ATK.
const OVERDRIVE_STEP: u32 = 4;

/// ATK-Bonus aller Kampfschiffe in dieser Runde: +1 ab Runde 8, +2 ab Runde 12, +3 ab Runde 16 usw.
pub fn overdrive_bonus(round: u32) -> i32 {
    if round < OVERDRIVE_START {
        0
    } else {
        ((round - OVERDRIVE_START) / OVERDRIVE_STEP + 1) as i32
    }
}

/// Ein Schiff auf dem Brett samt Live-Zustand; es zieht bei jeder Bewegung mit allen Werten um.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ship {
    pub card: ShipId,
    /// Kennung des Schiffs, solange es auf dem Brett steht; Schiffsfeuer verweist darauf innerhalb einer Runde.
    pub uid: u32,
    pub hp: i32,
    pub shield: i32,
    /// Scrap Raider: +1 ATK je gespielter Schrottverwertung; bleibt bis zu seiner Zerstörung.
    pub scrap_bonus: i32,
    /// Pendulum: Die Markierung folgt dem Schiff und hält über das Rundenende, bis die Railgun gefeuert hat.
    pub marked: bool,

    // Nur für die laufende Runde, im Cleanup zurückgesetzt; gehören nicht in den Snapshot.
    pub attack_bonus: i32,
    pub shield_bonus: i32,
    pub disabled: bool,
    /// Grundmanöver: Das Schiff hat diese Runde den Slot gewechselt und feuert dafür nicht.
    pub maneuvered: bool,
    /// Hat diese Runde den Slot gewechselt, per Grundmanöver oder Befehl (Ziel des Sperrfeuers).
    pub moved: bool,
    pub self_damage_after_combat: i32,
    /// Admiralsfähigkeiten, die in dieser Runde auf dem Schiff liegen; sie folgen ihm.
    pub powers: Vec<PowerId>,
    /// Kam in dieser Runde aufs Brett: Deploy samt Flaggschiff, Drohne des Titan Carriers, Trugbild, Drohnenstaffel.
    pub arrived: bool,
    /// Ablenkung: Gegnerische Schützen nehmen diese Runde dieses Schiff, solange sie es beschießen können.
    pub taunting: bool,
    /// Gegnerische Befehle, die dieses Schiff erfasst haben (Wärmesucher, Traktorstrahl); sie folgen ihm.
    pub locks: Vec<CommandId>,
    /// Feuerbefehl dieser Runde; `None`: Es feuert nicht.
    pub aim: Option<Aim>,
}

impl Ship {
    fn new(card: ShipId, uid: u32) -> Self {
        let stats = card.card();
        Ship {
            card,
            uid,
            hp: stats.hp,
            shield: stats.shield,
            scrap_bonus: 0,
            marked: false,
            attack_bonus: 0,
            shield_bonus: 0,
            disabled: false,
            maneuvered: false,
            moved: false,
            self_damage_after_combat: 0,
            powers: Vec::new(),
            arrived: false,
            taunting: false,
            locks: Vec::new(),
            aim: None,
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0
    }

    /// Schaden geht erst auf den Schild, der Rest auf die HP.
    pub fn damage(&mut self, amount: i32) {
        let absorbed = self.shield.min(amount);
        self.shield -= absorbed;
        self.shield_bonus = self.shield_bonus.min(self.shield);
        self.hp -= amount - absorbed;
    }

    /// Heilt höchstens bis zur HP der Karte.
    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.card.hp());
    }

    /// Schild für diese Runde; was davon am Rundenende noch steht, verfällt.
    pub fn add_shield(&mut self, amount: i32) {
        self.shield += amount;
        self.shield_bonus += amount;
    }

    /// Rundenmodifikatoren zurücksetzen; ein Bonus-Schild verfällt, soweit er noch steht.
    pub fn end_round(&mut self) {
        self.shield -= self.shield_bonus;
        self.attack_bonus = 0;
        self.shield_bonus = 0;
        self.disabled = false;
        self.maneuvered = false;
        self.moved = false;
        self.self_damage_after_combat = 0;
        self.powers.clear();
        self.arrived = false;
        self.taunting = false;
        self.locks.clear();
        self.aim = None;
    }

    pub fn state(&self) -> SlotState {
        SlotState {
            card_id: self.card,
            hp: self.hp,
            shield: self.shield,
            scrap_bonus: self.scrap_bonus,
            marked: self.marked,
        }
    }
}

/// Kurzsprung: ein Schiff, das in dieser Runde das Brett verlassen hat, samt Zustand; `slot` ist der Slot, in den es
/// im Cleanup zurückkehrt und der bis dahin gesperrt ist ([`Board::is_free`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jump {
    pub slot: SlotRef,
    pub ship: Ship,
}

/// Das Spielbrett; mit einem [`SlotRef`] indizierbar (`board[at]` ist der Slot, `None` wenn leer).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Board {
    slots: Sides<Lanes<[Option<Ship>; SLOT_COUNT]>>,
    /// ATK-Bonus aller Kampfschiffe aus der Rundenzahl ([`overdrive_bonus`]); gehört nicht in den Snapshot.
    pub overdrive: i32,
    /// So viele Drohnen hat die Hive je Seite in diesem Match gestartet; gehört nicht in den Snapshot.
    pub hive_launches: Sides<u32>,
    /// Kurzsprung: Schiffe, die in dieser Runde das Brett verlassen haben; zwischen den Runden leer.
    pub jumping: Vec<Jump>,
    /// Zuletzt vergebene Schiffskennung ([`Ship::uid`]).
    last_uid: u32,
}

impl Index<SlotRef> for Board {
    type Output = Option<Ship>;

    fn index(&self, at: SlotRef) -> &Option<Ship> {
        &self.slots[at.side][at.lane][at.slot_index]
    }
}

impl IndexMut<SlotRef> for Board {
    fn index_mut(&mut self, at: SlotRef) -> &mut Option<Ship> {
        &mut self.slots[at.side][at.lane][at.slot_index]
    }
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ein Brett mit dem Zustand des Snapshots.
    pub fn from_snapshot(snapshot: &BoardSnapshot) -> Self {
        let mut board = Board::new();
        board.load(snapshot);
        board
    }

    /// Das Schiff im Slot; der Slot muss belegt sein.
    pub fn ship(&self, at: SlotRef) -> &Ship {
        self[at].as_ref().expect("Slot ist belegt")
    }

    /// Das Schiff im Slot zum Ändern; der Slot muss belegt sein.
    pub fn ship_mut(&mut self, at: SlotRef) -> &mut Ship {
        self[at].as_mut().expect("Slot ist belegt")
    }

    /// Die Karte im Slot, falls belegt.
    pub fn card(&self, at: SlotRef) -> Option<ShipId> {
        self[at].as_ref().map(|ship| ship.card)
    }

    pub fn is_occupied(&self, at: SlotRef) -> bool {
        self[at].is_some()
    }

    /// Belegt und zerstört (liegt bis zum Entfernen noch auf dem Brett).
    pub fn is_destroyed(&self, at: SlotRef) -> bool {
        self[at].as_ref().is_some_and(Ship::is_destroyed)
    }

    /// Belegt und nicht zerstört.
    pub fn is_standing(&self, at: SlotRef) -> bool {
        self[at].as_ref().is_some_and(|ship| !ship.is_destroyed())
    }

    /// Leer und nicht für ein Schiff im Kurzsprung gesperrt: Hier darf ein Schiff hinein.
    pub fn is_free(&self, at: SlotRef) -> bool {
        self[at].is_none() && !self.jumping.iter().any(|jump| jump.slot == at)
    }

    /// Der serialisierbare Zustand des Slots.
    pub fn state(&self, at: SlotRef) -> Option<SlotState> {
        self[at].as_ref().map(Ship::state)
    }

    /// Eine neue Kennung für ein Schiff, das aufs Brett kommt.
    pub fn next_uid(&mut self) -> u32 {
        self.last_uid += 1;
        self.last_uid
    }

    /// Setzt ein neues Schiff in den Slot, ohne Regelprüfung (Tokens, Fähigkeiten); ein altes verschwindet.
    pub fn put(&mut self, at: SlotRef, card: ShipId) -> &mut Ship {
        let uid = self.next_uid();
        self[at].insert(Ship::new(card, uid))
    }

    /// Platziert ein Schiff nach den Regeln ([`Board::can_place`]); `None` bei Regelverstoß.
    pub fn place(&mut self, at: SlotRef, card: ShipId) -> Option<&mut Ship> {
        if !self.can_place(at, &[]) {
            return None;
        }
        Some(self.put(at, card))
    }

    /// Verlegt das Schiff samt Zustand in den Slot `to`; es gilt als bewegt.
    pub fn move_ship(&mut self, from: SlotRef, to: SlotRef) -> &mut Ship {
        let mut ship = self[from].take().expect("Slot ist belegt");
        ship.moved = true;
        self[to].insert(ship)
    }

    /// Phasentausch: Beide Schiffe tauschen die Plätze samt Zustand; keins gilt dabei als bewegt.
    pub fn swap(&mut self, a: SlotRef, b: SlotRef) {
        let first = self[a].take();
        self[a] = self[b].take();
        self[b] = first;
    }

    /// Belegte Slots einer Lane in Index-Reihenfolge.
    pub fn occupied(&self, side: Side, lane: LaneType) -> impl Iterator<Item = SlotRef> + '_ {
        lane_slots(side, lane).filter(|&at| self.is_occupied(at))
    }

    /// Schiffe einer Seite, Strike vor Command, jeweils nach Slot-Index.
    pub fn ships(&self, side: Side) -> impl Iterator<Item = SlotRef> + '_ {
        side_slots(side).filter(|&at| self.is_occupied(at))
    }

    /// Schiffe beider Seiten, erst [`Side::Player`], dann [`Side::Enemy`].
    pub fn all_ships(&self) -> impl Iterator<Item = SlotRef> + '_ {
        Side::ALL.into_iter().flat_map(|side| self.ships(side))
    }

    /// Das Flaggschiff der Seite; `None` vor Runde 1 und sobald es gefallen ist.
    pub fn flagship(&self, side: Side) -> Option<SlotRef> {
        self.ships(side)
            .find(|&at| self.ship(at).card.is_flagship())
    }

    /// Schiffe der Lane für die Lane-Grenze, auch eines im Kurzsprung; Tokens (Drohne, Trugbild) zählen nicht.
    pub fn lane_ship_count(&self, side: Side, lane: LaneType) -> usize {
        let on_board = self
            .occupied(side, lane)
            .filter(|&at| !self.ship(at).card.is_token())
            .count();
        let jumping = self
            .jumping
            .iter()
            .filter(|jump| {
                jump.slot.side == side && jump.slot.lane == lane && !jump.ship.card.is_token()
            })
            .count();
        on_board + jumping
    }

    pub fn lane_is_full(&self, side: Side, lane: LaneType) -> bool {
        self.lane_ship_count(side, lane) >= MAX_SHIPS_PER_LANE
    }

    /// Freier Slot für ein Schiff, das in diese Lane wechselt: derselbe Index, sonst der nächstgelegene; ein Token
    /// braucht nur einen freien Slot.
    pub fn free_slot_near(
        &self,
        side: Side,
        lane: LaneType,
        index: usize,
        token: bool,
    ) -> Option<SlotRef> {
        if self.lane_is_full(side, lane) && !token {
            return None;
        }
        nearest(lane_slots(side, lane).filter(|&at| self.is_free(at)), index)
    }

    /// Schiffe einer Seite für die Flottengrenze; Tokens (Drohne, Trugbild) zählen nicht.
    pub fn ship_count(&self, side: Side) -> usize {
        self.ships(side)
            .filter(|&at| !self.ship(at).card.is_token())
            .count()
    }

    /// ATK des Schiffs mit Overdrive (nur Kampfschiffe), Schrott, Signal Booster, Speerspitze und Rundenbonus.
    pub fn attack(&self, at: SlotRef) -> i32 {
        let ship = self.ship(at);
        let base = ship.card.atk();
        let overdrive = if base > 0 { self.overdrive } else { 0 };
        (base
            + overdrive
            + ship.scrap_bonus
            + self.signal_boost(at)
            + self.spearhead(at)
            + ship.attack_bonus)
            .max(0)
    }

    /// Vanguard (Speerspitze): Steht sie in der Command-Lane, hat das Kampfschiff im Strike-Slot direkt vor ihr
    /// mehr ATK.
    pub fn spearhead(&self, at: SlotRef) -> i32 {
        if at.lane != LaneType::Strike || self.ship(at).card.atk() == 0 {
            return 0;
        }
        if self.card(at.in_lane(LaneType::Command)) == Some(ShipId::Vanguard) {
            ABILITY_VALUES.spearhead
        } else {
            0
        }
    }

    /// Signal Booster: +1 ATK je anderem Booster in derselben Lane. Ein im Kampf tödlich getroffener Booster zählt
    /// noch, bis er im Cleanup das Brett verlässt.
    pub fn signal_boost(&self, at: SlotRef) -> i32 {
        self.occupied(at.side, at.lane)
            .filter(|&other| other != at && self.ship(other).card == ShipId::SignalBooster)
            .count() as i32
    }

    /// Ob das Schiff im Kampf schießen kann; Support-Schiffe (Basis-ATK 0) greifen nie an, auch nicht mit Bonus.
    pub fn can_attack(&self, at: SlotRef) -> bool {
        let Some(ship) = &self[at] else { return false };
        ship.card.atk() > 0
            && self.attack(at) > 0
            && !ship.disabled
            && !ship.maneuvered
            && !ship.is_destroyed()
    }

    /// Schutzregel: Ein Command-Schiff ist gedeckt, solange im Strike-Slot davor ein Schiff steht; ein zerstörtes
    /// deckt nicht mehr. `see_through`: so sehen es Flaggschiffe, die Trugbilder durchschauen; ein Trugbild davor
    /// deckt dann nicht.
    pub fn is_exposed(&self, at: SlotRef, see_through: bool) -> bool {
        if at.lane == LaneType::Strike {
            return true;
        }
        match &self[at.in_lane(LaneType::Strike)] {
            None => true,
            Some(front) => front.is_destroyed() || (see_through && front.card == ShipId::Decoy),
        }
    }

    /// Schiffe einer Seite, die beschossen werden können, in [`Board::ships`]-Reihenfolge; mit `see_through` ist ein
    /// Trugbild weder Ziel noch Deckung.
    pub fn exposed_ships(
        &self,
        side: Side,
        see_through: bool,
    ) -> impl Iterator<Item = SlotRef> + '_ {
        self.ships(side).filter(move |&at| {
            self.is_exposed(at, see_through)
                && !(see_through && self.ship(at).card == ShipId::Decoy)
        })
    }

    /// Ob ein Schiff auf dem Slot landen kann. `arrivals` sind die übrigen Deploys derselben Seite in dieser Runde:
    /// Ihre Slots gelten als besetzt, sie zählen zur Lane- und zur Flottengrenze.
    pub fn can_place(&self, at: SlotRef, arrivals: &[SlotTarget]) -> bool {
        if at.slot_index >= SLOT_COUNT || self.is_occupied(at) {
            return false;
        }
        let incoming: Vec<_> = arrivals
            .iter()
            .filter(|arrival| arrival.lane == at.lane)
            .collect();
        if incoming
            .iter()
            .any(|arrival| arrival.slot_index == at.slot_index)
        {
            return false;
        }
        if self.lane_ship_count(at.side, at.lane) + incoming.len() >= MAX_SHIPS_PER_LANE {
            return false;
        }
        self.ship_count(at.side) + arrivals.len() < MAX_SHIPS_PER_SIDE
    }

    /// Ob das Schiff in `from` in den freien Slot `to` seiner Seite wechseln kann, in der anderen oder derselben
    /// Lane. `deploys` sind die Slots, auf denen dieselbe Seite in dieser Runde Schiffe legt (sie landen vor allen
    /// Bewegungen), `other` ist ihre zweite Bewegung (Grundmanöver bzw. Bewegungsbefehl; ein Phasentausch gehört
    /// nicht hierher). Ihre Slots gelten als besetzt. Zur Lane-Grenze zählen die Deploys und die zweite Bewegung, wenn
    /// sie ein Schiff aus der anderen Lane bringt. Bewusst streng: Ein Schiff, das sich bewegt, gibt seinen Slot und
    /// seinen Platz in der Lane für die anderen Bewegungen dieser Runde nicht frei. Eine Bewegung innerhalb der Lane
    /// ändert deren Schiffszahl nicht. Eine Drohne zählt nicht zur Lane-Grenze und braucht nur einen freien Slot.
    pub fn can_move(
        &self,
        from: SlotRef,
        to: SlotTarget,
        deploys: &[SlotTarget],
        other: Option<PlannedMove>,
    ) -> bool {
        let to = to.on(from.side);
        if to.slot_index >= SLOT_COUNT
            || self.is_occupied(to)
            || other.is_some_and(|other| other.to == to)
        {
            return false;
        }
        let incoming = deploys.iter().filter(|deploy| deploy.lane == to.lane);
        if incoming
            .clone()
            .any(|deploy| deploy.slot_index == to.slot_index)
        {
            return false;
        }
        // `from` ist noch leer, wenn das Schiff erst in dieser Runde landet.
        if self.card(from).is_some_and(ShipId::is_token) {
            return true;
        }
        let joining = usize::from(other.is_some_and(|other| self.joins(other, to.lane)));
        let staying = usize::from(to.lane == from.lane);
        self.lane_ship_count(to.side, to.lane) + incoming.count() + joining
            < MAX_SHIPS_PER_LANE + staying
    }

    /// Ob die Bewegung ein Schiff aus der anderen Lane nach `lane` bringt, das zur Lane-Grenze zählt (kein Token).
    fn joins(&self, planned: PlannedMove, lane: LaneType) -> bool {
        planned.to.lane == lane
            && planned.from.lane != lane
            && !self.card(planned.from).is_some_and(ShipId::is_token)
    }

    /// Slots der anderen Lane, in die das Schiff in `from` per Bewegungsbefehl wechseln kann ([`Board::can_move`]).
    pub fn lane_switch_destinations(
        &self,
        from: SlotRef,
        deploys: &[SlotTarget],
        other: Option<PlannedMove>,
    ) -> Vec<SlotRef> {
        lane_slots(from.side, from.lane.other())
            .filter(|to| self.can_move(from, to.target(), deploys, other))
            .collect()
    }

    /// Slots beider eigenen Lanes, in die das Schiff in `from` per Grundmanöver wechseln kann ([`Board::can_move`]).
    pub fn maneuver_destinations(
        &self,
        from: SlotRef,
        deploys: &[SlotTarget],
        other: Option<PlannedMove>,
    ) -> Vec<SlotRef> {
        side_slots(from.side)
            .filter(|to| self.can_move(from, to.target(), deploys, other))
            .collect()
    }

    pub fn snapshot(&self) -> BoardSnapshot {
        let lanes = |side| Lanes {
            strike: std::array::from_fn(|index| {
                self.state(SlotRef::new(side, LaneType::Strike, index))
            }),
            command: std::array::from_fn(|index| {
                self.state(SlotRef::new(side, LaneType::Command, index))
            }),
        };
        Sides {
            player: lanes(Side::Player),
            enemy: lanes(Side::Enemy),
        }
    }

    /// Übernimmt den Zustand des Snapshots; jedes Schiff bekommt eine neue Kennung.
    pub fn load(&mut self, snapshot: &BoardSnapshot) {
        for side in Side::ALL {
            for at in side_slots(side) {
                self[at] = snapshot[side][at.lane][at.slot_index].map(|state| {
                    let mut ship = Ship::new(state.card_id, self.next_uid());
                    ship.hp = state.hp;
                    ship.shield = state.shield;
                    ship.scrap_bonus = state.scrap_bonus;
                    ship.marked = state.marked;
                    ship
                });
            }
        }
    }
}

/// Slot mit dem kleinsten Indexabstand; bei Gleichstand gewinnt der erste.
pub fn nearest(slots: impl IntoIterator<Item = SlotRef>, index: usize) -> Option<SlotRef> {
    slots
        .into_iter()
        .min_by_key(|slot| slot.slot_index.abs_diff(index))
}
