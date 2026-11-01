//! Der Computergegner: plant aus Brett und eigener Hand einen Zug. Er sieht nur, was auch ein Spieler sieht,
//! insbesondere nicht den Plan des Gegners ([`BotView`]). Portiert aus `server/src/bot.ts`.
//!
//! Faustregeln: immer so viele Schiffe legen wie erlaubt (in Runde 1 das Flaggschiff mittig in die Command-Lane und das
//! robusteste Kampfschiff davor; sonst Kampfschiffe zuerst, robuste nach vorne, Jäger und Support in einen gedeckten
//! Command-Slot, [`deploy`]), den Befehl spielen, der nach grober Schätzung am meisten Schaden anrichtet oder
//! verhindert ([`commands`]), und das Grundmanöver nutzen, um das Flaggschiff oder ein anderes Schiff zu retten, in
//! Deckung zu bringen oder ein anderes zu decken ([`maneuver`]). Die Admiralsfähigkeit nimmt er statt des Manövers, wenn
//! sie im Kampf dieser Runde mehr bringt. Befehle, die im Kampf dieser Runde wirken, spielt er auf einer Kopie durch
//! ([`situation`]); was vom unbekannten Gegnerzug abhängt (Gegenmaßnahmen, Funkstille, Minenfeld), schätzt er.
//!
//! Die Feuerbefehle gibt er zuletzt, für den fertigen Zug ([`fire`]): je Schütze eine Spalte der Gegnerseite, in der
//! das vorderste Schiff den Schuss nimmt und Overkill auf das Schiff dahinter durchgeht, oder ein Schiff, dem der Schuss
//! folgt, wenn es ausweicht, der aber an Deckung verpufft. Was der Gegner beschießt, vermutet er mit derselben Zielwahl
//! aus dessen Sicht ([`threat::predict_fire`]); im Probekampf zielen beide so. Ab und zu weicht er einem vermuteten
//! Torpedo aus, zielt auf ein Ausweichmanöver oder hält vor, wohin das gegnerische Flaggschiff in Deckung ausweicht,
//! damit ihn ein Mensch nicht einfach berechnen kann. Zufall kommt nur aus dem übergebenen `rng`.

mod commands;
mod deploy;
mod fire;
mod maneuver;
mod situation;
mod threat;

#[cfg(test)]
mod tests;

use rand::seq::IndexedRandom;
use rand::{Rng, RngCore};
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::board::{
    Board, LaneType, SLOT_COUNT, Side, SlotRef, SlotTarget, lane_slots, max_deploys,
};
use voidfleet_core::cards::{CommandId, ShipClass, ShipId, allows_maneuver_and_power};
use voidfleet_core::powers::can_mark;
use voidfleet_core::protocol::{CommandPlan, DraftOffer, DraftPick, TurnPlan};

pub use crate::game::BotView;

use self::deploy::plan_ships;
use self::situation::{Situation, Trial, TrialCommand};
use self::threat::{Lead, flagship_escapes, guard_of, predict_fire};

/// Runde 1: Das Flaggschiff kommt mittig in die Command-Lane, das robusteste Kampfschiff davor.
const FLAGSHIP_SLOT: usize = 2;
/// Ein Abschuss ist mehr wert als der reine Schaden.
const KILL_BONUS: i32 = 2;
/// Schaden am Flaggschiff zählt doppelt; sein Fall entscheidet das Match.
const FLAGSHIP_WEIGHT: i32 = 2;
const DECISIVE: f64 = 1000.0;
/// Ein Flaggschiff vorne zieht sich ab so wenigen HP in Deckung zurück.
const FLAGSHIP_RETREAT_HP: i32 = 7;
/// So oft weicht er dem Schiff aus, auf das ein Torpedo am ehesten zielt, bzw. zielt auf ein Ausweichmanöver.
const SURPRISE_CHANCE: f64 = 0.15;
/// Gewicht eines Torpedoziels, das sich wahrscheinlich bewegt.
const UNSURE: f64 = 0.5;
/// Wie wahrscheinlich ein angeschlagenes, ungedecktes Schiff die Lane wechselt (für das Sperrfeuer).
const FLEE_CHANCE: f64 = 0.4;
/// So oft tauscht die Mirage den Platz, wenn ein Torpedo droht, damit er das Schiff trifft, mit dem sie getauscht hat.
const SWAP_CHANCE: f64 = 0.4;

