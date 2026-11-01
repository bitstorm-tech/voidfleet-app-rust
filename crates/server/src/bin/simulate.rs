//! Misst Spieltempo und Balance der Admirale: Bot gegen Bot über [`Match`], beide Pläne per [`plan_bot_turn`], samt
//! Nachschub per [`plan_bot_draft`]. Jede Paarung von Admiralen (auch gegen sich selbst) spielt gleich viele Partien,
//! die Sitze wechseln sich ab; die Partien laufen parallel auf allen Kernen, jede mit festem Zufall aus ihrer Nummer.
//! Portiert aus `server/tools/simulate.ts`.
//!
//! Aufruf: `cargo run -p voidfleet-server --release --bin simulate -- [partien je paarung]` (Standard 2000).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::Serialize;
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::board::{Board, LaneType, Side, SlotRef, SlotTarget};
use voidfleet_core::cards::{CommandId, ShipId};
use voidfleet_core::protocol::{CommandEffect, ErrorCode, FireTarget, Miss, RoundEvent, TurnPlan};
use voidfleet_server::bot::{plan_bot_draft, plan_bot_turn};
use voidfleet_server::{Match, Outcome, Seat};

/// Wer so lange spielt, steckt im Patt.
const ROUND_LIMIT: u32 = 60;

// --- Zählen und Zusammenführen ---

/// Zählerstände, die die Threads getrennt sammeln und am Ende zusammenführen.
trait Merge {
    fn absorb(&mut self, other: Self);
}

impl Merge for u64 {
    fn absorb(&mut self, other: Self) {
        *self += other;
    }
}

impl Merge for i64 {
    fn absorb(&mut self, other: Self) {
        *self += other;
    }
}

impl<T> Merge for Vec<T> {
    fn absorb(&mut self, other: Self) {
        self.extend(other);
    }
}

impl<T: Merge, const N: usize> Merge for [T; N] {
    fn absorb(&mut self, other: Self) {
        for (mine, theirs) in self.iter_mut().zip(other) {
            mine.absorb(theirs);
        }
    }
}

impl<K: Eq + Hash, V: Merge + Default> Merge for HashMap<K, V> {
    fn absorb(&mut self, other: Self) {
        for (key, value) in other {
            self.entry(key).or_default().absorb(value);
        }
    }
}

impl<K: Ord, V: Merge + Default> Merge for BTreeMap<K, V> {
    fn absorb(&mut self, other: Self) {
        for (key, value) in other {
            self.entry(key).or_default().absorb(value);
        }
    }
}

/// Führt Strukturen feldweise zusammen.
macro_rules! merge_fields {
    ($($ty:ident { $($field:ident),* $(,)? })*) => {
        $(impl Merge for $ty {
            fn absorb(&mut self, other: Self) {
                $(self.$field.absorb(other.$field);)*
            }
        })*
    };
}

/// Feuerbefehle je Modus.
#[derive(Debug, Default, Clone, Copy)]
struct Modes {
    ship: u64,
    column: u64,
}

impl Modes {
    fn count(&mut self, mode: Mode) {
        match mode {
            Mode::Ship => self.ship += 1,
            Mode::Column => self.column += 1,
        }
    }

