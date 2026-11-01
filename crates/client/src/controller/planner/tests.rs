//! Die Klick-Zustandsmaschine: was ein Klick je nach Auswahl bewirkt und welcher Klang dazu gehört.

use voidfleet_core::{
    admirals::AdmiralId,
    board::LaneType::{Command, Strike},
    cards::{CommandId as C, ShipId as S},
    protocol::OpponentView,
};

use super::*;

const fn own(lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(Side::Player, lane, index)
}

const fn enemy(lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(Side::Enemy, lane, index)
}

/// Runde 2, beide Admirale gewählt; auf dem Brett eine eigene Iron Dreadnought auf Strike 1 und ein gegnerischer
/// Void Interceptor auf Strike 2.
fn game(admiral: AdmiralId) -> Match {
    let mut game = Match::default();
    game.round = 2;
    game.admiral = Some(admiral);
    game.opponent = Some(OpponentView {
        admiral: Some(AdmiralId::Vance),
        submitted: false,
        ship_hand_size: 4,
        command_hand_size: 3,
        ship_deck_size: 12,
        command_deck_size: 31,
        bot: true,
        played_commands: Vec::new(),
    });
    game.ship_hand = vec![S::HorizonCruiser, S::PlasmaFrigate];
    game.command_hand = vec![C::Torpedo, C::FlankingManeuver, C::Scavenge];
    game.board.place(own(Strike, 1), S::IronDreadnought);
    game.board.place(enemy(Strike, 2), S::VoidInterceptor);
    game
}

/// Spielzustand, Auswahl und Hinweise eines Tests.
#[derive(Default)]
struct Desk {
    game: Match,
    selection: Selection,
    notices: Notices,
}

impl Desk {
    fn new(admiral: AdmiralId) -> Self {
        Self {
            game: game(admiral),
            ..default()
        }
    }

    fn planner(&mut self) -> Planner<'_> {
        Planner {
            game: &mut self.game,
            selection: &mut self.selection,
            notices: &mut self.notices,
            can_plan: true,
        }
    }
}

#[test]
fn ship_card_then_slot_plans_the_ship() {
    let mut desk = Desk::new(AdmiralId::Osei);
    let card = HandCard::Ship(ShipPick::Hand(0));
    assert_eq!(desk.planner().press_hand_card(card), UiSound::Select);
    assert_eq!(desk.selection, Selection::Ship(ShipPick::Hand(0)));
    // Belegter Slot: verweigert, die Auswahl bleibt.
    assert_eq!(
        desk.planner().press_slot(own(Strike, 1)),
        Some(UiSound::Denied)
    );
    assert_eq!(
        desk.planner().press_slot(own(Strike, 3)),
        Some(UiSound::Place)
    );
    assert_eq!(desk.selection, Selection::Idle);
    assert_eq!(
        desk.game.planned_pick_at(own(Strike, 3)),
        Some(ShipPick::Hand(0))
    );
    // Die Karte eines eingeplanten Schiffs nimmt es zurück.
    assert_eq!(desk.planner().press_hand_card(card), UiSound::Cancel);
    assert!(desk.game.arrivals().is_empty());
}

#[test]
fn second_click_on_a_card_cancels_its_selection() {
    let mut desk = Desk::new(AdmiralId::Osei);
    let card = HandCard::Command(0);
    assert_eq!(desk.planner().press_hand_card(card), UiSound::Select);
    assert_eq!(desk.planner().press_hand_card(card), UiSound::Cancel);
    assert_eq!(desk.selection, Selection::Idle);
}

#[test]
fn nothing_goes_while_planning_is_closed() {
    let mut desk = Desk::new(AdmiralId::Osei);
    let mut planner = desk.planner();
    planner.can_plan = false;
    assert_eq!(
        planner.press_hand_card(HandCard::Ship(ShipPick::Hand(0))),
        UiSound::Denied
    );
    assert_eq!(planner.press_slot(own(Strike, 1)), None);
    assert_eq!(planner.escape(), Escape::OpenSettings);
    assert_eq!(desk.selection, Selection::Idle);
}