/// Ausweichen vor dem vermuteten Feuer ([`maneuver`]): Bedroht ist ein Schiff, das fällt oder mindestens `hit` Schaden
/// nimmt; das Ausweichen muss im Probekampf mindestens `gain` bringen; ein gedeckter Ziel-Slot zählt `cover` mehr, weil
/// er auch in den Folgerunden schützt; unter den Slots, die höchstens `margin` weniger bringen als der beste,
/// entscheidet der Zufall; geht es nicht um das Match, bleibt das Schiff mit `stay` trotzdem stehen.
struct Evade {
    hit: i32,
    gain: f64,
    cover: f64,
    margin: f64,
    stay: f64,
}

const EVADE: Evade = Evade {
    hit: 4,
    gain: 4.0,
    cover: 2.0,
    margin: 1.0,
    stay: 0.2,
};

/// Vorhalten ([`Lead`]): So oft setzt er darauf, dass das gegnerische Flaggschiff ausweicht: Reicht das Feuer nicht für
/// den Abschuss, wo es steht, und hinter der Deckung, hinter die es wohl ausweicht, geht die Deckung vor; und kann
/// Moreau ohne Schiff auf der Hand nur noch ihr Trugbild davor stellen oder ausweichen, nimmt er Schiffsfeuer statt
/// Spaltenfeuer.
const LEAD_CHANCE: f64 = 0.35;

/// Kerr: wie wahrscheinlich ein gedecktes Ziel beim Schuss der Railgun ungedeckt ist, bei schwacher bzw. stabiler
/// Deckung.
struct Kerr {
    weak_cover: f64,
    covered: f64,
}

const KERR: Kerr = Kerr {
    weak_cover: 0.3,
    covered: 0.15,
};

/// Admiralsfähigkeit oder Grundmanöver: Das Manöver lohnt oft erst in den Folgerunden und bekommt den Vorschuss
/// `maneuver_head_start`. `future`: so viele Runden Feuer zählt jedes Schiff zusätzlich (ein Abschuss nimmt dem Gegner
/// auch die nächsten Schüsse).
struct Power {
    maneuver_head_start: f64,
    future: i32,
}

const POWER: Power = Power {
    maneuver_head_start: 1.0,
    future: 2,
};

/// Schätzungen zum unbekannten Gegnerzug: wie oft er seine Admiralsfähigkeit einsetzt (Funkstille), wie oft er einen
/// Angriff oder eine Schwächung spielt, wenn er einen hält (Gegenmaßnahmen), wie sicher er ein Schiff legt und wie oft
/// Moreau ein Trugbild stellt (Minenfeld).
struct Guess {
    power: f64,
    hostile: f64,
    deploy: f64,
    decoy: f64,
}

const GUESS: Guess = Guess {
    power: 0.45,
    hostile: 0.8,
    deploy: 0.9,
    decoy: 0.3,
};

/// Gegenmaßnahmen: was ein typischer Angriff oder eine Schwächung anrichtet, und was ein gezogener Befehl wert ist.
struct Counter {
    typical: f64,
    draw: f64,
}

const COUNTER: Counter = Counter {
    typical: 3.0,
    draw: 1.0,
};

/// Schnellverlegung: Ein Schiff mehr auf dem Brett feuert auch in den Folgerunden; so viel zählt das über diese Runde
/// hinaus.
const RAPID_TEMPO: f64 = 1.0;

/// Befehle, die er zusätzlich zur Schätzung im Probekampf durchspielt ([`commands`]): Schaden, Schwächung, ATK und
/// Verlegung können dem eigenen Feuer einen Abschuss ermöglichen oder eine Deckung wegräumen. Torpedo, Wärmesucher und
/// Durchschlag spielt er für jedes Ziel durch.
const COMBOS: [CommandId; 13] = [
    CommandId::OrbitalStrike,
    CommandId::VolleyFire,
    CommandId::EmpBlast,
    CommandId::TargetingJammer,
    CommandId::PowerSurge,
    CommandId::Overcharge,
    CommandId::ScrapSalvage,
    CommandId::CoupDeGrace,
    CommandId::Bombardment,
    CommandId::Crossfire,
    CommandId::FlankingManeuver,
    CommandId::Intercept,
    CommandId::Recall,
];

/// Der Admiral des Computergegners: zufällig.
pub fn choose_bot_admiral(rng: &mut dyn RngCore) -> AdmiralId {
    *ADMIRAL_IDS.choose(rng).expect("es gibt Admirale")
}

