//! Der Computergegner, portiert aus `server/src/bot.test.ts`.

use rand::SeedableRng;
use rand::rngs::StdRng;
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::board::LaneType::{Command, Strike};
use voidfleet_core::board::Side::{Enemy, Player};
use voidfleet_core::board::{Board, LaneType, Side, SlotRef, SlotTarget};
use voidfleet_core::cards::{COMMANDS, CommandId as C, CommandType, ShipId as S};
use voidfleet_core::protocol::{
    CommandPlan, Deploy, FireOrder, FireTarget, Maneuver, Miss, OpponentView, PowerPlan,
    RoundEvent, TurnPlan,
};

use super::*;
use crate::game::{Match, Outcome, Seat};

// --- Hilfen ---

/// Zufall mit festem Wert in [0, 1]: Bei 1 kommen die seltenen Überraschungen nie, bei 0 immer.
struct Chance(f64);

impl RngCore for Chance {
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_u64(&mut self) -> u64 {
        // `random::<f64>()` nimmt die oberen 53 Bit.
        if self.0 >= 1.0 {
            u64::MAX
        } else {
            ((self.0 * (1u64 << 53) as f64) as u64) << 11
        }
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        dst.fill(0);
    }
}

const FIRST: Seat = Seat::First;
const SECOND: Seat = Seat::Second;

const fn strike(index: usize) -> SlotTarget {
    SlotTarget::new(Strike, index)
}

const fn command(index: usize) -> SlotTarget {
    SlotTarget::new(Command, index)
}

fn place(board: &mut Board, side: Side, lane: LaneType, index: usize, card: S) -> SlotRef {
    let at = SlotRef::new(side, lane, index);
    board.place(at, card).expect("Slot ist frei");
    at
}

fn place_hp(board: &mut Board, side: Side, lane: LaneType, index: usize, card: S, hp: i32) {
    let at = place(board, side, lane, index, card);
    board.ship_mut(at).hp = hp;
}

/// Die Sicht des Bots in Runde 2 auf Seite [`Player`]. Halvorsen braucht für seine Fähigkeit eigene Drohnen; ohne sie
/// plant der Bot wie ohne Admiralsfähigkeit.
fn view_as(board: &Board, ships: &[S], commands: &[C], admiral: AdmiralId) -> BotView {
    BotView {
        board: board.clone(),
        round: 2,
        side: Player,
        admiral,
        ship_hand: ships.to_vec(),
        command_hand: commands.to_vec(),
        opponent: OpponentView {
            admiral: None,
            submitted: false,
            ship_hand_size: 0,
            command_hand_size: 0,
            ship_deck_size: 0,
            command_deck_size: 0,
            bot: false,
            played_commands: Vec::new(),
        },
    }
}

fn view(board: &Board, ships: &[S], commands: &[C]) -> BotView {
    view_as(board, ships, commands, AdmiralId::Halvorsen)
}

/// Ein Zug ohne die seltenen Überraschungen.
fn plan(view: &BotView) -> TurnPlan {
    plan_bot_turn(view, &mut Chance(1.0))
}

fn plan_with(view: &BotView, chance: f64) -> TurnPlan {
    plan_bot_turn(view, &mut Chance(chance))
}

/// Ein Match ab `round`, in dem Seat 0 nur die angegebenen Karten auf der Hand hat; der Gegner spielt Halvorsen.
fn match_with(ships: &[S], commands: &[C], round: u32, admiral: AdmiralId) -> Match {
    let mut game = Match::with_rng("bot", StdRng::seed_from_u64(1));
    game.join("a").unwrap();
    game.join("b").unwrap();
    game.choose_admiral(FIRST, admiral).unwrap();
    game.choose_admiral(SECOND, AdmiralId::Halvorsen).unwrap();
    game.set_round(round);
    game.set_hands(FIRST, ships, commands);
    game
}

fn match_in_round_2(ships: &[S], commands: &[C]) -> Match {
    match_with(ships, commands, 2, AdmiralId::Halvorsen)
}