    fn total(self) -> u64 {
        self.ship + self.column
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ship,
    Column,
}

/// Schüsse im Kampf (ohne Railgun): alle, verpufft, durchgegangen auf das Schiff hinter einem gefallenen, von
/// Ablenkung gezogen.
#[derive(Debug, Default)]
struct Shots {
    all: u64,
    fizzled: u64,
    through: u64,
    taunted: u64,
}

/// Feuerbefehle je Lane des Ziels beim Planen (Schiffsfeuer: das Ziel; Spaltenfeuer: das vorderste Schiff der Spalte,
/// `empty`, wenn sie leer war).
#[derive(Debug, Default)]
struct OrderLanes {
    strike: Modes,
    command: Modes,
    empty: Modes,
}

#[derive(Debug, Default)]
struct Orders {
    all: Modes,
    admiral: [Modes; 5],
    lane: OrderLanes,
}

/// Was aus Spaltenfeuer wurde: Treffer, davon durchgegangen, von Ablenkung gezogen, ins Leere, nach einem Abschuss.
#[derive(Debug, Default)]
struct ColumnOutcome {
    shots: u64,
    hit: u64,
    through: u64,
    taunted: u64,
    void: u64,
    cleared: u64,
}

/// Was aus Schiffsfeuer wurde: Treffer, von Ablenkung gezogen, gedeckt, schon zerstört, nicht mehr auf dem Brett;
/// `moved`: auf ein Ziel, das vor dem Kampf den Slot gewechselt hat, `moved_hit` davon Treffer.
#[derive(Debug, Default)]
struct ShipOutcome {
    shots: u64,
    hit: u64,
    taunted: u64,
    covered: u64,
    destroyed: u64,
    gone: u64,
    moved: u64,
    moved_hit: u64,
}

/// Woher die Deckung kam, an der Schiffsfeuer verpuffte: Sie stand schon beim Planen und fiel nicht, das Ziel zog in
/// Deckung, vor ihm landete ein neues Schiff (Deploy, Trugbild, Drohne) bzw. zog eines davor.
#[derive(Debug, Default)]
struct Cover {
    held: u64,
    retreated: u64,
    arrived: u64,
    moved: u64,
}

/// Was aus den Schüssen je Modus wurde; `swarm`: Drohnen im Schwarmangriff ohne eigenen Feuerbefehl, `swarm_fizzled`
/// davon verpufft.
#[derive(Debug, Default)]
struct Outcomes {
    column: ColumnOutcome,
    ship: ShipOutcome,
    cover: Cover,
    swarm: u64,
    swarm_fizzled: u64,
}

/// Grundmanöver (ohne Phasentausch): alle, davon in eine andere Spalte (Ausweichen), davon mit gegnerischen Schüssen auf
/// die alte Spalte, davon mit Schüssen ins Leere dort, die Zahl dieser Schüsse und wie oft das Schiff in der neuen
/// Spalte trotzdem getroffen wurde.
#[derive(Debug, Default)]
struct Maneuvers {
    all: u64,
    evasions: u64,
    targeted: u64,
    dodged: u64,
    voided: u64,
    struck: u64,
}

/// Flaggschiff-Duell: Runden, in denen beide Seiten nur noch ihr Flaggschiff haben (Tokens nicht gezählt), je Partie
/// mit Duell, dazu Feuerbefehle je Modus, Schüsse, verpuffte Schüsse und Grundmanöver darin.
#[derive(Debug, Default)]
struct Duel {
    lengths: Vec<u64>,
    orders: Modes,
    shots: u64,
    fizzled: u64,
    maneuvers: u64,
}

/// Je Karte: wie oft gespielt, wie oft eine Kopie auf der Hand lag, wie oft ohne Wirkung, wie oft von Gegenmaßnahmen
/// verpufft.
#[derive(Debug, Default)]
struct CardStats {
    played: u64,
    held: u64,
    idle: u64,
    countered: u64,
}

/// Je Admiral: Partien, Runden ab Runde 2, davon mit Fähigkeit bzw. Grundmanöver (samt Phasentausch), und was seine
/// Fähigkeiten bewirkt haben.
#[derive(Debug, Default)]
struct Usage {
    games: u64,
    rounds: u64,
    powers: u64,
    maneuvers: u64,
    effects: BTreeMap<&'static str, i64>,
}

/// Kennzahlen der Befehle aus der Erweiterung: Kurzsprünge (davon des Flaggschiffs), Rammstöße (Ziel bzw. Rammer
/// zerstört), Gegenmaßnahmen (Befehl verpufft bzw. gezogen), von Funkstille entfernte Marken und Trugbilder, vom
/// Traktorstrahl freigelegte Flaggschiffe (danach beschossen), Wärmesucher mit 5 Schaden, Treffer der Minenfelder,
/// Drohnen der Drohnenstaffeln, Freie Hand mit Manöver und Fähigkeit.
#[derive(Debug, Default)]
struct Expansion {
    jumps: u64,
    flagship_jumps: u64,
    rams: u64,
    rammed_destroyed: u64,
    rammers_destroyed: u64,
    countered: u64,
    counter_draws: u64,
    jammed: u64,
    tractor_exposed: u64,
    tractor_exposed_hit: u64,
    heat_seeker_moved: u64,
    mine_hits: u64,
    squadron_drones: u64,
    carte_blanche_both: u64,
}

#[derive(Debug, Default)]
struct Stats {
    games: Vec<Game>,
    first_kills: Vec<u64>,
    /// Je Partie die Runde, in der das erste Flaggschiff fiel.
    flagship_falls: Vec<u64>,
    kills: u64,
    rounds_without_kill: u64,
    combat_damage: i64,
    command_damage: i64,
    shots: Shots,
    orders: Orders,
    outcomes: Outcomes,
    maneuvers: Maneuvers,
    duel: Duel,
    commands: u64,
    commands_without_effect: u64,
    empty_command_hand: [u64; 2],
    cards: HashMap<CommandId, CardStats>,
    expansion: Expansion,
    errors: HashMap<ErrorCode, u64>,
    usage: [Usage; 5],
}

merge_fields! {
    Modes { ship, column }
    Shots { all, fizzled, through, taunted }
    OrderLanes { strike, command, empty }
    Orders { all, admiral, lane }
    ColumnOutcome { shots, hit, through, taunted, void, cleared }
    ShipOutcome { shots, hit, taunted, covered, destroyed, gone, moved, moved_hit }
    Cover { held, retreated, arrived, moved }
    Outcomes { column, ship, cover, swarm, swarm_fizzled }
    Maneuvers { all, evasions, targeted, dodged, voided, struck }
    Duel { lengths, orders, shots, fizzled, maneuvers }
    CardStats { played, held, idle, countered }
    Usage { games, rounds, powers, maneuvers, effects }
    Expansion {
        jumps, flagship_jumps, rams, rammed_destroyed, rammers_destroyed, countered, counter_draws, jammed,
        tractor_exposed, tractor_exposed_hit, heat_seeker_moved, mine_hits, squadron_drones, carte_blanche_both,
    }
    Stats {
        games, first_kills, flagship_falls, kills, rounds_without_kill, combat_damage, command_damage, shots, orders,
        outcomes, maneuvers, duel, commands, commands_without_effect, empty_command_hand, cards, expansion, errors,
        usage,
    }
}

/// Wie eine Partie ausging: Sitz des Siegers oder unentschieden; `None` in [`Game::winner`] bei Patt (Rundenlimit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Winner {
    Seat(usize),
    Draw,
}

#[derive(Debug, Clone, Copy)]
struct Game {
    admirals: [AdmiralId; 2],
    winner: Option<Winner>,
    rounds: u32,
}

fn admiral_index(admiral: AdmiralId) -> usize {
    ADMIRAL_IDS
        .iter()
        .position(|&id| id == admiral)
        .expect("bekannter Admiral")
}

impl Stats {
    fn card(&mut self, id: CommandId) -> &mut CardStats {
        self.cards.entry(id).or_default()
    }

    fn effect(&mut self, admiral: AdmiralId, key: &'static str, amount: i64) {
        *self.usage[admiral_index(admiral)]
            .effects
            .entry(key)
            .or_default() += amount;
    }
}

// --- Auswertung der Ereignisse ---