#[test]
fn command_without_target_is_planned_with_one_click() {
    let mut desk = Desk::new(AdmiralId::Osei);
    assert_eq!(
        desk.planner().press_hand_card(HandCard::Command(2)),
        UiSound::Place
    );
    assert_eq!(desk.game.planned_command(), Some(C::Scavenge));
    // Ein zweiter Klick auf die eingeplante Karte nimmt den Befehl zurück.
    assert_eq!(
        desk.planner().press_hand_card(HandCard::Command(2)),
        UiSound::Cancel
    );
    assert_eq!(desk.game.planned_command(), None);
}

#[test]
fn torpedo_aims_at_any_enemy_slot() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner().press_hand_card(HandCard::Command(0));
    let lights = highlights(&desk.game, desk.selection);
    assert!(lights.slots.contains(&enemy(Command, 5)));
    assert!(!lights.slots.contains(&own(Strike, 1)));
    assert!(!lights.grid);
    assert_eq!(
        desk.planner().press_slot(own(Strike, 1)),
        Some(UiSound::Denied)
    );
    assert_eq!(
        desk.planner().press_slot(enemy(Command, 5)),
        Some(UiSound::Place)
    );
    assert_eq!(
        desk.game.planned_enemy_target(),
        Some(SlotTarget::new(Command, 5))
    );
}

#[test]
fn lane_change_takes_ship_then_destination() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner().press_hand_card(HandCard::Command(1));
    assert_eq!(
        desk.planner().press_slot(own(Strike, 1)),
        Some(UiSound::Select)
    );
    assert_eq!(
        desk.selection,
        Selection::Command {
            index: 1,
            move_source: Some(own(Strike, 1))
        }
    );
    assert!(highlights(&desk.game, desk.selection).grid);
    // Unter dem Zeiger zeigt die Szene den Lane-Wechsel schon.
    let hover = BoardHover {
        slot: Some(own(Command, 2)),
        column: None,
    };
    let shown = hypothetical(&desk.game, desk.selection, hover).expect("gültiger Ziel-Slot");
    assert_eq!(
        shown.planned_command_move().map(|planned| planned.to),
        Some(own(Command, 2))
    );
    assert_eq!(desk.game.planned_command(), None);
    assert_eq!(
        desk.planner().press_slot(own(Command, 2)),
        Some(UiSound::Place)
    );
    assert_eq!(
        desk.game.planned_command_move().map(|planned| planned.to),
        Some(own(Command, 2))
    );
    // Der Geist im Ziel-Slot steht für das verlegte Schiff.
    assert_eq!(unit_at(&desk.game, own(Command, 2)), Some(own(Strike, 1)));
}

#[test]
fn unit_toggles_fire_orders_and_stays_selected() {
    let mut desk = Desk::new(AdmiralId::Osei);
    let unit = own(Strike, 1);
    assert_eq!(desk.planner().press_slot(unit), Some(UiSound::Select));
    assert_eq!(shooter(&desk.game, desk.selection), Some(unit));
    assert_eq!(desk.planner().press_column(4), Some(UiSound::Place));
    assert_eq!(
        desk.game
            .fire_order_at(unit.target())
            .map(|order| order.target),
        Some(FireTarget::Column(4))
    );
    // Schiffsfeuer ersetzt das Spaltenfeuer, ein zweiter Klick aufs selbe Ziel hebt es auf.
    let target = enemy(Strike, 2);
    assert_eq!(desk.planner().press_slot(target), Some(UiSound::Place));
    assert_eq!(
        desk.game
            .fire_order_at(unit.target())
            .map(|order| order.target),
        Some(FireTarget::Ship(target.target()))
    );
    assert_eq!(desk.planner().press_slot(target), Some(UiSound::Cancel));
    assert_eq!(desk.game.fire_order_at(unit.target()), None);
    assert_eq!(desk.selection, Selection::Unit(unit));
    // Ein Klick daneben beendet die Auswahl.
    assert_eq!(desk.planner().miss(), Some(UiSound::Cancel));
    assert_eq!(desk.selection, Selection::Idle);
}