fn plan_in(game: &Match) -> TurnPlan {
    plan_bot_turn(&game.bot_view(FIRST), &mut Chance(1.0))
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

/// Worauf die Feuerbefehle zielen: Spalte (Spaltenfeuer) oder Slot des Ziels beim Planen (Schiffsfeuer).
fn aims(plan: &TurnPlan) -> Vec<FireTarget> {
    plan.fire.iter().map(|order| order.target).collect()
}

/// Die Spalten der Feuerbefehle, aufsteigend; Schiffsfeuer fehlt.
fn columns(plan: &TurnPlan) -> Vec<usize> {
    let mut columns: Vec<usize> = plan
        .fire
        .iter()
        .filter_map(|order| match order.target {
            FireTarget::Column(column) => Some(column),
            FireTarget::Ship(_) => None,
        })
        .collect();
    columns.sort_unstable();
    columns
}

fn columns_of(indices: &[usize]) -> Vec<FireTarget> {
    indices.iter().copied().map(FireTarget::Column).collect()
}

fn pass() -> TurnPlan {
    TurnPlan::default()
}

// --- Bot gegen Bot ---

#[test]
fn bot_gegen_bot_ueber_300_partien_gilt_jeder_zug_und_jede_partie_endet() {
    // Der Overdrive lässt den Schaden steigen, bis Repair Drones nicht mehr mithalten.
    let mut stalemates = 0;
    for game_index in 0..300 {
        let mut rng = StdRng::seed_from_u64(game_index as u64);
        let mut game = Match::with_rng("bots", StdRng::seed_from_u64(1000 + game_index as u64));
        game.join("a").unwrap();
        game.join("b").unwrap();
        let first = ADMIRAL_IDS[game_index % ADMIRAL_IDS.len()];
        let second = ADMIRAL_IDS[game_index / ADMIRAL_IDS.len() % ADMIRAL_IDS.len()];
        game.choose_admiral(FIRST, first).unwrap();
        game.choose_admiral(SECOND, second).unwrap();
        while game.outcome().is_none() {
            if game.round() == 100 {
                stalemates += 1;
                break;
            }
            for seat in Seat::ALL {
                if let Some(offer) = game.view(seat).draft {
                    let pick = plan_bot_draft(&game.bot_view(seat), &offer);
                    assert_eq!(game.draft(seat, pick), Ok(()));
                }
                let plan = plan_bot_turn(&game.bot_view(seat), &mut rng);
                assert_eq!(game.submit(seat, plan.clone()), Ok(()), "{plan:?}");
            }
            game.resolve_round();
        }
    }
    assert_eq!(stalemates, 0);
}

// --- Schiffe ---

#[test]
fn auf_leerem_brett_kommt_das_staerkste_kampfschiff_nach_vorne_in_die_mitte() {
    let plan = plan(&view(
        &Board::new(),
        &[S::RepairDrone, S::VoidInterceptor, S::IronDreadnought],
        &[],
    ));
    assert_eq!(plan.ships, [deploy(2, strike(2))]);
    assert_eq!(plan.command, None);
}

#[test]
fn in_runde_1_kommt_das_flaggschiff_mittig_nach_hinten_das_robusteste_kampfschiff_davor() {
    let mut game = match_with(
        &[S::RepairDrone, S::VoidInterceptor, S::IronDreadnought],
        &[],
        1,
        AdmiralId::Halvorsen,
    );
    let plan = plan_in(&game);
    assert_eq!(plan.flagship, Some(command(2)));
    // Hinter dem Dreadnought ist kein Command-Slot mehr frei: Der Jäger kommt daneben nach vorne.
    assert_eq!(plan.ships, [deploy(2, strike(2)), deploy(1, strike(3))]);
    // Beim Planen steht noch kein gegnerisches Schiff: In Runde 1 feuert niemand.
    assert_eq!(plan.fire, []);
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn jaeger_und_support_kommen_hinter_ein_strike_schiff_robuste_nach_vorne() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    let lane = |ship: S| plan(&view(&board, &[ship], &[])).ships[0].target;
    assert_eq!(lane(S::VoidInterceptor).lane, Command);
    assert_eq!(lane(S::RepairDrone).lane, Command);
    assert_eq!(lane(S::HorizonCruiser), strike(3));
}

#[test]
fn ist_die_lane_voll_weicht_das_schiff_aus_ist_die_flotte_voll_legt_er_nichts() {
    let mut board = Board::new();
    for index in 0..4 {
        place(&mut board, Player, Strike, index, S::HorizonCruiser);
    }
    let cruiser = |board: &Board| plan(&view(board, &[S::HorizonCruiser], &[])).ships;
    assert_eq!(cruiser(&board)[0].target.lane, Command);
    for index in 0..2 {
        place(&mut board, Player, Command, index, S::HorizonCruiser);
    }
    assert_eq!(cruiser(&board), []);
}

#[test]
fn robuste_schiffe_stellen_sich_vor_ein_ungedecktes_command_schiff() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Player, Command, 4, S::RepairDrone);
    let plan = plan(&view(&board, &[S::HorizonCruiser], &[]));
    assert_eq!(plan.ships[0].target, strike(4));
}

#[test]
fn ein_neues_schiff_stellt_sich_vor_das_command_schiff_auf_das_der_gegner_wohl_feuert() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Player, Command, 2, S::Hive);
    place(&mut board, Player, Command, 1, S::SignalBooster);
    place_hp(&mut board, Player, Command, 4, S::RepairDrone, 3);
    place(&mut board, Enemy, Strike, 4, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 5, S::HorizonCruiser);
    // Der Gegner zerstört die angeschlagene Repair Drone; der Kreuzer davor nimmt die Schüsse auf ihre Spalte.
    let plan = plan(&view(&board, &[S::HorizonCruiser], &[]));
    assert_eq!(plan.ships, [deploy(0, strike(4))]);
}

// --- Befehle ---

#[test]
fn ohne_gegner_auf_dem_brett_zieht_der_bot_nach_statt_ins_leere_zu_schiessen() {
    let plan = plan(&view(
        &Board::new(),
        &[S::VoidInterceptor],
        &[C::OrbitalStrike, C::Scavenge],
    ));
    assert_eq!(plan.command, order(1, None, None));
}

#[test]
fn ein_angriff_der_einen_gegner_zerstoert_geht_vor() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place_hp(&mut board, Enemy, Strike, 2, S::HorizonCruiser, 4);
    let plan = plan(&view(&board, &[], &[C::Scavenge, C::OrbitalStrike]));
    assert_eq!(plan.command, order(1, None, None));
}

#[test]
fn der_schild_geht_an_das_schwaechste_ungedeckte_schiff() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::IronDreadnought);
    place(&mut board, Player, Strike, 3, S::HorizonCruiser);
    let fighter = place(&mut board, Player, Command, 1, S::VoidInterceptor);
    place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
    // Der Jäger steht gedeckt hinter dem Dreadnought.
    let shields = |board: &Board| plan(&view(board, &[], &[C::EmergencyShields])).command;
    assert_eq!(shields(&board), order(0, Some(strike(3)), None));
    // Ungedeckt ist er das schwächste Ziel.
    board[SlotRef::new(Player, Command, 2)] = board[fighter].take();
    assert_eq!(shields(&board), order(0, Some(command(2)), None));
}

