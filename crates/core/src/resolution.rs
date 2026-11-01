//! Die Auflösung einer Runde: Deploy samt Fähigkeiten, Manöver, Effects, Combat, Cleanup. Portiert aus
//! `game_mechanics.gd`. Der Server löst damit die aufgedeckten Züge auf, der Client rechnet mit denselben Regeln die
//! Zielvorschau beim Planen. Jede Phase arbeitet auf dem Brett aus Sicht von Seat 0 und hängt ihre Ereignisse an
//! `events` an. Dazu kommen die Fähigkeiten der Flaggschiffe und Admirale ([`crate::admirals`]).

use std::cmp::Reverse;
use std::iter;

use rand::{Rng, RngCore};

use crate::admirals::{ABILITY_VALUES, PowerId};
use crate::board::{
    Aim, Board, Jump, LaneType, SLOT_COUNT, Ship, Side, SlotRef, SlotTarget, lane_slots, nearest,
};
use crate::cards::{CommandId, CommandType, ShipId};
use crate::powers::{PlannedArrival, railgun_fires};
use crate::protocol::{
    AbilityEffect, CommandEffect, FireOrder, FireTarget, Maneuver, Miss, RoundEvent, ShipRef,
    ShipStats,
};

/// Was ein zusätzliches Ziehen auf die Hand gebracht hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Drawn {
    pub ships: u32,
    pub commands: u32,
}

/// Wer für eine Seite Karten zieht (Plündern 1 + 1, Gegenmaßnahmen 0 + 1). Jede Closure
/// `FnMut(Side, u32, u32) -> Drawn` ist ein `Commander`.
pub trait Commander {
    /// Zieht zusätzlich für `side`; zählt, was auf der Hand gelandet ist.
    fn draw_extra(&mut self, side: Side, ships: u32, commands: u32) -> Drawn;
}

impl<F: FnMut(Side, u32, u32) -> Drawn> Commander for F {
    fn draw_extra(&mut self, side: Side, ships: u32, commands: u32) -> Drawn {
        self(side, ships, commands)
    }
}

/// Der aufgedeckte Befehl im Zug: die Karte statt ihres Hand-Index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurnCommand {
    pub card: CommandId,
    pub target: Option<SlotTarget>,
    pub destination: Option<usize>,
}

/// Die Admiralsfähigkeit im Zug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowerUse {
    pub id: PowerId,
    pub target: SlotTarget,
}

/// Der aufgedeckte Zug einer Seite: wie [`crate::protocol::TurnPlan`], aber mit den Karten statt ihrer Hand-Indizes.
/// Er ist geprüft; eigene Ziele beziehen sich auf die Stellung vor dem Manöver. Bauen mit
/// `Turn { …, ..Turn::new(side) }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub side: Side,
    pub ships: Vec<PlannedArrival>,
    /// Runde 1: das Flaggschiff des Admirals, das zusätzlich gelegt wird.
    pub flagship: Option<PlannedArrival>,
    pub command: Option<TurnCommand>,
    pub maneuver: Option<Maneuver>,
    /// Die Admiralsfähigkeit statt des Grundmanövers.
    pub power: Option<PowerUse>,
    /// Pendulum: das markierte gegnerische Schiff (Slot beim Planen).
    pub mark: Option<SlotTarget>,
    /// Feuerbefehle (Slots beim Planen); ein Kampfschiff ohne Feuerbefehl feuert nicht.
    pub fire: Vec<FireOrder>,
}

impl Turn {
    /// Ein leerer Zug: Die Seite passt.
    pub fn new(side: Side) -> Self {
        Turn {
            side,
            ships: Vec::new(),
            flagship: None,
            command: None,
            maneuver: None,
            power: None,
            mark: None,
            fire: Vec::new(),
        }
    }
}

/// Ein aufgedeckter Befehl mit seinen Slots zur Stellung nach den Manövern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayedCommand {
    pub side: Side,
    pub card: CommandId,
    /// Das eigene Zielschiff, beim Torpedo der gewählte Slot der Gegnerseite (auch leer), beim Durchschlag der
    /// Strike-Slot der Slot-Nummer, bei Wärmesucher und Traktorstrahl das erfasste Schiff beim Aufdecken.
    pub target: Option<SlotRef>,
    /// Bei Befehlen, die ihr Ziel verlegen: der freie Slot der Ziel-Lane.
    pub destination: Option<SlotRef>,
}

/// Das Schiff im Slot als Verweis für Ereignisse.
pub fn ship_ref(board: &Board, at: SlotRef) -> ShipRef {
    ShipRef::new(at, board.ship(at).card)
}

fn ship_stats(ship: &Ship) -> ShipStats {
    ShipStats {
        hp: ship.hp,
        shield: ship.shield,
    }
}

fn stats(board: &Board, at: SlotRef) -> ShipStats {
    ship_stats(board.ship(at))
}

/// Element mit dem höchsten Wert; bei Gleichstand das erste (bei Schiffen: in [`Board::ships`]-Reihenfolge).
pub fn highest<T>(items: impl IntoIterator<Item = T>, value: impl Fn(&T) -> i32) -> Option<T> {
    items.into_iter().min_by_key(|item| Reverse(value(item)))
}

/// Die Seiten, deren Railgun in dieser Runde feuert ([`railgun_fires`]).
pub fn firing_railguns(board: &Board) -> Vec<Side> {
    Side::ALL
        .into_iter()
        .filter(|&side| railgun_fires(board, side))
        .collect()
}

// --- Runde ---

