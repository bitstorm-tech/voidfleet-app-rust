use std::collections::HashSet;
use std::path::Path;
use std::sync::Once;

use rand::SeedableRng;
use rand::rngs::StdRng;

use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId, Station};
use voidfleet_core::battle::{Battle, BattleResult, BattleSetup, Encounter, RuleError, Target};
use voidfleet_core::board::{COLUMNS, Row};
use voidfleet_core::cards::{CardId, DeckCard, NEUTRAL_POOL, admiral_pool, starter_deck};
use voidfleet_core::data::{CardData, Data, REPO_DATA, Step, install};
use voidfleet_core::enemies::{Action, EnemyId, Unit};
use voidfleet_core::events::{BattleEvent, Breakdown, UnitRef};
use voidfleet_core::modules::ModuleId;
use voidfleet_core::run::{EventId, NodeKind, Run, SECTORS, SectorMap, Stage};

fn fixture() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"));
        install(Data::load(dir).expect("Prüfwerte")).expect("Prüfwerte gültig");
    });
}

fn custom_unit(id: EnemyId, hull: i32, attack: i32, armor: i32, pattern: Vec<Step>) -> Unit {
    Unit {
        id,
        hull,
        attack,
        armor,
        pattern,
    }
}

fn battle_with(
    admiral: AdmiralId,
    front: [Option<Unit>; COLUMNS],
    back: [Option<Unit>; COLUMNS],
) -> Battle {
    fixture();
    Battle::new(
        BattleSetup {
            admiral,
            hull: admiral.hull(),
            max_hull: admiral.hull(),
            deck: starter_deck(admiral),
            modules: Vec::new(),
            emergency_generator_available: false,
            encounter: Encounter {
                front,
                back,
                ..Encounter::default()
            },
        },
        7,
    )
}

fn simple_battle(admiral: AdmiralId, units: &[(EnemyId, Row, usize)]) -> Battle {
    let mut front: [Option<Unit>; COLUMNS] = Default::default();
    let mut back: [Option<Unit>; COLUMNS] = Default::default();
    for &(id, row, col) in units {
        match row {
            Row::Front => front[col] = Some(id.into()),
            Row::Back => back[col] = Some(id.into()),
        }
    }
    battle_with(admiral, front, back)
}

fn uid(battle: &Battle, id: EnemyId) -> u32 {
    battle.enemies.iter().find(|e| e.id == id).unwrap().uid
}

fn enemy_ref(uid: u32, battle: &Battle) -> UnitRef {
    battle.enemy(uid).unwrap().unit()
}

fn set_hand(battle: &mut Battle, cards: &[CardId]) {
    battle.hand = cards.iter().copied().map(DeckCard::new).collect();
}

fn hits_on(events: &[BattleEvent], target: UnitRef) -> Vec<Breakdown> {
    events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Hit {
                target: t,
                breakdown,
                ..
            } if *t == target => Some(*breakdown),
            _ => None,
        })
        .collect()
}

fn maps() -> [SectorMap; SECTORS as usize] {
    fixture();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/maps");
    std::array::from_fn(|index| {
        let name = SectorMap::file(index as u32 + 1);
        let json = std::fs::read_to_string(format!("{dir}/{name}")).expect(&name);
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("{name}: {e}"))
    })
}

#[test]
fn data_and_schema_validation_for_all_48_cards() {
    fixture();
    let test_data = Data::load(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data"
    )))
    .expect("tests/data lädt fehlerfrei");
    assert_eq!(test_data.cards.len(), 48);

    let data = Data::load(Path::new(REPO_DATA)).expect("assets/data lädt fehlerfrei");
    assert_eq!(data.cards.len(), 48);

    // Every card with extra has extra >= 0; non-extra cards reject upgraded.extra.
    let mut non_extra: CardData = data.cards[&CardId::Torpedo].clone();
    assert!(non_extra.check(false));
    non_extra.upgraded.extra = Some(1);
    assert!(!non_extra.check(false));

    let mut neg_extra: CardData = data.cards[&CardId::EmergencyPower].clone();
    assert!(neg_extra.check(true));
    neg_extra.upgraded.extra = Some(-1);
    assert!(!neg_extra.check(true));

    assert_eq!(CardId::EmergencyPower.extra(false), 0);
    assert_eq!(CardId::EmergencyPower.extra(true), 1);
    assert_eq!(CardId::MeltdownSalvo.extra(false), 4);
    assert_eq!(CardId::MeltdownSalvo.extra(true), 5);
}

#[test]
fn pools_partition_all_48_cards_and_starter_decks_unchanged() {
    fixture();
    let mut all_from_pools = HashSet::new();
    for card in NEUTRAL_POOL {
        assert!(
            all_from_pools.insert(card),
            "Duplikat im neutralen Pool: {card:?}"
        );
    }
    assert_eq!(NEUTRAL_POOL.len(), 28);

    for admiral in ADMIRAL_IDS {
        let pool = admiral_pool(admiral);
        assert_eq!(pool.len(), 5);
        assert!(pool.contains(&admiral.signature_card()));
        assert!(!NEUTRAL_POOL.contains(&admiral.signature_card()));
        for card in pool {
            assert!(
                all_from_pools.insert(card),
                "Überlappung zwischen Pools bei {card:?}"
            );
        }
        assert_eq!(starter_deck(admiral).len(), 10);
    }

    let expected_all: HashSet<CardId> = CardId::ALL.into_iter().collect();
    assert_eq!(expected_all.len(), 48);
    assert_eq!(all_from_pools, expected_all);
}

