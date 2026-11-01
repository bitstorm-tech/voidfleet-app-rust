//! Ein Match auf dem Server: zwei Sitzplätze, ein Brett, Decks und Hände.
//!
//! Der Server ist autoritativ: Er prüft die Züge, hält sie bis zum Aufdecken geheim und löst die Runde auf, sobald
//! beide abgegeben haben (die Phasen stehen in [`voidfleet_core::resolution`]). Vor Runde 1 wählt jeder verdeckt
//! seinen Admiral. Nach jeder Runde wählt jeder aus dem Nachschub ([`DraftOffer`]) ein Schiff und einen Befehl, bevor
//! er den nächsten Zug abgibt. Statt eines zweiten Menschen kann der Computergegner ([`crate::bot`]) einen Platz
//! einnehmen; er zieht, sobald der Mensch abgibt.
//!
//! Intern sitzt [`Seat::First`] auf der Seite [`Side::Player`] des Bretts und [`Seat::Second`] auf [`Side::Enemy`];
//! alles, was an den zweiten Sitz geht, wird gespiegelt.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use voidfleet_core::admirals::AdmiralId;
use voidfleet_core::board::{
    Board, PlannedMove, SLOT_COUNT, Side, Sides, SlotRef, SlotTarget, max_deploys, overdrive_bonus,
};
use voidfleet_core::cards::{CommandId, ShipClass, ShipId, allows_maneuver_and_power};
use voidfleet_core::powers::{PlannedArrival, can_mark, mark_targets, power_targets};
use voidfleet_core::protocol::{
    AbilityEffect, DraftOffer, DraftPick, ErrorCode, FireTarget, MatchResult, Mirror, OpponentView,
    PlayerView, RoundEvent, TurnPlan,
};
use voidfleet_core::resolution::{Drawn, PowerUse, Turn, TurnCommand, resolve_turns};

use crate::bot;
use crate::deck::Deck;

#[cfg(test)]
mod tests;

/// Höchstens so viele Schiffe und ebenso viele Befehle auf der Hand; gleiche Karten zählen einzeln.
const HAND_LIMIT: usize = 10;
/// Die Starthand bekommt immer 1 Support- und 3 Kampfschiffe, jeweils zufällig aus dem Deck, dazu 3 Befehle.
const STARTING_SUPPORT_SHIPS: usize = 1;
const STARTING_COMBAT_SHIPS: usize = 3;
const STARTING_COMMANDS: usize = 3;
/// Der Nachschub nach jeder Runde zeigt je so viele Schiffe und Befehle.
const DRAFT_SIZE: usize = 3;

/// Ein Sitzplatz; wer zuerst beitritt, sitzt auf dem ersten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seat {
    First,
    Second,
}

impl Seat {
    pub const ALL: [Seat; 2] = [Seat::First, Seat::Second];

    pub const fn index(self) -> usize {
        match self {
            Seat::First => 0,
            Seat::Second => 1,
        }
    }

    pub const fn other(self) -> Seat {
        match self {
            Seat::First => Seat::Second,
            Seat::Second => Seat::First,
        }
    }

    /// Die Seite des Bretts, auf der der Sitz intern spielt.
    pub const fn side(self) -> Side {
        match self {
            Seat::First => Side::Player,
            Seat::Second => Side::Enemy,
        }
    }

    /// Ein Wert je Seite (Brett, Hive-Zähler) aus Sicht des Sitzes: Für den zweiten tauschen die Seiten.
    pub fn mirror<T>(self, sides: Sides<T>) -> Sides<T> {
        match self {
            Seat::First => sides,
            Seat::Second => sides.mirrored(),
        }
    }

    /// Der Sitz, der auf dieser Seite des Bretts spielt.
    pub const fn of(side: Side) -> Seat {
        match side {
            Side::Player => Seat::First,
            Side::Enemy => Seat::Second,
        }
    }
}

/// Wie das Match ausgegangen ist, aus Sicht des Bretts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// Diese Seite hat gewonnen.
    Won(Side),
    /// Beide Flaggschiffe sind in derselben Runde gefallen.
    Draw,
}

