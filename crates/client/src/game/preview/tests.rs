//! Die Zielvorschau auf kleinen Brettern.

use voidfleet_core::{
    admirals::AdmiralId,
    board::{
        LaneType::{Command, Strike},
        Side::{Enemy, Player},
    },
    cards::ShipId as S,
    protocol::{Deploy, FireOrder},
};

use super::*;
use crate::game::ShipPick;

fn at(side: Side, lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(side, lane, index)
}

/// Eigene Schützen in Strike 0 und 1 und Command 0, beim Gegner eine Drohne, ein Kreuzer und dahinter gedeckt ein
/// Abfangjäger.
fn game() -> Match {
    let mut game = Match::default();
    game.round = 2;
    game.admiral = Some(AdmiralId::Osei);
    for (at, ship) in [
        (at(Player, Strike, 0), S::VoidInterceptor),
        (at(Player, Strike, 1), S::HorizonCruiser),
        (at(Player, Command, 0), S::PlasmaFrigate),
        (at(Enemy, Strike, 2), S::Drone),
        (at(Enemy, Strike, 3), S::HorizonCruiser),
        (at(Enemy, Command, 3), S::VoidInterceptor),
    ] {
        game.board.place(at, ship).expect("Slot ist frei");
    }
    game
}

fn aim(game: &mut Match, ship: SlotTarget, target: FireTarget) {
    game.set_fire(ship, Some(FireOrder { ship, target }));
}

#[test]
fn shots_hit_fizzle_and_destroy() {
    let mut game = game();
    aim(
        &mut game,
        SlotTarget::new(Strike, 0),
        FireTarget::Ship(SlotTarget::new(Strike, 2)),
    );
    aim(&mut game, SlotTarget::new(Strike, 1), FireTarget::Column(5));
    aim(
        &mut game,
        SlotTarget::new(Command, 0),
        FireTarget::Ship(SlotTarget::new(Command, 3)),
    );
    let preview = preview_round(&game, game.plan());

    let shot = |shooter: SlotRef| {
        *preview
            .shots
            .iter()
            .find(|shot| shot.shooter == Some(shooter))
            .expect("Schuss des Schützen")
    };
    let drone = shot(at(Player, Strike, 0));
    assert!(drone.tracking);
    assert_eq!(drone.target, at(Enemy, Strike, 2));
    assert!(
        matches!(drone.outcome, ShotOutcome::Hit { ship, through: false } if ship.card_id == S::Drone)
    );

    let empty = shot(at(Player, Strike, 1));
    assert!(!empty.tracking);
    assert_eq!(empty.target, at(Enemy, Strike, 5));
    assert_eq!(
        empty.outcome,
        ShotOutcome::Miss(Miss::Column {
            column: 5,
            cleared: false
        })
    );

    let covered = shot(at(Player, Command, 0));
    assert!(covered.tracking);
    assert_eq!(covered.target, at(Enemy, Command, 3));
    assert!(matches!(
        covered.outcome,
        ShotOutcome::Miss(Miss::Target { covered: true, .. })
    ));

    assert_eq!(preview.losses, [at(Enemy, Strike, 2)]);
    assert_eq!(preview.outcomes[&at(Enemy, Strike, 2)], None);
    assert_eq!(
        preview.outcomes[&at(Enemy, Strike, 3)],
        Some(ShipStats { hp: 8, shield: 0 })
    );
    // Der Gegner tut in der Vorschau nichts.
    assert_eq!(
        preview.outcomes[&at(Player, Strike, 0)],
        Some(ShipStats { hp: 5, shield: 0 })
    );
}

#[test]
fn column_fire_passes_a_fallen_front_ship() {
    let mut game = game();
    game.board
        .place(at(Enemy, Command, 2), S::HorizonCruiser)
        .expect("Slot ist frei");
    // Der erste Schuss räumt die Drohne vorn in Spalte 2, der zweite geht auf das Schiff dahinter durch.
    aim(&mut game, SlotTarget::new(Strike, 0), FireTarget::Column(2));
    aim(&mut game, SlotTarget::new(Strike, 1), FireTarget::Column(2));
    let preview = preview_round(&game, game.plan());
    let [front, behind] = preview.shots[..] else {
        panic!("genau zwei Schüsse: {:?}", preview.shots);
    };
    assert_eq!(front.target, at(Enemy, Strike, 2));
    assert!(matches!(
        front.outcome,
        ShotOutcome::Hit { through: false, .. }
    ));
    assert_eq!(behind.target, at(Enemy, Command, 2));
    assert!(matches!(
        behind.outcome,
        ShotOutcome::Hit { through: true, .. }
    ));
    assert_eq!(
        preview.outcomes[&at(Enemy, Command, 2)],
        Some(ShipStats { hp: 6, shield: 0 })
    );
}

#[test]
fn planned_titan_carrier_launches_a_drone() {
    let mut game = game();
    game.ship_hand = vec![S::TitanCarrier];
    game.plan_ship(ShipPick::Hand(0), SlotTarget::new(Strike, 4));
    assert_eq!(
        game.plan().ships,
        [Deploy {
            hand_index: 0,
            target: SlotTarget::new(Strike, 4)
        }]
    );
    let preview = preview_round(&game, game.plan());
    let [drone] = preview.drones[..] else {
        panic!("genau eine Drohne: {:?}", preview.drones);
    };
    assert_eq!((drone.side, drone.lane), (Player, Strike));
    assert!(preview.shots.is_empty());
}

#[test]
fn preview_of_a_tried_plan() {
    let game = game();
    let ship = SlotTarget::new(Strike, 1);
    let tried = game.try_plan(|game| aim(game, ship, FireTarget::Column(3)));
    assert!(preview_round(&game, game.plan()).shots.is_empty());
    let preview = preview_round(&game, &tried);
    assert_eq!(preview.shots.len(), 1);
    assert_eq!(
        preview.outcomes[&at(Enemy, Strike, 3)],
        Some(ShipStats { hp: 6, shield: 0 })
    );
}
