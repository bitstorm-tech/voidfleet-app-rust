use voidfleet_core::admirals::{AdmiralId, Station};
use voidfleet_core::battle::{
    Battle, BattleResult, BattleSetup, Encounter, Enemy, RuleError, Target,
};
use voidfleet_core::board::{EnemySlot, Row};
use voidfleet_core::cards::{CardId, CardTarget, DeckCard, starter_deck};
use voidfleet_core::enemies::{Action, EnemyId};
use voidfleet_core::events::{
    BattleEvent, Breakdown, DestroyCause, IdleReason, MissReason, UnitRef,
};

const MID: [bool; 3] = [false, true, false];

fn battle(admiral: AdmiralId, units: &[(EnemyId, Row, usize)]) -> Battle {
    Battle::new(
        BattleSetup {
            admiral,
            hull: admiral.hull(),
            max_hull: admiral.hull(),
            deck: starter_deck(admiral),
            modules: vec![],
            emergency_generator_available: false,
            encounter: Encounter {
                units: units
                    .iter()
                    .map(|&(id, row, c)| (id, EnemySlot::new(row, c)))
                    .collect(),
                hull_percent: 100,
                reinforcements: vec![],
                replacements: vec![],
            },
        },
        7,
    )
}

fn give(b: &mut Battle, cards: &[CardId]) {
    b.hand = cards.iter().map(|&c| DeckCard::new(c)).collect();
}

fn uid(b: &Battle, id: EnemyId) -> u32 {
    b.enemies.iter().find(|e| e.id == id).expect("steht").uid
}

fn unit(b: &mut Battle, id: EnemyId) -> &mut Enemy {
    b.enemies.iter_mut().find(|e| e.id == id).expect("steht")
}

fn play(b: &mut Battle, card: CardId, target: Target) -> Vec<BattleEvent> {
    let i = b
        .hand
        .iter()
        .position(|c| c.id == card)
        .expect("auf der Hand");
    b.play_card(i, target).expect("spielbar")
}

/// Die Rechnungen aller Treffer auf `target`.
fn hits_on(events: &[BattleEvent], target: UnitRef) -> Vec<Breakdown> {
    events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Hit {
                target: t,
                breakdown,
                ..
            } if same(*t, target) => Some(*breakdown),
            _ => None,
        })
        .collect()
}

fn same(a: UnitRef, b: UnitRef) -> bool {
    match (a, b) {
        (UnitRef::Enemy { uid: a, .. }, UnitRef::Enemy { uid: b, .. }) => a == b,
        _ => a == b,
    }
}

fn enemy_ref(uid: u32) -> UnitRef {
    UnitRef::Enemy {
        uid,
        id: EnemyId::Raiders,
        slot: EnemySlot::new(Row::Front, 0),
    }
}

/// Entwurf Abschnitt 10: Runde 3, Vanguard 38/50 mit Vorstoß +2, Belagerer aufgeladen, Plünderer hinten links.
fn example_round() -> Battle {
    let mut b = battle(
        AdmiralId::Vance,
        &[
            (EnemyId::Besieger, Row::Front, 1),
            (EnemyId::Raiders, Row::Back, 0),
        ],
    );
    b.round = 3;
    b.flagship.hull = 38;
    b.flagship.momentum = 2;
    give(
        &mut b,
        &[
            CardId::Torpedo,
            CardId::Torpedo,
            CardId::EmergencyShields,
            CardId::Overcharge,
            CardId::Scavenge,
        ],
    );
    let besieger = unit(&mut b, EnemyId::Besieger);
    besieger.hull = 22;
    besieger.charged = true;
    besieger.intents = [Action::ColumnFire { columns: MID }, Action::Shields(10)];
    besieger.step = 3;
    b
}

