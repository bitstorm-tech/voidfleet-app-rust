//! Spielzustand eines Matches aus Sicht des Clients (Port von `game/match.ts`): Brett, Hände, Admirale und der
//! geplante Zug der laufenden Runde, ohne Rendering. Der Server ist autoritativ; [`Match::apply`] übernimmt seine
//! Sicht, geplant wird lokal bis zum Abgeben ([`Match::to_turn_plan`]).
//!
//! Geplant wird erst, wenn beide ihren Admiral haben ([`Match::admirals_chosen`]). Ein Schiff zum Legen ist eine Karte
//! der Hand oder in Runde 1 das Flaggschiff ([`ShipPick`]); das Flaggschiff ist Pflicht und zählt nicht zu den
//! Deploys, aber zu Lane- und Flottengrenze. Für Befehle, Fähigkeit und Feuerbefehle zählen eingeplante Schiffe schon
//! auf ihren Slots ([`Match::ship_at`]). Jede Änderung hält den Plan gültig: Was dabei wegfällt, melden
//! [`Match::set_command`], [`Match::set_maneuver`] und [`Match::set_power`].
//!
//! Slots: [`SlotRef`] für Stellen, die auf beiden Seiten liegen können (Brett, Zeiger), [`SlotTarget`] für eigene
//! Schiffe beim Planen (Feuerbefehle).

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, PowerId},
    board::{
        Board, PlannedMove, Side, SlotRef, SlotTarget, lane_slots, max_deploys, overdrive_bonus,
        side_slots,
    },
    cards::{CommandId, ShipId, allows_maneuver_and_power},
    powers::{self, PlannedArrival},
    protocol::{
        CommandPlan, Deploy, DraftOffer, FireOrder, FireTarget, Maneuver, MatchResult,
        OpponentView, PlayerView, PowerPlan, TurnPlan,
    },
};

/// Ein Schiff zum Legen: eine Karte der Schiffshand (Index) oder in Runde 1 das Flaggschiff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShipPick {
    Hand(usize),
    Flagship,
}

/// Ein eingeplantes Schiff und der Slot, auf dem es landet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Arrival {
    pub pick: ShipPick,
    pub target: SlotTarget,
}

/// Stand eines Zug-Teils: eingeplant, noch möglich oder gar nicht möglich.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TurnPartState {
    Done,
    Open,
    Blocked,
}

impl TurnPartState {
    fn of(planned: bool, possible: bool) -> Self {
        if planned {
            Self::Done
        } else if possible {
            Self::Open
        } else {
            Self::Blocked
        }
    }
}

/// Wie viele Schiffe schon ein Ziel haben von allen, die eines bekommen können.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FireOrderCount {
    pub assigned: usize,
    pub shooters: usize,
}

/// Die Teile eines Zugs für die Checkliste; `None` heißt, der Teil gehört in dieser Runde nicht dazu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TurnProgress {
    /// Runde 1: das Flaggschiff, Pflicht.
    pub flagship: Option<TurnPartState>,
    pub ships: TurnPartState,
    pub command: TurnPartState,
    /// Grundmanöver oder Admiralsfähigkeit; mit Freier Hand nur das Grundmanöver.
    pub maneuver: TurnPartState,
    /// Freie Hand: die Admiralsfähigkeit als eigener Teil neben dem Grundmanöver.
    pub power: Option<TurnPartState>,
    /// Pendulum: die Zielerfassung, nur in Runden, in denen sie markieren darf.
    pub mark: Option<TurnPartState>,
    /// Erledigt, wenn jedes Schiff, das feuern kann, ein Ziel hat (auch eine leere Spalte); nicht möglich ohne Schütze.
    pub fire: TurnPartState,
    pub fire_orders: FireOrderCount,
}

/// Was beim Planen eines Befehls wegfiel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CommandChange {
    /// Weicht Schnellverlegung, das zuletzt geplante überzählige Schiff.
    pub ship: Option<ShipId>,
    /// Weicht Freie Hand, während Grundmanöver und Fähigkeit geplant sind: die Fähigkeit.
    pub power: bool,
    /// Ein Schiff, das wegen des neuen Befehls nicht feuert und seinen Feuerbefehl verlor.
    pub fire: Option<ShipId>,
}