#[test]
fn offer_cards_loot_and_market_guarantee_admiral_slot() {
    let maps = maps();
    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let legal: HashSet<CardId> = pool.iter().copied().chain(NEUTRAL_POOL).collect();
        let run = Run::new(admiral, 42, maps.clone());
        let mut rng = StdRng::seed_from_u64(123);

        assert!(run.offer_cards(&mut rng, 0).is_empty());

        let one = run.offer_cards(&mut rng, 1);
        assert_eq!(one.len(), 1);
        assert!(pool.contains(&one[0]));

        for size in [3, 4, 10] {
            for _ in 0..25 {
                let offer = run.offer_cards(&mut rng, size);
                assert_eq!(offer.len(), size);
                let set: HashSet<CardId> = offer.iter().copied().collect();
                assert_eq!(set.len(), size);
                assert!(offer.iter().any(|c| pool.contains(c)));
                assert!(offer.iter().all(|c| legal.contains(c)));
            }
        }

        let capped = run.offer_cards(&mut rng, 50);
        assert_eq!(capped.len(), 33);
        assert_eq!(capped.iter().copied().collect::<HashSet<_>>(), legal);
    }

    // Verify Battle loot, CombatSimulator loot, Market, and Pilot FlyYourself.
    for (idx, admiral) in ADMIRAL_IDS.into_iter().enumerate() {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let legal: HashSet<CardId> = pool.iter().copied().chain(NEUTRAL_POOL).collect();

        // Battle loot (with CombatSimulator -> 4 cards)
        let mut run = Run::new(admiral, 100 + idx as u64, maps.clone());
        run.modules.push(ModuleId::CombatSimulator);
        let col = run.choices()[0];
        run.map[0][col].kind = NodeKind::Battle;
        run.map[0][col].encounter = Some(Encounter {
            front: [None, Some(EnemyId::Raiders.into()), None],
            ..Encounter::default()
        });
        run.advance(col).unwrap();
        let mut battle = run.start_battle();
        battle.enemies.clear();
        battle.result = Some(BattleResult::Won);
        let loot = run.finish_battle(&battle);
        assert_eq!(loot.cards.len(), 4);
        assert!(loot.cards.iter().any(|c| pool.contains(c)));
        assert!(loot.cards.iter().all(|c| legal.contains(c)));

        // Market
        let mut run = Run::new(admiral, 200 + idx as u64, maps.clone());
        let col = run.choices()[0];
        run.map[0][col].kind = NodeKind::Market;
        run.map[0][col].encounter = None;
        run.advance(col).unwrap();
        let Stage::Market(market) = run.stage else {
            panic!("kein Markt");
        };
        assert_eq!(market.cards.len(), 3);
        assert!(market.cards.iter().any(|(c, _)| pool.contains(c)));
        assert!(market.cards.iter().all(|(c, _)| legal.contains(c)));

        // Pilot FlyYourself
        let mut run = Run::new(admiral, 300 + idx as u64, maps.clone());
        let col = run.choices()[0];
        run.map[0][col].kind = NodeKind::Event;
        run.map[0][col].encounter = None;
        run.advance(col).unwrap();
        run.stage = Stage::Event(EventId::Pilot);
        run.choose(1).unwrap();
        let Stage::Loot(loot) = run.stage else {
            panic!("keine Beute");
        };
        assert_eq!(loot.cards.len(), 3);
        assert!(loot.cards.iter().any(|c| pool.contains(c)));
        assert!(loot.cards.iter().all(|c| legal.contains(c)));
    }
}

#[test]
fn exhaust_micro_jump_and_emergency_power() {
    assert!(CardId::MicroJump.exhaust());
    assert!(CardId::EmergencyPower.exhaust());
    for card in CardId::ALL {
        if !matches!(card, CardId::MicroJump | CardId::EmergencyPower) {
            assert!(!card.exhaust(), "{card:?} sollte nicht Einmalig sein");
        }
    }

    let mut b = simple_battle(AdmiralId::Kerr, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    b.draw_pile.clear();
    b.discard_pile.clear();
    set_hand(&mut b, &[CardId::MicroJump, CardId::EmergencyPower]);
    b.hand[1].upgraded = true;

    let events = b.play_card(1, Target::None).unwrap();
    assert_eq!(b.points, 5);
    assert!(events.contains(&BattleEvent::PointsGained { amount: 2 }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BattleEvent::Drawn { cards } if cards.is_empty()))
    );
    assert_eq!(
        b.spent,
        vec![DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true
        }]
    );

    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 3);
    assert!(b.flagship.jumped);
    assert_eq!(b.spent.len(), 2);
    assert!(b.discard_pile.is_empty());
}

#[test]
fn retain_and_standby_fire_discount() {
    let mut b = simple_battle(AdmiralId::Osei, &[(EnemyId::Besieger, Row::Front, 1)]);
    b.draw_pile = vec![DeckCard::new(CardId::Torpedo); 10];
    set_hand(
        &mut b,
        &[CardId::StandbyFire, CardId::Torpedo, CardId::Bulwark],
    );

    assert!(!b.is_retained(0));
    assert_eq!(b.card_cost(0), 1);
    assert_eq!(b.card_cost(2), 2);

    b.end_turn();

    // StandbyFire (index 0) and Bulwark (index 1) retained; Torpedo discarded; 5 new drawn.
    assert_eq!(b.hand.len(), 7);
    assert_eq!(b.hand[0].id, CardId::StandbyFire);
    assert_eq!(b.hand[1].id, CardId::Bulwark);
    assert!(b.is_retained(0));
    assert!(b.is_retained(1));
    assert!(!b.is_retained(2));
    assert_eq!(b.card_cost(0), 0);
    assert_eq!(b.card_cost(1), 2);
    assert!(b.discard_pile.iter().any(|c| c.id == CardId::Torpedo));

    // Play retained StandbyFire for 0 BP; verify retained vector stays aligned for Bulwark now at index 0.
    let target = uid(&b, EnemyId::Besieger);
    let preview = b.preview_card(0, Target::Enemy(target)).unwrap();
    let events = b.play_card(0, Target::Enemy(target)).unwrap();
    assert_eq!(b.points, 3);
    assert_eq!(b.hand[0].id, CardId::Bulwark);
    assert!(b.is_retained(0));
    assert!(!b.is_retained(1));
    assert_eq!(
        preview[0].breakdown,
        hits_on(&events, enemy_ref(target, &b))[0]
    );
}

#[test]
fn marked_increases_damage_by_half_up_respects_cover_and_clears() {
    let front = custom_unit(EnemyId::GuardCruiser, 30, 0, 0, vec![Step::Fire]);
    let back = custom_unit(EnemyId::Spotter, 30, 0, 0, vec![Step::TargetLock]);
    let mut b = battle_with(
        AdmiralId::Kerr,
        [Some(front), None, None],
        [Some(back), None, None],
    );
    let f_uid = uid(&b, EnemyId::GuardCruiser);
    let b_uid = uid(&b, EnemyId::Spotter);

    // Mark only back unit; hit back unit with Torpedo (base 6, covered -> 3 to front, 3 to back).
    // Back takes 3 + half_up(3) = 5; Front is unmarked and takes 3 + 0 = 3.
    set_hand(
        &mut b,
        &[
            CardId::PrecisionLock,
            CardId::Torpedo,
            CardId::TargetPainter,
        ],
    );
    let events = b.play_card(0, Target::Enemy(b_uid)).unwrap();
    assert!(events.contains(&BattleEvent::Marked {
        target: enemy_ref(b_uid, &b)
    }));
    assert!(b.enemy(b_uid).unwrap().marked);

    let preview = b.preview_card(0, Target::Enemy(b_uid)).unwrap();
    let events = b.play_card(0, Target::Enemy(b_uid)).unwrap();
    let back_hits = hits_on(&events, enemy_ref(b_uid, &b));
    let front_hits = hits_on(&events, enemy_ref(f_uid, &b));
    assert_eq!(back_hits[0].marked, 2);
    assert_eq!(back_hits[0].hull, 5);
    assert_eq!(front_hits[0].marked, 0);
    assert_eq!(front_hits[0].hull, 3);
    assert_eq!(preview[0].breakdown, back_hits[0]);
    assert_eq!(preview[1].breakdown, front_hits[0]);

    // Odd damage on marked front unit: StandbyFire (7) -> 7 + half_up(7) = 11.
    b.play_card(0, Target::Enemy(f_uid)).unwrap();
    b.points = 1;
    b.hand.push(DeckCard::new(CardId::StandbyFire));
    let preview = b
        .preview_card(b.hand.len() - 1, Target::Enemy(f_uid))
        .unwrap();
    assert_eq!(preview[0].breakdown.marked, 4);
    assert_eq!(preview[0].breakdown.hull, 11);

    // Resolution clears Marked.
    b.end_turn();
    assert!(!b.enemy(f_uid).unwrap().marked);
    assert!(!b.enemy(b_uid).unwrap().marked);
}

