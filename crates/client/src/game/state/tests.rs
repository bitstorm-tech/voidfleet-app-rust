//! Die wichtigsten Planungsregeln.

use voidfleet_core::{
    board::{
        LaneType::{Command, Strike},
        Sides,
    },
    cards::{CommandId as C, ShipId as S},
    protocol::OpponentView,
};

use super::*;

const fn strike(index: usize) -> SlotTarget {
    SlotTarget::new(Strike, index)
}

const fn command(index: usize) -> SlotTarget {
    SlotTarget::new(Command, index)
}

/// Ein Match in `round`, beide Admirale gewählt, leeres Brett und leere Hände.
fn game(round: u32, admiral: AdmiralId) -> Match {
    Match {
        round,
        admiral: Some(admiral),
        opponent: Some(OpponentView {
            admiral: Some(AdmiralId::Vance),
            submitted: false,
            ship_hand_size: 4,
            command_hand_size: 3,
            ship_deck_size: 12,
            command_deck_size: 31,
            bot: true,
            played_commands: Vec::new(),
        }),
        ..default()
    }
}

fn place(game: &mut Match, at: SlotRef, ship: S) {
    game.board.place(at, ship).expect("Slot ist frei");
}

fn play(index: usize, target: Option<SlotTarget>) -> Option<CommandPlan> {
    Some(CommandPlan {
        hand_index: index,
        target,
        destination: None,
    })
}

#[test]
fn round_one_deploys_two_ships_plus_flagship() {
    let mut game = game(1, AdmiralId::Osei);
    game.ship_hand = vec![S::HorizonCruiser, S::VoidInterceptor, S::PlasmaFrigate];
    assert!(game.admirals_chosen());
    assert!(game.flagship_missing());
    assert_eq!(game.deploy_limit(), 2);

    game.plan_ship(ShipPick::Flagship, command(2));
    game.plan_ship(ShipPick::Hand(0), strike(0));
    game.plan_ship(ShipPick::Hand(1), strike(1));
    assert!(!game.flagship_missing());
    assert_eq!(game.deploys_left(), 0);
    assert_eq!(game.ship_at(own(command(2))), Some(S::Bastion));

    // Ein drittes Schiff verdrängt das zuerst geplante, nicht das Flaggschiff.
    game.plan_ship(ShipPick::Hand(2), strike(3));
    assert_eq!(game.planned_ships(), [S::VoidInterceptor, S::PlasmaFrigate]);
    assert_eq!(game.plan().flagship, Some(command(2)));
    // Umplanen eines schon geplanten Schiffs verdrängt keins.
    game.plan_ship(ShipPick::Hand(1), strike(4));
    assert_eq!(game.planned_ships(), [S::PlasmaFrigate, S::VoidInterceptor]);
    assert_eq!(game.planned_pick_at(own(strike(1))), None);
    // Hand 0 fände wieder Platz, indem das zuerst geplante weicht; der Slot des Flaggschiffs bleibt besetzt.
    assert!(game.can_deploy(own(strike(0)), ShipPick::Hand(0)));
    assert!(!game.can_deploy(own(command(2)), ShipPick::Hand(0)));
}

#[test]
fn rapid_deployment_drops_the_last_surplus_ship_but_not_the_target() {
    let mut game = game(2, AdmiralId::Osei);
    game.ship_hand = vec![S::HorizonCruiser, S::VoidInterceptor];
    game.command_hand = vec![C::RapidDeployment, C::EmergencyShields];
    game.set_command(play(0, None));
    assert_eq!(game.deploy_limit(), 2);
    game.plan_ship(ShipPick::Hand(0), strike(0));
    game.plan_ship(ShipPick::Hand(1), strike(1));

    let change = game.set_command(play(1, Some(strike(1))));
    assert_eq!(change.ship, Some(S::HorizonCruiser));
    assert_eq!(game.planned_ships(), [S::VoidInterceptor]);
    assert_eq!(game.planned_own_target(), Some(strike(1)));
}