/// Was der Computergegner auf seinem Sitz beim Planen weiß: das Brett (aus Sicht von [`Seat::First`], seine Seite ist
/// `side`), die Runde, seine eigenen Karten und, was jeder vom Gegner sieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotView {
    pub board: Board,
    pub round: u32,
    pub side: Side,
    pub admiral: AdmiralId,
    pub ship_hand: Vec<ShipId>,
    pub command_hand: Vec<CommandId>,
    pub opponent: OpponentView,
}

/// Der Nachschub nach der letzten Runde, bis der Spieler gewählt hat; die Karten fehlen solange im Deck.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Draft {
    ships: Vec<ShipId>,
    commands: Vec<CommandId>,
}

#[derive(Debug, Clone)]
struct Player {
    /// Der Computergegner hat kein Token; er wird nie über einen Socket angesprochen.
    token: Option<String>,
    seat: Seat,
    ship_deck: Deck<ShipId>,
    command_deck: Deck<CommandId>,
    ship_hand: Vec<ShipId>,
    command_hand: Vec<CommandId>,
    /// Die aufgedeckten Befehle in der Reihenfolge, in der er sie gespielt hat; der Gegner sieht sie.
    played_commands: Vec<CommandId>,
    /// Vor Runde 1 gewählt ([`Match::choose_admiral`]), beim Computergegner sofort.
    admiral: Option<AdmiralId>,
    plan: Option<TurnPlan>,
    draft: Option<Draft>,
}

fn is_support(card: ShipId) -> bool {
    card.card().class == ShipClass::Support
}

impl Player {
    fn new(token: Option<String>, seat: Seat, rng: &mut dyn RngCore) -> Self {
        let mut ship_deck = Deck::full(rng);
        let mut command_deck = Deck::full(rng);
        let mut draw_ships = |count: usize, support: bool| -> Vec<ShipId> {
            (0..count)
                .map(|_| {
                    ship_deck
                        .draw_where(|card| is_support(card) == support)
                        .expect("das Deck hat genug Schiffe jeder Art")
                })
                .collect()
        };
        let mut ship_hand = draw_ships(STARTING_SUPPORT_SHIPS, true);
        ship_hand.extend(draw_ships(STARTING_COMBAT_SHIPS, false));
        let command_hand = command_deck.draw_many(STARTING_COMMANDS);
        Player {
            token,
            seat,
            ship_deck,
            command_deck,
            ship_hand,
            command_hand,
            played_commands: Vec::new(),
            admiral: None,
            plan: None,
            draft: None,
        }
    }

    fn side(&self) -> Side {
        self.seat.side()
    }

    fn is_bot(&self) -> bool {
        self.token.is_none()
    }

    fn admiral(&self) -> AdmiralId {
        self.admiral.expect("der Admiral ist gewählt")
    }

    /// Zieht den Nachschub: je bis zu 3 Karten, für eine volle Handhälfte keine; mit der Schrottverwertung
    /// (`salvage`) ist sie der erste Befehl. Ohne Karten gibt es nichts zu wählen.
    fn offer_draft(&mut self, salvage: bool) {
        let full = |len: usize| len >= HAND_LIMIT;
        let ships = if full(self.ship_hand.len()) {
            Vec::new()
        } else {
            self.ship_deck.draw_many(DRAFT_SIZE)
        };
        let commands = match (full(self.command_hand.len()), salvage) {
            (true, _) => Vec::new(),
            (false, true) => std::iter::once(CommandId::ScrapSalvage)
                .chain(self.command_deck.draw_many(DRAFT_SIZE - 1))
                .collect(),
            (false, false) => self.command_deck.draw_many(DRAFT_SIZE),
        };
        self.draft =
            (!ships.is_empty() || !commands.is_empty()).then_some(Draft { ships, commands });
    }