/// Löst eine Runde mit den Zügen beider Seiten auf und gibt ihre Ereignisse zurück: Deploy samt Flaggschiff,
/// Feuerbefehle ([`Ship::aim`]), Admiralsfähigkeiten, Erfassung durch Wärmesucher und Traktorstrahl
/// ([`Ship::locks`]), Fähigkeiten beim Deploy, Markierung der Pendulum, Manöver und Phasentausch, Wächterschild,
/// Effects, Combat, Cleanup, Brutkammer. Fällt ein Flaggschiff, ist das Spiel sofort zu Ende: Die Runde bricht nach
/// dem Schritt ab. Ohne `rng` (Zielvorschau) feuern Plasma Frigates beim Deploy nicht, weil ihr Ziel Zufall ist.
pub fn resolve_turns(
    board: &mut Board,
    turns: &[Turn],
    commander: &mut dyn Commander,
    rng: Option<&mut dyn RngCore>,
) -> Vec<RoundEvent> {
    let mut events = Vec::new();
    // Die Railgun feuert auf eine Markierung aus der Vorrunde; eine Markierung dieser Runde gilt erst in der nächsten.
    let railguns = firing_railguns(board);
    // Ein Phasentausch ist ein Manöver auf ein belegtes Feld; das steht vor den Deploys fest.
    let swaps: Vec<Side> = turns
        .iter()
        .filter(|turn| {
            turn.maneuver
                .is_some_and(|maneuver| board.is_occupied(maneuver.destination.on(turn.side)))
        })
        .map(|turn| turn.side)
        .collect();
    let mut deployed = Vec::new();
    for turn in turns {
        let arrivals: Vec<_> = turn.flagship.iter().chain(&turn.ships).collect();
        for arrival in &arrivals {
            let at = arrival.target.on(turn.side);
            board
                .place(at, arrival.card)
                .expect("geprüfter Zug: der Slot ist frei")
                .arrived = true;
            deployed.push(at);
            events.push(RoundEvent::Deploy {
                side: turn.side,
                card_id: arrival.card,
                target: arrival.target,
            });
        }
        if arrivals.is_empty() {
            events.push(RoundEvent::Pass { side: turn.side });
        }
    }
    // Die Feuerbefehle greifen, sobald alle Schiffe stehen, und folgen dem Schützen bei jeder Bewegung, Schiffsfeuer
    // auch seinem Ziel; die Spalte bleibt.
    for turn in turns {
        for &order in &turn.fire {
            bind_fire(board, turn.side, order);
        }
    }
    for turn in turns {
        if let Some(power) = turn.power {
            resolve_power(board, turn.side, power, &mut events);
        }
    }
    // Wärmesucher und Traktorstrahl erfassen ihr Schiff, bevor sich etwas bewegt; die Erfassung folgt ihm.
    for turn in turns {
        if let Some(command) = turn.command
            && let Some(target) = command.target
            && command.card.targets_enemy_ship()
            && let Some(ship) = &mut board[target.on(turn.side.opposite())]
        {
            ship.locks.push(command.card);
        }
    }
    let reserved: Vec<SlotRef> = turns.iter().flat_map(move_destinations).collect();
    resolve_deploy_abilities(board, &deployed, &reserved, rng, &mut events);
    if flagship_fallen(board, &events) {
        return end_match(board, events);
    }
    for turn in turns {
        if let Some(mark) = turn.mark {
            resolve_mark(board, turn.side, mark, &mut events);
        }
    }

    // Jede Seite bewegt nur eigene Schiffe, die Manöver kommen sich also nicht in die Quere. Ein Schiff, das schon
    // beim Deploy gefallen ist, bewegt sich nicht mehr; fiel beim Phasentausch der Partner, bleibt die Mirage stehen
    // und feuert. Ein Befehl auf ein eigenes Schiff bleibt dann auf seinem Slot.
    let mut swapped = Vec::new();
    let mut stayed = Vec::new();
    for turn in turns {
        let Some(maneuver) = turn.maneuver else {
            continue;
        };
        let from = maneuver.ship.on(turn.side);
        let to = maneuver.destination.on(turn.side);
        if !board.is_occupied(from) || (swaps.contains(&turn.side) && !board.is_occupied(to)) {
            stayed.push(turn.side);
        } else if board.is_occupied(to) {
            resolve_swap(board, from, to, &mut events);
            swapped.push(turn.side);
        } else {
            resolve_maneuver(board, from, to, &mut events);
        }
    }
    resolve_guardian_shields(board, &mut events);

    let mut commands = Vec::new();
    for turn in turns {
        let Some(command) = turn.command else {
            continue;
        };
        let maneuver = turn.maneuver.filter(|_| !stayed.contains(&turn.side));
        let swapped = swapped.contains(&turn.side);
        commands.push(reveal_command(
            board,
            turn.side,
            command,
            maneuver,
            swapped,
            &mut events,
        ));
    }
    resolve_effects(board, &commands, commander, &mut events);
    if flagship_fallen(board, &events) {
        return end_match(board, events);
    }
    // Bewegt sich die Pendulum (auch per Kurzsprung), fällt ihr Schuss aus: in dieser Runde und, wenn sie gerade
    // markiert hat, in der nächsten.
    let moved: Vec<Side> = Side::ALL
        .into_iter()
        .filter(|&side| {
            board
                .ships(side)
                .map(|at| board.ship(at))
                .chain(
                    board
                        .jumping
                        .iter()
                        .filter(|jump| jump.slot.side == side)
                        .map(|jump| &jump.ship),
                )
                .any(|ship| ship.card == ShipId::Pendulum && ship.moved)
        })
        .collect();
    let firing: Vec<Side> = railguns
        .iter()
        .copied()
        .filter(|side| !moved.contains(side))
        .collect();
    resolve_combat(board, &mut events, &firing);
    if flagship_fallen(board, &events) {
        return end_match(board, events);
    }
    resolve_cleanup(board, &mut events);
    if flagship_fallen(board, &events) {
        return events;
    }
    resolve_brood_chamber(board, &mut events);
    // Nach dem Schuss, auch einem ausgefallenen, ist die Markierung verbraucht.
    for side in Side::ALL {
        if !railguns.contains(&side) && !moved.contains(&side) {
            continue;
        }
        let marked: Vec<SlotRef> = board.ships(side.opposite()).collect();
        for at in marked {
            board.ship_mut(at).marked = false;
        }
    }
    events
}

/// Ein Flaggschiff ist in dieser Runde gefallen: steht zerstört auf dem Brett oder hat es schon verlassen.
fn flagship_fallen(board: &Board, events: &[RoundEvent]) -> bool {
    Side::ALL.into_iter().any(|side| {
        board
            .flagship(side)
            .is_some_and(|at| board.is_destroyed(at))
    }) || events
        .iter()
        .any(|event| matches!(event, RoundEvent::Destroyed { ship } if ship.card_id.is_flagship()))
}

/// Das Spiel ist mit dem gefallenen Flaggschiff zu Ende, der Rest der Runde entfällt: Die Zerstörten verlassen das
/// Brett, Schiffe im Kurzsprung stehen ohne Auftritt wieder in ihrem Slot.
fn end_match(board: &mut Board, mut events: Vec<RoundEvent>) -> Vec<RoundEvent> {
    let destroyed: Vec<SlotRef> = board
        .all_ships()
        .filter(|&at| board.is_destroyed(at))
        .collect();
    for at in destroyed {
        events.push(RoundEvent::Destroyed {
            ship: ship_ref(board, at),
        });
        board[at] = None;
    }
    for jump in std::mem::take(&mut board.jumping) {
        board[jump.slot] = Some(jump.ship);
    }
    events
}

/// Feuerbefehl: Der Schütze merkt sich die Spalte oder die Kennung des gegnerischen Schiffs im Ziel-Slot
/// ([`Ship::aim`]).
pub fn bind_fire(board: &mut Board, side: Side, order: FireOrder) {
    let shooter = order.ship.on(side);
    if !board.is_occupied(shooter) {
        return;
    }
    let aim = match order.target {
        FireTarget::Column(column) => Some(Aim::Column(column)),
        FireTarget::Ship(target) => board[target.on(side.opposite())]
            .as_ref()
            .map(|ship| Aim::Ship(ship.uid)),
    };
    if aim.is_some() {
        board.ship_mut(shooter).aim = aim;
    }
}

/// Die Ziel-Slots der Bewegungen im Zug: Grundmanöver und Bewegungsbefehl (der nie das manövrierte Schiff verlegt).
fn move_destinations(turn: &Turn) -> Vec<SlotRef> {
    let moved = turn.command.and_then(|command| {
        Some(SlotTarget::new(
            command.card.destination_lane(command.target?.lane),
            command.destination?,
        ))
    });
    turn.maneuver
        .map(|maneuver| maneuver.destination)
        .into_iter()
        .chain(moved)
        .map(|target| target.on(turn.side))
        .collect()
}

