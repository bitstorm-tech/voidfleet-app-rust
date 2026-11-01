//! Die Auflösung einer Runde, portiert aus `server/src/resolution.test.ts`.

use rand::RngCore;
use voidfleet_core::admirals::{ABILITY_VALUES, PowerId};
use voidfleet_core::board::LaneType::{self, Command, Strike};
use voidfleet_core::board::Side::{self, Enemy, Player};
use voidfleet_core::board::{Aim, Board, Sides, SlotRef, SlotState, SlotTarget, overdrive_bonus};
use voidfleet_core::cards::{CommandId as C, ShipId as S};
use voidfleet_core::powers::{PlannedArrival, can_mark, railgun_fires};
use voidfleet_core::protocol::{
    AbilityEffect, CommandEffect, FireOrder, FireTarget, Maneuver, Miss, RoundEvent, ShipRef,
    ShipStats,
};
use voidfleet_core::resolution::{
    Drawn, PlayedCommand, PowerUse, Turn, TurnCommand, firing_railguns, resolve_cleanup,
    resolve_combat, resolve_deploy_abilities, resolve_effects, resolve_maneuver, resolve_turns,
    ship_ref,
};

// --- Hilfen ---

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

fn place(board: &mut Board, side: Side, lane: LaneType, index: usize, id: S) -> SlotRef {
    let at = SlotRef::new(side, lane, index);
    board.place(at, id).expect("Slot ist frei");
    at
}

/// `place` und danach HP setzen.
fn place_hp(
    board: &mut Board,
    side: Side,
    lane: LaneType,
    index: usize,
    id: S,
    hp: i32,
) -> SlotRef {
    let at = place(board, side, lane, index, id);
    board.ship_mut(at).hp = hp;
    at
}

fn hp(id: S) -> i32 {
    id.hp()
}

const fn at(lane: LaneType, index: usize) -> SlotTarget {
    SlotTarget::new(lane, index)
}

const fn slot(side: Side, lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(side, lane, index)
}

const fn ship(side: Side, lane: LaneType, index: usize, card: S) -> ShipRef {
    ShipRef::new(SlotRef::new(side, lane, index), card)
}

const fn stats(hp: i32, shield: i32) -> ShipStats {
    ShipStats { hp, shield }
}

fn draw_all(_: Side, ships: u32, commands: u32) -> Drawn {
    Drawn { ships, commands }
}

fn play(
    side: Side,
    card: C,
    target: Option<SlotRef>,
    destination: Option<SlotRef>,
) -> PlayedCommand {
    PlayedCommand {
        side,
        card,
        target,
        destination,
    }
}

fn effects(board: &mut Board, commands: &[PlayedCommand]) -> Vec<RoundEvent> {
    let mut events = Vec::new();
    resolve_effects(board, commands, &mut draw_all, &mut events);
    events
}

fn round(board: &mut Board, commands: &[PlayedCommand]) -> Vec<RoundEvent> {
    let mut events = effects(board, commands);
    let railguns = firing_railguns(board);
    resolve_combat(board, &mut events, &railguns);
    resolve_cleanup(board, &mut events);
    events
}

fn pass(side: Side) -> Turn {
    Turn::new(side)
}

/// Eine ganze Runde über `resolve_turns` ohne Zufall.
fn resolve(board: &mut Board, player: Turn, enemy: Turn) -> Vec<RoundEvent> {
    resolve_turns(board, &[player, enemy], &mut draw_all, None)
}

fn order(card: C) -> Option<TurnCommand> {
    Some(TurnCommand {
        card,
        target: None,
        destination: None,
    })
}

fn order_at(card: C, target: SlotTarget) -> Option<TurnCommand> {
    Some(TurnCommand {
        card,
        target: Some(target),
        destination: None,
    })
}

fn order_to(card: C, target: SlotTarget, destination: usize) -> Option<TurnCommand> {
    Some(TurnCommand {
        card,
        target: Some(target),
        destination: Some(destination),
    })
}

fn power(id: PowerId, target: SlotTarget) -> Option<PowerUse> {
    Some(PowerUse { id, target })
}

fn maneuver(ship: SlotTarget, destination: SlotTarget) -> Option<Maneuver> {
    Some(Maneuver { ship, destination })
}

/// Spaltenfeuer ohne `resolve_turns`: `shooter` feuert im Kampf auf die Spalte, in der `target` jetzt steht.
fn aim(board: &mut Board, shooter: SlotRef, target: SlotRef) {
    board.ship_mut(shooter).aim = Some(Aim::Column(target.slot_index));
}

/// Schiffsfeuer ohne `resolve_turns`: `shooter` feuert im Kampf auf das Schiff in `target`, wo es dann steht.
fn lock(board: &mut Board, shooter: SlotRef, target: SlotRef) {
    let uid = board.ship(target).uid;
    board.ship_mut(shooter).aim = Some(Aim::Ship(uid));
}

/// Spaltenfeuer im Zug: das eigene Schiff in `ship` auf die Spalte des gegnerischen Slots `target`.
fn fire(ship: SlotTarget, target: SlotTarget) -> FireOrder {
    FireOrder {
        ship,
        target: FireTarget::Column(target.slot_index),
    }
}

/// Schiffsfeuer im Zug: das eigene Schiff in `ship` auf das gegnerische Schiff in `target`.
fn fire_at(ship: SlotTarget, target: SlotTarget) -> FireOrder {
    FireOrder {
        ship,
        target: FireTarget::Ship(target),
    }
}

/// Ein Schuss im Kampf (`RoundEvent::Attack`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shot {
    attacker: ShipRef,
    target: ShipRef,
    amount: i32,
    after: ShipStats,
    focus: bool,
    swarm: bool,
    head_start: bool,
    through: bool,
    taunt: bool,
}

impl Shot {
    fn plain(attacker: ShipRef, target: ShipRef, amount: i32, after: ShipStats) -> Self {
        Shot {
            attacker,
            target,
            amount,
            after,
            focus: false,
            swarm: false,
            head_start: false,
            through: false,
            taunt: false,
        }
    }
}

fn as_shot(event: &RoundEvent) -> Option<Shot> {
    match *event {
        RoundEvent::Attack {
            attacker,
            target,
            amount,
            after,
            focus,
            swarm,
            head_start,
            through,
            taunt,
        } => Some(Shot {
            attacker,
            target,
            amount,
            after,
            focus,
            swarm,
            head_start,
            through,
            taunt,
        }),
        _ => None,
    }
}

fn attacks(events: &[RoundEvent]) -> Vec<Shot> {
    events.iter().filter_map(as_shot).collect()
}

fn attacks_by(events: &[RoundEvent], side: Side) -> Vec<Shot> {
    attacks(events)
        .into_iter()
        .filter(|shot| shot.attacker.side == side)
        .collect()
}

fn fizzles_by(events: &[RoundEvent], side: Side) -> Vec<(ShipRef, Miss)> {
    events
        .iter()
        .filter_map(|event| match *event {
            RoundEvent::Fizzle { attacker, miss, .. } if attacker.side == side => {
                Some((attacker, miss))
            }
            _ => None,
        })
        .collect()
}

/// Die Wirkung eines Befehls (`RoundEvent::Effect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Effect {
    side: Side,
    ship: ShipRef,
    effect: CommandEffect,
    after: ShipStats,
}

fn effects_of(events: &[RoundEvent], card: C) -> Vec<Effect> {
    events
        .iter()
        .filter_map(|event| match *event {
            RoundEvent::Effect {
                side,
                card_id,
                ship,
                effect,
                after,
            } if card_id == card => Some(Effect {
                side,
                ship,
                effect,
                after,
            }),
            _ => None,
        })
        .collect()
}