#[test]
fn intercept_holt_aus_der_command_lane_recall_aus_der_strike_lane_wenn_vorne_jemand_bleibt() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Strike, 1, S::HorizonCruiser, 3);
    place(&mut board, Player, Command, 4, S::VoidInterceptor);
    place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
    assert_eq!(
        plan(&view(&board, &[], &[C::Intercept])).command,
        order(0, Some(command(4)), Some(4))
    );
    // Allein vorne: Recall würde die Command-Lane freilegen.
    assert_eq!(plan(&view(&board, &[], &[C::Recall])).command, None);
    // Mit einem neuen Schiff vorne darf das angeschlagene zurück.
    let plan = plan(&view(&board, &[S::HorizonCruiser], &[C::Recall]));
    assert_eq!(plan.ships[0].target.lane, Strike);
    assert_eq!(plan.command, order(0, Some(strike(1)), Some(1)));
}

#[test]
fn ein_schiff_rueckt_vor_wenn_es_damit_ein_ungedecktes_schiff_deckt() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 3, S::IronDreadnought);
    place(&mut board, Player, Command, 2, S::RepairDrone);
    place(&mut board, Player, Command, 3, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    // Der Kreuzer landet in Strike-Slot 2 und deckt damit die Drohne; die Drohne selbst würde nur vor ihren leeren
    // Slot rücken.
    assert_eq!(
        plan(&view(&board, &[], &[C::FlankingManeuver])).command,
        order(0, Some(command(3)), Some(2))
    );
}

#[test]
fn befehle_ohne_nutzen_bleiben_auf_der_hand() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 2, S::VoidInterceptor);
    place(&mut board, Enemy, Command, 2, S::RepairDrone);
    // Power Surge ohne Schiff vorne, Jammer ohne gegnerische Strike-Schiffe, EMP gegen ein Schiff ohne ATK.
    let commands = [C::PowerSurge, C::TargetingJammer, C::EmpBlast];
    assert_eq!(plan(&view(&board, &[], &commands)).command, None);
}

#[test]
fn der_torpedo_zielt_auf_den_slot_der_am_meisten_verliert_auch_hinter_deckung() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Enemy, Strike, 1, S::TitanCarrier);
    place_hp(&mut board, Enemy, Command, 1, S::PlasmaFrigate, 5);
    assert_eq!(
        plan(&view(&board, &[], &[C::Torpedo])).command,
        order(0, Some(command(1)), None)
    );
}

#[test]
fn der_torpedo_zielt_auf_ein_schiff_das_stehen_bleibt_selten_auf_den_slot_eines_ausweichenden() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Player, Strike, 3, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Strike, 2, S::PlasmaFrigate, 3);
    place(&mut board, Enemy, Strike, 3, S::HorizonCruiser);
    // Die angeschlagene Fregatte fällt sonst im Kampf und weicht eher aus; der unbeschädigte Kreuzer bleibt. Ihr Schuss
    // zerstört kein eigenes Schiff, es lohnt also nicht, sie vor dem Kampf zu versenken.
    let torpedo = view(&board, &[], &[C::Torpedo]);
    assert_eq!(plan(&torpedo).command, order(0, Some(strike(3)), None));
    // Ab und zu zielt er auf den gedeckten Slot hinter dem Kreuzer, in den sich die Fregatte zurückziehen würde.
    assert_eq!(
        plan_with(&torpedo, 0.0).command,
        order(0, Some(command(3)), None)
    );
}

#[test]
fn sperrfeuer_lohnt_wenn_ein_angeschlagener_gegner_sonst_faellt_und_eher_ausweicht() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    let enemy = place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
    let choice = |board: &Board| {
        plan(&view(board, &[], &[C::Scavenge, C::Barrage]))
            .command
            .map(|command| command.hand_index)
    };
    // Gegen ein unbeschädigtes Schiff hat es nur einen kleinen Grundwert; Nachziehen ist mehr wert.
    assert_eq!(choice(&board), Some(0));
    board.ship_mut(enemy).hp = 2;
    assert_eq!(choice(&board), Some(1));
}

#[test]
fn overcharge_und_schild_gehen_auch_an_ein_kampfschiff_das_gerade_vorne_landet() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 2, S::VoidInterceptor);
    place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
    // Der Dreadnought deckt den Jäger und übersteht Overcharge samt Treffer.
    let overcharge = plan(&view(&board, &[S::IronDreadnought], &[C::Overcharge]));
    assert_eq!(overcharge.ships, [deploy(0, strike(2))]);
    assert_eq!(overcharge.command, order(0, Some(strike(2)), None));
    // Den Kreuzer kann der Gegner in dieser Runde nicht beschießen, aber er fängt den Schuss auf den Jäger dahinter ab.
    assert_eq!(
        plan(&view(&board, &[S::HorizonCruiser], &[C::EmergencyShields])).command,
        order(0, Some(strike(2)), None)
    );
    // Ohne Ziel für den Gegner ist der Schild auf dem frischen Schiff nichts wert.
    let mut empty = Board::new();
    place(&mut empty, Enemy, Strike, 2, S::PlasmaFrigate);
    assert_eq!(
        plan(&view(&empty, &[S::HorizonCruiser], &[C::EmergencyShields])).command,
        None
    );
}