/// Was beim Planen des Grundmanövers wegfiel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ManeuverChange {
    /// Die Fähigkeit, außer mit Freier Hand.
    pub power: bool,
    /// Das manövrierende Schiff verlor seinen Feuerbefehl (nicht beim Phasentausch).
    pub fire: Option<ShipId>,
}

/// Befehle, deren eigenes Zielschiff in dieser Runde nicht feuert: Der Rammer rammt statt zu feuern, der Kurzsprung
/// nimmt es vor dem Kampf vom Brett.
const HOLDS_FIRE: [CommandId; 2] = [CommandId::CollisionCourse, CommandId::MicroJump];

/// Der eigene Slot.
const fn own(target: SlotTarget) -> SlotRef {
    target.on(Side::Player)
}

/// Der Spielzustand; als Ressource eingefügt vom [`super::GamePlugin`].
#[derive(Resource, Clone, Debug, Default)]
pub struct Match {
    pub board: Board,
    pub ship_hand: Vec<ShipId>,
    pub command_hand: Vec<CommandId>,
    pub ship_deck_size: usize,
    pub command_deck_size: usize,
    pub round: u32,
    /// Der eigene Admiral; `None`, solange noch nicht gewählt.
    pub admiral: Option<AdmiralId>,
    plan: TurnPlan,
    /// Der Zug ist abgegeben und bis zur Auflösung gesperrt.
    pub submitted: bool,
    /// Der Nachschub nach der letzten Runde, solange nicht gewählt; bis dahin wird nicht geplant.
    pub draft: Option<DraftOffer>,
    /// Was man vom Gegner weiß, sobald er beigetreten ist.
    pub opponent: Option<OpponentView>,
    pub result: Option<MatchResult>,
    pub surrendered: bool,
}

impl Match {
    /// Der geplante Zug.
    pub fn plan(&self) -> &TurnPlan {
        &self.plan
    }

    /// Der Zug zum Abgeben.
    pub fn to_turn_plan(&self) -> TurnPlan {
        self.plan.clone()
    }

    /// Das eigene Flaggschiff; `None`, solange kein Admiral gewählt ist.
    pub fn flagship(&self) -> Option<ShipId> {
        self.admiral.map(AdmiralId::flagship)
    }

    /// Die eigene Admiralsfähigkeit; `None`, solange kein Admiral gewählt ist.
    pub fn power(&self) -> Option<PowerId> {
        self.admiral.map(AdmiralId::power)
    }

    /// Runde 1 beginnt erst, wenn beide ihren Admiral gewählt haben; erst dann sieht man den des Gegners.
    pub fn admirals_chosen(&self) -> bool {
        self.admiral.is_some()
            && self
                .opponent
                .as_ref()
                .is_some_and(|opponent| opponent.admiral.is_some())
    }

    /// Eine Runde läuft: beide Admirale gewählt, das Match nicht entschieden. Dann gibt es Zug und Zielvorschau, auch
    /// nach dem Abgeben.
    pub fn round_running(&self) -> bool {
        self.round > 0 && self.admirals_chosen() && self.result.is_none()
    }

    /// Der Zug dieser Runde ist offen: nicht abgegeben und der Nachschub gewählt (ohne Verbindung und Wiedergabe, die
    /// kennt der Controller).
    pub fn turn_open(&self) -> bool {
        self.round_running() && !self.submitted && self.draft.is_none()
    }

    /// In Runde 1 kommt das Flaggschiff aufs Brett; abgeben geht erst, wenn es eingeplant ist.
    pub fn needs_flagship(&self) -> bool {
        self.round == 1 && self.flagship().is_some()
    }

    pub fn flagship_missing(&self) -> bool {
        self.needs_flagship() && self.plan.flagship.is_none()
    }

    /// Die eingeplanten Schiffe der Hand.
    pub fn planned_ships(&self) -> Vec<ShipId> {
        self.plan
            .ships
            .iter()
            .map(|ship| self.ship_hand[ship.hand_index])
            .collect()
    }

    /// Wie viele Schiffe in dieser Runde gelegt werden dürfen, mit eingeplanter Schnellverlegung eines mehr.
    pub fn deploy_limit(&self) -> usize {
        max_deploys(self.round, self.planned_command())
    }

    /// Wie viele Schiffe in dieser Runde noch gelegt werden dürfen; das Flaggschiff zählt nicht mit.
    pub fn deploys_left(&self) -> usize {
        self.deploy_limit().saturating_sub(self.plan.ships.len())
    }