/// Die Art eines Ereignisses, wie sie im JSON steht.
fn kind(event: &RoundEvent) -> String {
    serde_json::to_value(event).unwrap()["kind"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn kinds(events: &[RoundEvent]) -> Vec<String> {
    events.iter().map(kind).collect()
}

fn count(events: &[RoundEvent], wanted: &str) -> usize {
    events.iter().filter(|event| kind(event) == wanted).count()
}

fn has(events: &[RoundEvent], wanted: &str) -> bool {
    count(events, wanted) > 0
}

fn abilities(events: &[RoundEvent]) -> Vec<RoundEvent> {
    events
        .iter()
        .copied()
        .filter(|event| matches!(event, RoundEvent::Ability { .. }))
        .collect()
}

fn card(board: &Board, at: SlotRef) -> Option<S> {
    board.card(at)
}

fn hp_at(board: &Board, at: SlotRef) -> i32 {
    board.ship(at).hp
}

/// Dieselbe Lage mit vertauschten Sitzen: `a` spielt einmal auf der Seite `Player` (zuerst), einmal auf der Seite
/// `Enemy` (danach). Das Brett muss gespiegelt gleich ausgehen; zurück kommt das Brett des ersten Laufs samt
/// Ereignissen.
fn mirrored(setup: impl Fn(&mut Board, Side, Side), a: Turn, b: Turn) -> (Board, Vec<RoundEvent>) {
    let run = |first: Side| {
        let mut board = Board::new();
        setup(&mut board, first, first.opposite());
        let (player, enemy) = if first == Player {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        let events = resolve(
            &mut board,
            Turn {
                side: Player,
                ..player
            },
            Turn {
                side: Enemy,
                ..enemy
            },
        );
        (board, events)
    };
    let straight = run(Player);
    let swapped = run(Enemy).0.snapshot();
    assert_eq!(swapped.mirrored(), straight.0.snapshot());
    straight
}

mod deploy_abilities {
    use super::*;

    #[test]
    fn titan_carrier_setzt_drohne_in_naechsten_freien_slot_seiner_lane() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 3, S::HorizonCruiser);
        let carrier = place(&mut board, Player, Strike, 2, S::TitanCarrier);
        let mut events = Vec::new();
        resolve_deploy_abilities(
            &mut board,
            &[carrier],
            &[],
            Some(&mut rand::rng()),
            &mut events,
        );
        assert_eq!(
            board.state(slot(Player, Strike, 1)),
            Some(SlotState {
                card_id: S::Drone,
                hp: 1,
                shield: 0,
                scrap_bonus: 0,
                marked: false
            })
        );
        assert_eq!(
            events,
            [RoundEvent::Spawn {
                source: ship(Player, Strike, 2, S::TitanCarrier),
                ship: ship(Player, Strike, 1, S::Drone),
            }]
        );
    }

    #[test]
    fn drohne_meidet_ziele_der_bewegungen_und_entfaellt_nur_ohne_freien_slot() {
        let mut board = Board::new();
        let carrier = place(&mut board, Player, Strike, 2, S::TitanCarrier);
        let reserved = [slot(Player, Strike, 1), slot(Player, Strike, 3)];
        resolve_deploy_abilities(
            &mut board,
            &[carrier],
            &reserved,
            Some(&mut rand::rng()),
            &mut Vec::new(),
        );
        assert_eq!(card(&board, slot(Player, Strike, 0)), Some(S::Drone));

        let mut full = Board::new();
        for index in [0, 1, 3] {
            place(&mut full, Player, Strike, index, S::HorizonCruiser);
        }
        place(&mut full, Player, Strike, 4, S::Drone);
        let second = place(&mut full, Player, Strike, 2, S::TitanCarrier);
        let mut events = Vec::new();
        resolve_deploy_abilities(
            &mut full,
            &[second],
            &[slot(Player, Strike, 5)],
            Some(&mut rand::rng()),
            &mut events,
        );
        assert_eq!(events, []);
    }

    #[test]
    fn drohne_startet_auch_in_volle_lane_und_zaehlt_zu_keiner_grenze() {
        let mut board = Board::new();
        for index in [0, 1] {
            place(&mut board, Player, Command, index, S::HorizonCruiser);
        }
        for index in [0, 1, 3] {
            place(&mut board, Player, Strike, index, S::HorizonCruiser);
        }
        let carrier = place(&mut board, Player, Strike, 2, S::TitanCarrier);
        resolve_deploy_abilities(
            &mut board,
            &[carrier],
            &[],
            Some(&mut rand::rng()),
            &mut Vec::new(),
        );
        assert_eq!(card(&board, slot(Player, Strike, 4)), Some(S::Drone));
        assert_eq!(board.lane_ship_count(Player, Strike), 4);
        assert_eq!(board.ship_count(Player), 6);

        board[slot(Player, Command, 0)] = None;
        assert!(!board.can_place(slot(Player, Strike, 5), &[]));
        board[slot(Player, Strike, 0)] = None;
        assert!(board.can_place(slot(Player, Strike, 5), &[]));
        assert!(board.can_move(slot(Player, Strike, 4), at(Command, 0), &[], None));
    }

    #[test]
    fn plasma_frigate_trifft_zufaelliges_schiff_der_gleichnamigen_lane_auch_gedeckt() {
        let mut board = Board::new();
        let frigate = place(&mut board, Player, Command, 0, S::PlasmaFrigate);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        place(&mut board, Enemy, Command, 1, S::HorizonCruiser);
        let covered = place(&mut board, Enemy, Command, 4, S::SignalBooster);
        let mut events = Vec::new();
        resolve_deploy_abilities(
            &mut board,
            &[frigate],
            &[],
            Some(&mut Fixed(u64::MAX)),
            &mut events,
        );
        assert_eq!(hp_at(&board, covered), hp(S::SignalBooster) - 3);
        assert!(matches!(
            events[..],
            [RoundEvent::Ability {
                source: ShipRef {
                    card_id: S::PlasmaFrigate,
                    ..
                },
                ship: ShipRef {
                    card_id: S::SignalBooster,
                    ..
                },
                effect: AbilityEffect::Damage { amount: 3 },
                ..
            }]
        ));
    }

    #[test]
    fn plasma_frigate_kann_frische_drohne_abschiessen_ohne_gegner_bleibt_sie_still() {
        let mut board = Board::new();
        let frigate = place(&mut board, Player, Strike, 0, S::PlasmaFrigate);
        let idle = place(&mut board, Player, Command, 0, S::PlasmaFrigate);
        let carrier = place(&mut board, Enemy, Strike, 0, S::TitanCarrier);
        let mut events = Vec::new();
        resolve_deploy_abilities(
            &mut board,
            &[frigate, idle, carrier],
            &[],
            Some(&mut Fixed(u64::MAX)),
            &mut events,
        );
        assert_eq!(kinds(&events), ["spawn", "ability", "destroyed"]);
        assert_eq!(board.ship_count(Enemy), 1);
    }
}

mod scrap_raider {
    use super::*;

    #[test]
    fn zerstoertes_trugbild_hinterlaesst_keinen_schrott() {
        let mut board = Board::new();
        let raider = place(&mut board, Player, Strike, 0, S::ScrapRaider);
        let decoy = place(&mut board, Enemy, Strike, 0, S::Decoy);
        aim(&mut board, raider, decoy);
        let events = round(&mut board, &[]);
        assert!(events.contains(&RoundEvent::Destroyed {
            ship: ship(Enemy, Strike, 0, S::Decoy)
        }));
        assert!(!has(&events, "ability"));
    }

    #[test]
    fn birgt_schrott_einmal_je_raider_der_bonus_kommt_erst_mit_dem_nachschub() {
        let mut board = Board::new();
        let raider = place(&mut board, Player, Strike, 0, S::ScrapRaider);
        place(&mut board, Player, Command, 2, S::ScrapRaider);
        place_hp(&mut board, Enemy, Strike, 0, S::RepairDrone, 1);
        place_hp(&mut board, Enemy, Strike, 1, S::RepairDrone, 1);
        let events = round(&mut board, &[play(Player, C::VolleyFire, None, None)]);
        assert_eq!(board.ship(raider).scrap_bonus, 0);
        let own = ship(Player, Strike, 0, S::ScrapRaider);
        assert_eq!(
            abilities(&events),
            [RoundEvent::Ability {
                source: own,
                ship: own,
                effect: AbilityEffect::Salvage,
                after: stats(hp(S::ScrapRaider), 0),
            }]
        );
    }

    #[test]
    fn bonus_bleibt_folgt_dem_schiff_und_steht_im_snapshot() {
        let mut board = Board::new();
        let raider = place(&mut board, Player, Strike, 0, S::ScrapRaider);
        board.ship_mut(raider).scrap_bonus = 1;
        round(&mut board, &[]);
        let to = slot(Player, Command, 3);
        resolve_maneuver(&mut board, raider, to, &mut Vec::new());
        assert_eq!(board.ship(to).scrap_bonus, 1);
        let copy = Board::from_snapshot(&board.snapshot());
        assert_eq!(copy.attack(to), S::ScrapRaider.atk() + 1);
    }

    #[test]
    fn raider_der_selbst_faellt_bekommt_nichts() {
        let mut board = Board::new();
        place_hp(&mut board, Player, Strike, 0, S::ScrapRaider, 1);
        place_hp(&mut board, Enemy, Strike, 0, S::RepairDrone, 1);
        let events = round(
            &mut board,
            &[
                play(Player, C::VolleyFire, None, None),
                play(Enemy, C::VolleyFire, None, None),
            ],
        );
        assert!(!has(&events, "ability"));
    }
}

mod manoever {
    use super::*;

    #[test]
    fn manoevriertes_schiff_feuert_nicht_wird_aber_beschossen() {
        let mut board = Board::new();
        let from = place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        let enemy = place(&mut board, Enemy, Strike, 0, S::VoidInterceptor);
        aim(&mut board, from, enemy);
        aim(&mut board, enemy, from);
        let to = slot(Player, Command, 0);
        resolve_maneuver(&mut board, from, to, &mut Vec::new());
        assert!(!board.is_occupied(from));
        assert!(board.ship(to).maneuvered);
        let shots = attacks(&round(&mut board, &[]));
        let summary: Vec<_> = shots
            .iter()
            .map(|shot| (shot.attacker.side, shot.target.lane, shot.target.slot_index))
            .collect();
        assert_eq!(summary, [(Enemy, Command, 0)]);
        assert_eq!(hp_at(&board, to), hp(S::VoidInterceptor) - 3);
        assert!(!board.ship(to).maneuvered);
    }

    #[test]
    fn wer_per_manoever_vor_ein_schiff_zieht_faengt_den_schuss_ab() {
        let mut board = Board::new();
        let drone = place(&mut board, Player, Command, 2, S::RepairDrone);
        let cruiser = place(&mut board, Player, Command, 4, S::HorizonCruiser);
        let shooter = place(&mut board, Enemy, Strike, 0, S::VoidInterceptor);
        aim(&mut board, shooter, drone);
        resolve_maneuver(
            &mut board,
            cruiser,
            slot(Player, Strike, 2),
            &mut Vec::new(),
        );
        // Der Jäger zielt auf die Spalte der Drohne; jetzt steht der Kreuzer davor und fängt den Schuss ab.
        let shots = attacks(&round(&mut board, &[]));
        let summary: Vec<_> = shots
            .iter()
            .map(|shot| (shot.attacker.side, shot.target.card_id))
            .collect();
        assert_eq!(summary, [(Enemy, S::HorizonCruiser)]);
        assert_eq!(hp_at(&board, drone), hp(S::RepairDrone));
    }
}

mod combat {
    use super::*;

    #[test]
    fn alle_feuern_gleichzeitig_auch_ein_im_selben_kampf_zerstoertes() {
        let mut board = Board::new();
        let interceptor = place_hp(&mut board, Player, Strike, 0, S::VoidInterceptor, 3);
        let enemy = place_hp(&mut board, Enemy, Strike, 0, S::VoidInterceptor, 3);
        aim(&mut board, interceptor, enemy);
        aim(&mut board, enemy, interceptor);
        let events = round(&mut board, &[]);
        assert!(!board.is_occupied(interceptor));
        assert!(!board.is_occupied(enemy));
        assert_eq!(count(&events, "destroyed"), 2);
    }

    #[test]
    fn schuss_auf_eine_spalte_ohne_schiff_verpufft() {
        let mut board = Board::new();
        let shooters: Vec<_> = (0..3)
            .map(|index| place(&mut board, Player, Strike, index, S::VoidInterceptor))
            .collect();
        let weak = place_hp(&mut board, Enemy, Strike, 0, S::HorizonCruiser, 3);
        let other = place(&mut board, Enemy, Strike, 3, S::HorizonCruiser);
        aim(&mut board, shooters[0], weak);
        aim(&mut board, shooters[1], weak);
        aim(&mut board, shooters[2], other);
        let events = round(&mut board, &[]);
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker.slot_index, shot.target.slot_index))
            .collect();
        assert_eq!(summary, [(0, 0), (2, 3)]);
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship(Player, Strike, 1, S::VoidInterceptor),
                Miss::Column {
                    column: 0,
                    cleared: true
                }
            )]
        );
        assert!(!board.is_occupied(weak));
        assert_eq!(hp_at(&board, other), hp(S::HorizonCruiser) - 3);
    }

    #[test]
    fn ohne_feuerbefehl_feuert_kein_schiff_mit_feuerbefehl_jedes_auf_sein_ziel() {
        let mut board = Board::new();
        let interceptor = place(&mut board, Player, Command, 0, S::VoidInterceptor);
        let dreadnought = place(&mut board, Player, Command, 1, S::IronDreadnought);
        place(&mut board, Player, Command, 4, S::HorizonCruiser);
        let weak = place(&mut board, Enemy, Strike, 0, S::SignalBooster);
        place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
        let tough = place(&mut board, Enemy, Strike, 5, S::TitanCarrier);
        aim(&mut board, interceptor, tough);
        aim(&mut board, dreadnought, weak);
        let summary: Vec<_> = attacks_by(&round(&mut board, &[]), Player)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.target.card_id))
            .collect();
        assert_eq!(
            summary,
            [
                (S::VoidInterceptor, S::TitanCarrier),
                (S::IronDreadnought, S::SignalBooster)
            ]
        );
        assert_eq!(hp_at(&board, weak), hp(S::SignalBooster) - 2);
        assert_eq!(hp_at(&board, tough), hp(S::TitanCarrier) - 3);
    }

    #[test]
    fn die_spalte_bleibt_zieht_das_ziel_weg_trifft_der_schuss_was_dort_steht() {
        let mut board = Board::new();
        let cruiser = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let raider = place(&mut board, Player, Strike, 1, S::ScrapRaider);
        let mover = place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
        let behind = place(&mut board, Enemy, Command, 2, S::SignalBooster);
        let gone = place(&mut board, Enemy, Strike, 4, S::VoidInterceptor);
        aim(&mut board, cruiser, mover);
        aim(&mut board, raider, gone);
        let to = slot(Enemy, Strike, 5);
        resolve_maneuver(&mut board, mover, to, &mut Vec::new());
        board[gone] = None;
        let events = round(&mut board, &[]);
        assert_eq!(
            attacks_by(&events, Player),
            [Shot::plain(
                ship_ref(&board, cruiser),
                ship_ref(&board, behind),
                2,
                stats(hp(S::SignalBooster) - 2, 0)
            )]
        );
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship_ref(&board, raider),
                Miss::Column {
                    column: 4,
                    cleared: false
                }
            )]
        );
        assert_eq!(hp_at(&board, to), hp(S::VoidInterceptor));
    }

    #[test]
    fn durchbruch_faellt_das_vordere_schiff_trifft_der_naechste_schuss_das_dahinter() {
        let mut board = Board::new();
        let first = place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        let second = place(&mut board, Player, Strike, 1, S::VoidInterceptor);
        let front = place_hp(&mut board, Enemy, Strike, 0, S::HorizonCruiser, 3);
        let back = place(&mut board, Enemy, Command, 0, S::SignalBooster);
        aim(&mut board, first, front);
        aim(&mut board, second, back);
        let expected = [
            (ship_ref(&board, first), ship_ref(&board, front), false),
            (ship_ref(&board, second), ship_ref(&board, back), true),
        ];
        let events = round(&mut board, &[]);
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker, shot.target, shot.through))
            .collect();
        assert_eq!(summary, expected);
        assert_eq!(fizzles_by(&events, Player), []);
        assert!(!board.is_occupied(front));
        assert_eq!(hp_at(&board, back), hp(S::SignalBooster) - 3);
    }

    #[test]
    fn im_selben_kampf_zerstoertes_schiff_feuert_noch() {
        let mut board = Board::new();
        let own = place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        let enemy = place_hp(&mut board, Enemy, Strike, 0, S::VoidInterceptor, 3);
        aim(&mut board, own, enemy);
        let victim = place_hp(&mut board, Player, Strike, 2, S::HorizonCruiser, 3);
        aim(&mut board, enemy, victim);
        round(&mut board, &[]);
        assert!(!board.is_occupied(enemy));
        assert!(!board.is_occupied(victim));
    }

    #[test]
    fn schuesse_wechseln_sich_zwischen_den_seiten_ab() {
        let mut board = Board::new();
        let own: Vec<_> = (0..3)
            .map(|index| place(&mut board, Player, Strike, index, S::HorizonCruiser))
            .collect();
        let enemies: Vec<_> = (0..2)
            .map(|index| place(&mut board, Enemy, Strike, index, S::TitanCarrier))
            .collect();
        for &shooter in &own {
            aim(&mut board, shooter, enemies[0]);
        }
        for &shooter in &enemies {
            aim(&mut board, shooter, own[0]);
        }
        let sides: Vec<_> = attacks(&round(&mut board, &[]))
            .iter()
            .map(|shot| shot.attacker.side)
            .collect();
        assert_eq!(sides, [Player, Enemy, Player, Enemy, Player]);
    }

    #[test]
    fn spaltenfeuer_trifft_zuerst_das_strike_schiff_sonst_das_command_schiff() {
        let mut board = Board::new();
        let first = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let second = place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        let front = place(&mut board, Enemy, Strike, 3, S::TitanCarrier);
        let covered = place(&mut board, Enemy, Command, 3, S::SignalBooster);
        let exposed = place(&mut board, Enemy, Command, 0, S::SignalBooster);
        aim(&mut board, first, covered);
        aim(&mut board, second, exposed);
        let events = round(&mut board, &[]);
        assert_eq!(
            attacks_by(&events, Player),
            [
                Shot::plain(
                    ship_ref(&board, first),
                    ship_ref(&board, front),
                    2,
                    stats(hp(S::TitanCarrier) - 2, 0)
                ),
                Shot::plain(
                    ship_ref(&board, second),
                    ship_ref(&board, exposed),
                    2,
                    stats(hp(S::SignalBooster) - 2, 0)
                ),
            ]
        );
        assert_eq!(hp_at(&board, exposed), hp(S::SignalBooster) - 2);
        assert_eq!(hp_at(&board, covered), hp(S::SignalBooster));
        assert_eq!(hp_at(&board, front), hp(S::TitanCarrier) - 2);
    }

    #[test]
    fn jeder_schuss_wird_mit_den_werten_des_ziels_danach_gemeldet() {
        let mut board = Board::new();
        let target = place(&mut board, Enemy, Strike, 0, S::IronDreadnought);
        let a = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let b = place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        aim(&mut board, a, target);
        aim(&mut board, b, target);
        let summary: Vec<_> = attacks_by(&round(&mut board, &[]), Player)
            .iter()
            .map(|shot| (shot.amount, shot.after))
            .collect();
        assert_eq!(
            summary,
            [
                (2, stats(hp(S::IronDreadnought), 0)),
                (2, stats(hp(S::IronDreadnought) - 2, 0))
            ]
        );
    }

    #[test]
    fn schild_faengt_schaden_zuerst_ab_und_regeneriert_nicht() {
        let mut board = Board::new();
        let dreadnought = place(&mut board, Enemy, Strike, 0, S::IronDreadnought);
        let frigate = place(&mut board, Player, Strike, 0, S::PlasmaFrigate);
        aim(&mut board, frigate, dreadnought);
        round(&mut board, &[]);
        let ship = board.ship(dreadnought);
        assert_eq!((ship.shield, ship.hp), (0, hp(S::IronDreadnought) - 1));
    }

    #[test]
    fn support_greift_nicht_an() {
        let mut board = Board::new();
        let enemy = place(&mut board, Enemy, Strike, 0, S::SignalBooster);
        let booster = place(&mut board, Player, Strike, 0, S::SignalBooster);
        aim(&mut board, booster, enemy);
        let events = round(&mut board, &[]);
        assert!(!has(&events, "attack") && !has(&events, "fizzle"));
        assert_eq!(hp_at(&board, enemy), hp(S::SignalBooster));
    }
}

mod befehle {
    use super::*;

    #[test]
    fn orbital_strike_trifft_das_schiff_mit_den_meisten_hp_bevor_es_feuern_kann() {
        let mut board = Board::new();
        let own = place_hp(&mut board, Player, Strike, 0, S::SignalBooster, 4);
        let interceptor = place_hp(&mut board, Enemy, Strike, 0, S::VoidInterceptor, 3);
        let target = place_hp(&mut board, Enemy, Command, 0, S::ScrapRaider, 4);
        aim(&mut board, interceptor, own);
        aim(&mut board, target, own);
        round(&mut board, &[play(Player, C::OrbitalStrike, None, None)]);
        assert!(!board.is_occupied(target));
        assert_eq!(hp_at(&board, own), 4 - 3);
    }