    /// Nimmt die gewählten Karten des Nachschubs auf die Hand; die übrigen gehen gemischt zurück ins Deck.
    fn take_draft(&mut self, pick: DraftPick, rng: &mut dyn RngCore) {
        let Draft {
            mut ships,
            mut commands,
        } = self.draft.take().expect("es gibt einen Nachschub");
        self.ship_hand
            .extend(pick.ship.map(|index| ships.remove(index)));
        self.command_hand
            .extend(pick.command.map(|index| commands.remove(index)));
        self.ship_deck.put_back(ships, rng);
        self.command_deck.put_back(commands, rng);
    }

    fn draft_offer(&self) -> Option<DraftOffer> {
        self.draft.as_ref().map(|draft| DraftOffer {
            ships: draft.ships.clone(),
            commands: draft.commands.clone(),
        })
    }

    /// Zieht für einen Befehl (Plündern, Gegenmaßnahmen); zählt, was auf der Hand gelandet ist. Eine Karte, die
    /// auf eine volle Hand gezogen wird, verfällt.
    fn draw_extra(&mut self, ships: u32, commands: u32) -> Drawn {
        fn draw_into<T: crate::deck::Card>(
            hand: &mut Vec<T>,
            deck: &mut Deck<T>,
            count: u32,
        ) -> u32 {
            let mut landed = 0;
            for _ in 0..count {
                if let Some(card) = deck.draw()
                    && hand.len() < HAND_LIMIT
                {
                    hand.push(card);
                    landed += 1;
                }
            }
            landed
        }
        Drawn {
            ships: draw_into(&mut self.ship_hand, &mut self.ship_deck, ships),
            commands: draw_into(&mut self.command_hand, &mut self.command_deck, commands),
        }
    }

    /// Deckt den abgegebenen Zug auf: Seine Karten verlassen die Hand, der Befehl ist ab jetzt für den Gegner
    /// sichtbar.
    fn reveal(&mut self) -> Turn {
        let plan = self.plan.take().expect("der Zug ist abgegeben");
        let admiral = self.admiral();
        let ships = plan
            .ships
            .iter()
            .map(|deploy| PlannedArrival {
                card: self.ship_hand[deploy.hand_index],
                target: deploy.target,
            })
            .collect();
        // Von hinten entfernen, damit die übrigen Hand-Indizes gültig bleiben.
        let mut indices: Vec<usize> = plan.ships.iter().map(|deploy| deploy.hand_index).collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices {
            self.ship_hand.remove(index);
        }
        let command = plan.command.map(|command| {
            let card = self.command_hand.remove(command.hand_index);
            self.played_commands.push(card);
            TurnCommand {
                card,
                target: command.target,
                destination: command.destination,
            }
        });
        Turn {
            side: self.side(),
            ships,
            flagship: plan.flagship.map(|target| PlannedArrival {
                card: admiral.flagship(),
                target,
            }),
            command,
            maneuver: plan.maneuver,
            power: plan.power.map(|power| PowerUse {
                id: admiral.power(),
                target: power.target,
            }),
            mark: plan.mark,
            fire: plan.fire,
        }
    }

    /// Was der Gegner von diesem Spieler weiß; den Admiral erst, wenn beide gewählt haben.
    fn public_view(&self, admirals_chosen: bool) -> OpponentView {
        OpponentView {
            admiral: self.admiral.filter(|_| admirals_chosen),
            submitted: self.plan.is_some(),
            ship_hand_size: self.ship_hand.len(),
            command_hand_size: self.command_hand.len(),
            ship_deck_size: self.ship_deck.len(),
            command_deck_size: self.command_deck.len(),
            bot: self.is_bot(),
            played_commands: self.played_commands.clone(),
        }
    }
}

/// Für die Tests des Computergegners: Brett, Runde und Hände direkt setzen.
#[cfg(test)]
impl Match {
    pub(crate) fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub(crate) fn set_round(&mut self, round: u32) {
        self.round = round;
    }

    pub(crate) fn set_hands(&mut self, seat: Seat, ships: &[ShipId], commands: &[CommandId]) {
        let player = self.player_mut(seat);
        player.ship_hand = ships.to_vec();
        player.command_hand = commands.to_vec();
    }
}

