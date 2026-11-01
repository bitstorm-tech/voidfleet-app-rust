//! Der Server über eine echte WebSocket-Verbindung: eingebettet gestartet ([`spawn_local`]), ein Match gegen den
//! Computergegner bis in Runde 2.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use voidfleet_core::admirals::AdmiralId;
use voidfleet_core::board::{LaneType, SlotTarget};
use voidfleet_core::protocol::{
    ClientMessage, DraftPick, ErrorCode, PlayerView, ServerMessage, TurnPlan,
};
use voidfleet_server::spawn_local;

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn receive(socket: &mut Socket) -> ServerMessage {
    loop {
        match socket
            .next()
            .await
            .expect("Verbindung offen")
            .expect("lesbar")
        {
            Message::Text(text) => return serde_json::from_str(&text).expect("Servernachricht"),
            Message::Close(_) => panic!("Server hat die Verbindung geschlossen"),
            _ => {}
        }
    }
}

async fn state(socket: &mut Socket) -> PlayerView {
    match receive(socket).await {
        ServerMessage::State { view } => *view,
        other => panic!("Sicht erwartet, nicht {other:?}"),
    }
}

async fn send(socket: &mut Socket, message: &ClientMessage) {
    let json = serde_json::to_string(message).unwrap();
    socket.send(Message::text(json)).await.unwrap();
}

#[tokio::test]
async fn ein_match_gegen_den_computergegner_kommt_zustande() {
    let address = spawn_local().unwrap();
    let url = format!("ws://{address}/ws?match=probe&token=a&bot");
    let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    // Der Bot sitzt sofort und hat gewählt; sein Admiral bleibt verdeckt, bis man selbst gewählt hat.
    let joined = state(&mut socket).await;
    let opponent = joined.opponent.expect("der Bot sitzt");
    assert!(opponent.bot);
    assert_eq!(
        (joined.round, joined.admiral, opponent.admiral),
        (1, None, None)
    );

    send(
        &mut socket,
        &ClientMessage::ChooseAdmiral {
            admiral: AdmiralId::Vance,
        },
    )
    .await;
    let started = state(&mut socket).await;
    assert_eq!(started.admiral, Some(AdmiralId::Vance));
    assert!(started.opponent.unwrap().admiral.is_some());

    // Ein ungültiger Zug kommt als Fehlercode zurück.
    send(
        &mut socket,
        &ClientMessage::Submit {
            plan: TurnPlan::default(),
        },
    )
    .await;
    assert_eq!(
        receive(&mut socket).await,
        ServerMessage::Error {
            code: ErrorCode::FlagshipRequired
        }
    );

    let opening = TurnPlan {
        flagship: Some(SlotTarget::new(LaneType::Command, 2)),
        ..TurnPlan::default()
    };
    send(&mut socket, &ClientMessage::Submit { plan: opening }).await;
    match receive(&mut socket).await {
        ServerMessage::Round { round, events } => {
            assert_eq!(round, 1);
            assert!(!events.is_empty());
        }
        other => panic!("Runde erwartet, nicht {other:?}"),
    }
    let second = state(&mut socket).await;
    assert_eq!((second.round, second.result), (2, None));
    let offer = second.draft.expect("Nachschub nach der Runde");

    send(
        &mut socket,
        &ClientMessage::Draft {
            pick: DraftPick {
                ship: (!offer.ships.is_empty()).then_some(0),
                command: (!offer.commands.is_empty()).then_some(0),
            },
        },
    )
    .await;
    assert_eq!(state(&mut socket).await.draft, None);
}

#[tokio::test]
async fn ohne_match_und_token_gibt_es_keinen_websocket() {
    let address = spawn_local().unwrap();
    let refused = tokio_tungstenite::connect_async(format!("ws://{address}/ws?match=probe")).await;
    assert!(refused.is_err());
}