    #[test]
    fn emergency_shields_faengt_orbital_strike_ab_weil_buffs_vor_angriffen_wirken() {
        let mut board = Board::new();
        let own = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let events = effects(
            &mut board,
            &[
                play(Enemy, C::OrbitalStrike, None, None),
                play(Player, C::EmergencyShields, Some(own), None),
            ],
        );
        let ship = board.ship(own);
        assert_eq!((ship.hp, ship.shield), (hp(S::HorizonCruiser) - (4 - 3), 0));
        assert_eq!(kinds(&events), ["effect", "effect"]);
    }

    #[test]
    fn bonus_schild_verfaellt_im_cleanup_soweit_er_noch_steht() {
        let mut board = Board::new();
        let own = place(&mut board, Player, Strike, 0, S::IronDreadnought);
        round(
            &mut board,
            &[play(Player, C::EmergencyShields, Some(own), None)],
        );
        let ship = board.ship(own);
        assert_eq!((ship.shield, ship.hp), (2, hp(S::IronDreadnought)));
    }

    #[test]
    fn emp_legt_das_staerkste_schiff_lahm_targeting_jammer_senkt_die_strike_lane() {
        let mut board = Board::new();
        let frigate = place(&mut board, Enemy, Strike, 0, S::PlasmaFrigate);
        let cruiser = place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let own = place(&mut board, Player, Strike, 0, S::TitanCarrier);
        aim(&mut board, frigate, own);
        aim(&mut board, cruiser, own);
        round(
            &mut board,
            &[
                play(Player, C::EmpBlast, None, None),
                play(Player, C::TargetingJammer, None, None),
            ],
        );
        assert_eq!(hp_at(&board, own), hp(S::TitanCarrier) - (2 - 1));
        assert!(!board.ship(frigate).disabled);
        assert_eq!(board.attack(cruiser), 2);
    }

    #[test]
    fn intercept_zieht_ein_schiff_in_den_gewaehlten_strike_slot_und_gibt_2_atk() {
        let mut board = Board::new();
        let back = place(&mut board, Player, Command, 2, S::HorizonCruiser);
        let enemy = place(&mut board, Enemy, Strike, 2, S::TitanCarrier);
        aim(&mut board, back, enemy);
        let destination = slot(Player, Strike, 5);
        let events = round(
            &mut board,
            &[play(Player, C::Intercept, Some(back), Some(destination))],
        );
        assert!(!board.is_occupied(back));
        assert_eq!(card(&board, destination), Some(S::HorizonCruiser));
        assert!(
            effects_of(&events, C::Intercept)
                .iter()
                .any(|effect| effect.effect == CommandEffect::Move { to: at(Strike, 5) })
        );
        assert_eq!(hp_at(&board, enemy), hp(S::TitanCarrier) - 4);
    }

    #[test]
    fn recall_zieht_zurueck_und_heilt_overcharge_kostet_nach_dem_kampf_2() {
        let mut board = Board::new();
        let front = place_hp(&mut board, Player, Strike, 1, S::HorizonCruiser, 2);
        let moved = slot(Player, Command, 4);
        round(
            &mut board,
            &[play(Player, C::Recall, Some(front), Some(moved))],
        );
        assert_eq!(hp_at(&board, moved), 4);

        round(
            &mut board,
            &[play(Player, C::Overcharge, Some(moved), None)],
        );
        assert_eq!(hp_at(&board, moved), 2);
    }
}

mod torpedo_und_sperrfeuer {
    use super::*;

    #[test]
    fn torpedo_trifft_wer_nach_dem_manoever_im_slot_steht_und_verpufft_im_geraeumten_slot() {
        let mut board = Board::new();
        let dodger = place(&mut board, Enemy, Strike, 3, S::PlasmaFrigate);
        let mover = place(&mut board, Enemy, Strike, 4, S::HorizonCruiser);
        resolve_maneuver(&mut board, dodger, slot(Enemy, Command, 2), &mut Vec::new());
        resolve_maneuver(&mut board, mover, slot(Enemy, Command, 4), &mut Vec::new());

        let missed = effects(
            &mut board,
            &[play(Player, C::Torpedo, Some(slot(Enemy, Strike, 3)), None)],
        );
        assert_eq!(
            missed,
            [RoundEvent::Miss {
                side: Player,
                card_id: C::Torpedo,
                target: slot(Enemy, Strike, 3)
            }]
        );
        assert_eq!(hp_at(&board, slot(Enemy, Command, 2)), hp(S::PlasmaFrigate));

        let hit = effects(
            &mut board,
            &[play(
                Player,
                C::Torpedo,
                Some(slot(Enemy, Command, 4)),
                None,
            )],
        );
        assert!(matches!(
            hit[..],
            [RoundEvent::Effect {
                effect: CommandEffect::Damage { amount: 5 },
                ship: ShipRef {
                    side: Enemy,
                    lane: Command,
                    slot_index: 4,
                    ..
                },
                ..
            }]
        ));
        assert_eq!(
            hp_at(&board, slot(Enemy, Command, 4)),
            hp(S::HorizonCruiser) - 5
        );
    }

    #[test]
    fn torpedo_trifft_auch_ein_gedecktes_command_schiff_der_schild_faengt_zuerst_ab() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 1, S::TitanCarrier);
        let covered = place(&mut board, Enemy, Command, 1, S::IronDreadnought);
        effects(&mut board, &[play(Player, C::Torpedo, Some(covered), None)]);
        let ship = board.ship(covered);
        assert_eq!((ship.shield, ship.hp), (0, hp(S::IronDreadnought) - 3));
    }

    #[test]
    fn sperrfeuer_trifft_nur_bewegte_schiffe_gedeckt_oder_nicht() {
        let mut board = Board::new();
        let staying = place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        let maneuvering = place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let flanking = place(&mut board, Enemy, Command, 0, S::HorizonCruiser);
        let own = place(&mut board, Player, Strike, 5, S::HorizonCruiser);
        resolve_maneuver(
            &mut board,
            maneuvering,
            slot(Enemy, Command, 2),
            &mut Vec::new(),
        );
        resolve_maneuver(&mut board, own, slot(Player, Command, 5), &mut Vec::new());
        place(&mut board, Enemy, Strike, 2, S::TitanCarrier);
        let events = effects(
            &mut board,
            &[
                play(Player, C::Barrage, None, None),
                play(
                    Enemy,
                    C::FlankingManeuver,
                    Some(flanking),
                    Some(slot(Enemy, Strike, 4)),
                ),
            ],
        );
        let hits: Vec<_> = effects_of(&events, C::Barrage)
            .iter()
            .map(|effect| (effect.ship.lane, effect.ship.slot_index))
            .collect();
        assert_eq!(hits, [(Strike, 4), (Command, 2)]);
        assert_eq!(hp_at(&board, staying), hp(S::HorizonCruiser));
        assert_eq!(
            hp_at(&board, slot(Enemy, Command, 2)),
            hp(S::HorizonCruiser) - 3
        );
        assert_eq!(
            hp_at(&board, slot(Player, Command, 5)),
            hp(S::HorizonCruiser)
        );
    }

    #[test]
    fn sperrfeuer_ohne_bewegten_gegner_bleibt_ohne_wirkung_das_flag_gilt_nur_eine_runde() {
        let mut board = Board::new();
        let enemy = place(&mut board, Enemy, Command, 0, S::HorizonCruiser);
        resolve_maneuver(&mut board, enemy, slot(Enemy, Strike, 0), &mut Vec::new());
        round(&mut board, &[]);
        let events = effects(&mut board, &[play(Player, C::Barrage, None, None)]);
        assert_eq!(
            events,
            [RoundEvent::NoEffect {
                side: Player,
                card_id: C::Barrage
            }]
        );
    }

    #[test]
    fn schiff_das_gegen_sperrfeuer_manoevriert_wird_getroffen_und_feuert_nicht() {
        let mut board = Board::new();
        let enemy = place(&mut board, Enemy, Strike, 0, S::PlasmaFrigate);
        let own = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        aim(&mut board, enemy, own);
        aim(&mut board, own, enemy);
        let to = slot(Enemy, Command, 0);
        resolve_maneuver(&mut board, enemy, to, &mut Vec::new());
        let events = round(&mut board, &[play(Player, C::Barrage, None, None)]);
        let sides: Vec<_> = attacks(&events)
            .iter()
            .map(|shot| shot.attacker.side)
            .collect();
        assert_eq!(sides, [Player]);
        assert_eq!(hp_at(&board, to), hp(S::PlasmaFrigate) - 3 - 2);
        assert_eq!(hp_at(&board, own), hp(S::HorizonCruiser));
    }
}

mod faehigkeiten {
    use super::*;

    #[test]
    fn signal_booster_gibt_allen_anderen_schiffen_seiner_lane_1_atk() {
        let mut board = Board::new();
        let booster = place(&mut board, Player, Strike, 2, S::SignalBooster);
        let neighbour = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let other_lane = place(&mut board, Player, Command, 0, S::HorizonCruiser);
        let enemy = place(&mut board, Enemy, Strike, 4, S::HorizonCruiser);
        let values: Vec<_> = [neighbour, other_lane, enemy, booster]
            .iter()
            .map(|&at| board.attack(at))
            .collect();
        assert_eq!(values, [3, 2, 2, 0]);
        place(&mut board, Player, Strike, 5, S::SignalBooster);
        assert_eq!(board.attack(neighbour), 4);
    }

    #[test]
    fn im_kampf_zerstoerter_signal_booster_verstaerkt_die_schuesse_dieses_kampfes_noch() {
        let mut board = Board::new();
        let booster = place_hp(&mut board, Player, Strike, 2, S::SignalBooster, 3);
        let cruiser = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let frigate = place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
        aim(&mut board, cruiser, frigate);
        aim(&mut board, frigate, booster);
        round(&mut board, &[]);
        assert!(!board.is_occupied(booster));
        assert_eq!(hp_at(&board, frigate), hp(S::PlasmaFrigate) - 3);
        assert_eq!(board.attack(cruiser), 2);
    }

    #[test]
    fn repair_drone_heilt_das_beschaedigte_kampfschiff_mit_den_wenigsten_hp() {
        let mut board = Board::new();
        let drone = place_hp(&mut board, Player, Command, 0, S::RepairDrone, 1);
        let scratched = place_hp(&mut board, Player, Command, 1, S::IronDreadnought, 11);
        let weakest = place_hp(&mut board, Player, Command, 2, S::HorizonCruiser, 3);
        place_hp(&mut board, Player, Command, 3, S::SignalBooster, 1);
        let events = round(&mut board, &[]);
        assert_eq!(count(&events, "ability"), 1);
        let values: Vec<_> = [drone, scratched, weakest]
            .iter()
            .map(|&at| hp_at(&board, at))
            .collect();
        assert_eq!(values, [1, 11, 5]);
    }

    #[test]
    fn im_kampf_zerstoerte_repair_drone_heilt_nicht_mehr() {
        let mut board = Board::new();
        let drone = place_hp(&mut board, Player, Strike, 0, S::RepairDrone, 3);
        let cruiser = place_hp(&mut board, Player, Command, 1, S::HorizonCruiser, 4);
        let frigate = place(&mut board, Enemy, Strike, 0, S::PlasmaFrigate);
        aim(&mut board, frigate, drone);
        assert!(!has(&round(&mut board, &[]), "ability"));
        assert_eq!(hp_at(&board, cruiser), 4);
    }
}

mod overdrive {
    use super::*;

    #[test]
    fn ab_runde_8_plus_1_atk_alle_4_runden_ein_weiterer_punkt() {
        let bonus: Vec<_> = [1, 7, 8, 11, 12, 16].map(overdrive_bonus).into();
        assert_eq!(bonus, [0, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn kampfschiffe_treffen_mit_dem_bonus_support_bleibt_bei_0_atk() {
        let mut board = Board::new();
        board.overdrive = 2;
        let drone = place(&mut board, Player, Command, 0, S::RepairDrone);
        let interceptor = place(&mut board, Player, Strike, 1, S::VoidInterceptor);
        let target = place(&mut board, Enemy, Strike, 0, S::IronDreadnought);
        aim(&mut board, interceptor, target);
        aim(&mut board, target, interceptor);
        assert_eq!(board.attack(drone), 0);
        assert!(!board.can_attack(drone));
        let summary: Vec<_> = attacks(&round(&mut board, &[]))
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.amount))
            .collect();
        assert_eq!(
            summary,
            [
                (S::VoidInterceptor, S::VoidInterceptor.atk() + 2),
                (S::IronDreadnought, S::IronDreadnought.atk() + 2)
            ]
        );
        let ship = board.ship(target);
        assert_eq!(
            ship.hp + ship.shield,
            hp(S::IronDreadnought) + S::IronDreadnought.card().shield
                - S::VoidInterceptor.atk()
                - 2
        );
    }
}

mod flaggschiffe {
    use super::*;

    #[test]
    fn waechterschild_bastion_2_und_schiff_vor_ihr_1_fuer_die_runde() {
        let mut board = Board::new();
        let bastion = place(&mut board, Player, Command, 2, S::Bastion);
        let guard = place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        let other = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 2, S::PlasmaFrigate);
        let events = resolve(
            &mut board,
            pass(Player),
            Turn {
                fire: vec![fire(at(Strike, 2), at(Strike, 2))],
                ..pass(Enemy)
            },
        );
        let source = ship_ref(&board, bastion);
        assert_eq!(
            abilities(&events),
            [
                RoundEvent::Ability {
                    source,
                    ship: source,
                    effect: AbilityEffect::Shield { amount: 2 },
                    after: stats(hp(S::Bastion), 2),
                },
                RoundEvent::Ability {
                    source,
                    ship: ship_ref(&board, guard),
                    effect: AbilityEffect::Shield { amount: 1 },
                    after: stats(hp(S::HorizonCruiser), 1),
                },
            ]
        );
        // Der Schild fängt 1 der 3 Schaden der Plasma Frigate ab und verfällt im Cleanup.
        let values = [
            board.ship(guard).hp,
            board.ship(guard).shield,
            board.ship(other).shield,
            board.ship(bastion).shield,
        ];
        assert_eq!(values, [hp(S::HorizonCruiser) - 2, 0, 0, 0]);
    }