#[test]
fn osei_cards_and_bastion_doctrine() {
    let armored = custom_unit(EnemyId::Besieger, 40, 0, 3, vec![Step::Evade]);
    let mut b = battle_with(
        AdmiralId::Osei,
        [Some(armored), None, None],
        [None, None, None],
    );
    let target = uid(&b, EnemyId::Besieger);

    // BastionDoctrine (upgraded -> 1 BP) + Bulwark (2 BP -> 12 + 2 = 14 shield, +1 counterfire)
    set_hand(
        &mut b,
        &[
            CardId::BastionDoctrine,
            CardId::Bulwark,
            CardId::ShieldBash,
            CardId::FlakBreaker,
        ],
    );
    b.hand[0].upgraded = true;
    b.hand[2].upgraded = true; // ShieldBash 0 BP
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 2);

    // BattleStations is a Doctrine, not a Shield Buff card, so should not trigger BastionDoctrine.
    b.hand.push(DeckCard::new(CardId::BattleStations));
    b.points += 2;
    b.play_card(b.hand.len() - 1, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 0);

    // Bulwark triggers BastionDoctrine (+2 shield, +1 counterfire) -> 14 shield total.
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 14);
    assert_eq!(b.flagship.hold_the_line, 1);

    // ShieldBash deals flagship.shield (14) homing (ignores Besieger's Evade).
    let preview = b.preview_card(0, Target::Enemy(target)).unwrap();
    let events = b.play_card(0, Target::Enemy(target)).unwrap();
    let hit = hits_on(&events, enemy_ref(target, &b))[0];
    assert_eq!(hit.base, 14);
    assert_eq!(hit.evaded, 0);
    assert_eq!(hit.armor, 3);
    assert_eq!(hit.hull, 11);
    assert_eq!(preview[0].breakdown, hit);

    // FlakBreaker on non-evading armored enemy: 2×4 hits (each reduced by 3 armor -> 1 hull each), then -1 armor.
    let front_non_evading = custom_unit(EnemyId::GuardCruiser, 30, 0, 3, vec![Step::Fire]);
    let mut b2 = battle_with(
        AdmiralId::Osei,
        [Some(front_non_evading), None, None],
        [None, None, None],
    );
    let t2 = uid(&b2, EnemyId::GuardCruiser);
    b2.hand = vec![
        DeckCard::new(CardId::FlakBreaker),
        DeckCard {
            id: CardId::FlakBreaker,
            upgraded: true,
        },
    ];
    let events = b2.play_card(0, Target::Enemy(t2)).unwrap();
    let hits = hits_on(&events, enemy_ref(t2, &b2));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].hull, 1);
    assert_eq!(hits[1].hull, 1);
    assert_eq!(b2.enemy(t2).unwrap().armor, 2);
    assert!(events.contains(&BattleEvent::ArmorBroken {
        target: enemy_ref(t2, &b2),
        amount: 1
    }));

    // Upgraded FlakBreaker: 2×6 hits against remaining 2 armor -> 4 hull each.
    let events = b2.play_card(0, Target::Enemy(t2)).unwrap();
    let hits = hits_on(&events, enemy_ref(t2, &b2));
    assert_eq!(hits[0].hull, 4);
    assert_eq!(hits[1].hull, 4);
    assert_eq!(b2.enemy(t2).unwrap().armor, 1);
}

#[test]
fn vance_cards_and_assault_doctrine() {
    let front = custom_unit(EnemyId::GuardCruiser, 30, 0, 2, vec![Step::Fire]);
    let back = custom_unit(EnemyId::Spotter, 20, 0, 0, vec![Step::TargetLock]);
    let fragile = custom_unit(EnemyId::Raiders, 10, 0, 0, vec![Step::Fire]);
    let mut b = battle_with(
        AdmiralId::Vance,
        [Some(front), Some(fragile), None],
        [Some(back), None, None],
    );
    b.flagship.momentum = 3;
    let f_uid = uid(&b, EnemyId::GuardCruiser);
    let b_uid = uid(&b, EnemyId::Spotter);
    let r_uid = uid(&b, EnemyId::Raiders);

    // AssaultDoctrine (5 shield on Weapons / FocusFire)
    set_hand(
        &mut b,
        &[
            CardId::AssaultDoctrine,
            CardId::Onslaught,
            CardId::ArmorPiercer,
            CardId::BreakthroughVolley,
        ],
    );
    b.play_card(0, Target::None).unwrap();
    b.run_station(Station::Weapons, Target::Enemy(f_uid))
        .unwrap();
    assert_eq!(b.flagship.shield, 5);

    // Onslaught: 7 + 2*3 = 13 piercing against covered back unit.
    b.points = 10;
    let preview = b.preview_card(0, Target::Enemy(b_uid)).unwrap();
    let events = b.play_card(0, Target::Enemy(b_uid)).unwrap();
    let hit = hits_on(&events, enemy_ref(b_uid, &b))[0];
    assert_eq!(hit.base, 13);
    assert_eq!(hit.covered, None);
    assert_eq!(preview[0].breakdown, hit);

    // ArmorPiercer: 6 piercing + Marked + -1 armor on GuardCruiser.
    let events = b.play_card(0, Target::Enemy(f_uid)).unwrap();
    assert!(b.enemy(f_uid).unwrap().marked);
    assert_eq!(b.enemy(f_uid).unwrap().armor, 1);
    assert!(events.contains(&BattleEvent::ArmorBroken {
        target: enemy_ref(f_uid, &b),
        amount: 1
    }));

    // BreakthroughVolley on Raiders (10 HP, 11 dmg -> falls -> +2 BP, draw 1).
    let points_before = b.points;
    let hand_before = b.hand.len();
    let events = b.play_card(0, Target::Enemy(r_uid)).unwrap();
    assert!(b.enemy(r_uid).is_none());
    assert_eq!(b.points, points_before - 2 + 2);
    assert_eq!(b.hand.len(), hand_before); // -1 played + 1 drawn
    assert!(events.contains(&BattleEvent::PointsGained { amount: 2 }));
}