#[test]
fn intercept_holt_einen_frisch_in_die_command_lane_gelegten_jaeger_sofort_nach_vorne() {
    let mut game = match_in_round_2(&[S::VoidInterceptor], &[C::Intercept]);
    place(game.board_mut(), Player, Strike, 2, S::IronDreadnought);
    place(game.board_mut(), Enemy, Strike, 2, S::HorizonCruiser);
    let plan = plan_in(&game);
    assert_eq!(plan.ships, [deploy(0, command(2))]);
    assert_eq!(plan.command, order(0, Some(command(2)), Some(1)));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn der_kurzsprung_rettet_das_flaggschiff_wenn_kein_notschild_reicht_sonst_der_schild() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Strike, 2, S::Hive, 3);
    place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
    let commands = [C::MicroJump, C::EmergencyShields];
    assert_eq!(
        plan(&view(&board, &[], &commands)).command,
        order(1, Some(strike(2)), None)
    );
    // Hält der Gegner noch Befehle, droht zusätzlich sein Torpedo: Gegen 3 + 5 Schaden hilft nur der Sprung.
    let mut armed = view(&board, &[], &commands);
    armed.opponent.command_hand_size = 3;
    assert_eq!(plan(&armed).command, order(0, Some(strike(2)), None));
}

#[test]
fn notschild_statt_kurzsprung_auch_fuer_ein_gedecktes_flaggschiff_wenn_overkill_durchgeht() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Command, 2, S::Hive, 5);
    place_hp(&mut board, Player, Strike, 2, S::VoidInterceptor, 2);
    for index in [0, 1, 3, 4] {
        place(&mut board, Enemy, Strike, index, S::HorizonCruiser);
    }
    // Schlimmstenfalls nimmt der Interceptor 2 der 8 Schaden, 6 gehen durch; mit dem Schild bleiben der Hive 2 HP.
    let plan = plan(&view(&board, &[], &[C::MicroJump, C::EmergencyShields]));
    assert_eq!(plan.command, order(1, Some(command(2)), None));
}

#[test]
fn mit_schnellverlegung_legt_der_bot_zwei_schiffe() {
    let mut game = match_in_round_2(
        &[S::HorizonCruiser, S::IronDreadnought],
        &[C::RapidDeployment],
    );
    place(game.board_mut(), Enemy, Strike, 2, S::HorizonCruiser);
    let plan = plan_in(&game);
    assert_eq!(plan.command, order(0, None, None));
    let mut hand: Vec<usize> = plan.ships.iter().map(|ship| ship.hand_index).collect();
    hand.sort_unstable();
    assert_eq!(hand, [0, 1]);
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn mit_freier_hand_plant_der_bot_grundmanoever_und_admiralsfaehigkeit() {
    let mut game = match_with(&[], &[C::CarteBlanche], 2, AdmiralId::Vance);
    let board = game.board_mut();
    place(board, Player, Strike, 0, S::HorizonCruiser);
    place(board, Player, Strike, 4, S::HorizonCruiser);
    place(board, Player, Command, 1, S::RepairDrone);
    place(board, Enemy, Strike, 0, S::TitanCarrier);
    place(board, Enemy, Strike, 2, S::ScrapRaider);
    let plan = plan_in(&game);
    assert_eq!(plan.command, order(0, None, None));
    // Die Drohne geht hinter einen Kreuzer in Deckung, das Fokusfeuer liegt auf dem Scrap Raider.
    assert_eq!(
        plan.maneuver.map(|maneuver| maneuver.ship),
        Some(command(1))
    );
    assert_eq!(plan.power, power(strike(2)));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn rammkurs_mit_einem_grossen_schiff_gegen_ein_kleineres_nie_umgekehrt() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    assert_eq!(
        plan(&view(&board, &[], &[C::CollisionCourse])).command,
        order(0, Some(strike(2)), None)
    );
    let mut small = Board::new();
    place(&mut small, Player, Strike, 2, S::ScrapRaider);
    place(&mut small, Enemy, Strike, 2, S::IronDreadnought);
    assert_eq!(
        plan(&view(&small, &[], &[C::CollisionCourse])).command,
        None
    );
}

#[test]
fn der_traktorstrahl_zieht_den_waechter_vor_dem_gegnerischen_flaggschiff_weg() {
    let mut game = match_in_round_2(&[], &[C::TractorBeam]);
    let board = game.board_mut();
    for index in [1, 2, 3] {
        place(board, Player, Strike, index, S::HorizonCruiser);
    }
    place(board, Player, Command, 2, S::Hive);
    place(board, Enemy, Command, 2, S::Hive);
    place(board, Enemy, Strike, 2, S::HorizonCruiser);
    place(board, Enemy, Strike, 4, S::ScrapRaider);
    let plan = plan_in(&game);
    assert_eq!(plan.command, order(0, Some(strike(2)), None));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn gegenmassnahmen_solange_der_gegner_viele_angriffe_uebrig_hat_sonst_lieber_ziehen() {
    let mut armed = view(&Board::new(), &[], &[C::Scavenge, C::Countermeasures]);
    armed.opponent.command_hand_size = 6;
    assert_eq!(
        plan(&armed).command.map(|command| command.hand_index),
        Some(1)
    );
    let mut spent = armed.clone();
    spent.opponent.played_commands = COMMANDS
        .into_iter()
        .filter(|card| {
            matches!(
                card.command_type(),
                CommandType::Attack | CommandType::Debuff
            )
        })
        .flat_map(|card| std::iter::repeat_n(card, card.deck_copies()))
        .collect();
    assert_eq!(
        plan(&spent).command.map(|command| command.hand_index),
        Some(0)
    );
}

#[test]
fn gegen_ungepaarte_trifft_kreuzfeuer_gegen_die_command_lane_das_bombardement() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Command, 1, S::RepairDrone, 2);
    place_hp(&mut board, Enemy, Command, 3, S::RepairDrone, 2);
    // Bombardement zerstört beide Drohnen hinten; Kreuzfeuer trifft nur die ungepaarte in Slot 3.
    let plan = plan(&view(&board, &[], &[C::Crossfire, C::Bombardment]));
    assert_eq!(plan.command.map(|command| command.hand_index), Some(1));
}

// --- Manöver und Admiralsfähigkeit ---