/// Deckt den Befehl auf: Ein eigenes Ziel folgt dem Manöver, ein Slot der Gegnerseite (Torpedo) bleibt stehen, der
/// Durchschlag meldet den Strike-Slot seiner Slot-Nummer, ein erfasstes Schiff (Wärmesucher, Traktorstrahl) seinen
/// Slot jetzt, `None`, wenn es nicht mehr auf dem Brett steht.
fn reveal_command(
    board: &Board,
    side: Side,
    command: TurnCommand,
    maneuver: Option<Maneuver>,
    swapped: bool,
    events: &mut Vec<RoundEvent>,
) -> PlayedCommand {
    let TurnCommand {
        card,
        target,
        destination,
    } = command;
    let enemy = side.opposite();
    let slot = target.and_then(|target| {
        if card.targets_enemy_ship() {
            locked(board, enemy, card)
        } else if card.targets_enemy_column() {
            Some(SlotRef::new(enemy, LaneType::Strike, target.slot_index))
        } else if card.targets_enemy_slot() {
            Some(target.on(enemy))
        } else {
            Some(after_maneuver(maneuver, target, swapped).on(side))
        }
    });
    events.push(RoundEvent::Command {
        side,
        card_id: card,
        target: slot,
    });
    PlayedCommand {
        side,
        card,
        target: slot,
        destination: slot
            .zip(destination)
            .map(|(slot, index)| SlotRef::new(slot.side, card.destination_lane(slot.lane), index)),
    }
}

/// Das Schiff der Seite `side`, das der Befehl `card` der Gegenseite erfasst hat; `None`, wenn es nicht (mehr) auf
/// dem Brett steht.
fn locked(board: &Board, side: Side, card: CommandId) -> Option<SlotRef> {
    board
        .ships(side)
        .find(|&at| board.ship(at).locks.contains(&card))
}

/// Wo ein eigenes Befehlsziel nach dem Manöver steht: Zielt der Befehl auf das manövrierte Schiff, folgt er ihm;
/// nach einem Phasentausch folgt er auch dem Schiff, das mit der Mirage getauscht hat.
fn after_maneuver(maneuver: Option<Maneuver>, target: SlotTarget, swapped: bool) -> SlotTarget {
    match maneuver {
        Some(maneuver) if target == maneuver.ship => maneuver.destination,
        Some(maneuver) if swapped && target == maneuver.destination => maneuver.ship,
        _ => target,
    }
}

// --- Deploy ---

/// Die Admiralsfähigkeit, gleich nach den Deploys: Das Trugbild erscheint in seinem Strike-Slot; die übrigen liegen
/// in dieser Runde auf dem gewählten Schiff ([`Ship::powers`]) und folgen ihm, wenn es sich bewegt.
pub fn resolve_power(board: &mut Board, side: Side, power: PowerUse, events: &mut Vec<RoundEvent>) {
    let target_side = if power.id.targets_enemy() {
        side.opposite()
    } else {
        side
    };
    let at = power.target.on(target_side);
    events.push(RoundEvent::Power {
        side,
        power: power.id,
        target: at,
    });
    if power.id != PowerId::Decoy {
        if let Some(ship) = &mut board[at] {
            ship.powers.push(power.id);
        }
        return;
    }
    board.put(at, ShipId::Decoy).arrived = true;
    events.push(RoundEvent::Decoy {
        ship: ship_ref(board, at),
    });
}

/// Fähigkeiten beim Deploy, sobald beide Seiten gelegt haben (`deployed`, in Deploy-Reihenfolge): Erst startet die
/// Drohne jedes Titan Carriers in den freien Slot seiner Lane, der ihm am nächsten liegt. `reserved` sind die
/// Ziel-Slots der Bewegungen dieser Runde: Die Drohne meidet sie, die Bewegungen bleiben also gültig. Zu Lane- und
/// Flottengrenze zählt sie nicht; nur wenn kein Slot der Lane frei ist, gibt es keine Drohne. Dann schießt jede Plasma
/// Frigate 3 Schaden auf ein zufälliges gegnerisches Schiff der gleichnamigen Lane, gedeckt oder nicht, auch auf eine
/// frische Drohne. Ohne `rng` (Zielvorschau) feuern die Plasma Frigates nicht.
pub fn resolve_deploy_abilities(
    board: &mut Board,
    deployed: &[SlotRef],
    reserved: &[SlotRef],
    rng: Option<&mut dyn RngCore>,
    events: &mut Vec<RoundEvent>,
) {
    let with_card = |board: &Board, card: ShipId| -> Vec<SlotRef> {
        deployed
            .iter()
            .copied()
            .filter(|&at| board.card(at) == Some(card))
            .collect()
    };
    for carrier in with_card(board, ShipId::TitanCarrier) {
        let free = lane_slots(carrier.side, carrier.lane)
            .filter(|at| board.is_free(*at) && !reserved.contains(at));
        let Some(slot) = nearest(free, carrier.slot_index) else {
            continue;
        };
        board.put(slot, ShipId::Drone).arrived = true;
        events.push(RoundEvent::Spawn {
            source: ship_ref(board, carrier),
            ship: ship_ref(board, slot),
        });
    }
    let Some(rng) = rng else {
        return;
    };
    for frigate in with_card(board, ShipId::PlasmaFrigate) {
        let targets: Vec<SlotRef> = board
            .occupied(frigate.side.opposite(), frigate.lane)
            .filter(|&at| board.is_standing(at))
            .collect();
        if targets.is_empty() {
            continue;
        }
        let target = targets[rng.random_range(0..targets.len())];
        let ship = ship_ref(board, target);
        board.ship_mut(target).damage(3);
        events.push(RoundEvent::Ability {
            source: ship_ref(board, frigate),
            ship,
            effect: AbilityEffect::Damage { amount: 3 },
            after: stats(board, target),
        });
    }
    remove_destroyed(board, events);
}

/// Pendulum: markiert nach den Deploys das gegnerische Schiff im geplanten Slot, sofern es noch steht; die
/// Markierung folgt ihm.
fn resolve_mark(board: &mut Board, side: Side, target: SlotTarget, events: &mut Vec<RoundEvent>) {
    let at = target.on(side.opposite());
    let Some(ship) = &mut board[at] else {
        return;
    };
    ship.marked = true;
    events.push(RoundEvent::Mark {
        side,
        ship: ship_ref(board, at),
    });
}

// --- Manöver ---

/// Grundmanöver: Das Schiff wechselt in einen freien Slot, in der anderen oder derselben Lane, und feuert dafür in
/// dieser Runde nicht.
pub fn resolve_maneuver(
    board: &mut Board,
    from: SlotRef,
    to: SlotRef,
    events: &mut Vec<RoundEvent>,
) {
    let ship = ship_ref(board, from);
    board.move_ship(from, to).maneuvered = true;
    events.push(RoundEvent::Maneuver {
        side: ship.side,
        ship,
        to: to.target(),
    });
}

/// Phasentausch statt Grundmanöver: Die Mirage und das Schiff im Ziel-Slot tauschen die Plätze samt Zustand und
/// feuern beide.
pub fn resolve_swap(
    board: &mut Board,
    mirage: SlotRef,
    other: SlotRef,
    events: &mut Vec<RoundEvent>,
) {
    events.push(RoundEvent::Swap {
        side: mirage.side,
        a: ship_ref(board, mirage),
        b: ship_ref(board, other),
    });
    board.swap(mirage, other);
}

