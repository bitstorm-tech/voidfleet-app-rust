//! Die Verbindung gegen den eingebetteten Server: Match gegen den Computer anlegen, Zustand empfangen, Admiral wählen,
//! einen Zug planen und abgeben.

use std::time::Duration;

use voidfleet_client::{
    game::{Match, ShipPick, preview_round},
    net::{Connection, Endpoint, Incoming},
};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side, SlotTarget},
    protocol::{ClientMessage, DraftPick, FireTarget, PlayerView, ServerMessage},
};

const TIMEOUT: Duration = Duration::from_secs(5);

/// Die nächste Sicht des Servers; Rundenereignisse dazwischen zählen mit.
fn next_view(connection: &Connection, rounds: &mut u32) -> PlayerView {
    loop {
        match connection.receive_timeout(TIMEOUT) {
            Some(Incoming::Message(ServerMessage::State { view })) => return *view,
            Some(Incoming::Message(ServerMessage::Round { .. })) => *rounds += 1,
            other => panic!("statt eines Zustands: {other:?}"),
        }
    }
}

#[test]
fn plays_a_round_against_the_bot() {
    let address = voidfleet_server::server::spawn_local().expect("Server startet");
    let endpoint = Endpoint::new(format!("ws://{address}"), None, true);
    let connection = Connection::open(endpoint.url());
    let mut rounds = 0;

    let view = next_view(&connection, &mut rounds);
    assert_eq!(view.match_id, endpoint.match_id);
    assert!(view.opponent.as_ref().is_some_and(|opponent| opponent.bot));

    connection.send(ClientMessage::ChooseAdmiral {
        admiral: AdmiralId::Vance,
    });
    let mut game = Match::default();
    while !game.admirals_chosen() || game.round != 1 {
        game.apply(next_view(&connection, &mut rounds));
    }

    // Runde 1: Flaggschiff und zwei Schiffe, alle Schützen feuern blind in Spalte 2.
    game.plan_ship(ShipPick::Flagship, SlotTarget::new(LaneType::Command, 2));
    for (index, slot) in [(0, 1), (1, 3)] {
        game.plan_ship(
            ShipPick::Hand(index),
            SlotTarget::new(LaneType::Strike, slot),
        );
    }
    for ship in game.shooters() {
        let order = game.next_fire(ship, FireTarget::Column(2));
        game.set_fire(ship, order);
    }
    assert!(!game.flagship_missing());
    assert_eq!(
        preview_round(&game, game.plan()).shots.len(),
        game.shooters().len()
    );
    connection.send(ClientMessage::Submit {
        plan: game.to_turn_plan(),
    });

    // Nach der Runde kommt der Nachschub; gewählt wird je die erste Karte.
    while game.round == 1 || game.submitted {
        game.apply(next_view(&connection, &mut rounds));
    }
    assert_eq!(rounds, 1);
    assert_eq!(game.round, 2);
    // Das Flaggschiff steht (daneben je nach Hand auch Drohnen, mancher Deploy ist vielleicht schon gefallen).
    assert_eq!(
        game.board
            .flagship(Side::Player)
            .map(|at| game.board.ship(at).card),
        Some(AdmiralId::Vance.flagship())
    );
    let draft = game.draft.clone().expect("Nachschub nach der Runde");
    connection.send(ClientMessage::Draft {
        pick: DraftPick {
            ship: (!draft.ships.is_empty()).then_some(0),
            command: (!draft.commands.is_empty()).then_some(0),
        },
    });
    game.apply(next_view(&connection, &mut rounds));
    assert_eq!(game.draft, None);
}
