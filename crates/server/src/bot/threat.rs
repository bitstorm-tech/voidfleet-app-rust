//! Was die Seiten voneinander erwarten: wie Schiffe den Schüssen wohl entgehen ([`risks`]), worauf der Gegner feuert
//! ([`predict_fire`]) und wohin das gegnerische Flaggschiff in Deckung ausweicht ([`flagship_escapes`]).

use std::collections::HashMap;

use voidfleet_core::admirals::AdmiralId;
use voidfleet_core::board::{Board, LaneType, Side, SlotRef, SlotTarget};
use voidfleet_core::cards::ShipId;
use voidfleet_core::protocol::{FireOrder, FireTarget};
use voidfleet_core::resolution::{
    bind_fire, firing_railguns, resolve_combat, resolve_guardian_shields, resolve_maneuver,
};

use super::deploy::{guard_target, guardable};
use super::fire::{Target, aim_fire};
use super::{durability, free_slot_near_center, is_flagship, is_token, stays};

/// Vorhalten: die Spalten, in deren gedeckten Command-Slot das tödlich bedrohte gegnerische Flaggschiff wohl ausweicht,
/// die wahrscheinlichste zuerst ([`flagship_escapes`]), und ob der Bot auf das Ausweichen setzt
/// ([`super::LEAD_CHANCE`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Lead {
    pub(super) escapes: Vec<usize>,
    pub(super) gamble: bool,
}

/// Womit eine Seite in dieser Runde wohl noch Deckung schafft ([`risks`]): mit einem Schiff von ihrer Hand, vor ein
/// Command-Schiff oder einen freien Command-Slot, in den eins ausweicht; ohne Schiff mit Moreaus Trugbild, das sie
/// statt des Grundmanövers stellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Guard {
    Ship,
    Decoy,
}

pub(super) fn guard_of(ship_hand_size: usize, admiral: Option<AdmiralId>) -> Option<Guard> {
    if ship_hand_size > 0 {
        Some(Guard::Ship)
    } else if admiral == Some(AdmiralId::Moreau) {
        Some(Guard::Decoy)
    } else {
        None
    }
}

/// Wie ein Schiff den Schüssen der Gegenseite wohl entgeht ([`risks`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Risk {
    /// Es entgeht wohl jedem Schuss, auf es zu zielen zählt halb: Vor ihm landet das nächste Schiff seiner Seite, das
    /// die ersten Schüsse auf seine Spalte nimmt und an dem Schiffsfeuer verpufft, oder es weicht angeschlagen aus,
    /// und Schiffsfeuer folgt ihm nicht (`open`).
    pub(super) unsure: bool,
    /// Weicht es aus, dann nur ungedeckt: Dorthin folgt ihm Schiffsfeuer, Spaltenfeuer ginge ins Leere. Sonst bringt
    /// Schiffsfeuer nichts, was Spaltenfeuer nicht auch bringt.
    pub(super) open: bool,
    /// Ein ungedecktes Command-Schiff, vor das die Seite noch etwas legen kann ([`guardable`], [`Guard`]); wovor es
    /// kommt, entscheidet [`guard_target`].
    pub(super) guard: Option<Guard>,
}

/// Ein Schiff ohne Ausweg: Es steht, wo es steht.
pub(super) const SAFE: Risk = Risk {
    unsure: false,
    open: false,
    guard: None,
};

/// Wohin ein Schiff ausweichen kann ([`escape_route`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// Mindestens einer der Slots ist gedeckt.
    Cover,
    Open,
}

/// Wie die Schiffe von `side` den Schüssen der Gegenseite wohl entgehen, für deren Zielwahl ([`aim_fire`]). Kann die
/// Seite noch Deckung schaffen (`guard`), legt sie sie vor ein ungedecktes Command-Schiff ([`guard_target`]), und gibt
/// es keins, vorne mittig vor einen freien Command-Slot, hinter den eins ausweichen kann. Ausweichen kann ein Schiff in
/// einen freien Slot einer anderen Spalte ([`escape_route`]); angeschlagen tut es das wohl ([`stays`]), und wo dabei
/// Deckung ist, nimmt es die. Ein Strike-Schiff, das ein Schiff dahinter deckt, gilt nicht als `open`: Weicht es doch
/// aus, trifft Spaltenfeuer das Schiff dahinter.
pub(super) fn risks(board: &Board, side: Side, guard: Option<Guard>) -> HashMap<SlotRef, Risk> {
    let exposed = if guard.is_some() {
        guardable(board, side)
    } else {
        Vec::new()
    };
    let guarded = guard_target(board, &exposed, |_| 0);
    let shelter = free_slot_near_center(board, side, LaneType::Strike)
        .filter(|&center| guard.is_some() && guarded.is_none() && board.can_place(center, &[]))
        .map(|center| center.slot_index);
    board
        .ships(side)
        .map(|at| {
            let covering =
                at.lane == LaneType::Strike && board.is_occupied(at.in_lane(LaneType::Command));
            let open = escape_route(board, at, shelter) == Some(Route::Open) && !covering;
            let risk = Risk {
                unsure: Some(at) == guarded || (!stays(board, at) && !open),
                open,
                guard: guard.filter(|_| exposed.contains(&at)),
            };
            (at, risk)
        })
        .collect()
}