/// Bastion (Wächterschild): Nach den Manövern bekommt sie Schild für diese Runde (er verfällt im Cleanup); steht sie
/// in der Command-Lane, auch das Schiff im Strike-Slot direkt vor ihr, mit eigenem Wert.
pub fn resolve_guardian_shields(board: &mut Board, events: &mut Vec<RoundEvent>) {
    for side in Side::ALL {
        let Some(bastion) = board
            .flagship(side)
            .filter(|&at| board.ship(at).card == ShipId::Bastion)
        else {
            continue;
        };
        let front = Some(bastion.in_lane(LaneType::Strike))
            .filter(|front| bastion.lane == LaneType::Command && board.is_occupied(*front));
        let shielded = iter::once((bastion, ABILITY_VALUES.guardian_shield))
            .chain(front.map(|front| (front, ABILITY_VALUES.guardian_shield_front)));
        for (at, amount) in shielded {
            board.ship_mut(at).add_shield(amount);
            events.push(RoundEvent::Ability {
                source: ship_ref(board, bastion),
                ship: ship_ref(board, at),
                effect: AbilityEffect::Shield { amount },
                after: stats(board, at),
            });
        }
    }
}

// --- Effects ---

/// Die Befehle nach [`crate::cards::COMMAND_ORDER`]. Gegenmaßnahmen (Spezial) lassen einen Schwächungs- oder
/// Angriffsbefehl der Gegenseite verpuffen, sonst ziehen sie einen Befehl. Alle Rammstöße der Runde wirken
/// gleichzeitig.
pub fn resolve_effects(
    board: &mut Board,
    commands: &[PlayedCommand],
    commander: &mut dyn Commander,
    events: &mut Vec<RoundEvent>,
) {
    let mut ordered: Vec<usize> = (0..commands.len()).collect();
    ordered.sort_by_key(|&index| commands[index].card.command_type());
    let rams: Vec<PlayedCommand> = ordered
        .iter()
        .map(|&index| commands[index])
        .filter(|command| command.card == CommandId::CollisionCourse)
        .collect();
    let mut rammed = false;
    let mut countered = Vec::new();
    for index in ordered {
        if countered.contains(&index) {
            continue;
        }
        let command = commands[index];
        match command.card {
            CommandId::Countermeasures => {
                let hostile = commands.iter().position(|other| {
                    other.side != command.side
                        && matches!(
                            other.card.command_type(),
                            CommandType::Debuff | CommandType::Attack
                        )
                });
                if let Some(hostile) = hostile {
                    countered.push(hostile);
                    events.push(RoundEvent::Countered {
                        side: command.side,
                        card_id: command.card,
                        countered: commands[hostile].card,
                    });
                } else {
                    let drawn = commander.draw_extra(command.side, 0, 1);
                    events.push(RoundEvent::Draw {
                        side: command.side,
                        card_id: command.card,
                        ships: drawn.ships,
                        commands: drawn.commands,
                    });
                }
            }
            // Alle Rammstöße beim ersten auf einmal.
            CommandId::CollisionCourse => {
                if !rammed {
                    rammed = true;
                    resolve_rams(board, &rams, events);
                }
            }
            _ => {
                if !apply_command(board, command, commander, events) {
                    events.push(RoundEvent::NoEffect {
                        side: command.side,
                        card_id: command.card,
                    });
                }
            }
        }
    }
    remove_destroyed(board, events);
}

/// Rammkurs: Das Ziel jedes Rammers ist das ungedeckte gegnerische Schiff mit derselben Slot-Nummer, sonst das
/// nächstgelegene (ein Trugbild deckt und kann gerammt werden). Das Ziel nimmt die HP des Rammers, der Rammer die des
/// Ziels, beides vor dem Schild. Ziele und Beträge stehen mit dem Stand vor dem ersten Rammstoß fest; rammen sich zwei
/// Schiffe gegenseitig, ist das eine Kollision, gemeldet beim ersten Befehl. Der Rammer feuert in dieser Runde nicht.
fn resolve_rams(board: &mut Board, commands: &[PlayedCommand], events: &mut Vec<RoundEvent>) {
    struct Ram {
        side: Side,
        rammer: SlotRef,
        target: Option<SlotRef>,
        dealt: i32,
        taken: i32,
    }
    let rams: Vec<Ram> = commands
        .iter()
        .map(|command| {
            let rammer = command.target.expect("Rammkurs hat ein Ziel");
            let target = if board.is_standing(rammer) {
                let targets = board
                    .exposed_ships(command.side.opposite(), false)
                    .filter(|&at| !board.is_destroyed(at));
                nearest(targets, rammer.slot_index)
            } else {
                None
            };
            Ram {
                side: command.side,
                rammer,
                target,
                dealt: board[rammer].as_ref().map_or(0, |ship| ship.hp),
                taken: target.map_or(0, |at| board.ship(at).hp),
            }
        })
        .collect();
    for ram in rams.iter().filter(|ram| ram.target.is_some()) {
        board.ship_mut(ram.rammer).disabled = true;
    }
    for (index, ram) in rams.iter().enumerate() {
        let Some(target) = ram.target else {
            events.push(RoundEvent::NoEffect {
                side: ram.side,
                card_id: CommandId::CollisionCourse,
            });
            continue;
        };
        if rams[..index]
            .iter()
            .any(|other| other.rammer == target && other.target == Some(ram.rammer))
        {
            continue;
        }
        let ship = ship_ref(board, ram.rammer);
        let struck = ship_ref(board, target);
        board.ship_mut(target).damage(ram.dealt);
        board.ship_mut(ram.rammer).damage(ram.taken);
        events.push(RoundEvent::Effect {
            side: ram.side,
            card_id: CommandId::CollisionCourse,
            ship,
            effect: CommandEffect::Ram {
                target: struck,
                dealt: ram.dealt,
                taken: ram.taken,
                target_after: stats(board, target),
            },
            after: stats(board, ram.rammer),
        });
    }
}

/// Meldet die Wirkung eines Befehls an einem Schiff: `ship` ist die Stelle vor dem Effekt, `after` die Werte danach.
fn report(
    events: &mut Vec<RoundEvent>,
    command: PlayedCommand,
    ship: ShipRef,
    effect: CommandEffect,
    after: ShipStats,
) {
    events.push(RoundEvent::Effect {
        side: command.side,
        card_id: command.card,
        ship,
        effect,
        after,
    });
}

/// Wendet `apply` auf jedes Schiff in `ships` an und meldet es; false, wenn es keins gab.
fn affect(
    board: &mut Board,
    events: &mut Vec<RoundEvent>,
    command: PlayedCommand,
    ships: impl IntoIterator<Item = SlotRef>,
    mut apply: impl FnMut(&mut Ship) -> CommandEffect,
) -> bool {
    let mut any = false;
    for at in ships {
        let ship = ship_ref(board, at);
        let effect = apply(board.ship_mut(at));
        report(events, command, ship, effect, stats(board, at));
        any = true;
    }
    any
}

fn damage(amount: i32) -> impl FnMut(&mut Ship) -> CommandEffect {
    move |ship| {
        ship.damage(amount);
        CommandEffect::Damage { amount }
    }
}

fn shield(amount: i32) -> impl FnMut(&mut Ship) -> CommandEffect {
    move |ship| {
        ship.add_shield(amount);
        CommandEffect::Shield {
            amount,
            permanent: false,
        }
    }
}

fn attack_bonus(delta: i32) -> impl FnMut(&mut Ship) -> CommandEffect {
    move |ship| {
        ship.attack_bonus += delta;
        CommandEffect::Attack { delta }
    }
}