    /// Freie Hand ist eingeplant: Grundmanöver und Admiralsfähigkeit ersetzen einander nicht.
    pub fn keeps_maneuver_and_power(&self) -> bool {
        allows_maneuver_and_power(self.planned_command())
    }

    /// Die Karte des eingeplanten Befehls.
    pub fn planned_command(&self) -> Option<CommandId> {
        self.plan
            .command
            .and_then(|command| self.command_hand.get(command.hand_index).copied())
    }

    pub fn card_of(&self, pick: ShipPick) -> ShipId {
        match pick {
            ShipPick::Hand(index) => self.ship_hand[index],
            ShipPick::Flagship => self.flagship().expect("das Flaggschiff gehört zum Admiral"),
        }
    }

    /// Was in dieser Runde landet: das Flaggschiff (Runde 1) und die eingeplanten Schiffe der Hand.
    pub fn arrivals(&self) -> Vec<Arrival> {
        self.kept_arrivals(None)
    }

    /// Das eingeplante Schiff, das auf diesem Slot landet; auf dem Brett ist er bis zur Auflösung leer.
    pub fn planned_pick_at(&self, at: SlotRef) -> Option<ShipPick> {
        if at.side != Side::Player {
            return None;
        }
        self.arrivals()
            .into_iter()
            .find(|arrival| arrival.target == at.target())
            .map(|arrival| arrival.pick)
    }

    /// Das Schiff auf einem Slot, wie es für Befehle und Fähigkeit zählt: auch ein eingeplantes, das vor ihnen landet.
    pub fn ship_at(&self, at: SlotRef) -> Option<ShipId> {
        match self.planned_pick_at(at) {
            Some(pick) => Some(self.card_of(pick)),
            None => self.board.card(at),
        }
    }

    /// Das Ziel eines eingeplanten Befehls auf der Gegnerseite (Torpedo, Durchschlag, Wärmesucher, Traktorstrahl).
    pub fn planned_enemy_target(&self) -> Option<SlotTarget> {
        if self.planned_command()?.targets_enemy_side() {
            self.plan.command?.target
        } else {
            None
        }
    }

    /// Das eigene Zielschiff des eingeplanten Befehls; `None` ohne Befehl, ohne Ziel oder mit Ziel auf der Gegnerseite.
    pub fn planned_own_target(&self) -> Option<SlotTarget> {
        if self.planned_command()?.targets_enemy_side() {
            None
        } else {
            self.plan.command?.target
        }
    }

    /// Ob `card` das eigene Schiff auf `at` zum Ziel haben darf, auch ein eingeplantes: Lane und Schiff.
    pub fn can_target_own(&self, card: CommandId, at: SlotRef) -> bool {
        at.side == Side::Player
            && self
                .ship_at(at)
                .is_some_and(|ship| card.can_target_lane(at.lane) && card.can_target_ship(ship))
    }

    /// Die übrigen Ankünfte, wenn `pick` (neu) geplant wird (`None`: alle): Es selbst zieht um, und sind schon alle
    /// Deploys vergeben, weicht beim Schiff der Hand das zuerst geplante. Das Flaggschiff zählt nicht zu den Deploys.
    pub fn kept_arrivals(&self, pick: Option<ShipPick>) -> Vec<Arrival> {
        let flagship = self
            .plan
            .flagship
            .filter(|_| pick != Some(ShipPick::Flagship))
            .map(|target| Arrival {
                pick: ShipPick::Flagship,
                target,
            });
        let ships = match pick {
            None => self.plan.ships.clone(),
            Some(pick) => self.kept_deploys(pick),
        };
        flagship
            .into_iter()
            .chain(ships.into_iter().map(|ship| Arrival {
                pick: ShipPick::Hand(ship.hand_index),
                target: ship.target,
            }))
            .collect()
    }

    /// Plant ein Schiff (siehe [`Self::kept_arrivals`]).
    pub fn plan_ship(&mut self, pick: ShipPick, target: SlotTarget) {
        match pick {
            ShipPick::Flagship => self.set_arrivals(Some(target), self.plan.ships.clone()),
            ShipPick::Hand(hand_index) => {
                let mut ships = self.kept_deploys(pick);
                ships.push(Deploy { hand_index, target });
                self.set_arrivals(self.plan.flagship, ships);
            }
        }
    }

