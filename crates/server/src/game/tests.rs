//! Das Match, portiert aus `server/src/match.test.ts`.

use rand::SeedableRng;
use rand::rngs::StdRng;
use voidfleet_core::admirals::{AdmiralId, PowerId};
use voidfleet_core::board::LaneType::{self, Command, Strike};
use voidfleet_core::board::Side::{Enemy, Player};
use voidfleet_core::board::SlotState;
use voidfleet_core::cards::{CommandId as C, SHIPS, ShipId as S};
use voidfleet_core::protocol::{
    CommandEffect, CommandPlan, Deploy, FireOrder, Maneuver, PowerPlan, ShipRef, ShipStats,
};

use super::*;

// --- Hilfen ---

const FIRST: Seat = Seat::First;
const SECOND: Seat = Seat::Second;
const OSEI: [AdmiralId; 2] = [AdmiralId::Osei, AdmiralId::Osei];

/// Zufall mit festem Wert: `Fixed(0)` wählt das erste, `Fixed(u64::MAX)` das letzte Element.
struct Fixed(u64);

impl RngCore for Fixed {
    fn next_u32(&mut self) -> u32 {
        self.0 as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.0
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        dst.fill(self.0 as u8);
    }
}

const fn strike(index: usize) -> SlotTarget {
    SlotTarget::new(Strike, index)
}

const fn command(index: usize) -> SlotTarget {
    SlotTarget::new(Command, index)
}

const fn slot(side: Side, target: SlotTarget) -> SlotRef {
    target.on(side)
}

const fn ship(side: Side, target: SlotTarget, card: S) -> ShipRef {
    ShipRef::new(target.on(side), card)
}

fn seeded() -> StdRng {
    StdRng::seed_from_u64(7)
}

/// Beide sitzen und haben ihren Admiral gewählt: Runde 1 kann beginnen.
fn new_match_with(admirals: [AdmiralId; 2], rng: impl RngCore + Send + 'static) -> Match {
    let mut game = Match::with_rng("test", rng);
    game.join("a").unwrap();
    game.join("b").unwrap();
    game.choose_admiral(FIRST, admirals[0]).unwrap();
    game.choose_admiral(SECOND, admirals[1]).unwrap();
    game
}

fn new_match(admirals: [AdmiralId; 2]) -> Match {
    new_match_with(admirals, seeded())
}

/// Ein Match in Runde 2, auf dem Brett noch ohne Flaggschiffe; für eine einzelne Runde genügt das, danach ist es
/// vorbei.
fn mid_game(admirals: [AdmiralId; 2]) -> Match {
    let mut game = new_match(admirals);
    game.round = 2;
    game
}

/// Ein Match in Runde 2 mit je einer Bastion als Flaggschiff, damit es nach der Runde weitergeht.
fn ongoing() -> Match {
    let mut game = mid_game(OSEI);
    for side in Side::ALL {
        place(&mut game, side, command(2), S::Bastion);
    }
    game
}

/// Ein Zug; ohne Angaben passt er (in Runde 1 fehlt dann noch das Flaggschiff).
fn pass() -> TurnPlan {
    TurnPlan::default()
}

fn deploy(hand_index: usize, target: SlotTarget) -> Deploy {
    Deploy { hand_index, target }
}

fn order(
    hand_index: usize,
    target: Option<SlotTarget>,
    destination: Option<usize>,
) -> Option<CommandPlan> {
    Some(CommandPlan {
        hand_index,
        target,
        destination,
    })
}

fn maneuver(ship: SlotTarget, destination: SlotTarget) -> Option<Maneuver> {
    Some(Maneuver { ship, destination })
}

fn power(target: SlotTarget) -> Option<PowerPlan> {
    Some(PowerPlan { target })
}

fn column(ship: SlotTarget, column: usize) -> FireOrder {
    FireOrder {
        ship,
        target: FireTarget::Column(column),
    }
}

fn at_ship(ship: SlotTarget, target: SlotTarget) -> FireOrder {
    FireOrder {
        ship,
        target: FireTarget::Ship(target),
    }
}

/// Aus dem Nachschub je die erste Karte.
const FIRST_PICK: DraftPick = DraftPick {
    ship: Some(0),
    command: Some(0),
};

fn place(game: &mut Match, side: Side, target: SlotTarget, card: S) -> SlotRef {
    let at = slot(side, target);
    game.board.place(at, card).expect("Slot ist frei");
    at
}

fn place_hp(game: &mut Match, side: Side, target: SlotTarget, card: S, hp: i32) -> SlotRef {
    let at = place(game, side, target, card);
    game.board.ship_mut(at).hp = hp;
    at
}

fn state(card_id: S, hp: i32) -> Option<SlotState> {
    Some(SlotState {
        card_id,
        hp,
        shield: 0,
        scrap_bonus: 0,
        marked: false,
    })
}

fn set_commands(game: &mut Match, seat: Seat, cards: &[C]) {
    game.player_mut(seat).command_hand = cards.to_vec();
}

/// Ersetzt die Schiffshände und leert die Schiffsdecks, damit kein Nachschub kommt.
fn set_ship_hands(game: &mut Match, hands: [&[S]; 2]) {
    for (player, hand) in game.players.iter_mut().zip(hands) {
        player.ship_hand = hand.to_vec();
        player.ship_deck.clear();
    }
}

/// Gibt den Zug ab und nimmt ihn gleich zurück, um die nächste Variante zu prüfen.
fn try_submit(game: &mut Match, plan: TurnPlan) -> Result<(), ErrorCode> {
    let result = game.submit(FIRST, plan);
    game.player_mut(FIRST).plan = None;
    result
}

fn play_round(game: &mut Match, plans: [TurnPlan; 2]) -> Vec<RoundEvent> {
    let [first, second] = plans;
    assert_eq!(game.submit(FIRST, first), Ok(()));
    assert_eq!(game.submit(SECOND, second), Ok(()));
    game.resolve_round()
}

fn play_passes(game: &mut Match) -> Vec<RoundEvent> {
    play_round(game, [pass(), pass()])
}

/// Die Art jedes Ereignisses, wie sie übertragen wird.
fn kinds(events: &[RoundEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            serde_json::to_value(event).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

fn deployed(events: &[RoundEvent], side: Side) -> Vec<(S, SlotTarget)> {
    events
        .iter()
        .filter_map(|event| match *event {
            RoundEvent::Deploy {
                side: deployer,
                card_id,
                target,
            } if deployer == side => Some((card_id, target)),
            _ => None,
        })
        .collect()
}

fn salvaged(events: &[RoundEvent]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            RoundEvent::Ability {
                effect: AbilityEffect::Salvage,
                ..
            }
        )
    })
}

// --- Sitze und Admirale ---

#[test]
fn ereignisse_fuer_den_zweiten_sitz_sind_gespiegelt_auch_in_verschachtelten_schiffen() {
    let game = new_match(OSEI);
    let attack = |attacker, target| RoundEvent::Attack {
        attacker,
        target,
        amount: 2,
        after: ShipStats { hp: 1, shield: 0 },
        focus: false,
        swarm: false,
        head_start: false,
        through: false,
        taunt: false,
    };
    let event = attack(
        ship(Player, strike(0), S::HorizonCruiser),
        ship(Enemy, strike(1), S::RepairDrone),
    );
    assert_eq!(
        game.events_for(SECOND, &[event]),
        [attack(
            ship(Enemy, strike(0), S::HorizonCruiser),
            ship(Player, strike(1), S::RepairDrone),
        )]
    );
}

#[test]
fn vor_runde_1_waehlt_jeder_einmal_seinen_admiral_sichtbar_erst_wenn_beide_gewaehlt_haben() {
    let mut game = Match::with_rng("test", seeded());
    game.join("a").unwrap();
    let opening = TurnPlan {
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, opening.clone()),
        Err(ErrorCode::AdmiralRequired)
    );
    assert_eq!(game.choose_admiral(FIRST, AdmiralId::Kerr), Ok(()));
    assert_eq!(
        game.choose_admiral(FIRST, AdmiralId::Osei),
        Err(ErrorCode::AdmiralAlreadyChosen)
    );
    assert_eq!(game.view(FIRST).admiral, Some(AdmiralId::Kerr));
    assert_eq!(
        game.submit(FIRST, opening.clone()),
        Err(ErrorCode::OpponentMissing)
    );

    game.join("b").unwrap();
    assert_eq!(
        game.submit(FIRST, opening.clone()),
        Err(ErrorCode::OpponentChoosing)
    );
    assert_eq!(
        game.submit(SECOND, opening.clone()),
        Err(ErrorCode::AdmiralRequired)
    );
    let second = game.view(SECOND);
    assert_eq!(
        (second.admiral, second.opponent.unwrap().admiral),
        (None, None)
    );
    assert_eq!(game.choose_admiral(SECOND, AdmiralId::Kerr), Ok(()));
    assert_eq!(
        (
            game.view(FIRST).opponent.unwrap().admiral,
            game.view(SECOND).opponent.unwrap().admiral
        ),
        (Some(AdmiralId::Kerr), Some(AdmiralId::Kerr))
    );
    assert_eq!(game.submit(FIRST, opening), Ok(()));
}