/// Was eine Flaggschiff- oder Admiralsfähigkeit bewirkt hat, für die Seite, deren Admiral sie gehört.
fn ability_effect(event: &RoundEvent) -> Option<(Side, &'static str, i64)> {
    match *event {
        RoundEvent::Counterfire { source, amount, .. } => {
            Some((source.side, "Gegenfeuer-Schaden", i64::from(amount)))
        }
        RoundEvent::Attack {
            attacker,
            target,
            focus,
            swarm,
            ..
        } => {
            if focus {
                Some((attacker.side, "Fokustreffer", 1))
            } else if swarm {
                Some((attacker.side, "Schwarmtreffer", 1))
            } else if target.card_id == ShipId::Decoy {
                Some((target.side, "Treffer aufs Trugbild", 1))
            } else {
                None
            }
        }
        RoundEvent::Spawn { source, .. } if source.card_id == ShipId::Hive => {
            Some((source.side, "Drohnen der Hive", 1))
        }
        RoundEvent::Swap { side, .. } => Some((side, "Phasentausche", 1)),
        RoundEvent::Railgun {
            source, blocked, ..
        } => {
            let key = if blocked {
                "Railgun blockiert"
            } else {
                "Railgun-Treffer"
            };
            Some((source.side, key, 1))
        }
        RoundEvent::Silenced { ship } => Some((ship.side.opposite(), "verstummt", 1)),
        _ => None,
    }
}

/// Zählt, was ein Effekt der neuen Befehle bewirkt hat; `flagships` stehen vor der Runde, `events` sind die der Runde.
fn new_card_effect(
    stats: &mut Stats,
    event: &RoundEvent,
    flagships: &[SlotRef],
    events: &[RoundEvent],
) {
    let RoundEvent::Effect {
        card_id,
        ship,
        effect,
        after,
        ..
    } = *event
    else {
        return;
    };
    let expansion = &mut stats.expansion;
    match effect {
        CommandEffect::Jump => {
            expansion.jumps += 1;
            expansion.flagship_jumps += u64::from(ship.card_id.is_flagship());
        }
        CommandEffect::Ram { target_after, .. } => {
            expansion.rams += 1;
            expansion.rammed_destroyed += u64::from(target_after.hp <= 0);
            expansion.rammers_destroyed += u64::from(after.hp <= 0);
        }
        CommandEffect::Jam { .. } => expansion.jammed += 1,
        _ => {}
    }
    match card_id {
        CommandId::HeatSeeker => {
            expansion.heat_seeker_moved += u64::from(effect == CommandEffect::Damage { amount: 5 });
        }
        CommandId::Minefield => expansion.mine_hits += 1,
        CommandId::DroneSquadron => expansion.squadron_drones += 1,
        CommandId::TractorBeam
            if matches!(effect, CommandEffect::Move { .. }) && ship.lane == LaneType::Strike =>
        {
            let behind = ship.slot().in_lane(LaneType::Command);
            if let Some(&flagship) = flagships.iter().find(|&&flagship| flagship == behind) {
                expansion.tractor_exposed += 1;
                let hit = events.iter().any(|other| {
                    matches!(other, RoundEvent::Attack { target, .. }
                        if target.side == flagship.side && target.card_id.is_flagship())
                });
                expansion.tractor_exposed_hit += u64::from(hit);
            }
        }
        _ => {}
    }
}

/// Zählt ein Grundmanöver und, wechselt es die Spalte, was die gegnerischen Schüsse daraus machen.
fn evasion(stats: &mut Stats, event: &RoundEvent, events: &[RoundEvent]) {
    let RoundEvent::Maneuver { ship, to, .. } = *event else {
        return;
    };
    let maneuvers = &mut stats.maneuvers;
    maneuvers.all += 1;
    if to.slot_index == ship.slot_index {
        return;
    }
    maneuvers.evasions += 1;
    let column = ship.slot_index;
    let aimed = events.iter().any(|other| match *other {
        RoundEvent::Fizzle {
            attacker,
            miss: Miss::Column { column: aimed, .. },
            ..
        } => attacker.side != ship.side && aimed == column,
        RoundEvent::Attack {
            attacker,
            target,
            swarm,
            taunt,
            ..
        } => attacker.side != ship.side && !swarm && !taunt && target.slot_index == column,
        _ => false,
    });
    if aimed {
        maneuvers.targeted += 1;
    }
    let voided = events
        .iter()
        .filter(|other| {
            matches!(**other, RoundEvent::Fizzle {
                attacker,
                miss: Miss::Column { column: aimed, cleared: false },
                ..
            } if attacker.side != ship.side && aimed == column)
        })
        .count() as u64;
    maneuvers.voided += voided;
    if voided > 0 {
        maneuvers.dodged += 1;
    }
    let struck = events.iter().any(|other| {
        matches!(*other, RoundEvent::Attack { target, .. }
            if target.side == ship.side && target.lane == to.lane && target.slot_index == to.slot_index)
    });
    if struck {
        maneuvers.struck += 1;
    }
}

/// Ein Feuerbefehl dieser Runde, verfolgt bis zu seinem Schuss: Schütze und Ziel als Kennung (`Ship::uid`, neue
/// Schiffe negativ), sobald die Befehle greifen.
struct TrackedOrder {
    side: Side,
    mode: Mode,
    shooter: SlotRef,
    target: Option<SlotRef>,
    shooter_id: i64,
    target_id: i64,
}

/// Ordnet die Schüsse einer Runde den Feuerbefehlen zu und zählt Modus und Ausgang. Vor der Auflösung angelegt, mit den
/// Zügen beider Seiten; [`FireTracker::finish`] nimmt die Ereignisse der Runde. Die Schiffe werden dafür bis zum Kampf
/// über die Ereignisse verfolgt: Deploy, Manöver, Phasentausch, Verlegung per Befehl, Kurzsprung und was neu aufs
/// Brett kommt.
struct FireTracker {
    /// Wer wo steht, je Kennung.
    at: HashMap<SlotRef, i64>,
    fire: Vec<TrackedOrder>,
}