/// Das Schiff in einem Slot; `None` auch für einen Slot außerhalb des Bretts (Züge kommen ungeprüft vom Client).
fn occupant(board: &Board, at: SlotRef) -> Option<ShipId> {
    (at.slot_index < SLOT_COUNT)
        .then(|| board.card(at))
        .flatten()
}

/// Ein autoritatives Match.
pub struct Match {
    id: String,
    board: Board,
    /// Sitz `i` ist `players[i]`; der zweite fehlt, bis jemand beitritt.
    players: Vec<Player>,
    round: u32,
    /// `None`, solange gespielt wird.
    winner: Option<Outcome>,
    /// Wer aufgegeben hat; `None`, wenn das Match auf dem Brett entschieden wurde oder noch läuft.
    surrendered_by: Option<Side>,
    /// Mischt die Decks und entscheidet über das Ziel der Plasma Frigate und die Wahl des Computergegners.
    rng: Box<dyn RngCore + Send>,
}

impl Match {
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_rng(id, StdRng::from_os_rng())
    }

    /// Ein Match mit vorgegebenem Zufall (Tests, Simulation).
    pub fn with_rng(id: impl Into<String>, rng: impl RngCore + Send + 'static) -> Self {
        Match {
            id: id.into(),
            board: Board::new(),
            players: Vec::new(),
            round: 1,
            winner: None,
            surrendered_by: None,
            rng: Box::new(rng),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.winner
    }

    /// Die besetzten Sitze.
    pub fn seats(&self) -> impl Iterator<Item = Seat> + use<> {
        Seat::ALL.into_iter().take(self.players.len())
    }

    fn player(&self, seat: Seat) -> &Player {
        &self.players[seat.index()]
    }

    fn player_mut(&mut self, seat: Seat) -> &mut Player {
        &mut self.players[seat.index()]
    }

    /// Sitzplatz für ein Token: bekannter Spieler, sonst ein freier Platz.
    pub fn join(&mut self, token: &str) -> Result<Seat, ErrorCode> {
        match self
            .players
            .iter()
            .find(|player| player.token.as_deref() == Some(token))
        {
            Some(known) => Ok(known.seat),
            None => self.seat_player(Some(token.to_owned())),
        }
    }

    /// Setzt den Computergegner auf den freien Platz; er wählt seinen Admiral sofort.
    pub fn add_bot(&mut self) -> Result<Seat, ErrorCode> {
        let seat = self.seat_player(None)?;
        let admiral = bot::choose_bot_admiral(self.rng.as_mut());
        self.player_mut(seat).admiral = Some(admiral);
        Ok(seat)
    }

    /// Nimmt den Computergegner wieder vom Platz, damit ein Mitspieler beitreten kann; nur, solange das Match nicht
    /// begonnen hat.
    pub fn remove_bot(&mut self) -> Result<(), ErrorCode> {
        if self.started() {
            return Err(ErrorCode::MatchStarted);
        }
        if !self.players.get(1).is_some_and(Player::is_bot) {
            return Err(ErrorCode::OpponentMissing);
        }
        self.players.pop();
        Ok(())
    }

    fn seat_player(&mut self, token: Option<String>) -> Result<Seat, ErrorCode> {
        let seat = *Seat::ALL
            .get(self.players.len())
            .ok_or(ErrorCode::MatchFull)?;
        let player = Player::new(token, seat, self.rng.as_mut());
        self.players.push(player);
        Ok(seat)
    }

    /// Wählt den Admiral des Sitzes, einmal und vor Runde 1, auch schon, solange der Gegner fehlt.
    pub fn choose_admiral(&mut self, seat: Seat, admiral: AdmiralId) -> Result<(), ErrorCode> {
        let player = self.player_mut(seat);
        if player.admiral.is_some() {
            return Err(ErrorCode::AdmiralAlreadyChosen);
        }
        player.admiral = Some(admiral);
        Ok(())
    }

    /// Runde 1 beginnt, sobald beide Spieler ihren Admiral gewählt haben; erst dann sieht jeder den des anderen.
    pub fn started(&self) -> bool {
        self.players.len() == 2 && self.players.iter().all(|player| player.admiral.is_some())
    }

    /// Nimmt den Zug an, wenn er gilt. Der Computergegner zieht, sobald der Mensch abgegeben hat; den fremden Plan
    /// kennt er nicht.
    pub fn submit(&mut self, seat: Seat, plan: TurnPlan) -> Result<(), ErrorCode> {
        let player = self.player(seat);
        if self.winner.is_some() {
            return Err(ErrorCode::MatchOver);
        }
        if player.admiral.is_none() {
            return Err(ErrorCode::AdmiralRequired);
        }
        if self.players.len() < 2 {
            return Err(ErrorCode::OpponentMissing);
        }
        if !self.started() {
            return Err(ErrorCode::OpponentChoosing);
        }
        if player.plan.is_some() {
            return Err(ErrorCode::AlreadySubmitted);
        }
        if player.draft.is_some() {
            return Err(ErrorCode::DraftPending);
        }
        self.validate(seat, &plan)?;
        self.player_mut(seat).plan = Some(plan);

        let waiting: Vec<Seat> = self
            .players
            .iter()
            .filter(|player| player.is_bot() && player.plan.is_none())
            .map(|player| player.seat)
            .collect();
        for bot in waiting {
            let view = self.bot_view(bot);
            let plan = bot::plan_bot_turn(&view, self.rng.as_mut());
            self.player_mut(bot).plan = Some(plan);
        }
        Ok(())
    }

    /// Die Wahl aus dem Nachschub: ein Schiff, wenn es welche gibt, und ein Befehl, wenn es welche gibt.
    pub fn draft(&mut self, seat: Seat, pick: DraftPick) -> Result<(), ErrorCode> {
        if self.winner.is_some() {
            return Err(ErrorCode::MatchOver);
        }
        let draft = self
            .player(seat)
            .draft
            .as_ref()
            .ok_or(ErrorCode::DraftUnavailable)?;
        // Ein Index ist Pflicht, sobald es etwas zu wählen gibt.
        let valid = |index: Option<usize>, count: usize| index.map_or(count == 0, |i| i < count);
        if !valid(pick.ship, draft.ships.len()) || !valid(pick.command, draft.commands.len()) {
            return Err(ErrorCode::DraftInvalid);
        }
        let player = &mut self.players[seat.index()];
        player.take_draft(pick, self.rng.as_mut());
        Ok(())
    }

    /// Was der Computergegner auf diesem Sitz beim Planen weiß: eigene Karten und, was jeder vom Gegner sieht.
    pub fn bot_view(&self, seat: Seat) -> BotView {
        let player = self.player(seat);
        BotView {
            board: self.board.clone(),
            round: self.round,
            side: player.side(),
            admiral: player.admiral(),
            ship_hand: player.ship_hand.clone(),
            command_hand: player.command_hand.clone(),
            opponent: self.player(seat.other()).public_view(true),
        }
    }

    /// Gibt das Match auf: Der Gegner gewinnt sofort, ein abgegebener Zug verfällt.
    pub fn surrender(&mut self, seat: Seat) -> Result<(), ErrorCode> {
        if self.players.len() < 2 {
            return Err(ErrorCode::OpponentMissing);
        }
        if self.winner.is_some() {
            return Err(ErrorCode::MatchOver);
        }
        self.surrendered_by = Some(seat.side());
        self.winner = Some(Outcome::Won(seat.side().opposite()));
        for player in &mut self.players {
            player.plan = None;
        }
        Ok(())
    }

    /// Die vollständige Zugprüfung. Ziele im Zug sind relativ zum Planenden (Lane und Index; die Seite folgt aus der
    /// Karte bzw. der Fähigkeit).
    fn validate(&self, seat: Seat, plan: &TurnPlan) -> Result<(), ErrorCode> {
        use ErrorCode as E;
        let board = &self.board;
        let player = self.player(seat);
        let side = player.side();
        let enemy = side.opposite();
        // Der Befehl kann den Zug selbst ändern (Schnellverlegung, Freie Hand); geprüft wird er unten.
        let card = plan
            .command
            .and_then(|command| player.command_hand.get(command.hand_index).copied());
        if plan.ships.len() > max_deploys(self.round, card) {
            return Err(E::TooManyShips);
        }
        match (self.round, plan.flagship) {
            (1, None) => return Err(E::FlagshipRequired),
            (2.., Some(_)) => return Err(E::FlagshipNotAllowed),
            _ => {}
        }

        // Die Schiffe, die in dieser Runde gelegt werden (in Runde 1 zuerst das Flaggschiff), landen vor Manöver und
        // Befehlen und belegen ihre Slots; jeder Deploy zählt die vorigen zur Lane- und Flottengrenze.
        let mut arrivals: Vec<PlannedArrival> = plan
            .flagship
            .map(|target| PlannedArrival {
                card: player.admiral().flagship(),
                target,
            })
            .into_iter()
            .collect();
        for (i, deploy) in plan.ships.iter().enumerate() {
            let card = player
                .ship_hand
                .get(deploy.hand_index)
                .ok_or(E::ShipNotInHand)?;
            if plan.ships[..i]
                .iter()
                .any(|other| other.hand_index == deploy.hand_index)
            {
                return Err(E::ShipNotInHand);
            }
            arrivals.push(PlannedArrival {
                card: *card,
                target: deploy.target,
            });
        }
        let deploys: Vec<SlotTarget> = arrivals.iter().map(|arrival| arrival.target).collect();
        for (i, target) in deploys.iter().enumerate() {
            if !board.can_place(target.on(side), &deploys[..i]) {
                return Err(E::SlotUnavailable);
            }
        }
        let arriving = |target: SlotTarget| {
            arrivals
                .iter()
                .find(|arrival| arrival.target == target)
                .map(|arrival| arrival.card)
        };

        // Das Grundmanöver geht in einen freien Slot; nur die Mirage darf in einen belegten und tauscht dann.
        let mut swap = false;
        let mut maneuver: Option<PlannedMove> = None;
        if let Some(planned) = plan.maneuver {
            let from = planned.ship.on(side);
            let to = planned.destination.on(side);
            let ship = occupant(board, from).ok_or(E::ManeuverNotOwnShip)?;
            swap = to != from && occupant(board, to).is_some();
            if swap && ship != ShipId::Mirage {
                return Err(E::SwapNotAllowed);
            }
            if !swap && !board.can_move(from, planned.destination, &deploys, None) {
                return Err(E::ManeuverDestinationUnavailable);
            }
            // Der Phasentausch ändert die Schiffszahl keiner Lane und steht dem Befehl nicht im Weg.
            if !swap {
                maneuver = Some(PlannedMove { from, to });
            }
        }

        // Manöver und Bewegungsbefehl dürfen nicht im selben Slot landen und zusammen mit den Deploys die Lane-Grenze
        // nicht überschreiten; `can_move` prüft das beim Befehl mit den Deploys und dem Manöver.
        let mut moved: Option<SlotTarget> = None;
        if let Some(command) = plan.command {
            let card = card.ok_or(E::CommandNotInHand)?;
            match (card.needs_target(), command.target) {
                (true, None) => return Err(E::TargetRequired),
                (false, Some(_)) => return Err(E::TargetNotAllowed),
                _ => {}
            }
            if let Some(target) = command.target {
                // Torpedo und Durchschlag zielen auf einen Slot der Gegnerseite, auch einen leeren; Wärmesucher und
                // Traktorstrahl auf ein gegnerisches Schiff; alle übrigen auf ein eigenes Schiff, auch eines, das in
                // dieser Runde landet (eingeplante Schiffe zählen nur auf der eigenen Seite).
                let on_enemy = card.targets_enemy_side();
                let at = target.on(if on_enemy { enemy } else { side });
                let exists = target.slot_index < SLOT_COUNT;
                let ship =
                    occupant(board, at).or_else(|| (!on_enemy).then(|| arriving(target)).flatten());
                if on_enemy && !exists {
                    return Err(E::TargetNotEnemySlot);
                }
                if card.targets_enemy_ship() && ship.is_none() {
                    return Err(E::TargetNotEnemyShip);
                }
                if !on_enemy && ship.is_none() {
                    return Err(E::TargetNotOwnShip);
                }
                if !card.can_target_lane(target.lane) {
                    return Err(E::TargetWrongLane);
                }
                if ship.is_some_and(|ship| !card.can_target_ship(ship)) {
                    return Err(E::TargetWrongShip);
                }
            }
            let moves = card.card().moves.is_some();
            match (moves, command.destination) {
                (true, None) => return Err(E::DestinationRequired),
                (false, Some(_)) => return Err(E::DestinationNotAllowed),
                _ => {}
            }
            if let (Some(target), Some(destination)) = (command.target, command.destination) {
                // Das Grundmanöver bewegt sein Schiff, der Phasentausch auch das Schiff im Ziel-Slot; keins davon
                // verlegt der Befehl.
                let maneuvered = plan.maneuver.is_some_and(|planned| {
                    planned.ship == target || (swap && planned.destination == target)
                });
                if maneuvered {
                    return Err(E::ManeuverConflict);
                }
                let to = SlotTarget::new(card.destination_lane(target.lane), destination);
                // Bewegt sich ein gelegtes Schiff, ist `from` sein noch leerer Slot.
                if !board.can_move(target.on(side), to, &deploys, maneuver) {
                    return Err(E::DestinationUnavailable);
                }
                moved = Some(to);
            }
        }

        // Die Admiralsfähigkeit statt des Grundmanövers, mit Freier Hand dazu; das Trugbild erscheint nicht, wo der
        // Befehl oder das Manöver ein Schiff hinbringt.
        if let Some(power) = plan.power {
            if plan.maneuver.is_some() && !allows_maneuver_and_power(card) {
                return Err(E::PowerAndManeuver);
            }
            let blocked: Vec<SlotTarget> = moved
                .into_iter()
                .chain(plan.maneuver.map(|planned| planned.destination))
                .collect();
            let targets = power_targets(board, side, player.admiral().power(), &arrivals, &blocked);
            if !targets.iter().any(|at| at.target() == power.target) {
                return Err(E::PowerTargetInvalid);
            }
        }
        if let Some(mark) = plan.mark {
            if !can_mark(board, side) {
                return Err(E::MarkUnavailable);
            }
            if !mark_targets(board, side)
                .iter()
                .any(|at| at.target() == mark)
            {
                return Err(E::MarkTargetInvalid);
            }
        }

        // Feuerbefehle: je eigenem Kampfschiff höchstens einer, auch für ein gelegtes, nicht für das Schiff des
        // Grundmanövers (es feuert nicht; beim Phasentausch feuern beide); Ziel ist eine Spalte der Gegnerseite, auch
        // eine leere, oder ein gegnerisches Schiff beim Planen, auch ein gedecktes.
        for (i, order) in plan.fire.iter().enumerate() {
            let shooter = occupant(board, order.ship.on(side)).or_else(|| arriving(order.ship));
            let repeated = plan.fire[..i].iter().any(|other| other.ship == order.ship);
            let maneuvering = !swap
                && plan
                    .maneuver
                    .is_some_and(|planned| planned.ship == order.ship);
            if !shooter.is_some_and(|ship| ship.atk() > 0) || repeated || maneuvering {
                return Err(E::FireShipInvalid);
            }
            let valid = match order.target {
                FireTarget::Column(column) => column < SLOT_COUNT,
                FireTarget::Ship(target) => occupant(board, target.on(enemy)).is_some(),
            };
            if !valid {
                return Err(E::FireTargetInvalid);
            }
        }
        Ok(())
    }

    pub fn is_round_complete(&self) -> bool {
        self.players.len() == 2 && self.players.iter().all(|player| player.plan.is_some())
    }

    /// Deckt beide Züge auf und löst die Runde auf ([`resolve_turns`]; sie bricht mit einem gefallenen Flaggschiff
    /// sofort ab). Ist danach kein Flaggschiff gefallen, beginnt die nächste Runde mit dem Overdrive und dem
    /// Nachschub, den der Computergegner gleich wählt. Gibt die Ereignisse aus Sicht von [`Seat::First`] zurück.
    pub fn resolve_round(&mut self) -> Vec<RoundEvent> {
        let turns: Vec<Turn> = self.players.iter_mut().map(Player::reveal).collect();
        let players = &mut self.players;
        let mut commander = |side: Side, ships: u32, commands: u32| {
            players[Seat::of(side).index()].draw_extra(ships, commands)
        };
        let rng: &mut dyn RngCore = self.rng.as_mut();
        let events = resolve_turns(&mut self.board, &turns, &mut commander, Some(rng));
        self.winner = self.decide_winner();
        if self.winner.is_none() {
            self.round += 1;
            // Gilt schon beim Planen der neuen Runde, damit der Computergegner mit denselben Werten rechnet.
            self.board.overdrive = overdrive_bonus(self.round);
            for seat in Seat::ALL {
                let salvage = self.salvaged(seat.side(), &events);
                let player = self.player_mut(seat);
                player.offer_draft(salvage);
                if let Some(offer) = player.draft_offer().filter(|_| player.is_bot()) {
                    let pick = bot::plan_bot_draft(&self.bot_view(seat), &offer);
                    self.draft(seat, pick)
                        .expect("der Computergegner wählt gültig");
                }
            }
        }
        events
    }

    /// Ein eigener Scrap Raider hat in der Runde Schrott geborgen, und auf der Seite steht danach noch einer.
    fn salvaged(&self, side: Side, events: &[RoundEvent]) -> bool {
        let scrap = events.iter().any(|event| {
            matches!(event, RoundEvent::Ability { source, effect: AbilityEffect::Salvage, .. } if source.side == side)
        });
        scrap
            && self
                .board
                .ships(side)
                .any(|at| self.board.ship(at).card == ShipId::ScrapRaider)
    }

    /// Wessen Flaggschiff gefallen ist, hat verloren; fallen beide in derselben Runde, ist es unentschieden.
    fn decide_winner(&self) -> Option<Outcome> {
        let fallen = |side| self.board.flagship(side).is_none();
        match (fallen(Side::Player), fallen(Side::Enemy)) {
            (true, true) => Some(Outcome::Draw),
            (true, false) => Some(Outcome::Won(Side::Enemy)),
            (false, true) => Some(Outcome::Won(Side::Player)),
            (false, false) => None,
        }
    }

    fn result_for(&self, seat: Seat) -> Option<MatchResult> {
        self.winner.map(|outcome| match outcome {
            Outcome::Draw => MatchResult::Draw,
            Outcome::Won(side) if side == seat.side() => MatchResult::Won,
            Outcome::Won(_) => MatchResult::Lost,
        })
    }

    /// Die Sicht des Sitzes; für [`Seat::Second`] gespiegelt.
    pub fn view(&self, seat: Seat) -> PlayerView {
        let player = self.player(seat);
        PlayerView {
            match_id: self.id.clone(),
            round: self.round,
            admiral: player.admiral,
            board: seat.mirror(self.board.snapshot()),
            hive_launches: seat.mirror(self.board.hive_launches),
            ship_hand: player.ship_hand.clone(),
            command_hand: player.command_hand.clone(),
            ship_deck_size: player.ship_deck.len(),
            command_deck_size: player.command_deck.len(),
            submitted_plan: player.plan.clone(),
            draft: player.draft_offer(),
            opponent: self
                .players
                .get(seat.other().index())
                .map(|opponent| opponent.public_view(self.started())),
            result: self.result_for(seat),
            surrendered: self.surrendered_by.is_some(),
        }
    }

    /// Die Ereignisse der Runde aus Sicht des Sitzes; für [`Seat::Second`] gespiegelt.
    pub fn events_for(&self, seat: Seat, events: &[RoundEvent]) -> Vec<RoundEvent> {
        match seat {
            Seat::First => events.to_vec(),
            Seat::Second => events.iter().copied().map(Mirror::mirrored).collect(),
        }
    }
}