#[test]
fn example_round_hammer() {
    let mut b = example_round();
    let besieger = uid(&b, EnemyId::Besieger);
    let raiders = uid(&b, EnemyId::Raiders);
    assert_eq!(b.points, 3);
    assert_eq!(b.flagship_attack(), 13);
    assert_eq!(b.announced(besieger)[0].damage, 16);
    assert_eq!(b.announced(raiders)[0].action, Action::Fire);
    assert_eq!(b.announced(raiders)[0].damage, 2);
    assert!(!b.covered(raiders));

    play(&mut b, CardId::Overcharge, Target::None);
    let events = play(&mut b, CardId::Torpedo, Target::Enemy(besieger));
    let torpedo = hits_on(&events, enemy_ref(besieger))[0];
    assert_eq!((torpedo.base, torpedo.armor, torpedo.hull), (6, 2, 4));
    assert_eq!(b.enemy(besieger).unwrap().hull, 18);

    let preview = b
        .preview_station(Station::Admiral, Target::Enemy(besieger))
        .unwrap();
    let mut events = b
        .run_station(Station::Admiral, Target::Enemy(besieger))
        .unwrap();
    let focus = hits_on(&events, enemy_ref(besieger));
    assert_eq!(
        preview.iter().map(|h| h.breakdown).collect::<Vec<_>>(),
        focus
    );
    assert_eq!(
        (
            focus[0].base,
            focus[0].armor,
            focus[0].hull,
            focus[0].lethal
        ),
        (20, 2, 18, true)
    );
    assert!(b.enemy(besieger).is_none());
    assert_eq!(b.kills, 1);
    events.extend(b.end_turn());
    assert!(events.contains(&BattleEvent::SelfDamage {
        amount: 3,
        after: voidfleet_core::events::UnitStats {
            hull: 35,
            shield: 0
        }
    }));
    let plunder = hits_on(&events, UnitRef::Flagship);
    assert_eq!((plunder[0].base, plunder[0].hull), (4, 4));
    assert_eq!(b.flagship.hull, 31);
    assert_eq!(b.flagship.momentum, 3);
    assert_eq!(b.enemy(raiders).unwrap().intents[0], Action::Fire);
    assert_eq!((b.round, b.result), (4, None));
}

#[test]
fn example_round_safe() {
    let mut b = example_round();
    let besieger = uid(&b, EnemyId::Besieger);
    play(&mut b, CardId::Torpedo, Target::Enemy(besieger));
    play(&mut b, CardId::EmergencyShields, Target::None);
    let mut events = b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    events.extend(b.end_turn());
    assert!(events.contains(&BattleEvent::Maneuvered { from: 1, to: 0 }));
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::Missed {
            reason: MissReason::Maneuvered,
            source: UnitRef::Enemy {
                id: EnemyId::Besieger,
                ..
            }
        }
    )));
    let plunder = hits_on(&events, UnitRef::Flagship);
    assert_eq!((plunder[0].shielded, plunder[0].hull), (2, 0));
    assert_eq!(
        (b.flagship.hull, b.flagship.momentum, b.flagship.column),
        (38, 0, 0)
    );
    let besieger = b.enemy(besieger).unwrap();
    assert_eq!((besieger.hull, besieger.charged), (18, false));
    assert_eq!(besieger.intents[0], Action::Shields(10));
}

#[test]
fn damage_order_with_evade_armor_cover_and_shield() {
    let mut b = battle(
        AdmiralId::Vance,
        &[
            (EnemyId::GuardCruiser, Row::Front, 0),
            (EnemyId::Besieger, Row::Back, 0),
        ],
    );
    let besieger = uid(&b, EnemyId::Besieger);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    unit(&mut b, EnemyId::Besieger).intents[0] = Action::Evade;
    unit(&mut b, EnemyId::Besieger).shield = 1;
    unit(&mut b, EnemyId::GuardCruiser).shield = 2;
    assert!(b.covered(besieger));
    let preview = b
        .preview_station(Station::Weapons, Target::Enemy(besieger))
        .unwrap();
    let events = b
        .run_station(Station::Weapons, Target::Enemy(besieger))
        .unwrap();
    // 11, ausgewichen −5, Panzerung −2, gedeckt: 2 an den Wachkreuzer, Schild 1, Hülle 1.
    let expected = Breakdown {
        base: 11,
        evaded: 5,
        armor: 2,
        covered: Some((cruiser, 2)),
        shielded: 1,
        countermeasures: 0,
        hull: 1,
        lethal: false,
    };
    assert_eq!(hits_on(&events, enemy_ref(besieger)), vec![expected]);
    let front = hits_on(&events, enemy_ref(cruiser))[0];
    assert_eq!((front.base, front.shielded, front.hull), (2, 2, 0));
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0].breakdown, expected);
    assert_eq!(preview[1].breakdown, front);
    assert_eq!(b.enemy(besieger).unwrap().hull, 29);
}