/// Verlegt das Ziel in den gewählten Slot. Positionen ändern sich nur durch eigene Manöver und Befehle, und die
/// Zugprüfung hält ihre Ziel-Slots auseinander; er ist also noch frei.
fn move_target(board: &mut Board, events: &mut Vec<RoundEvent>, command: PlayedCommand) -> SlotRef {
    let from = command.target.expect("verlegender Befehl hat ein Ziel");
    let to = command
        .destination
        .expect("verlegender Befehl hat einen Ziel-Slot");
    let ship = ship_ref(board, from);
    board.move_ship(from, to);
    report(
        events,
        command,
        ship,
        CommandEffect::Move { to: to.target() },
        stats(board, to),
    );
    to
}

/// Wendet den Befehl an; false, wenn er nichts bewirkt hat.
fn apply_command(
    board: &mut Board,
    command: PlayedCommand,
    commander: &mut dyn Commander,
    events: &mut Vec<RoundEvent>,
) -> bool {
    let PlayedCommand {
        side, card, target, ..
    } = command;
    // Das eigene Ziel kann schon vor den Befehlen gefallen sein, durch den Schuss einer Plasma Frigate.
    if target.is_some_and(|at| !board.is_occupied(at)) && !card.targets_enemy_side() {
        return false;
    }
    let enemy = side.opposite();
    let enemy_ships: Vec<SlotRef> = board.ships(enemy).collect();
    let target_ship = || target.expect("Befehl mit Zielwahl hat ein Ziel");

    match card {
        CommandId::OrbitalStrike => {
            let strongest = highest(enemy_ships, |&at| board.ship(at).hp);
            affect(board, events, command, strongest, damage(4))
        }
        CommandId::VolleyFire => affect(board, events, command, enemy_ships, damage(1)),
        CommandId::Torpedo => {
            // Trifft, wer nach allen Bewegungen im Slot steht, gedeckt oder nicht; sonst verpufft er sichtbar.
            let at = target_ship();
            if !board.is_occupied(at) {
                events.push(RoundEvent::Miss {
                    side,
                    card_id: card,
                    target: at,
                });
                return true;
            }
            affect(board, events, command, Some(at), damage(5))
        }
        CommandId::Barrage => {
            // Debuffs wirken nach allen Bewegungen; Deckung spielt keine Rolle.
            let moved: Vec<SlotRef> = enemy_ships
                .into_iter()
                .filter(|&at| board.ship(at).moved)
                .collect();
            affect(board, events, command, moved, damage(3))
        }
        CommandId::PowerSurge => {
            let strike: Vec<SlotRef> = board.occupied(side, LaneType::Strike).collect();
            affect(board, events, command, strike, attack_bonus(1))
        }
        CommandId::TargetingJammer => {
            let strike: Vec<SlotRef> = board.occupied(enemy, LaneType::Strike).collect();
            affect(board, events, command, strike, attack_bonus(-1))
        }
        CommandId::EmpBlast => {
            let strongest = highest(enemy_ships, |&at| board.attack(at));
            affect(board, events, command, strongest, |ship| {
                ship.disabled = true;
                CommandEffect::Disable
            })
        }
        CommandId::EmergencyShields => affect(board, events, command, target, shield(3)),
        CommandId::Overcharge => affect(board, events, command, target, |ship| {
            ship.attack_bonus += 3;
            ship.self_damage_after_combat = 2;
            CommandEffect::Attack { delta: 3 }
        }),
        CommandId::FlankingManeuver => {
            move_target(board, events, command);
            true
        }
        CommandId::Intercept => {
            let at = move_target(board, events, command);
            affect(board, events, command, Some(at), attack_bonus(2))
        }
        CommandId::Recall => {
            let at = move_target(board, events, command);
            affect(board, events, command, Some(at), |ship| {
                ship.heal(2);
                CommandEffect::Heal { amount: 2 }
            })
        }
        CommandId::Scavenge => {
            let drawn = commander.draw_extra(side, 1, 1);
            events.push(RoundEvent::Draw {
                side,
                card_id: card,
                ships: drawn.ships,
                commands: drawn.commands,
            });
            true
        }
        CommandId::EvasiveAction => {
            let at = move_target(board, events, command);
            affect(board, events, command, Some(at), shield(2))
        }
        CommandId::MicroJump => {
            // Das Schiff verlässt samt Zustand das Brett; sein Slot bleibt bis zur Rückkehr im Cleanup gesperrt.
            let at = target_ship();
            let mut ship = board[at].take().expect("Ziel steht auf dem Brett");
            ship.moved = true;
            let jumped = ShipRef::new(at, ship.card);
            let after = ship_stats(&ship);
            board.jumping.push(Jump { slot: at, ship });
            report(events, command, jumped, CommandEffect::Jump, after);
            true
        }
        CommandId::RapidDeployment | CommandId::CarteBlanche => {
            events.push(RoundEvent::Rule {
                side,
                card_id: card,
            });
            true
        }
        CommandId::DrawFire => affect(board, events, command, target, |ship| {
            ship.taunting = true;
            CommandEffect::Taunt
        }),
        CommandId::ShieldGrid => {
            let own: Vec<SlotRef> = board.ships(side).collect();
            affect(board, events, command, own, shield(1))
        }
        CommandId::AblativeArmor => affect(board, events, command, target, |ship| {
            ship.shield += 2;
            CommandEffect::Shield {
                amount: 2,
                permanent: true,
            }
        }),
        CommandId::ScrapSalvage => affect(board, events, command, target, |ship| {
            ship.scrap_bonus += 1;
            CommandEffect::Attack { delta: 1 }
        }),
        CommandId::CoveringFire => {
            let at = target_ship();
            let partner = Some(at.in_lane(at.lane.other())).filter(|&p| board.is_occupied(p));
            for at in iter::once(at).chain(partner) {
                let ship = ship_ref(board, at);
                board.ship_mut(at).attack_bonus += 1;
                report(
                    events,
                    command,
                    ship,
                    CommandEffect::Attack { delta: 1 },
                    stats(board, at),
                );
                let effect = shield(1)(board.ship_mut(at));
                report(events, command, ship, effect, stats(board, at));
            }
            true
        }
        CommandId::DroneSquadron => {
            // `ship` im Ereignis ist die gerade gestartete Drohne.
            let slots: Vec<SlotRef> = launch_slots(board, side).into_iter().take(2).collect();
            for &at in &slots {
                board.put(at, ShipId::Drone).arrived = true;
                report(
                    events,
                    command,
                    ship_ref(board, at),
                    CommandEffect::Spawn,
                    stats(board, at),
                );
            }
            !slots.is_empty()
        }
        CommandId::RadioSilence => jam_powers(board, command, events),
        CommandId::Minefield => {
            let arrived: Vec<SlotRef> = enemy_ships
                .into_iter()
                .filter(|&at| board.ship(at).arrived)
                .collect();
            affect(board, events, command, arrived, damage(3))
        }
        CommandId::TractorBeam => {
            let Some(ship) = locked(board, enemy, card) else {
                return false;
            };
            let token = board.ship(ship).card.is_token();
            let Some(to) = board.free_slot_near(enemy, ship.lane.other(), ship.slot_index, token)
            else {
                return false;
            };
            let from = ship_ref(board, ship);
            board.move_ship(ship, to);
            report(
                events,
                command,
                from,
                CommandEffect::Move { to: to.target() },
                stats(board, to),
            );
            true
        }
        CommandId::HeatSeeker => {
            // Ignoriert Deckung; ein Schiff, das sich in dieser Runde bewegt hat, trifft er härter.
            let locked = locked(board, enemy, card);
            affect(board, events, command, locked, |ship| {
                damage(if ship.moved { 5 } else { 3 })(ship)
            })
        }
        CommandId::CoupDeGrace => {
            let candidates = enemy_ships
                .into_iter()
                .filter(|&at| !board.ship(at).card.is_token() && !board.is_destroyed(at));
            let weakest = highest(candidates, |&at| -board.ship(at).hp);
            affect(board, events, command, weakest, damage(4))
        }
        CommandId::PiercingShot => {
            // Das vorderste Schiff der Slot-Nummer nimmt 3; stand vorne ein Strike-Schiff, nimmt das Command-Schiff
            // dahinter 2.
            let front = target_ship();
            let column: Vec<SlotRef> = [front, front.in_lane(LaneType::Command)]
                .into_iter()
                .filter(|&at| board.is_standing(at))
                .collect();
            if column.is_empty() {
                events.push(RoundEvent::Miss {
                    side,
                    card_id: card,
                    target: front,
                });
                return true;
            }
            affect(board, events, command, Some(column[0]), damage(3));
            affect(board, events, command, column.get(1).copied(), damage(2));
            true
        }
        CommandId::Bombardment => {
            let command_lane: Vec<SlotRef> = board
                .occupied(enemy, LaneType::Command)
                .filter(|&at| board.is_standing(at))
                .collect();
            affect(board, events, command, command_lane, damage(2))
        }
        CommandId::Crossfire => {
            // Ohne Partner: kein (nicht zerstörtes) Schiff im Slot gleicher Nummer der anderen Lane; Tokens zählen
            // mit.
            let standing: Vec<SlotRef> = enemy_ships
                .into_iter()
                .filter(|&at| board.is_standing(at))
                .collect();
            let unpaired: Vec<SlotRef> = standing
                .iter()
                .copied()
                .filter(|at| {
                    !standing
                        .iter()
                        .any(|other| other.slot_index == at.slot_index && other != at)
                })
                .collect();
            affect(board, events, command, unpaired, damage(3))
        }
        // Wirken in `resolve_effects`, weil sie die Befehle beider Seiten brauchen.
        CommandId::Countermeasures | CommandId::CollisionCourse => true,
    }
}