    /// Nimmt das eingeplante Schiff zurück.
    pub fn unplan_ship(&mut self, pick: ShipPick) {
        match pick {
            ShipPick::Flagship => self.set_arrivals(None, self.plan.ships.clone()),
            ShipPick::Hand(index) => {
                let ships = self
                    .plan
                    .ships
                    .iter()
                    .copied()
                    .filter(|ship| ship.hand_index != index)
                    .collect();
                self.set_arrivals(self.plan.flagship, ships);
            }
        }
    }

    /// Plant einen Befehl (`None`: keinen) und liefert, was dabei wegfällt. Weicht Schnellverlegung, fällt das zuletzt
    /// geplante überzählige Schiff weg (nicht das Ziel des neuen Befehls), samt Fähigkeit und Feuerbefehl darauf;
    /// weicht Freie Hand, während Grundmanöver und Fähigkeit geplant sind, fällt die Fähigkeit weg. Hält der neue
    /// Befehl sein Zielschiff vom Feuern ab ([`HOLDS_FIRE`]), fällt dessen Feuerbefehl weg.
    pub fn set_command(&mut self, command: Option<CommandPlan>) -> CommandChange {
        let had_power = self.plan.power.is_some();
        self.plan.command = command;
        let mut ship = None;
        if self.plan.ships.len() > self.deploy_limit() {
            let target = self.planned_own_target();
            let dropped = *self
                .plan
                .ships
                .iter()
                .rev()
                .find(|planned| Some(planned.target) != target)
                .expect("von zwei überzähligen Schiffen ist höchstens eines das Ziel");
            ship = Some(self.ship_hand[dropped.hand_index]);
            self.unplan_ship(ShipPick::Hand(dropped.hand_index));
        }
        if self.plan.maneuver.is_some()
            && self.plan.power.is_some()
            && !self.keeps_maneuver_and_power()
        {
            self.plan.power = None;
        }
        CommandChange {
            ship,
            power: had_power && self.plan.power.is_none(),
            fire: self.keep_fire().first().copied(),
        }
    }

    /// Plant das Grundmanöver (`None`: keins) und liefert, was dabei wegfällt: die Fähigkeit, außer mit Freier Hand,
    /// und der Feuerbefehl des manövrierenden Schiffs, außer beim Phasentausch.
    pub fn set_maneuver(&mut self, maneuver: Option<Maneuver>) -> ManeuverChange {
        let power =
            maneuver.is_some() && self.plan.power.is_some() && !self.keeps_maneuver_and_power();
        self.plan.maneuver = maneuver;
        if power {
            self.plan.power = None;
        }
        ManeuverChange {
            power,
            fire: self.keep_fire().first().copied(),
        }
    }

    /// Plant die Admiralsfähigkeit (`None`: keine). Sie ersetzt das Grundmanöver, außer mit Freier Hand; geliefert wird
    /// das Schiff, dessen Manöver dabei wegfiel.
    pub fn set_power(&mut self, power: Option<PowerPlan>) -> Option<ShipId> {
        self.plan.power = power;
        if power.is_none() || self.keeps_maneuver_and_power() {
            return None;
        }
        let replaced = self.planned_maneuver()?;
        self.plan.maneuver = None;
        self.board.card(replaced.from)
    }

    /// Plant die Zielerfassung der Pendulum (`None`: keine); sie ersetzt nichts.
    pub fn set_mark(&mut self, mark: Option<SlotTarget>) {
        self.plan.mark = mark;
    }

    /// Die eingeplanten Schiffe der Hand ohne `pick`; sind schon alle Deploys vergeben, weicht das zuerst geplante.
    fn kept_deploys(&self, pick: ShipPick) -> Vec<Deploy> {
        let ShipPick::Hand(index) = pick else {
            return self.plan.ships.clone();
        };
        let others: Vec<Deploy> = self
            .plan
            .ships
            .iter()
            .copied()
            .filter(|ship| ship.hand_index != index)
            .collect();
        let surplus = (others.len() + 1).saturating_sub(self.deploy_limit());
        others[surplus..].to_vec()
    }