#[test]
fn der_computergegner_macht_seinen_platz_frei_solange_das_match_nicht_begonnen_hat() {
    let mut game = Match::with_rng("test", seeded());
    assert_eq!(game.remove_bot(), Err(ErrorCode::OpponentMissing));
    game.join("a").unwrap();
    game.add_bot().unwrap();
    assert!(game.view(FIRST).opponent.unwrap().bot);
    assert_eq!(game.remove_bot(), Ok(()));
    assert_eq!(game.view(FIRST).opponent, None);
    assert_eq!(game.join("b"), Ok(SECOND));
    assert!(!game.view(FIRST).opponent.unwrap().bot);
    assert_eq!(game.remove_bot(), Err(ErrorCode::OpponentMissing));

    let mut started = Match::with_rng("started", seeded());
    started.join("a").unwrap();
    started.add_bot().unwrap();
    started.choose_admiral(FIRST, AdmiralId::Osei).unwrap();
    assert_eq!(started.remove_bot(), Err(ErrorCode::MatchStarted));
}

/// Der Spieler gegen den Computergegner, der mit `Fixed(u64::MAX)` den letzten Admiral (Kerr) nimmt; die Runde 1 ist
/// aufgelöst.
fn opening_against_bot() -> (Match, Vec<RoundEvent>) {
    let mut game = Match::with_rng("test", Fixed(u64::MAX));
    game.join("a").unwrap();
    assert_eq!(game.add_bot(), Ok(SECOND));
    assert_eq!(game.add_bot(), Err(ErrorCode::MatchFull));
    assert_eq!(game.join("c"), Err(ErrorCode::MatchFull));
    assert_eq!(game.player(SECOND).admiral, Some(AdmiralId::Kerr));
    let opponent = game.view(FIRST).opponent.unwrap();
    assert_eq!(opponent.admiral, None);
    assert!(!opponent.submitted);
    assert!(opponent.played_commands.is_empty());
    // Die Hände sind gemischt; ohne Schnellverlegung legt der Bot in Runde 1 genau zwei Schiffe.
    set_commands(&mut game, SECOND, &[C::EmergencyShields]);

    let opening = TurnPlan {
        ships: vec![deploy(0, strike(0))],
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, opening.clone()),
        Err(ErrorCode::AdmiralRequired)
    );
    game.choose_admiral(FIRST, AdmiralId::Osei).unwrap();
    assert_eq!(
        game.view(FIRST).opponent.unwrap().admiral,
        Some(AdmiralId::Kerr)
    );
    assert_eq!(game.submit(FIRST, opening), Ok(()));
    assert!(game.is_round_complete());
    let events = game.resolve_round();
    (game, events)
}

#[test]
fn der_computergegner_nimmt_den_freien_platz_waehlt_sofort_seinen_admiral_und_zieht_mit() {
    let (game, events) = opening_against_bot();
    assert_eq!(
        deployed(&events, Enemy).first().map(|(card, _)| *card),
        Some(S::Pendulum)
    );
    assert_eq!(game.outcome(), None);
    assert_eq!(game.round(), 2);
}

#[test]
#[ignore = "braucht Bot"]
fn der_computergegner_legt_in_runde_1_sein_flaggschiff_und_zwei_schiffe() {
    let (_, events) = opening_against_bot();
    let cards: Vec<S> = deployed(&events, Enemy)
        .into_iter()
        .map(|(card, _)| card)
        .collect();
    assert_eq!(cards.len(), 3);
    assert_eq!(cards[0], S::Pendulum);
}

#[test]
fn der_computergegner_waehlt_seinen_nachschub_sofort() {
    let mut game = Match::with_rng("test", seeded());
    game.join("a").unwrap();
    game.add_bot().unwrap();
    game.choose_admiral(FIRST, AdmiralId::Osei).unwrap();
    let opening = TurnPlan {
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(game.submit(FIRST, opening), Ok(()));
    let before = game.player(SECOND).ship_hand.len();
    game.resolve_round();
    assert_eq!(game.player(SECOND).draft, None);
    assert!(game.view(FIRST).draft.is_some());
    assert!(game.player(SECOND).ship_hand.len() + 3 > before);
}

#[test]
fn die_starthand_hat_genau_1_support_und_3_kampfschiffe() {
    for seed in 0..50 {
        let game = new_match_with(OSEI, StdRng::seed_from_u64(seed));
        for player in &game.players {
            let support = player
                .ship_hand
                .iter()
                .filter(|&&card| is_support(card))
                .count();
            assert_eq!((player.ship_hand.len(), support), (4, 1));
            assert_eq!(player.ship_deck.len(), SHIPS.len() * 2 - 4);
        }
    }
}

// --- Deploys ---

#[test]
fn in_runde_1_legt_jeder_sein_flaggschiff_und_bis_zu_zwei_schiffe_in_verschiedene_slots() {
    let mut game = new_match([AdmiralId::Vance, AdmiralId::Kerr]);
    let two_ships = |first, second, flagship: Option<SlotTarget>| TurnPlan {
        ships: vec![deploy(0, first), deploy(1, second)],
        flagship,
        ..pass()
    };
    let three_ships = TurnPlan {
        ships: (0..3).map(|index| deploy(index, strike(index))).collect(),
        flagship: Some(command(5)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, three_ships),
        Err(ErrorCode::TooManyShips)
    );
    assert_eq!(
        game.submit(FIRST, two_ships(strike(2), strike(3), None)),
        Err(ErrorCode::FlagshipRequired)
    );
    assert_eq!(
        game.submit(FIRST, two_ships(strike(2), strike(2), Some(command(5)))),
        Err(ErrorCode::SlotUnavailable)
    );
    assert_eq!(
        game.submit(FIRST, two_ships(strike(2), strike(3), Some(strike(3)))),
        Err(ErrorCode::SlotUnavailable)
    );
    let same_card = TurnPlan {
        ships: vec![deploy(1, strike(2)), deploy(1, strike(3))],
        ..two_ships(strike(2), strike(3), Some(command(5)))
    };
    assert_eq!(game.submit(FIRST, same_card), Err(ErrorCode::ShipNotInHand));
    assert_eq!(
        game.submit(FIRST, two_ships(strike(2), command(2), Some(command(5)))),
        Ok(())
    );
    assert_eq!(
        game.submit(SECOND, two_ships(strike(1), strike(4), Some(command(1)))),
        Ok(())
    );
    let events = game.resolve_round();
    let player = deployed(&events, Player);
    assert_eq!((player.len(), player[0]), (3, (S::Vanguard, command(5))));
    let enemy = deployed(&events, Enemy);
    assert_eq!((enemy.len(), enemy[0]), (3, (S::Pendulum, command(1))));
    assert_eq!(game.player(FIRST).ship_hand.len(), 2);
    assert_eq!(game.draft(FIRST, FIRST_PICK), Ok(()));
    assert_eq!(game.player(FIRST).ship_hand.len(), 3);
    assert_eq!(
        game.submit(FIRST, two_ships(strike(0), strike(5), None)),
        Err(ErrorCode::TooManyShips)
    );
    let flagship = TurnPlan {
        flagship: Some(command(4)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, flagship),
        Err(ErrorCode::FlagshipNotAllowed)
    );
}

#[test]
fn das_flaggschiff_und_die_deploys_halten_zusammen_lane_und_flottengrenze_ein() {
    let mut game = new_match(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    let two_ships = |first, second, flagship| TurnPlan {
        ships: vec![deploy(0, first), deploy(1, second)],
        flagship: Some(flagship),
        ..pass()
    };
    for index in [0, 1] {
        place(&mut game, Player, strike(index), card);
    }
    // Strike: 2 Schiffe + Flaggschiff + Deploy = 4, für den zweiten Deploy ist dort kein Platz mehr.
    assert_eq!(
        game.submit(FIRST, two_ships(strike(3), strike(4), strike(2))),
        Err(ErrorCode::SlotUnavailable)
    );
    for index in [0, 1] {
        place(&mut game, Player, command(index), card);
    }
    // 4 Schiffe + Flaggschiff + 2 Deploys überschreiten die Flottengrenze von 6.
    assert_eq!(
        game.submit(FIRST, two_ships(strike(3), command(3), command(2))),
        Err(ErrorCode::SlotUnavailable)
    );
    let one_ship = TurnPlan {
        ships: vec![deploy(0, strike(3))],
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(game.submit(FIRST, one_ship), Ok(()));
}

#[test]
fn befehle_und_lane_wechsel_beruecksichtigen_flaggschiff_und_beide_deploys_der_ersten_runde() {
    let mut game = new_match(OSEI);
    set_commands(&mut game, FIRST, &[C::Intercept, C::EmergencyShields]);
    let intercept = |destination| TurnPlan {
        ships: vec![deploy(0, strike(2)), deploy(1, command(4))],
        flagship: Some(command(0)),
        command: order(0, Some(command(4)), Some(destination)),
        ..pass()
    };
    // Intercept auf das zweite gelegte Schiff, aber nicht in den Slot des ersten.
    assert_eq!(
        try_submit(&mut game, intercept(2)),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(try_submit(&mut game, intercept(4)), Ok(()));
    // Auch das Flaggschiff, das gerade landet, darf ein Befehl treffen.
    let shields = TurnPlan {
        flagship: Some(command(0)),
        command: order(1, Some(command(0)), None),
        ..pass()
    };
    assert_eq!(game.submit(FIRST, shields), Ok(()));
}

#[test]
fn mit_schnellverlegung_darf_ein_schiff_mehr_gelegt_werden_in_runde_1_also_drei() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::RapidDeployment, C::Scavenge]);
    let deploys = |count: usize, hand_index| TurnPlan {
        ships: (0..count)
            .map(|index| deploy(index, strike(index)))
            .collect(),
        command: order(hand_index, None, None),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, deploys(2, 1)),
        Err(ErrorCode::TooManyShips)
    );
    assert_eq!(
        game.submit(FIRST, deploys(3, 0)),
        Err(ErrorCode::TooManyShips)
    );
    assert_eq!(game.submit(FIRST, deploys(2, 0)), Ok(()));

    let mut opening = new_match(OSEI);
    set_commands(&mut opening, FIRST, &[C::RapidDeployment]);
    let flagship = TurnPlan {
        flagship: Some(command(2)),
        ..pass()
    };
    let three = TurnPlan {
        flagship: Some(command(2)),
        ..deploys(3, 0)
    };
    let events = play_round(&mut opening, [three, flagship]);
    assert_eq!(deployed(&events, Player).len(), 4);
    assert!(events.contains(&RoundEvent::Rule {
        side: Player,
        card_id: C::RapidDeployment
    }));
}