#[test]
fn ein_angeschlagenes_strike_schiff_das_sonst_faellt_zieht_sich_hinter_ein_anderes_zurueck() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Strike, 2, S::PlasmaFrigate, 2);
    place(&mut board, Player, Strike, 3, S::IronDreadnought);
    let enemy = place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    let retreat = plan(&view(&board, &[], &[]));
    assert_eq!(retreat.maneuver, maneuver(strike(2), command(3)));
    // Wer manövriert, feuert nicht und bekommt keinen Feuerbefehl.
    assert_eq!(retreat.fire, [column(strike(3), 2)]);
    // Zerstört sein Schuss den Gegner, ist er mehr wert als die 2 HP: Die Fregatte bleibt und tauscht.
    board.ship_mut(enemy).hp = 3;
    assert_eq!(plan(&view(&board, &[], &[])).maneuver, None);
}

#[test]
fn ein_ungedecktes_support_schiff_hinten_rueckt_innerhalb_der_lane_hinter_ein_strike_schiff() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Player, Command, 0, S::RepairDrone);
    place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    assert_eq!(
        plan(&view(&board, &[], &[])).maneuver,
        maneuver(command(0), command(2))
    );
    // Ist es schon gedeckt, bleibt es stehen.
    let mut covered = Board::new();
    place(&mut covered, Player, Strike, 2, S::IronDreadnought);
    place(&mut covered, Player, Command, 2, S::RepairDrone);
    place(&mut covered, Enemy, Strike, 2, S::VoidInterceptor);
    assert_eq!(plan(&view(&covered, &[], &[])).maneuver, None);
}

#[test]
fn das_einzige_angriffsfaehige_schiff_manoevriert_nicht() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Strike, 2, S::PlasmaFrigate, 2);
    place(&mut board, Player, Strike, 3, S::RepairDrone);
    place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    // Rückzug nur, wenn vorne ein Angreifer bleibt; die Drohne zählt nicht.
    assert_eq!(plan(&view(&board, &[], &[])).maneuver, None);

    let mut alone = Board::new();
    place(&mut alone, Player, Strike, 2, S::PlasmaFrigate);
    place(&mut alone, Enemy, Strike, 2, S::VoidInterceptor);
    // Auch dem vermuteten Torpedo weicht es nicht aus, wenn sonst niemand feuert.
    assert_eq!(plan_with(&view(&alone, &[], &[]), 0.0).maneuver, None);
}

#[test]
fn manoever_und_bewegungsbefehl_verlegen_verschiedene_schiffe_in_verschiedene_slots() {
    let mut game = match_in_round_2(&[], &[C::Recall]);
    let board = game.board_mut();
    place_hp(board, Player, Strike, 1, S::PlasmaFrigate, 2);
    place_hp(board, Player, Strike, 2, S::PlasmaFrigate, 2);
    place(board, Player, Strike, 3, S::IronDreadnought);
    place(board, Enemy, Strike, 1, S::VoidInterceptor);
    place(board, Enemy, Strike, 2, S::VoidInterceptor);
    let plan = plan_in(&game);
    // Recall holt die erste Fregatte hinter die zweite; die zieht sich hinter den Dreadnought zurück, weil Command-Slot 2
    // schon vergeben und Slot 1 nach dem Recall ungedeckt ist.
    assert_eq!(plan.command, order(0, Some(strike(1)), Some(2)));
    assert_eq!(plan.maneuver, maneuver(strike(2), command(3)));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn kerr_markiert_mit_der_pendulum_aber_nicht_in_der_runde_in_der_ihre_railgun_feuert() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 2, S::Pendulum);
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    let target = place(&mut board, Enemy, Strike, 3, S::HorizonCruiser);
    let mark = |board: &Board| plan(&view_as(board, &[], &[], AdmiralId::Kerr)).mark;
    assert_eq!(mark(&board), Some(strike(3)));
    board.ship_mut(target).marked = true;
    assert_eq!(mark(&board), None);
}

#[test]
fn vance_setzt_fokusfeuer_statt_eines_manoevers_wenn_es_ein_schiff_zerstoert() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 0, S::HorizonCruiser);
    place(&mut board, Player, Strike, 4, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 0, S::TitanCarrier);
    place(&mut board, Enemy, Strike, 2, S::ScrapRaider);
    let plan = plan(&view_as(&board, &[], &[], AdmiralId::Vance));
    assert_eq!((plan.power, plan.maneuver), (power(strike(2)), None));
}

#[test]
fn das_flaggschiff_geht_in_deckung_wenn_es_ungedeckt_unter_beschuss_steht() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 0, S::Bastion);
    place(&mut board, Player, Strike, 3, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 1, S::PlasmaFrigate);
    // Hinter dem Kreuzer fängt er die Schüsse auf die Bastion ab; das bringt mehr als Gegenfeuer.
    let plan = plan(&view_as(&board, &[], &[], AdmiralId::Osei));
    assert_eq!(plan.maneuver, maneuver(command(0), command(3)));
}

#[test]
fn die_mirage_tauscht_ungedeckt_unter_beschuss_mit_einem_robusten_schiff_in_deckung() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::Mirage);
    place(&mut board, Player, Strike, 2, S::IronDreadnought);
    place(&mut board, Player, Command, 2, S::TitanCarrier);
    place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
    let plan = plan(&view_as(&board, &[], &[], AdmiralId::Moreau));
    assert_eq!(plan.maneuver, maneuver(strike(1), command(2)));
    // Beim Phasentausch feuern beide.
    let shooters: Vec<SlotTarget> = plan.fire.iter().map(|order| order.ship).collect();
    assert!(shooters.contains(&strike(1)) && shooters.contains(&command(2)));
}