    /// Setzt die Ankünfte. Ein Befehl auf ein eingeplantes Schiff fällt mit weg, sobald auf dessen Slot nichts mehr
    /// landet; die Fähigkeit, sobald ihr Ziel nicht mehr gilt (z. B. Taktvorsprung, wenn statt des Kampfschiffs ein
    /// Support-Schiff dort landet). Der Feuerbefehl gehört dem Schiff: Wird es zurückgenommen oder umgeplant, fällt
    /// er weg, auch wenn danach ein anderes auf seinem Slot landet.
    fn set_arrivals(&mut self, flagship: Option<SlotTarget>, ships: Vec<Deploy>) {
        let before = self.arrivals();
        self.plan.flagship = flagship;
        self.plan.ships = ships;
        let after = self.arrivals();
        let gone: Vec<SlotTarget> = before
            .into_iter()
            .filter(|old| !after.contains(old))
            .map(|old| old.target)
            .collect();
        self.plan.fire.retain(|order| !gone.contains(&order.ship));
        if let (Some(card), Some(target)) = (self.planned_command(), self.planned_own_target())
            && !self.can_target_own(card, own(target))
        {
            self.plan.command = None;
        }
        if let Some(power) = self.planned_power_target()
            && !self.power_targets().contains(&power)
        {
            self.plan.power = None;
        }
    }

    /// Ob das eigene Schiff auf `at` (Slot beim Planen, auch ein eingeplantes) einen Feuerbefehl bekommen kann: ein
    /// Kampfschiff, das in dieser Runde feuert. Das Schiff des Grundmanövers feuert nicht (beim Phasentausch feuern
    /// beide), ebenso wenig das Zielschiff von Rammkurs und Kurzsprung ([`HOLDS_FIRE`]).
    pub fn can_fire(&self, at: SlotRef) -> bool {
        if at.side != Side::Player || !self.ship_at(at).is_some_and(|ship| ship.atk() > 0) {
            return false;
        }
        let here = at.target();
        if self
            .plan
            .maneuver
            .is_some_and(|maneuver| maneuver.ship == here)
            && self.planned_swap().is_none()
        {
            return false;
        }
        !(self.planned_own_target() == Some(here)
            && self
                .planned_command()
                .is_some_and(|command| HOLDS_FIRE.contains(&command)))
    }

    /// Die eigenen Schiffe, die einen Feuerbefehl bekommen können, als Slot beim Planen (Strike vor Command).
    pub fn shooters(&self) -> Vec<SlotTarget> {
        side_slots(Side::Player)
            .filter(|&at| self.can_fire(at))
            .map(SlotRef::target)
            .collect()
    }

    /// Der Feuerbefehl des eigenen Schiffs auf `ship` (Slot beim Planen).
    pub fn fire_order_at(&self, ship: SlotTarget) -> Option<FireOrder> {
        self.plan
            .fire
            .iter()
            .find(|order| order.ship == ship)
            .copied()
    }

    /// Der Feuerbefehl des eigenen Schiffs auf `ship` nach einem Tipp auf `aim` (Pfeil vor einer gegnerischen Spalte
    /// bzw. gegnerisches Schiff): Spalten- bzw. Schiffsfeuer; gilt ihm der Befehl schon, `None` (keiner mehr).
    pub fn next_fire(&self, ship: SlotTarget, aim: FireTarget) -> Option<FireOrder> {
        let order = FireOrder { ship, target: aim };
        (self.fire_order_at(ship) != Some(order)).then_some(order)
    }

    /// Setzt den Feuerbefehl des eigenen Schiffs auf `ship`; ein alter weicht, `None` nimmt ihn zurück.
    pub fn set_fire(&mut self, ship: SlotTarget, order: Option<FireOrder>) {
        self.plan.fire.retain(|old| old.ship != ship);
        self.plan.fire.extend(order);
    }

    /// Die Drohne fliegt den eigenen Schwarmangriff: Sie feuert auf dessen Ziel, auch ohne Feuerbefehl.
    pub fn flies_swarm(&self, ship: SlotTarget) -> bool {
        self.power() == Some(PowerId::SwarmStrike)
            && self.plan.power.is_some()
            && self.ship_at(own(ship)) == Some(ShipId::Drone)
    }

    /// Das Schiff feuert in dieser Runde auf etwas: mit Feuerbefehl oder im eigenen Schwarmangriff.
    pub fn is_armed(&self, ship: SlotTarget) -> bool {
        self.fire_order_at(ship).is_some() || self.flies_swarm(ship)
    }

    /// Feuerbefehle von Schiffen, die nicht (mehr) feuern, fallen weg; geliefert werden diese Schiffe.
    fn keep_fire(&mut self) -> Vec<ShipId> {
        let (kept, dropped): (Vec<FireOrder>, Vec<FireOrder>) = self
            .plan
            .fire
            .iter()
            .copied()
            .partition(|order| self.can_fire(own(order.ship)));
        self.plan.fire = kept;
        dropped
            .into_iter()
            .filter_map(|order| self.ship_at(own(order.ship)))
            .collect()
    }