impl FireTracker {
    fn new(stats: &mut Stats, board: &Board, turns: &[(Side, AdmiralId, TurnPlan)]) -> Self {
        let at = board
            .all_ships()
            .map(|slot| (slot, i64::from(board.ship(slot).uid)))
            .collect();
        let mut fire = Vec::new();
        for (side, admiral, plan) in turns {
            let enemy = side.opposite();
            for order in &plan.fire {
                let (mode, lane, target) = match order.target {
                    FireTarget::Ship(target) => {
                        (Mode::Ship, Some(target.lane), Some(target.on(enemy)))
                    }
                    FireTarget::Column(column) => {
                        let front = SlotRef::new(enemy, LaneType::Strike, column);
                        let lane = [front, front.in_lane(LaneType::Command)]
                            .into_iter()
                            .find(|&slot| board.is_occupied(slot))
                            .map(|slot| slot.lane);
                        (Mode::Column, lane, None)
                    }
                };
                let orders = &mut stats.orders;
                orders.all.count(mode);
                orders.admiral[admiral_index(*admiral)].count(mode);
                match lane {
                    Some(LaneType::Strike) => orders.lane.strike.count(mode),
                    Some(LaneType::Command) => orders.lane.command.count(mode),
                    None => orders.lane.empty.count(mode),
                }
                fire.push(TrackedOrder {
                    side: *side,
                    mode,
                    shooter: order.ship.on(*side),
                    target,
                    shooter_id: 0,
                    target_id: 0,
                });
            }
        }
        FireTracker { at, fire }
    }

    fn finish(mut self, stats: &mut Stats, events: &[RoundEvent]) {
        let mut moved: HashSet<i64> = HashSet::new();
        let mut jumping: HashMap<SlotRef, i64> = HashMap::new();
        let mut fresh = 0;
        let mut bound = false;
        for event in events {
            // Die Feuerbefehle greifen, sobald alle Schiffe stehen.
            if !bound && !matches!(event, RoundEvent::Deploy { .. } | RoundEvent::Pass { .. }) {
                bound = true;
                for order in &mut self.fire {
                    order.shooter_id = self.at.get(&order.shooter).copied().unwrap_or(0);
                    order.target_id = order
                        .target
                        .and_then(|target| self.at.get(&target).copied())
                        .unwrap_or(0);
                }
            }
            let mut arrive = |at: &mut HashMap<SlotRef, i64>, slot: SlotRef| {
                fresh -= 1;
                at.insert(slot, fresh);
            };
            let mut relocate = |at: &mut HashMap<SlotRef, i64>, from: SlotRef, to: SlotRef| {
                let id = at.remove(&from).expect("das Schiff wird verfolgt");
                at.insert(to, id);
                moved.insert(id);
            };
            match *event {
                RoundEvent::Deploy { side, target, .. } => arrive(&mut self.at, target.on(side)),
                RoundEvent::Decoy { ship } | RoundEvent::Spawn { ship, .. } => {
                    arrive(&mut self.at, ship.slot());
                }
                RoundEvent::Maneuver { side, ship, to } => {
                    relocate(&mut self.at, ship.slot(), to.on(side));
                }
                RoundEvent::Swap { a, b, .. } => {
                    let first = self.at[&a.slot()];
                    let second = self.at[&b.slot()];
                    self.at.insert(a.slot(), second);
                    self.at.insert(b.slot(), first);
                    moved.extend([first, second]);
                }
                RoundEvent::Effect { ship, effect, .. } => match effect {
                    CommandEffect::Spawn => arrive(&mut self.at, ship.slot()),
                    CommandEffect::Move { to } => {
                        relocate(&mut self.at, ship.slot(), to.on(ship.side));
                    }
                    CommandEffect::Jump => {
                        let id = self
                            .at
                            .remove(&ship.slot())
                            .expect("das Schiff wird verfolgt");
                        jumping.insert(ship.slot(), id);
                    }
                    _ => {}
                },
                RoundEvent::Return { ship, .. } => {
                    let id = jumping[&ship.slot()];
                    self.at.insert(ship.slot(), id);
                }
                RoundEvent::Destroyed { ship } => {
                    self.at.remove(&ship.slot());
                }
                RoundEvent::Attack { attacker, .. } | RoundEvent::Fizzle { attacker, .. } => {
                    self.shot(stats, event, attacker.slot(), &moved);
                }
                _ => {}
            }
        }
    }

    /// Zählt einen Schuss bei seinem Feuerbefehl.
    fn shot(&self, stats: &mut Stats, event: &RoundEvent, attacker: SlotRef, moved: &HashSet<i64>) {
        let shooter = self.at.get(&attacker).copied();
        let order = self
            .fire
            .iter()
            .find(|order| order.side == attacker.side && Some(order.shooter_id) == shooter);
        let outcomes = &mut stats.outcomes;
        let Some(order) = order else {
            outcomes.swarm += 1;
            if matches!(event, RoundEvent::Fizzle { .. }) {
                outcomes.swarm_fizzled += 1;
            }
            return;
        };
        if order.mode == Mode::Column {
            let column = &mut outcomes.column;
            column.shots += 1;
            match *event {
                RoundEvent::Attack { through, taunt, .. } => {
                    if taunt {
                        column.taunted += 1;
                    } else {
                        column.hit += 1;
                    }
                    if through {
                        column.through += 1;
                    }
                }
                RoundEvent::Fizzle {
                    miss: Miss::Column { cleared: true, .. },
                    ..
                } => column.cleared += 1,
                _ => column.void += 1,
            }
            return;
        }
        let ship = &mut outcomes.ship;
        ship.shots += 1;
        let followed = moved.contains(&order.target_id);
        if followed {
            ship.moved += 1;
        }
        match *event {
            RoundEvent::Attack { taunt, .. } => {
                if taunt {
                    ship.taunted += 1;
                } else {
                    ship.hit += 1;
                    if followed {
                        ship.moved_hit += 1;
                    }
                }
            }
            RoundEvent::Fizzle {
                miss: Miss::Target { target, covered },
                ..
            } => {
                if covered {
                    ship.covered += 1;
                    let target = target.expect("ein gedecktes Ziel steht").slot();
                    let front = self.at[&target.in_lane(LaneType::Strike)];
                    let cover = &mut outcomes.cover;
                    if followed {
                        cover.retreated += 1;
                    } else if front < 0 {
                        cover.arrived += 1;
                    } else if moved.contains(&front) {
                        cover.moved += 1;
                    } else {
                        cover.held += 1;
                    }
                } else if target.is_some() {
                    ship.destroyed += 1;
                } else {
                    ship.gone += 1;
                }
            }
            _ => {}
        }
    }
}

// --- Partie ---