#[test]
fn burn_and_burned_out() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    b.flagship.burning = true;
    let mut events = b
        .run_station(Station::Weapons, Target::Enemy(cruiser))
        .unwrap();
    events.extend(b.end_turn());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Ignited { .. }))
    );
    assert_eq!(b.enemy(cruiser).unwrap().hull, 16 - 11 - 2);
    assert_eq!(b.flagship.hull, 50 - 4 - 2);

    unit(&mut b, EnemyId::GuardCruiser).hull = 2;
    let mut events = b.run_station(Station::DamageControl, Target::None).unwrap();
    events.extend(b.end_turn());
    assert!(events.contains(&BattleEvent::Extinguished {
        target: UnitRef::Flagship
    }));
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::Destroyed {
            cause: DestroyCause::BurnedOut,
            ..
        }
    )));
    assert_eq!(b.result, Some(BattleResult::Won));
}

#[test]
fn reyes_plasma_and_ember() {
    let mut b = battle(AdmiralId::Reyes, &[(EnemyId::Besieger, Row::Front, 0)]);
    let besieger = uid(&b, EnemyId::Besieger);
    b.run_station(Station::Weapons, Target::Enemy(besieger))
        .unwrap();
    b.end_turn();
    assert!(b.enemy(besieger).unwrap().burning);
    assert_eq!(b.enemy(besieger).unwrap().hull, 30 - 7 - 3);
}

#[test]
fn emergency_halves_attack_and_ends_evade() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    let enemy = unit(&mut b, EnemyId::GuardCruiser);
    enemy.hull = 4;
    let enemy = b.enemy(cruiser).unwrap().clone();
    assert_eq!(b.enemy_attack(&enemy), 2);
    assert_eq!(b.announced(cruiser)[0].damage, 2);
    let enemy = unit(&mut b, EnemyId::GuardCruiser);
    enemy.intents[0] = Action::Evade;
    assert!(enemy.emergency() && !enemy.is_evading());

    b.flagship.hull = 12;
    assert_eq!(b.flagship_attack(), 6);
    assert_eq!(
        b.station_open(Station::Admiral),
        Err(RuleError::AdmiralStationInEmergency)
    );
    let hit = b
        .preview_station(Station::Weapons, Target::Enemy(cruiser))
        .unwrap()[0]
        .breakdown;
    assert_eq!((hit.base, hit.evaded, hit.lethal), (6, 0, true));
}

#[test]
fn counterfire_and_hold_the_line() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::GuardCruiser, Row::Front, 1)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    give(&mut b, &[CardId::HoldTheLine]);
    b.run_station(Station::Admiral, Target::None).unwrap();
    play(&mut b, CardId::HoldTheLine, Target::None);
    let events = b.end_turn();
    assert_eq!(hits_on(&events, UnitRef::Flagship)[0].shielded, 4);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Counterfire { amount: 3, .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::HoldTheLine { amount: 2, .. }))
    );
    assert_eq!(b.enemy(cruiser).unwrap().hull, 11);
    assert_eq!(b.flagship.hull, 70);
}

#[test]
fn micro_jump() {
    let mut b = battle(
        AdmiralId::Kerr,
        &[
            (EnemyId::GuardCruiser, Row::Front, 0),
            (EnemyId::TorpedoWing, Row::Front, 1),
        ],
    );
    give(&mut b, &[CardId::MicroJump]);
    // Erst schießen, dann springen: Kurzsprung bleibt nach einer Station spielbar.
    b.run_station(
        Station::Weapons,
        Target::Enemy(uid(&b, EnemyId::GuardCruiser)),
    )
    .unwrap();
    let events = play(&mut b, CardId::MicroJump, Target::None);
    assert!(events.contains(&BattleEvent::Jumped));
    assert_eq!(b.station_open(Station::Shields), Err(RuleError::Jumped));
    assert_eq!(
        b.enemy(uid(&b, EnemyId::GuardCruiser)).unwrap().hull,
        16 - 8
    );
    let events = b.end_turn();
    let misses = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                BattleEvent::Missed {
                    reason: MissReason::Jumped,
                    ..
                }
            )
        })
        .count();
    assert_eq!(misses, 2);
    assert!(events.contains(&BattleEvent::Returned));
    assert_eq!((b.flagship.hull, b.flagship.jumped), (56, false));
}