/// Drohnenstaffel: freie eigene Strike-Slots in Startreihenfolge: zuerst vor dem ungedeckten eigenen Flaggschiff,
/// dann vor anderen ungedeckten eigenen Command-Schiffen (niedrigere Nummer zuerst), dann nach Nähe zur Lane-Mitte
/// (Gleichstand: niedrigerer Index).
fn launch_slots(board: &Board, side: Side) -> Vec<SlotRef> {
    let rank = |at: &SlotRef| match board.card(at.in_lane(LaneType::Command)) {
        Some(card) if card.is_flagship() => (0, 0, 0),
        Some(_) => (1, at.slot_index, 0),
        None => (
            2,
            (2 * at.slot_index).abs_diff(SLOT_COUNT - 1),
            at.slot_index,
        ),
    };
    let mut slots: Vec<SlotRef> = lane_slots(side, LaneType::Strike)
        .filter(|&at| board.is_free(at))
        .collect();
    slots.sort_by_key(rank);
    slots
}

/// Funkstille: Alle Marken der gegnerischen Admiralsfähigkeit fallen weg, auf gegnerischen Schiffen die mit eigenem
/// Ziel, auf eigenen die mit gegnerischem Ziel; ein gegnerisches Trugbild verschwindet sofort (ohne Schrott, ohne
/// `Vanish`). Meldet jede entfernte Marke; false, wenn keine da war.
fn jam_powers(board: &mut Board, command: PlayedCommand, events: &mut Vec<RoundEvent>) -> bool {
    let side = command.side;
    let ships: Vec<SlotRef> = board
        .ships(side.opposite())
        .chain(board.ships(side))
        .collect();
    let mut jammed = false;
    for at in ships {
        let before = ship_ref(board, at);
        let ship = board.ship_mut(at);
        if at.side != side && ship.card == ShipId::Decoy {
            board[at] = None;
            let effect = CommandEffect::Jam {
                power: PowerId::Decoy,
            };
            report(events, command, before, effect, ShipStats::default());
            jammed = true;
            continue;
        }
        let hostile: Vec<PowerId> = ship
            .powers
            .iter()
            .copied()
            .filter(|power| power.targets_enemy() == (at.side == side))
            .collect();
        if hostile.is_empty() {
            continue;
        }
        ship.powers.retain(|power| !hostile.contains(power));
        let after = ship_stats(ship);
        for power in hostile {
            report(events, command, before, CommandEffect::Jam { power }, after);
        }
        jammed = true;
    }
    jammed
}

// --- Combat ---

/// Ein Treffer auf ein Schiff mit Gegenfeuer (`source`); der Schütze nimmt dafür nach dem Kampf Schaden.
struct Counterfire {
    attacker: SlotRef,
    source: SlotRef,
}