/// Ersatzzug, falls die Zugprüfung einen Bot-Zug ablehnt: nur das Flaggschiff in Runde 1, sonst passen.
fn fallback(round: u32) -> TurnPlan {
    TurnPlan {
        flagship: (round == 1).then_some(SlotTarget::new(LaneType::Command, 2)),
        ..TurnPlan::default()
    }
}

/// Spielt eine Partie mit dem Zufall aus `seed` und zählt in `stats`.
fn play(stats: &mut Stats, admirals: [AdmiralId; 2], seed: u64) {
    let mut game = Match::with_rng("simulate", StdRng::seed_from_u64(seed));
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5eed_b075);
    game.join("a").expect("Sitz frei");
    game.join("b").expect("Sitz frei");
    for (seat, admiral) in Seat::ALL.into_iter().zip(admirals) {
        game.choose_admiral(seat, admiral)
            .expect("noch nicht gewählt");
    }
    let admiral_of = |side: Side| admirals[Seat::of(side).index()];
    let mut first_kill = None;
    let mut flagship_fall = None;
    let mut duel_rounds = 0;
    while game.outcome().is_none() && game.round() <= ROUND_LIMIT {
        let board = game.board();
        let dueling = Side::ALL.into_iter().all(|side| {
            board.flagship(side).is_some()
                && board.ships(side).all(|at| {
                    let card = board.ship(at).card;
                    card.is_flagship() || card.is_token()
                })
        });
        let mut turns = Vec::new();
        for seat in Seat::ALL {
            if let Some(offer) = game.view(seat).draft {
                let pick = plan_bot_draft(&game.bot_view(seat), &offer);
                game.draft(seat, pick).expect("der Bot wählt gültig");
            }
            let view = game.bot_view(seat);
            if view.command_hand.is_empty() {
                stats.empty_command_hand[seat.index()] += 1;
            }
            for &id in &view.command_hand {
                stats.card(id).held += 1;
            }
            let mut plan = plan_bot_turn(&view, &mut rng);
            if let Err(error) = game.submit(seat, plan.clone()) {
                *stats.errors.entry(error).or_default() += 1;
                plan = fallback(game.round());
                game.submit(seat, plan.clone()).expect("der Ersatzzug gilt");
            }
            turns.push((seat.side(), admirals[seat.index()], plan));
        }
        let tracker = FireTracker::new(stats, game.board(), &turns);
        let round = u64::from(game.round());
        // Wo die Flaggschiffe vor der Runde stehen: Zieht ein Traktorstrahl den Wächter davor weg, liegt eins frei.
        let flagships: Vec<SlotRef> = Side::ALL
            .into_iter()
            .filter_map(|side| game.board().flagship(side))
            .collect();
        let events = game.resolve_round();
        tracker.finish(stats, &events);
        if dueling {
            duel_rounds += 1;
            count_duel(stats, &turns, &events);
        }
        let destroyed = events
            .iter()
            .filter(|event| matches!(event, RoundEvent::Destroyed { .. }))
            .count() as u64;
        stats.kills += destroyed;
        if destroyed == 0 {
            stats.rounds_without_kill += 1;
        } else {
            first_kill = first_kill.or(Some(round));
        }
        if round > 1 {
            for side in Side::ALL {
                stats.usage[admiral_index(admiral_of(side))].rounds += 1;
            }
        }
        let flagship_fell = events.iter().any(
            |event| matches!(event, RoundEvent::Destroyed { ship } if ship.card_id.is_flagship()),
        );
        if flagship_fell {
            flagship_fall = flagship_fall.or(Some(round));
        }
        count_events(stats, &events, admirals, &flagships);
    }
    if duel_rounds > 0 {
        stats.duel.lengths.push(duel_rounds);
    }
    stats.first_kills.extend(first_kill);
    stats.flagship_falls.extend(flagship_fall);
    for admiral in admirals {
        stats.usage[admiral_index(admiral)].games += 1;
    }
    let winner = game.outcome().map(|outcome| match outcome {
        Outcome::Won(side) => Winner::Seat(Seat::of(side).index()),
        Outcome::Draw => Winner::Draw,
    });
    stats.games.push(Game {
        admirals,
        winner,
        rounds: game.round().min(ROUND_LIMIT),
    });
}

/// Zählt eine Runde im Flaggschiff-Duell: Feuerbefehle je Modus, Schüsse, verpuffte Schüsse und Grundmanöver.
fn count_duel(stats: &mut Stats, turns: &[(Side, AdmiralId, TurnPlan)], events: &[RoundEvent]) {
    for (_, _, plan) in turns {
        for order in &plan.fire {
            stats.duel.orders.count(match order.target {
                FireTarget::Ship(_) => Mode::Ship,
                FireTarget::Column(_) => Mode::Column,
            });
        }
    }
    let shots = |event: &&RoundEvent| {
        matches!(event, RoundEvent::Attack { .. } | RoundEvent::Fizzle { .. })
    };
    stats.duel.shots += events.iter().filter(shots).count() as u64;
    stats.duel.fizzled += events
        .iter()
        .filter(|event| matches!(event, RoundEvent::Fizzle { .. }))
        .count() as u64;
    stats.duel.maneuvers += events
        .iter()
        .filter(|event| matches!(event, RoundEvent::Maneuver { .. } | RoundEvent::Swap { .. }))
        .count() as u64;
}