#[test]
fn maneuver_drops_the_fire_order_and_second_click_takes_it_back() {
    let mut desk = Desk::new(AdmiralId::Osei);
    let unit = own(Strike, 1);
    desk.planner().press_slot(unit);
    desk.planner().press_column(1);
    assert_eq!(
        desk.planner().press_slot(own(Command, 3)),
        Some(UiSound::Place)
    );
    assert_eq!(
        desk.game.planned_maneuver().map(|planned| planned.to),
        Some(own(Command, 3))
    );
    assert_eq!(desk.game.fire_order_at(unit.target()), None);
    assert_eq!(
        desk.notices.0,
        vec![Notice::FireCleared(S::IronDreadnought)]
    );
    // Wieder gewählt über seinen Geist; ein zweiter Klick nimmt das Manöver zurück.
    assert_eq!(
        desk.planner().press_slot(own(Command, 3)),
        Some(UiSound::Select)
    );
    assert!(desk.notices.0.is_empty());
    assert_eq!(desk.selection, Selection::Unit(unit));
    assert_eq!(desk.planner().press_slot(unit), Some(UiSound::Cancel));
    assert_eq!(desk.game.planned_maneuver(), None);
}

#[test]
fn escape_cancels_selection_then_maneuver_then_opens_settings() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner().press_slot(own(Strike, 1));
    desk.planner().press_slot(own(Command, 3));
    desk.planner()
        .press_hand_card(HandCard::Ship(ShipPick::Hand(1)));
    assert_eq!(desk.planner().escape(), Escape::TookBack);
    assert_eq!(desk.selection, Selection::Idle);
    assert!(desk.game.planned_maneuver().is_some());
    assert_eq!(desk.planner().escape(), Escape::TookBack);
    assert_eq!(desk.game.planned_maneuver(), None);
    assert_eq!(desk.planner().escape(), Escape::OpenSettings);
}

#[test]
fn power_without_target_is_denied_with_a_notice() {
    // Halvorsens Schwarmangriff braucht eine eigene Drohne.
    let mut desk = Desk::new(AdmiralId::Halvorsen);
    assert_eq!(desk.planner().press_power(), UiSound::Denied);
    assert_eq!(
        desk.notices.0,
        vec![Notice::NoPowerTarget(PowerId::SwarmStrike)]
    );
    assert_eq!(desk.selection, Selection::Idle);
}

#[test]
fn power_replaces_the_maneuver() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner().press_slot(own(Strike, 1));
    desk.planner().press_slot(own(Command, 3));
    assert_eq!(desk.planner().press_power(), UiSound::Select);
    let target = *desk
        .game
        .power_targets()
        .first()
        .expect("Osei hat ein Ziel");
    assert_eq!(desk.planner().press_slot(target), Some(UiSound::Place));
    assert_eq!(desk.game.planned_maneuver(), None);
    assert_eq!(
        desk.notices.0,
        vec![Notice::PowerReplacesManeuver {
            power: AdmiralId::Osei.power(),
            ship: S::IronDreadnought,
        }]
    );
    // Ein weiterer Klick auf die Fähigkeitskarte nimmt sie zurück.
    assert_eq!(desk.planner().press_power(), UiSound::Cancel);
    assert!(desk.game.plan().power.is_none());
}

#[test]
fn unplanning_a_ship_ends_its_selection() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner()
        .press_hand_card(HandCard::Ship(ShipPick::Hand(1)));
    desk.planner().press_slot(own(Strike, 4));
    // Das eingeplante Schiff ist wählbar wie ein echtes; der zweite Klick nimmt es zurück.
    assert_eq!(
        desk.planner().press_slot(own(Strike, 4)),
        Some(UiSound::Select)
    );
    assert_eq!(
        desk.planner().press_slot(own(Strike, 4)),
        Some(UiSound::Cancel)
    );
    assert!(desk.game.arrivals().is_empty());
    assert_eq!(desk.selection, Selection::Idle);
}

#[test]
fn hover_shows_the_ship_on_the_slot() {
    let mut desk = Desk::new(AdmiralId::Osei);
    desk.planner()
        .press_hand_card(HandCard::Ship(ShipPick::Hand(0)));
    let over = |slot| BoardHover {
        slot: Some(slot),
        column: None,
    };
    let shown =
        hypothetical(&desk.game, desk.selection, over(own(Command, 0))).expect("freier Slot");
    assert_eq!(
        shown.planned_pick_at(own(Command, 0)),
        Some(ShipPick::Hand(0))
    );
    // Über einem ungültigen Slot bleibt der eingeplante Zug stehen.
    assert!(hypothetical(&desk.game, desk.selection, over(own(Strike, 1))).is_none());
}