// --- Befehle ---

#[test]
fn befehlsziele_muessen_in_der_passenden_lane_stehen() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    place(&mut game, Player, strike(0), card);
    set_commands(&mut game, FIRST, &[C::Intercept]);
    let intercept = TurnPlan {
        command: order(0, Some(strike(0)), Some(1)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, intercept),
        Err(ErrorCode::TargetWrongLane)
    );
}

#[test]
fn verlegende_befehle_brauchen_einen_freien_ziel_slot_in_der_anderen_lane_auch_nach_dem_deploy() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    place(&mut game, Player, command(2), card);
    place(&mut game, Player, strike(0), card);
    set_commands(&mut game, FIRST, &[C::Intercept]);
    let intercept = |destination, deploy_to: Option<usize>| TurnPlan {
        ships: deploy_to
            .map(|index| deploy(0, strike(index)))
            .into_iter()
            .collect(),
        command: order(0, Some(command(2)), destination),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, intercept(None, None)),
        Err(ErrorCode::DestinationRequired)
    );
    assert_eq!(
        game.submit(FIRST, intercept(Some(0), None)),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(
        game.submit(FIRST, intercept(Some(4), Some(4))),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(game.submit(FIRST, intercept(Some(5), Some(4))), Ok(()));
}

#[test]
fn befehle_duerfen_das_schiff_treffen_das_in_derselben_runde_gelegt_wird() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::Overcharge]);
    let overcharge = |target| TurnPlan {
        ships: vec![deploy(0, strike(2))],
        command: order(0, Some(target), None),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, overcharge(strike(3))),
        Err(ErrorCode::TargetNotOwnShip)
    );
    let without_deploy = TurnPlan {
        ships: Vec::new(),
        ..overcharge(strike(2))
    };
    assert_eq!(
        game.submit(FIRST, without_deploy),
        Err(ErrorCode::TargetNotOwnShip)
    );
    assert_eq!(game.submit(FIRST, overcharge(strike(2))), Ok(()));
}

#[test]
fn das_gelegte_schiff_darf_sofort_die_lane_wechseln_die_lane_grenze_gilt_dabei_weiter() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    set_commands(&mut game, FIRST, &[C::Intercept, C::Recall]);
    // Legt ein Schiff in `lane` auf Slot `deploy_to` und verlegt es mit Intercept (aus Command) bzw. Recall (aus
    // Strike).
    let deploy_and_move = |lane: LaneType, deploy_to, destination| {
        let target = SlotTarget::new(lane, deploy_to);
        TurnPlan {
            ships: vec![deploy(0, target)],
            command: order(usize::from(lane == Strike), Some(target), Some(destination)),
            ..pass()
        }
    };
    // Intercept auf ein frisch in die Command-Lane gelegtes Schiff, auch in den Strike-Slot direkt davor.
    assert_eq!(
        try_submit(&mut game, deploy_and_move(Command, 1, 1)),
        Ok(())
    );

    // Strike voll: Das gelegte Schiff kommt nicht hinein, mit einem freien Platz schon.
    for index in [0, 1, 2, 3] {
        place(&mut game, Player, strike(index), card);
    }
    assert_eq!(
        try_submit(&mut game, deploy_and_move(Command, 1, 4)),
        Err(ErrorCode::DestinationUnavailable)
    );
    game.board[slot(Player, strike(3))] = None;
    assert_eq!(
        try_submit(&mut game, deploy_and_move(Command, 1, 4)),
        Ok(())
    );

    // Recall vom gelegten Schiff zurück in die Command-Lane, aber nicht in einen belegten Slot.
    place(&mut game, Player, command(5), card);
    assert_eq!(
        try_submit(&mut game, deploy_and_move(Strike, 3, 5)),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(try_submit(&mut game, deploy_and_move(Strike, 3, 4)), Ok(()));
}

#[test]
fn ein_anderes_schiff_darf_nicht_in_den_deploy_slot_und_fuellt_dessen_lane_nicht_ueber_die_grenze()
{
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    set_commands(&mut game, FIRST, &[C::Recall]);
    place(&mut game, Player, strike(0), card);
    let recall = |deploy_to, destination| TurnPlan {
        ships: vec![deploy(0, command(deploy_to))],
        command: order(0, Some(strike(0)), Some(destination)),
        ..pass()
    };
    assert_eq!(
        try_submit(&mut game, recall(2, 2)),
        Err(ErrorCode::DestinationUnavailable)
    );
    for index in [3, 4] {
        place(&mut game, Player, command(index), card);
    }
    // Command: 2 Schiffe + Deploy = 3, der Wechsel macht 4.
    assert_eq!(try_submit(&mut game, recall(2, 1)), Ok(()));
    place(&mut game, Player, command(5), card);
    // Command: 3 Schiffe + Deploy = 4, kein Platz mehr für den Wechsel.
    assert_eq!(
        try_submit(&mut game, recall(2, 1)),
        Err(ErrorCode::DestinationUnavailable)
    );
}

#[test]
fn ein_befehl_auf_das_gelegte_schiff_wirkt_in_derselben_runde_und_es_feuert_schon() {
    let mut game = mid_game(OSEI);
    set_ship_hands(&mut game, [&[S::VoidInterceptor], &[S::VoidInterceptor]]);
    for seat in Seat::ALL {
        set_commands(&mut game, seat, &[C::EmergencyShields, C::Intercept]);
    }
    for side in Side::ALL {
        place(&mut game, side, strike(5), S::HorizonCruiser);
    }
    let shielded = TurnPlan {
        ships: vec![deploy(0, strike(0))],
        command: order(0, Some(strike(0)), None),
        fire: vec![column(strike(0), 5)],
        ..pass()
    };
    let intercepted = TurnPlan {
        ships: vec![deploy(0, command(2))],
        command: order(1, Some(command(2)), Some(4)),
        fire: vec![column(command(2), 5)],
        ..pass()
    };
    let events = play_round(&mut game, [shielded, intercepted]);
    assert!(events.iter().any(|event| matches!(
        event,
        RoundEvent::Effect { card_id: C::EmergencyShields, ship, .. } if ship.slot() == slot(Player, strike(0))
    )));
    let interceptor = state(S::VoidInterceptor, S::VoidInterceptor.hp());
    assert_eq!(game.board.state(slot(Player, strike(0))), interceptor);
    assert!(!game.board.is_occupied(slot(Enemy, command(2))));
    assert_eq!(game.board.state(slot(Enemy, strike(4))), interceptor);
    // Der Interceptor des Gegners trifft nach dem Intercept mit 3 + 2 ATK, der eigene mit 3.
    let cruiser = S::HorizonCruiser.hp();
    assert_eq!(game.board.ship(slot(Player, strike(5))).hp, cruiser - 5);
    assert_eq!(game.board.ship(slot(Enemy, strike(5))).hp, cruiser - 3);
}