    #[test]
    fn waechterschild_steht_die_bastion_vorne_bekommt_nur_sie_den_schild() {
        let mut board = Board::new();
        let bastion = place(&mut board, Player, Strike, 1, S::Bastion);
        place(&mut board, Player, Command, 1, S::HorizonCruiser);
        let events = resolve(&mut board, pass(Player), pass(Enemy));
        let bastion = ship_ref(&board, bastion);
        assert!(matches!(
            abilities(&events)[..],
            [RoundEvent::Ability { ship, effect: AbilityEffect::Shield { amount: 2 }, .. }] if ship == bastion
        ));
    }

    #[test]
    fn speerspitze_steht_die_vanguard_hinten_hat_das_kampfschiff_davor_1_atk() {
        let mut board = Board::new();
        let vanguard = place(&mut board, Player, Command, 1, S::Vanguard);
        let front = place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        let beside = place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        let values: Vec<_> = [front, beside, vanguard]
            .iter()
            .map(|&at| board.attack(at))
            .collect();
        assert_eq!(values, [3, 2, 3]);
        let forward = slot(Player, Strike, 3);
        resolve_maneuver(&mut board, vanguard, forward, &mut Vec::new());
        assert_eq!([board.attack(front), board.attack(forward)], [2, 3]);

        let mut support = Board::new();
        place(&mut support, Player, Command, 0, S::Vanguard);
        let drone = place(&mut support, Player, Strike, 0, S::RepairDrone);
        assert_eq!(support.attack(drone), 0);
    }

    #[test]
    fn brutkammer_startet_nach_dem_cleanup_eine_drohne_in_den_naechsten_freien_strike_slot() {
        let mut board = Board::new();
        let hive = place(&mut board, Player, Command, 2, S::Hive);
        place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        let events = resolve(&mut board, pass(Player), pass(Enemy));
        assert_eq!(
            events.last(),
            Some(&RoundEvent::Spawn {
                source: ship_ref(&board, hive),
                ship: ship(Player, Strike, 1, S::Drone)
            })
        );
        assert_eq!(
            board.hive_launches,
            Sides {
                player: 1,
                enemy: 0
            }
        );
    }

    #[test]
    fn brutkammer_hoechstens_2_drohnen_auf_der_seite_und_ein_budget_im_match() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 2, S::Hive);
        place(&mut board, Player, Strike, 5, S::Drone);
        let drones = |board: &Board| {
            board
                .ships(Player)
                .filter(|&at| board.ship(at).card == S::Drone)
                .count()
        };
        resolve(&mut board, pass(Player), pass(Enemy));
        assert_eq!(drones(&board), 2);
        assert!(!has(
            &resolve(&mut board, pass(Player), pass(Enemy)),
            "spawn"
        ));
        board[slot(Player, Strike, 5)] = None;
        board.hive_launches.player = 10;
        assert!(!has(
            &resolve(&mut board, pass(Player), pass(Enemy)),
            "spawn"
        ));
        assert_eq!(drones(&board), 1);
    }

    #[test]
    fn phasentausch_entfaellt_wenn_der_partner_beim_deploy_faellt() {
        let mut board = Board::new();
        let mirage = place(&mut board, Player, Command, 2, S::Mirage);
        place_hp(&mut board, Player, Strike, 0, S::VoidInterceptor, 2);
        place(&mut board, Enemy, Command, 5, S::TitanCarrier);
        // Die Plasma Frigate des Gegners schießt beim Deploy auf das einzige Schiff gegenüber, den Tauschpartner.
        let events = resolve_turns(
            &mut board,
            &[
                Turn {
                    maneuver: maneuver(at(Command, 2), at(Strike, 0)),
                    command: order_at(C::Overcharge, at(Strike, 0)),
                    fire: vec![fire(at(Command, 2), at(Command, 5))],
                    ..pass(Player)
                },
                Turn {
                    ships: vec![PlannedArrival {
                        card: S::PlasmaFrigate,
                        target: at(Strike, 0),
                    }],
                    ..pass(Enemy)
                },
            ],
            &mut draw_all,
            Some(&mut Fixed(0)),
        );
        assert!(!has(&events, "swap") && !has(&events, "maneuver"));
        assert_eq!(card(&board, mirage), Some(S::Mirage));
        assert!(events.contains(&RoundEvent::NoEffect {
            side: Player,
            card_id: C::Overcharge
        }));
        assert!(
            attacks(&events)
                .iter()
                .any(|shot| shot.attacker.card_id == S::Mirage)
        );
        assert!(!has(&events, "self_damage"));
    }

    #[test]
    fn phasentausch_tauscht_samt_zustand_beide_feuern_keins_gilt_als_bewegt() {
        let mut board = Board::new();
        place_hp(&mut board, Player, Strike, 1, S::Mirage, 9);
        place(&mut board, Player, Command, 4, S::IronDreadnought);
        place(&mut board, Enemy, Strike, 1, S::TitanCarrier);
        let events = resolve(
            &mut board,
            Turn {
                maneuver: maneuver(at(Strike, 1), at(Command, 4)),
                command: order_at(C::EmergencyShields, at(Command, 4)),
                fire: vec![
                    fire(at(Strike, 1), at(Strike, 1)),
                    fire(at(Command, 4), at(Strike, 1)),
                ],
                ..pass(Player)
            },
            Turn {
                command: order(C::Barrage),
                fire: vec![fire(at(Strike, 1), at(Strike, 1))],
                ..pass(Enemy)
            },
        );
        assert!(events.contains(&RoundEvent::Swap {
            side: Player,
            a: ship(Player, Strike, 1, S::Mirage),
            b: ship(Player, Command, 4, S::IronDreadnought),
        }));
        assert!(!has(&events, "maneuver"));
        // Der Schild folgt dem Dreadnought, das Sperrfeuer findet kein bewegtes Schiff.
        assert!(events.contains(&RoundEvent::Command {
            side: Player,
            card_id: C::EmergencyShields,
            target: Some(slot(Player, Strike, 1)),
        }));
        assert!(events.contains(&RoundEvent::NoEffect {
            side: Enemy,
            card_id: C::Barrage
        }));
        let shooters: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| shot.attacker.card_id)
            .collect();
        assert_eq!(shooters, [S::IronDreadnought, S::Mirage]);
        // Der Titan Carrier feuert auf Spalte 1: Dort steht jetzt der Dreadnought und fängt den Schuss mit dem Schild ab.
        let targets: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| shot.target)
            .collect();
        assert_eq!(targets, [ship(Player, Strike, 1, S::IronDreadnought)]);
        assert_eq!(
            board.state(slot(Player, Strike, 1)),
            Some(SlotState {
                card_id: S::IronDreadnought,
                hp: hp(S::IronDreadnought),
                shield: 1,
                scrap_bonus: 0,
                marked: false,
            })
        );
        assert_eq!(
            board.state(slot(Player, Command, 4)),
            Some(SlotState {
                card_id: S::Mirage,
                hp: 9,
                shield: 0,
                scrap_bonus: 0,
                marked: false,
            })
        );
    }

    #[test]
    fn railgun_markierung_nach_den_deploys_feuer_in_der_naechsten_runde_vor_allen_schuessen() {
        let mut board = Board::new();
        let pendulum = place(&mut board, Player, Command, 2, S::Pendulum);
        place(&mut board, Player, Strike, 2, S::RepairDrone);
        let target = place(&mut board, Enemy, Strike, 4, S::TitanCarrier);
        let marking = resolve(
            &mut board,
            Turn {
                mark: Some(at(Strike, 4)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(marking.contains(&RoundEvent::Mark {
            side: Player,
            ship: ship_ref(&board, target)
        }));
        assert!(!has(&marking, "railgun"));
        assert_eq!(
            [
                board.ship(target).marked,
                railgun_fires(&board, Player),
                can_mark(&board, Player)
            ],
            [true, true, false]
        );

        let before = hp_at(&board, target);
        let firing: Vec<_> = resolve(&mut board, pass(Player), pass(Enemy))
            .into_iter()
            .filter(|event| {
                matches!(
                    event,
                    RoundEvent::Railgun { .. } | RoundEvent::Attack { .. }
                )
            })
            .collect();
        assert_eq!(
            firing[0],
            RoundEvent::Railgun {
                source: ship_ref(&board, pendulum),
                target: ship_ref(&board, target),
                amount: ABILITY_VALUES.railgun,
                after: stats(before - ABILITY_VALUES.railgun, 0),
                blocked: false,
            }
        );
        assert_eq!(
            [board.ship(target).marked, can_mark(&board, Player)],
            [false, true]
        );
    }

    fn railgun(events: &[RoundEvent]) -> Option<(i32, ShipStats, bool)> {
        events.iter().find_map(|event| match *event {
            RoundEvent::Railgun {
                amount,
                after,
                blocked,
                ..
            } => Some((amount, after, blocked)),
            _ => None,
        })
    }

    #[test]
    fn railgun_gedecktes_ziel_blockiert_den_schuss_ein_trugbild_deckt_nicht() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 2, S::Pendulum);
        place(&mut board, Player, Strike, 2, S::RepairDrone);
        let target = place(&mut board, Enemy, Command, 3, S::SignalBooster);
        board.ship_mut(target).marked = true;
        let covered = resolve(
            &mut board,
            pass(Player),
            Turn {
                ships: vec![PlannedArrival {
                    card: S::HorizonCruiser,
                    target: at(Strike, 3),
                }],
                ..pass(Enemy)
            },
        );
        assert_eq!(
            railgun(&covered),
            Some((0, stats(hp(S::SignalBooster), 0), true))
        );
        assert!(!board.ship(target).marked);

        board[slot(Enemy, Strike, 3)] = None;
        board.ship_mut(target).marked = true;
        let decoyed = resolve(
            &mut board,
            pass(Player),
            Turn {
                power: power(PowerId::Decoy, at(Strike, 3)),
                ..pass(Enemy)
            },
        );
        let (amount, _, blocked) = railgun(&decoyed).unwrap();
        assert_eq!((amount, blocked), (ABILITY_VALUES.railgun, false));
    }

    #[test]
    fn railgun_bewegt_sich_die_pendulum_faellt_der_schuss_aus() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 2, S::Pendulum);
        let target = place(&mut board, Enemy, Strike, 4, S::TitanCarrier);
        let marking = resolve(
            &mut board,
            Turn {
                mark: Some(at(Strike, 4)),
                maneuver: maneuver(at(Command, 2), at(Command, 3)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(has(&marking, "mark"));
        assert!(!board.ship(target).marked);

        board.ship_mut(target).marked = true;
        let firing = resolve(
            &mut board,
            Turn {
                maneuver: maneuver(at(Command, 3), at(Command, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(!has(&firing, "railgun"));
        assert!(!board.ship(target).marked);
    }

    #[test]
    fn railgun_markierung_folgt_dem_schiff_ein_zerstoertes_ziel_feuert_noch() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 2, S::Pendulum);
        place(&mut board, Enemy, Strike, 4, S::TitanCarrier);
        resolve(
            &mut board,
            Turn {
                mark: Some(at(Strike, 4)),
                ..pass(Player)
            },
            Turn {
                maneuver: maneuver(at(Strike, 4), at(Command, 4)),
                ..pass(Enemy)
            },
        );
        assert!(board.ship(slot(Enemy, Command, 4)).marked);

        let mut fragile = Board::new();
        place(&mut fragile, Player, Command, 2, S::Pendulum);
        let own = place(&mut fragile, Player, Strike, 2, S::HorizonCruiser);
        let marked = place(&mut fragile, Enemy, Strike, 2, S::VoidInterceptor);
        fragile.ship_mut(marked).marked = true;
        let events = resolve(
            &mut fragile,
            pass(Player),
            Turn {
                fire: vec![fire(at(Strike, 2), at(Strike, 2))],
                ..pass(Enemy)
            },
        );
        assert_eq!(
            railgun(&events).map(|(_, after, _)| after.hp),
            Some(hp(S::VoidInterceptor) - ABILITY_VALUES.railgun)
        );
        let shots: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.target))
            .collect();
        assert_eq!(shots, [(S::VoidInterceptor, ship_ref(&fragile, own))]);
        assert!(!has(&events, "silenced"));
        assert_eq!(hp_at(&fragile, own), hp(S::HorizonCruiser) - 3);
    }
}

mod admiralsfaehigkeiten {
    use super::*;

    #[test]
    fn gegenfeuer_wer_das_schiff_trifft_nimmt_je_treffer_1_ein_flaggschiff_nicht() {
        let mut board = Board::new();
        let guard = place(&mut board, Player, Strike, 2, S::IronDreadnought);
        let near = place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let far = place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let vanguard = place(&mut board, Enemy, Command, 0, S::Vanguard);
        let at_guard = [at(Strike, 1), at(Strike, 2), at(Command, 0)]
            .map(|shooter| fire(shooter, at(Strike, 2)))
            .to_vec();
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::Counterfire, at(Strike, 2)),
                fire: vec![fire(at(Strike, 2), at(Command, 0))],
                ..pass(Player)
            },
            Turn {
                fire: at_guard,
                ..pass(Enemy)
            },
        );
        assert!(events.contains(&RoundEvent::Power {
            side: Player,
            power: PowerId::Counterfire,
            target: slot(Player, Strike, 2)
        }));
        let shooters: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| shot.attacker.card_id)
            .collect();
        assert_eq!(
            shooters,
            [S::HorizonCruiser, S::HorizonCruiser, S::Vanguard]
        );
        let counterfire: Vec<_> = events
            .iter()
            .copied()
            .filter(|event| matches!(event, RoundEvent::Counterfire { .. }))
            .collect();
        let source = ship_ref(&board, guard);
        assert_eq!(
            counterfire,
            [
                RoundEvent::Counterfire {
                    source,
                    ship: ship_ref(&board, far),
                    amount: 1,
                    after: stats(hp(S::HorizonCruiser) - 1, 0)
                },
                RoundEvent::Counterfire {
                    source,
                    ship: ship_ref(&board, near),
                    amount: 1,
                    after: stats(hp(S::HorizonCruiser) - 1, 0)
                },
            ]
        );
        // Den Treffer des Dreadnoughts nimmt die Vanguard, Gegenfeuer nicht.
        assert_eq!(hp_at(&board, vanguard), hp(S::Vanguard) - 2);
    }

    #[test]
    fn fokusfeuer_jeder_treffer_auf_das_ziel_macht_1_mehr_auf_das_zerstoerte_verpufft_er() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Player, Strike, 1, S::IronDreadnought);
        place(&mut board, Player, Strike, 2, S::VoidInterceptor);
        place(&mut board, Player, Strike, 3, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 5, S::ScrapRaider);
        place(&mut board, Enemy, Strike, 0, S::TitanCarrier);
        let raider = at(Strike, 5);
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::FocusFire, raider),
                fire: vec![
                    fire(at(Strike, 0), raider),
                    fire(at(Strike, 1), raider),
                    fire(at(Strike, 2), at(Strike, 0)),
                    fire(at(Strike, 3), raider),
                ],
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| {
                (
                    shot.attacker.card_id,
                    shot.target.card_id,
                    shot.amount,
                    shot.focus,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (S::HorizonCruiser, S::ScrapRaider, 3, true),
                (S::IronDreadnought, S::ScrapRaider, 3, true),
                (S::VoidInterceptor, S::TitanCarrier, 3, false),
            ]
        );
        let fizzles: Vec<_> = fizzles_by(&events, Player)
            .iter()
            .map(|(attacker, _)| attacker.slot_index)
            .collect();
        assert_eq!(fizzles, [3]);
    }

    #[test]
    fn fokusfeuer_ein_schuss_den_die_deckung_abfaengt_bekommt_keinen_bonus() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 3, S::TitanCarrier);
        let covered = place(&mut board, Enemy, Command, 3, S::SignalBooster);
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::FocusFire, at(Command, 3)),
                fire: vec![fire(at(Strike, 0), at(Command, 3))],
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.target.card_id, shot.amount, shot.focus))
            .collect();
        assert_eq!(summary, [(S::TitanCarrier, 2, false)]);
        assert_eq!(hp_at(&board, covered), hp(S::SignalBooster));
    }

    #[test]
    fn schwarmangriff_drohnen_feuern_ohne_feuerbefehl_durch_deckung_ist_es_zerstoert_verpufft_der_rest()
     {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::Drone);
        place(&mut board, Player, Strike, 1, S::Drone);
        place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 3, S::TitanCarrier);
        place_hp(&mut board, Enemy, Command, 3, S::SignalBooster, 2);
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::SwarmStrike, at(Command, 3)),
                fire: vec![
                    fire(at(Strike, 1), at(Strike, 3)),
                    fire(at(Strike, 2), at(Strike, 3)),
                ],
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| {
                (
                    shot.attacker.card_id,
                    shot.target.card_id,
                    shot.amount,
                    shot.swarm,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (S::Drone, S::SignalBooster, 3, true),
                (S::HorizonCruiser, S::TitanCarrier, 2, false)
            ]
        );
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship(Player, Strike, 1, S::Drone),
                Miss::Target {
                    target: Some(ship(Enemy, Command, 3, S::SignalBooster)),
                    covered: false
                }
            )]
        );
    }

    #[test]
    fn trugbild_erscheint_nach_den_deploys_und_faengt_die_schuesse_auf_seine_spalte_ab() {
        let mut board = Board::new();
        let hidden = place(&mut board, Player, Command, 3, S::RepairDrone);
        place(&mut board, Enemy, Strike, 0, S::VoidInterceptor);
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::Decoy, at(Strike, 3)),
                ..pass(Player)
            },
            Turn {
                fire: vec![fire(at(Strike, 0), at(Command, 3))],
                ..pass(Enemy)
            },
        );
        let decoy = ship(Player, Strike, 3, S::Decoy);
        assert_eq!(
            events[..4],
            [
                RoundEvent::Pass { side: Player },
                RoundEvent::Pass { side: Enemy },
                RoundEvent::Power {
                    side: Player,
                    power: PowerId::Decoy,
                    target: slot(Player, Strike, 3)
                },
                RoundEvent::Decoy { ship: decoy },
            ]
        );
        let shots: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.target, shot.through))
            .collect();
        assert_eq!(shots, [(decoy, false)]);
        assert!(events.contains(&RoundEvent::Destroyed { ship: decoy }));
        assert!(!has(&events, "vanish"));
        assert_eq!(hp_at(&board, hidden), hp(S::RepairDrone));
    }

    #[test]
    fn flaggschiffe_schiessen_am_trugbild_vorbei_steht_keins_dahinter_treffen_sie_es() {
        let cases = [
            (Some(S::RepairDrone), (Command, S::RepairDrone)),
            (None, (Strike, S::Decoy)),
        ];
        for (behind, (lane, target)) in cases {
            let mut board = Board::new();
            let decoy = place(&mut board, Player, Strike, 3, S::Decoy);
            if let Some(behind) = behind {
                place(&mut board, Player, Command, 3, behind);
            }
            let vanguard = place(&mut board, Enemy, Strike, 0, S::Vanguard);
            aim(&mut board, vanguard, decoy);
            let targets: Vec<_> = attacks_by(&round(&mut board, &[]), Enemy)
                .iter()
                .map(|shot| {
                    (
                        shot.target.slot_index,
                        shot.target.lane,
                        shot.target.card_id,
                    )
                })
                .collect();
            assert_eq!(targets, [(3, lane, target)]);
        }
    }

    #[test]
    fn trugbild_verschwindet_am_rundenende_und_zaehlt_zu_keiner_grenze() {
        let mut board = Board::new();
        for index in 0..4 {
            place(&mut board, Player, Strike, index, S::HorizonCruiser);
        }
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::Decoy, at(Strike, 4)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert_eq!(
            events.last(),
            Some(&RoundEvent::Vanish {
                ship: ship(Player, Strike, 4, S::Decoy)
            })
        );
        assert!(!board.is_occupied(slot(Player, Strike, 4)));
        assert_eq!(board.lane_ship_count(Player, Strike), 4);
    }

    #[test]
    fn trugbild_flaggschiffe_durchschauen_es_und_es_deckt_nicht_gegen_sie() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 3, S::RepairDrone);
        place(&mut board, Enemy, Strike, 0, S::VoidInterceptor);
        place(&mut board, Enemy, Strike, 3, S::Vanguard);
        let at_hidden = vec![
            fire(at(Strike, 0), at(Command, 3)),
            fire(at(Strike, 3), at(Command, 3)),
        ];
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::Decoy, at(Strike, 3)),
                ..pass(Player)
            },
            Turn {
                fire: at_hidden,
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.target.card_id))
            .collect();
        assert_eq!(
            summary,
            [
                (S::VoidInterceptor, S::Decoy),
                (S::Vanguard, S::RepairDrone)
            ]
        );
    }

    #[test]
    fn taktvorsprung_das_schiff_feuert_vor_allen_wer_dabei_faellt_feuert_nicht_mehr() {
        let mut board = Board::new();
        let quick = place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        place_hp(&mut board, Enemy, Strike, 0, S::VoidInterceptor, 3);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let at_quick = vec![
            fire(at(Strike, 0), at(Strike, 0)),
            fire(at(Strike, 1), at(Strike, 0)),
        ];
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::HeadStart, at(Strike, 0)),
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Player)
            },
            Turn {
                fire: at_quick,
                ..pass(Enemy)
            },
        );
        let quick_ref = ship_ref(&board, quick);
        let relevant: Vec<_> = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RoundEvent::Attack { .. } | RoundEvent::Silenced { .. }
                )
            })
            .collect();
        assert!(matches!(
            relevant[..],
            [
                RoundEvent::Attack {
                    attacker: ShipRef { side: Player, card_id: S::VoidInterceptor, .. },
                    target: ShipRef { side: Enemy, slot_index: 0, .. },
                    head_start: true,
                    ..
                },
                RoundEvent::Silenced {
                    ship: ShipRef { side: Enemy, lane: Strike, slot_index: 0, card_id: S::VoidInterceptor }
                },
                RoundEvent::Attack {
                    attacker: ShipRef { side: Enemy, card_id: S::HorizonCruiser, .. },
                    target,
                    ..
                },
            ] if *target == quick_ref
        ));
        assert_eq!(hp_at(&board, quick), hp(S::VoidInterceptor) - 2);
    }

    #[test]
    fn taktvorsprung_auf_beiden_seiten_diese_schiffe_feuern_zuerst_und_gleichzeitig() {
        let mut board = Board::new();
        place_hp(&mut board, Player, Strike, 0, S::VoidInterceptor, 3);
        place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        place_hp(&mut board, Enemy, Strike, 0, S::VoidInterceptor, 3);
        let head_start = power(PowerId::HeadStart, at(Strike, 0));
        let events = resolve(
            &mut board,
            Turn {
                power: head_start,
                fire: vec![
                    fire(at(Strike, 0), at(Strike, 0)),
                    fire(at(Strike, 1), at(Strike, 0)),
                ],
                ..pass(Player)
            },
            Turn {
                power: head_start,
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Enemy)
            },
        );
        let relevant: Vec<_> = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RoundEvent::Attack { .. }
                        | RoundEvent::Silenced { .. }
                        | RoundEvent::Fizzle { .. }
                )
            })
            .collect();
        assert!(matches!(
            relevant[..],
            [
                RoundEvent::Attack {
                    attacker: ShipRef {
                        side: Player,
                        card_id: S::VoidInterceptor,
                        ..
                    },
                    head_start: true,
                    ..
                },
                RoundEvent::Attack {
                    attacker: ShipRef {
                        side: Enemy,
                        card_id: S::VoidInterceptor,
                        ..
                    },
                    head_start: true,
                    ..
                },
                RoundEvent::Fizzle {
                    attacker: ShipRef {
                        side: Player,
                        card_id: S::HorizonCruiser,
                        ..
                    },
                    ..
                },
            ]
        ));
        assert_eq!(count(&events, "destroyed"), 2);
    }

    #[test]
    fn die_marke_einer_faehigkeit_folgt_dem_schiff() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 0, S::TitanCarrier);
        place(&mut board, Enemy, Strike, 3, S::ScrapRaider);
        let events = resolve(
            &mut board,
            // Gezielt wird auf die Spalte, in die der Raider ausweicht.
            Turn {
                power: power(PowerId::FocusFire, at(Strike, 3)),
                fire: vec![fire(at(Strike, 0), at(Strike, 5))],
                ..pass(Player)
            },
            Turn {
                maneuver: maneuver(at(Strike, 3), at(Strike, 5)),
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.target, shot.amount, shot.focus))
            .collect();
        assert_eq!(summary, [(ship(Enemy, Strike, 5, S::ScrapRaider), 3, true)]);
    }
}