/// Zählt die Ereignisse einer Runde: Schüsse, Schaden, Grundmanöver, Befehle, Fähigkeiten und die Kennzahlen der
/// Erweiterung. `flagships` stehen vor der Runde.
fn count_events(
    stats: &mut Stats,
    events: &[RoundEvent],
    admirals: [AdmiralId; 2],
    flagships: &[SlotRef],
) {
    let admiral_of = |side: Side| admirals[Seat::of(side).index()];
    for event in events {
        match *event {
            RoundEvent::Attack {
                amount,
                through,
                taunt,
                ..
            } => {
                stats.shots.all += 1;
                stats.shots.through += u64::from(through);
                stats.shots.taunted += u64::from(taunt);
                stats.combat_damage += i64::from(amount);
            }
            RoundEvent::Fizzle { .. } => {
                stats.shots.all += 1;
                stats.shots.fizzled += 1;
            }
            RoundEvent::Maneuver { side, .. } => {
                evasion(stats, event, events);
                stats.usage[admiral_index(admiral_of(side))].maneuvers += 1;
            }
            RoundEvent::Swap { side, .. } => {
                stats.usage[admiral_index(admiral_of(side))].maneuvers += 1;
            }
            RoundEvent::Effect { effect, .. } => {
                if let CommandEffect::Damage { amount } = effect {
                    stats.command_damage += i64::from(amount);
                }
                new_card_effect(stats, event, flagships, events);
            }
            RoundEvent::Command { card_id, .. } => {
                stats.commands += 1;
                stats.card(card_id).played += 1;
            }
            RoundEvent::NoEffect { card_id, .. } | RoundEvent::Miss { card_id, .. } => {
                stats.commands_without_effect += 1;
                stats.card(card_id).idle += 1;
            }
            RoundEvent::Countered { countered, .. } => {
                stats.card(countered).countered += 1;
                stats.expansion.countered += 1;
            }
            RoundEvent::Draw {
                card_id: CommandId::Countermeasures,
                ..
            } => stats.expansion.counter_draws += 1,
            RoundEvent::Power { side, .. } => {
                stats.usage[admiral_index(admiral_of(side))].powers += 1;
            }
            _ => {}
        }
        if let Some((side, key, amount)) = ability_effect(event) {
            stats.effect(admiral_of(side), key, amount);
        }
    }
    for side in Side::ALL {
        let own = |event: &RoundEvent| event_side(event) == Some(side);
        let carte_blanche = events.iter().any(|event| {
            own(event)
                && matches!(
                    event,
                    RoundEvent::Rule {
                        card_id: CommandId::CarteBlanche,
                        ..
                    }
                )
        });
        if carte_blanche {
            let maneuvered = events.iter().any(|event| {
                own(event) && matches!(event, RoundEvent::Maneuver { .. } | RoundEvent::Swap { .. })
            });
            let powered = events
                .iter()
                .any(|event| own(event) && matches!(event, RoundEvent::Power { .. }));
            stats.expansion.carte_blanche_both += u64::from(maneuvered && powered);
        }
    }
}

/// Die Seite, die ein Ereignis mit eigenem `side`-Feld nennt.
fn event_side(event: &RoundEvent) -> Option<Side> {
    match *event {
        RoundEvent::Deploy { side, .. }
        | RoundEvent::Pass { side }
        | RoundEvent::Power { side, .. }
        | RoundEvent::Mark { side, .. }
        | RoundEvent::Maneuver { side, .. }
        | RoundEvent::Swap { side, .. }
        | RoundEvent::Command { side, .. }
        | RoundEvent::Effect { side, .. }
        | RoundEvent::Draw { side, .. }
        | RoundEvent::NoEffect { side, .. }
        | RoundEvent::Countered { side, .. }
        | RoundEvent::Rule { side, .. }
        | RoundEvent::Miss { side, .. } => Some(side),
        _ => None,
    }
}

// --- Ausgabe ---

/// Der Name, wie er übertragen wird (`snake_case`).
fn name_of(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("Ids werden als Text übertragen")
}

fn percent(part: u64, whole: u64) -> String {
    format!("{:.1} %", 100.0 * part as f64 / whole.max(1) as f64)
}

fn average(values: &[u64]) -> String {
    format!(
        "{:.1}",
        values.iter().sum::<u64>() as f64 / values.len().max(1) as f64
    )
}

/// Siegquote von `admiral` gegen `opponent`; Unentschieden und Patts zählen halb.
fn score(games: &[Game], admiral: AdmiralId, opponent: AdmiralId) -> f64 {
    let mut points = 0.0;
    let mut count = 0;
    for game in games {
        let seat = if game.admirals == [admiral, opponent] {
            0
        } else if game.admirals == [opponent, admiral] {
            1
        } else {
            continue;
        };
        count += 1;
        points += match game.winner {
            Some(Winner::Seat(winner)) if winner == seat => 1.0,
            Some(Winner::Seat(_)) => 0.0,
            Some(Winner::Draw) | None => 0.5,
        };
    }
    points / f64::from(count)
}

fn mode_share(modes: Modes) -> String {
    format!(
        "{} von {}",
        percent(modes.ship, modes.total()),
        modes.total()
    )
}