#[test]
fn station_targets_like_cards() {
    let mut b = battle(AdmiralId::Kerr, &[(EnemyId::Besieger, Row::Front, 0)]);
    let besieger = uid(&b, EnemyId::Besieger);
    assert_eq!(b.station_target(Station::Admiral), CardTarget::Enemy);
    for (station, target) in [
        (Station::Weapons, Target::None),
        (Station::Weapons, Target::Column(0)),
        (Station::Weapons, Target::Enemy(99)),
        (Station::Maneuver, Target::Column(1)),
        (Station::Maneuver, Target::Column(3)),
        (Station::Shields, Target::Enemy(besieger)),
    ] {
        assert_eq!(
            b.run_station(station, target),
            Err(RuleError::TargetInvalid)
        );
    }
    assert!(b.flagship.stations_used.is_empty());
    b.run_station(Station::Maneuver, Target::Column(2)).unwrap();
    assert_eq!(b.flagship.column, 2);
}

#[test]
fn stations_once_per_slot_and_red_alert() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::Besieger, Row::Front, 0)]);
    b.run_station(Station::Weapons, Target::Enemy(uid(&b, EnemyId::Besieger)))
        .unwrap();
    assert_eq!(
        b.station_open(Station::Weapons),
        Err(RuleError::StationUsed)
    );
    assert_eq!(
        b.station_open(Station::Shields),
        Err(RuleError::NoStationSlot)
    );
    give(&mut b, &[CardId::RedAlert]);
    play(&mut b, CardId::RedAlert, Target::None);
    let events = b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(
        events[0],
        BattleEvent::StationTriggered {
            station: Station::Shields
        }
    );
    assert_eq!(
        b.flagship.stations_used,
        [Station::Weapons, Station::Shields]
    );
    assert_eq!(
        b.station_open(Station::DamageControl),
        Err(RuleError::NoStationSlot)
    );
    b.end_turn();
    assert_eq!(b.flagship.momentum, 0);
    assert_eq!(
        (b.flagship.stations_used.len(), b.flagship.station_slots),
        (0, 1)
    );
    assert_eq!(b.station_open(Station::Weapons), Ok(()));
}

#[test]
fn end_turn_runs_no_station() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    let events = b.end_turn();
    assert!(hits_on(&events, enemy_ref(cruiser)).is_empty());
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, BattleEvent::StationTriggered { .. }))
    );
    assert_eq!(b.enemy(cruiser).unwrap().hull, 16);
    // Die Station wirkt sofort beim Auslösen.
    let events = b
        .run_station(Station::Weapons, Target::Enemy(cruiser))
        .unwrap();
    assert_eq!(hits_on(&events, enemy_ref(cruiser))[0].hull, 11);
    assert_eq!(b.enemy(cruiser).unwrap().hull, 5);
}