#[test]
fn ein_markiertes_schiff_das_die_railgun_zerstoeren_wuerde_geht_in_deckung() {
    let mut board = Board::new();
    let marked = place(&mut board, Player, Command, 0, S::PlasmaFrigate);
    board.ship_mut(marked).marked = true;
    place(&mut board, Player, Strike, 3, S::IronDreadnought);
    place(&mut board, Enemy, Command, 2, S::Pendulum);
    let plan = plan(&view_as(&board, &[], &[], AdmiralId::Osei));
    assert_eq!(plan.maneuver, maneuver(command(0), command(3)));
}

#[test]
fn droht_dem_flaggschiff_der_fall_weicht_es_in_eine_unbeschossene_spalte_aus() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Command, 2, S::Hive, 4);
    place_hp(&mut board, Player, Strike, 2, S::VoidInterceptor, 2);
    place(&mut board, Player, Strike, 4, S::IronDreadnought);
    for index in [1, 2, 3] {
        place(&mut board, Enemy, Strike, index, S::HorizonCruiser);
    }
    // Der Gegner zielt mit 6 Schaden auf die Spalte der Hive: Der Interceptor davor fällt, der Rest geht durch. Hinter
    // dem Dreadnought schießt niemand hin; der Interceptor vor ihr weicht nie aus, sonst stünde sie offen.
    assert_eq!(
        plan(&view(&board, &[], &[])).maneuver,
        maneuver(command(2), command(4))
    );
}

#[test]
fn unter_gleich_guten_slots_zum_ausweichen_entscheidet_der_zufall() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Command, 2, S::Hive, 4);
    place(&mut board, Player, Strike, 0, S::IronDreadnought);
    place(&mut board, Player, Strike, 4, S::IronDreadnought);
    place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
    place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
    // Hinter beiden Dreadnoughts ist die Hive gleich sicher. (Unter 0,2 bliebe sie, weil es nicht um das Match geht,
    // manchmal stehen.)
    let destination = |chance: f64| {
        plan_with(&view(&board, &[], &[]), chance)
            .maneuver
            .map(|maneuver| maneuver.destination)
    };
    assert_eq!(destination(0.25), Some(command(0)));
    assert_eq!(destination(0.99), Some(command(4)));
}

#[test]
fn gegen_schiffsfeuer_weicht_ein_schiff_nicht_ungedeckt_aus_hinter_ein_neues_schiff_schon() {
    let mut board = Board::new();
    place_hp(&mut board, Player, Strike, 1, S::PlasmaFrigate, 2);
    place(&mut board, Player, Strike, 4, S::VoidInterceptor);
    place(&mut board, Player, Command, 4, S::SignalBooster);
    place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
    // Ohne Schiff auf der Hand findet die Fregatte keine Deckung: Der Gegner zielt per Schiffsfeuer auf sie, und
    // ausweichen brächte nichts.
    assert_eq!(plan(&view(&board, &[], &[])).maneuver, None);
    // Der neue Kreuzer landet vorne mittig; hinter ihm ist die Fregatte vor beiden Modi sicher.
    let plan = plan(&view(&board, &[S::HorizonCruiser], &[]));
    assert_eq!(plan.ships, [deploy(0, strike(2))]);
    assert_eq!(plan.maneuver, maneuver(strike(1), command(2)));
}

// --- Feuerbefehle ---

#[test]
fn faellt_das_gegnerische_flaggschiff_zielen_so_viele_wie_noetig_die_uebrigen_auf_sein_ausweichen()
{
    let mut game = match_in_round_2(&[], &[]);
    let board = game.board_mut();
    for index in [1, 2, 3] {
        place(board, Player, Strike, index, S::HorizonCruiser);
    }
    place(board, Player, Strike, 4, S::VoidInterceptor);
    place(board, Player, Command, 3, S::Hive);
    place_hp(board, Enemy, Command, 2, S::Hive, 4);
    place(board, Enemy, Strike, 0, S::Drone);
    let plan = plan_in(&game);
    // Interceptor und Hive reichen für die 4 HP. Die Kreuzer fällen sie auch hinter der Drohne, wohin sie gedeckt am
    // ehesten auswiche, per Spaltenfeuer durch die Drohne: Der erste verschenkt 1 an ihr, dann braucht die Hive beide
    // anderen.
    assert_eq!(aims(&plan), columns_of(&[0, 0, 0, 2, 2]));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
    assert_eq!(game.submit(SECOND, pass()), Ok(()));
    let destroyed: Vec<S> = game
        .resolve_round()
        .into_iter()
        .filter_map(|event| match event {
            RoundEvent::Destroyed { ship } => Some(ship.card_id),
            _ => None,
        })
        .collect();
    assert_eq!(destroyed.len(), 2);
    assert!(destroyed.contains(&S::Drone) && destroyed.contains(&S::Hive));
}

#[test]
fn er_verteilt_das_feuer_statt_schaden_zu_verschenken() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::HorizonCruiser);
    place(&mut board, Player, Strike, 2, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Strike, 1, S::VoidInterceptor, 2);
    place_hp(&mut board, Enemy, Strike, 3, S::VoidInterceptor, 2);
    // Jeder Kreuzer reicht für einen Jäger; ein zweiter Schuss auf denselben verpuffte.
    assert_eq!(columns(&plan(&view(&board, &[], &[]))), [1, 3]);
}