#[test]
fn kerr_cards_time_window_and_clockwork_doctrine() {
    let charger = custom_unit(EnemyId::Besieger, 60, 0, 0, vec![Step::Charge, Step::Fire]);
    let evader = custom_unit(EnemyId::Raiders, 30, 0, 0, vec![Step::Evade, Step::Fire]);
    let mut b = battle_with(
        AdmiralId::Kerr,
        [Some(evader), Some(charger), None],
        [None, None, None],
    );
    let charger_uid = uid(&b, EnemyId::Besieger);
    let evader_uid = uid(&b, EnemyId::Raiders);

    // ClockworkDoctrine: +1 draw at round start, first Tactic/Debuff costs 1 less.
    set_hand(
        &mut b,
        &[
            CardId::ClockworkDoctrine,
            CardId::TimeWindow,
            CardId::ChronoStrike,
            CardId::PrecisionLock,
        ],
    );
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 1);
    // First Tactic (TimeWindow, base cost 1) now costs 0!
    assert_eq!(b.card_cost(0), 0);
    assert_eq!(b.card_cost(2), 0);
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 1);
    assert_eq!(b.flagship.double_next, 1);

    // Double PrecisionLock (0 BP) -> marks twice (1 Marked event), draws 1+1=2 cards.
    b.draw_pile = vec![DeckCard::new(CardId::Torpedo); 10];
    let hand_before = b.hand.len();
    let events = b.play_card(1, Target::Enemy(charger_uid)).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BattleEvent::Marked { .. }))
            .count(),
        1
    );
    assert_eq!(b.hand.len(), hand_before - 1 + 2);
    assert_eq!(b.flagship.double_next, 0);

    // ChronoStrike on charging enemy: base 10 * 2 = 20, plus Marked (+10) = 30!
    assert_eq!(b.enemy(charger_uid).unwrap().intents[0], Action::Charge);
    b.points = 2;
    let preview = b.preview_card(0, Target::Enemy(charger_uid)).unwrap();
    let events = b.play_card(0, Target::Enemy(charger_uid)).unwrap();
    let hit = hits_on(&events, enemy_ref(charger_uid, &b))[0];
    assert_eq!(hit.base, 20);
    assert_eq!(hit.marked, 10);
    assert_eq!(hit.hull, 30);
    assert_eq!(preview[0].breakdown, hit);

    // End turn -> ClockworkDoctrine draws 6 cards instead of 5 and resets tactic discount.
    b.end_turn();
    assert_eq!(b.hand.len(), 6);

    // Test TimeWindow safely stopping when 1st hit kills Target::Enemy.
    b.hand = vec![
        DeckCard {
            id: CardId::TimeWindow,
            upgraded: true,
        },
        DeckCard::new(CardId::BreakthroughVolley),
    ];
    b.enemy(evader_uid); // still alive (30 HP)
    b.enemies
        .iter_mut()
        .find(|e| e.uid == evader_uid)
        .unwrap()
        .hull = 5;
    b.points = 3;
    b.play_card(0, Target::None).unwrap();
    let events = b.play_card(0, Target::Enemy(evader_uid)).unwrap();
    assert!(b.enemy(evader_uid).is_none());
    // Only 1 BreakthroughVolley reward (+2 BP, 1 draw), not 2!
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BattleEvent::PointsGained { .. }))
            .count(),
        1
    );
}

#[test]
fn reyes_cards_and_ember_doctrine() {
    let e1 = custom_unit(EnemyId::GuardCruiser, 25, 0, 2, vec![Step::Fire]);
    let e2 = custom_unit(EnemyId::Raiders, 30, 0, 0, vec![Step::Fire]);
    let mut b = battle_with(
        AdmiralId::Reyes,
        [Some(e1), Some(e2), None],
        [None, None, None],
    );
    let u1 = uid(&b, EnemyId::GuardCruiser);
    let u2 = uid(&b, EnemyId::Raiders);

    // EmberDoctrine (upgraded -> 1 BP): +3 burn tick, +5 shield on burning kill.
    set_hand(
        &mut b,
        &[
            CardId::EmberDoctrine,
            CardId::Wildfire,
            CardId::Accelerant,
            CardId::MeltdownSalvo,
        ],
    );
    b.hand[0].upgraded = true;
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.burn_damage(enemy_ref(u1, &b)), 6); // 3 (Reyes) + 3 (EmberDoctrine)

    // Wildfire hits both for 3 (u1 armor 2 -> 1 hull, u2 -> 3 hull) and ignites both.
    b.play_card(0, Target::None).unwrap();
    assert!(b.enemy(u1).unwrap().burning);
    assert!(b.enemy(u2).unwrap().burning);

    // Accelerant on u1: 2 * 6 = 12 hull damage bypassing armor, then marks u1.
    let preview = b.preview_card(0, Target::Enemy(u1)).unwrap();
    let events = b.play_card(0, Target::Enemy(u1)).unwrap();
    let hit = hits_on(&events, enemy_ref(u1, &b))[0];
    assert_eq!(hit.base, 12);
    assert_eq!(hit.hull, 12);
    assert!(b.enemy(u1).unwrap().marked);
    assert_eq!(preview[0].breakdown, hit);

    // MeltdownSalvo (upgraded: 7 + 5*2 burning = 17) on u2 (unmarked) -> 17 hull.
    b.hand[0].upgraded = true;
    b.points = 3;
    let preview = b.preview_card(0, Target::Enemy(u2)).unwrap();
    let events = b.play_card(0, Target::Enemy(u2)).unwrap();
    assert_eq!(hits_on(&events, enemy_ref(u2, &b))[0].hull, 17);
    assert_eq!(preview[0].breakdown.hull, 17);

    // Kill burning u1 -> triggers EmberDoctrine (+5 shield, since u2 is still alive).
    b.hand = vec![DeckCard::new(CardId::Accelerant)];
    b.points = 1;
    let events = b.play_card(0, Target::Enemy(u1)).unwrap();
    assert!(b.enemy(u1).is_none());
    assert_eq!(b.flagship.shield, 5);
    assert!(events.iter().any(|e| matches!(
        e,
        BattleEvent::Shielded {
            target: UnitRef::Flagship,
            amount: 5,
            ..
        }
    )));

    // Kill final burning enemy u2 -> battle ends; EmberDoctrine must NOT emit Shielded after BattleEnded.
    b.hand = vec![DeckCard::new(CardId::Accelerant)];
    b.points = 1;
    let events = b.play_card(0, Target::Enemy(u2)).unwrap();
    assert_eq!(b.result, Some(BattleResult::Won));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, BattleEvent::Shielded { .. }))
    );
}