    /// Der Plan, wie er mit `change` aussähe; der eigene bleibt dabei unverändert (Zielvorschau unter dem Zeiger).
    pub fn try_plan(&self, change: impl FnOnce(&mut Match)) -> TurnPlan {
        let mut tried = self.clone();
        change(&mut tried);
        tried.plan
    }

    /// Die Bewegung des verlegenden Befehls in seine Ziel-Lane (die andere, beim Ausweichmanöver dieselbe). `from`
    /// kann der Slot eines eingeplanten Schiffs sein, der auf dem Brett noch leer ist (siehe [`Self::ship_at`]).
    pub fn planned_command_move(&self) -> Option<PlannedMove> {
        let command = self.plan.command?;
        let (target, destination) = (command.target?, command.destination?);
        let lane = self.planned_command()?.destination_lane(target.lane);
        Some(PlannedMove {
            from: own(target),
            to: SlotRef::new(Side::Player, lane, destination),
        })
    }

    /// Das Grundmanöver: ein Schiff, das schon auf dem Brett steht, in einen freien Slot beider Lanes; es feuert in
    /// dieser Runde nicht. Die Mirage darf auch in einen belegten Slot: Phasentausch, beide tauschen und feuern.
    pub fn planned_maneuver(&self) -> Option<PlannedMove> {
        self.plan.maneuver.map(|maneuver| PlannedMove {
            from: own(maneuver.ship),
            to: own(maneuver.destination),
        })
    }

    /// Das Grundmanöver, wenn es ein Phasentausch ist: Sein Ziel-Slot ist belegt.
    pub fn planned_swap(&self) -> Option<PlannedMove> {
        self.planned_maneuver()
            .filter(|maneuver| self.board.is_occupied(maneuver.to))
    }

    /// Das Trugbild: der freie Strike-Slot, in dem es in dieser Runde steht.
    pub fn planned_decoy(&self) -> Option<SlotTarget> {
        if self.power() == Some(PowerId::Decoy) {
            self.plan.power.map(|power| power.target)
        } else {
            None
        }
    }

    /// Die Slots aller Ankünfte.
    fn arrival_targets(&self) -> Vec<SlotTarget> {
        self.arrivals()
            .into_iter()
            .map(|arrival| arrival.target)
            .collect()
    }

    /// Ziel-Slots für den verlegenden Befehl `card` auf das Schiff in `from`, in seiner Ziel-Lane: nicht für das
    /// manövrierende Schiff (auch nicht für den Tauschpartner der Mirage); Deploys, Manöver-Ziel und Trugbild zählen
    /// als besetzt.
    pub fn command_move_destinations(&self, card: CommandId, from: SlotRef) -> Vec<SlotRef> {
        let maneuver = self.planned_maneuver();
        let swap = self.planned_swap();
        if maneuver.is_some_and(|maneuver| maneuver.from == from)
            || swap.is_some_and(|swap| swap.to == from)
        {
            return Vec::new();
        }
        let deploys = self.arrival_targets();
        // Beim Phasentausch ist das Ziel schon belegt; der Tausch ändert die Schiffszahl keiner Lane.
        let other = if swap.is_some() { None } else { maneuver };
        let decoy = self.planned_decoy();
        lane_slots(Side::Player, card.destination_lane(from.lane))
            .filter(|to| {
                self.board.can_move(from, to.target(), &deploys, other)
                    && decoy != Some(to.target())
            })
            .collect()
    }

    /// Ziel-Slots für das Manöver des Schiffs in `from`: nur echte Schiffe, die kein Bewegungsbefehl verlegt. Die
    /// Mirage tauscht auch mit einem eigenen Schiff, das schon auf dem Brett steht und das kein Bewegungsbefehl
    /// verlegt. Mit Freier Hand bleibt ein eingeplantes Trugbild stehen; sein Slot ist dann kein Ziel.
    pub fn maneuver_destinations(&self, from: SlotRef) -> Vec<SlotRef> {
        let command = self.planned_command_move();
        let Some(card) = self.board.card(from) else {
            return Vec::new();
        };
        if command.is_some_and(|command| command.from == from) {
            return Vec::new();
        }
        let decoy = self
            .keeps_maneuver_and_power()
            .then(|| self.planned_decoy())
            .flatten();
        let mut destinations: Vec<SlotRef> = self
            .board
            .maneuver_destinations(from, &self.arrival_targets(), command)
            .into_iter()
            .filter(|to| decoy != Some(to.target()))
            .collect();
        if card == ShipId::Mirage {
            destinations.extend(
                self.board
                    .ships(Side::Player)
                    .filter(|&at| at != from && command.is_none_or(|command| command.from != at)),
            );
        }
        destinations
    }