#[test]
fn der_torpedo_zielt_auf_einen_slot_der_gegnerseite_auch_einen_leeren_und_verlegt_nichts() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::Torpedo]);
    let torpedo = |target, destination| TurnPlan {
        command: order(0, target, destination),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, torpedo(None, None)),
        Err(ErrorCode::TargetRequired)
    );
    assert_eq!(
        game.submit(FIRST, torpedo(Some(strike(6)), None)),
        Err(ErrorCode::TargetNotEnemySlot)
    );
    assert_eq!(
        game.submit(FIRST, torpedo(Some(strike(2)), Some(2))),
        Err(ErrorCode::DestinationNotAllowed)
    );
    assert_eq!(game.submit(FIRST, torpedo(Some(command(4)), None)), Ok(()));
}

#[test]
fn der_zweite_sitz_schiesst_einen_torpedo_er_trifft_den_gespiegelten_slot() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, SECOND, &[C::Torpedo]);
    place_hp(&mut game, Player, strike(2), S::VoidInterceptor, 17);
    place_hp(&mut game, Enemy, strike(0), S::VoidInterceptor, 20);
    let torpedo = TurnPlan {
        command: order(0, Some(strike(2)), None),
        ..pass()
    };
    let hit = play_round(&mut game, [pass(), torpedo]);
    let effect = |events: &[RoundEvent]| {
        events
            .iter()
            .find_map(|event| match *event {
                RoundEvent::Effect {
                    side,
                    ship,
                    effect,
                    after,
                    ..
                } => Some((side, ship.slot(), effect, after.hp)),
                _ => None,
            })
            .unwrap()
    };
    // Brett: Der zweite Sitz spielt auf "enemy", sein Ziel liegt auf "player". 17 − 5.
    let damage = CommandEffect::Damage { amount: 5 };
    assert_eq!(effect(&hit), (Enemy, slot(Player, strike(2)), damage, 12));
    let fired = |side, target_side| RoundEvent::Command {
        side,
        card_id: C::Torpedo,
        target: Some(slot(target_side, strike(2))),
    };
    assert!(hit.contains(&fired(Enemy, Player)));
    let mirrored = game.events_for(SECOND, &hit);
    assert!(mirrored.contains(&fired(Player, Enemy)));
    assert_eq!(
        effect(&mirrored),
        (Player, slot(Enemy, strike(2)), damage, 12)
    );
}

#[test]
fn der_zweite_sitz_schiesst_einen_torpedo_in_den_slot_aus_dem_der_gegner_manoevriert_er_verpufft() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, SECOND, &[C::Torpedo]);
    place_hp(&mut game, Player, strike(2), S::VoidInterceptor, 12);
    let missed = play_round(
        &mut game,
        [
            TurnPlan {
                maneuver: maneuver(strike(2), command(3)),
                ..pass()
            },
            TurnPlan {
                command: order(0, Some(strike(2)), None),
                ..pass()
            },
        ],
    );
    let miss = |side, target_side| RoundEvent::Miss {
        side,
        card_id: C::Torpedo,
        target: slot(target_side, strike(2)),
    };
    assert!(missed.contains(&miss(Enemy, Player)));
    assert!(
        game.events_for(SECOND, &missed)
            .contains(&miss(Player, Enemy))
    );
    assert_eq!(
        game.board.card(slot(Player, command(3))),
        Some(S::VoidInterceptor)
    );
}

#[test]
fn faellt_ein_schiff_beim_deploy_durch_die_plasma_frigate_entfallen_sein_manoever_und_der_befehl_darauf()
 {
    let mut game = new_match_with(OSEI, Fixed(0));
    game.round = 2;
    set_ship_hands(&mut game, [&[], &[S::PlasmaFrigate]]);
    set_commands(&mut game, FIRST, &[C::EmergencyShields]);
    place_hp(&mut game, Player, strike(3), S::VoidInterceptor, 1);
    let shield_and_maneuver = TurnPlan {
        command: order(0, Some(strike(3)), None),
        maneuver: maneuver(strike(3), command(1)),
        ..pass()
    };
    let frigate = TurnPlan {
        ships: vec![deploy(0, strike(2))],
        ..pass()
    };
    let events = play_round(&mut game, [shield_and_maneuver, frigate]);
    assert_eq!(
        kinds(&events),
        [
            "pass",
            "deploy",
            "ability",
            "destroyed",
            "command",
            "no_effect"
        ]
    );
}

#[test]
fn waermesucher_und_traktorstrahl_zielen_auf_ein_gegnerisches_schiff_der_traktorstrahl_nie_auf_ein_flaggschiff()
 {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::HeatSeeker, C::TractorBeam]);
    place(&mut game, Enemy, strike(1), S::HorizonCruiser);
    place(&mut game, Enemy, command(2), S::Bastion);
    let aim = |hand_index, target, destination| TurnPlan {
        command: order(hand_index, target, destination),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, aim(0, None, None)),
        Err(ErrorCode::TargetRequired)
    );
    assert_eq!(
        game.submit(FIRST, aim(0, Some(strike(2)), None)),
        Err(ErrorCode::TargetNotEnemyShip)
    );
    // Ein eigenes Schiff, das im gleichen Slot der eigenen Seite landet, macht den leeren gegnerischen Slot nicht
    // zum Ziel.
    let own_deploy = TurnPlan {
        ships: vec![deploy(0, strike(2))],
        ..aim(0, Some(strike(2)), None)
    };
    assert_eq!(
        game.submit(FIRST, own_deploy),
        Err(ErrorCode::TargetNotEnemyShip)
    );
    assert_eq!(
        game.submit(FIRST, aim(0, Some(strike(6)), None)),
        Err(ErrorCode::TargetNotEnemySlot)
    );
    assert_eq!(
        game.submit(FIRST, aim(0, Some(strike(1)), Some(2))),
        Err(ErrorCode::DestinationNotAllowed)
    );
    assert_eq!(
        game.submit(FIRST, aim(1, Some(command(2)), None)),
        Err(ErrorCode::TargetWrongShip)
    );
    assert_eq!(game.submit(FIRST, aim(0, Some(command(2)), None)), Ok(()));
}

#[test]
fn rammkurs_nur_mit_einem_eigenen_strike_schiff_nie_mit_dem_flaggschiff_auch_nicht_dem_gerade_gelegten()
 {
    let mut game = new_match([AdmiralId::Vance, AdmiralId::Osei]);
    set_commands(&mut game, FIRST, &[C::CollisionCourse]);
    let ram = |target, flagship, ships| TurnPlan {
        command: order(0, Some(target), None),
        flagship: Some(flagship),
        ships,
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, ram(strike(2), strike(2), vec![])),
        Err(ErrorCode::TargetWrongShip)
    );
    assert_eq!(
        game.submit(FIRST, ram(command(2), command(2), vec![])),
        Err(ErrorCode::TargetWrongLane)
    );
    assert_eq!(
        game.submit(
            FIRST,
            ram(strike(0), command(2), vec![deploy(0, strike(0))])
        ),
        Ok(())
    );
}

#[test]
fn der_durchschlag_zielt_auf_eine_slot_nummer_der_gegnerseite_gemeldet_als_ihr_strike_slot() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, SECOND, &[C::PiercingShot]);
    place_hp(&mut game, Player, command(4), S::VoidInterceptor, 10);
    let shot = |target| TurnPlan {
        command: order(0, Some(target), None),
        ..pass()
    };
    assert_eq!(
        game.submit(SECOND, shot(command(6))),
        Err(ErrorCode::TargetNotEnemySlot)
    );
    assert_eq!(game.submit(FIRST, pass()), Ok(()));
    assert_eq!(game.submit(SECOND, shot(command(4))), Ok(()));
    let events = game.resolve_round();
    assert!(events.contains(&RoundEvent::Command {
        side: Enemy,
        card_id: C::PiercingShot,
        target: Some(slot(Player, strike(4))),
    }));
    assert_eq!(game.board.ship(slot(Player, command(4))).hp, 7);
}