/// Beide Seiten feuern gleichzeitig: Wer zu Kampfbeginn schießen kann ([`Board::can_attack`]) und einen Feuerbefehl
/// hat ([`Ship::aim`]), schießt, auch wenn er im selben Kampf zerstört wird; ohne Feuerbefehl feuert niemand.
/// Innerhalb einer Seite feuern die Schiffe nacheinander in Slot-Reihenfolge, erst die Strike-Lane, dann die
/// Command-Lane; gemeldet werden die Schüsse abwechselnd, damit die Animation hin und her geht.
///
/// Spaltenfeuer trifft das vorderste Schiff, das in der Spalte noch steht; fällt das vordere im selben Kampf, gehen
/// die übrigen Schüsse auf das dahinter (`through`). Schiffsfeuer trifft sein Ziel, wo es jetzt steht, wenn es in
/// diesem Moment ungedeckt ist. Flaggschiffe durchschauen Trugbilder. Sonst verpufft der Schuss.
///
/// Zuerst feuert die Railgun jeder Pendulum in `railguns` ([`firing_railguns`] für alle, deren Markierung steht) auf
/// ihr markiertes Schiff. Dann die Schiffe mit Taktvorsprung, beider Seiten gleichzeitig; wer dabei fällt und noch
/// stand, feuert nicht mehr (`Silenced`). Dann alle übrigen. Drohnen feuern auf das Ziel des Schwarmangriffs, auch
/// durch Deckung und ohne eigenen Feuerbefehl. Die übrigen Schüsse zieht ein Schiff mit Ablenkung auf sich, solange es
/// steht und für den Schützen ungedeckt ist. Zuletzt das Gegenfeuer. Fällt ein Flaggschiff, endet der Kampf nach dem
/// Schritt.
pub fn resolve_combat(board: &mut Board, events: &mut Vec<RoundEvent>, railguns: &[Side]) {
    let attackers = Side::ALL.map(|side| -> Vec<SlotRef> {
        board
            .ships(side)
            .filter(|&at| {
                board.can_attack(at)
                    && (board.ship(at).aim.is_some() || swarm_target(board, at).is_some())
            })
            .collect()
    });
    for &side in railguns {
        events.extend(fire_railgun(board, side));
    }
    let mut counterfire = Vec::new();
    let head_start =
        |board: &Board, at: SlotRef| board.ship(at).powers.contains(&PowerId::HeadStart);
    let first = attackers.clone().map(|slots| {
        slots
            .into_iter()
            .filter(|&at| head_start(board, at))
            .collect()
    });
    let later: [Vec<SlotRef>; 2] = attackers.map(|slots| {
        slots
            .into_iter()
            .filter(|&at| !head_start(board, at))
            .collect()
    });
    let standing: Vec<SlotRef> = later
        .iter()
        .flatten()
        .copied()
        .filter(|&at| !board.is_destroyed(at))
        .collect();
    let flagships: Vec<SlotRef> = Side::ALL
        .into_iter()
        .filter_map(|side| board.flagship(side))
        .filter(|&at| !board.is_destroyed(at))
        .collect();
    events.extend(fire(board, &first, &mut counterfire));
    if flagships.iter().any(|&at| board.is_destroyed(at)) {
        return;
    }
    let silenced: Vec<SlotRef> = standing
        .into_iter()
        .filter(|&at| board.is_destroyed(at))
        .collect();
    events.extend(silenced.iter().map(|&at| RoundEvent::Silenced {
        ship: ship_ref(board, at),
    }));
    let later = later.map(|slots| {
        slots
            .into_iter()
            .filter(|at| !silenced.contains(at))
            .collect()
    });
    events.extend(fire(board, &later, &mut counterfire));
    if Side::ALL.into_iter().any(|side| {
        board
            .flagship(side)
            .is_some_and(|at| board.is_destroyed(at))
    }) {
        return;
    }
    for Counterfire { attacker, source } in counterfire {
        let amount = ABILITY_VALUES.counterfire;
        board.ship_mut(attacker).damage(amount);
        events.push(RoundEvent::Counterfire {
            source: ship_ref(board, source),
            ship: ship_ref(board, attacker),
            amount,
            after: stats(board, attacker),
        });
    }
}

/// Die Schüsse beider Seiten, abwechselnd gemeldet.
fn fire(
    board: &mut Board,
    shooters: &[Vec<SlotRef>; 2],
    counterfire: &mut Vec<Counterfire>,
) -> Vec<RoundEvent> {
    let [player, enemy] = Side::ALL;
    let player = volley(board, player, &shooters[0], counterfire);
    let enemy = volley(board, enemy, &shooters[1], counterfire);
    interleave(player, enemy)
}

/// Railgun der Pendulum zu Kampfbeginn: Schaden auf das markierte Schiff, wenn es ungedeckt ist (ein Trugbild deckt
/// nicht, die Pendulum durchschaut es), sonst ist der Schuss blockiert.
fn fire_railgun(board: &mut Board, side: Side) -> Option<RoundEvent> {
    let pendulum = board.flagship(side)?;
    let target = board
        .ships(side.opposite())
        .find(|&at| board.ship(at).marked)?;
    let blocked = !board.is_exposed(target, true);
    let amount = if blocked { 0 } else { ABILITY_VALUES.railgun };
    board.ship_mut(target).damage(amount);
    Some(RoundEvent::Railgun {
        source: ship_ref(board, pendulum),
        target: ship_ref(board, target),
        amount,
        after: stats(board, target),
        blocked,
    })
}

/// Das Ziel des Schwarmangriffs für eine Drohne: das gegnerische Schiff mit der Marke, solange es auf dem Brett steht.
fn swarm_target(board: &Board, attacker: SlotRef) -> Option<SlotRef> {
    if board.ship(attacker).card != ShipId::Drone {
        return None;
    }
    board
        .ships(attacker.side.opposite())
        .find(|&at| board.ship(at).powers.contains(&PowerId::SwarmStrike))
}

/// Ein Schuss trifft ein Schiff (`through`: Spaltenfeuer, das durchging) oder verpufft.
enum Shot {
    Hit { target: SlotRef, through: bool },
    Miss(Miss),
}

/// Die Schüsse einer Seite in Slot-Reihenfolge (siehe [`resolve_combat`]); Treffer auf ein Schiff mit Gegenfeuer
/// landen in `counterfire`.
fn volley(
    board: &mut Board,
    side: Side,
    shooters: &[SlotRef],
    counterfire: &mut Vec<Counterfire>,
) -> Vec<RoundEvent> {
    let enemy = side.opposite();
    let taunt = board.ships(enemy).find(|&at| board.ship(at).taunting);
    let mut shots = Vec::new();
    for &attacker in shooters {
        let see_through = board.ship(attacker).card.is_flagship();
        let head_start = board.ship(attacker).powers.contains(&PowerId::HeadStart);
        let swarm = swarm_target(board, attacker);
        let taunted = taunt.filter(|&at| {
            swarm.is_none() && !board.is_destroyed(at) && board.is_exposed(at, see_through)
        });
        // Schwarmangriff und Ablenkung lenken den Schuss am Feuerbefehl vorbei.
        let shot = match swarm.or(taunted) {
            None => {
                let aim = board
                    .ship(attacker)
                    .aim
                    .expect("Schütze hat einen Feuerbefehl");
                aimed_shot(board, enemy, aim, see_through)
            }
            Some(pulled) if board.is_destroyed(pulled) => Shot::Miss(Miss::Target {
                target: Some(ship_ref(board, pulled)),
                covered: false,
            }),
            Some(pulled) => Shot::Hit {
                target: pulled,
                through: false,
            },
        };
        let (target, through) = match shot {
            Shot::Miss(miss) => {
                shots.push(RoundEvent::Fizzle {
                    attacker: ship_ref(board, attacker),
                    head_start,
                    miss,
                });
                continue;
            }
            Shot::Hit { target, through } => (target, through),
        };
        let focus = board.ship(target).powers.contains(&PowerId::FocusFire);
        let amount = board.attack(attacker)
            + if swarm.is_some() {
                ABILITY_VALUES.swarm_strike
            } else {
                0
            }
            + if focus { ABILITY_VALUES.focus_fire } else { 0 };
        board.ship_mut(target).damage(amount);
        if board.ship(target).powers.contains(&PowerId::Counterfire) && !see_through {
            counterfire.push(Counterfire {
                attacker,
                source: target,
            });
        }
        shots.push(RoundEvent::Attack {
            attacker: ship_ref(board, attacker),
            target: ship_ref(board, target),
            amount,
            after: stats(board, target),
            focus,
            swarm: swarm.is_some(),
            head_start,
            through,
            taunt: taunted.is_some(),
        });
    }
    shots
}