/// Der Zug des Computergegners zur Stellung in `view`.
pub fn plan_bot_turn(view: &BotView, rng: &mut dyn RngCore) -> TurnPlan {
    let board = &view.board;
    let side = view.side;
    let enemy = side.opposite();
    let round = view.round;
    // Geplant wird auf einer Kopie, auf der die gelegten Schiffe schon stehen, in Runde 1 zuerst das Flaggschiff.
    let mut start = board.clone();
    let flagship = (round == 1).then_some(SlotTarget::new(LaneType::Command, FLAGSHIP_SLOT));
    if let Some(flagship) = flagship {
        start.place(flagship.on(side), view.admiral.flagship());
    }
    // Der Gegner zielt auf die Stellung, wie sie jetzt steht: Die Schiffe, die der Bot gleich legt, kennt er noch nicht.
    let enemy_fire = predict_fire(
        board,
        enemy,
        guard_of(view.ship_hand.len(), Some(view.admiral)),
    );
    // Vorhalten: Fällt das gegnerische Flaggschiff, zielt ein Teil des Feuers dorthin, wohin es wohl in Deckung
    // ausweicht.
    let opponent = &view.opponent;
    let escapes = flagship_escapes(
        board,
        enemy,
        guard_of(opponent.ship_hand_size, opponent.admiral),
    );
    let lead = board.flagship(enemy).map(|_| Lead {
        escapes,
        gamble: rng.random::<f64>() < LEAD_CHANCE,
    });
    let (start, enemy_fire, lead) = (&start, enemy_fire.as_slice(), lead.as_ref());
    let deploy = move |count: usize| {
        let mut after = start.clone();
        let ships = plan_ships(&mut after, view, count, enemy_fire);
        // Auch das Flaggschiff landet erst in dieser Runde und manövriert noch nicht.
        let arrivals: Vec<SlotTarget> = ships
            .iter()
            .map(|ship| ship.target)
            .chain(flagship)
            .collect();
        let situation = Situation::new(after, view, &arrivals, enemy_fire, lead);
        (ships, situation)
    };
    let (mut ships, mut situation) = deploy(max_deploys(round, None));
    // Schnellverlegung: Passt ein weiteres Schiff, zählt, was es schon in dieser Runde bringt; gewählt, legt er es.
    let rapid = view
        .command_hand
        .contains(&CommandId::RapidDeployment)
        .then(|| deploy(max_deploys(round, None) + 1));
    if let Some((extra_ships, extra)) = &rapid
        && extra_ships.len() > ships.len()
    {
        let target = extra_ships.last().expect("ein Schiff mehr").target;
        situation.extra_ship = Some(extra.arrival_gain(&situation, target));
    }
    let command = situation.plan_command(rng);
    let card = command
        .as_ref()
        .map(|(hand_index, _)| view.command_hand[*hand_index]);
    if card == Some(CommandId::RapidDeployment) {
        (ships, situation) = rapid.expect("Schnellverlegung ist auf der Hand");
    }
    let choice = command.as_ref().map(|(_, choice)| choice);
    let planned = situation.plan_maneuver(choice, rng);
    let (maneuver, power) = situation.plan_power(planned, choice, allows_maneuver_and_power(card));
    let mark = if can_mark(board, side) {
        situation.plan_mark(maneuver)
    } else {
        None
    };
    let fire = situation.plan_fire(&Trial {
        power: power.map(|power| power.target),
        maneuver,
        command: card.zip(choice).map(|(card, choice)| TrialCommand {
            card,
            target: choice.target,
            destination: choice.destination,
        }),
        enemy_power: None,
    });
    TurnPlan {
        ships,
        command: command.map(|(hand_index, choice)| CommandPlan {
            hand_index,
            target: choice.target.map(SlotRef::target),
            destination: choice.destination.map(|destination| destination.slot_index),
        }),
        maneuver,
        flagship,
        power,
        mark,
        fire,
    }
}

/// Die Wahl aus dem Nachschub: das stärkste Schiff ([`ship_value`], Kampfschiffe vor Support), die Schrottverwertung,
/// sonst den Befehl, von dem er am wenigsten auf der Hand hat (bei Gleichstand den ersten).
pub fn plan_bot_draft(view: &BotView, offer: &DraftOffer) -> DraftPick {
    let held = |id: CommandId| view.command_hand.iter().filter(|&&card| card == id).count();
    let command = if offer.commands.contains(&CommandId::ScrapSalvage) {
        offer
            .commands
            .iter()
            .position(|&card| card == CommandId::ScrapSalvage)
    } else {
        lowest(0..offer.commands.len(), |&index| {
            held(offer.commands[index])
        })
    };
    DraftPick {
        ship: highest(0..offer.ships.len(), |&index| {
            ship_value(offer.ships[index])
        }),
        command,
    }
}

// --- Gemeinsame Hilfen ---