#[test]
fn order_matters() {
    // Schuss bringt den Wachkreuzer in den Notfall, Gnadenstoß danach trifft mit dem Zuschlag.
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    unit(&mut b, EnemyId::GuardCruiser).hull = 12;
    give(&mut b, &[CardId::CoupDeGrace]);
    let before = b.preview_card(0, Target::Enemy(cruiser)).unwrap()[0]
        .breakdown
        .base;
    b.run_station(Station::Weapons, Target::Enemy(cruiser))
        .unwrap();
    assert!(b.enemy(cruiser).unwrap().emergency());
    let after = b.preview_card(0, Target::Enemy(cruiser)).unwrap()[0]
        .breakdown
        .base;
    assert_eq!(
        after - before,
        voidfleet_core::cards::COUP_DE_GRACE_EMERGENCY_BONUS
    );

    // Energiestoß nach dem Schuss ändert ihn nicht mehr, davor schon.
    let shot = |boost_first: bool| {
        let mut b = battle(AdmiralId::Vance, &[(EnemyId::Besieger, Row::Front, 0)]);
        let besieger = uid(&b, EnemyId::Besieger);
        give(&mut b, &[CardId::PowerSurge]);
        if boost_first {
            play(&mut b, CardId::PowerSurge, Target::None);
        }
        let base = hits_on(
            &b.run_station(Station::Weapons, Target::Enemy(besieger))
                .unwrap(),
            enemy_ref(besieger),
        )[0]
        .base;
        if !boost_first {
            play(&mut b, CardId::PowerSurge, Target::None);
        }
        (base, b.enemy(besieger).unwrap().hull)
    };
    let (late, late_hull) = shot(false);
    let (early, early_hull) = shot(true);
    assert_eq!(late, 11);
    assert!(early > late && early_hull < late_hull);
}

#[test]
fn stasis_delays_intent_and_keeps_charge() {
    let mut b = battle(AdmiralId::Kerr, &[(EnemyId::Besieger, Row::Front, 1)]);
    let besieger = uid(&b, EnemyId::Besieger);
    let enemy = unit(&mut b, EnemyId::Besieger);
    enemy.charged = true;
    enemy.intents = [Action::ColumnFire { columns: MID }, Action::Shields(10)];
    enemy.step = 3;
    b.run_station(Station::Admiral, Target::Enemy(besieger))
        .unwrap();
    let events = b.end_turn();
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::Idle {
            reason: IdleReason::Stasis,
            ..
        }
    )));
    assert_eq!(b.flagship.hull, 56);
    let enemy = b.enemy(besieger).unwrap();
    assert!(enemy.charged);
    assert_eq!(
        enemy.intents,
        [Action::ColumnFire { columns: MID }, Action::Shields(10)]
    );
    assert_eq!(b.announced(besieger)[0].damage, 16);
}

#[test]
fn follow_up() {
    let mut b = battle(
        AdmiralId::Reyes,
        &[
            (EnemyId::GuardCruiser, Row::Front, 0),
            (EnemyId::Raiders, Row::Front, 1),
            (EnemyId::PlasmaCruiser, Row::Front, 2),
        ],
    );
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    unit(&mut b, EnemyId::GuardCruiser).burning = true;
    unit(&mut b, EnemyId::Raiders).burning = true;
    give(&mut b, &[CardId::Firebomb, CardId::Firebomb]);
    // Vor dem Auslösen kein Bonus.
    let before = b.preview_card(0, Target::Enemy(cruiser)).unwrap()[0]
        .breakdown
        .base;
    let hits = b.preview_station(Station::Admiral, Target::None).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!((hits[0].breakdown.hull, hits[1].breakdown.hull), (6, 6));
    b.run_station(Station::Admiral, Target::None).unwrap();
    assert_eq!(b.enemy(cruiser).unwrap().hull, 10);
    play(&mut b, CardId::Firebomb, Target::Enemy(cruiser));
    // Nach dem Auslösen +2 auf brennende Ziele.
    assert_eq!(before, 2);
    assert_eq!(b.enemy(cruiser).unwrap().hull, 10 - 4);
}

#[test]
fn carrier_calls_reinforcement() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::Carrier, Row::Front, 1)]);
    b.end_turn();
    let events = b.end_turn();
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::EnemyAction {
            action: Action::Call(EnemyId::Raiders),
            ..
        }
    )));
    assert_eq!(b.enemies.len(), 1);
    let events = b.end_turn();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Arrived { .. }))
    );
    assert_eq!(b.enemies.len(), 2);
    assert_eq!(
        unit(&mut b, EnemyId::Raiders).slot,
        EnemySlot::new(Row::Front, 0)
    );
}