/// Wohin ein Schuss nach seinem Feuerbefehl geht (siehe [`resolve_combat`]). Spaltenfeuer: das vorderste Schiff, das
/// dort noch steht, mit `see_through` (Flaggschiffe) ein Trugbild nur, wenn kein echtes Schiff dahinter steht.
/// Schiffsfeuer: das Ziel, wenn es noch steht und für den Schützen ungedeckt ist.
fn aimed_shot(board: &Board, side: Side, aim: Aim, see_through: bool) -> Shot {
    match aim {
        Aim::Column(column) => {
            let front = SlotRef::new(side, LaneType::Strike, column);
            let back = front.in_lane(LaneType::Command);
            let standing: Vec<SlotRef> = [front, back]
                .into_iter()
                .filter(|&at| board.is_standing(at))
                .collect();
            let real = standing
                .iter()
                .copied()
                .find(|&at| see_through && board.ship(at).card != ShipId::Decoy);
            match real.or(standing.first().copied()) {
                Some(hit) => Shot::Hit {
                    target: hit,
                    through: hit == back && board.is_destroyed(front),
                },
                None => Shot::Miss(Miss::Column {
                    column,
                    cleared: board.is_occupied(front) || board.is_occupied(back),
                }),
            }
        }
        Aim::Ship(uid) => {
            let Some(aimed) = board.ships(side).find(|&at| board.ship(at).uid == uid) else {
                return Shot::Miss(Miss::Target {
                    target: None,
                    covered: false,
                });
            };
            if board.is_destroyed(aimed) {
                return Shot::Miss(Miss::Target {
                    target: Some(ship_ref(board, aimed)),
                    covered: false,
                });
            }
            if !board.is_exposed(aimed, see_through) {
                return Shot::Miss(Miss::Target {
                    target: Some(ship_ref(board, aimed)),
                    covered: true,
                });
            }
            Shot::Hit {
                target: aimed,
                through: false,
            }
        }
    }
}

/// Reißverschluss: abwechselnd ein Element aus jeder Liste, bis beide leer sind.
fn interleave<T>(first: Vec<T>, second: Vec<T>) -> Vec<T> {
    let mut result = Vec::with_capacity(first.len() + second.len());
    let mut first = first.into_iter();
    let mut second = second.into_iter();
    loop {
        let (a, b) = (first.next(), second.next());
        if a.is_none() && b.is_none() {
            return result;
        }
        result.extend(a);
        result.extend(b);
    }
}

// --- Cleanup ---

/// Rückkehr aus dem Kurzsprung, Eigenschaden, Zerstörte entfernen, Reparaturen, das Trugbild verschwindet,
/// Rundenmodifikatoren zurücksetzen.
pub fn resolve_cleanup(board: &mut Board, events: &mut Vec<RoundEvent>) {
    for Jump { slot, ship } in std::mem::take(&mut board.jumping) {
        board[slot] = Some(ship);
        events.push(RoundEvent::Return {
            ship: ship_ref(board, slot),
            after: stats(board, slot),
        });
    }
    let ships: Vec<SlotRef> = board.all_ships().collect();
    for at in ships {
        let amount = board.ship(at).self_damage_after_combat;
        if amount == 0 {
            continue;
        }
        board.ship_mut(at).damage(amount);
        events.push(RoundEvent::SelfDamage {
            ship: ship_ref(board, at),
            amount,
            after: stats(board, at),
        });
    }
    remove_destroyed(board, events);
    resolve_repairs(board, events);
    let decoys: Vec<SlotRef> = board
        .all_ships()
        .filter(|&at| board.ship(at).card == ShipId::Decoy)
        .collect();
    for at in decoys {
        events.push(RoundEvent::Vanish {
            ship: ship_ref(board, at),
        });
        board[at] = None;
    }
    let ships: Vec<SlotRef> = board.all_ships().collect();
    for at in ships {
        board.ship_mut(at).end_round();
    }
}

/// Repair Drone: Jede überlebende Drohne heilt am Rundenende 2 HP beim beschädigten eigenen Kampfschiff mit den
/// wenigsten HP. Support-Schiffe heilt sie nicht, sonst halten sich Drohnen in der Strike-Lane gegenseitig am Leben.
/// Mehrere Drohnen wirken nacheinander und wählen jeweils neu.
fn resolve_repairs(board: &mut Board, events: &mut Vec<RoundEvent>) {
    for side in Side::ALL {
        let ships: Vec<SlotRef> = board.ships(side).collect();
        let drones: Vec<SlotRef> = ships
            .iter()
            .copied()
            .filter(|&at| board.ship(at).card == ShipId::RepairDrone)
            .collect();
        for drone in drones {
            let damaged = ships.iter().copied().filter(|&at| {
                let ship = board.ship(at);
                ship.card.atk() > 0 && ship.hp < ship.card.hp()
            });
            let Some(target) = highest(damaged, |&at| -board.ship(at).hp) else {
                continue;
            };
            board.ship_mut(target).heal(2);
            events.push(RoundEvent::Ability {
                source: ship_ref(board, drone),
                ship: ship_ref(board, target),
                effect: AbilityEffect::Heal { amount: 2 },
                after: stats(board, target),
            });
        }
    }
}

/// Hive (Brutkammer): Am Rundenende, nach dem Cleanup, startet sie eine Drohne in den freien Strike-Slot, der ihr am
/// nächsten liegt, solange ihre Seite weniger Drohnen hat als `max_drones` (auch die der Titan Carrier) und sie in
/// diesem Match weniger als `hive_budget` gestartet hat ([`Board::hive_launches`]).
fn resolve_brood_chamber(board: &mut Board, events: &mut Vec<RoundEvent>) {
    for side in Side::ALL {
        let Some(hive) = board
            .flagship(side)
            .filter(|&at| board.ship(at).card == ShipId::Hive)
        else {
            continue;
        };
        if board.hive_launches[side] >= ABILITY_VALUES.hive_budget {
            continue;
        }
        let drones = board
            .ships(side)
            .filter(|&at| board.ship(at).card == ShipId::Drone)
            .count();
        if drones >= ABILITY_VALUES.max_drones {
            continue;
        }
        let free = lane_slots(side, LaneType::Strike).filter(|&at| !board.is_occupied(at));
        let Some(slot) = nearest(free, hive.slot_index) else {
            continue;
        };
        board.put(slot, ShipId::Drone);
        board.hive_launches[side] += 1;
        events.push(RoundEvent::Spawn {
            source: ship_ref(board, hive),
            ship: ship_ref(board, slot),
        });
    }
}

/// Entfernt die zerstörten Schiffe. Jeder überlebende Scrap Raider, auf dessen Gegenseite in der gleichnamigen Lane
/// ein Schiff fiel (egal wodurch), birgt Schrott (`Salvage`): Steht nach der Runde noch ein Scrap Raider auf seiner
/// Seite, gehört zum Nachschub die Schrottverwertung.
fn remove_destroyed(board: &mut Board, events: &mut Vec<RoundEvent>) {
    let destroyed: Vec<SlotRef> = board
        .all_ships()
        .filter(|&at| board.is_destroyed(at))
        .collect();
    for &at in &destroyed {
        events.push(RoundEvent::Destroyed {
            ship: ship_ref(board, at),
        });
    }
    let raiders: Vec<SlotRef> = board
        .all_ships()
        .filter(|&at| board.ship(at).card == ShipId::ScrapRaider && !board.is_destroyed(at))
        .collect();
    for raider in raiders {
        // Ein Trugbild ist ein Hologramm und hinterlässt keinen Schrott.
        let scrap = destroyed.iter().any(|&at| {
            at.side != raider.side && at.lane == raider.lane && board.ship(at).card != ShipId::Decoy
        });
        if !scrap {
            continue;
        }
        let ship = ship_ref(board, raider);
        events.push(RoundEvent::Ability {
            source: ship,
            ship,
            effect: AbilityEffect::Salvage,
            after: stats(board, raider),
        });
    }
    for at in destroyed {
        board[at] = None;
    }
}