#[test]
fn neutral_cards_autocannon_chain_command_bridge_coordination() {
    let e1 = custom_unit(EnemyId::Raiders, 12, 0, 1, vec![Step::Evade]);
    let e2 = custom_unit(EnemyId::GuardCruiser, 30, 0, 0, vec![Step::Fire]);
    let mut b = battle_with(
        AdmiralId::Osei,
        [Some(e1), Some(e2), None],
        [None, None, None],
    );
    b.draw_pile = vec![DeckCard::new(CardId::Torpedo); 15];
    let u1 = uid(&b, EnemyId::Raiders);

    // BridgeCoordination (1 BP): draw 1 whenever a station triggers.
    // ChainCommand (1 BP): draw 2; +1 BP if a station was already triggered this round.
    set_hand(
        &mut b,
        &[
            CardId::BridgeCoordination,
            CardId::ChainCommand,
            CardId::AutocannonSalvo,
        ],
    );
    b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 2);

    // ChainCommand BEFORE station -> draws 2, no +1 BP.
    let events = b.play_card(0, Target::None).unwrap();
    assert_eq!(b.points, 1);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, BattleEvent::PointsGained { .. }))
    );

    // Trigger Shields station -> BridgeCoordination draws 1.
    let hand_before = b.hand.len();
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), hand_before + 1);

    // ChainCommand AFTER station -> draws 2 and +1 BP!
    b.hand.push(DeckCard::new(CardId::ChainCommand));
    let points_before = b.points;
    let events = b.play_card(b.hand.len() - 1, Target::None).unwrap();
    assert_eq!(b.points, points_before - 1 + 1);
    assert!(events.contains(&BattleEvent::PointsGained { amount: 1 }));

    // AutocannonSalvo (upgraded 3×3 homing) on evading Raiders (1 armor -> 2 dmg per hit * 3 = 6).
    let idx = b
        .hand
        .iter()
        .position(|c| c.id == CardId::AutocannonSalvo)
        .unwrap();
    b.hand[idx].upgraded = true;
    b.points = 2;
    let preview = b.preview_card(idx, Target::Enemy(u1)).unwrap();
    let events = b.play_card(idx, Target::Enemy(u1)).unwrap();
    let hits = hits_on(&events, enemy_ref(u1, &b));
    assert_eq!(hits.len(), 3);
    assert!(hits.iter().all(|h| h.evaded == 0 && h.hull == 2));
    assert_eq!(preview.len(), 3);
}

#[test]
fn invalid_targets_and_time_window_idempotency() {
    let mut b = simple_battle(AdmiralId::Kerr, &[(EnemyId::GuardCruiser, Row::Front, 0)]);
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    set_hand(
        &mut b,
        &[
            CardId::TimeWindow,
            CardId::EvasiveAction,
            CardId::TimeWindow,
            CardId::MicroJump,
        ],
    );
    b.points = 10;

    // Invalid column for EvasiveAction.
    assert_eq!(
        b.play_card(1, Target::Column(b.flagship.column)),
        Err(RuleError::TargetInvalid)
    );
    // Target::None on Enemy card fails.
    b.hand.push(DeckCard::new(CardId::ShieldBash));
    assert_eq!(
        b.play_card(b.hand.len() - 1, Target::None),
        Err(RuleError::TargetInvalid)
    );

    // Doubled EvasiveAction emits Maneuvered only once.
    b.play_card(0, Target::None).unwrap();
    let events = b.play_card(0, Target::Column(0)).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BattleEvent::Maneuvered { .. }))
            .count(),
        1
    );

    // Doubled MicroJump emits Jumped only once.
    b.play_card(0, Target::None).unwrap();
    let events = b.play_card(0, Target::None).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BattleEvent::Jumped))
            .count(),
        1
    );
    assert!(b.enemy(cruiser).is_some());
}

#[test]
fn t5_ember_doctrine_burn_tick_kill_no_phantom_shield_and_retaliation_parity() {
    // 1) Burn-tick kill during resolution() must NOT emit a phantom Shielded event that gets wiped to 0.
    let e1 = custom_unit(EnemyId::Raiders, 3, 0, 0, vec![Step::Fire]);
    let e2 = custom_unit(EnemyId::GuardCruiser, 50, 0, 0, vec![Step::Fire]);
    let mut b = battle_with(
        AdmiralId::Reyes,
        [Some(e1), Some(e2), None],
        [None, None, None],
    );
    set_hand(&mut b, &[CardId::EmberDoctrine]);
    b.points = 10;
    b.play_card(0, Target::None).unwrap();
    b.enemies[0].burning = true;
    b.enemies[1].burning = true;
    let events = b.end_turn();
    assert_eq!(b.enemies.len(), 1);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            BattleEvent::Shielded {
                target: UnitRef::Flagship,
                ..
            }
        )),
        "burn-tick kill in resolution() must not emit Flagship Shielded event"
    );
    assert_eq!(b.flagship.shield, 0);

    // 2) Retaliation kill during enemy_turn() DOES grant EmberDoctrine shield and protects against subsequent hits.
    let htl_shield = CardId::HoldTheLine.value(true);
    let ek = CardId::EmberDoctrine.extra(true);
    let shooter1 = custom_unit(EnemyId::Raiders, 2, 1, 0, vec![Step::Fire]);
    let shooter2 = custom_unit(
        EnemyId::GuardCruiser,
        40,
        htl_shield + ek - 1,
        0,
        vec![Step::Fire],
    );
    let mut b_ret = battle_with(
        AdmiralId::Osei,
        [Some(shooter1), Some(shooter2), None],
        [None, None, None],
    );
    set_hand(&mut b_ret, &[CardId::EmberDoctrine, CardId::HoldTheLine]);
    b_ret.hand[0].upgraded = true;
    b_ret.hand[1].upgraded = true;
    b_ret.points = 10;
    b_ret.play_card(0, Target::None).unwrap();
    b_ret.play_card(0, Target::None).unwrap();
    b_ret.enemies[0].burning = true;
    let shield_before_end = b_ret.flagship.shield;
    let hull_before_end = b_ret.flagship.hull;
    let ev_ret = b_ret.end_turn();
    assert!(
        ev_ret.iter().any(|e| matches!(
            e,
            BattleEvent::Shielded {
                target: UnitRef::Flagship,
                amount,
                ..
            } if *amount == ek
        )),
        "retaliation kill in enemy_turn() should emit EmberDoctrine Shielded"
    );
    // shooter1 (1) + shooter2 (htl_shield + ek - 1) = htl_shield + ek: absorbed only because EmberDoctrine+ granted +ek during enemy_turn().
    assert_eq!(shield_before_end, htl_shield);
    assert_eq!(b_ret.flagship.hull, hull_before_end);
    assert_eq!(b_ret.flagship.shield, 0);
}

