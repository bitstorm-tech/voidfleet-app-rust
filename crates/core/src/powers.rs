//! Ziele der Admiralsfähigkeiten und der Markierung der Pendulum beim Planen: Der Client bietet damit die Zielwahl
//! an, der Server prüft den Zug mit denselben Regeln. Ziele gelten zur Stellung beim Planen; was in der Runde daraus
//! wird, regelt [`crate::resolution`].

use serde::{Deserialize, Serialize};

use crate::admirals::{PowerId, PowerTarget};
use crate::board::{Board, LaneType, Side, SlotRef, SlotTarget, lane_slots};
use crate::cards::ShipId;

/// Ein eigenes Schiff, das in dieser Runde landet (Deploy oder Flaggschiff), samt seinem Slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlannedArrival {
    pub card: ShipId,
    pub target: SlotTarget,
}

/// Gültige Ziele der Admiralsfähigkeit `power` für `side` beim Planen. `arrivals`: eigene Schiffe, die in dieser
/// Runde landen (Deploys samt Flaggschiff); sie zählen als eigene Schiffe bzw. belegen ihren Slot. `blocked`: eigene
/// Slots, in die sich in dieser Runde etwas bewegt (Ziel eines Bewegungsbefehls, mit Freier Hand auch des
/// Grundmanövers); dort kann kein Trugbild erscheinen.
pub fn power_targets(
    board: &Board,
    side: Side,
    power: PowerId,
    arrivals: &[PlannedArrival],
    blocked: &[SlotTarget],
) -> Vec<SlotRef> {
    let enemy = side.opposite();
    let landing = |arrival: &PlannedArrival| arrival.target.on(side);
    // Den Schwarmangriff fliegen die Drohnen, die beim Planen schon stehen.
    if power == PowerId::SwarmStrike
        && !board
            .ships(side)
            .any(|at| board.ship(at).card == ShipId::Drone)
    {
        return Vec::new();
    }
    match power.target() {
        PowerTarget::OwnShip => board
            .ships(side)
            .chain(arrivals.iter().map(landing))
            .collect(),
        PowerTarget::OwnCombatShip => board
            .ships(side)
            .filter(|&at| board.ship(at).card.atk() > 0)
            .chain(
                arrivals
                    .iter()
                    .filter(|arrival| arrival.card.atk() > 0)
                    .map(landing),
            )
            .collect(),
        PowerTarget::EnemyShip => board.ships(enemy).collect(),
        PowerTarget::EnemyShipNotFlagship => board
            .ships(enemy)
            .filter(|&at| !board.ship(at).card.is_flagship())
            .collect(),
        PowerTarget::FreeOwnStrikeSlot => {
            let taken = |at: SlotRef| {
                arrivals.iter().any(|arrival| arrival.target == at.target())
                    || blocked.contains(&at.target())
            };
            lane_slots(side, LaneType::Strike)
                .filter(|&at| !board.is_occupied(at) && !taken(at))
                .collect()
        }
    }
}

/// Ob die Railgun der Pendulum von `side` in dieser Runde feuert: Auf der Gegenseite steht ein Schiff, das sie in der
/// Vorrunde markiert hat.
pub fn railgun_fires(board: &Board, side: Side) -> bool {
    board.ships(side.opposite()).any(|at| board.ship(at).marked)
}

/// Ob die Pendulum der Seite in dieser Runde markieren darf (sie steht auf dem Brett und feuert in dieser Runde nicht).
pub fn can_mark(board: &Board, side: Side) -> bool {
    board
        .flagship(side)
        .is_some_and(|at| board.ship(at).card == ShipId::Pendulum)
        && !railgun_fires(board, side)
}

/// Gültige Ziele der Markierung (gegnerische Schiffe), leer, wenn [`can_mark`] false ist.
pub fn mark_targets(board: &Board, side: Side) -> Vec<SlotRef> {
    if can_mark(board, side) {
        board.ships(side.opposite()).collect()
    } else {
        Vec::new()
    }
}
