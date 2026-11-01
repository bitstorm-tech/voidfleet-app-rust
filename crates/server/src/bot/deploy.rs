//! Die Schiffe der Runde: Kampfschiffe zuerst, ein Wächter vor das ungedeckte Flaggschiff oder ein markiertes Schiff,
//! robuste vor das ungedeckte Command-Schiff, auf das der Gegner wohl am meisten feuert, sonst nach vorne, Jäger und
//! Support in einen gedeckten Command-Slot.

use voidfleet_core::board::{Board, LaneType, Side, SlotRef, lane_slots};
use voidfleet_core::cards::ShipId;
use voidfleet_core::protocol::{Deploy, FireOrder, FireTarget};

use super::{
    BotView, free_slot_near_center, highest, is_flagship, is_fragile, near_center, ship_value,
};

/// Legt die Schiffe nacheinander auf `board`, die Kopie des Bretts: jedes mit Rücksicht auf die vorigen. `enemy_fire`:
/// die vermuteten Feuerbefehle des Gegners ([`super::threat::predict_fire`]).
pub(super) fn plan_ships(
    board: &mut Board,
    view: &BotView,
    count: usize,
    enemy_fire: &[FireOrder],
) -> Vec<Deploy> {
    let mut hand: Vec<(usize, ShipId)> = view.ship_hand.iter().copied().enumerate().collect();
    let mut ships = Vec::new();
    for _ in 0..count {
        let cards: Vec<ShipId> = hand.iter().map(|&(_, card)| card).collect();
        let Some((index, at)) = plan_ship(board, view.side, &cards, enemy_fire) else {
            break;
        };
        let (hand_index, card) = hand.remove(index);
        board.place(at, card);
        ships.push(Deploy {
            hand_index,
            target: at.target(),
        });
    }
    ships
}

/// Die ungedeckten Command-Schiffe von `side`, vor die ein Schiff gelegt werden kann.
pub(super) fn guardable(board: &Board, side: Side) -> Vec<SlotRef> {
    board
        .occupied(side, LaneType::Command)
        .filter(|at| board.can_place(at.in_lane(LaneType::Strike), &[]))
        .collect()
}

/// Vor welches der ungedeckten Command-Schiffe `exposed` ([`guardable`]) ein robustes Schiff kommt: das Flaggschiff,
/// sonst ein markiertes (die gegnerische Railgun zielt darauf), sonst das, auf das am meisten Feuer zielt (`fire`),
/// sonst das nächst der Mitte. So erwartet der Bot es auch vom Gegner.
pub(super) fn guard_target(
    board: &Board,
    exposed: &[SlotRef],
    fire: impl Fn(SlotRef) -> i32,
) -> Option<SlotRef> {
    let exposed = exposed.iter().copied();
    exposed
        .clone()
        .find(|&at| is_flagship(board, at))
        .or_else(|| exposed.clone().find(|&at| board.ship(at).marked))
        .or_else(|| highest(exposed.clone().filter(|&at| fire(at) > 0), |&at| fire(at)))
        .or_else(|| near_center(exposed))
}

/// Das nächste Schiff von der Hand `ship_hand` und sein Slot; `None`, wenn keins passt. Ein Strike-Schiff deckt den
/// Command-Slot dahinter: Schiffsfeuer darauf verpufft, und es nimmt die ersten Schüsse auf die Spalte.
fn plan_ship(
    board: &Board,
    side: Side,
    ship_hand: &[ShipId],
    enemy_fire: &[FireOrder],
) -> Option<(usize, SlotRef)> {
    let card = highest(0..ship_hand.len(), |&index| ship_value(ship_hand[index]))?;
    // Vor das ungedeckte Flaggschiff und ein markiertes Schiff stellt sich das robusteste Kampfschiff; sonst kommen
    // robuste Schiffe vor das ungedeckte Command-Schiff, auf das der Gegner wohl am meisten feuert, sonst nach vorne,
    // Jäger und Support in einen gedeckten Command-Slot.
    let guarded = guard_target(board, &guardable(board, side), |at| {
        fire_on(board, side, enemy_fire, at)
    });
    if let Some(guarded) = guarded
        && (is_flagship(board, guarded) || board.ship(guarded).marked)
    {
        let robust = (0..ship_hand.len()).filter(|&index| ship_hand[index].atk() > 0);
        let guard = highest(robust, |&index| {
            let stats = ship_hand[index].card();
            stats.hp + stats.shield
        })
        .unwrap_or(card);
        return Some((guard, guarded.in_lane(LaneType::Strike)));
    }
    let preferred = if is_fragile(ship_hand[card]) {
        near_center(lane_slots(side, LaneType::Command).filter(|&at| {
            !board.is_occupied(at) && board.is_occupied(at.in_lane(LaneType::Strike))
        }))
    } else {
        guarded.map(|at| at.in_lane(LaneType::Strike))
    };
    [
        preferred,
        free_slot_near_center(board, side, LaneType::Strike),
        free_slot_near_center(board, side, LaneType::Command),
    ]
    .into_iter()
    .flatten()
    .find(|&at| board.can_place(at, &[]))
    .map(|at| (card, at))
}

/// Wie viel Schaden der Gegner mit seinen vermuteten Feuerbefehlen `enemy_fire` auf das ungedeckte Schiff `at` von
/// `side` lenkt.
fn fire_on(board: &Board, side: Side, enemy_fire: &[FireOrder], at: SlotRef) -> i32 {
    enemy_fire
        .iter()
        .filter(|order| match order.target {
            FireTarget::Ship(target) => target == at.target(),
            FireTarget::Column(column) => column == at.slot_index,
        })
        .map(|order| board.attack(order.ship.on(side.opposite())))
        .sum()
}