/// Das Element mit dem höchsten Wert; bei Gleichstand das erste.
fn highest<T, V: PartialOrd>(
    items: impl IntoIterator<Item = T>,
    mut value: impl FnMut(&T) -> V,
) -> Option<T> {
    let mut best: Option<(T, V)> = None;
    for item in items {
        let score = value(&item);
        if best.as_ref().is_none_or(|(_, top)| score > *top) {
            best = Some((item, score));
        }
    }
    best.map(|(item, _)| item)
}

/// Das Element mit dem niedrigsten Wert; bei Gleichstand das erste.
fn lowest<T, V: PartialOrd>(
    items: impl IntoIterator<Item = T>,
    mut value: impl FnMut(&T) -> V,
) -> Option<T> {
    let mut best: Option<(T, V)> = None;
    for item in items {
        let score = value(&item);
        if best.as_ref().is_none_or(|(_, bottom)| score < *bottom) {
            best = Some((item, score));
        }
    }
    best.map(|(item, _)| item)
}

/// Der Slot nächst der Lane-Mitte (zwischen Slot 2 und 3); bei Gleichstand der erste.
fn near_center(slots: impl IntoIterator<Item = SlotRef>) -> Option<SlotRef> {
    slots
        .into_iter()
        .min_by_key(|slot| (2 * slot.slot_index).abs_diff(SLOT_COUNT - 1))
}

/// Freier Slot der Lane nächst der Mitte, für ein Schiff, das zur Lane-Grenze zählt.
fn free_slot_near_center(board: &Board, side: Side, lane: LaneType) -> Option<SlotRef> {
    if board.lane_is_full(side, lane) {
        return None;
    }
    near_center(lane_slots(side, lane).filter(|&at| board.is_free(at)))
}

/// Kampfschiffe vor Support, dann nach Werten.
fn ship_value(card: ShipId) -> i32 {
    let stats = card.card();
    (if stats.atk > 0 { 100 } else { 0 }) + stats.hp + stats.shield + 2 * stats.atk
}

/// Jäger und Support gehören in Deckung.
fn is_fragile(card: ShipId) -> bool {
    matches!(card.card().class, ShipClass::Fighter | ShipClass::Support)
}

fn card(board: &Board, at: SlotRef) -> ShipId {
    board.ship(at).card
}

fn attacks(board: &Board, at: SlotRef) -> bool {
    card(board, at).atk() > 0
}

fn hp(board: &Board, at: SlotRef) -> i32 {
    board.ship(at).hp
}

fn durability(board: &Board, at: SlotRef) -> i32 {
    let ship = board.ship(at);
    ship.hp + ship.shield
}

fn damaged(board: &Board, at: SlotRef) -> bool {
    let ship = board.ship(at);
    ship.hp < ship.card.hp()
}

fn is_flagship(board: &Board, at: SlotRef) -> bool {
    card(board, at).is_flagship()
}

fn is_token(board: &Board, at: SlotRef) -> bool {
    card(board, at).is_token()
}

/// Schaden, der bei einem Schiff ankommt, plus Bonus, wenn es dabei zerstört wird; beim Flaggschiff doppelt, sein Fall
/// entscheidet.
fn damage_value(board: &Board, at: SlotRef, amount: i32) -> f64 {
    let remaining = durability(board, at);
    let dealt = f64::from(amount.min(remaining));
    let killed = remaining <= amount;
    if is_flagship(board, at) {
        dealt * f64::from(FLAGSHIP_WEIGHT) + if killed { DECISIVE } else { 0.0 }
    } else {
        dealt + if killed { f64::from(KILL_BONUS) } else { 0.0 }
    }
}

/// Der Slot gleicher Nummer in der anderen Lane derselben Seite.
fn partner(at: SlotRef) -> SlotRef {
    at.in_lane(at.lane.other())
}

/// Wie viele Kopien eines Befehls der Gegner noch nicht gespielt hat.
fn unplayed(view: &BotView, id: CommandId) -> i32 {
    let played = view
        .opponent
        .played_commands
        .iter()
        .filter(|&&card| card == id)
        .count();
    id.deck_copies() as i32 - played as i32
}

/// Ob ein Schiff wahrscheinlich stehen bleibt: unbeschädigt, gedeckt hinten oder einziger Angreifer vorne. Für Befehle,
/// die einen Slot treffen (Torpedo, Durchschlag), und die Feuerbefehle ([`threat::risks`]); so zielt der Bot selbst, so
/// erwartet er es vom Gegner.
fn stays(board: &Board, at: SlotRef) -> bool {
    let lone_attacker = at.lane == LaneType::Strike
        && !board
            .occupied(at.side, LaneType::Strike)
            .any(|other| other != at && attacks(board, other));
    !damaged(board, at) || !board.is_exposed(at, false) || lone_attacker
}