#[test]
fn fireship_rams_and_vanishes() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::Fireship, Row::Front, 0)]);
    b.end_turn();
    b.end_turn();
    assert_eq!(b.enemies[0].intents[0], Action::Ram);
    assert_eq!(b.announced(b.enemies[0].uid)[0].damage, 10);
    let events = b.end_turn();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::FireshipSpent { .. }))
    );
    assert_eq!(b.flagship.hull, 60);
    assert_eq!((b.result, b.kills), (Some(BattleResult::Won), 0));
}

#[test]
fn warden_fires_on_two_columns() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::Warden, Row::Back, 1)]);
    let warden = uid(&b, EnemyId::Warden);
    b.end_turn();
    let columns = [true, true, false];
    assert_eq!(
        b.announced(warden)[1].action,
        Action::ColumnFire { columns }
    );
    assert_eq!(b.announced(warden)[1].damage, 30);
    b.end_turn();
    assert_eq!(b.announced(warden)[0].damage, 30);
    b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    let events = b.end_turn();
    assert_eq!(hits_on(&events, UnitRef::Flagship)[0].hull, 30);
    assert_eq!(b.flagship.hull, 40);
    assert_eq!(b.enemy(warden).unwrap().intents[0], Action::Shields(15));
}

#[test]
fn scrap_king_cannibalizes() {
    let mut b = battle(
        AdmiralId::Vance,
        &[
            (EnemyId::Raiders, Row::Front, 0),
            (EnemyId::ScrapKing, Row::Front, 1),
        ],
    );
    unit(&mut b, EnemyId::ScrapKing).hull = 40;
    let events = b
        .run_station(Station::Weapons, Target::Enemy(uid(&b, EnemyId::Raiders)))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Cannibalized { amount: 8, .. }))
    );
}

#[test]
fn seal_bolt_and_replacements() {
    let mut b = battle(
        AdmiralId::Vance,
        &[
            (EnemyId::GuardCruiser, Row::Front, 1),
            (EnemyId::Seal, Row::Back, 1),
        ],
    );
    b.replacements = vec![EnemyId::Raiders];
    unit(&mut b, EnemyId::GuardCruiser).hull = 5;
    let mut events = b
        .run_station(
            Station::Weapons,
            Target::Enemy(uid(&b, EnemyId::GuardCruiser)),
        )
        .unwrap();
    events.extend(b.end_turn());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Arrived { .. }))
    );
    assert_eq!(
        unit(&mut b, EnemyId::Raiders).slot,
        EnemySlot::new(Row::Front, 1)
    );
    assert!(b.replacements.is_empty());

    assert_eq!(b.flagship.hull, 42);
    let events = b.end_turn();
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::EnemyAction {
            action: Action::Bolt,
            ..
        }
    )));
    assert!(b.enemies.iter().all(|e| e.shield == 4));
    let seal = uid(&b, EnemyId::Seal);
    unit(&mut b, EnemyId::Seal).hull = 1;
    give(&mut b, &[CardId::OrbitalStrike]);
    play(&mut b, CardId::OrbitalStrike, Target::Enemy(seal));
    assert_eq!(b.result, Some(BattleResult::Won));
    assert_eq!(b.enemies.len(), 1);
}

#[test]
fn deck_cycles_through_discard_and_doctrines_leave() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::Spotter, Row::Back, 0)]);
    assert_eq!((b.hand.len(), b.draw_pile.len()), (5, 5));
    b.end_turn();
    assert_eq!(
        (b.hand.len(), b.draw_pile.len(), b.discard_pile.len()),
        (5, 0, 5)
    );
    b.hand[0] = DeckCard::new(CardId::FireControlDoctrine);
    b.play_card(0, Target::None).unwrap();
    let events = b.end_turn();
    assert!(events.contains(&BattleEvent::Shuffled));
    assert_eq!(
        (b.hand.len(), b.draw_pile.len(), b.discard_pile.len()),
        (5, 4, 0)
    );
    assert_eq!(b.doctrines.len(), 1);
    assert_eq!(b.flagship.fire_control, 1);
}