#[test]
fn t5_chrono_strike_after_radio_silence_on_charging_enemy_not_doubled() {
    let charger = custom_unit(
        EnemyId::GuardCruiser,
        60,
        5,
        0,
        vec![Step::Charge, Step::Fire],
    );
    let mut b = battle_with(
        AdmiralId::Kerr,
        [Some(charger), None, None],
        [None, None, None],
    );
    let u = uid(&b, EnemyId::GuardCruiser);
    assert_eq!(b.enemies[0].intents[0], Action::Charge);
    let val = CardId::ChronoStrike.value(false);

    set_hand(
        &mut b,
        &[
            CardId::RadioSilence,
            CardId::ChronoStrike,
            CardId::ChronoStrike,
        ],
    );
    b.points = 10;

    // Before RadioSilence: preview shows 2× damage.
    let pre_before = b.preview_card(1, Target::Enemy(u)).unwrap();
    assert_eq!(pre_before[0].breakdown.base, 2 * val);

    // Silence cancels Charge intent -> ChronoStrike must NOT double in preview or execution.
    b.play_card(0, Target::Enemy(u)).unwrap();
    assert!(b.enemies[0].silenced);
    assert_eq!(b.enemies[0].intents[0], Action::Charge);
    let pre_after = b.preview_card(0, Target::Enemy(u)).unwrap();
    assert_eq!(pre_after[0].breakdown.base, val);
    let ev = b.play_card(0, Target::Enemy(u)).unwrap();
    assert_eq!(hits_on(&ev, enemy_ref(u, &b))[0], pre_after[0].breakdown);
    assert_eq!(b.enemy(u).unwrap().hull, 60 - val);

    // Stasis still doubles ChronoStrike even on a silenced target.
    b.run_station(Station::Admiral, Target::Enemy(u)).unwrap();
    let pre_stasis = b.preview_card(0, Target::Enemy(u)).unwrap();
    assert_eq!(pre_stasis[0].breakdown.base, 2 * val);
    let ev_stasis = b.play_card(0, Target::Enemy(u)).unwrap();
    assert_eq!(
        hits_on(&ev_stasis, enemy_ref(u, &b))[0],
        pre_stasis[0].breakdown
    );
    assert_eq!(b.enemy(u).unwrap().hull, 60 - 3 * val);
}

#[test]
fn t5_time_window_tractor_beam_does_not_undo_pull() {
    use voidfleet_core::board::EnemySlot;

    // Case 1: Only Front-1 is free (Front-0 GuardCruiser covering Back-0 Spotter, Front-2 Raiders).
    let front_cruiser = custom_unit(EnemyId::GuardCruiser, 40, 4, 0, vec![Step::Fire]);
    let front_raiders = custom_unit(EnemyId::Raiders, 15, 2, 0, vec![Step::Fire]);
    let back_spotter = custom_unit(EnemyId::Spotter, 20, 0, 0, vec![Step::TargetLock]);
    let mut b = battle_with(
        AdmiralId::Kerr,
        [Some(front_cruiser), None, Some(front_raiders)],
        [Some(back_spotter), None, None],
    );
    let cruiser = uid(&b, EnemyId::GuardCruiser);
    let spotter = uid(&b, EnemyId::Spotter);
    assert!(b.covered(spotter));

    set_hand(&mut b, &[CardId::TimeWindow, CardId::TractorBeam]);
    b.points = 10;
    b.play_card(0, Target::None).unwrap();
    let events = b.play_card(0, Target::Enemy(cruiser)).unwrap();
    assert_eq!(b.flagship.double_next, 0);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BattleEvent::Tractored { .. }))
            .count(),
        1
    );
    assert_eq!(
        b.enemy(cruiser).unwrap().slot,
        EnemySlot::new(Row::Front, 1)
    );
    assert!(!b.covered(spotter));

    // Case 2: Both Front-1 and Front-2 are free; still pulls only once (to Front-1).
    let mut b2 = battle_with(
        AdmiralId::Kerr,
        [
            Some(custom_unit(
                EnemyId::GuardCruiser,
                40,
                4,
                0,
                vec![Step::Fire],
            )),
            None,
            None,
        ],
        [
            Some(custom_unit(
                EnemyId::Spotter,
                20,
                0,
                0,
                vec![Step::TargetLock],
            )),
            None,
            None,
        ],
    );
    let c2 = uid(&b2, EnemyId::GuardCruiser);
    let s2 = uid(&b2, EnemyId::Spotter);
    set_hand(&mut b2, &[CardId::TimeWindow, CardId::TractorBeam]);
    b2.points = 10;
    b2.play_card(0, Target::None).unwrap();
    let ev2 = b2.play_card(0, Target::Enemy(c2)).unwrap();
    assert_eq!(
        ev2.iter()
            .filter(|e| matches!(e, BattleEvent::Tractored { .. }))
            .count(),
        1
    );
    assert_eq!(b2.enemy(c2).unwrap().slot, EnemySlot::new(Row::Front, 1));
    assert!(!b2.covered(s2));
}

#[test]
fn t5_shield_bash_zero_base_does_not_mark_hit_for_unknown_contact_or_raiders() {
    // 1) UnknownContact still regenerates after a 0-base ShieldBash.
    let mut b_uc = simple_battle(AdmiralId::Osei, &[(EnemyId::UnknownContact, Row::Front, 1)]);
    let contact = uid(&b_uc, EnemyId::UnknownContact);
    let start_hull = b_uc.enemies[0].max_hull - 10;
    b_uc.enemies[0].hull = start_hull;
    assert_eq!(b_uc.flagship.shield, 0);
    set_hand(&mut b_uc, &[CardId::ShieldBash]);
    b_uc.points = 5;
    let preview = b_uc.preview_card(0, Target::Enemy(contact)).unwrap();
    assert_eq!(preview[0].breakdown.base, 0);
    let ev_play = b_uc.play_card(0, Target::Enemy(contact)).unwrap();
    assert_eq!(hits_on(&ev_play, enemy_ref(contact, &b_uc))[0].base, 0);
    assert!(!b_uc.enemy(contact).unwrap().hit);
    assert!(b_uc.enemy(contact).unwrap().regeneration() > 0);
    let ev_end = b_uc.end_turn();
    assert!(
        ev_end
            .iter()
            .any(|e| matches!(e, BattleEvent::Regenerated { amount, .. } if *amount > 0))
    );
    assert!(b_uc.enemy(contact).unwrap().hull > start_hull);

    // 2) Raiders does NOT switch next intent to Evade after a 0-base ShieldBash.
    let raiders = custom_unit(EnemyId::Raiders, 15, 2, 0, vec![Step::Fire, Step::Fire]);
    let mut b_r = battle_with(
        AdmiralId::Osei,
        [Some(raiders), None, None],
        [None, None, None],
    );
    let r_uid = uid(&b_r, EnemyId::Raiders);
    assert_eq!(b_r.flagship.shield, 0);
    set_hand(&mut b_r, &[CardId::ShieldBash]);
    b_r.points = 5;
    b_r.play_card(0, Target::Enemy(r_uid)).unwrap();
    assert!(!b_r.enemy(r_uid).unwrap().hit);
    b_r.end_turn();
    assert_eq!(b_r.enemy(r_uid).unwrap().intents[0], Action::Fire);
    assert!(!b_r.enemy(r_uid).unwrap().is_evading());

    // 3) Positive control: 0 shields WITH FireControlDoctrine (base > 0) DOES set enemy.hit = true.
    let mut b_fc = simple_battle(AdmiralId::Osei, &[(EnemyId::UnknownContact, Row::Front, 1)]);
    let c_fc = uid(&b_fc, EnemyId::UnknownContact);
    b_fc.enemies[0].hull = b_fc.enemies[0].max_hull - 10;
    set_hand(
        &mut b_fc,
        &[CardId::FireControlDoctrine, CardId::ShieldBash],
    );
    b_fc.points = 10;
    b_fc.play_card(0, Target::None).unwrap();
    b_fc.play_card(0, Target::Enemy(c_fc)).unwrap();
    assert!(b_fc.enemy(c_fc).unwrap().hit);
    assert_eq!(b_fc.enemy(c_fc).unwrap().regeneration(), 0);
}