mod feuerbefehle {
    use super::*;

    #[test]
    fn ein_schiff_das_in_dieser_runde_landet_feuert_schon_im_kampf() {
        let mut board = Board::new();
        let target = place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                ships: vec![PlannedArrival {
                    card: S::VoidInterceptor,
                    target: at(Strike, 4),
                }],
                fire: vec![fire(at(Strike, 4), at(Strike, 1))],
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker, shot.target, shot.amount))
            .collect();
        assert_eq!(
            summary,
            [(
                ship(Player, Strike, 4, S::VoidInterceptor),
                ship_ref(&board, target),
                3
            )]
        );
    }

    #[test]
    fn faellt_die_deckung_schon_vor_dem_kampf_trifft_der_schuss_das_schiff_dahinter() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::PlasmaFrigate);
        let guard = place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
        let flagship = place(&mut board, Enemy, Command, 2, S::Pendulum);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::Torpedo, at(Strike, 2)),
                fire: vec![fire(at(Strike, 0), at(Command, 2))],
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(!board.is_occupied(guard));
        assert_eq!(
            attacks_by(&events, Player),
            [Shot::plain(
                ship(Player, Strike, 0, S::PlasmaFrigate),
                ship_ref(&board, flagship),
                3,
                stats(hp(S::Pendulum) - 3, 0)
            )]
        );
    }

    #[test]
    fn der_feuerbefehl_folgt_dem_schuetzen_auch_wenn_der_gegner_ihn_per_traktorstrahl_verlegt() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 1, S::HorizonCruiser);
        let target = place(&mut board, Enemy, Strike, 4, S::TitanCarrier);
        let events = resolve(
            &mut board,
            Turn {
                fire: vec![fire(at(Command, 1), at(Strike, 4))],
                ..pass(Player)
            },
            Turn {
                command: order_at(C::TractorBeam, at(Command, 1)),
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker.lane, shot.attacker.slot_index, shot.target))
            .collect();
        assert_eq!(summary, [(Strike, 1, ship_ref(&board, target))]);
    }

    #[test]
    fn feuerbefehle_beider_seiten_gehen_gleich_aus_mit_vertauschten_sitzen() {
        let (_, events) = mirrored(
            |board, a, b| {
                place(board, a, Strike, 0, S::VoidInterceptor);
                place(board, a, Strike, 1, S::HorizonCruiser);
                place_hp(board, b, Strike, 3, S::HorizonCruiser, 3);
                place(board, b, Command, 3, S::SignalBooster);
                place(board, b, Strike, 4, S::ScrapRaider);
            },
            Turn {
                fire: vec![
                    fire(at(Strike, 0), at(Strike, 3)),
                    fire(at(Strike, 1), at(Command, 3)),
                ],
                ..pass(Player)
            },
            Turn {
                fire: vec![
                    fire(at(Strike, 3), at(Strike, 0)),
                    fire(at(Strike, 4), at(Strike, 1)),
                ],
                ..pass(Enemy)
            },
        );
        let through: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| shot.through)
            .collect();
        assert_eq!(through, [false, true]);
        assert_eq!(fizzles_by(&events, Player).len(), 0);
        assert_eq!(attacks_by(&events, Enemy).len(), 2);
    }
}

mod schiffsfeuer {
    use super::*;