fn report(stats: &Stats, games_per_pairing: usize) {
    let games = &stats.games;
    let rounds: Vec<u64> = games.iter().map(|game| u64::from(game.rounds)).collect();
    let total_rounds: u64 = rounds.iter().sum();
    let mut sorted = rounds.clone();
    sorted.sort_unstable();
    let quantile = |q: f64| sorted[((q * sorted.len() as f64) as usize).min(sorted.len() - 1)];
    let admiral_name = |id: AdmiralId| format!("{} ({})", name_of(id), name_of(id.flagship()));

    println!(
        "{games_per_pairing} Partien je Paarung Bot gegen Bot, Sitze abwechselnd, {} insgesamt",
        games.len()
    );
    println!("Siegquote der Zeile gegen die Spalte (Unentschieden und Patts halb):");
    let header: String = ADMIRAL_IDS
        .iter()
        .map(|&id| format!("{:>11}", name_of(id)))
        .collect();
    println!("{:20}{header}     gesamt", "");
    let mut pairings = Vec::new();
    for row in ADMIRAL_IDS {
        let others: Vec<AdmiralId> = ADMIRAL_IDS.into_iter().filter(|&id| id != row).collect();
        let cells: String = ADMIRAL_IDS
            .iter()
            .map(|&column| {
                let cell = if column == row {
                    "–".to_owned()
                } else {
                    format!("{:.0} %", 100.0 * score(games, row, column))
                };
                format!("{cell:>11}")
            })
            .collect();
        let scores: Vec<f64> = others
            .iter()
            .map(|&column| score(games, row, column))
            .collect();
        let total = scores.iter().sum::<f64>() / scores.len() as f64;
        pairings.extend(scores);
        println!("{:20}{cells}   {:>6.1} %", admiral_name(row), 100.0 * total);
    }
    let low = pairings.iter().copied().fold(f64::INFINITY, f64::min);
    let high = pairings.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    println!("Paarungen: {:.1} bis {:.1} %", 100.0 * low, 100.0 * high);
    println!(
        "Runden: Ø {}, Median {}, p10 {}, p90 {}, max {}",
        average(&rounds),
        quantile(0.5),
        quantile(0.1),
        quantile(0.9),
        sorted.last().copied().unwrap_or(0)
    );
    let early = games
        .iter()
        .filter(|game| game.winner.is_some() && game.rounds <= 6)
        .count();
    println!(
        "Bis Runde 6 entschieden: {}",
        percent(early as u64, games.len() as u64)
    );
    let draws = games
        .iter()
        .filter(|game| game.winner == Some(Winner::Draw))
        .count();
    let stalemates = games.iter().filter(|game| game.winner.is_none()).count();
    println!("Unentschieden: {draws}, Patts (Runde {ROUND_LIMIT} erreicht): {stalemates}");
    println!("Einsatz ab Runde 2 (Anteil der Runden) und Wirkung der Fähigkeiten je Partie:");
    for id in ADMIRAL_IDS {
        let usage = &stats.usage[admiral_index(id)];
        let per_game: Vec<String> = usage
            .effects
            .iter()
            .map(|(key, &amount)| format!("{key} {:.1}", amount as f64 / usage.games.max(1) as f64))
            .collect();
        println!(
            "  {:20} Fähigkeit {:>7}, Grundmanöver {:>7}; {}",
            admiral_name(id),
            percent(usage.powers, usage.rounds),
            percent(usage.maneuvers, usage.rounds),
            per_game.join(", ")
        );
    }
    println!(
        "Abschüsse pro Runde: {:.2}, Runden ohne Abschuss: {}",
        stats.kills as f64 / total_rounds.max(1) as f64,
        percent(stats.rounds_without_kill, total_rounds)
    );
    println!(
        "Erster Abschuss: Ø Runde {}; erstes Flaggschiff fällt: Ø Runde {}",
        average(&stats.first_kills),
        average(&stats.flagship_falls)
    );
    let Outcomes {
        column,
        ship,
        cover,
        swarm,
        swarm_fizzled,
    } = &stats.outcomes;
    let shots = &stats.shots;
    println!(
        "Schüsse im Kampf: {:.2} je Runde, davon verpufft {} (Spalte ins Leere {}, Spalte nach Abschuss {}, \
         Schiff gedeckt {}, Schiff schon zerstört {}, Schiff weg {}, Schwarmangriff {}), durchgegangen {}, \
         von Ablenkung gezogen {}",
        shots.all as f64 / total_rounds.max(1) as f64,
        percent(shots.fizzled, shots.all),
        percent(column.void, shots.all),
        percent(column.cleared, shots.all),
        percent(ship.covered, shots.all),
        percent(ship.destroyed, shots.all),
        percent(ship.gone, shots.all),
        percent(*swarm_fizzled, shots.all),
        percent(shots.through, shots.all),
        percent(shots.taunted, shots.all)
    );
    let orders = &stats.orders;
    let per_admiral: Vec<String> = ADMIRAL_IDS
        .iter()
        .map(|&id| {
            format!(
                "{} {}",
                name_of(id),
                mode_share(orders.admiral[admiral_index(id)])
            )
        })
        .collect();
    println!(
        "Feuerbefehle: {:.2} je Runde, Schiffsfeuer {}; je Admiral {}",
        orders.all.total() as f64 / total_rounds.max(1) as f64,
        mode_share(orders.all),
        per_admiral.join(", ")
    );
    println!(
        "  Schiffsfeuer je Lane des Ziels beim Planen (Spaltenfeuer: vorderstes Schiff der Spalte): Strike {}, \
         Command {}; auf eine leere Spalte {}",
        mode_share(orders.lane.strike),
        mode_share(orders.lane.command),
        orders.lane.empty.column
    );
    println!(
        "  Spaltenfeuer: {} Schüsse, Treffer {} (durchgegangen {}), von Ablenkung gezogen {}, ins Leere {}, \
         nach Abschuss {}",
        column.shots,
        percent(column.hit, column.shots),
        percent(column.through, column.shots),
        percent(column.taunted, column.shots),
        percent(column.void, column.shots),
        percent(column.cleared, column.shots)
    );
    println!(
        "  Schiffsfeuer: {} Schüsse, Treffer {}, von Ablenkung gezogen {}, gedeckt {} (Deckung stand schon und \
         hielt {}, Ziel zog in Deckung {}, neues Schiff davor {}, Schiff zog davor {}), schon zerstört {}, weg {}",
        ship.shots,
        percent(ship.hit, ship.shots),
        percent(ship.taunted, ship.shots),
        percent(ship.covered, ship.shots),
        percent(cover.held, ship.covered),
        percent(cover.retreated, ship.covered),
        percent(cover.arrived, ship.covered),
        percent(cover.moved, ship.covered),
        percent(ship.destroyed, ship.shots),
        percent(ship.gone, ship.shots)
    );
    println!(
        "  Schiffsfeuer auf ein Ziel, das vor dem Kampf den Slot wechselte: {}, davon getroffen {}; \
         Schwarmangriff ohne Feuerbefehl: {swarm} Schüsse",
        ship.moved,
        percent(ship.moved_hit, ship.moved)
    );
    let duel = &stats.duel;
    let duel_rounds: u64 = duel.lengths.iter().sum();
    println!(
        "Flaggschiff-Duell (beide nur noch mit Flaggschiff): in {} der Partien, Ø {} Runden, max {}; darin \
         Schiffsfeuer {} Befehlen, verpufft {} der Schüsse, Grundmanöver {:.2} je Runde",
        percent(duel.lengths.len() as u64, games.len() as u64),
        average(&duel.lengths),
        duel.lengths.iter().max().copied().unwrap_or(0),
        mode_share(duel.orders),
        percent(duel.fizzled, duel.shots),
        duel.maneuvers as f64 / duel_rounds.max(1) as f64
    );
    let maneuvers = &stats.maneuvers;
    println!(
        "Grundmanöver: {:.2} je Runde ({} der Züge), davon in eine andere Spalte {}; auf deren alte Spalte schoss \
         der Gegner bei {}, {} ließen Schüsse ins Leere laufen ({:.2} je Ausweichen), {} wurden in der neuen \
         Spalte trotzdem getroffen",
        maneuvers.all as f64 / total_rounds.max(1) as f64,
        percent(maneuvers.all, 2 * total_rounds),
        percent(maneuvers.evasions, maneuvers.all),
        percent(maneuvers.targeted, maneuvers.evasions),
        percent(maneuvers.dodged, maneuvers.evasions),
        maneuvers.voided as f64 / maneuvers.dodged.max(1) as f64,
        percent(maneuvers.struck, maneuvers.evasions)
    );
    let damage = |amount: i64| u64::try_from(amount).expect("Schaden ist nicht negativ");
    println!(
        "Schaden durch Befehle: {} (Nennwerte, ohne Eigenschaden)",
        percent(
            damage(stats.command_damage),
            damage(stats.command_damage + stats.combat_damage)
        )
    );
    println!(
        "Befehle ohne Wirkung: {} (noEffect und Fehlschüsse)",
        percent(stats.commands_without_effect, stats.commands)
    );
    println!(
        "Leere Befehlshand: Seat 0 {}, Seat 1 {}",
        percent(stats.empty_command_hand[0], total_rounds),
        percent(stats.empty_command_hand[1], total_rounds)
    );
    println!(
        "Befehle je Karte: gespielt (Anteil an allen), wie oft gespielt je Runde auf der Hand, davon ohne Wirkung \
         bzw. von Gegenmaßnahmen verpufft:"
    );
    let mut cards: Vec<(&CommandId, &CardStats)> = stats.cards.iter().collect();
    cards.sort_by_key(|(id, card)| (std::cmp::Reverse(card.played), name_of(**id)));
    for (&id, card) in cards {
        println!(
            "  {:18} {:>6} {:>7}   je Handrunde {:>7}   ohne Wirkung {:>7}   verpufft {:>7}",
            name_of(id),
            card.played,
            percent(card.played, stats.commands),
            percent(card.played, card.held),
            percent(card.idle, card.played),
            percent(card.countered, card.played)
        );
    }
    println!("Neue Karten (Anzahl über alle Partien):");
    let expansion = &stats.expansion;
    let played = |id: CommandId| stats.cards.get(&id).map_or(0, |card| card.played);
    println!(
        "  Kurzsprünge {}, davon Flaggschiff {}",
        expansion.jumps, expansion.flagship_jumps
    );
    println!(
        "  Rammstöße {}, Ziel zerstört {}, Rammer zerstört {}",
        expansion.rams, expansion.rammed_destroyed, expansion.rammers_destroyed
    );
    println!(
        "  Gegenmaßnahmen {}: Befehl verpufft {}, gezogen {}",
        played(CommandId::Countermeasures),
        expansion.countered,
        expansion.counter_draws
    );
    println!(
        "  Funkstille {}: entfernte Marken bzw. Trugbilder {}",
        played(CommandId::RadioSilence),
        expansion.jammed
    );
    println!(
        "  Traktorstrahl {}: Flaggschiff freigelegt {}, danach beschossen {}",
        played(CommandId::TractorBeam),
        expansion.tractor_exposed,
        expansion.tractor_exposed_hit
    );
    println!(
        "  Wärmesucher {}: mit 5 Schaden {}",
        played(CommandId::HeatSeeker),
        expansion.heat_seeker_moved
    );
    let per_card = |count: u64, id: CommandId| count as f64 / played(id).max(1) as f64;
    println!(
        "  Minenfeld {}: Treffer je Karte {:.2}",
        played(CommandId::Minefield),
        per_card(expansion.mine_hits, CommandId::Minefield)
    );
    println!(
        "  Drohnenstaffel {}: Drohnen je Karte {:.2}",
        played(CommandId::DroneSquadron),
        per_card(expansion.squadron_drones, CommandId::DroneSquadron)
    );
    println!(
        "  Freie Hand {}: mit Manöver und Fähigkeit {}",
        played(CommandId::CarteBlanche),
        expansion.carte_blanche_both
    );
    if stats.errors.is_empty() {
        println!("Zugprüfung: keine Fehler");
    } else {
        let errors: Vec<String> = stats
            .errors
            .iter()
            .map(|(&code, count)| format!("{} {count}×", name_of(code)))
            .collect();
        println!("Zugprüfung: {}", errors.join(", "));
    }
}