#[test]
fn das_ausweichmanoever_verlegt_in_einen_freien_slot_derselben_lane_nicht_das_manoevrierte_schiff()
{
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::EvasiveAction]);
    for index in [0, 1, 2, 3] {
        place(&mut game, Player, strike(index), S::VoidInterceptor);
    }
    let evade = |destination, maneuver| TurnPlan {
        command: order(0, Some(strike(0)), Some(destination)),
        maneuver,
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, evade(1, None)),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(
        game.submit(FIRST, evade(4, maneuver(strike(0), command(0)))),
        Err(ErrorCode::ManeuverConflict)
    );
    assert_eq!(
        game.submit(FIRST, evade(4, maneuver(strike(1), strike(4)))),
        Err(ErrorCode::DestinationUnavailable)
    );
    // Die Lane ist voll, das Ausweichen innerhalb der Lane geht trotzdem.
    assert_eq!(
        game.submit(FIRST, evade(5, maneuver(strike(1), strike(4)))),
        Ok(())
    );
}

#[test]
fn ziehende_befehle_ziehen_ueber_den_spieler() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::Countermeasures]);
    set_commands(&mut game, SECOND, &[C::EmergencyShields]);
    place(&mut game, Enemy, strike(0), S::VoidInterceptor);
    let events = play_round(
        &mut game,
        [
            TurnPlan {
                command: order(0, None, None),
                ..pass()
            },
            TurnPlan {
                command: order(0, Some(strike(0)), None),
                ..pass()
            },
        ],
    );
    assert!(events.contains(&RoundEvent::Draw {
        side: Player,
        card_id: C::Countermeasures,
        ships: 0,
        commands: 1,
    }));
    // Gezogen beim Gegenmaßnahmen-Befehl; Nachschub gibt es keinen, ohne Flaggschiffe ist das Match vorbei.
    assert_eq!(game.player(FIRST).command_hand.len(), 1);
}

// --- Grundmanöver ---

#[test]
fn das_grundmanoever_verlegt_ein_eigenes_schiff_in_einen_freien_slot_auch_in_derselben_lane() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    place(&mut game, Player, strike(1), card);
    place(&mut game, Player, command(3), card);
    let moving = |from, destination, deploy_to: Option<SlotTarget>| TurnPlan {
        ships: deploy_to
            .map(|target| deploy(0, target))
            .into_iter()
            .collect(),
        maneuver: maneuver(from, destination),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, moving(strike(0), command(2), None)),
        Err(ErrorCode::ManeuverNotOwnShip)
    );
    // Das gelegte Schiff ist noch nicht auf dem Brett und manövriert nicht.
    assert_eq!(
        game.submit(FIRST, moving(strike(2), command(2), Some(strike(2)))),
        Err(ErrorCode::ManeuverNotOwnShip)
    );
    // In einen belegten Slot geht nur die Mirage (Phasentausch).
    assert_eq!(
        game.submit(FIRST, moving(strike(1), command(3), None)),
        Err(ErrorCode::SwapNotAllowed)
    );
    for (destination, deploy_to) in [
        (command(6), None),
        (command(2), Some(command(2))),
        (strike(1), None),
        (strike(4), Some(strike(4))),
    ] {
        assert_eq!(
            game.submit(FIRST, moving(strike(1), destination, deploy_to)),
            Err(ErrorCode::ManeuverDestinationUnavailable)
        );
    }
    assert_eq!(
        game.submit(FIRST, moving(strike(1), strike(4), None)),
        Ok(())
    );
}

#[test]
fn innerhalb_der_lane_manoevriert_ein_schiff_auch_in_einer_vollen_lane_etwa_in_die_deckung() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    place(&mut game, Player, strike(2), card);
    for index in [0, 3, 4, 5] {
        place(&mut game, Player, command(index), S::RepairDrone);
    }
    assert!(game.board.is_exposed(slot(Player, command(0)), false));
    // In die volle Command-Lane wechselt niemand, innerhalb von ihr schon.
    assert!(
        !game
            .board
            .can_move(slot(Player, strike(2)), command(1), &[], None)
    );
    let into_cover = TurnPlan {
        maneuver: maneuver(command(0), command(2)),
        ..pass()
    };
    let events = play_round(&mut game, [into_cover, pass()]);

    assert!(events.iter().any(|event| matches!(
        event,
        RoundEvent::Maneuver { ship, to, .. } if ship.slot() == slot(Player, command(0)) && *to == command(2)
    )));
    assert!(!game.board.is_occupied(slot(Player, command(0))));
    assert_eq!(
        game.board.card(slot(Player, command(2))),
        Some(S::RepairDrone)
    );
    assert!(!game.board.is_exposed(slot(Player, command(2)), false));
}

#[test]
fn manoever_und_bewegungsbefehl_bewegen_verschiedene_schiffe_in_verschiedene_slots_unter_der_lane_grenze()
 {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    for index in [1, 4] {
        place(&mut game, Player, strike(index), card);
    }
    for index in [3, 5] {
        place(&mut game, Player, command(index), card);
    }
    set_commands(&mut game, FIRST, &[C::FlankingManeuver]);
    let moves = |deploy_to: Option<SlotTarget>, from, destination, flank_from, flank_to| TurnPlan {
        ships: deploy_to
            .map(|target| deploy(0, target))
            .into_iter()
            .collect(),
        command: order(0, Some(flank_from), Some(flank_to)),
        maneuver: maneuver(from, destination),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, moves(None, strike(1), command(2), strike(1), 4)),
        Err(ErrorCode::ManeuverConflict)
    );
    assert_eq!(
        game.submit(FIRST, moves(None, strike(1), command(2), strike(4), 2)),
        Err(ErrorCode::DestinationUnavailable)
    );
    // Der Slot, den das Manöver räumt, bleibt für den Befehl belegt.
    assert_eq!(
        game.submit(FIRST, moves(None, command(3), strike(2), strike(1), 3)),
        Err(ErrorCode::DestinationUnavailable)
    );
    // Command: 2 Schiffe + Deploy + Manöver = 4, für den Befehl ist kein Platz mehr; ohne Deploy schon.
    assert_eq!(
        game.submit(
            FIRST,
            moves(Some(command(0)), strike(1), command(2), strike(4), 4)
        ),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(
        game.submit(FIRST, moves(None, strike(1), command(2), strike(4), 4)),
        Ok(())
    );
}

#[test]
fn ein_manoever_innerhalb_der_ziel_lane_und_eine_drohne_zaehlen_dort_nicht_zur_lane_grenze() {
    let mut game = mid_game(OSEI);
    let card = game.player(FIRST).ship_hand[0];
    for index in [0, 1, 2] {
        place(&mut game, Player, strike(index), card);
    }
    for index in [1, 3] {
        place(&mut game, Player, command(index), card);
    }
    place(&mut game, Player, command(5), S::Drone);
    set_commands(&mut game, FIRST, &[C::Intercept]);
    let intercept = |from, destination| TurnPlan {
        command: order(0, Some(command(3)), Some(4)),
        maneuver: maneuver(from, destination),
        ..pass()
    };
    // Strike: 3 Schiffe und das verlegte macht 4; das manövrierende bleibt in der Lane, die Drohne zählt nicht.
    assert_eq!(
        try_submit(&mut game, intercept(strike(0), strike(5))),
        Ok(())
    );
    assert_eq!(
        try_submit(&mut game, intercept(command(5), strike(5))),
        Ok(())
    );
    // Ein Schiff aus der Command-Lane wäre das fünfte; in den Slot des Manövers wechselt der Befehl nie.
    assert_eq!(
        try_submit(&mut game, intercept(command(1), strike(5))),
        Err(ErrorCode::DestinationUnavailable)
    );
    assert_eq!(
        try_submit(&mut game, intercept(strike(0), strike(4))),
        Err(ErrorCode::DestinationUnavailable)
    );
}