    /// Ob `pick` auf `at` landen kann, neben den übrigen Ankünften ([`Self::kept_arrivals`]), nicht im Slot des
    /// Trugbilds und ohne den eingeplanten Bewegungen im Weg zu stehen. Verlegt der Befehl ein Schiff, das dabei
    /// wegfällt, fällt er beim Umplanen ohnehin weg ([`Self::plan_ship`]).
    pub fn can_deploy(&self, at: SlotRef, pick: ShipPick) -> bool {
        if self.planned_decoy() == Some(at.target()) {
            return false;
        }
        let mut deploys: Vec<SlotTarget> = self
            .kept_arrivals(Some(pick))
            .into_iter()
            .map(|arrival| arrival.target)
            .collect();
        if !self.board.can_place(at, &deploys) {
            return false;
        }
        deploys.push(at.target());
        let keeps = |planned: &PlannedMove| {
            self.board.is_occupied(planned.from) || deploys.contains(&planned.from.target())
        };
        let command = self.planned_command_move().filter(keeps);
        // Der Phasentausch geht in einen belegten Slot; ihm kann kein Deploy im Weg stehen.
        let maneuver = if self.planned_swap().is_some() {
            None
        } else {
            self.planned_maneuver()
        };
        let fits = |planned: Option<PlannedMove>, other: Option<PlannedMove>| {
            planned.is_none_or(|planned| {
                self.board
                    .can_move(planned.from, planned.to.target(), &deploys, other)
            })
        };
        fits(command, maneuver) && fits(maneuver, command)
    }

    /// Gültige Ziele der eigenen Admiralsfähigkeit beim jetzigen Plan; ein geplantes Manöver weicht ihr, außer mit
    /// Freier Hand: Dann bleibt es, und sein Ziel-Slot ist für das Trugbild besetzt.
    pub fn power_targets(&self) -> Vec<SlotRef> {
        let Some(power) = self.power() else {
            return Vec::new();
        };
        let arrivals: Vec<PlannedArrival> = self
            .arrivals()
            .into_iter()
            .map(|arrival| PlannedArrival {
                card: self.card_of(arrival.pick),
                target: arrival.target,
            })
            .collect();
        let maneuver = if self.keeps_maneuver_and_power() && self.planned_swap().is_none() {
            self.planned_maneuver()
        } else {
            None
        };
        let blocked: Vec<SlotTarget> = [self.planned_command_move(), maneuver]
            .into_iter()
            .flatten()
            .map(|planned| planned.to.target())
            .collect();
        powers::power_targets(&self.board, Side::Player, power, &arrivals, &blocked)
    }

    /// Die eingeplante Fähigkeit als Slot samt Seite.
    pub fn planned_power_target(&self) -> Option<SlotRef> {
        let target = self.plan.power?.target;
        let side = if self.power()?.targets_enemy() {
            Side::Enemy
        } else {
            Side::Player
        };
        Some(target.on(side))
    }

    /// Die Pendulum darf markieren: Sie steht und ihre Railgun feuert in dieser Runde nicht.
    pub fn can_mark(&self) -> bool {
        powers::can_mark(&self.board, Side::Player)
    }

    pub fn mark_targets(&self) -> Vec<SlotRef> {
        powers::mark_targets(&self.board, Side::Player)
    }

    /// Das gegnerische Schiff, auf das die eigene Railgun in dieser Runde feuert: das markierte.
    pub fn railgun_target(&self) -> Option<SlotRef> {
        self.board
            .ships(Side::Enemy)
            .find(|&at| self.board.ship(at).marked)
    }