#[test]
fn battle_clock_and_spotter() {
    let mut b = battle(
        AdmiralId::Osei,
        &[
            (EnemyId::GuardCruiser, Row::Front, 0),
            (EnemyId::Spotter, Row::Back, 1),
        ],
    );
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    assert_eq!(b.announced(cruiser)[0].damage, 5);
    b.round = 7;
    let events = b.end_turn();
    assert!(events.contains(&BattleEvent::Clock { bonus: 1 }));
    assert_eq!(b.announced(cruiser)[0].damage, 6);
    give(&mut b, &[CardId::RadioSilence]);
    let spotter = uid(&b, EnemyId::Spotter);
    play(&mut b, CardId::RadioSilence, Target::Enemy(spotter));
    assert_eq!(b.announced(cruiser)[0].damage, 5);
}

#[test]
fn upgrades() {
    let up = |id: CardId| (id.cost(true), id.value(true));
    assert_eq!(up(CardId::Torpedo), (2, 8));
    assert_eq!(up(CardId::HeatSeeker), (2, 7));
    assert_eq!(up(CardId::CoupDeGrace), (2, 5));
    assert_eq!(up(CardId::CollisionCourse), (3, 2));
    assert_eq!(up(CardId::EmergencyShields), (1, 7));
    assert_eq!(up(CardId::BattleStations), (2, 4));
    assert_eq!(up(CardId::Overcharge), (0, 5));
    assert_eq!(up(CardId::MicroJump), (1, 0));
    assert_eq!(up(CardId::FireControlDoctrine), (1, 1));
    assert_eq!(up(CardId::Countermeasures), (0, 0));

    let mut b = battle(AdmiralId::Vance, &[(EnemyId::Spotter, Row::Front, 0)]);
    b.hand = vec![DeckCard {
        id: CardId::Torpedo,
        upgraded: true,
    }];
    let spotter = uid(&b, EnemyId::Spotter);
    let hit = b.preview_card(0, Target::Enemy(spotter)).unwrap()[0].breakdown;
    assert_eq!(hit.base, 8);
}

#[test]
fn collision_course() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    give(&mut b, &[CardId::CollisionCourse]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    let events = play(&mut b, CardId::CollisionCourse, Target::Enemy(cruiser));
    assert_eq!(hits_on(&events, enemy_ref(cruiser))[0].base, 22);
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::Destroyed {
            cause: DestroyCause::Rammed,
            ..
        }
    )));
    assert_eq!(b.flagship.hull, 46);
}

#[test]
fn raiders_evade_after_hit() {
    let mut b = battle(AdmiralId::Osei, &[(EnemyId::Raiders, Row::Front, 0)]);
    b.enemies[0].hull = 20;
    b.enemies[0].max_hull = 20;
    b.run_station(Station::Weapons, Target::Enemy(uid(&b, EnemyId::Raiders)))
        .unwrap();
    assert!(!b.enemies[0].is_evading());
    b.end_turn();
    // Angesagt heißt aktiv: Die Staffel weicht schon in diesem Spielerzug aus, im nächsten nicht mehr.
    assert_eq!(b.enemies[0].intents[0], Action::Evade);
    assert!(b.enemies[0].is_evading());
    b.end_turn();
    assert!(!b.enemies[0].is_evading());
}

#[test]
fn emergency_generator_and_defeat() {
    let mut b = battle(AdmiralId::Vance, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    b.emergency_generator_available = true;
    b.flagship.hull = 3;
    b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    let events = b.end_turn();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::EmergencyGenerator { .. }))
    );
    assert!(b.emergency_generator_used);
    assert_eq!((b.flagship.hull, b.result), (1, None));
    b.run_station(Station::Maneuver, Target::Column(1)).unwrap();
    let events = b.end_turn();
    assert_eq!(b.result, Some(BattleResult::Lost));
    assert_eq!(
        events.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Lost
        })
    );
}

#[test]
fn serde_names() {
    let json = |v: serde_json::Value| v.as_str().unwrap().to_owned();
    assert_eq!(
        json(serde_json::json!(CardId::CoupDeGrace)),
        "coup_de_grace"
    );
    assert_eq!(
        json(serde_json::json!(EnemyId::TorpedoWing)),
        "torpedo_wing"
    );
    let module = voidfleet_core::modules::ModuleId::FireControlComputer;
    assert_eq!(json(serde_json::json!(module)), "fire_control_computer");
}