    #[test]
    fn der_schuss_folgt_seinem_ziel_spaltenfeuer_trifft_was_in_der_spalte_bleibt() {
        let mut board = Board::new();
        let hunter = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let gunner = place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        let mover = place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
        let behind = place(&mut board, Enemy, Command, 2, S::SignalBooster);
        lock(&mut board, hunter, mover);
        aim(&mut board, gunner, mover);
        let to = slot(Enemy, Strike, 5);
        resolve_maneuver(&mut board, mover, to, &mut Vec::new());
        let events = round(&mut board, &[]);
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| {
                (
                    shot.attacker.slot_index,
                    shot.target.card_id,
                    shot.target.slot_index,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [(0, S::VoidInterceptor, 5), (1, S::SignalBooster, 2)]
        );
        assert_eq!(hp_at(&board, to), hp(S::VoidInterceptor) - 2);
        assert_eq!(hp_at(&board, behind), hp(S::SignalBooster) - 2);
    }

    #[test]
    fn steht_vor_dem_ziel_ein_schiff_verpufft_der_schuss_die_deckung_bleibt_unberuehrt() {
        let mut board = Board::new();
        let shooter = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let front = place(&mut board, Enemy, Strike, 3, S::TitanCarrier);
        let covered = place(&mut board, Enemy, Command, 3, S::SignalBooster);
        lock(&mut board, shooter, covered);
        let events = round(&mut board, &[]);
        assert_eq!(attacks_by(&events, Player), []);
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship_ref(&board, shooter),
                Miss::Target {
                    target: Some(ship_ref(&board, covered)),
                    covered: true
                }
            )]
        );
        assert_eq!(hp_at(&board, front), hp(S::TitanCarrier));
        assert_eq!(hp_at(&board, covered), hp(S::SignalBooster));
    }

    #[test]
    fn die_deckung_zaehlt_im_moment_des_schusses_ist_sie_schon_gefallen_trifft_er() {
        // Strike-Schiffe feuern vor Command-Schiffen, jeweils in Slot-Reihenfolge.
        let mut board = Board::new();
        let early = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let breaker = place(&mut board, Player, Strike, 1, S::VoidInterceptor);
        let late = place(&mut board, Player, Command, 0, S::HorizonCruiser);
        let front = place_hp(&mut board, Enemy, Strike, 3, S::ScrapRaider, 3);
        let behind = place(&mut board, Enemy, Command, 3, S::SignalBooster);
        lock(&mut board, early, behind);
        aim(&mut board, breaker, front);
        lock(&mut board, late, behind);
        let events = round(&mut board, &[]);
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship_ref(&board, early),
                Miss::Target {
                    target: Some(ship_ref(&board, behind)),
                    covered: true
                }
            )]
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.target.card_id, shot.through))
            .collect();
        assert_eq!(
            summary,
            [
                (S::VoidInterceptor, S::ScrapRaider, false),
                (S::HorizonCruiser, S::SignalBooster, false)
            ]
        );
        assert_eq!(hp_at(&board, behind), hp(S::SignalBooster) - 2);
    }

    #[test]
    fn ueberschuss_verpufft_ist_das_ziel_schon_zerstoert_geht_der_schuss_nicht_weiter() {
        let mut board = Board::new();
        let first = place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        let second = place(&mut board, Player, Strike, 1, S::VoidInterceptor);
        let front = place_hp(&mut board, Enemy, Strike, 2, S::HorizonCruiser, 3);
        let behind = place(&mut board, Enemy, Command, 2, S::SignalBooster);
        let aimed = ship_ref(&board, front);
        lock(&mut board, first, front);
        lock(&mut board, second, front);
        let events = round(&mut board, &[]);
        assert_eq!(
            fizzles_by(&events, Player),
            [(
                ship_ref(&board, second),
                Miss::Target {
                    target: Some(aimed),
                    covered: false
                }
            )]
        );
        assert!(!board.is_occupied(front));
        assert_eq!(hp_at(&board, behind), hp(S::SignalBooster));
    }

    #[test]
    fn ist_das_ziel_nicht_mehr_auf_dem_brett_etwa_im_kurzsprung_verpufft_der_schuss() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 3, S::Vanguard);
        place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Strike, 3)),
                ..pass(Player)
            },
            Turn {
                fire: vec![fire_at(at(Strike, 0), at(Strike, 3))],
                ..pass(Enemy)
            },
        );
        assert_eq!(
            fizzles_by(&events, Enemy),
            [(
                ship(Enemy, Strike, 0, S::HorizonCruiser),
                Miss::Target {
                    target: None,
                    covered: false
                }
            )]
        );
    }

    #[test]
    fn ein_trugbild_deckt_gegen_schiffsfeuer_nur_nicht_gegen_flaggschiffe() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 3, S::Decoy);
        let hidden = place(&mut board, Player, Command, 3, S::RepairDrone);
        let interceptor = place(&mut board, Enemy, Strike, 0, S::VoidInterceptor);
        let vanguard = place(&mut board, Enemy, Strike, 1, S::Vanguard);
        lock(&mut board, interceptor, hidden);
        lock(&mut board, vanguard, hidden);
        let events = round(&mut board, &[]);
        assert_eq!(
            fizzles_by(&events, Enemy),
            [(
                ship_ref(&board, interceptor),
                Miss::Target {
                    target: Some(ship_ref(&board, hidden)),
                    covered: true
                }
            )]
        );
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.target.card_id))
            .collect();
        assert_eq!(summary, [(S::Vanguard, S::RepairDrone)]);
    }

    #[test]
    fn phasentausch_schiffsfeuer_folgt_der_mirage_spaltenfeuer_trifft_den_partner() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 1, S::Mirage);
        place(&mut board, Player, Strike, 4, S::IronDreadnought);
        place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                maneuver: maneuver(at(Strike, 1), at(Strike, 4)),
                ..pass(Player)
            },
            Turn {
                fire: vec![
                    fire_at(at(Strike, 0), at(Strike, 1)),
                    fire(at(Strike, 2), at(Strike, 1)),
                ],
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.target.card_id, shot.target.slot_index))
            .collect();
        assert_eq!(summary, [(S::Mirage, 4), (S::IronDreadnought, 1)]);
    }
}

// --- Neue Befehle ---

mod umgruppierung {
    use super::*;

    #[test]
    fn ausweichmanoever_verlegt_in_derselben_lane_gibt_schild_2_der_waermesucher_folgt_mit_5() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_to(C::EvasiveAction, at(Strike, 1), 4),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::HeatSeeker, at(Strike, 1)),
                ..pass(Enemy)
            },
        );
        assert!(events.contains(&RoundEvent::Command {
            side: Enemy,
            card_id: C::HeatSeeker,
            target: Some(slot(Player, Strike, 1))
        }));
        let applied: Vec<_> = [
            effects_of(&events, C::EvasiveAction),
            effects_of(&events, C::HeatSeeker),
        ]
        .concat()
        .iter()
        .map(|effect| {
            (
                effect.ship.side,
                effect.ship.slot_index,
                effect.effect,
                effect.after,
            )
        })
        .collect();
        assert_eq!(
            applied,
            [
                (
                    Player,
                    1,
                    CommandEffect::Move { to: at(Strike, 4) },
                    stats(8, 0)
                ),
                (
                    Player,
                    4,
                    CommandEffect::Shield {
                        amount: 2,
                        permanent: false
                    },
                    stats(8, 2)
                ),
                (Player, 4, CommandEffect::Damage { amount: 5 }, stats(5, 0)),
            ]
        );
        assert_eq!(hp_at(&board, slot(Player, Strike, 4)), 5);
    }

    #[test]
    fn kurzsprung_torpedo_trifft_ins_leere_schiff_feuert_nicht_und_kehrt_samt_zustand_zurueck() {
        let mut board = Board::new();
        let raider = place_hp(&mut board, Player, Strike, 2, S::ScrapRaider, 4);
        board.ship_mut(raider).shield = 1;
        board.ship_mut(raider).scrap_bonus = 1;
        place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Strike, 2)),
                fire: vec![fire(at(Strike, 2), at(Strike, 2))],
                ..pass(Player)
            },
            Turn {
                command: order_at(C::Torpedo, at(Strike, 2)),
                ..pass(Enemy)
            },
        );
        let raider_ref = ship(Player, Strike, 2, S::ScrapRaider);
        assert_eq!(
            effects_of(&events, C::MicroJump),
            [Effect {
                side: Player,
                ship: raider_ref,
                effect: CommandEffect::Jump,
                after: stats(4, 1)
            }]
        );
        assert!(events.contains(&RoundEvent::Miss {
            side: Enemy,
            card_id: C::Torpedo,
            target: slot(Player, Strike, 2)
        }));
        assert!(!has(&events, "attack") && !has(&events, "fizzle"));
        assert!(events.contains(&RoundEvent::Return {
            ship: raider_ref,
            after: stats(4, 1)
        }));
        assert_eq!(
            board.state(raider),
            Some(SlotState {
                card_id: S::ScrapRaider,
                hp: 4,
                shield: 1,
                scrap_bonus: 1,
                marked: false
            })
        );
        assert!(board.jumping.is_empty());
    }

    #[test]
    fn kurzsprung_springendes_flaggschiff_entgeht_toedlichem_schaden() {
        let mut board = Board::new();
        let vanguard = place_hp(&mut board, Player, Strike, 3, S::Vanguard, 1);
        let behind = place(&mut board, Player, Command, 3, S::SignalBooster);
        place(&mut board, Enemy, Strike, 3, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 4, S::VoidInterceptor);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Strike, 3)),
                ..pass(Player)
            },
            Turn {
                command: order(C::VolleyFire),
                fire: vec![
                    fire(at(Strike, 3), at(Command, 3)),
                    fire(at(Strike, 4), at(Strike, 3)),
                ],
                ..pass(Enemy)
            },
        );
        assert_eq!(board.flagship(Player), Some(vanguard));
        assert_eq!(hp_at(&board, vanguard), 1);
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.target.lane, shot.target.slot_index, shot.amount))
            .collect();
        assert_eq!(summary, [(Command, 3, 2), (Command, 3, 3)]);
        assert_eq!(fizzles_by(&events, Enemy), []);
        assert!(!board.is_occupied(behind));
    }

    #[test]
    fn kurzsprung_die_pendulum_verliert_ihren_schuss_die_markierung_ist_verbraucht() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 2, S::Pendulum);
        let marked = place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        board.ship_mut(marked).marked = true;
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Command, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(!has(&events, "railgun"));
        assert_eq!(hp_at(&board, marked), hp(S::HorizonCruiser));
        assert!(!board.ship(marked).marked);
    }

    #[test]
    fn kurzsprung_der_slot_bleibt_gesperrt_der_traktorstrahl_zieht_in_den_naechsten_freien() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        place(&mut board, Player, Command, 2, S::RepairDrone);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Strike, 2)),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::TractorBeam, at(Command, 2)),
                ..pass(Enemy)
            },
        );
        let pulled: Vec<_> = effects_of(&events, C::TractorBeam)
            .iter()
            .map(|effect| (effect.side, effect.ship.slot(), effect.effect))
            .collect();
        assert_eq!(
            pulled,
            [(
                Enemy,
                slot(Player, Command, 2),
                CommandEffect::Move { to: at(Strike, 1) }
            )]
        );
        assert_eq!(card(&board, slot(Player, Strike, 1)), Some(S::RepairDrone));
        assert_eq!(
            card(&board, slot(Player, Strike, 2)),
            Some(S::HorizonCruiser)
        );
    }

    #[test]
    fn kurzsprung_waermesucher_und_traktorstrahl_verpuffen_wenn_ihr_schiff_springt() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 2, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::MicroJump, at(Strike, 2)),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::HeatSeeker, at(Strike, 2)),
                ..pass(Enemy)
            },
        );
        assert!(events.contains(&RoundEvent::NoEffect {
            side: Enemy,
            card_id: C::HeatSeeker
        }));
        assert_eq!(
            hp_at(&board, slot(Player, Strike, 2)),
            hp(S::HorizonCruiser)
        );
    }
}

mod taktik {
    use super::*;

    #[test]
    fn schnellverlegung_und_freie_hand_melden_nur_rule() {
        let mut board = Board::new();
        let events = resolve(
            &mut board,
            Turn {
                command: order(C::RapidDeployment),
                ..pass(Player)
            },
            Turn {
                command: order(C::CarteBlanche),
                ..pass(Enemy)
            },
        );
        let rules: Vec<_> = events
            .into_iter()
            .filter(|event| matches!(event, RoundEvent::Rule { .. }))
            .collect();
        assert_eq!(
            rules,
            [
                RoundEvent::Rule {
                    side: Player,
                    card_id: C::RapidDeployment
                },
                RoundEvent::Rule {
                    side: Enemy,
                    card_id: C::CarteBlanche
                },
            ]
        );
    }

    #[test]
    fn ablenkung_zieht_die_schuesse_mit_feuerbefehl_auf_sich_solange_es_steht() {
        let mut board = Board::new();
        let decoy = place_hp(&mut board, Player, Strike, 0, S::HorizonCruiser, 3);
        let drone = place(&mut board, Player, Strike, 4, S::RepairDrone);
        place(&mut board, Enemy, Strike, 4, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 3, S::VoidInterceptor);
        place(&mut board, Enemy, Strike, 5, S::VoidInterceptor);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::DrawFire, at(Strike, 0)),
                ..pass(Player)
            },
            Turn {
                fire: vec![
                    fire_at(at(Strike, 3), at(Strike, 4)),
                    fire(at(Strike, 4), at(Strike, 4)),
                ],
                ..pass(Enemy)
            },
        );
        let taunts: Vec<_> = effects_of(&events, C::DrawFire)
            .iter()
            .map(|effect| (effect.ship.slot_index, effect.effect))
            .collect();
        assert_eq!(taunts, [(0, CommandEffect::Taunt)]);
        // Der Jäger in Slot 3 fällt auf die Ablenkung herein und zerstört sie; der Kreuzer trifft danach sein Ziel.
        // Wer keinen Feuerbefehl hat, feuert nicht.
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.attacker.slot_index, shot.target.slot_index, shot.taunt))
            .collect();
        assert_eq!(summary, [(3, 0, true), (4, 4, false)]);
        assert!(!board.is_occupied(decoy));
        assert_eq!(hp_at(&board, drone), hp(S::RepairDrone) - 2);
    }

    #[test]
    fn ablenkung_ein_gedecktes_schiff_zieht_kein_feuer() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::IronDreadnought);
        place(&mut board, Player, Command, 0, S::RepairDrone);
        place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::DrawFire, at(Command, 0)),
                ..pass(Player)
            },
            Turn {
                fire: vec![fire(at(Strike, 2), at(Strike, 0))],
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = attacks_by(&events, Enemy)
            .iter()
            .map(|shot| (shot.target.lane, shot.target.slot_index, shot.taunt))
            .collect();
        assert_eq!(summary, [(Strike, 0, false)]);
    }
}

mod verstaerkung {
    use super::*;