    /// Ob sich die eigene Pendulum laut Plan bewegt (auch per Kurzsprung): Dann verfällt ihre Markierung bzw. ihr
    /// Schuss.
    pub fn pendulum_moves(&self) -> bool {
        let Some(pendulum) = self
            .board
            .flagship(Side::Player)
            .filter(|&at| self.board.ship(at).card == ShipId::Pendulum)
        else {
            return false;
        };
        let jumps = self.planned_command() == Some(CommandId::MicroJump)
            && self.planned_own_target() == Some(pendulum.target());
        jumps
            || self
                .planned_maneuver()
                .is_some_and(|planned| planned.from == pendulum)
            || self
                .planned_command_move()
                .is_some_and(|planned| planned.from == pendulum)
    }

    /// Was vom Zug noch offen ist. Schiffe gelten als erledigt, sobald alle Deploys vergeben sind oder wenigstens
    /// eines eingeplant ist und kein weiteres mehr passt.
    pub fn turn_progress(&self) -> TurnProgress {
        use TurnPartState::{Blocked, Done, Open};
        let slots: Vec<SlotRef> = side_slots(Side::Player).collect();
        let can_deploy_more = self.deploys_left() > 0
            && (0..self.ship_hand.len())
                .filter(|&index| !self.plan.ships.iter().any(|ship| ship.hand_index == index))
                .any(|index| {
                    slots
                        .iter()
                        .any(|&at| self.can_deploy(at, ShipPick::Hand(index)))
                });
        let can_maneuver = slots
            .iter()
            .any(|&at| !self.maneuver_destinations(at).is_empty());
        let can_power = !self.power_targets().is_empty();
        let both = self.keeps_maneuver_and_power();
        let shooters = self.shooters();
        let assigned = shooters.iter().filter(|&&ship| self.is_armed(ship)).count();
        let plan = &self.plan;
        TurnProgress {
            flagship: self
                .needs_flagship()
                .then(|| TurnPartState::of(plan.flagship.is_some(), true)),
            ships: if can_deploy_more {
                Open
            } else if plan.ships.is_empty() {
                Blocked
            } else {
                Done
            },
            command: TurnPartState::of(plan.command.is_some(), !self.command_hand.is_empty()),
            maneuver: if both {
                TurnPartState::of(plan.maneuver.is_some(), can_maneuver)
            } else {
                TurnPartState::of(
                    plan.maneuver.is_some() || plan.power.is_some(),
                    can_maneuver || can_power,
                )
            },
            power: both.then(|| TurnPartState::of(plan.power.is_some(), can_power)),
            mark: self
                .can_mark()
                .then(|| TurnPartState::of(plan.mark.is_some(), !self.mark_targets().is_empty())),
            fire: if shooters.is_empty() {
                Blocked
            } else if assigned == shooters.len() {
                Done
            } else {
                Open
            },
            fire_orders: FireOrderCount {
                assigned,
                shooters: shooters.len(),
            },
        }
    }

    /// Eigene Kriegsschiffe auf Brett und Hand, ohne Drohnen.
    pub fn warships(&self) -> usize {
        self.board
            .ships(Side::Player)
            .map(|at| self.board.ship(at).card)
            .chain(self.ship_hand.iter().copied())
            .filter(|card| card.atk() > 0 && !card.is_token())
            .count()
    }

    /// Die Runde ist aufgelöst, der Plan verbraucht. Das meldet erst die Auflösung selbst: Wer als Zweiter abgibt,
    /// bekommt keinen Zustand mit abgegebenem Plan, und nach der letzten Runde steigt die Rundenzahl nicht mehr.
    pub fn end_round(&mut self) {
        self.plan = TurnPlan::default();
    }

    /// Übernimmt die Server-Sicht. Ein noch nicht abgegebener Plan bleibt innerhalb der Runde erhalten.
    pub fn apply(&mut self, view: PlayerView) {
        self.submitted = view.submitted_plan.is_some();
        if let Some(plan) = view.submitted_plan {
            self.plan = plan;
        } else if view.round != self.round || view.admiral != self.admiral {
            self.plan = TurnPlan::default();
        }
        self.draft = view.draft;
        self.round = view.round;
        self.admiral = view.admiral;
        self.board.load(&view.board);
        self.board.overdrive = overdrive_bonus(view.round);
        self.board.hive_launches = view.hive_launches;
        self.ship_hand = view.ship_hand;
        self.command_hand = view.command_hand;
        self.ship_deck_size = view.ship_deck_size;
        self.command_deck_size = view.command_deck_size;
        self.opponent = view.opponent;
        self.result = view.result;
        self.surrendered = view.surrendered;
    }
}

#[cfg(test)]
mod tests;