#[test]
fn feuerbefehle_gibt_es_fuer_eigene_kampfschiffe_auch_ein_frisch_gelegtes() {
    let mut game = match_in_round_2(&[S::VoidInterceptor], &[]);
    let board = game.board_mut();
    place(board, Player, Strike, 2, S::IronDreadnought);
    place(board, Player, Command, 2, S::RepairDrone);
    place(board, Enemy, Strike, 2, S::HorizonCruiser);
    let plan = plan_in(&game);
    // Die Repair Drone greift nie an; Dreadnought und der gelegte Jäger feuern auf den Kreuzer. Der Gegner kann noch ein
    // Schiff legen, hinter das er ausweichen könnte: Spaltenfeuer, denn Schiffsfeuer brächte nichts mehr.
    assert_eq!(plan.ships, [deploy(0, strike(3))]);
    assert_eq!(plan.fire, [column(strike(2), 2), column(strike(3), 2)]);
    assert_eq!(game.submit(FIRST, plan), Ok(()));
}

#[test]
fn ein_schuss_auf_eine_spalte_trifft_das_schiff_vorne() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::VoidInterceptor);
    place(&mut board, Player, Strike, 2, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Command, 2, S::Hive, 2);
    place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Strike, 4, S::VoidInterceptor, 3);
    // Die Hive fiele mit einem Schuss, doch erst nimmt der Kreuzer davor alle 5 Schaden: Der Jäger schießt den
    // Interceptor ab, per Schiffsfeuer, das ihm folgt, wenn er ausweicht (Deckung findet er nirgends); der eigene
    // Kreuzer beschädigt den gegnerischen.
    assert_eq!(
        plan(&view(&board, &[], &[])).fire,
        [at_ship(strike(1), strike(4)), column(strike(2), 2)]
    );
}

#[test]
fn der_torpedo_raeumt_die_deckung_vor_dem_flaggschiff_weg_wenn_das_feuer_es_danach_zerstoert() {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::HorizonCruiser);
    place(&mut board, Player, Strike, 2, S::VoidInterceptor);
    place(&mut board, Player, Strike, 3, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Command, 2, S::Hive, 6);
    place_hp(&mut board, Enemy, Strike, 2, S::HorizonCruiser, 5);
    let plan = plan(&view(&board, &[], &[C::Torpedo]));
    assert_eq!(plan.command, order(0, Some(strike(2)), None));
    // Ohne den Kreuzer davor reichen 2 + 3 + 2 für die 6 HP. Ausweichen kann die Hive nur ungedeckt: Alle zielen per
    // Schiffsfeuer auf sie; der Kreuzer davor deckt nicht mehr, wenn sie schießen.
    assert_eq!(aims(&plan), [FireTarget::Ship(command(2)); 3]);
}

#[test]
fn kerr_gibt_taktvorsprung_dem_schiff_das_einen_gegnerischen_schuetzen_vorher_abschiesst() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 2, S::Pendulum);
    place_hp(&mut board, Player, Strike, 2, S::VoidInterceptor, 3);
    place(&mut board, Player, Strike, 3, S::HorizonCruiser);
    place_hp(&mut board, Enemy, Strike, 2, S::VoidInterceptor, 3);
    let plan = plan(&view_as(&board, &[], &[], AdmiralId::Kerr));
    assert_eq!(plan.power, power(strike(2)));
    assert!(plan.fire.contains(&at_ship(strike(2), strike(2))));
}

#[test]
fn drohnen_im_schwarmangriff_zielen_auch_per_feuerbefehl_auf_die_spalte_seines_ziels() {
    let mut board = Board::new();
    place(&mut board, Player, Command, 2, S::Hive);
    // Der Titan Carrier vor der Hive nimmt die gegnerischen Schüsse auf ihre Spalte; sie muss nicht ausweichen.
    place(&mut board, Player, Strike, 2, S::TitanCarrier);
    place(&mut board, Player, Strike, 1, S::Drone);
    place(&mut board, Player, Strike, 3, S::Drone);
    place(&mut board, Enemy, Strike, 2, S::IronDreadnought);
    place_hp(&mut board, Enemy, Command, 2, S::ScrapRaider, 3);
    // Hinter dem Dreadnought erreicht nur der Schwarmangriff den angeschlagenen Raider: 2 × (1 + 1) Schaden.
    let plan = plan(&view(&board, &[], &[]));
    assert_eq!(plan.power, power(command(2)));
    for index in [1, 3] {
        assert!(plan.fire.contains(&column(strike(index), 2)));
    }
}

#[test]
fn durchbruch_reicht_das_feuer_fuer_waechter_und_flaggschiff_zielen_alle_auf_die_spalte() {
    let mut game = match_in_round_2(&[], &[]);
    let board = game.board_mut();
    place(board, Player, Strike, 1, S::VoidInterceptor);
    place(board, Player, Strike, 3, S::HorizonCruiser);
    place(board, Player, Strike, 4, S::HorizonCruiser);
    place(board, Player, Command, 4, S::Hive);
    place_hp(board, Enemy, Strike, 2, S::ScrapRaider, 2);
    place_hp(board, Enemy, Command, 2, S::Hive, 3);
    let plan = plan_in(&game);
    // Der Interceptor feuert zuerst und verschenkt 1 am Raider, dann braucht die Hive beide Kreuzer; die eigene Hive
    // zielt zur Sicherheit auch dorthin. Der Gegner kann noch ein Schiff legen, hinter das die Hive ausweichen könnte:
    // Spaltenfeuer.
    assert_eq!(aims(&plan), columns_of(&[2, 2, 2, 2]));
    assert_eq!(game.submit(FIRST, plan), Ok(()));
    assert_eq!(game.submit(SECOND, pass()), Ok(()));
    let shots: Vec<(Option<S>, bool)> = game
        .resolve_round()
        .into_iter()
        .filter_map(|event| match event {
            RoundEvent::Attack {
                target, through, ..
            } => Some((Some(target.card_id), through)),
            RoundEvent::Fizzle {
                miss: Miss::Column { cleared, .. },
                ..
            } => Some((None, cleared)),
            _ => None,
        })
        .collect();
    assert_eq!(
        shots,
        [
            (Some(S::ScrapRaider), false),
            (Some(S::Hive), true),
            (Some(S::Hive), true),
            (None, true),
        ]
    );
    assert_eq!(game.outcome(), Some(Outcome::Won(Player)));
}