#[test]
fn command_on_a_planned_ship_falls_with_it() {
    let mut game = game(2, AdmiralId::Osei);
    game.ship_hand = vec![S::HorizonCruiser];
    game.command_hand = vec![C::EmergencyShields, C::Torpedo];
    game.plan_ship(ShipPick::Hand(0), strike(2));
    assert!(game.can_target_own(C::EmergencyShields, own(strike(2))));
    assert!(!game.can_target_own(C::EmergencyShields, own(strike(3))));
    game.set_command(play(0, Some(strike(2))));
    game.unplan_ship(ShipPick::Hand(0));
    assert_eq!(game.plan().command, None);

    // Ein Ziel auf der Gegnerseite ist kein eigenes.
    game.set_command(play(1, Some(strike(4))));
    assert_eq!(game.planned_enemy_target(), Some(strike(4)));
    assert_eq!(game.planned_own_target(), None);
}

#[test]
fn maneuvering_ship_loses_its_fire_order() {
    let mut game = game(2, AdmiralId::Osei);
    place(&mut game, own(strike(0)), S::HorizonCruiser);
    place(&mut game, own(strike(1)), S::VoidInterceptor);
    place(&mut game, own(command(1)), S::RepairDrone);
    assert_eq!(game.shooters(), [strike(0), strike(1)]);

    let order = game.next_fire(strike(0), FireTarget::Column(3));
    game.set_fire(strike(0), order);
    let progress = game.turn_progress();
    assert_eq!(
        progress.fire_orders,
        FireOrderCount {
            assigned: 1,
            shooters: 2
        }
    );
    assert_eq!(progress.fire, TurnPartState::Open);
    // Derselbe Tipp ein zweites Mal nimmt den Feuerbefehl zurück, ein anderes Ziel ersetzt ihn.
    assert_eq!(game.next_fire(strike(0), FireTarget::Column(3)), None);
    assert!(
        game.next_fire(strike(0), FireTarget::Ship(strike(3)))
            .is_some()
    );

    let change = game.set_maneuver(Some(Maneuver {
        ship: strike(0),
        destination: command(0),
    }));
    assert_eq!(change.fire, Some(S::HorizonCruiser));
    assert_eq!(game.fire_order_at(strike(0)), None);
    assert!(!game.can_fire(own(strike(0))));
    assert_eq!(game.shooters(), [strike(1)]);
}

#[test]
fn phase_shift_keeps_both_firing() {
    let mut game = game(2, AdmiralId::Moreau);
    place(&mut game, own(strike(0)), S::Mirage);
    place(&mut game, own(strike(1)), S::HorizonCruiser);
    assert!(
        game.maneuver_destinations(own(strike(0)))
            .contains(&own(strike(1)))
    );
    assert!(
        !game
            .maneuver_destinations(own(strike(1)))
            .contains(&own(strike(0)))
    );
    for ship in [strike(0), strike(1)] {
        let order = game.next_fire(ship, FireTarget::Column(2));
        game.set_fire(ship, order);
    }
    let change = game.set_maneuver(Some(Maneuver {
        ship: strike(0),
        destination: strike(1),
    }));
    assert!(game.planned_swap().is_some());
    assert_eq!(change.fire, None);
    assert_eq!(game.plan().fire.len(), 2);
}

#[test]
fn power_and_maneuver_replace_each_other_unless_carte_blanche() {
    let mut game = game(2, AdmiralId::Osei);
    game.command_hand = vec![C::CarteBlanche];
    place(&mut game, own(strike(0)), S::HorizonCruiser);
    place(&mut game, own(strike(1)), S::VoidInterceptor);
    let maneuver = Maneuver {
        ship: strike(0),
        destination: command(0),
    };
    let counterfire = PowerPlan { target: strike(1) };

    game.set_maneuver(Some(maneuver));
    assert_eq!(game.set_power(Some(counterfire)), Some(S::HorizonCruiser));
    assert_eq!(game.plan().maneuver, None);
    assert!(game.set_maneuver(Some(maneuver)).power);
    assert_eq!(game.plan().power, None);

    game.set_command(play(0, None));
    assert!(game.keeps_maneuver_and_power());
    assert_eq!(game.set_power(Some(counterfire)), None);
    assert_eq!(game.plan().maneuver, Some(maneuver));
    assert!(game.turn_progress().power.is_some());

    // Ohne Freie Hand weicht die Fähigkeit.
    assert!(game.set_command(None).power);
    assert_eq!(game.plan().maneuver, Some(maneuver));
}