    #[test]
    fn schildgitter_alle_eigenen_schiffe_auch_tokens_bekommen_schild_1() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Player, Strike, 1, S::Drone);
        place(&mut board, Player, Command, 0, S::RepairDrone);
        let events = round(&mut board, &[play(Player, C::ShieldGrid, None, None)]);
        let shield = CommandEffect::Shield {
            amount: 1,
            permanent: false,
        };
        let summary: Vec<_> = effects_of(&events, C::ShieldGrid)
            .iter()
            .map(|effect| (effect.ship.card_id, effect.effect))
            .collect();
        assert_eq!(
            summary,
            [
                (S::HorizonCruiser, shield),
                (S::Drone, shield),
                (S::RepairDrone, shield)
            ]
        );
        let shields: Vec<_> = board
            .ships(Player)
            .map(|at| board.ship(at).shield)
            .collect();
        assert_eq!(shields, [0, 0, 0]);
    }

    #[test]
    fn ablativpanzerung_schild_2_der_am_rundenende_bleibt() {
        let mut board = Board::new();
        let target = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let events = round(
            &mut board,
            &[play(Player, C::AblativeArmor, Some(target), None)],
        );
        let summary: Vec<_> = effects_of(&events, C::AblativeArmor)
            .iter()
            .map(|effect| (effect.effect, effect.after))
            .collect();
        assert_eq!(
            summary,
            [(
                CommandEffect::Shield {
                    amount: 2,
                    permanent: true
                },
                stats(8, 2)
            )]
        );
        assert_eq!(board.state(target).map(|state| state.shield), Some(2));
    }

    #[test]
    fn feuerschutz_ziel_und_partner_bekommen_1_atk_und_schild_1() {
        let mut board = Board::new();
        let front = place(&mut board, Player, Strike, 1, S::HorizonCruiser);
        let back = place(&mut board, Player, Command, 1, S::PlasmaFrigate);
        place(&mut board, Player, Command, 2, S::VoidInterceptor);
        let events = effects(
            &mut board,
            &[play(Player, C::CoveringFire, Some(back), None)],
        );
        let attack = CommandEffect::Attack { delta: 1 };
        let shield = CommandEffect::Shield {
            amount: 1,
            permanent: false,
        };
        let summary: Vec<_> = effects_of(&events, C::CoveringFire)
            .iter()
            .map(|effect| (effect.ship.lane, effect.effect))
            .collect();
        assert_eq!(
            summary,
            [
                (Command, attack),
                (Command, shield),
                (Strike, attack),
                (Strike, shield)
            ]
        );
        let values = [
            board.attack(front),
            board.ship(front).shield,
            board.attack(back),
            board.ship(back).shield,
        ];
        assert_eq!(values, [3, 1, 4, 1]);
    }

    #[test]
    fn drohnenstaffel_vor_das_ungedeckte_flaggschiff_dann_vor_command_schiffe_sonst_zur_mitte() {
        let mut board = Board::new();
        place(&mut board, Player, Command, 4, S::Bastion);
        place(&mut board, Player, Command, 1, S::RepairDrone);
        place(&mut board, Player, Command, 0, S::HorizonCruiser);
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let events = effects(&mut board, &[play(Player, C::DroneSquadron, None, None)]);
        let spawn = |index| RoundEvent::Effect {
            side: Player,
            card_id: C::DroneSquadron,
            ship: ship(Player, Strike, index, S::Drone),
            effect: CommandEffect::Spawn,
            after: stats(1, 0),
        };
        let spawned: Vec<_> = events
            .iter()
            .copied()
            .filter(|event| {
                matches!(
                    event,
                    RoundEvent::Effect {
                        card_id: C::DroneSquadron,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(spawned, [spawn(4), spawn(1)]);
        assert!(board.ship(slot(Player, Strike, 4)).arrived);

        let mut empty = Board::new();
        effects(&mut empty, &[play(Player, C::DroneSquadron, None, None)]);
        let indices: Vec<_> = empty.ships(Player).map(|at| at.slot_index).collect();
        assert_eq!(indices, [2, 3]);

        let mut full = Board::new();
        for index in 0..6 {
            place(&mut full, Player, Strike, index, S::Drone);
        }
        let none = effects(&mut full, &[play(Player, C::DroneSquadron, None, None)]);
        assert_eq!(
            none,
            [RoundEvent::NoEffect {
                side: Player,
                card_id: C::DroneSquadron
            }]
        );
    }

    #[test]
    fn drohnenstaffel_drohnen_haben_keinen_feuerbefehl_feuern_nur_per_schwarmangriff() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let idle = resolve(
            &mut board,
            Turn {
                command: order(C::DroneSquadron),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(!has(&idle, "attack") && !has(&idle, "fizzle"));

        let mut swarming = Board::new();
        place(&mut swarming, Enemy, Strike, 2, S::HorizonCruiser);
        let events = resolve(
            &mut swarming,
            Turn {
                command: order(C::DroneSquadron),
                power: power(PowerId::SwarmStrike, at(Strike, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| (shot.attacker.card_id, shot.amount, shot.swarm))
            .collect();
        assert_eq!(summary, [(S::Drone, 3, true), (S::Drone, 3, true)]);
    }
}

mod spezial {
    use super::*;

    fn rams(events: &[RoundEvent]) -> Vec<(ShipRef, i32, i32)> {
        effects_of(events, C::CollisionCourse)
            .iter()
            .map(|effect| match effect.effect {
                CommandEffect::Ram {
                    target,
                    dealt,
                    taken,
                    ..
                } => (target, dealt, taken),
                other => panic!("kein Rammstoß: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn rammkurs_ziel_und_rammer_nehmen_die_hp_des_anderen_der_rammer_feuert_nicht() {
        let mut board = Board::new();
        place_hp(&mut board, Player, Strike, 1, S::HorizonCruiser, 6);
        place_hp(&mut board, Enemy, Strike, 2, S::IronDreadnought, 5);
        place(&mut board, Enemy, Command, 1, S::RepairDrone);
        place(&mut board, Enemy, Strike, 1, S::SignalBooster);
        place(&mut board, Player, Strike, 4, S::PlasmaFrigate);
        let at_target = vec![
            fire(at(Strike, 1), at(Strike, 2)),
            fire(at(Strike, 4), at(Strike, 2)),
        ];
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 1)),
                fire: at_target,
                ..pass(Player)
            },
            pass(Enemy),
        );
        // Gleiche Slot-Nummer zuerst: der Signal Booster vor der gedeckten Repair Drone.
        assert_eq!(
            effects_of(&events, C::CollisionCourse),
            [Effect {
                side: Player,
                ship: ship(Player, Strike, 1, S::HorizonCruiser),
                effect: CommandEffect::Ram {
                    target: ship(Enemy, Strike, 1, S::SignalBooster),
                    dealt: 6,
                    taken: 5,
                    target_after: stats(-1, 0),
                },
                after: stats(1, 0),
            }]
        );
        let shooters: Vec<_> = attacks_by(&events, Player)
            .iter()
            .map(|shot| shot.attacker.card_id)
            .collect();
        assert_eq!(shooters, [S::PlasmaFrigate]);
    }

    #[test]
    fn rammkurs_sonst_das_naechstgelegene_ungedeckte_schiff_der_schild_faengt_ab() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 1, S::ScrapRaider);
        let dreadnought = place(&mut board, Enemy, Strike, 3, S::IronDreadnought);
        place(&mut board, Enemy, Command, 0, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 1)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let summary: Vec<_> = rams(&events)
            .iter()
            .map(|(target, dealt, taken)| (target.slot_index, target.lane, *dealt, *taken))
            .collect();
        assert_eq!(summary, [(0, Command, 6, 8)]);
        assert_eq!(hp_at(&board, dreadnought), hp(S::IronDreadnought));

        let mut shielded = Board::new();
        place(&mut shielded, Player, Strike, 1, S::ScrapRaider);
        let tough = place(&mut shielded, Enemy, Strike, 1, S::IronDreadnought);
        resolve(
            &mut shielded,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 1)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let ship = shielded.ship(tough);
        assert_eq!((ship.hp, ship.shield), (hp(S::IronDreadnought) - 4, 0));
    }

    #[test]
    fn rammkurs_gegen_ein_trugbild_es_faellt_ohne_schrott_der_rammer_nimmt_1() {
        let mut board = Board::new();
        let rammer = place(&mut board, Player, Strike, 2, S::ScrapRaider);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 2)),
                ..pass(Player)
            },
            Turn {
                power: power(PowerId::Decoy, at(Strike, 2)),
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = rams(&events)
            .iter()
            .map(|(target, dealt, taken)| (target.card_id, *dealt, *taken))
            .collect();
        assert_eq!(summary, [(S::Decoy, 6, 1)]);
        let ship = board.ship(rammer);
        assert_eq!((ship.hp, ship.scrap_bonus), (5, 0));
        assert!(!has(&events, "vanish"));
    }

    #[test]
    fn rammkurs_ohne_ziel_bleibt_ohne_wirkung() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 2, S::ScrapRaider);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(events.contains(&RoundEvent::NoEffect {
            side: Player,
            card_id: C::CollisionCourse
        }));
    }

    #[test]
    fn rammkurs_gegen_rammkurs_eine_kollision_gleich_mit_vertauschten_sitzen() {
        let ram = Turn {
            command: order_at(C::CollisionCourse, at(Strike, 2)),
            ..pass(Player)
        };
        let (board, events) = mirrored(
            |board, a, b| {
                place(board, a, Strike, 2, S::ScrapRaider);
                place(board, b, Strike, 2, S::HorizonCruiser);
            },
            ram.clone(),
            ram,
        );
        assert_eq!(effects_of(&events, C::CollisionCourse).len(), 1);
        assert_eq!(board.ships(Player).count(), 0);
        assert_eq!(hp_at(&board, slot(Enemy, Strike, 2)), 2);
    }

    #[test]
    fn rammkurs_ziele_und_betraege_stehen_vor_dem_ersten_rammstoss_fest() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 1, S::ScrapRaider);
        place(&mut board, Enemy, Strike, 0, S::PlasmaFrigate);
        // Der Kreuzer rammt die Fregatte, der Raider den Kreuzer, beide mit den HP vor allen Rammstößen.
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 0)),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::CollisionCourse, at(Strike, 1)),
                ..pass(Enemy)
            },
        );
        let summary: Vec<_> = rams(&events)
            .iter()
            .map(|(target, dealt, taken)| (target.card_id, *dealt, *taken))
            .collect();
        assert_eq!(
            summary,
            [(S::PlasmaFrigate, 8, 6), (S::HorizonCruiser, 6, 8)]
        );
        assert_eq!(board.ships(Player).count(), 0);
    }

    /// Ereignisse mit Befehlskarte außer dem Aufdecken (`Command`).
    fn command_results(events: &[RoundEvent]) -> Vec<RoundEvent> {
        events
            .iter()
            .copied()
            .filter(|event| {
                matches!(
                    event,
                    RoundEvent::Effect { .. }
                        | RoundEvent::Draw { .. }
                        | RoundEvent::NoEffect { .. }
                        | RoundEvent::Countered { .. }
                        | RoundEvent::Rule { .. }
                        | RoundEvent::Miss { .. }
                )
            })
            .collect()
    }

    #[test]
    fn gegenmassnahmen_angriff_und_schwaechung_verpuffen_sonst_ziehen_sie_einen_befehl() {
        let mut board = Board::new();
        let own = place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        let torpedo = resolve(
            &mut board,
            Turn {
                command: order(C::Countermeasures),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::Torpedo, at(Strike, 0)),
                ..pass(Enemy)
            },
        );
        assert_eq!(
            command_results(&torpedo),
            [RoundEvent::Countered {
                side: Player,
                card_id: C::Countermeasures,
                countered: C::Torpedo
            }]
        );
        assert_eq!(hp_at(&board, own), hp(S::HorizonCruiser));

        place(&mut board, Enemy, Strike, 0, S::RepairDrone);
        let emp = resolve(
            &mut board,
            Turn {
                command: order(C::Countermeasures),
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Player)
            },
            Turn {
                command: order(C::EmpBlast),
                ..pass(Enemy)
            },
        );
        assert!(emp.contains(&RoundEvent::Countered {
            side: Player,
            card_id: C::Countermeasures,
            countered: C::EmpBlast
        }));
        assert_eq!(attacks_by(&emp, Player).len(), 1);

        let shields = resolve(
            &mut board,
            Turn {
                command: order(C::Countermeasures),
                ..pass(Player)
            },
            Turn {
                command: order_at(C::EmergencyShields, at(Strike, 0)),
                ..pass(Enemy)
            },
        );
        assert!(shields.contains(&RoundEvent::Draw {
            side: Player,
            card_id: C::Countermeasures,
            ships: 0,
            commands: 1
        }));
        assert_eq!(effects_of(&shields, C::EmergencyShields).len(), 1);
    }

    #[test]
    fn gegenmassnahmen_auf_beiden_seiten_beide_ziehen_gleich_mit_vertauschten_sitzen() {
        let countermeasures = Turn {
            command: order(C::Countermeasures),
            ..pass(Player)
        };
        let (_, events) = mirrored(
            |board, a, b| {
                place(board, a, Strike, 0, S::HorizonCruiser);
                place(board, b, Strike, 1, S::VoidInterceptor);
            },
            countermeasures.clone(),
            countermeasures,
        );
        assert_eq!(count(&events, "draw"), 2);
    }
}

mod schwaechung {
    use super::*;