#[test]
fn vorhalten_reicht_das_feuer_nur_fuer_eine_stelle_setzt_er_manchmal_auf_das_ausweichen() {
    let mut board = Board::new();
    for index in [1, 2, 3] {
        place(&mut board, Player, Strike, index, S::HorizonCruiser);
    }
    place_hp(&mut board, Enemy, Command, 2, S::Hive, 4);
    place(&mut board, Enemy, Strike, 0, S::Drone);
    // Zwei Kreuzer reichen für die Hive, wo sie steht; hinter der Drohne braucht es alle drei, weil der erste 1 an ihr
    // verschenkt. Sonst schießt der dritte die Drohne ab.
    let fire = |chance: f64| columns(&plan_with(&view(&board, &[], &[]), chance));
    assert_eq!(fire(1.0), [0, 2, 2]);
    assert_eq!(fire(0.3), [0, 0, 0]);
}

#[test]
fn kann_das_gegnerische_flaggschiff_nur_ungedeckt_ausweichen_folgt_ihm_schiffsfeuer() {
    let mut game = match_in_round_2(&[], &[]);
    let board = game.board_mut();
    for index in [1, 2, 3, 4] {
        place(board, Player, Strike, index, S::HorizonCruiser);
    }
    place(board, Player, Command, 2, S::Hive);
    place_hp(board, Enemy, Command, 2, S::Hive, 4);
    // Ohne Schiff auf der Hand schafft der Gegner keine Deckung mehr. Zwei Kreuzer reichen, die übrigen Schützen zielen
    // zur Sicherheit auch auf die Hive.
    let commands = game.view(SECOND).command_hand;
    game.set_hands(SECOND, &[], &commands);
    let plan = plan_in(&game);
    assert_eq!(aims(&plan), [FireTarget::Ship(command(2)); 5]);
    assert_eq!(game.submit(FIRST, plan), Ok(()));
    let dodge = TurnPlan {
        maneuver: maneuver(command(2), command(5)),
        ..pass()
    };
    assert_eq!(game.submit(SECOND, dodge), Ok(()));
    game.resolve_round();
    assert_eq!(game.outcome(), Some(Outcome::Won(Player)));
}

#[test]
fn vor_einem_moeglichen_waechter_spaltenfeuer_vor_einem_trugbild_manchmal_schiffsfeuer() {
    let mut board = Board::new();
    for index in [1, 2] {
        place(&mut board, Player, Strike, index, S::HorizonCruiser);
    }
    place_hp(&mut board, Enemy, Command, 2, S::Mirage, 4);
    let fire = |ship_hand_size: usize, chance: f64| {
        let mut bot = view(&board, &[], &[]);
        bot.opponent.admiral = Some(AdmiralId::Moreau);
        bot.opponent.ship_hand_size = ship_hand_size;
        aims(&plan_with(&bot, chance))
    };
    assert_eq!(fire(2, 1.0), columns_of(&[2, 2]));
    assert_eq!(fire(2, 0.3), columns_of(&[2, 2]));
    // Ohne Schiff auf der Hand stellt Moreau ein Trugbild oder weicht aus, beides zugleich geht nicht.
    assert_eq!(fire(0, 1.0), columns_of(&[2, 2]));
    assert_eq!(fire(0, 0.3), [FireTarget::Ship(command(2)); 2]);
}

#[test]
fn spaltenfeuer_bricht_die_deckung_schiffsfeuer_trifft_das_schiff_dahinter() {
    let mut board = Board::new();
    for index in [0, 1, 2] {
        place(&mut board, Player, Strike, index, S::HorizonCruiser);
    }
    place(&mut board, Player, Command, 2, S::PlasmaFrigate);
    place_hp(&mut board, Enemy, Strike, 3, S::IronDreadnought, 4);
    place_hp(&mut board, Enemy, Command, 3, S::Hive, 3);
    // Die Kreuzer zerstören den Dreadnought (4 HP, Schild 2), dann trifft die Fregatte die Hive, auch wenn sie
    // ausweicht.
    let mut expected = columns_of(&[3, 3, 3]);
    expected.push(FireTarget::Ship(command(3)));
    assert_eq!(aims(&plan(&view(&board, &[], &[]))), expected);
    // Mit zwei Kreuzern fällt nur der Dreadnought; auf die Hive dahinter zielt keiner.
    board[SlotRef::new(Player, Strike, 0)] = None;
    assert_eq!(aims(&plan(&view(&board, &[], &[]))), columns_of(&[3, 3, 3]));
}

// --- Nachschub ---

#[test]
fn aus_dem_nachschub_das_staerkste_schiff_und_die_schrottverwertung() {
    let bot = view(&Board::new(), &[], &[C::Torpedo]);
    let offer = DraftOffer {
        ships: vec![S::RepairDrone, S::IronDreadnought, S::VoidInterceptor],
        commands: vec![C::Torpedo, C::Barrage, C::ScrapSalvage],
    };
    let pick = plan_bot_draft(&bot, &offer);
    assert_eq!(
        pick,
        DraftPick {
            ship: Some(1),
            command: Some(2)
        }
    );
    // Ohne Schrottverwertung den Befehl, von dem er am wenigsten hält.
    let offer = DraftOffer {
        ships: Vec::new(),
        commands: vec![C::Torpedo, C::Barrage],
    };
    let pick = plan_bot_draft(&bot, &offer);
    assert_eq!(
        pick,
        DraftPick {
            ship: None,
            command: Some(1)
        }
    );
}