#[test]
fn collision_course_holds_fire() {
    let mut game = game(2, AdmiralId::Osei);
    game.command_hand = vec![C::CollisionCourse];
    place(&mut game, own(strike(0)), S::HorizonCruiser);
    let order = game.next_fire(strike(0), FireTarget::Column(0));
    game.set_fire(strike(0), order);
    let change = game.set_command(play(0, Some(strike(0))));
    assert_eq!(change.fire, Some(S::HorizonCruiser));
    assert!(game.shooters().is_empty());
    assert_eq!(game.turn_progress().fire, TurnPartState::Blocked);
}

#[test]
fn swarm_drones_count_as_armed() {
    let mut game = game(2, AdmiralId::Halvorsen);
    place(&mut game, own(strike(0)), S::Drone);
    place(
        &mut game,
        SlotRef::new(Side::Enemy, Strike, 2),
        S::HorizonCruiser,
    );
    assert!(!game.is_armed(strike(0)));
    assert_eq!(game.power_targets(), [SlotRef::new(Side::Enemy, Strike, 2)]);
    game.set_power(Some(PowerPlan { target: strike(2) }));
    assert!(game.flies_swarm(strike(0)));
    assert_eq!(game.turn_progress().fire, TurnPartState::Done);
}

#[test]
fn pendulum_marks_and_warns_when_moving() {
    let mut game = game(2, AdmiralId::Kerr);
    place(&mut game, own(command(0)), S::Pendulum);
    place(
        &mut game,
        SlotRef::new(Side::Enemy, Strike, 4),
        S::VoidInterceptor,
    );
    assert!(game.can_mark());
    assert_eq!(game.turn_progress().mark, Some(TurnPartState::Open));
    game.set_mark(Some(strike(4)));
    assert_eq!(game.turn_progress().mark, Some(TurnPartState::Done));
    assert!(!game.pendulum_moves());
    game.set_maneuver(Some(Maneuver {
        ship: command(0),
        destination: command(3),
    }));
    assert!(game.pendulum_moves());
}

#[test]
fn try_plan_leaves_the_plan_alone() {
    let mut game = game(2, AdmiralId::Osei);
    place(&mut game, own(strike(0)), S::HorizonCruiser);
    let tried = game.try_plan(|game| {
        game.set_fire(strike(0), game.next_fire(strike(0), FireTarget::Column(1)))
    });
    assert_eq!(tried.fire.len(), 1);
    assert!(game.plan().fire.is_empty());
}

fn view(round: u32, game: &Match) -> PlayerView {
    PlayerView {
        match_id: "abcd".into(),
        round,
        admiral: game.admiral,
        board: game.board.snapshot(),
        hive_launches: Sides::default(),
        ship_hand: game.ship_hand.clone(),
        command_hand: game.command_hand.clone(),
        ship_deck_size: 10,
        command_deck_size: 20,
        submitted_plan: None,
        draft: None,
        opponent: game.opponent.clone(),
        result: None,
        surrendered: false,
    }
}

#[test]
fn apply_keeps_the_plan_within_the_round() {
    let mut game = game(2, AdmiralId::Osei);
    place(&mut game, own(strike(0)), S::HorizonCruiser);
    game.set_fire(strike(0), game.next_fire(strike(0), FireTarget::Column(1)));

    game.apply(view(2, &game.clone()));
    assert_eq!(game.plan().fire.len(), 1);
    assert_eq!(game.ship_deck_size, 10);
    game.apply(view(8, &game.clone()));
    assert!(game.plan().fire.is_empty());
    assert_eq!(game.board.overdrive, 1);

    let mut submitted = view(8, &game);
    let plan = TurnPlan {
        mark: Some(strike(2)),
        ..default()
    };
    submitted.submitted_plan = Some(plan.clone());
    game.apply(submitted);
    assert!(game.submitted);
    assert_eq!(game.to_turn_plan(), plan);
    game.end_round();
    assert_eq!(game.plan(), &TurnPlan::default());
}
