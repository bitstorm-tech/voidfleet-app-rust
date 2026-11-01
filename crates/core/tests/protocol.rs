//! JSON-Format und Spiegelung der Protokoll-Nachrichten.

use serde_json::json;
use voidfleet_core::admirals::AdmiralId;
use voidfleet_core::board::LaneType::{Command, Strike};
use voidfleet_core::board::Side::{Enemy, Player};
use voidfleet_core::board::{Board, SlotRef, SlotTarget};
use voidfleet_core::cards::{CommandId, ShipId};
use voidfleet_core::protocol::{
    ClientMessage, CommandPlan, ErrorCode, FireOrder, FireTarget, Mirror, Miss, RoundEvent,
    ServerMessage, ShipRef, TurnPlan,
};

#[test]
fn nachrichten_als_json_mit_snake_case_ids() {
    let submit = ClientMessage::Submit {
        plan: TurnPlan {
            command: Some(CommandPlan {
                hand_index: 0,
                target: Some(SlotTarget::new(Strike, 2)),
                destination: None,
            }),
            fire: vec![
                FireOrder {
                    ship: SlotTarget::new(Command, 1),
                    target: FireTarget::Column(3),
                },
                FireOrder {
                    ship: SlotTarget::new(Strike, 0),
                    target: FireTarget::Ship(SlotTarget::new(Command, 4)),
                },
            ],
            ..TurnPlan::default()
        },
    };
    let value = serde_json::to_value(&submit).unwrap();
    assert_eq!(value["type"], "submit");
    assert_eq!(
        value["plan"]["command"]["target"],
        json!({ "lane": "strike", "slot_index": 2 })
    );
    assert_eq!(value["plan"]["fire"][0]["target"], json!({ "column": 3 }));
    assert_eq!(
        value["plan"]["fire"][1]["target"],
        json!({ "ship": { "lane": "command", "slot_index": 4 } })
    );
    assert_eq!(
        serde_json::from_value::<ClientMessage>(value).unwrap(),
        submit
    );

    let choose = json!({ "type": "choose_admiral", "admiral": "halvorsen" });
    assert_eq!(
        serde_json::from_value::<ClientMessage>(choose).unwrap(),
        ClientMessage::ChooseAdmiral {
            admiral: AdmiralId::Halvorsen
        }
    );
    assert_eq!(
        serde_json::to_value(ServerMessage::Error {
            code: ErrorCode::FlagshipRequired
        })
        .unwrap(),
        json!({ "type": "error", "code": "flagship_required" })
    );
}

#[test]
fn ereignisse_tragen_ihre_art_und_spiegeln_jede_seite() {
    let event = RoundEvent::Fizzle {
        attacker: ShipRef::new(SlotRef::new(Player, Strike, 1), ShipId::VoidInterceptor),
        head_start: false,
        miss: Miss::Target {
            target: Some(ShipRef::new(SlotRef::new(Enemy, Command, 3), ShipId::Hive)),
            covered: true,
        },
    };
    let value = serde_json::to_value(event).unwrap();
    assert_eq!(value["kind"], "fizzle");
    assert_eq!(value["attacker"]["card_id"], "void_interceptor");
    assert_eq!(value["miss"]["reason"], "target");
    assert_eq!(serde_json::from_value::<RoundEvent>(value).unwrap(), event);

    let mirrored = event.mirrored();
    let RoundEvent::Fizzle { attacker, miss, .. } = mirrored else {
        panic!("Fizzle bleibt Fizzle");
    };
    assert_eq!(attacker.side, Enemy);
    assert!(matches!(
        miss,
        Miss::Target {
            target: Some(ShipRef { side: Player, .. }),
            ..
        }
    ));
    assert_eq!(mirrored.mirrored(), event);

    let draw = RoundEvent::NoEffect {
        side: Player,
        card_id: CommandId::CoupDeGrace,
    };
    assert_eq!(
        serde_json::to_value(draw).unwrap(),
        json!({ "kind": "no_effect", "side": "player", "card_id": "coup_de_grace" })
    );
}

#[test]
fn snapshot_uebersteht_json() {
    let mut board = Board::new();
    board.place(SlotRef::new(Player, Strike, 2), ShipId::IronDreadnought);
    board.place(SlotRef::new(Enemy, Command, 0), ShipId::Pendulum);
    let snapshot = board.snapshot();
    let json = serde_json::to_string(&snapshot).unwrap();
    assert_eq!(
        serde_json::from_str::<voidfleet_core::board::BoardSnapshot>(&json).unwrap(),
        snapshot
    );
    assert_eq!(Board::from_snapshot(&snapshot).snapshot(), snapshot);
}