    #[test]
    fn funkstille_entfernt_jede_gegnerische_faehigkeit_das_trugbild_verschwindet_sofort() {
        let cases = [
            (
                PowerId::Counterfire,
                at(Strike, 0),
                ship(Enemy, Strike, 0, S::HorizonCruiser),
            ),
            (
                PowerId::HeadStart,
                at(Strike, 0),
                ship(Enemy, Strike, 0, S::HorizonCruiser),
            ),
            (
                PowerId::FocusFire,
                at(Strike, 1),
                ship(Player, Strike, 1, S::HorizonCruiser),
            ),
            (
                PowerId::SwarmStrike,
                at(Strike, 1),
                ship(Player, Strike, 1, S::HorizonCruiser),
            ),
            (
                PowerId::Decoy,
                at(Strike, 3),
                ship(Enemy, Strike, 3, S::Decoy),
            ),
        ];
        for (id, target, jammed) in cases {
            let mut board = Board::new();
            place(&mut board, Player, Strike, 1, S::HorizonCruiser);
            place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
            place(&mut board, Enemy, Strike, 5, S::Drone);
            let events = resolve(
                &mut board,
                Turn {
                    command: order(C::RadioSilence),
                    fire: vec![fire(at(Strike, 1), at(Strike, 0))],
                    ..pass(Player)
                },
                Turn {
                    power: power(id, target),
                    fire: vec![
                        fire(at(Strike, 0), at(Strike, 1)),
                        fire(at(Strike, 5), at(Strike, 1)),
                    ],
                    ..pass(Enemy)
                },
            );
            let jams: Vec<_> = effects_of(&events, C::RadioSilence)
                .iter()
                .map(|effect| (effect.ship, effect.effect))
                .collect();
            assert_eq!(jams, [(jammed, CommandEffect::Jam { power: id })]);
            assert!(!has(&events, "counterfire"));
            assert!(!has(&events, "vanish"));
            assert!(
                !attacks(&events)
                    .iter()
                    .any(|shot| shot.focus || shot.swarm || shot.head_start)
            );
            assert!(!events.iter().any(|event| matches!(
                event,
                RoundEvent::Destroyed {
                    ship: ShipRef {
                        card_id: S::Decoy,
                        ..
                    }
                }
            )));
        }
        let mut board = Board::new();
        let events = resolve(
            &mut board,
            Turn {
                command: order(C::RadioSilence),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(events.contains(&RoundEvent::NoEffect {
            side: Player,
            card_id: C::RadioSilence
        }));
    }

    #[test]
    fn funkstille_im_spiegelmatch_die_eigene_marke_bleibt_gleich_mit_vertauschten_sitzen() {
        let counterfire = power(PowerId::Counterfire, at(Strike, 0));
        let (board, events) = mirrored(
            |board, a, b| {
                place(board, a, Strike, 0, S::HorizonCruiser);
                place(board, b, Strike, 0, S::HorizonCruiser);
            },
            Turn {
                command: order(C::RadioSilence),
                power: counterfire,
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Player)
            },
            Turn {
                power: counterfire,
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Player)
            },
        );
        let jams: Vec<_> = effects_of(&events, C::RadioSilence)
            .iter()
            .map(|effect| (effect.ship.side, effect.effect))
            .collect();
        assert_eq!(
            jams,
            [(
                Enemy,
                CommandEffect::Jam {
                    power: PowerId::Counterfire
                }
            )]
        );
        assert_eq!(hp_at(&board, slot(Player, Strike, 0)), 6);
        assert_eq!(hp_at(&board, slot(Enemy, Strike, 0)), 5);
    }

    #[test]
    fn minenfeld_trifft_deploys_carrier_drohne_und_trugbild_nicht_die_hive_drohne_der_vorrunde() {
        let mut board = Board::new();
        place(&mut board, Enemy, Command, 0, S::Hive);
        place(&mut board, Enemy, Strike, 0, S::Drone);
        let events = resolve(
            &mut board,
            Turn {
                command: order(C::Minefield),
                ..pass(Player)
            },
            Turn {
                ships: vec![PlannedArrival {
                    card: S::TitanCarrier,
                    target: at(Strike, 3),
                }],
                power: power(PowerId::Decoy, at(Strike, 5)),
                ..pass(Enemy)
            },
        );
        let hits: Vec<_> = effects_of(&events, C::Minefield)
            .iter()
            .map(|effect| (effect.ship.slot_index, effect.ship.card_id))
            .collect();
        assert_eq!(hits, [(2, S::Drone), (3, S::TitanCarrier), (5, S::Decoy)]);
        let again = resolve(
            &mut board,
            Turn {
                command: order(C::Minefield),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(again.contains(&RoundEvent::NoEffect {
            side: Player,
            card_id: C::Minefield
        }));
    }

    #[test]
    fn minenfeld_die_brutkammer_setzt_kein_flag_das_in_die_naechste_runde_reicht() {
        let mut board = Board::new();
        place(&mut board, Enemy, Command, 2, S::Hive);
        resolve(&mut board, pass(Player), pass(Enemy));
        let ships: Vec<_> = board
            .ships(Enemy)
            .map(|at| (board.ship(at).card, board.ship(at).arrived))
            .collect();
        assert_eq!(ships, [(S::Drone, false), (S::Hive, false)]);
    }

    #[test]
    fn traktorstrahl_zieht_in_die_andere_lane_die_lane_grenze_gilt_eine_drohne_braucht_nur_einen_slot()
     {
        let mut board = Board::new();
        let cruiser = place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::TractorBeam, at(Strike, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let pulled: Vec<_> = effects_of(&events, C::TractorBeam)
            .iter()
            .map(|effect| (effect.ship.slot(), effect.effect))
            .collect();
        assert_eq!(
            pulled,
            [(
                slot(Enemy, Strike, 2),
                CommandEffect::Move { to: at(Command, 2) }
            )]
        );
        assert!(!board.is_occupied(cruiser));
        assert_eq!(
            card(&board, slot(Enemy, Command, 2)),
            Some(S::HorizonCruiser)
        );

        let mut full = Board::new();
        for index in 0..4 {
            place(&mut full, Enemy, Command, index, S::RepairDrone);
        }
        place(&mut full, Enemy, Strike, 4, S::HorizonCruiser);
        place(&mut full, Enemy, Strike, 3, S::Drone);
        let blocked = resolve(
            &mut full,
            Turn {
                command: order_at(C::TractorBeam, at(Strike, 4)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        assert!(blocked.contains(&RoundEvent::NoEffect {
            side: Player,
            card_id: C::TractorBeam
        }));
        let drone = resolve(
            &mut full,
            Turn {
                command: order_at(C::TractorBeam, at(Strike, 3)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let moves: Vec<_> = effects_of(&drone, C::TractorBeam)
            .iter()
            .map(|effect| effect.effect)
            .collect();
        assert_eq!(moves, [CommandEffect::Move { to: at(Command, 4) }]);
    }

    #[test]
    fn traktorstrahl_auf_beiden_seiten_gleich_mit_vertauschten_sitzen() {
        let tractor = Turn {
            command: order_at(C::TractorBeam, at(Strike, 1)),
            ..pass(Player)
        };
        let (board, _) = mirrored(
            |board, a, b| {
                place(board, a, Strike, 1, S::HorizonCruiser);
                place(board, a, Command, 1, S::RepairDrone);
                place(board, b, Strike, 1, S::VoidInterceptor);
                place(board, b, Strike, 2, S::HorizonCruiser);
            },
            tractor.clone(),
            tractor,
        );
        assert_eq!(
            card(&board, slot(Player, Command, 0)),
            Some(S::HorizonCruiser)
        );
        assert_eq!(
            card(&board, slot(Enemy, Command, 1)),
            Some(S::VoidInterceptor)
        );
    }
}

mod angriff {
    use super::*;

    #[test]
    fn waermesucher_3_schaden_am_erfassten_schiff_auch_gedeckt_ohne_bewegung() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 2, S::HorizonCruiser);
        let covered = place(&mut board, Enemy, Command, 2, S::RepairDrone);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::HeatSeeker, at(Command, 2)),
                ..pass(Player)
            },
            pass(Enemy),
        );
        let applied: Vec<_> = effects_of(&events, C::HeatSeeker)
            .iter()
            .map(|effect| effect.effect)
            .collect();
        assert_eq!(applied, [CommandEffect::Damage { amount: 3 }]);
        assert_eq!(hp_at(&board, covered), 2);
    }

    #[test]
    fn waermesucher_die_erfassung_folgt_dem_manoever_und_trifft_mit_5() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 2, S::IronDreadnought);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::HeatSeeker, at(Strike, 2)),
                ..pass(Player)
            },
            Turn {
                maneuver: maneuver(at(Strike, 2), at(Command, 4)),
                ..pass(Enemy)
            },
        );
        assert!(events.contains(&RoundEvent::Command {
            side: Player,
            card_id: C::HeatSeeker,
            target: Some(slot(Enemy, Command, 4))
        }));
        assert_eq!(
            hp_at(&board, slot(Enemy, Command, 4)),
            hp(S::IronDreadnought) - 3
        );
    }

    #[test]
    fn gnadenstoss_4_schaden_am_schiff_mit_den_wenigsten_hp_ohne_tokens() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 0, S::Drone);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let weak = place(&mut board, Enemy, Command, 3, S::RepairDrone);
        let events = effects(&mut board, &[play(Player, C::CoupDeGrace, None, None)]);
        let applied: Vec<_> = effects_of(&events, C::CoupDeGrace)
            .iter()
            .map(|effect| (effect.ship.card_id, effect.effect))
            .collect();
        assert_eq!(
            applied,
            [(S::RepairDrone, CommandEffect::Damage { amount: 4 })]
        );
        assert_eq!(hp_at(&board, weak), 1);
    }

    #[test]
    fn durchschlag_3_vorne_2_dahinter_sonst_nur_das_command_schiff_leer_verfehlt() {
        let mut board = Board::new();
        let front = place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let back = place(&mut board, Enemy, Command, 1, S::IronDreadnought);
        let lone = place(&mut board, Enemy, Command, 3, S::HorizonCruiser);
        let mut events = Vec::new();
        for index in [1, 3, 5] {
            events.extend(effects(
                &mut board,
                &[play(
                    Player,
                    C::PiercingShot,
                    Some(slot(Enemy, Strike, index)),
                    None,
                )],
            ));
        }
        let values = [
            hp_at(&board, front),
            hp_at(&board, back),
            board.ship(back).shield,
            hp_at(&board, lone),
        ];
        assert_eq!(values, [5, 12, 0, 5]);
        assert!(events.contains(&RoundEvent::Miss {
            side: Player,
            card_id: C::PiercingShot,
            target: slot(Enemy, Strike, 5)
        }));
    }

    #[test]
    fn bombardement_2_an_jedes_schiff_der_gegnerischen_command_lane() {
        let mut board = Board::new();
        let front = place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let ships = [
            place(&mut board, Enemy, Command, 1, S::HorizonCruiser),
            place(&mut board, Enemy, Command, 4, S::RepairDrone),
        ];
        effects(&mut board, &[play(Player, C::Bombardment, None, None)]);
        let values = [
            hp_at(&board, front),
            hp_at(&board, ships[0]),
            hp_at(&board, ships[1]),
        ];
        assert_eq!(values, [8, 6, 3]);
        let none = effects(
            &mut Board::new(),
            &[play(Player, C::Bombardment, None, None)],
        );
        assert_eq!(
            none,
            [RoundEvent::NoEffect {
                side: Player,
                card_id: C::Bombardment
            }]
        );
    }

    #[test]
    fn kreuzfeuer_3_an_jedes_schiff_ohne_partner_tokens_zaehlen_als_partner_und_ziel() {
        let mut board = Board::new();
        place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Command, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        place(&mut board, Enemy, Command, 2, S::RepairDrone);
        place(&mut board, Enemy, Strike, 3, S::Drone);
        place(&mut board, Enemy, Command, 3, S::HorizonCruiser);
        place(&mut board, Enemy, Strike, 5, S::Drone);
        let events = effects(&mut board, &[play(Player, C::Crossfire, None, None)]);
        let hits: Vec<_> = effects_of(&events, C::Crossfire)
            .iter()
            .map(|effect| (effect.ship.lane, effect.ship.slot_index))
            .collect();
        assert_eq!(hits, [(Strike, 1), (Strike, 5), (Command, 2)]);
    }
}

mod spielende {
    use super::*;

    #[test]
    fn faellt_ein_flaggschiff_bei_den_effects_endet_die_runde_sofort() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::HorizonCruiser);
        place_hp(&mut board, Enemy, Strike, 2, S::Vanguard, 1);
        place(&mut board, Enemy, Strike, 3, S::HorizonCruiser);
        place(&mut board, Enemy, Command, 3, S::RepairDrone);
        let events = resolve(
            &mut board,
            Turn {
                command: order_at(C::Torpedo, at(Strike, 2)),
                fire: vec![fire(at(Strike, 0), at(Strike, 3))],
                ..pass(Player)
            },
            Turn {
                fire: vec![fire(at(Strike, 3), at(Strike, 0))],
                ..pass(Enemy)
            },
        );
        assert_eq!(
            events.last(),
            Some(&RoundEvent::Destroyed {
                ship: ship(Enemy, Strike, 2, S::Vanguard)
            })
        );
        assert!(!has(&events, "attack"));
        assert_eq!(board.flagship(Enemy), None);
    }

    #[test]
    fn fallen_beide_flaggschiffe_im_selben_kampf_ist_es_unentschieden() {
        let mut board = Board::new();
        let wounded = place_hp(
            &mut board,
            Player,
            Strike,
            1,
            S::HorizonCruiser,
            hp(S::HorizonCruiser) - 1,
        );
        place_hp(&mut board, Player, Strike, 0, S::Vanguard, 1);
        place(&mut board, Player, Command, 1, S::RepairDrone);
        place_hp(&mut board, Enemy, Strike, 0, S::Vanguard, 1);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                fire: vec![fire(at(Strike, 1), at(Strike, 0))],
                ..pass(Player)
            },
            Turn {
                fire: vec![fire(at(Strike, 1), at(Strike, 0))],
                ..pass(Enemy)
            },
        );
        assert!(matches!(
            events[events.len() - 2..],
            [
                RoundEvent::Destroyed {
                    ship: ShipRef {
                        side: Player,
                        card_id: S::Vanguard,
                        ..
                    }
                },
                RoundEvent::Destroyed {
                    ship: ShipRef {
                        side: Enemy,
                        card_id: S::Vanguard,
                        ..
                    }
                },
            ]
        ));
        assert_eq!(hp_at(&board, wounded), hp(S::HorizonCruiser) - 1);
        assert_eq!(
            [board.flagship(Player), board.flagship(Enemy)],
            [None, None]
        );
    }

    #[test]
    fn faellt_ein_flaggschiff_durch_taktvorsprung_feuern_die_uebrigen_nicht_mehr() {
        let mut board = Board::new();
        place(&mut board, Player, Strike, 0, S::VoidInterceptor);
        let own = place_hp(&mut board, Player, Strike, 1, S::Vanguard, 1);
        place_hp(&mut board, Enemy, Strike, 0, S::Vanguard, 1);
        place(&mut board, Enemy, Strike, 1, S::HorizonCruiser);
        let events = resolve(
            &mut board,
            Turn {
                power: power(PowerId::HeadStart, at(Strike, 0)),
                fire: vec![fire(at(Strike, 0), at(Strike, 0))],
                ..pass(Player)
            },
            Turn {
                fire: vec![fire(at(Strike, 1), at(Strike, 1))],
                ..pass(Enemy)
            },
        );
        assert_eq!(attacks_by(&events, Enemy), []);
        assert!(matches!(
            events.last(),
            Some(RoundEvent::Destroyed {
                ship: ShipRef {
                    side: Enemy,
                    card_id: S::Vanguard,
                    ..
                }
            })
        ));
        assert_eq!(board.flagship(Player), Some(own));
    }
}