fn main() {
    let games_per_pairing: usize = env::args().nth(1).map_or(2000, |arg| {
        arg.parse().expect("Partien je Paarung als Zahl")
    });
    let started = Instant::now();
    let jobs: Vec<[AdmiralId; 2]> = ADMIRAL_IDS
        .iter()
        .enumerate()
        .flat_map(|(index, &first)| {
            ADMIRAL_IDS[index..].iter().flat_map(move |&second| {
                (0..games_per_pairing).map(move |game| {
                    if game % 2 == 0 {
                        [first, second]
                    } else {
                        [second, first]
                    }
                })
            })
        })
        .collect();
    let next = AtomicUsize::new(0);
    let workers = thread::available_parallelism().map_or(1, usize::from);
    let mut stats = Stats::default();
    thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut stats = Stats::default();
                    loop {
                        let job = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&admirals) = jobs.get(job) else {
                            return stats;
                        };
                        play(&mut stats, admirals, job as u64);
                    }
                })
            })
            .collect();
        for handle in handles {
            stats.absorb(handle.join().expect("eine Partie ist abgestürzt"));
        }
    });
    report(&stats, games_per_pairing);
    println!(
        "Dauer {:.1} s auf {workers} Threads",
        started.elapsed().as_secs_f64()
    );
}