#[test]
fn t5_collision_course_mutual_kill_and_last_kill_with_emergency_generator() {
    // 1) Mutual kill on last enemy (no EmergencyGenerator) -> BattleResult::Lost, single BattleEnded, zero trailing events.
    let mut b_mut = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(
                EnemyId::GuardCruiser,
                10,
                10,
                0,
                vec![Step::Fire],
            )),
            None,
            None,
        ],
        [None, None, None],
    );
    b_mut.flagship.hull = 5;
    let cruiser = uid(&b_mut, EnemyId::GuardCruiser);
    set_hand(&mut b_mut, &[CardId::CollisionCourse]);
    b_mut.points = 5;
    let pre_mut = b_mut.preview_card(0, Target::Enemy(cruiser)).unwrap();
    assert_eq!(pre_mut.len(), 2);
    let ev_mut = b_mut.play_card(0, Target::Enemy(cruiser)).unwrap();
    assert_eq!(b_mut.result, Some(BattleResult::Lost));
    assert_eq!(b_mut.flagship.hull, 0);
    assert!(b_mut.enemy(cruiser).is_none());
    assert_eq!(
        ev_mut
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_mut.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Lost
        })
    );

    // 2) Last enemy kill where flagship survives via EmergencyGenerator -> Won, EmergencyGenerator BEFORE BattleEnded, zero trailing events.
    let mut b_eg = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(
                EnemyId::GuardCruiser,
                10,
                10,
                0,
                vec![Step::Fire],
            )),
            None,
            None,
        ],
        [None, None, None],
    );
    b_eg.flagship.hull = 5;
    b_eg.emergency_generator_available = true;
    let cruiser_eg = uid(&b_eg, EnemyId::GuardCruiser);
    set_hand(&mut b_eg, &[CardId::CollisionCourse]);
    b_eg.points = 5;
    let ev_eg = b_eg.play_card(0, Target::Enemy(cruiser_eg)).unwrap();
    assert_eq!(b_eg.result, Some(BattleResult::Won));
    assert_eq!(b_eg.flagship.hull, 1);
    assert!(b_eg.emergency_generator_used);
    assert!(b_eg.enemy(cruiser_eg).is_none());
    assert_eq!(
        ev_eg
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_eg.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );
    let eg_idx = ev_eg
        .iter()
        .position(|e| matches!(e, BattleEvent::EmergencyGenerator { .. }))
        .expect("EmergencyGenerator must trigger");
    let end_idx = ev_eg.len() - 1;
    assert!(eg_idx < end_idx);

    // 3) Last burning enemy kill with EmberDoctrine -> Won, no Shielded after BattleEnded.
    let mut b_ember_last = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(
                EnemyId::GuardCruiser,
                10,
                4,
                0,
                vec![Step::Fire],
            )),
            None,
            None,
        ],
        [None, None, None],
    );
    let c_el = uid(&b_ember_last, EnemyId::GuardCruiser);
    b_ember_last.enemies[0].burning = true;
    set_hand(
        &mut b_ember_last,
        &[CardId::EmberDoctrine, CardId::CollisionCourse],
    );
    b_ember_last.points = 10;
    b_ember_last.play_card(0, Target::None).unwrap();
    let ev_el = b_ember_last.play_card(0, Target::Enemy(c_el)).unwrap();
    assert_eq!(b_ember_last.result, Some(BattleResult::Won));
    assert_eq!(
        b_ember_last.flagship.hull,
        b_ember_last.flagship.max_hull - 4
    );
    assert!(
        !ev_el
            .iter()
            .any(|e| matches!(e, BattleEvent::Shielded { .. })),
        "no Shielded event should be emitted when battle ends on CollisionCourse"
    );
    assert_eq!(
        ev_el.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );

    // 4) Non-last burning enemy kill with EmberDoctrine -> battle continues, flagship takes collision damage and gains EmberDoctrine shield.
    let mut b_ember_mid = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(
                EnemyId::GuardCruiser,
                10,
                4,
                0,
                vec![Step::Fire],
            )),
            Some(custom_unit(
                EnemyId::GuardCruiser,
                30,
                4,
                0,
                vec![Step::Fire],
            )),
            None,
        ],
        [None, None, None],
    );
    let c_em = b_ember_mid.enemies[0].uid;
    b_ember_mid.enemies[0].burning = true;
    set_hand(
        &mut b_ember_mid,
        &[CardId::EmberDoctrine, CardId::CollisionCourse],
    );
    b_ember_mid.points = 10;
    b_ember_mid.play_card(0, Target::None).unwrap();
    let ev_em = b_ember_mid.play_card(0, Target::Enemy(c_em)).unwrap();
    assert_eq!(b_ember_mid.result, None);
    assert_eq!(b_ember_mid.flagship.hull, b_ember_mid.flagship.max_hull - 4);
    assert_eq!(
        b_ember_mid.flagship.shield,
        CardId::EmberDoctrine.extra(false)
    );
    assert!(ev_em.iter().any(|e| matches!(
        e,
        BattleEvent::Shielded {
            target: UnitRef::Flagship,
            ..
        }
    )));

    // 5) Mutual kill on Seal in a breach encounter -> Lost, no Breach triggered, single BattleEnded.
    let mut b_breach = battle_with(
        AdmiralId::Vance,
        [None, None, None],
        [
            None,
            Some(custom_unit(EnemyId::Seal, 10, 10, 0, vec![Step::Bolt])),
            None,
        ],
    );
    b_breach.breach = Some(EnemyId::UnknownContact.into());
    b_breach.flagship.hull = 5;
    let seal = uid(&b_breach, EnemyId::Seal);
    set_hand(&mut b_breach, &[CardId::CollisionCourse]);
    b_breach.points = 5;
    let ev_breach = b_breach.play_card(0, Target::Enemy(seal)).unwrap();
    assert_eq!(b_breach.result, Some(BattleResult::Lost));
    assert!(!ev_breach.iter().any(|e| matches!(e, BattleEvent::Breach)));
    assert_eq!(
        ev_breach
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_breach.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Lost
        })
    );
}