#[test]
fn ein_befehl_auf_das_manoevrierte_schiff_folgt_ihm_es_feuert_nicht_wird_aber_beschossen() {
    let mut game = mid_game(OSEI);
    set_commands(&mut game, FIRST, &[C::Overcharge]);
    for side in Side::ALL {
        place_hp(&mut game, side, strike(0), S::VoidInterceptor, 7);
    }
    let overcharge_and_maneuver = TurnPlan {
        command: order(0, Some(strike(0)), None),
        maneuver: maneuver(strike(0), command(2)),
        ..pass()
    };
    let fire = TurnPlan {
        fire: vec![column(strike(0), 2)],
        ..pass()
    };
    let events = play_round(&mut game, [overcharge_and_maneuver, fire]);

    assert!(events.contains(&RoundEvent::Maneuver {
        side: Player,
        ship: ship(Player, strike(0), S::VoidInterceptor),
        to: command(2),
    }));
    assert!(events.iter().any(|event| matches!(
        event,
        RoundEvent::Command { target, .. } if *target == Some(slot(Player, command(2)))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RoundEvent::Effect { ship, .. } if ship.slot() == slot(Player, command(2))
    )));
    let attackers: Vec<Side> = events
        .iter()
        .filter_map(|event| match event {
            RoundEvent::Attack { attacker, .. } => Some(attacker.side),
            _ => None,
        })
        .collect();
    assert_eq!(attackers, [Enemy]);
    // 7 HP, 3 Schaden vom Gegner, 2 Eigenschaden durch Overcharge.
    assert_eq!(
        game.board.state(slot(Player, command(2))),
        state(S::VoidInterceptor, 2)
    );
    assert!(!game.board.ship(slot(Player, command(2))).maneuvered);
    assert_eq!(
        game.board.state(slot(Enemy, strike(0))),
        state(S::VoidInterceptor, 7)
    );
}

#[test]
fn nur_die_mirage_tauscht_beim_grundmanoever_und_kein_befehl_verlegt_eins_der_beiden() {
    let mut game = mid_game([AdmiralId::Moreau, AdmiralId::Osei]);
    set_commands(&mut game, FIRST, &[C::Recall]);
    place(&mut game, Player, strike(1), S::Mirage);
    place(&mut game, Player, strike(2), S::HorizonCruiser);
    place(&mut game, Player, command(1), S::IronDreadnought);
    let moving = |from, destination, command| TurnPlan {
        maneuver: maneuver(from, destination),
        command,
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, moving(strike(2), command(1), None)),
        Err(ErrorCode::SwapNotAllowed)
    );
    assert_eq!(
        game.submit(FIRST, moving(strike(1), strike(1), None)),
        Err(ErrorCode::ManeuverDestinationUnavailable)
    );
    assert_eq!(
        game.submit(
            FIRST,
            moving(strike(1), strike(2), order(0, Some(strike(2)), Some(4)))
        ),
        Err(ErrorCode::ManeuverConflict)
    );
    assert_eq!(
        game.submit(FIRST, moving(strike(1), command(1), None)),
        Ok(())
    );
}

// --- Admiralsfähigkeiten und Markierung ---

#[test]
fn die_admiralsfaehigkeit_gibt_es_statt_des_grundmanoevers_nur_auf_ein_gueltiges_ziel() {
    let mut game = mid_game([AdmiralId::Vance, AdmiralId::Osei]);
    place(&mut game, Player, strike(0), S::HorizonCruiser);
    place(&mut game, Enemy, strike(1), S::HorizonCruiser);
    let focus = |target, maneuver| TurnPlan {
        power: power(target),
        maneuver,
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, focus(strike(1), maneuver(strike(0), strike(3)))),
        Err(ErrorCode::PowerAndManeuver)
    );
    assert_eq!(
        game.submit(FIRST, focus(strike(0), None)),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(game.submit(FIRST, focus(strike(1), None)), Ok(()));
}

#[test]
fn gegenfeuer_und_taktvorsprung_zielen_auf_eigene_schiffe_auch_gelegte_der_taktvorsprung_nur_auf_kampfschiffe()
 {
    let mut game = mid_game([AdmiralId::Kerr, AdmiralId::Osei]);
    game.player_mut(FIRST).ship_hand = vec![S::RepairDrone, S::VoidInterceptor];
    let deploy_and_power = |hand_index| TurnPlan {
        ships: vec![deploy(hand_index, strike(3))],
        power: power(strike(3)),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, deploy_and_power(0)),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(game.submit(FIRST, deploy_and_power(1)), Ok(()));

    let mut wall = mid_game(OSEI);
    place(&mut wall, Enemy, strike(3), S::HorizonCruiser);
    let counterfire = |target| TurnPlan {
        power: power(target),
        ..pass()
    };
    assert_eq!(
        wall.submit(FIRST, counterfire(strike(3))),
        Err(ErrorCode::PowerTargetInvalid)
    );
    place(&mut wall, Player, command(3), S::RepairDrone);
    assert_eq!(wall.submit(FIRST, counterfire(command(3))), Ok(()));
}

#[test]
fn den_schwarmangriff_fliegen_drohnen_die_schon_stehen_nie_auf_das_flaggschiff() {
    let mut game = mid_game([AdmiralId::Halvorsen, AdmiralId::Osei]);
    place(&mut game, Enemy, strike(1), S::HorizonCruiser);
    place(&mut game, Enemy, command(2), S::Bastion);
    let swarm = |target| TurnPlan {
        power: power(target),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, swarm(strike(1))),
        Err(ErrorCode::PowerTargetInvalid)
    );
    place(&mut game, Player, strike(0), S::Drone);
    assert_eq!(
        game.submit(FIRST, swarm(command(2))),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(game.submit(FIRST, swarm(strike(1))), Ok(()));
}