/// Wohin ein Schiff ausweichen kann, wie der Bot es täte (`Situation::evasions`): in einen freien Slot einer anderen
/// Spalte, das Flaggschiff nur in die Command-Lane, die Mirage auch per Phasentausch. [`Route::Cover`], wenn einer
/// davon gedeckt ist, auch hinter dem Strike-Slot `shelter`, in den die Seite ihr nächstes Schiff legt; `None` für
/// Tokens, die nie ausweichen, und wenn kein Slot frei ist.
fn escape_route(board: &Board, at: SlotRef, shelter: Option<usize>) -> Option<Route> {
    if is_token(board, at) {
        return None;
    }
    let flagship = is_flagship(board, at);
    let mut destinations: Vec<SlotRef> = board
        .maneuver_destinations(at, &[], None)
        .into_iter()
        .filter(|to| to.slot_index != at.slot_index && (!flagship || to.lane == LaneType::Command))
        .collect();
    if board.ship(at).card == ShipId::Mirage {
        destinations.extend(
            board
                .ships(at.side)
                .filter(|&to| to.slot_index != at.slot_index && !is_token(board, to)),
        );
    }
    if destinations.is_empty() {
        return None;
    }
    let covered = |to: &SlotRef| {
        to.lane == LaneType::Command
            && (board.is_occupied(to.in_lane(LaneType::Strike)) || Some(to.slot_index) == shelter)
    };
    Some(if destinations.iter().any(covered) {
        Route::Cover
    } else {
        Route::Open
    })
}

/// Die vermuteten Feuerbefehle von `side`, dem Gegner: dieselbe Zielwahl aus seiner Sicht ([`aim_fire`]), auf die
/// Stellung beim Planen samt Wächterschild. Die Schiffe, die der Bot in dieser Runde legt, stehen dort noch nicht; sie
/// fangen höchstens die ersten Schüsse auf ihre Spalte ab, und Schiffsfeuer auf das Schiff dahinter verpufft an ihnen.
/// `guard`: womit der Bot noch Deckung schaffen kann ([`Guard`]).
pub(super) fn predict_fire(board: &Board, side: Side, guard: Option<Guard>) -> Vec<FireOrder> {
    let mut view = board.clone();
    resolve_guardian_shields(&mut view, &mut Vec::new());
    let risk = risks(&view, side.opposite(), guard);
    aim_fire(&view, side, &|_| true, &|at| risk[&at], None)
        .into_iter()
        .map(|(shooter, target)| FireOrder {
            ship: shooter.target(),
            target: match target {
                Target::Column(column) => FireTarget::Column(column),
                Target::Ship(ship) => FireTarget::Ship(ship.target()),
            },
        })
        .collect()
}

/// Vorhalten: Wohin das Flaggschiff von `side` in Deckung ausweicht, wenn es das vermutete Feuer der Gegenseite
/// ([`predict_fire`]) fällt, wie der Bot es an seiner Stelle täte: in einen freien Command-Slot einer anderen Spalte
/// hinter einem Strike-Schiff, der es rettet, am ehesten dorthin, wo ihm nach dem Kampf am meisten bleibt. Gibt die
/// Spalten zurück; leer, wenn es nicht fällt oder kein solcher Slot es rettet. Ungedeckte Slots zählen nicht: Dort
/// rettet es nur, wenn der Gegner auf seine Spalte zielt, und das tut er, wenn es auch in Deckung ausweichen kann.
/// `guard`: womit `side` noch Deckung schaffen kann ([`Guard`]).
pub(super) fn flagship_escapes(board: &Board, side: Side, guard: Option<Guard>) -> Vec<usize> {
    let Some(flagship) = board.flagship(side) else {
        return Vec::new();
    };
    let fire = predict_fire(board, side.opposite(), guard);
    // Was dem Flaggschiff nach dem Kampf bleibt, wenn es in den Command-Slot `destination` ausweicht (`None`: bleibt
    // stehen).
    let remaining = |destination: Option<SlotTarget>| {
        let mut copy = board.clone();
        for &order in &fire {
            bind_fire(&mut copy, side.opposite(), order);
        }
        let to = destination.map(|destination| destination.on(side));
        if let Some(to) = to {
            resolve_maneuver(&mut copy, flagship, to, &mut Vec::new());
        }
        resolve_guardian_shields(&mut copy, &mut Vec::new());
        let railguns = firing_railguns(&copy);
        resolve_combat(&mut copy, &mut Vec::new(), &railguns);
        let ship = to.unwrap_or(flagship);
        if copy.ship(ship).hp > 0 {
            durability(&copy, ship)
        } else {
            0
        }
    };
    if remaining(None) > 0 {
        return Vec::new();
    }
    let mut escapes: Vec<(usize, i32)> = board
        .maneuver_destinations(flagship, &[], None)
        .into_iter()
        .filter(|to| {
            to.lane == LaneType::Command
                && to.slot_index != flagship.slot_index
                && board.is_occupied(to.in_lane(LaneType::Strike))
        })
        .map(|to| (to.slot_index, remaining(Some(to.target()))))
        .filter(|&(_, left)| left > 0)
        .collect();
    escapes.sort_by_key(|&(_, left)| std::cmp::Reverse(left));
    escapes.into_iter().map(|(column, _)| column).collect()
}