#[test]
fn t5_aoe_cards_and_follow_up_stop_on_battle_ended() {
    // 1) VolleyFire stops immediately when primary enemy dies leaving only Spotter.
    let mut b_vf = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(EnemyId::Raiders, 2, 2, 0, vec![Step::Fire])),
            None,
            None,
        ],
        [
            Some(custom_unit(
                EnemyId::Spotter,
                10,
                0,
                0,
                vec![Step::TargetLock],
            )),
            None,
            None,
        ],
    );
    set_hand(&mut b_vf, &[CardId::VolleyFire]);
    let ev_vf = b_vf.play_card(0, Target::None).unwrap();
    assert_eq!(b_vf.result, Some(BattleResult::Won));
    assert_eq!(b_vf.kills, 1);
    assert_eq!(b_vf.enemies.len(), 1);
    assert_eq!(b_vf.enemies[0].hull, 10);
    assert_eq!(
        ev_vf
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_vf.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );

    // 2) Barrage stops immediately when first evading enemy dies leaving only evading Spotter.
    let mut b_bar = battle_with(
        AdmiralId::Vance,
        [
            Some(custom_unit(EnemyId::Raiders, 2, 2, 0, vec![Step::Evade])),
            None,
            None,
        ],
        [
            Some(custom_unit(EnemyId::Spotter, 10, 0, 0, vec![Step::Evade])),
            None,
            None,
        ],
    );
    set_hand(&mut b_bar, &[CardId::Barrage]);
    let ev_bar = b_bar.play_card(0, Target::None).unwrap();
    assert_eq!(b_bar.result, Some(BattleResult::Won));
    assert_eq!(b_bar.kills, 1);
    assert_eq!(b_bar.enemies.len(), 1);
    assert_eq!(b_bar.enemies[0].hull, 10);
    assert_eq!(
        ev_bar
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_bar.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );

    // 3) Bombardment stops immediately when Seal in Back dies (non-breach), leaving Back Spotter untouched.
    let mut b_bom = battle_with(
        AdmiralId::Vance,
        [None, None, None],
        [
            Some(custom_unit(EnemyId::Seal, 2, 0, 0, vec![Step::Bolt])),
            Some(custom_unit(
                EnemyId::Spotter,
                10,
                0,
                0,
                vec![Step::TargetLock],
            )),
            None,
        ],
    );
    set_hand(&mut b_bom, &[CardId::Bombardment]);
    let ev_bom = b_bom.play_card(0, Target::None).unwrap();
    assert_eq!(b_bom.result, Some(BattleResult::Won));
    assert_eq!(b_bom.kills, 1);
    assert_eq!(b_bom.enemies.len(), 1);
    assert_eq!(b_bom.enemies[0].hull, 10);
    assert_eq!(
        ev_bom
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_bom.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );

    // 4) AdmiralStation::FollowUp stops immediately when burning primary enemy dies leaving only burning Spotter.
    let mut b_fu = battle_with(
        AdmiralId::Reyes,
        [
            Some(custom_unit(EnemyId::Raiders, 4, 2, 0, vec![Step::Fire])),
            None,
            None,
        ],
        [
            Some(custom_unit(
                EnemyId::Spotter,
                10,
                0,
                0,
                vec![Step::TargetLock],
            )),
            None,
            None,
        ],
    );
    b_fu.enemies[0].burning = true;
    b_fu.enemies[1].burning = true;
    let ev_fu = b_fu.run_station(Station::Admiral, Target::None).unwrap();
    assert_eq!(b_fu.result, Some(BattleResult::Won));
    assert_eq!(b_fu.kills, 1);
    assert_eq!(b_fu.enemies.len(), 1);
    assert_eq!(b_fu.enemies[0].hull, 10);
    assert_eq!(
        ev_fu
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_fu.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );
}

#[test]
fn t5_cover_spill_stops_on_battle_ended() {
    // 1) Covered Seal (non-breach) killed by Torpedo -> front ship untouched, single BattleEnded, preview parity.
    let mut b_seal = battle_with(
        AdmiralId::Vance,
        [
            None,
            Some(custom_unit(
                EnemyId::GuardCruiser,
                10,
                4,
                0,
                vec![Step::Fire],
            )),
            None,
        ],
        [
            None,
            Some(custom_unit(EnemyId::Seal, 2, 0, 0, vec![Step::Bolt])),
            None,
        ],
    );
    let seal = uid(&b_seal, EnemyId::Seal);
    let cruiser = uid(&b_seal, EnemyId::GuardCruiser);
    set_hand(&mut b_seal, &[CardId::Torpedo]);
    b_seal.points = 5;
    let preview = b_seal.preview_card(0, Target::Enemy(seal)).unwrap();
    assert_eq!(preview.len(), 1);
    let ev_seal = b_seal.play_card(0, Target::Enemy(seal)).unwrap();
    assert_eq!(b_seal.result, Some(BattleResult::Won));
    assert_eq!(b_seal.kills, 1);
    assert_eq!(b_seal.enemies.len(), 1);
    assert_eq!(b_seal.enemy(cruiser).unwrap().hull, 10);
    assert_eq!(
        ev_seal
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_seal.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );

    // 2) Covered GuardCruiser behind Spotter killed by Torpedo -> Spotter untouched, single BattleEnded.
    let mut b_spot = battle_with(
        AdmiralId::Vance,
        [
            None,
            Some(custom_unit(
                EnemyId::Spotter,
                10,
                0,
                0,
                vec![Step::TargetLock],
            )),
            None,
        ],
        [
            None,
            Some(custom_unit(
                EnemyId::GuardCruiser,
                2,
                4,
                0,
                vec![Step::Fire],
            )),
            None,
        ],
    );
    let gc = uid(&b_spot, EnemyId::GuardCruiser);
    let spotter = uid(&b_spot, EnemyId::Spotter);
    set_hand(&mut b_spot, &[CardId::Torpedo]);
    b_spot.points = 5;
    let ev_spot = b_spot.play_card(0, Target::Enemy(gc)).unwrap();
    assert_eq!(b_spot.result, Some(BattleResult::Won));
    assert_eq!(b_spot.kills, 1);
    assert_eq!(b_spot.enemies.len(), 1);
    assert_eq!(b_spot.enemy(spotter).unwrap().hull, 10);
    assert_eq!(
        ev_spot
            .iter()
            .filter(|e| matches!(e, BattleEvent::BattleEnded { .. }))
            .count(),
        1
    );
    assert_eq!(
        ev_spot.last(),
        Some(&BattleEvent::BattleEnded {
            result: BattleResult::Won
        })
    );
}