#[test]
fn das_trugbild_erscheint_nur_in_einem_freien_eigenen_strike_slot_in_dem_nichts_landet() {
    let mut game = mid_game([AdmiralId::Moreau, AdmiralId::Osei]);
    set_commands(&mut game, FIRST, &[C::Intercept]);
    place(&mut game, Player, strike(0), S::HorizonCruiser);
    place(&mut game, Player, command(3), S::HorizonCruiser);
    let decoy = |target, ships, command| TurnPlan {
        power: power(target),
        ships,
        command,
        ..pass()
    };
    let intercept = || order(0, Some(command(3)), Some(4));
    assert_eq!(
        game.submit(FIRST, decoy(strike(0), vec![], None)),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(
        game.submit(FIRST, decoy(command(1), vec![], None)),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(
        game.submit(FIRST, decoy(strike(2), vec![deploy(0, strike(2))], None)),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(
        game.submit(FIRST, decoy(strike(4), vec![], intercept())),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(
        game.submit(FIRST, decoy(strike(5), vec![], intercept())),
        Ok(())
    );
}

#[test]
fn mit_freier_hand_gibt_es_grundmanoever_und_admiralsfaehigkeit() {
    let mut game = mid_game([AdmiralId::Moreau, AdmiralId::Osei]);
    set_commands(&mut game, FIRST, &[C::CarteBlanche, C::Scavenge]);
    place(&mut game, Player, command(1), S::HorizonCruiser);
    let both = |hand_index, decoy| TurnPlan {
        command: order(hand_index, None, None),
        maneuver: maneuver(command(1), strike(1)),
        power: power(decoy),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, both(1, strike(3))),
        Err(ErrorCode::PowerAndManeuver)
    );
    // Das Trugbild erscheint nicht im Ziel-Slot des Manövers.
    assert_eq!(
        game.submit(FIRST, both(0, strike(1))),
        Err(ErrorCode::PowerTargetInvalid)
    );
    assert_eq!(game.submit(FIRST, both(0, strike(3))), Ok(()));
    assert_eq!(game.submit(SECOND, pass()), Ok(()));
    let kinds = kinds(&game.resolve_round());
    for kind in ["maneuver", "decoy", "rule"] {
        assert!(kinds.iter().any(|k| k == kind), "{kind} fehlt in {kinds:?}");
    }
}

#[test]
fn die_pendulum_markiert_nur_wenn_sie_auf_dem_brett_steht_und_ihre_railgun_nicht_feuert() {
    let mut game = mid_game([AdmiralId::Kerr, AdmiralId::Osei]);
    let target = place(&mut game, Enemy, strike(3), S::HorizonCruiser);
    let mark = |at| TurnPlan {
        mark: Some(at),
        ..pass()
    };
    assert_eq!(
        game.submit(FIRST, mark(strike(3))),
        Err(ErrorCode::MarkUnavailable)
    );
    place(&mut game, Player, command(2), S::Pendulum);
    assert_eq!(
        game.submit(FIRST, mark(strike(2))),
        Err(ErrorCode::MarkTargetInvalid)
    );
    game.board.ship_mut(target).marked = true;
    assert_eq!(
        game.submit(FIRST, mark(strike(3))),
        Err(ErrorCode::MarkUnavailable)
    );
    game.board.ship_mut(target).marked = false;
    assert_eq!(game.submit(FIRST, mark(strike(3))), Ok(()));
}

#[test]
fn faehigkeit_und_markierung_werden_aufgedeckt_und_fuer_den_zweiten_sitz_gespiegelt() {
    let mut game = mid_game([AdmiralId::Kerr, AdmiralId::Halvorsen]);
    place(&mut game, Player, command(2), S::Pendulum);
    place(&mut game, Player, strike(2), S::IronDreadnought);
    place(&mut game, Enemy, command(2), S::Hive);
    place(&mut game, Enemy, strike(4), S::TitanCarrier);
    game.board.hive_launches = Sides {
        player: 0,
        enemy: 3,
    };
    let head_start_and_mark = TurnPlan {
        power: power(strike(2)),
        mark: Some(strike(4)),
        ..pass()
    };
    let events = play_round(&mut game, [head_start_and_mark, pass()]);
    let head_start = |side| RoundEvent::Power {
        side,
        power: PowerId::HeadStart,
        target: slot(side, strike(2)),
    };
    assert!(events.contains(&head_start(Player)));
    assert!(
        game.events_for(SECOND, &events)
            .contains(&head_start(Enemy))
    );
    assert!(events.contains(&RoundEvent::Mark {
        side: Player,
        ship: ship(Enemy, strike(4), S::TitanCarrier),
    }));
    for marked in [
        game.view(FIRST).board.enemy.strike[4],
        game.view(SECOND).board.player.strike[4],
    ] {
        let marked = marked.unwrap();
        assert_eq!((marked.card_id, marked.marked), (S::TitanCarrier, true));
    }
    // Die Hive hat am Rundenende ihre vierte Drohne gestartet.
    assert_eq!(
        [
            game.view(FIRST).hive_launches,
            game.view(SECOND).hive_launches
        ],
        [
            Sides {
                player: 0,
                enemy: 4
            },
            Sides {
                player: 4,
                enemy: 0
            }
        ]
    );
    assert_eq!(game.outcome(), None);
}

// --- Feuerbefehle ---

#[test]
fn feuerbefehle_je_kampfschiff_einer_auch_fuer_ein_gelegtes_nicht_fuers_manoevrierende() {
    let mut game = mid_game([AdmiralId::Moreau, AdmiralId::Osei]);
    set_ship_hands(&mut game, [&[S::VoidInterceptor], &[]]);
    place(&mut game, Player, strike(1), S::HorizonCruiser);
    place(&mut game, Player, strike(2), S::RepairDrone);
    place(&mut game, Player, command(3), S::Mirage);
    place(&mut game, Enemy, strike(4), S::TitanCarrier);
    place(&mut game, Enemy, command(4), S::SignalBooster);
    let fire = |fire| TurnPlan { fire, ..pass() };

    assert_eq!(
        try_submit(
            &mut game,
            fire(vec![column(strike(1), 4), column(command(3), 4)])
        ),
        Ok(())
    );
    let deployed = TurnPlan {
        ships: vec![deploy(0, strike(0))],
        ..fire(vec![column(strike(0), 4)])
    };
    assert_eq!(try_submit(&mut game, deployed), Ok(()));
    for invalid in [
        vec![column(strike(2), 4)],
        vec![column(strike(0), 4)],
        vec![column(strike(1), 4), column(strike(1), 4)],
    ] {
        assert_eq!(
            try_submit(&mut game, fire(invalid)),
            Err(ErrorCode::FireShipInvalid)
        );
    }
    let maneuvering = TurnPlan {
        maneuver: maneuver(strike(1), strike(3)),
        ..fire(vec![column(strike(1), 4)])
    };
    assert_eq!(
        try_submit(&mut game, maneuvering),
        Err(ErrorCode::FireShipInvalid)
    );
    // Beim Phasentausch feuern beide.
    let swapping = TurnPlan {
        maneuver: maneuver(command(3), strike(1)),
        ..fire(vec![column(command(3), 4), column(strike(1), 4)])
    };
    assert_eq!(try_submit(&mut game, swapping), Ok(()));
    // Eine leere Spalte ist erlaubt, eine außerhalb des Bretts nicht (negative gibt das Protokoll nicht her).
    assert_eq!(
        try_submit(&mut game, fire(vec![column(strike(1), 3)])),
        Ok(())
    );
    assert_eq!(
        try_submit(&mut game, fire(vec![column(strike(1), 6)])),
        Err(ErrorCode::FireTargetInvalid)
    );
    // Schiffsfeuer: auf ein gegnerisches Schiff, auch ein gedecktes, nicht auf einen leeren Slot.
    assert_eq!(
        try_submit(
            &mut game,
            fire(vec![
                at_ship(strike(1), command(4)),
                at_ship(command(3), strike(4))
            ])
        ),
        Ok(())
    );
    assert_eq!(
        try_submit(&mut game, fire(vec![at_ship(strike(1), strike(3))])),
        Err(ErrorCode::FireTargetInvalid)
    );
}

// --- Sieg, Aufgabe, Overdrive ---

#[test]
fn wessen_flaggschiff_faellt_verliert_danach_gibt_es_keine_zuege_mehr() {
    let mut game = mid_game([AdmiralId::Vance, AdmiralId::Vance]);
    place_hp(&mut game, Player, strike(0), S::Vanguard, 3);
    place(&mut game, Enemy, strike(0), S::Vanguard);
    let duel = || TurnPlan {
        fire: vec![column(strike(0), 0)],
        ..pass()
    };
    play_round(&mut game, [duel(), duel()]);
    assert_eq!(game.view(FIRST).result, Some(MatchResult::Lost));
    assert_eq!(game.view(SECOND).result, Some(MatchResult::Won));
    assert_eq!(game.round(), 2);
    assert_eq!(game.submit(FIRST, pass()), Err(ErrorCode::MatchOver));
}

#[test]
fn fallen_beide_flaggschiffe_in_derselben_runde_endet_das_match_unentschieden() {
    let mut game = mid_game([AdmiralId::Vance, AdmiralId::Vance]);
    for side in Side::ALL {
        place_hp(&mut game, side, strike(0), S::Vanguard, 3);
    }
    let duel = || TurnPlan {
        fire: vec![column(strike(0), 0)],
        ..pass()
    };
    play_round(&mut game, [duel(), duel()]);
    assert_eq!(game.outcome(), Some(Outcome::Draw));
    assert_eq!(
        [game.view(FIRST).result, game.view(SECOND).result],
        [Some(MatchResult::Draw); 2]
    );
}

#[test]
fn solange_beide_flaggschiffe_stehen_geht_es_weiter_auch_ohne_andere_kampfschiffe() {
    let mut game = mid_game(OSEI);
    set_ship_hands(&mut game, [&[], &[]]);
    for side in Side::ALL {
        place(&mut game, side, command(2), S::Bastion);
    }
    play_passes(&mut game);
    assert_eq!(game.outcome(), None);
    assert_eq!(game.round(), 3);
}

#[test]
fn wer_aufgibt_verliert_sofort_auch_mit_abgegebenem_zug() {
    let mut game = new_match(OSEI);
    let opening = TurnPlan {
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(game.submit(FIRST, opening.clone()), Ok(()));
    assert_eq!(game.surrender(SECOND), Ok(()));
    let winner = game.view(FIRST);
    assert_eq!(
        (winner.result, winner.surrendered, winner.submitted_plan),
        (Some(MatchResult::Won), true, None)
    );
    let loser = game.view(SECOND);
    assert_eq!(
        (loser.result, loser.surrendered),
        (Some(MatchResult::Lost), true)
    );
    assert!(!game.is_round_complete());
    assert_eq!(game.submit(SECOND, opening), Err(ErrorCode::MatchOver));
    assert_eq!(game.surrender(FIRST), Err(ErrorCode::MatchOver));
}

#[test]
fn aufgeben_geht_erst_wenn_der_gegner_da_ist() {
    let mut game = Match::with_rng("test", seeded());
    game.join("a").unwrap();
    assert_eq!(game.surrender(FIRST), Err(ErrorCode::OpponentMissing));
}

#[test]
fn ab_runde_8_gilt_der_overdrive_schon_beim_planen_der_runde() {
    let mut game = mid_game(OSEI);
    game.round = 7;
    for side in Side::ALL {
        place(&mut game, side, command(2), S::Bastion);
    }
    play_passes(&mut game);
    assert_eq!(game.round(), 8);
    assert_eq!(game.board.overdrive, 1);
}

#[test]
fn der_gegner_sieht_admiral_handgroessen_decks_und_aufgedeckte_befehle_aber_keine_handkarten() {
    let mut alone = Match::with_rng("test", seeded());
    alone.join("a").unwrap();
    assert_eq!(alone.view(FIRST).opponent, None);

    let mut game = new_match([AdmiralId::Vance, AdmiralId::Osei]);
    set_commands(&mut game, FIRST, &[C::Scavenge, C::Torpedo]);
    let torpedo = TurnPlan {
        flagship: Some(command(2)),
        command: order(1, Some(strike(0)), None),
        ..pass()
    };
    assert_eq!(game.submit(FIRST, torpedo), Ok(()));
    // Bis zum Aufdecken ist nur bekannt, dass er abgegeben hat.
    let before = game.view(SECOND).opponent.unwrap();
    assert!(before.submitted);
    assert_eq!(before.command_hand_size, 2);
    assert!(before.played_commands.is_empty());
    let flagship = TurnPlan {
        flagship: Some(command(2)),
        ..pass()
    };
    assert_eq!(game.submit(SECOND, flagship), Ok(()));
    game.resolve_round();
    assert_eq!(game.draft(FIRST, FIRST_PICK), Ok(()));
    assert_eq!(
        game.view(SECOND).opponent,
        Some(OpponentView {
            admiral: Some(AdmiralId::Vance),
            submitted: false,
            ship_hand_size: 5,
            command_hand_size: 2,
            ship_deck_size: 11,
            command_deck_size: 30,
            bot: false,
            played_commands: vec![C::Torpedo],
        })
    );
}

// --- Nachschub ---

#[test]
fn nach_der_runde_gibt_es_3_schiffe_und_3_befehle_zur_wahl_erst_danach_geht_der_zug() {
    let mut game = ongoing();
    let decks = |game: &Match| {
        let player = game.player(FIRST);
        (player.ship_deck.len(), player.command_deck.len())
    };
    let hands = |game: &Match| {
        let player = game.player(FIRST);
        (player.ship_hand.len(), player.command_hand.len())
    };
    let (ship_deck, command_deck) = decks(&game);
    let (ship_hand, command_hand) = hands(&game);
    assert_eq!(
        game.draft(FIRST, FIRST_PICK),
        Err(ErrorCode::DraftUnavailable)
    );
    play_passes(&mut game);
    let offer = game.view(FIRST).draft.unwrap();
    assert_eq!((offer.ships.len(), offer.commands.len()), (3, 3));
    assert_eq!(decks(&game).0, ship_deck - 3);
    assert_eq!(game.submit(FIRST, pass()), Err(ErrorCode::DraftPending));
    for invalid in [(Some(3), Some(0)), (Some(0), None)] {
        let pick = DraftPick {
            ship: invalid.0,
            command: invalid.1,
        };
        assert_eq!(game.draft(FIRST, pick), Err(ErrorCode::DraftInvalid));
    }
    let pick = DraftPick {
        ship: Some(2),
        command: Some(1),
    };
    assert_eq!(game.draft(FIRST, pick), Ok(()));
    assert_eq!(game.view(FIRST).draft, None);
    let player = game.player(FIRST);
    assert_eq!(player.ship_hand.last(), Some(&offer.ships[2]));
    assert_eq!(player.command_hand.last(), Some(&offer.commands[1]));
    assert_eq!(hands(&game), (ship_hand + 1, command_hand + 1));
    assert_eq!(decks(&game), (ship_deck - 1, command_deck - 1));
    assert_eq!(
        game.draft(FIRST, FIRST_PICK),
        Err(ErrorCode::DraftUnavailable)
    );
    assert_eq!(game.submit(FIRST, pass()), Ok(()));
}

#[test]
fn eine_volle_handhaelfte_bekommt_keinen_nachschub_ein_leeres_deck_nur_was_es_noch_hat() {
    let mut game = ongoing();
    let player = game.player_mut(FIRST);
    player.command_hand.resize(HAND_LIMIT, C::Torpedo);
    while player.ship_deck.len() > 2 {
        player.ship_deck.draw();
    }
    play_passes(&mut game);
    let offer = game.view(FIRST).draft.unwrap();
    assert_eq!((offer.ships.len(), offer.commands.len()), (2, 0));
    assert_eq!(game.draft(FIRST, FIRST_PICK), Err(ErrorCode::DraftInvalid));
    let pick = DraftPick {
        ship: Some(1),
        command: None,
    };
    assert_eq!(game.draft(FIRST, pick), Ok(()));
}

#[test]
fn birgt_ein_scrap_raider_schrott_ist_ein_befehl_die_schrottverwertung_gespielt_gibt_sie_1_atk() {
    let mut game = ongoing();
    let raider = place(&mut game, Player, strike(0), S::ScrapRaider);
    let other = place(&mut game, Player, command(4), S::ScrapRaider);
    place_hp(&mut game, Enemy, strike(0), S::VoidInterceptor, 1);
    let shoot = TurnPlan {
        fire: vec![at_ship(strike(0), strike(0))],
        ..pass()
    };
    let events = play_round(&mut game, [shoot, pass()]);
    assert!(salvaged(&events));
    let offer = game.view(FIRST).draft.unwrap();
    assert_eq!(
        (offer.commands.len(), offer.commands[0]),
        (3, C::ScrapSalvage)
    );
    assert!(
        !game
            .view(SECOND)
            .draft
            .unwrap()
            .commands
            .contains(&C::ScrapSalvage)
    );
    let decks = game.player(FIRST).command_deck.len();
    let pick = DraftPick {
        ship: Some(0),
        command: Some(0),
    };
    assert_eq!(game.draft(FIRST, pick), Ok(()));
    // Sie wirkt erst gespielt; die übrigen Befehle gehen zurück ins Deck.
    let bonus = |game: &Match| {
        (
            game.board.ship(raider).scrap_bonus,
            game.board.ship(other).scrap_bonus,
        )
    };
    assert_eq!(
        game.player(FIRST).command_hand.last(),
        Some(&C::ScrapSalvage)
    );
    assert_eq!(bonus(&game), (0, 0));
    assert_eq!(game.player(FIRST).command_deck.len(), decks + 2);
    let hand_index = game.player(FIRST).command_hand.len() - 1;
    let salvage = |target| TurnPlan {
        command: order(hand_index, Some(target), None),
        ..pass()
    };
    // Nur ein Scrap Raider ist ein gültiges Ziel; egal welcher den Schrott geborgen hat.
    assert_eq!(
        game.submit(FIRST, salvage(command(2))),
        Err(ErrorCode::TargetWrongShip)
    );
    game.draft(SECOND, FIRST_PICK).unwrap();
    play_round(&mut game, [salvage(command(4)), pass()]);
    assert_eq!(bonus(&game), (0, 1));
    assert_eq!(game.board.attack(other), S::ScrapRaider.atk() + 1);
}

#[test]
fn bleibt_die_schrottverwertung_im_nachschub_liegen_verfaellt_sie() {
    let mut game = ongoing();
    place(&mut game, Player, strike(0), S::ScrapRaider);
    place_hp(&mut game, Enemy, strike(0), S::VoidInterceptor, 1);
    let shoot = TurnPlan {
        fire: vec![at_ship(strike(0), strike(0))],
        ..pass()
    };
    play_round(&mut game, [shoot, pass()]);
    let decks = game.player(FIRST).command_deck.len();
    let pick = DraftPick {
        ship: Some(0),
        command: Some(1),
    };
    assert_eq!(game.draft(FIRST, pick), Ok(()));
    let player = game.player(FIRST);
    assert_eq!(player.command_deck.len(), decks + 1);
    assert!(!player.command_hand.contains(&C::ScrapSalvage));
}

#[test]
fn faellt_der_raider_nach_dem_bergen_noch_in_derselben_runde_gibt_es_keine_schrottverwertung() {
    let mut game = ongoing();
    place_hp(&mut game, Player, strike(0), S::ScrapRaider, 1);
    place_hp(&mut game, Enemy, strike(0), S::VoidInterceptor, 1);
    set_commands(&mut game, FIRST, &[C::VolleyFire]);
    let volley = TurnPlan {
        command: order(0, None, None),
        ..pass()
    };
    let shoot = TurnPlan {
        fire: vec![at_ship(command(2), strike(0))],
        ..pass()
    };
    let events = play_round(&mut game, [volley, shoot]);
    assert!(salvaged(&events));
    assert!(events.contains(&RoundEvent::Destroyed {
        ship: ship(Player, strike(0), S::ScrapRaider),
    }));
    assert!(
        !game
            .view(FIRST)
            .draft
            .unwrap()
            .commands
            .contains(&C::ScrapSalvage)
    );
}
