use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use rand::SeedableRng;
use rand::rngs::StdRng;
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId, AdmiralStation, FlagshipId, Station};
use voidfleet_core::battle::{
    Battle, BattleResult, BattleSetup, Encounter, Reinforcement, RuleError, Target,
};
use voidfleet_core::board::{COLUMNS, EnemySlot, Row};
use voidfleet_core::cards::{
    CardId, CardTarget, CardType, DeckCard, EXTRA_CARDS, admiral_pool, neutral_pool, starter_deck,
};
use voidfleet_core::data::{Data, REPO_DATA, Step, data, read, rules};
use voidfleet_core::enemies::{Action, EnemyId, Unit};
use voidfleet_core::events::{BattleEvent, Breakdown, DestroyCause, Hit, UnitRef};
use voidfleet_core::modules::ModuleId;
use voidfleet_core::run::{Gain, MapNode, NodeKind, Run, SECTORS, SectorMap, Stage};

fn load_repo_data() -> Data {
    Data::load(Path::new(REPO_DATA)).expect("repository data must load cleanly")
}

fn load_sector_maps() -> [SectorMap; SECTORS as usize] {
    [1, 2, 3].map(|s| {
        read(
            Path::new(REPO_DATA),
            &format!("maps/{}", SectorMap::file(s)),
        )
        .expect("sector map must load")
    })
}

fn unit(id: EnemyId, hull: i32, attack: i32, armor: i32, pattern: Vec<Step>) -> Unit {
    Unit {
        id,
        hull,
        attack,
        armor,
        pattern,
    }
}

fn dummy_front(hull: i32, attack: i32, armor: i32) -> Unit {
    unit(EnemyId::Raiders, hull, attack, armor, vec![Step::Fire])
}

fn encounter_single(u: Unit) -> Encounter {
    Encounter {
        front: [Some(u), None, None],
        ..Encounter::default()
    }
}

fn battle_with(
    admiral: AdmiralId,
    deck: Vec<DeckCard>,
    encounter: Encounter,
    modules: Vec<ModuleId>,
    seed: u64,
) -> Battle {
    Battle::new(
        BattleSetup {
            admiral,
            hull: admiral.hull(),
            max_hull: admiral.hull(),
            deck,
            modules,
            emergency_generator_available: false,
            encounter,
        },
        seed,
    )
}

fn battle_requiring_hand(
    admiral: AdmiralId,
    deck: Vec<DeckCard>,
    encounter: Encounter,
    modules: Vec<ModuleId>,
    required: &[CardId],
) -> Battle {
    for seed in 1..20_000 {
        let battle = battle_with(
            admiral,
            deck.clone(),
            encounter.clone(),
            modules.clone(),
            seed,
        );
        if required
            .iter()
            .all(|id| battle.hand.iter().any(|c| c.id == *id))
        {
            return battle;
        }
    }
    panic!("failed to find seed drawing required cards: {required:?}");
}

fn find_card(battle: &Battle, id: CardId) -> usize {
    battle
        .hand
        .iter()
        .position(|c| c.id == id)
        .unwrap_or_else(|| panic!("card {id:?} not in hand: {:?}", battle.hand))
}

fn find_card_upg(battle: &Battle, id: CardId, upgraded: bool) -> usize {
    battle
        .hand
        .iter()
        .position(|c| c.id == id && c.upgraded == upgraded)
        .unwrap_or_else(|| panic!("card ({id:?}, {upgraded}) not in hand"))
}

fn play(battle: &mut Battle, id: CardId, target: Target) -> Vec<BattleEvent> {
    let idx = find_card(battle, id);
    battle
        .play_card(idx, target)
        .unwrap_or_else(|e| panic!("play_card({id:?}, {target:?}) failed: {e:?}"))
}

fn play_upg(battle: &mut Battle, id: CardId, upgraded: bool, target: Target) -> Vec<BattleEvent> {
    let idx = find_card_upg(battle, id, upgraded);
    battle
        .play_card(idx, target)
        .unwrap_or_else(|e| panic!("play_card(({id:?}, {upgraded}), {target:?}) failed: {e:?}"))
}

fn hit_breakdowns(events: &[BattleEvent], target_uid: u32) -> Vec<Breakdown> {
    events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Hit {
                target: UnitRef::Enemy { uid, .. },
                breakdown,
                ..
            } if *uid == target_uid => Some(*breakdown),
            _ => None,
        })
        .collect()
}

fn assert_preview_matches_execution(
    battle: &mut Battle,
    hand_index: usize,
    target: Target,
) -> Vec<BattleEvent> {
    let preview: Vec<Hit> = battle
        .preview_card(hand_index, target)
        .expect("preview_card must succeed");
    let events = battle
        .play_card(hand_index, target)
        .expect("play_card must succeed");
    let actual: Vec<Hit> = events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Hit {
                target, breakdown, ..
            } => Some(Hit {
                target: *target,
                breakdown: *breakdown,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(preview, actual, "preview_card must match actual Hit events");
    events
}

fn synthetic_battle_maps(kind_at_0: NodeKind) -> [SectorMap; 3] {
    let boss_encounter = encounter_single(dummy_front(10, 1, 0));
    let make_sector = |first_kind: NodeKind| {
        let first_enc = match first_kind {
            NodeKind::Battle | NodeKind::Elite => Some(encounter_single(dummy_front(10, 1, 0))),
            _ => None,
        };
        SectorMap {
            rows: vec![
                vec![MapNode {
                    kind: first_kind,
                    next: vec![0],
                    encounter: first_enc,
                }],
                vec![MapNode {
                    kind: NodeKind::Boss,
                    next: vec![],
                    encounter: Some(boss_encounter.clone()),
                }],
            ],
        }
    };
    [
        make_sector(kind_at_0),
        make_sector(NodeKind::Battle),
        make_sector(NodeKind::Battle),
    ]
}

fn card_count(deck: &[DeckCard], id: CardId) -> usize {
    deck.iter().filter(|c| c.id == id).count()
}

// ============================================================================
// TIER 1 — FEATURE COVERAGE (F1–F14, 95 tests)
// ============================================================================

// --- F1: Starter Decks & 26 Existing Cards Preservation (6 tests) ---

#[test]
fn t1_f1_starter_decks_unchanged() {
    let d = load_repo_data();
    let expected_sig = [
        (AdmiralId::Osei, CardId::HoldTheLine),
        (AdmiralId::Vance, CardId::Overcharge),
        (AdmiralId::Kerr, CardId::MicroJump),
        (AdmiralId::Reyes, CardId::Firebomb),
    ];
    for (admiral, sig) in expected_sig {
        let deck = starter_deck(admiral);
        assert_eq!(
            deck.len(),
            10,
            "{admiral:?} starter deck must have 10 cards"
        );
        assert!(deck.iter().all(|c| !c.upgraded));
        assert_eq!(card_count(&deck, CardId::Torpedo), 3);
        assert_eq!(card_count(&deck, CardId::EmergencyShields), 3);
        assert_eq!(card_count(&deck, CardId::RepairCrew), 1);
        assert_eq!(card_count(&deck, CardId::Scavenge), 1);
        assert_eq!(card_count(&deck, sig), 2);
        assert_eq!(d.admirals[&admiral].signature_card, sig);
        assert_eq!(admiral.signature_card(), sig);
    }
}

#[test]
fn t1_f1_neutral_existing_22_data_unchanged() {
    type ExistingSpec = (
        CardId,
        CardType,
        CardTarget,
        u32,
        i32,
        Option<i32>,
        Option<u32>,
        Option<i32>,
    );
    let d = load_repo_data();
    let specs: &[ExistingSpec] = &[
        (
            CardId::Torpedo,
            CardType::Attack,
            CardTarget::Enemy,
            2,
            9,
            None,
            None,
            Some(12),
        ),
        (
            CardId::OrbitalStrike,
            CardType::Attack,
            CardTarget::Enemy,
            3,
            16,
            None,
            None,
            Some(21),
        ),
        (
            CardId::VolleyFire,
            CardType::Attack,
            CardTarget::None,
            1,
            3,
            None,
            None,
            Some(5),
        ),
        (
            CardId::Barrage,
            CardType::Attack,
            CardTarget::None,
            0,
            6,
            None,
            None,
            Some(9),
        ),
        (
            CardId::HeatSeeker,
            CardType::Attack,
            CardTarget::Enemy,
            2,
            9,
            Some(4),
            None,
            Some(12),
        ),
        (
            CardId::CoupDeGrace,
            CardType::Attack,
            CardTarget::Enemy,
            1,
            4,
            Some(8),
            None,
            Some(6),
        ),
        (
            CardId::Bombardment,
            CardType::Attack,
            CardTarget::None,
            1,
            5,
            None,
            None,
            Some(8),
        ),
        (
            CardId::CollisionCourse,
            CardType::Attack,
            CardTarget::Enemy,
            2,
            0,
            None,
            None,
            Some(4),
        ),
        (
            CardId::EmergencyShields,
            CardType::Buff,
            CardTarget::None,
            1,
            5,
            None,
            None,
            Some(8),
        ),
        (
            CardId::AblativeArmor,
            CardType::Doctrine,
            CardTarget::None,
            1,
            1,
            None,
            None,
            Some(2),
        ),
        (
            CardId::PowerSurge,
            CardType::Buff,
            CardTarget::None,
            1,
            7,
            None,
            None,
            Some(10),
        ),
        (
            CardId::RepairCrew,
            CardType::Buff,
            CardTarget::None,
            1,
            4,
            None,
            None,
            Some(6),
        ),
        (
            CardId::EmpBlast,
            CardType::Debuff,
            CardTarget::Enemy,
            1,
            0,
            None,
            Some(0),
            None,
        ),
        (
            CardId::TargetingJammer,
            CardType::Debuff,
            CardTarget::None,
            1,
            3,
            None,
            None,
            Some(4),
        ),
        (
            CardId::RadioSilence,
            CardType::Debuff,
            CardTarget::Enemy,
            3,
            0,
            None,
            Some(2),
            None,
        ),
        (
            CardId::TractorBeam,
            CardType::Debuff,
            CardTarget::Enemy,
            1,
            0,
            None,
            Some(0),
            None,
        ),
        (
            CardId::EvasiveAction,
            CardType::Tactic,
            CardTarget::Column,
            1,
            0,
            None,
            Some(0),
            None,
        ),
        (
            CardId::Countermeasures,
            CardType::Tactic,
            CardTarget::None,
            1,
            0,
            None,
            Some(0),
            None,
        ),
        (
            CardId::Scavenge,
            CardType::Tactic,
            CardTarget::None,
            1,
            0,
            Some(3),
            Some(0),
            None,
        ),
        (
            CardId::RedAlert,
            CardType::Tactic,
            CardTarget::None,
            2,
            0,
            None,
            Some(1),
            None,
        ),
        (
            CardId::FireControlDoctrine,
            CardType::Doctrine,
            CardTarget::None,
            1,
            1,
            None,
            None,
            Some(2),
        ),
        (
            CardId::BattleStations,
            CardType::Doctrine,
            CardTarget::None,
            1,
            2,
            None,
            None,
            Some(3),
        ),
    ];
    assert_eq!(specs.len(), 22);
    for &(id, ct, tgt, cost, val, extra, upg_cost, upg_val) in specs {
        let cd = &d.cards[&id];
        assert_eq!(cd.card_type, ct, "{id:?} card_type");
        assert_eq!(cd.target, tgt, "{id:?} target");
        assert_eq!(cd.cost, cost, "{id:?} cost");
        assert_eq!(cd.value, val, "{id:?} value");
        assert_eq!(cd.extra, extra, "{id:?} extra");
        assert_eq!(cd.upgraded.cost, upg_cost, "{id:?} upgraded.cost");
        assert_eq!(cd.upgraded.value, upg_val, "{id:?} upgraded.value");
        assert_eq!(cd.upgraded.extra, None, "{id:?} upgraded.extra");
        assert!(!id.exhaust(), "{id:?} must not exhaust");
        assert!(!id.retain(), "{id:?} must not retain");
    }
}

#[test]
fn t1_f1_signature_existing_4_data_unchanged_except_micro_jump_exhaust() {
    let d = load_repo_data();
    assert_eq!(d.cards[&CardId::HoldTheLine].card_type, CardType::Buff);
    assert_eq!(d.cards[&CardId::HoldTheLine].target, CardTarget::None);
    assert_eq!(d.cards[&CardId::HoldTheLine].cost, 1);
    assert_eq!(d.cards[&CardId::HoldTheLine].value, 4);
    assert_eq!(d.cards[&CardId::HoldTheLine].extra, Some(2));
    assert_eq!(d.cards[&CardId::HoldTheLine].upgraded.cost, None);
    assert_eq!(d.cards[&CardId::HoldTheLine].upgraded.value, Some(6));
    assert_eq!(d.cards[&CardId::HoldTheLine].upgraded.extra, None);
    assert!(!CardId::HoldTheLine.exhaust());
    assert!(!CardId::HoldTheLine.retain());

    assert_eq!(d.cards[&CardId::Overcharge].card_type, CardType::Buff);
    assert_eq!(d.cards[&CardId::Overcharge].target, CardTarget::None);
    assert_eq!(d.cards[&CardId::Overcharge].cost, 0);
    assert_eq!(d.cards[&CardId::Overcharge].value, 6);
    assert_eq!(d.cards[&CardId::Overcharge].extra, Some(2));
    assert_eq!(d.cards[&CardId::Overcharge].upgraded.cost, None);
    assert_eq!(d.cards[&CardId::Overcharge].upgraded.value, Some(9));
    assert_eq!(d.cards[&CardId::Overcharge].upgraded.extra, None);
    assert!(!CardId::Overcharge.exhaust());
    assert!(!CardId::Overcharge.retain());

    assert_eq!(d.cards[&CardId::Firebomb].card_type, CardType::Attack);
    assert_eq!(d.cards[&CardId::Firebomb].target, CardTarget::Enemy);
    assert_eq!(d.cards[&CardId::Firebomb].cost, 1);
    assert_eq!(d.cards[&CardId::Firebomb].value, 4);
    assert_eq!(d.cards[&CardId::Firebomb].extra, None);
    assert_eq!(d.cards[&CardId::Firebomb].upgraded.cost, None);
    assert_eq!(d.cards[&CardId::Firebomb].upgraded.value, Some(6));
    assert_eq!(d.cards[&CardId::Firebomb].upgraded.extra, None);
    assert!(!CardId::Firebomb.exhaust());
    assert!(!CardId::Firebomb.retain());

    assert_eq!(d.cards[&CardId::MicroJump].card_type, CardType::Tactic);
    assert_eq!(d.cards[&CardId::MicroJump].target, CardTarget::None);
    assert_eq!(d.cards[&CardId::MicroJump].cost, 3);
    assert_eq!(d.cards[&CardId::MicroJump].value, 0);
    assert_eq!(d.cards[&CardId::MicroJump].extra, None);
    assert_eq!(d.cards[&CardId::MicroJump].upgraded.cost, Some(2));
    assert_eq!(d.cards[&CardId::MicroJump].upgraded.value, None);
    assert_eq!(d.cards[&CardId::MicroJump].upgraded.extra, None);
    assert!(CardId::MicroJump.exhaust(), "micro_jump must gain Einmalig");
    assert!(!CardId::MicroJump.retain());
}

#[test]
fn t1_f1_existing_attacks_behavior_preserved() {
    let enc = Encounter {
        front: [
            Some(unit(EnemyId::Raiders, 50, 3, 1, vec![Step::Evade])),
            Some(unit(EnemyId::GuardCruiser, 50, 4, 0, vec![Step::Fire])),
            None,
        ],
        back: [
            None,
            Some(unit(EnemyId::Besieger, 50, 5, 0, vec![Step::Fire])),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::VolleyFire),
        DeckCard::new(CardId::Bombardment),
        DeckCard::new(CardId::HeatSeeker),
        DeckCard::new(CardId::Firebomb),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let evader = b.enemies[0].uid;
    let front_mid = b.enemies[1].uid;
    let back_mid = b.enemies[2].uid;

    // Torpedo (9, homing) vs evader with 1 armor -> 8 dmg
    b.points = 10;
    play(&mut b, CardId::Torpedo, Target::Enemy(evader));
    assert_eq!(b.enemy(evader).unwrap().hull, 42);

    // HeatSeeker (9 + 4 = 13, homing, piercing) vs evader with 1 armor -> 12 dmg
    play(&mut b, CardId::HeatSeeker, Target::Enemy(evader));
    assert_eq!(b.enemy(evader).unwrap().hull, 30);

    // VolleyFire (3 piercing) hits all 3 enemies
    play(&mut b, CardId::VolleyFire, Target::None);
    assert_eq!(b.enemy(front_mid).unwrap().hull, 47);
    assert_eq!(b.enemy(back_mid).unwrap().hull, 47);

    // Bombardment (5 piercing) hits only Back row
    play(&mut b, CardId::Bombardment, Target::None);
    assert_eq!(b.enemy(front_mid).unwrap().hull, 47);
    assert_eq!(b.enemy(back_mid).unwrap().hull, 42);

    // Firebomb (4 + burn) on front_mid
    play(&mut b, CardId::Firebomb, Target::Enemy(front_mid));
    assert_eq!(b.enemy(front_mid).unwrap().hull, 43);
    assert!(b.enemy(front_mid).unwrap().burning);

    // Barrage (0 BP, Homing, hits ALL evading enemies) + OrbitalStrike (3 BP, 16 dmg, Piercing)
    let enc2 = Encounter {
        front: [
            Some(unit(EnemyId::Raiders, 50, 3, 1, vec![Step::Evade])),
            Some(unit(EnemyId::GuardCruiser, 50, 4, 0, vec![Step::Fire])),
            Some(unit(EnemyId::Raiders, 50, 3, 0, vec![Step::Evade])),
        ],
        back: [
            None,
            Some(unit(EnemyId::Besieger, 50, 5, 0, vec![Step::Fire])),
            None,
        ],
        ..Encounter::default()
    };
    let deck2 = vec![
        DeckCard::new(CardId::Barrage),
        DeckCard::new(CardId::OrbitalStrike),
    ];
    let mut b2 = battle_with(AdmiralId::Vance, deck2, enc2, vec![], 1);
    let evader_a = b2.enemies[0].uid;
    let guard_front = b2.enemies[1].uid;
    let evader_b = b2.enemies[2].uid;
    let covered_back = b2.enemies[3].uid;

    let barrage_idx = find_card(&b2, CardId::Barrage);
    assert_eq!(b2.card_cost(barrage_idx), 0);
    assert_eq!(b2.points, 3);
    play(&mut b2, CardId::Barrage, Target::None);
    assert_eq!(b2.points, 3);
    assert_eq!(b2.enemy(evader_a).unwrap().hull, 50 - (6 - 1));
    assert_eq!(b2.enemy(evader_b).unwrap().hull, 50 - 6);
    assert_eq!(b2.enemy(guard_front).unwrap().hull, 50);
    assert_eq!(b2.enemy(covered_back).unwrap().hull, 50);

    play(&mut b2, CardId::OrbitalStrike, Target::Enemy(covered_back));
    assert_eq!(b2.points, 0);
    assert_eq!(
        b2.enemy(guard_front).unwrap().hull,
        50,
        "OrbitalStrike Piercing must not be intercepted by Front cover"
    );
    assert_eq!(b2.enemy(covered_back).unwrap().hull, 50 - 16);
}

#[test]
fn t1_f1_existing_buffs_debuffs_tactics_doctrines_preserved() {
    let enc = Encounter {
        front: [
            Some(unit(
                EnemyId::GuardCruiser,
                60,
                8,
                0,
                vec![Step::Charge, Step::Fire],
            )),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::HoldTheLine),
        DeckCard::new(CardId::PowerSurge),
        DeckCard::new(CardId::TargetingJammer),
        DeckCard::new(CardId::FireControlDoctrine),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    b.points = 10;
    play(&mut b, CardId::EmergencyShields, Target::None);
    play(&mut b, CardId::HoldTheLine, Target::None);
    play(&mut b, CardId::PowerSurge, Target::None);
    play(&mut b, CardId::TargetingJammer, Target::None);
    play(&mut b, CardId::FireControlDoctrine, Target::None);

    assert_eq!(b.flagship.shield, 9);
    assert_eq!(b.flagship.hold_the_line, 2);
    assert_eq!(b.flagship.attack_bonus, 7);
    assert_eq!(b.flagship.fire_control, 1);
    assert_eq!(b.flagship.jammer, 3);
    assert_eq!(b.flagship_attack(), 8 + 7 + 1);

    // AblativeArmor, BattleStations, and EmpBlast behavior
    let enc2 = Encounter {
        front: [
            Some(unit(
                EnemyId::GuardCruiser,
                60,
                8,
                0,
                vec![Step::Fire, Step::Shields(6), Step::Fire],
            )),
            Some(unit(
                EnemyId::GuardCruiser,
                60,
                5,
                0,
                vec![Step::Charge, Step::Shields(6), Step::Fire],
            )),
            None,
        ],
        ..Encounter::default()
    };
    let deck2 = vec![
        DeckCard::new(CardId::AblativeArmor),
        DeckCard::new(CardId::BattleStations),
        DeckCard::new(CardId::EmpBlast),
        DeckCard::new(CardId::EmpBlast),
        DeckCard::new(CardId::EmpBlast),
    ];
    let mut b2 = battle_with(AdmiralId::Osei, deck2, enc2, vec![], 1);
    let charger_uid = b2.enemies[1].uid;

    play(&mut b2, CardId::AblativeArmor, Target::None);
    assert_eq!(b2.flagship.ablative, 1);
    assert_eq!(b2.flagship_armor(), 1);

    play(&mut b2, CardId::BattleStations, Target::None);
    assert_eq!(b2.flagship.battle_stations, 2);

    let hull_before = b2.flagship.hull;
    let ev1 = b2.end_turn();
    assert!(ev1.iter().any(|e| matches!(
        e,
        BattleEvent::Shielded {
            target: UnitRef::Flagship,
            amount: 2,
            ..
        }
    )));
    // Front[0] shoots for 8 -> minus 1 ablative armor = 7 -> minus 2 BattleStations shield = 5 hull loss
    assert_eq!(b2.flagship.hull, hull_before - 5);
    assert!(b2.enemy(charger_uid).unwrap().charged);

    // Round 2: enemies gain Shields(6) while Front[1] retains charge
    b2.end_turn();
    assert_eq!(b2.enemy(charger_uid).unwrap().shield, 6);
    assert!(b2.enemy(charger_uid).unwrap().charged);

    play(&mut b2, CardId::EmpBlast, Target::Enemy(charger_uid));
    assert_eq!(b2.enemy(charger_uid).unwrap().shield, 0);
    assert!(!b2.enemy(charger_uid).unwrap().charged);
}

#[test]
fn t1_f1_existing_cards_discarded_not_exhausted() {
    let deck = vec![
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::PowerSurge),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 2, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    play(&mut b, CardId::EmergencyShields, Target::None);
    play(&mut b, CardId::PowerSurge, Target::None);
    assert!(b.spent.is_empty());
    assert_eq!(b.discard_pile.len(), 3);
}

// --- F2: Pool Split & Data/Localization Integrity (6 tests) ---

#[test]
fn t1_f2_card_id_all_has_48_unique_cards() {
    assert_eq!(CardId::ALL.len(), 48);
    let unique: HashSet<CardId> = CardId::ALL.iter().copied().collect();
    assert_eq!(unique.len(), 48);
    let d = load_repo_data();
    assert_eq!(d.cards.len(), 48);
    d.check().expect("Data::check must pass");
}

#[test]
fn t1_f2_neutral_pool_has_28_unique_non_signature_cards() {
    let neutral = neutral_pool();
    assert_eq!(neutral.len(), 28);
    let neutral_set: HashSet<CardId> = neutral.iter().copied().collect();
    assert_eq!(neutral_set.len(), 28);
    for admiral in ADMIRAL_IDS {
        for &c in &admiral_pool(admiral) {
            assert!(
                !neutral_set.contains(&c),
                "admiral card {c:?} must not be in neutral_pool"
            );
        }
    }
}

#[test]
fn t1_f2_admiral_pools_5_each_disjoint() {
    let mut union: HashSet<CardId> = neutral_pool().into_iter().collect();
    let mut seen_admiral_cards = HashSet::new();
    for admiral in ADMIRAL_IDS {
        let pool = admiral_pool(admiral);
        assert_eq!(pool.len(), 5);
        assert!(pool.contains(&admiral.signature_card()));
        let pool_set: HashSet<CardId> = pool.iter().copied().collect();
        assert_eq!(pool_set.len(), 5);
        for c in pool {
            assert!(
                seen_admiral_cards.insert(c),
                "duplicate card {c:?} across admiral pools"
            );
            assert!(union.insert(c));
        }
    }
    assert_eq!(seen_admiral_cards.len(), 20);
    let all_set: HashSet<CardId> = CardId::ALL.iter().copied().collect();
    assert_eq!(union, all_set);
}

#[test]
fn t1_f2_all_48_cards_have_de_and_en_texts() {
    let d = load_repo_data();
    for id in CardId::ALL {
        let card = &d.cards[&id];
        for lang in ["de", "en"] {
            let txt = card
                .text
                .get(lang)
                .unwrap_or_else(|| panic!("{id:?} missing {lang} text"));
            assert!(!txt.name.trim().is_empty(), "{id:?} {lang} name empty");
            assert!(!txt.effect.trim().is_empty(), "{id:?} {lang} effect empty");
        }
    }
}

#[test]
fn t1_f2_all_22_new_cards_metadata_matches_spec() {
    struct CardContractSpec {
        id: CardId,
        card_type: CardType,
        target: CardTarget,
        has_extra: bool,
        exhaust: bool,
        retain: bool,
    }
    macro_rules! spec {
        ($id:expr, $ct:expr, $tgt:expr, $ex:expr, $exh:expr, $ret:expr) => {
            CardContractSpec {
                id: $id,
                card_type: $ct,
                target: $tgt,
                has_extra: $ex,
                exhaust: $exh,
                retain: $ret,
            }
        };
    }
    let d = load_repo_data();
    let expected: &[CardContractSpec] = &[
        spec!(
            CardId::ShieldBash,
            CardType::Attack,
            CardTarget::Enemy,
            false,
            false,
            false
        ),
        spec!(
            CardId::Bulwark,
            CardType::Buff,
            CardTarget::None,
            false,
            false,
            true
        ),
        spec!(
            CardId::FlakBreaker,
            CardType::Attack,
            CardTarget::Enemy,
            true,
            false,
            false
        ),
        spec!(
            CardId::BastionDoctrine,
            CardType::Doctrine,
            CardTarget::None,
            true,
            false,
            false
        ),
        spec!(
            CardId::Onslaught,
            CardType::Attack,
            CardTarget::Enemy,
            false,
            false,
            false
        ),
        spec!(
            CardId::BreakthroughVolley,
            CardType::Attack,
            CardTarget::Enemy,
            true,
            false,
            false
        ),
        spec!(
            CardId::ArmorPiercer,
            CardType::Attack,
            CardTarget::Enemy,
            true,
            false,
            false
        ),
        spec!(
            CardId::AssaultDoctrine,
            CardType::Doctrine,
            CardTarget::None,
            false,
            false,
            false
        ),
        spec!(
            CardId::TimeWindow,
            CardType::Tactic,
            CardTarget::None,
            false,
            false,
            true
        ),
        spec!(
            CardId::PrecisionLock,
            CardType::Debuff,
            CardTarget::Enemy,
            false,
            false,
            true
        ),
        spec!(
            CardId::ChronoStrike,
            CardType::Attack,
            CardTarget::Enemy,
            false,
            false,
            false
        ),
        spec!(
            CardId::ClockworkDoctrine,
            CardType::Doctrine,
            CardTarget::None,
            true,
            false,
            false
        ),
        spec!(
            CardId::Wildfire,
            CardType::Attack,
            CardTarget::None,
            false,
            false,
            false
        ),
        spec!(
            CardId::Accelerant,
            CardType::Debuff,
            CardTarget::Enemy,
            false,
            false,
            false
        ),
        spec!(
            CardId::MeltdownSalvo,
            CardType::Attack,
            CardTarget::Enemy,
            true,
            false,
            false
        ),
        spec!(
            CardId::EmberDoctrine,
            CardType::Doctrine,
            CardTarget::None,
            true,
            false,
            false
        ),
        spec!(
            CardId::EmergencyPower,
            CardType::Tactic,
            CardTarget::None,
            true,
            true,
            false
        ),
        spec!(
            CardId::AutocannonSalvo,
            CardType::Attack,
            CardTarget::Enemy,
            true,
            false,
            false
        ),
        spec!(
            CardId::TargetPainter,
            CardType::Debuff,
            CardTarget::Enemy,
            false,
            false,
            false
        ),
        spec!(
            CardId::StandbyFire,
            CardType::Attack,
            CardTarget::Enemy,
            false,
            false,
            true
        ),
        spec!(
            CardId::ChainCommand,
            CardType::Tactic,
            CardTarget::None,
            true,
            false,
            false
        ),
        spec!(
            CardId::BridgeCoordination,
            CardType::Doctrine,
            CardTarget::None,
            false,
            false,
            false
        ),
    ];
    assert_eq!(expected.len(), 22);
    for s in expected {
        let cd = &d.cards[&s.id];
        assert!(cd.check(s.id.has_extra()), "{:?} CardData::check", s.id);
        assert_eq!(cd.card_type, s.card_type, "{:?} card_type", s.id);
        assert_eq!(cd.target, s.target, "{:?} target", s.id);
        assert_eq!(s.id.has_extra(), s.has_extra, "{:?} has_extra", s.id);
        assert_eq!(cd.extra.is_some(), s.has_extra, "{:?} extra presence", s.id);
        assert_eq!(s.id.exhaust(), s.exhaust, "{:?} exhaust", s.id);
        assert_eq!(s.id.retain(), s.retain, "{:?} retain", s.id);
        assert!(cd.value >= 0, "{:?} non-negative value", s.id);
        assert!(cd.extra.unwrap_or(0) >= 0, "{:?} non-negative extra", s.id);
        assert!(
            cd.upgraded.cost.is_some()
                || cd.upgraded.value.is_some()
                || cd.upgraded.extra.is_some(),
            "{:?} must define an upgrade improvement",
            s.id
        );
        if let Some(uc) = cd.upgraded.cost {
            assert!(
                uc < cd.cost,
                "{:?} upgraded cost must strictly improve",
                s.id
            );
        }
        if let Some(uv) = cd.upgraded.value {
            assert!(
                uv > cd.value,
                "{:?} upgraded value must strictly improve",
                s.id
            );
        }
        if let Some(ue) = cd.upgraded.extra {
            assert!(
                ue > cd.extra.expect("extra must exist if upgraded.extra exists"),
                "{:?} upgraded extra must strictly improve",
                s.id
            );
        }
    }
}

#[test]
fn t1_f2_upgrade_extra_validated() {
    let d = load_repo_data();
    assert!(d.cards[&CardId::EmergencyPower].check(true));
    assert!(d.cards[&CardId::MeltdownSalvo].check(true));
    assert_eq!(
        CardId::EmergencyPower.extra(false),
        d.cards[&CardId::EmergencyPower].extra.unwrap()
    );
    assert_eq!(
        CardId::EmergencyPower.extra(true),
        d.cards[&CardId::EmergencyPower]
            .upgraded
            .extra
            .unwrap_or_else(|| d.cards[&CardId::EmergencyPower].extra.unwrap())
    );
    assert_eq!(
        CardId::MeltdownSalvo.extra(false),
        d.cards[&CardId::MeltdownSalvo].extra.unwrap()
    );
    assert_eq!(
        CardId::MeltdownSalvo.extra(true),
        d.cards[&CardId::MeltdownSalvo]
            .upgraded
            .extra
            .unwrap_or_else(|| d.cards[&CardId::MeltdownSalvo].extra.unwrap())
    );

    let mut invalid = d.cards[&CardId::MeltdownSalvo].clone();
    invalid.upgraded.extra = Some(-1);
    assert!(!invalid.check(true));

    let mut stray_extra = d.cards[&CardId::Torpedo].clone();
    stray_extra.upgraded.extra = Some(1);
    assert!(!stray_extra.check(false));
}

// --- F3: Loot & Market Guaranteed Admiral Slot (6 tests) ---

#[test]
fn t1_f3_loot_offer_contains_at_least_one_admiral_card_all_admirals() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        for seed in 1..=50 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.advance(0).unwrap();
            let mut battle = run.start_battle();
            battle.result = Some(BattleResult::Won);
            let loot = run.finish_battle(&battle);
            assert_eq!(loot.cards.len(), 3);
            assert!(
                loot.cards.iter().any(|c| pool.contains(c)),
                "{admiral:?} seed {seed} loot missing admiral slot: {:?}",
                loot.cards
            );
        }
    }
}

#[test]
fn t1_f3_loot_offer_3_distinct_cards_in_admiral_plus_neutral() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let allowed: HashSet<CardId> = admiral_pool(admiral)
            .into_iter()
            .chain(neutral_pool())
            .collect();
        for seed in 100..125 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.advance(0).unwrap();
            let mut battle = run.start_battle();
            battle.result = Some(BattleResult::Won);
            let loot = run.finish_battle(&battle);
            let unique: HashSet<CardId> = loot.cards.iter().copied().collect();
            assert_eq!(unique.len(), 3);
            assert!(unique.is_subset(&allowed));
        }
    }
}

#[test]
fn t1_f3_loot_offer_4_distinct_cards_with_combat_simulator() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let allowed: HashSet<CardId> = admiral_pool(admiral)
            .into_iter()
            .chain(neutral_pool())
            .collect();
        for seed in 200..215 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.modules.push(ModuleId::CombatSimulator);
            run.advance(0).unwrap();
            let mut battle = run.start_battle();
            battle.result = Some(BattleResult::Won);
            let loot = run.finish_battle(&battle);
            assert_eq!(loot.cards.len(), 4);
            let unique: HashSet<CardId> = loot.cards.iter().copied().collect();
            assert_eq!(unique.len(), 4);
            assert!(unique.is_subset(&allowed));
            assert!(loot.cards.iter().any(|c| pool.contains(c)));
        }
    }
}

#[test]
fn t1_f3_market_offer_contains_at_least_one_admiral_card_all_admirals() {
    let maps = synthetic_battle_maps(NodeKind::Market);
    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let allowed: HashSet<CardId> = admiral_pool(admiral)
            .into_iter()
            .chain(neutral_pool())
            .collect();
        for seed in 300..320 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.advance(0).unwrap();
            let Stage::Market(market) = &run.stage else {
                panic!("expected market stage");
            };
            assert_eq!(market.cards.len(), 3);
            let ids: Vec<CardId> = market.cards.iter().map(|&(c, _)| c).collect();
            let unique: HashSet<CardId> = ids.iter().copied().collect();
            assert_eq!(unique.len(), 3);
            assert!(unique.is_subset(&allowed));
            assert!(ids.iter().any(|c| pool.contains(c)));
        }
    }
}

#[test]
fn t1_f3_signature_cards_appear_in_own_admiral_offers_never_others() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let sig = admiral.signature_card();
        let other_sigs: HashSet<CardId> = ADMIRAL_IDS
            .into_iter()
            .filter(|&a| a != admiral)
            .map(AdmiralId::signature_card)
            .collect();
        let run = Run::new(admiral, 1, maps.clone());
        let mut rng = StdRng::seed_from_u64(42);
        let mut saw_own_sig = false;
        for _ in 0..100 {
            let offer = run.offer_cards(&mut rng, 3);
            if offer.contains(&sig) {
                saw_own_sig = true;
            }
            for card in offer {
                assert!(
                    !other_sigs.contains(&card),
                    "leaked other signature {card:?}"
                );
            }
        }
        assert!(
            saw_own_sig,
            "{admiral:?} never offered own signature {sig:?}"
        );
    }
}

#[test]
fn t1_f3_market_purchase_and_loot_take_adds_card_to_deck() {
    let mut run_loot = Run::new(AdmiralId::Osei, 7, synthetic_battle_maps(NodeKind::Battle));
    run_loot.advance(0).unwrap();
    let mut battle = run_loot.start_battle();
    battle.result = Some(BattleResult::Won);
    let loot = run_loot.finish_battle(&battle);
    let chosen_loot = loot.cards[1];
    run_loot.take_loot(Some(1)).unwrap();
    assert_eq!(run_loot.deck.len(), 11);
    assert_eq!(run_loot.deck.last().unwrap().id, chosen_loot);

    let mut run_mkt = Run::new(AdmiralId::Kerr, 9, synthetic_battle_maps(NodeKind::Market));
    run_mkt.scrap = 500;
    run_mkt.advance(0).unwrap();
    let bought = match &run_mkt.stage {
        Stage::Market(m) => m.cards[0].0,
        _ => panic!("expected market"),
    };
    run_mkt.buy_card(0).unwrap();
    assert_eq!(run_mkt.deck.len(), 11);
    assert_eq!(run_mkt.deck.last().unwrap().id, bought);
}

// --- F4: Keyword Einmalig (Exhaust) (5 tests) ---

#[test]
fn t1_f4_micro_jump_moves_to_spent_not_discard() {
    let deck = vec![DeckCard::new(CardId::MicroJump)];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 2, 0)),
        vec![],
        1,
    );
    play(&mut b, CardId::MicroJump, Target::None);
    assert_eq!(b.spent, vec![DeckCard::new(CardId::MicroJump)]);
    assert!(b.discard_pile.is_empty());
}

#[test]
fn t1_f4_emergency_power_moves_to_spent_not_discard() {
    let deck = vec![DeckCard::new(CardId::EmergencyPower)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 2, 0)),
        vec![],
        1,
    );
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.spent, vec![DeckCard::new(CardId::EmergencyPower)]);
    assert!(b.discard_pile.is_empty());
}

#[test]
fn t1_f4_exhausted_card_not_redrawn_on_reshuffle() {
    let deck = vec![
        DeckCard::new(CardId::EmergencyPower),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        &[CardId::EmergencyPower],
    );
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.spent.len(), 1);
    for _ in 0..4 {
        b.end_turn();
        assert!(!b.hand.iter().any(|c| c.id == CardId::EmergencyPower));
        assert!(!b.draw_pile.iter().any(|c| c.id == CardId::EmergencyPower));
        assert!(
            !b.discard_pile
                .iter()
                .any(|c| c.id == CardId::EmergencyPower)
        );
    }
}

#[test]
fn t1_f4_upgraded_exhaust_cards_still_exhaust() {
    let deck = vec![
        DeckCard {
            id: CardId::MicroJump,
            upgraded: true,
        },
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    play_upg(&mut b, CardId::EmergencyPower, true, Target::None);
    play_upg(&mut b, CardId::MicroJump, true, Target::None);
    assert_eq!(b.spent.len(), 2);
    assert!(b.discard_pile.is_empty());
}

fn half_up(value: i32) -> i32 {
    (value + 1) / 2
}

#[test]
fn t1_f4_two_copies_of_exhaust_card_exhaust_independently() {
    let deck = vec![
        DeckCard::new(CardId::EmergencyPower),
        DeckCard::new(CardId::EmergencyPower),
    ];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let gain = CardId::EmergencyPower.value(false) as u32;
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.spent.len(), 1);
    assert_eq!(b.points, 3 + gain);
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.spent.len(), 2);
    assert_eq!(b.points, 3 + 2 * gain);
}

// --- F5: Keyword Behalten (Retain) (5 tests) ---

#[test]
fn t1_f5_bulwark_retained_on_end_turn() {
    let deck = vec![DeckCard::new(CardId::Bulwark)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    assert!(!b.is_retained(0));
    b.end_turn();
    assert_eq!(b.hand, vec![DeckCard::new(CardId::Bulwark)]);
    assert!(b.is_retained(0));
    assert!(b.discard_pile.is_empty());
}

#[test]
fn t1_f5_time_window_retained_on_end_turn() {
    let deck = vec![DeckCard::new(CardId::TimeWindow)];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert_eq!(b.hand, vec![DeckCard::new(CardId::TimeWindow)]);
    assert!(b.is_retained(0));
}

#[test]
fn t1_f5_precision_lock_retained_on_end_turn() {
    let deck = vec![DeckCard::new(CardId::PrecisionLock)];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert_eq!(b.hand, vec![DeckCard::new(CardId::PrecisionLock)]);
    assert!(b.is_retained(0));
}

#[test]
fn t1_f5_standby_fire_retained_on_end_turn() {
    let deck = vec![DeckCard::new(CardId::StandbyFire)];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert_eq!(b.hand, vec![DeckCard::new(CardId::StandbyFire)]);
    assert!(b.is_retained(0));
}

#[test]
fn t1_f5_non_retain_cards_discarded_while_retain_cards_kept() {
    let deck = vec![
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::Scavenge),
        DeckCard::new(CardId::RepairCrew),
        DeckCard::new(CardId::PowerSurge),
        DeckCard::new(CardId::AblativeArmor),
        DeckCard::new(CardId::VolleyFire),
        DeckCard::new(CardId::Countermeasures),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::Bulwark, CardId::StandbyFire],
    );
    b.end_turn();
    assert!(b.hand.iter().any(|c| c.id == CardId::Bulwark));
    assert!(b.hand.iter().any(|c| c.id == CardId::StandbyFire));
    for (i, card) in b.hand.iter().enumerate() {
        assert_eq!(
            b.is_retained(i),
            card.id.retain(),
            "retained flag mismatch at {i} for {:?}",
            card.id
        );
    }
    assert_eq!(b.hand.len(), 7);
    assert_eq!(b.discard_pile.len(), 3);
}

// --- F6: Keyword Markiert (Marked) (6 tests) ---

#[test]
fn t1_f6_marked_adds_50_percent_rounded_up_to_flagship_shot() {
    // Osei (8 -> 12), Kerr (9 -> 14), Reyes (10 -> 15), Vance (16 -> 24)
    for (admiral, expected) in [
        (AdmiralId::Osei, 12),
        (AdmiralId::Kerr, 14),
        (AdmiralId::Reyes, 15),
        (AdmiralId::Vance, 24),
    ] {
        let deck = vec![DeckCard::new(CardId::PrecisionLock)];
        let mut b = battle_with(
            admiral,
            deck,
            encounter_single(dummy_front(100, 1, 0)),
            vec![],
            1,
        );
        let uid = b.enemies[0].uid;
        b.points = 10;
        play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
        let events = b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
        let bds = hit_breakdowns(&events, uid);
        assert_eq!(bds.len(), 1);
        assert_eq!(bds[0].hull, expected, "{admiral:?} marked shot");
        assert_eq!(bds[0].marked, expected - admiral.attack());
    }
}

#[test]
fn t1_f6_marked_applies_to_attack_cards() {
    let deck = vec![
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::VolleyFire),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let ev1 = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev1, uid)[0].hull, 9 + 5);
    let ev2 = play(&mut b, CardId::VolleyFire, Target::None);
    assert_eq!(hit_breakdowns(&ev2, uid)[0].hull, 3 + 2);
}

#[test]
fn t1_f6_marked_clears_in_resolution() {
    let deck = vec![DeckCard::new(CardId::PrecisionLock)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().marked);
    b.end_turn();
    assert!(!b.enemy(uid).unwrap().marked);
    let events = b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(hit_breakdowns(&events, uid)[0].marked, 0);
    assert_eq!(hit_breakdowns(&events, uid)[0].hull, 8);
}

#[test]
fn t1_f6_marked_applies_per_hit_on_multihit() {
    let deck = vec![
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let ev = play(&mut b, CardId::AutocannonSalvo, Target::Enemy(uid));
    let base = CardId::AutocannonSalvo.value(false);
    let hits = CardId::AutocannonSalvo.extra(false);
    let m = half_up(base);
    let bds = hit_breakdowns(&ev, uid);
    assert_eq!(bds.len(), hits as usize);
    for bd in bds {
        assert_eq!(bd.base, base);
        assert_eq!(bd.marked, m);
        assert_eq!(bd.hull, base + m);
    }
    assert_eq!(b.enemy(uid).unwrap().hull, 100 - hits * (base + m));
}

#[test]
fn t1_f6_marked_preview_matches_actual_hit() {
    let deck = vec![
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let st_preview = b
        .preview_station(Station::Weapons, Target::Enemy(uid))
        .unwrap();
    let idx = find_card(&b, CardId::Torpedo);
    assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    let st_events = b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    let st_actual: Vec<Hit> = st_events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Hit {
                target, breakdown, ..
            } => Some(Hit {
                target: *target,
                breakdown: *breakdown,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(st_preview, st_actual);
}

#[test]
fn t1_f6_marked_applied_by_all_four_markers() {
    for marker in [
        CardId::ArmorPiercer,
        CardId::PrecisionLock,
        CardId::Accelerant,
        CardId::TargetPainter,
    ] {
        let mut b = battle_with(
            AdmiralId::Osei,
            vec![DeckCard::new(marker)],
            encounter_single(dummy_front(50, 1, 0)),
            vec![],
            1,
        );
        let uid = b.enemies[0].uid;
        b.points = 10;
        play(&mut b, marker, Target::Enemy(uid));
        assert!(b.enemy(uid).unwrap().marked, "{marker:?} must mark target");
    }
}

// --- F7: Keyword Panzerbruch (Armor Break) (5 tests) ---

#[test]
fn t1_f7_target_painter_reduces_armor_by_1() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TargetPainter)],
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(
        b.enemy(uid).unwrap().armor,
        2 - CardId::TargetPainter.value(false)
    );
}

#[test]
fn t1_f7_flak_breaker_reduces_armor_by_1() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::FlakBreaker)],
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::FlakBreaker.value(false);
    let lost = 2 * (v - 2).max(0);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(
        b.enemy(uid).unwrap().armor,
        2 - CardId::FlakBreaker.extra(false)
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - lost);
}

#[test]
fn t1_f7_armor_piercer_reduces_armor_by_1() {
    let mut b = battle_with(
        AdmiralId::Vance,
        vec![DeckCard::new(CardId::ArmorPiercer)],
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::ArmorPiercer.value(false);
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(uid));
    assert_eq!(
        b.enemy(uid).unwrap().armor,
        2 - CardId::ArmorPiercer.extra(false)
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - (v - 2).max(0));
}

#[test]
fn t1_f7_armor_break_persists_across_rounds() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TargetPainter)],
        encounter_single(dummy_front(50, 1, 3)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let expected = 3 - CardId::TargetPainter.value(false);
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, expected);
    b.end_turn();
    assert_eq!(b.enemy(uid).unwrap().armor, expected);
    b.end_turn();
    assert_eq!(b.enemy(uid).unwrap().armor, expected);
}

#[test]
fn t1_f7_armor_break_stacks_from_multiple_cards() {
    let deck = vec![
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::FlakBreaker),
        DeckCard::new(CardId::ArmorPiercer),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 3)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 2);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 1);
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

// --- F8: Zeitfenster (double_next) (6 tests) ---

#[test]
fn t1_f8_time_window_doubles_attack_card() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.flagship.double_next, 1);
    let ev = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 2);
    assert_eq!(b.enemy(uid).unwrap().hull, 32);
    assert_eq!(b.flagship.double_next, 0);
}

#[test]
fn t1_f8_time_window_doubles_buff_and_tactic() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Scavenge),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[
            CardId::TimeWindow,
            CardId::EmergencyShields,
            CardId::Scavenge,
        ],
    );
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert_eq!(b.flagship.shield, 10);

    let hand_before = b.hand.len();
    if b.hand.iter().any(|c| c.id == CardId::TimeWindow) {
        play(&mut b, CardId::TimeWindow, Target::None);
        play(&mut b, CardId::Scavenge, Target::None);
        assert_eq!(b.hand.len(), hand_before - 2 + 6);
    } else {
        let deck2 = vec![
            DeckCard::new(CardId::TimeWindow),
            DeckCard::new(CardId::Scavenge),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
            DeckCard::new(CardId::Torpedo),
        ];
        let mut b2 = battle_requiring_hand(
            AdmiralId::Kerr,
            deck2,
            encounter_single(dummy_front(50, 1, 0)),
            vec![],
            &[CardId::TimeWindow, CardId::Scavenge],
        );
        b2.points = 10;
        let before = b2.hand.len();
        play(&mut b2, CardId::TimeWindow, Target::None);
        play(&mut b2, CardId::Scavenge, Target::None);
        assert_eq!(b2.hand.len(), before - 2 + 6);
    }
}

#[test]
fn t1_f8_time_window_doubles_debuff() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::TargetingJammer),
        DeckCard::new(CardId::TargetPainter),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 8, 2)),
        vec![],
        1,
    );
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::TargetingJammer, Target::None);
    assert_eq!(b.flagship.jammer, 6);

    let deck2 = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::TargetPainter),
    ];
    let mut b2 = battle_with(
        AdmiralId::Kerr,
        deck2,
        encounter_single(dummy_front(50, 8, 2)),
        vec![],
        1,
    );
    let uid2 = b2.enemies[0].uid;
    b2.points = 10;
    play(&mut b2, CardId::TimeWindow, Target::None);
    play(&mut b2, CardId::TargetPainter, Target::Enemy(uid2));
    assert_eq!(b2.enemy(uid2).unwrap().armor, 0);
}

#[test]
fn t1_f8_time_window_costs_bp_only_once_for_doubled_card() {
    let tw_cost = CardId::TimeWindow.cost(false);
    let ep_gain = CardId::EmergencyPower.value(false) as u32;
    let deck = vec![
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::TimeWindow],
    );
    let uid = b.enemies[0].uid;
    assert_eq!(b.points, 3);
    for _ in 0..4 {
        play_upg(&mut b, CardId::EmergencyPower, true, Target::None);
    }
    let expected_pre_tw = 3 + 4 * ep_gain;
    assert_eq!(b.points, expected_pre_tw);
    if b.points < tw_cost + 4 {
        b.points = tw_cost + 4;
    }
    let pre_tw = b.points;
    play(&mut b, CardId::TimeWindow, Target::None);
    let after_tw = pre_tw - tw_cost;
    assert_eq!(b.points, after_tw);
    assert!(after_tw >= 4, "must hold >= 2x Torpedo cost to catch M5");
    play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(
        b.points,
        after_tw - 2,
        "doubled Torpedo must deduct 2 BP once, never twice"
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 18);
}

#[test]
fn t1_f8_time_window_does_not_affect_stations() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    let st_ev = b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(hit_breakdowns(&st_ev, uid).len(), 1);
    assert_eq!(b.flagship.double_next, 1);
    let card_ev = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&card_ev, uid).len(), 2);
}

#[test]
fn t1_f8_time_window_expires_at_end_of_turn() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.flagship.double_next, 1);
    b.end_turn();
    assert_eq!(b.flagship.double_next, 0);
    let ev = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 1);
}

// --- F9: Osei's 4 New Cards (Base & Upgraded) (8 tests) ---

#[test]
fn t1_f9_shield_bash_base_hits_for_flagship_shield_homing() {
    let deck = vec![DeckCard::new(CardId::ShieldBash)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let cost = CardId::ShieldBash.cost(false);
    b.points = cost.max(3);
    let start_pts = b.points;
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 11);
    let idx = find_card(&b, CardId::ShieldBash);
    assert_eq!(b.card_cost(idx), cost);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid)[0].hull, 11);
    assert_eq!(b.points, start_pts - cost);
}

#[test]
fn t1_f9_shield_bash_upgraded_costs_0_bp() {
    let deck = vec![DeckCard {
        id: CardId::ShieldBash,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let upg_cost = CardId::ShieldBash.cost(true);
    assert!(upg_cost < CardId::ShieldBash.cost(false));
    b.run_station(Station::Shields, Target::None).unwrap();
    b.points = upg_cost;
    let idx = find_card(&b, CardId::ShieldBash);
    assert_eq!(b.card_cost(idx), upg_cost);
    play_upg(&mut b, CardId::ShieldBash, true, Target::Enemy(uid));
    assert_eq!(b.points, 0);
    assert_eq!(b.enemy(uid).unwrap().hull, 39);
}

#[test]
fn t1_f9_bulwark_base_grants_12_shields_and_retains() {
    let deck = vec![DeckCard::new(CardId::Bulwark)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let cost = CardId::Bulwark.cost(false);
    let val = CardId::Bulwark.value(false);
    b.end_turn();
    assert!(b.is_retained(0));
    b.points = cost + 1;
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(b.flagship.shield, val);
    assert_eq!(b.points, 1);
}

#[test]
fn t1_f9_bulwark_upgraded_grants_16_shields() {
    let deck = vec![DeckCard {
        id: CardId::Bulwark,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let val = CardId::Bulwark.value(true);
    assert!(val > CardId::Bulwark.value(false));
    b.points = CardId::Bulwark.cost(true);
    play_upg(&mut b, CardId::Bulwark, true, Target::None);
    assert_eq!(b.flagship.shield, val);
}

#[test]
fn t1_f9_flak_breaker_base_deals_2x4_and_breaks_armor() {
    let deck = vec![DeckCard::new(CardId::FlakBreaker)];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::FlakBreaker.value(false);
    let lost = 2 * (v - 2).max(0);
    let ev = play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 2);
    assert_eq!(
        b.enemy(uid).unwrap().armor,
        2 - CardId::FlakBreaker.extra(false)
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - lost);
}

#[test]
fn t1_f9_flak_breaker_upgraded_deals_2x6_and_breaks_armor() {
    let deck = vec![DeckCard {
        id: CardId::FlakBreaker,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::FlakBreaker.value(true);
    assert!(v > CardId::FlakBreaker.value(false));
    let lost = 2 * (v - 2).max(0);
    let ev = play_upg(&mut b, CardId::FlakBreaker, true, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 2);
    assert_eq!(
        b.enemy(uid).unwrap().armor,
        2 - CardId::FlakBreaker.extra(true)
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - lost);
}

#[test]
fn t1_f9_bastion_doctrine_base_grants_bonus_per_shield_card() {
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let cost = CardId::BastionDoctrine.cost(false);
    let bs = CardId::BastionDoctrine.value(false);
    let bc = CardId::BastionDoctrine.extra(false);
    b.points = cost + 2;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    assert_eq!(b.points, 2);
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert_eq!(b.flagship.shield, 5 + bs);
    assert_eq!(b.flagship.hold_the_line, bc);
}

#[test]
fn t1_f9_bastion_doctrine_upgraded_costs_1bp() {
    let deck = vec![
        DeckCard {
            id: CardId::BastionDoctrine,
            upgraded: true,
        },
        DeckCard::new(CardId::Bulwark),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let upg_cost = CardId::BastionDoctrine.cost(true);
    assert!(upg_cost < CardId::BastionDoctrine.cost(false));
    b.points = 10;
    play_upg(&mut b, CardId::BastionDoctrine, true, Target::None);
    assert_eq!(b.points, 10 - upg_cost);
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(
        b.flagship.shield,
        CardId::Bulwark.value(false) + CardId::BastionDoctrine.value(true)
    );
    assert_eq!(
        b.flagship.hold_the_line,
        CardId::BastionDoctrine.extra(true)
    );
}

// --- F10: Vance's 4 New Cards (Base & Upgraded) (8 tests) ---

#[test]
fn t1_f10_onslaught_base_uses_double_momentum_and_piercing() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [Some(dummy_front(50, 1, 0)), None, None],
        ..Encounter::default()
    };
    let deck = vec![DeckCard::new(CardId::Onslaught)];
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    b.flagship.momentum = 3;
    let dmg = CardId::Onslaught.value(false) + 2 * 3;
    let ev = play(&mut b, CardId::Onslaught, Target::Enemy(back_uid));
    assert_eq!(hit_breakdowns(&ev, back_uid)[0].hull, dmg);
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - dmg);
}

#[test]
fn t1_f10_onslaught_upgraded_deals_10_plus_double_momentum() {
    let deck = vec![DeckCard {
        id: CardId::Onslaught,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.flagship.momentum = 2;
    let dmg = CardId::Onslaught.value(true) + 2 * 2;
    assert!(CardId::Onslaught.value(true) > CardId::Onslaught.value(false));
    play_upg(&mut b, CardId::Onslaught, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - dmg);
}

#[test]
fn t1_f10_breakthrough_volley_base_kills_for_2bp_and_draw_1() {
    let v = CardId::BreakthroughVolley.value(false);
    let cost = CardId::BreakthroughVolley.cost(false);
    let refund = CardId::BreakthroughVolley.extra(false) as u32;
    let deck = vec![
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Overcharge),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(v, 1, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        enc,
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let target = b.enemies[0].uid;
    b.points = cost + 1;
    play(&mut b, CardId::BreakthroughVolley, Target::Enemy(target));
    assert!(b.enemy(target).is_none());
    assert_eq!(b.points, 1 + refund);
    assert_eq!(b.hand.len(), 5);
}

#[test]
fn t1_f10_breakthrough_volley_upgraded_deals_15_and_refunds_on_kill() {
    let v = CardId::BreakthroughVolley.value(true);
    assert!(v > CardId::BreakthroughVolley.value(false));
    let cost = CardId::BreakthroughVolley.cost(true);
    let refund = CardId::BreakthroughVolley.extra(true) as u32;
    let deck = vec![
        DeckCard {
            id: CardId::BreakthroughVolley,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Overcharge),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(v, 1, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        enc,
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let target = b.enemies[0].uid;
    b.points = cost + 1;
    play_upg(
        &mut b,
        CardId::BreakthroughVolley,
        true,
        Target::Enemy(target),
    );
    assert!(b.enemy(target).is_none());
    assert_eq!(b.points, 1 + refund);
    assert_eq!(b.hand.len(), 5);
}

#[test]
fn t1_f10_armor_piercer_base_deals_6_piercing_marks_and_breaks_armor() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [Some(dummy_front(50, 1, 2)), None, None],
        ..Encounter::default()
    };
    let deck = vec![DeckCard::new(CardId::ArmorPiercer)];
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    let dmg = (CardId::ArmorPiercer.value(false) - 2).max(0);
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(back_uid));
    let back = b.enemy(back_uid).unwrap();
    assert_eq!(back.hull, 50 - dmg);
    assert!(back.marked);
    assert_eq!(back.armor, 2 - CardId::ArmorPiercer.extra(false));
}

#[test]
fn t1_f10_armor_piercer_upgraded_deals_9_piercing_marks_and_breaks_armor() {
    let deck = vec![DeckCard {
        id: CardId::ArmorPiercer,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::ArmorPiercer.value(true);
    assert!(v > CardId::ArmorPiercer.value(false));
    let dmg = (v - 2).max(0);
    play_upg(&mut b, CardId::ArmorPiercer, true, Target::Enemy(uid));
    let e = b.enemy(uid).unwrap();
    assert_eq!(e.hull, 50 - dmg);
    assert!(e.marked);
    assert_eq!(e.armor, 2 - CardId::ArmorPiercer.extra(true));
}

#[test]
fn t1_f10_assault_doctrine_base_grants_5_shields_on_weapons_and_focus_fire() {
    let deck = vec![DeckCard::new(CardId::AssaultDoctrine)];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let s = CardId::AssaultDoctrine.value(false);
    b.points = CardId::AssaultDoctrine.cost(false);
    play(&mut b, CardId::AssaultDoctrine, Target::None);
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
    b.end_turn();
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
}

#[test]
fn t1_f10_assault_doctrine_upgraded_grants_8_shields() {
    let deck = vec![DeckCard {
        id: CardId::AssaultDoctrine,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(100, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let s = CardId::AssaultDoctrine.value(true);
    assert!(s > CardId::AssaultDoctrine.value(false));
    b.points = CardId::AssaultDoctrine.cost(true);
    play_upg(&mut b, CardId::AssaultDoctrine, true, Target::None);
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
}

// --- F11: Kerr's 4 New Cards (Base & Upgraded) (8 tests) ---

#[test]
fn t1_f11_time_window_base_and_upgraded_costs() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard {
            id: CardId::TimeWindow,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let base_cost = CardId::TimeWindow.cost(false);
    let upg_cost = CardId::TimeWindow.cost(true);
    assert!(upg_cost < base_cost);
    let base_idx = find_card_upg(&b, CardId::TimeWindow, false);
    assert_eq!(b.card_cost(base_idx), base_cost);
    let upg_idx = find_card_upg(&b, CardId::TimeWindow, true);
    assert_eq!(b.card_cost(upg_idx), upg_cost);
    b.points = upg_cost + 2;
    play_upg(&mut b, CardId::TimeWindow, true, Target::None);
    assert_eq!(b.points, 2);
    play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 32);
}

#[test]
fn t1_f11_precision_lock_base_marks_draws_1_and_retains() {
    let draw = CardId::PrecisionLock.value(false) as usize;
    let cost = CardId::PrecisionLock.cost(false);
    let deck = vec![
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::RepairCrew),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::PrecisionLock],
    );
    let uid = b.enemies[0].uid;
    b.points = cost + 1;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.points, 1);
    assert_eq!(b.hand.len(), 5 - 1 + draw);
}

#[test]
fn t1_f11_precision_lock_upgraded_draws_2() {
    let draw = CardId::PrecisionLock.value(true) as usize;
    let cost = CardId::PrecisionLock.cost(true);
    let deck = vec![
        DeckCard {
            id: CardId::PrecisionLock,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::RepairCrew),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::PrecisionLock],
    );
    let uid = b.enemies[0].uid;
    b.points = cost + 1;
    play_upg(&mut b, CardId::PrecisionLock, true, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.points, 1);
    assert_eq!(b.hand.len(), 5 - 1 + draw);
}

#[test]
fn t1_f11_chrono_strike_base_10_normal_20_stasis_or_charging() {
    let enc = Encounter {
        front: [
            Some(unit(EnemyId::Raiders, 50, 2, 0, vec![Step::Fire])),
            None,
            None,
        ],
        back: [
            Some(unit(
                EnemyId::GuardCruiser,
                50,
                2,
                0,
                vec![Step::Charge, Step::Fire],
            )),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::ChronoStrike),
        DeckCard::new(CardId::ChronoStrike),
    ];
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 1);
    let normal_uid = b.enemies[0].uid;
    let charging_uid = b.enemies[1].uid;
    let v = CardId::ChronoStrike.value(false);
    b.points = 10;
    play(&mut b, CardId::ChronoStrike, Target::Enemy(normal_uid));
    assert_eq!(b.enemy(normal_uid).unwrap().hull, 50 - v);
    play(&mut b, CardId::ChronoStrike, Target::Enemy(charging_uid));
    assert_eq!(b.enemy(charging_uid).unwrap().hull, 50 - 2 * v);
}

#[test]
fn t1_f11_chrono_strike_upgraded_13_normal_26_stasis() {
    let deck = vec![
        DeckCard {
            id: CardId::ChronoStrike,
            upgraded: true,
        },
        DeckCard {
            id: CardId::ChronoStrike,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(60, 2, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::ChronoStrike.value(true);
    assert!(v > CardId::ChronoStrike.value(false));
    b.points = 10;
    play_upg(&mut b, CardId::ChronoStrike, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 60 - v);
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    play_upg(&mut b, CardId::ChronoStrike, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 60 - v - 2 * v);
}

#[test]
fn t1_f11_clockwork_doctrine_base_draws_plus1_and_discounts_first_tactic_debuff() {
    let cost = CardId::ClockworkDoctrine.cost(false);
    let extra_draw = CardId::ClockworkDoctrine.value(false) as usize;
    let discount = CardId::ClockworkDoctrine.extra(false) as u32;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::Scavenge),
        DeckCard::new(CardId::TargetingJammer),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::ClockworkDoctrine],
    );
    b.points = cost + 1;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    assert_eq!(b.points, 1);
    b.end_turn();
    // 6 cards remain after ClockworkDoctrine enters doctrines; draw 5 + extra_draw draws all of them
    assert_eq!(b.hand.len(), (5 + extra_draw).min(6));
    let scav_idx = find_card(&b, CardId::Scavenge);
    let jam_idx = find_card(&b, CardId::TargetingJammer);
    assert_eq!(b.card_cost(scav_idx), 1u32.saturating_sub(discount));
    assert_eq!(b.card_cost(jam_idx), 1u32.saturating_sub(discount));
    play(&mut b, CardId::Scavenge, Target::None);
    assert_eq!(b.points, 3);
    let jam_idx_after = find_card(&b, CardId::TargetingJammer);
    assert_eq!(b.card_cost(jam_idx_after), 1);
}

#[test]
fn t1_f11_clockwork_doctrine_upgraded_costs_1bp() {
    let upg_cost = CardId::ClockworkDoctrine.cost(true);
    let discount = CardId::ClockworkDoctrine.extra(true) as u32;
    assert!(upg_cost < CardId::ClockworkDoctrine.cost(false));
    let deck = vec![
        DeckCard {
            id: CardId::ClockworkDoctrine,
            upgraded: true,
        },
        DeckCard::new(CardId::RadioSilence),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = upg_cost + (3 - discount);
    play_upg(&mut b, CardId::ClockworkDoctrine, true, Target::None);
    assert_eq!(b.points, 3 - discount);
    let rs_idx = find_card(&b, CardId::RadioSilence);
    assert_eq!(b.card_cost(rs_idx), 3 - discount);
    play(&mut b, CardId::RadioSilence, Target::Enemy(uid));
    assert_eq!(b.points, 0);
    assert!(b.enemy(uid).unwrap().silenced);
}

#[test]
fn t1_f11_clockwork_doctrine_persists_across_rounds_for_draw_and_discount() {
    let extra_draw = CardId::ClockworkDoctrine.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::ClockworkDoctrine],
    );
    b.points = CardId::ClockworkDoctrine.cost(false);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    b.end_turn();
    assert_eq!(b.hand.len(), 5 + extra_draw);
    b.end_turn();
    assert_eq!(b.hand.len(), 5 + extra_draw);
}

// --- F12: Reyes's 4 New Cards (Base & Upgraded) (8 tests) ---

#[test]
fn t1_f12_wildfire_base_hits_all_piercing_and_ignites() {
    let enc = Encounter {
        front: [Some(dummy_front(40, 1, 0)), None, None],
        back: [Some(dummy_front(40, 1, 0)), None, None],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Reyes,
        vec![DeckCard::new(CardId::Wildfire)],
        enc,
        vec![],
        1,
    );
    let dmg = CardId::Wildfire.value(false);
    play(&mut b, CardId::Wildfire, Target::None);
    for e in &b.enemies {
        assert_eq!(e.hull, 40 - dmg);
        assert!(e.burning);
    }
}

#[test]
fn t1_f12_wildfire_upgraded_deals_5_and_ignites() {
    let mut b = battle_with(
        AdmiralId::Reyes,
        vec![DeckCard {
            id: CardId::Wildfire,
            upgraded: true,
        }],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let dmg = CardId::Wildfire.value(true);
    assert!(dmg > CardId::Wildfire.value(false));
    play_upg(&mut b, CardId::Wildfire, true, Target::None);
    assert_eq!(b.enemies[0].hull, 40 - dmg);
    assert!(b.enemies[0].burning);
}

#[test]
fn t1_f12_accelerant_base_deals_2x_burn_and_marks() {
    let deck = vec![
        DeckCard::new(CardId::Firebomb),
        DeckCard::new(CardId::Accelerant),
    ];
    let mut b = battle_with(
        AdmiralId::Reyes,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let cost = CardId::Accelerant.cost(false);
    let mult = CardId::Accelerant.value(false);
    b.points = 1 + cost + 1;
    play(&mut b, CardId::Firebomb, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 46);
    play(&mut b, CardId::Accelerant, Target::Enemy(uid));
    assert_eq!(
        b.enemy(uid).unwrap().hull,
        46 - mult * rules().battle.ember_burn_damage
    );
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.points, 1);
}

#[test]
fn t1_f12_accelerant_upgraded_costs_0_bp() {
    let deck = vec![
        DeckCard::new(CardId::Firebomb),
        DeckCard {
            id: CardId::Accelerant,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Reyes,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let upg_cost = CardId::Accelerant.cost(true);
    assert!(upg_cost < CardId::Accelerant.cost(false));
    let mult = CardId::Accelerant.value(true);
    b.points = 1 + upg_cost + 1;
    play(&mut b, CardId::Firebomb, Target::Enemy(uid));
    assert_eq!(b.points, upg_cost + 1);
    play_upg(&mut b, CardId::Accelerant, true, Target::Enemy(uid));
    assert_eq!(b.points, 1);
    assert_eq!(
        b.enemy(uid).unwrap().hull,
        46 - mult * rules().battle.ember_burn_damage
    );
    assert!(b.enemy(uid).unwrap().marked);
}

#[test]
fn t1_f12_meltdown_salvo_base_scales_with_burning_enemies() {
    let enc = Encounter {
        front: [
            Some(dummy_front(50, 1, 0)),
            Some(dummy_front(50, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::MeltdownSalvo),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    let wv = CardId::Wildfire.value(false);
    let mv = CardId::MeltdownSalvo.value(false);
    let me = CardId::MeltdownSalvo.extra(false);
    b.points = 10;
    play(&mut b, CardId::Wildfire, Target::None);
    play(&mut b, CardId::MeltdownSalvo, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - wv - (mv + 2 * me));
}

#[test]
fn t1_f12_meltdown_salvo_upgraded_deals_7_plus_5_per_burning() {
    let enc = Encounter {
        front: [
            Some(dummy_front(50, 1, 0)),
            Some(dummy_front(50, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::Wildfire),
        DeckCard {
            id: CardId::MeltdownSalvo,
            upgraded: true,
        },
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    let wv = CardId::Wildfire.value(false);
    let mv = CardId::MeltdownSalvo.value(true);
    let me = CardId::MeltdownSalvo.extra(true);
    assert!(mv > CardId::MeltdownSalvo.value(false));
    b.points = 10;
    play(&mut b, CardId::Wildfire, Target::None);
    play_upg(&mut b, CardId::MeltdownSalvo, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - wv - (mv + 2 * me));
}

#[test]
fn t1_f12_ember_doctrine_base_boosts_burn_and_shields_on_burning_kill() {
    let enc = Encounter {
        front: [
            Some(dummy_front(4, 1, 0)),
            Some(dummy_front(50, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Firebomb),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let u0 = b.enemies[0].uid;
    let u1 = b.enemies[1].uid;
    let eb = CardId::EmberDoctrine.value(false);
    let ek = CardId::EmberDoctrine.extra(false);
    b.points = 10;
    play(&mut b, CardId::EmberDoctrine, Target::None);
    // Ignite u1 via Weapons shot (Reyes Plasma)
    b.run_station(Station::Weapons, Target::Enemy(u1)).unwrap();
    assert_eq!(
        b.burn_damage(b.enemy(u1).unwrap().unit()),
        rules().battle.ember_burn_damage + eb
    );
    b.enemies[0].burning = true;
    play(&mut b, CardId::Firebomb, Target::Enemy(u0));
    assert_eq!(b.flagship.shield, ek);
}

#[test]
fn t1_f12_ember_doctrine_upgraded_costs_1bp() {
    let deck = vec![DeckCard {
        id: CardId::EmberDoctrine,
        upgraded: true,
    }];
    let mut b = battle_with(
        AdmiralId::Reyes,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let upg_cost = CardId::EmberDoctrine.cost(true);
    assert!(upg_cost < CardId::EmberDoctrine.cost(false));
    b.points = upg_cost + 1;
    play_upg(&mut b, CardId::EmberDoctrine, true, Target::None);
    assert_eq!(b.points, 1);
    assert_eq!(
        b.flagship.ember_burn_bonus,
        CardId::EmberDoctrine.value(true)
    );
    assert_eq!(
        b.flagship.ember_kill_shield,
        CardId::EmberDoctrine.extra(true)
    );
}

// --- F13: 6 New Neutral Cards (Base & Upgraded) (12 tests) ---

#[test]
fn t1_f13_emergency_power_base_adds_2bp_and_exhausts() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::EmergencyPower)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let gain = CardId::EmergencyPower.value(false) as u32;
    let ev = play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.points, 3 + gain);
    assert_eq!(b.spent.len(), 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, BattleEvent::PointsGained { amount } if *amount == gain))
    );
}

#[test]
fn t1_f13_emergency_power_upgraded_adds_2bp_and_draws_1() {
    let gain = CardId::EmergencyPower.value(true) as u32;
    let draw = CardId::EmergencyPower.extra(true) as usize;
    let deck = vec![
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::EmergencyPower],
    );
    play_upg(&mut b, CardId::EmergencyPower, true, Target::None);
    assert_eq!(b.points, 3 + gain);
    assert_eq!(b.hand.len(), 5 - 1 + draw);
    assert_eq!(b.spent.len(), 1);
}

#[test]
fn t1_f13_autocannon_salvo_base_deals_3x2_homing() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::AutocannonSalvo)],
        encounter_single(unit(EnemyId::Raiders, 40, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::AutocannonSalvo.value(false);
    let hits = CardId::AutocannonSalvo.extra(false);
    let ev = play(&mut b, CardId::AutocannonSalvo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), hits as usize);
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - hits * v);
}

#[test]
fn t1_f13_autocannon_salvo_upgraded_deals_3x3_homing() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard {
            id: CardId::AutocannonSalvo,
            upgraded: true,
        }],
        encounter_single(unit(EnemyId::Raiders, 40, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::AutocannonSalvo.value(true);
    assert!(v > CardId::AutocannonSalvo.value(false));
    let hits = CardId::AutocannonSalvo.extra(true);
    let ev = play_upg(&mut b, CardId::AutocannonSalvo, true, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), hits as usize);
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - hits * v);
}

#[test]
fn t1_f13_target_painter_base_marks_and_breaks_armor_at_1bp() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TargetPainter)],
        encounter_single(dummy_front(40, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let cost = CardId::TargetPainter.cost(false);
    let brk = CardId::TargetPainter.value(false);
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.points, 3 - cost);
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.enemy(uid).unwrap().armor, 2 - brk);
}

#[test]
fn t1_f13_target_painter_upgraded_costs_0bp() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard {
            id: CardId::TargetPainter,
            upgraded: true,
        }],
        encounter_single(dummy_front(40, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let cost = CardId::TargetPainter.cost(true);
    let brk = CardId::TargetPainter.value(true);
    assert!(cost <= CardId::TargetPainter.cost(false));
    assert!(brk >= CardId::TargetPainter.value(false));
    play_upg(&mut b, CardId::TargetPainter, true, Target::Enemy(uid));
    assert_eq!(b.points, 3 - cost);
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.enemy(uid).unwrap().armor, (2 - brk).max(0));
}

#[test]
fn t1_f13_standby_fire_base_1bp_then_0bp_when_retained() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::StandbyFire)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let cost = CardId::StandbyFire.cost(false);
    let dmg = CardId::StandbyFire.value(false);
    assert_eq!(b.card_cost(0), cost);
    b.end_turn();
    assert!(b.is_retained(0));
    assert_eq!(b.card_cost(0), 0);
    play(&mut b, CardId::StandbyFire, Target::Enemy(uid));
    assert_eq!(b.points, 3);
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - dmg);
}

#[test]
fn t1_f13_standby_fire_upgraded_deals_10_dmg() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard {
            id: CardId::StandbyFire,
            upgraded: true,
        }],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let dmg = CardId::StandbyFire.value(true);
    assert!(dmg > CardId::StandbyFire.value(false));
    b.end_turn();
    assert_eq!(b.card_cost(0), 0);
    play_upg(&mut b, CardId::StandbyFire, true, Target::Enemy(uid));
    assert_eq!(b.points, 3);
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - dmg);
}

#[test]
fn t1_f13_chain_command_base_draws_2_and_gains_1bp_after_station() {
    let cost = CardId::ChainCommand.cost(false);
    let draw = CardId::ChainCommand.value(false) as usize;
    let gain = CardId::ChainCommand.extra(false) as u32;
    let mut deck = vec![DeckCard::new(CardId::ChainCommand)];
    deck.extend([DeckCard::new(CardId::Torpedo); 5 + 3]);
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::ChainCommand],
    );
    b.points = cost + 1;
    b.run_station(Station::Shields, Target::None).unwrap();
    play(&mut b, CardId::ChainCommand, Target::None);
    assert_eq!(b.points, 1 + gain);
    assert_eq!(b.hand.len(), 5 - 1 + draw);
}

#[test]
fn t1_f13_chain_command_upgraded_costs_0bp() {
    let upg_cost = CardId::ChainCommand.cost(true);
    assert!(upg_cost < CardId::ChainCommand.cost(false));
    let draw = CardId::ChainCommand.value(true) as usize;
    let gain = CardId::ChainCommand.extra(true) as u32;
    let mut deck = vec![DeckCard {
        id: CardId::ChainCommand,
        upgraded: true,
    }];
    deck.extend([DeckCard::new(CardId::Torpedo); 5 + 3]);
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::ChainCommand],
    );
    b.points = upg_cost + 1;
    b.run_station(Station::Shields, Target::None).unwrap();
    play_upg(&mut b, CardId::ChainCommand, true, Target::None);
    assert_eq!(b.points, 1 + gain);
    assert_eq!(b.hand.len(), 5 - 1 + draw);
}

#[test]
fn t1_f13_bridge_coordination_base_draws_1_on_station() {
    let cost = CardId::BridgeCoordination.cost(false);
    let draw = CardId::BridgeCoordination.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::BridgeCoordination],
    );
    b.points = cost + 1;
    play(&mut b, CardId::BridgeCoordination, Target::None);
    assert_eq!(b.points, 1);
    assert_eq!(b.hand.len(), 4);
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), 4 + draw);
}

#[test]
fn t1_f13_bridge_coordination_upgraded_costs_0bp() {
    let upg_cost = CardId::BridgeCoordination.cost(true);
    assert!(upg_cost < CardId::BridgeCoordination.cost(false));
    let draw = CardId::BridgeCoordination.value(true) as usize;
    let deck = vec![
        DeckCard {
            id: CardId::BridgeCoordination,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        &[CardId::BridgeCoordination],
    );
    b.points = upg_cost + 1;
    play_upg(&mut b, CardId::BridgeCoordination, true, Target::None);
    assert_eq!(b.points, 1);
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), 4 + draw);
}

// --- F14: Build Doctrines Persistence & Stacking (6 tests) ---

#[test]
fn t1_f14_bastion_doctrine_persists_and_stacks() {
    let bs = CardId::BastionDoctrine.value(false);
    let bc = CardId::BastionDoctrine.extra(false);
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    b.points = 10;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert_eq!(b.flagship.shield, 5 + 2 * bs);
    assert_eq!(b.flagship.hold_the_line, 2 * bc);
    b.end_turn();
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert_eq!(b.flagship.shield, 5 + 2 * bs);
    assert_eq!(b.flagship.hold_the_line, 2 * bc);
}

#[test]
fn t1_f14_assault_doctrine_persists_and_stacks() {
    let s = CardId::AssaultDoctrine.value(false) + CardId::AssaultDoctrine.value(true);
    let deck = vec![
        DeckCard::new(CardId::AssaultDoctrine),
        DeckCard {
            id: CardId::AssaultDoctrine,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play_upg(&mut b, CardId::AssaultDoctrine, false, Target::None);
    play_upg(&mut b, CardId::AssaultDoctrine, true, Target::None);
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
    b.end_turn();
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
}

#[test]
fn t1_f14_clockwork_doctrine_persists_and_stacks() {
    let extra_draw = CardId::ClockworkDoctrine.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = (1..20_000)
        .map(|seed| {
            battle_with(
                AdmiralId::Kerr,
                deck.clone(),
                encounter_single(dummy_front(80, 1, 0)),
                vec![],
                seed,
            )
        })
        .find(|b| {
            b.hand
                .iter()
                .filter(|c| c.id == CardId::ClockworkDoctrine)
                .count()
                == 2
        })
        .expect("must draw 2 ClockworkDoctrine in Round 1");
    b.points = 10;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    b.end_turn();
    assert_eq!(b.hand.len(), 5 + 2 * extra_draw);
    b.end_turn();
    assert_eq!(b.hand.len(), 5 + 2 * extra_draw);
}

#[test]
fn t1_f14_ember_doctrine_persists_and_stacks() {
    let eb = CardId::EmberDoctrine.value(false);
    let expected_tick = rules().battle.burn_damage + 2 * eb;
    let deck = vec![
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::EmberDoctrine),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    b.points = 10;
    play(&mut b, CardId::EmberDoctrine, Target::None);
    play(&mut b, CardId::EmberDoctrine, Target::None);
    b.enemies[0].burning = true;
    let uid = b.enemies[0].uid;
    assert_eq!(b.burn_damage(b.enemy(uid).unwrap().unit()), expected_tick);
    b.end_turn();
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - expected_tick);
    b.end_turn();
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 2 * expected_tick);
}

#[test]
fn t1_f14_bridge_coordination_persists_and_stacks() {
    let draw = CardId::BridgeCoordination.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = (1..20_000)
        .map(|seed| {
            battle_with(
                AdmiralId::Osei,
                deck.clone(),
                encounter_single(dummy_front(80, 1, 0)),
                vec![],
                seed,
            )
        })
        .find(|b| {
            b.hand
                .iter()
                .filter(|c| c.id == CardId::BridgeCoordination)
                .count()
                == 2
        })
        .expect("must draw 2 BridgeCoordination in Round 1");
    b.points = 10;
    play(&mut b, CardId::BridgeCoordination, Target::None);
    play(&mut b, CardId::BridgeCoordination, Target::None);
    let before = b.hand.len();
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), before + 2 * draw);
    b.end_turn();
    let before_r2 = b.hand.len();
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), before_r2 + 2 * draw);
}

#[test]
fn t1_f14_doctrines_do_not_go_to_discard_or_spent() {
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::AssaultDoctrine),
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::BridgeCoordination),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    b.points = 20;
    for id in [
        CardId::BastionDoctrine,
        CardId::AssaultDoctrine,
        CardId::ClockworkDoctrine,
        CardId::EmberDoctrine,
        CardId::BridgeCoordination,
    ] {
        play(&mut b, id, Target::None);
    }
    assert_eq!(b.doctrines.len(), 5);
    assert!(b.discard_pile.is_empty());
    assert!(b.spent.is_empty());
}

// ============================================================================
// TIER 2 — BOUNDARY & CORNER CASES (98 tests)
// ============================================================================

// --- F1 Boundaries (5 tests) ---

#[test]
fn t2_f1_overcharge_self_damage_applies_at_end_of_turn() {
    let mut b = battle_with(
        AdmiralId::Vance,
        vec![DeckCard::new(CardId::Overcharge)],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.flagship.hull = 15;
    assert!(!b.flagship.emergency());
    play(&mut b, CardId::Overcharge, Target::None);
    assert_eq!(b.flagship.hull, 15);
    assert_eq!(b.flagship.attack_bonus, 6);
    let ev = b.end_turn();
    assert_eq!(b.flagship.hull, 13);
    assert!(b.flagship.emergency());
    assert!(
        ev.iter()
            .any(|e| matches!(e, BattleEvent::SelfDamage { amount: 2, .. }))
    );
}

#[test]
fn t2_f1_repair_crew_and_damage_control_share_repair_limit_6() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::RepairCrew)],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.flagship.hull = 40;
    b.flagship.burning = true;
    play(&mut b, CardId::RepairCrew, Target::None);
    assert!(!b.flagship.burning);
    assert_eq!(b.flagship.hull, 44);
    assert_eq!(b.flagship.repaired, 4);

    b.run_station(Station::DamageControl, Target::None).unwrap();
    assert_eq!(b.flagship.hull, 46);
    assert_eq!(b.flagship.repaired, rules().battle.repair_limit);
}

#[test]
fn t2_f1_tractor_beam_fails_when_row_full_or_unknown_contact() {
    let full_front = Encounter {
        front: [
            Some(dummy_front(20, 1, 0)),
            Some(dummy_front(20, 1, 0)),
            Some(dummy_front(20, 1, 0)),
        ],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TractorBeam)],
        full_front,
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    assert_eq!(
        b.play_card(0, Target::Enemy(uid)),
        Err(RuleError::TargetInvalid)
    );

    let uc_enc = encounter_single(unit(EnemyId::UnknownContact, 100, 6, 1, vec![Step::Fire]));
    let mut b2 = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TractorBeam)],
        uc_enc,
        vec![],
        1,
    );
    let uc_uid = b2.enemies[0].uid;
    assert_eq!(
        b2.play_card(0, Target::Enemy(uc_uid)),
        Err(RuleError::TargetInvalid)
    );
}

#[test]
fn t2_f1_collision_course_ram_hits_flagship_unshielded_and_uses_attack_bonus() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PowerSurge),
            DeckCard {
                id: CardId::CollisionCourse,
                upgraded: true,
            },
        ],
        encounter_single(dummy_front(50, 7, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.run_station(Station::Shields, Target::None).unwrap();
    play(&mut b, CardId::PowerSurge, Target::None);
    play_upg(&mut b, CardId::CollisionCourse, true, Target::Enemy(uid));
    // 2 * (8 + 7) + 4 = 34 dmg to enemy; 7 unshielded dmg to flagship
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 34);
    assert_eq!(b.flagship.hull, 64 - 7);
    assert_eq!(b.flagship.shield, 11);
}

#[test]
fn t2_f1_coup_de_grace_and_heat_seeker_conditional_bonuses() {
    let enc = Encounter {
        front: [
            Some(unit(EnemyId::Raiders, 40, 2, 0, vec![Step::Evade])),
            Some(dummy_front(40, 2, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::HeatSeeker),
        DeckCard::new(CardId::CoupDeGrace),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    let evader = b.enemies[0].uid;
    let emer = b.enemies[1].uid;
    b.enemies[1].hull = 10; // <= 25% of 40
    assert!(b.enemy(emer).unwrap().emergency());
    play(&mut b, CardId::HeatSeeker, Target::Enemy(evader));
    assert_eq!(b.enemy(evader).unwrap().hull, 40 - 13);
    play(&mut b, CardId::CoupDeGrace, Target::Enemy(emer));
    assert!(b.enemy(emer).is_none());
}

// --- F2 Boundaries (5 tests) ---

#[test]
fn t2_f2_card_data_check_rejects_negative_value_and_missing_or_extra_fields() {
    let d = load_repo_data();
    let mut neg_val = d.cards[&CardId::Torpedo].clone();
    neg_val.value = -1;
    assert!(!neg_val.check(false));

    let mut neg_extra = d.cards[&CardId::FlakBreaker].clone();
    neg_extra.extra = Some(-2);
    assert!(!neg_extra.check(true));

    let mut missing_extra = d.cards[&CardId::FlakBreaker].clone();
    missing_extra.extra = None;
    assert!(!missing_extra.check(true));

    let mut extra_on_non_extra = d.cards[&CardId::Torpedo].clone();
    extra_on_non_extra.extra = Some(1);
    assert!(!extra_on_non_extra.check(false));
}

#[test]
fn t2_f2_data_check_rejects_empty_rules_range() {
    let mut d = load_repo_data();
    d.rules.run.card_price = [80, 40];
    assert!(d.check().is_err());
}

#[test]
fn t2_f2_admiral_data_check_requires_positive_hull_and_signature_in_deck() {
    let d = load_repo_data();
    let mut bad_hull = d.admirals[&AdmiralId::Osei].clone();
    bad_hull.hull = 0;
    assert!(!bad_hull.check());

    let mut missing_sig = d.admirals[&AdmiralId::Osei].clone();
    missing_sig.deck.retain(|&c| c != CardId::HoldTheLine);
    assert!(!missing_sig.check());
}

#[test]
fn t2_f2_all_48_card_files_deserialize_with_no_unknown_fields_and_nonempty_texts() {
    let d = load_repo_data();
    let dir = Path::new(REPO_DATA);
    for id in CardId::ALL {
        let loaded: voidfleet_core::data::CardData = read(dir, &id.file()).unwrap();
        assert_eq!(loaded, d.cards[&id]);
        assert!(!loaded.text["de"].name.is_empty());
        assert!(!loaded.text["en"].name.is_empty());
        assert!(!loaded.text["de"].effect.is_empty());
        assert!(!loaded.text["en"].effect.is_empty());
    }
}

#[test]
fn t2_f2_extra_cards_list_exact_match_with_has_extra() {
    let extra_set: HashSet<CardId> = EXTRA_CARDS.into_iter().collect();
    assert_eq!(extra_set.len(), EXTRA_CARDS.len());
    for id in CardId::ALL {
        assert_eq!(
            id.has_extra(),
            extra_set.contains(&id),
            "{id:?} has_extra mismatch"
        );
    }
}

// --- F3 Boundaries (6 tests) ---

#[test]
fn t2_f3_offer_cards_size_zero_returns_empty() {
    let run = Run::new(AdmiralId::Osei, 1, synthetic_battle_maps(NodeKind::Battle));
    let mut rng = StdRng::seed_from_u64(1);
    assert!(run.offer_cards(&mut rng, 0).is_empty());
}

#[test]
fn t2_f3_offer_cards_size_one_is_always_admiral_card() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let run = Run::new(admiral, 1, maps.clone());
        let mut rng = StdRng::seed_from_u64(99);
        for _ in 0..30 {
            let offer = run.offer_cards(&mut rng, 1);
            assert_eq!(offer.len(), 1);
            assert!(pool.contains(&offer[0]));
        }
    }
}

#[test]
fn t2_f3_offer_cards_size_exceeding_pool_caps_at_33_distinct() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let run = Run::new(admiral, 1, maps.clone());
        let mut rng = StdRng::seed_from_u64(5);
        let offer = run.offer_cards(&mut rng, 100);
        assert_eq!(offer.len(), 33);
        let unique: HashSet<CardId> = offer.into_iter().collect();
        assert_eq!(unique.len(), 33);
    }
}

#[test]
fn t2_f3_loot_offer_never_contains_other_admirals_cards_across_all_4() {
    let maps = synthetic_battle_maps(NodeKind::Battle);
    for admiral in ADMIRAL_IDS {
        let other_cards: HashSet<CardId> = ADMIRAL_IDS
            .into_iter()
            .filter(|&a| a != admiral)
            .flat_map(admiral_pool)
            .collect();
        for seed in 1..=25 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.advance(0).unwrap();
            let mut battle = run.start_battle();
            battle.result = Some(BattleResult::Won);
            let loot = run.finish_battle(&battle);
            for card in loot.cards {
                assert!(!other_cards.contains(&card), "{admiral:?} got {card:?}");
            }
        }
    }
}

#[test]
fn t2_f3_market_card_prices_within_rules_span_and_buy_removes_entry() {
    let mut run = Run::new(
        AdmiralId::Vance,
        44,
        synthetic_battle_maps(NodeKind::Market),
    );
    run.scrap = 300;
    run.advance(0).unwrap();
    let [min_p, max_p] = rules().run.card_price;
    let first_card = match &run.stage {
        Stage::Market(m) => {
            for &(_, price) in &m.cards {
                assert!((min_p..=max_p).contains(&price));
            }
            m.cards[0].0
        }
        _ => panic!("expected market"),
    };
    run.buy_card(0).unwrap();
    match &run.stage {
        Stage::Market(m) => assert_eq!(m.cards.len(), 2),
        _ => panic!("expected market"),
    }
    assert_eq!(run.deck.last().unwrap().id, first_card);
}

#[test]
fn t2_f3_fly_yourself_event_loot_has_guaranteed_admiral_card() {
    let maps = synthetic_battle_maps(NodeKind::Event);
    for seed in 1..200 {
        let mut run = Run::new(AdmiralId::Reyes, seed, maps.clone());
        run.advance(0).unwrap();
        if matches!(run.stage, Stage::Event(voidfleet_core::run::EventId::Pilot)) {
            let pool: HashSet<CardId> = admiral_pool(AdmiralId::Reyes).into_iter().collect();
            let gain = run.choose(1).unwrap(); // FlyYourself
            assert_eq!(gain, Gain::Nothing);
            let Stage::Loot(loot) = &run.stage else {
                panic!("expected Loot after FlyYourself");
            };
            assert_eq!(loot.cards.len(), 3);
            assert!(loot.cards.iter().any(|c| pool.contains(c)));
            return;
        }
    }
    panic!("Pilot event never appeared in 200 seeds");
}

// --- F4 Boundaries (5 tests) ---

#[test]
fn t2_f4_micro_jump_rejected_after_station_used_without_exhausting() {
    let mut b = battle_with(
        AdmiralId::Kerr,
        vec![DeckCard::new(CardId::MicroJump)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.play_card(0, Target::None), Err(RuleError::StationUsed));
    assert!(b.spent.is_empty());
    assert_eq!(b.hand.len(), 1);
}

#[test]
fn t2_f4_emergency_power_exhausts_even_with_empty_draw_and_discard() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::EmergencyPower)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    assert!(b.draw_pile.is_empty());
    assert!(b.discard_pile.is_empty());
    let gain = CardId::EmergencyPower.value(false) as u32;
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.points, 3 + gain);
    assert_eq!(b.spent.len(), 1);
    assert!(b.hand.is_empty());
    assert!(b.discard_pile.is_empty());
}

#[test]
fn t2_f4_exhausted_card_not_in_draw_or_discard_and_battle_ends_with_empty_hand_safely() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::EmergencyPower)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    play(&mut b, CardId::EmergencyPower, Target::None);
    b.end_turn();
    assert!(b.hand.is_empty());
    assert!(b.draw_pile.is_empty());
    assert!(b.discard_pile.is_empty());
    assert_eq!(b.spent.len(), 1);
}

#[test]
fn t2_f4_upgraded_emergency_power_draws_on_empty_draw_by_reshuffling_discard() {
    let deck = vec![
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert!(b.draw_pile.is_empty());
    assert_eq!(b.discard_pile.len(), 1);
    play_upg(&mut b, CardId::EmergencyPower, true, Target::None);
    assert!(b.hand.iter().any(|c| c.id == CardId::Torpedo));
    assert_eq!(b.spent.len(), 1);
}

#[test]
fn t2_f4_micro_jump_blocks_stations_and_evasive_action_same_round_after_exhausting() {
    let deck = vec![
        DeckCard::new(CardId::MicroJump),
        DeckCard::new(CardId::EvasiveAction),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    b.points = 10;
    play(&mut b, CardId::MicroJump, Target::None);
    assert_eq!(b.spent.len(), 1);
    assert_eq!(
        b.run_station(Station::Shields, Target::None),
        Err(RuleError::Jumped)
    );
    assert_eq!(b.play_card(0, Target::Column(0)), Err(RuleError::Jumped));
}

// --- F5 Boundaries (5 tests) ---

#[test]
fn t2_f5_retained_card_persists_across_multiple_end_turns_without_being_played() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::Bulwark)],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert!(b.is_retained(0));
    b.end_turn();
    assert!(b.is_retained(0));
    b.end_turn();
    assert_eq!(b.hand, vec![DeckCard::new(CardId::Bulwark)]);
    assert!(b.is_retained(0));
}

#[test]
fn t2_f5_hand_grows_above_5_when_retained_cards_stay_and_5_new_drawn() {
    let deck = vec![
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        &[CardId::Bulwark, CardId::StandbyFire],
    );
    b.end_turn();
    // 2 retained + 5 drawn = 7; 3 non-retain cards discarded
    assert_eq!(b.hand.len(), 7);
    assert_eq!(b.discard_pile.len(), 3);
}

#[test]
fn t2_f5_playing_retained_card_clears_retained_flag_and_shifts_indices() {
    let deck = vec![
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = (1..20_000)
        .map(|seed| {
            battle_with(
                AdmiralId::Osei,
                deck.clone(),
                encounter_single(dummy_front(50, 0, 0)),
                vec![],
                seed,
            )
        })
        .find(|b| {
            b.hand
                .iter()
                .filter(|c| c.id == CardId::StandbyFire)
                .count()
                == 1
                && b.hand.iter().any(|c| c.id == CardId::Bulwark)
        })
        .expect("must find seed with 1 StandbyFire and Bulwark in Round 1 hand");
    let r1_standby = find_card(&b, CardId::StandbyFire);
    let sf_cost = CardId::StandbyFire.cost(false);
    let sf_val = CardId::StandbyFire.value(false);
    assert!(!b.is_retained(r1_standby));
    assert_eq!(b.card_cost(r1_standby), sf_cost);

    b.end_turn();
    let uid = b.enemies[0].uid;
    let bulwark_idx_before = find_card(&b, CardId::Bulwark);
    assert!(b.is_retained(bulwark_idx_before));

    // Retained StandbyFire (0 BP) and freshly drawn StandbyFire (base cost) coexist in Round 2 hand
    let standby_indices: Vec<usize> = b
        .hand
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (c.id == CardId::StandbyFire).then_some(i))
        .collect();
    assert_eq!(standby_indices.len(), 2);
    let retained_sf = *standby_indices
        .iter()
        .find(|&&i| b.is_retained(i))
        .expect("one StandbyFire must be retained");
    let fresh_sf = *standby_indices
        .iter()
        .find(|&&i| !b.is_retained(i))
        .expect("second StandbyFire must be freshly drawn");
    assert_eq!(b.card_cost(retained_sf), 0);
    assert_eq!(b.card_cost(fresh_sf), sf_cost);

    // Play the retained 0-BP copy and confirm index shift preserves retained flags accurately
    b.play_card(retained_sf, Target::Enemy(uid)).unwrap();
    assert_eq!(b.points, 3);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - sf_val);

    let bulwark_idx_after = find_card(&b, CardId::Bulwark);
    assert!(b.is_retained(bulwark_idx_after));
    let remaining_sf = find_card(&b, CardId::StandbyFire);
    assert!(!b.is_retained(remaining_sf));
    assert_eq!(b.card_cost(remaining_sf), sf_cost);
    for (i, card) in b.hand.iter().enumerate() {
        let expected_retained = card.id == CardId::Bulwark;
        assert_eq!(b.is_retained(i), expected_retained);
    }
}

#[test]
fn t2_f5_all_5_cards_retained_draws_next_cards_into_expanded_hand() {
    let mut deck = vec![DeckCard::new(CardId::PrecisionLock); 5];
    deck.extend([DeckCard::new(CardId::Torpedo); 5]);
    let mut b = (1..20_000)
        .map(|seed| {
            battle_with(
                AdmiralId::Kerr,
                deck.clone(),
                encounter_single(dummy_front(50, 0, 0)),
                vec![],
                seed,
            )
        })
        .find(|b| b.hand.iter().all(|c| c.id == CardId::PrecisionLock))
        .expect("must find seed with 5 PrecisionLock in Round 1 hand");
    b.end_turn();
    assert_eq!(b.hand.len(), 10);
    for (i, c) in b.hand.iter().enumerate() {
        assert_eq!(b.is_retained(i), c.id == CardId::PrecisionLock);
    }
}

#[test]
fn t2_f5_retained_card_with_empty_draw_and_discard_stays_in_hand() {
    let mut b = battle_with(
        AdmiralId::Kerr,
        vec![DeckCard::new(CardId::TimeWindow)],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert_eq!(b.hand.len(), 1);
    assert!(b.is_retained(0));
    assert!(b.draw_pile.is_empty());
    assert!(b.discard_pile.is_empty());
}

// --- F6 Boundaries: Marked Rounding & Modifiers (8 tests) ---

#[test]
fn t2_f6_marked_rounding_on_0_1_2_3_5_7_base_damage() {
    // 0->0, 1->2, 2->3, 3->5, 5->8, 7->11
    let cases = [(0, 0), (1, 2), (2, 3), (3, 5), (5, 8), (7, 11)];
    for (base, expected_total) in cases {
        let mut b = battle_with(
            AdmiralId::Osei,
            vec![DeckCard::new(CardId::PrecisionLock)],
            encounter_single(dummy_front(50, 1, 0)),
            vec![],
            1,
        );
        let uid = b.enemies[0].uid;
        b.points = 10;
        play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
        b.flagship.attack_bonus = base - AdmiralId::Osei.attack();
        let ev = b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
        let bd = hit_breakdowns(&ev, uid)[0];
        assert_eq!(bd.base, base);
        assert_eq!(bd.marked, expected_total - base);
        assert_eq!(bd.hull, expected_total, "base={base}");
    }
}

#[test]
fn t2_f6_marked_plus_evade_non_homing_order_and_preview_parity() {
    // Non-homing attack on Evading + Marked enemy:
    // Evade halves first (floor), then Marked adds +50% rounded up
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PrecisionLock),
            DeckCard::new(CardId::StandbyFire),
        ],
        encounter_single(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::StandbyFire.value(false);
    let evaded = v / 2;
    let after_ev = v - evaded;
    let marked = half_up(after_ev);
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let idx = find_card(&b, CardId::StandbyFire);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    let bd = hit_breakdowns(&ev, uid)[0];
    assert_eq!(bd.base, v);
    assert_eq!(bd.evaded, evaded);
    assert_eq!(bd.marked, marked);
    assert_eq!(bd.hull, after_ev + marked);
}

#[test]
fn t2_f6_marked_plus_cover_non_piercing_order_and_preview_parity() {
    // Back enemy covered by Front enemy. Non-piercing StandbyFire on Marked Back enemy.
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [Some(dummy_front(50, 1, 0)), None, None],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PrecisionLock),
            DeckCard::new(CardId::StandbyFire),
        ],
        enc,
        vec![],
        1,
    );
    let front_uid = b.enemies[0].uid;
    let back_uid = b.enemies[1].uid;
    let v = CardId::StandbyFire.value(false);
    let front_share = half_up(v);
    let back_rem = v - front_share;
    let marked = half_up(back_rem);
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(back_uid));
    let idx = find_card(&b, CardId::StandbyFire);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(back_uid));
    let back_bd = hit_breakdowns(&ev, back_uid)[0];
    let front_bd = hit_breakdowns(&ev, front_uid)[0];
    assert_eq!(front_bd.hull, front_share);
    assert_eq!(back_bd.covered, Some((front_uid, front_share)));
    assert_eq!(back_bd.marked, marked);
    assert_eq!(back_bd.hull, back_rem + marked);
}

#[test]
fn t2_f6_marked_plus_evade_plus_cover_simultaneous_and_preview_parity() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [
            Some(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PrecisionLock),
            DeckCard::new(CardId::StandbyFire),
        ],
        enc,
        vec![],
        1,
    );
    let back_uid = b.enemies[1].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(back_uid));
    let idx = find_card(&b, CardId::StandbyFire);
    assert_preview_matches_execution(&mut b, idx, Target::Enemy(back_uid));
}

#[test]
fn t2_f6_marked_plus_enemy_shields_and_armor_order() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PrecisionLock),
            DeckCard::new(CardId::Torpedo),
        ],
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.enemies[0].shield = 6;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let idx = find_card(&b, CardId::Torpedo);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    // 9 -> marked +5 = 14 -> shield absorbs 6 -> 8 -> armor 2 -> 6 hull
    let bd = hit_breakdowns(&ev, uid)[0];
    assert_eq!(bd.base, 9);
    assert_eq!(bd.marked, 5);
    assert_eq!(bd.shielded, 6);
    assert_eq!(bd.armor, 2);
    assert_eq!(bd.hull, 6);
}

#[test]
fn t2_f6_remarking_already_marked_enemy_is_idempotent() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::PrecisionLock),
            DeckCard::new(CardId::TargetPainter),
            DeckCard::new(CardId::Torpedo),
        ],
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().marked);
    let ev = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid)[0].hull, 14);
}

#[test]
fn t2_f6_marked_does_not_multiply_burn_tick_in_resolution() {
    let mut b = battle_with(
        AdmiralId::Reyes,
        vec![
            DeckCard::new(CardId::Firebomb),
            DeckCard::new(CardId::PrecisionLock),
        ],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::Firebomb, Target::Enemy(uid));
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 46);
    let ev = b.end_turn();
    assert!(
        ev.iter()
            .any(|e| matches!(e, BattleEvent::BurnTick { amount: 6, .. }))
    );
    assert_eq!(b.enemy(uid).unwrap().hull, 40);
}

#[test]
fn t2_f6_marked_cleared_even_if_enemy_was_in_stasis_or_silenced() {
    let mut b = battle_with(
        AdmiralId::Kerr,
        vec![DeckCard::new(CardId::PrecisionLock)],
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().stasis);
    assert!(b.enemy(uid).unwrap().marked);
    b.end_turn();
    assert!(!b.enemy(uid).unwrap().marked);
}

// --- F7 Boundaries: Panzerbruch (5 tests) ---

#[test]
fn t2_f7_armor_zero_floor_never_negative_for_all_three_breakers() {
    let deck = vec![
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::FlakBreaker),
        DeckCard::new(CardId::ArmorPiercer),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

#[test]
fn t2_f7_armor_1_drops_to_0_and_subsequent_hits_unmitigated() {
    let deck = vec![
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 1)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let ac_val = CardId::AutocannonSalvo.value(false);
    let ac_marked = ac_val + half_up(ac_val);
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
    let ev = play(&mut b, CardId::AutocannonSalvo, Target::Enemy(uid));
    for bd in hit_breakdowns(&ev, uid) {
        assert_eq!(bd.armor, 0);
        assert_eq!(bd.hull, ac_marked);
    }
}

#[test]
fn t2_f7_flak_breaker_armor_break_timing_matches_preview_against_armor_1() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::FlakBreaker)],
        encounter_single(dummy_front(50, 1, 1)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    assert_preview_matches_execution(&mut b, 0, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

#[test]
fn t2_f7_armor_piercer_armor_break_timing_matches_preview_against_armor_1() {
    let mut b = battle_with(
        AdmiralId::Vance,
        vec![DeckCard::new(CardId::ArmorPiercer)],
        encounter_single(dummy_front(50, 1, 1)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    assert_preview_matches_execution(&mut b, 0, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

#[test]
fn t2_f7_panzerbruch_against_covered_back_enemy_reduces_back_armor() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 2)), None, None],
        back: [Some(dummy_front(50, 1, 2)), None, None],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![
            DeckCard::new(CardId::TargetPainter),
            DeckCard::new(CardId::FlakBreaker),
        ],
        enc,
        vec![],
        1,
    );
    let front_uid = b.enemies[0].uid;
    let back_uid = b.enemies[1].uid;
    play(&mut b, CardId::TargetPainter, Target::Enemy(back_uid));
    assert_eq!(b.enemy(back_uid).unwrap().armor, 1);
    assert_eq!(b.enemy(front_uid).unwrap().armor, 2);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(back_uid));
    assert_eq!(b.enemy(back_uid).unwrap().armor, 0);
}

// --- F8 Boundaries: TimeWindow Corner Cases (8 tests) ---

#[test]
fn t2_f8_time_window_plus_evasive_action_moves_column() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::EvasiveAction),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::EvasiveAction, Target::Column(0));
    assert_eq!(b.flagship.column, 0);
    assert_eq!(b.flagship.double_next, 0);
}

#[test]
fn t2_f8_time_window_plus_emergency_power_grants_4bp_and_exhausts_once() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::EmergencyPower),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let tw_cost = CardId::TimeWindow.cost(false);
    let ep_gain = CardId::EmergencyPower.value(false) as u32;
    b.points = tw_cost + 1;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.points, 1);
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.points, 1 + 2 * ep_gain);
    assert_eq!(b.spent.len(), 1);
}

#[test]
fn t2_f8_time_window_plus_bastion_and_clockwork_doctrines_stacks() {
    let bs = CardId::BastionDoctrine.value(false);
    let bc = CardId::BastionDoctrine.extra(false);
    let cd = CardId::ClockworkDoctrine.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.points = 20;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    assert_eq!(b.flagship.bastion_shield, 2 * bs);
    assert_eq!(b.flagship.bastion_counterfire, 2 * bc);
    assert_eq!(b.flagship.clockwork_draw, 2 * cd);
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert_eq!(b.flagship.shield, 5 + 2 * bs);
    assert_eq!(b.flagship.hold_the_line, 2 * bc);
}

#[test]
fn t2_f8_time_window_single_target_kill_on_first_hit_fizzles_second_safely() {
    let bv_val = CardId::BreakthroughVolley.value(false);
    let bv_cost = CardId::BreakthroughVolley.cost(false);
    let bv_refund = CardId::BreakthroughVolley.extra(false) as u32;
    let tw_cost = CardId::TimeWindow.cost(false);
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(bv_val, 1, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        enc,
        vec![],
        &[CardId::TimeWindow, CardId::BreakthroughVolley],
    );
    let target = b.enemies[0].uid;
    b.points = tw_cost + bv_cost + 1;
    play(&mut b, CardId::TimeWindow, Target::None);
    let ev = play(&mut b, CardId::BreakthroughVolley, Target::Enemy(target));
    assert_eq!(hit_breakdowns(&ev, target).len(), 1);
    assert_eq!(b.points, 1 + bv_refund);
}

#[test]
fn t2_f8_time_window_collision_course_kill_no_second_self_damage() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::CollisionCourse),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(15, 5, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 1);
    let target = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::CollisionCourse, Target::Enemy(target));
    // Kerr ATK 9 -> 18 dmg kills 15-HP target on 1st hit; only 1x 5 self-dmg
    assert!(b.enemy(target).is_none());
    assert_eq!(b.flagship.hull, AdmiralId::Kerr.hull() - 5);
}

#[test]
fn t2_f8_time_window_stacked_twice_in_same_round() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 20;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.flagship.double_next, 1);
    // Second TimeWindow is itself doubled! -> double_next becomes 2
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.flagship.double_next, 2);
    let ev1 = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev1, uid).len(), 2);
    assert_eq!(b.flagship.double_next, 1);
    let ev2 = play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev2, uid).len(), 2);
    assert_eq!(b.flagship.double_next, 0);
}

#[test]
fn t2_f8_time_window_plus_retained_standby_fire_0bp_double_hit() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::StandbyFire),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let tw_cost = CardId::TimeWindow.cost(false);
    let sf_val = CardId::StandbyFire.value(false);
    b.end_turn();
    b.points = tw_cost + 1;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.points, 1);
    let idx = find_card(&b, CardId::StandbyFire);
    assert_eq!(b.card_cost(idx), 0);
    play(&mut b, CardId::StandbyFire, Target::Enemy(uid));
    assert_eq!(b.points, 1);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 2 * sf_val);
}

#[test]
fn t2_f8_time_window_preview_matches_doubled_attack_execution() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::FlakBreaker),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    let idx = find_card(&b, CardId::FlakBreaker);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 4);
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

// --- F9 Boundaries: Osei (8 tests) ---

#[test]
fn t2_f9_shield_bash_at_zero_shields_deals_0_base_and_ignores_evade() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::ShieldBash)],
        encounter_single(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    assert_eq!(b.flagship.shield, 0);
    let ev = assert_preview_matches_execution(&mut b, 0, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid)[0].hull, 0);
    assert_eq!(b.enemy(uid).unwrap().hull, 50);
}

#[test]
fn t2_f9_shield_bash_at_11_shields_ignores_evade_and_does_not_consume_shields() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::ShieldBash)],
        encounter_single(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.run_station(Station::Shields, Target::None).unwrap();
    play(&mut b, CardId::ShieldBash, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 39);
    assert_eq!(b.flagship.shield, 11);
}

#[test]
fn t2_f9_shield_bash_with_bulwark_and_bastion_doctrine_and_marked() {
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::ShieldBash),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let total_shield = CardId::Bulwark.value(false) + CardId::BastionDoctrine.value(false);
    let expected = total_shield + half_up(total_shield);
    b.points = 15;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(b.flagship.shield, total_shield);
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let idx = find_card(&b, CardId::ShieldBash);
    assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - expected);
}

#[test]
fn t2_f9_bastion_doctrine_triggers_only_on_shield_cards_not_other_buffs_or_stations() {
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::PowerSurge),
        DeckCard::new(CardId::RepairCrew),
        DeckCard::new(CardId::Overcharge),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    b.points = 10;
    b.flagship.hull = 50;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::PowerSurge, Target::None);
    play(&mut b, CardId::RepairCrew, Target::None);
    play(&mut b, CardId::Overcharge, Target::None);
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 11);
    assert_eq!(b.flagship.hold_the_line, 0);
}

#[test]
fn t2_f9_bastion_doctrine_retaliation_damages_shooters_and_unknown_contact_without_stopping_regen()
{
    let uc = unit(EnemyId::UnknownContact, 100, 5, 0, vec![Step::Fire]);
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::Bulwark),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, encounter_single(uc), vec![], 1);
    let uid = b.enemies[0].uid;
    let bc = CardId::BastionDoctrine.extra(false);
    b.points = 10;
    b.enemies[0].hull = 80;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(b.flagship.hold_the_line, bc);
    b.end_turn();
    assert_eq!(
        b.enemy(uid).unwrap().hull,
        80 - bc + rules().enemies.regeneration
    );
}

#[test]
fn t2_f9_bulwark_retained_plus_osei_shields_station_stacks() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::Bulwark)],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert!(b.is_retained(0));
    b.run_station(Station::Shields, Target::None).unwrap();
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(b.flagship.shield, 11 + CardId::Bulwark.value(false));
}

#[test]
fn t2_f9_flak_breaker_against_shielded_enemy_breaks_armor_even_if_shield_absorbs() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::FlakBreaker)],
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.enemies[0].shield = 20;
    let v = CardId::FlakBreaker.value(false);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 1);
    assert_eq!(b.enemy(uid).unwrap().hull, 50);
    assert_eq!(b.enemy(uid).unwrap().shield, 20 - 2 * v);
}

#[test]
fn t2_f9_shield_bash_upgraded_0bp_playable_at_0_points() {
    let deck = vec![
        DeckCard {
            id: CardId::ShieldBash,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let upg_cost = CardId::ShieldBash.cost(true);
    b.points = 3 + upg_cost;
    play(&mut b, CardId::EmergencyShields, Target::None);
    play(&mut b, CardId::Torpedo, Target::Enemy(uid));
    assert_eq!(b.points, upg_cost);
    play_upg(&mut b, CardId::ShieldBash, true, Target::Enemy(uid));
    assert_eq!(b.points, 0);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 9 - 5);
}

// --- F10 Boundaries: Vance (8 tests) ---

#[test]
fn t2_f10_breakthrough_volley_upgraded_armor_adjusted_exact_lethal_grants_2bp_and_draws_1() {
    let v = CardId::BreakthroughVolley.value(true);
    let cost = CardId::BreakthroughVolley.cost(true);
    let refund = CardId::BreakthroughVolley.extra(true) as u32;
    let deck = vec![
        DeckCard {
            id: CardId::BreakthroughVolley,
            upgraded: true,
        },
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Overcharge),
    ];
    // Upgraded deals v vs 1 armor = (v - 1) net damage -> exact lethal on (v - 1)-HP enemy
    let enc_lethal = Encounter {
        front: [
            Some(dummy_front(v - 1, 1, 1)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck.clone(),
        enc_lethal,
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let uid = b.enemies[0].uid;
    b.points = cost + 1;
    play_upg(&mut b, CardId::BreakthroughVolley, true, Target::Enemy(uid));
    assert!(b.enemy(uid).is_none());
    assert_eq!(b.points, 1 + refund);
    assert_eq!(b.hand.len(), 5);

    // v HP with 1 armor survives at 1 HP -> no BP refund, no draw
    let mut b_survive = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(v, 1, 1)),
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let uid_s = b_survive.enemies[0].uid;
    b_survive.points = cost + 1;
    play_upg(
        &mut b_survive,
        CardId::BreakthroughVolley,
        true,
        Target::Enemy(uid_s),
    );
    assert_eq!(b_survive.enemy(uid_s).unwrap().hull, 1);
    assert_eq!(b_survive.points, 1);
    assert_eq!(b_survive.hand.len(), 4);
}

#[test]
fn t2_f10_breakthrough_volley_non_lethal_12_grants_nothing() {
    let v = CardId::BreakthroughVolley.value(false);
    let cost = CardId::BreakthroughVolley.cost(false);
    let deck = vec![
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Overcharge),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(v + 1, 1, 0)),
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let uid = b.enemies[0].uid;
    b.points = cost + 1;
    play(&mut b, CardId::BreakthroughVolley, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 1);
    assert_eq!(b.points, 1);
    assert_eq!(b.hand.len(), 4);
}

#[test]
fn t2_f10_breakthrough_volley_cover_kills_front_while_back_target_survives_no_reward() {
    let cost = CardId::BreakthroughVolley.cost(false);
    let enc = Encounter {
        front: [Some(dummy_front(4, 1, 0)), None, None],
        back: [Some(dummy_front(40, 1, 0)), None, None],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Overcharge),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        enc,
        vec![],
        &[CardId::BreakthroughVolley],
    );
    let front_uid = b.enemies[0].uid;
    let back_uid = b.enemies[1].uid;
    b.points = cost + 1;
    play(&mut b, CardId::BreakthroughVolley, Target::Enemy(back_uid));
    assert!(b.enemy(front_uid).is_none());
    assert!(b.enemy(back_uid).is_some());
    assert_eq!(b.points, 1);
    assert_eq!(b.hand.len(), 4);
}

#[test]
fn t2_f10_breakthrough_volley_empty_draw_pile_reshuffles_discard_on_kill_draw() {
    let v = CardId::BreakthroughVolley.value(false);
    let deck = vec![
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Overcharge),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(v, 1, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    play(&mut b, CardId::Overcharge, Target::None);
    assert!(b.draw_pile.is_empty());
    assert_eq!(b.discard_pile.len(), 1);
    let ev = play(&mut b, CardId::BreakthroughVolley, Target::Enemy(uid));
    assert!(ev.iter().any(|e| matches!(e, BattleEvent::Shuffled)));
    assert_eq!(b.hand.len(), 1);
    assert!(b.discard_pile.is_empty());
    assert_eq!(b.draw_pile.len(), 1);
}

#[test]
fn t2_f10_breakthrough_volley_last_enemy_kill_ends_battle_won() {
    let v = CardId::BreakthroughVolley.value(false);
    let mut b = battle_with(
        AdmiralId::Vance,
        vec![DeckCard::new(CardId::BreakthroughVolley)],
        encounter_single(dummy_front(v, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    play(&mut b, CardId::BreakthroughVolley, Target::Enemy(uid));
    assert_eq!(b.result, Some(BattleResult::Won));
}

#[test]
fn t2_f10_onslaught_momentum_0_and_5_and_reset_after_maneuver() {
    let deck = vec![
        DeckCard::new(CardId::Onslaught),
        DeckCard::new(CardId::Onslaught),
        DeckCard::new(CardId::Onslaught),
    ];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(unit(EnemyId::GuardCruiser, 100, 0, 0, vec![Step::Fire])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::Onslaught.value(false);
    assert_eq!(b.flagship.momentum, 0);
    play(&mut b, CardId::Onslaught, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 100 - v);

    b.flagship.momentum = 5;
    play(&mut b, CardId::Onslaught, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 100 - v - (v + 10));

    b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    b.end_turn();
    assert_eq!(b.flagship.momentum, 0);
    play(&mut b, CardId::Onslaught, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 100 - v - (v + 10) - v);
}

#[test]
fn t2_f10_assault_doctrine_triggers_on_weapons_and_focus_fire_only() {
    let mut b = battle_with(
        AdmiralId::Vance,
        vec![DeckCard::new(CardId::AssaultDoctrine)],
        encounter_single(dummy_front(100, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let s = CardId::AssaultDoctrine.value(false);
    b.points = CardId::AssaultDoctrine.cost(false);
    play(&mut b, CardId::AssaultDoctrine, Target::None);
    b.run_station(Station::DamageControl, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 0);
    b.end_turn();
    b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    assert_eq!(b.flagship.shield, 0);
    b.end_turn();
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
    b.end_turn();
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
}

#[test]
fn t2_f10_armor_piercer_piercing_bypasses_cover_and_marks_before_follow_up_hits() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 2)), None, None],
        back: [Some(dummy_front(50, 1, 2)), None, None],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::ArmorPiercer),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    let ap_dmg = (CardId::ArmorPiercer.value(false) - 2).max(0);
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(back_uid));
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - ap_dmg);
    assert!(b.enemy(back_uid).unwrap().marked);
    assert_eq!(b.enemy(back_uid).unwrap().armor, 1);
    play(&mut b, CardId::Torpedo, Target::Enemy(back_uid));
    // Torpedo (9) -> Cover 5 to front, 4 to back -> Marked +2 = 6 -> armor 1 -> 5 dmg
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - ap_dmg - 5);
}

// --- F11 Boundaries: Kerr (9 tests) ---

#[test]
fn t2_f11_chrono_strike_in_stasis_deals_20_upg_26() {
    let deck = vec![
        DeckCard::new(CardId::ChronoStrike),
        DeckCard {
            id: CardId::ChronoStrike,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let base_v = CardId::ChronoStrike.value(false);
    let upg_v = CardId::ChronoStrike.value(true);
    b.points = 10;
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    play_upg(&mut b, CardId::ChronoStrike, false, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 2 * base_v);
    play_upg(&mut b, CardId::ChronoStrike, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 2 * base_v - 2 * upg_v);
}

#[test]
fn t2_f11_chrono_strike_intending_charge_and_already_charged_semantics() {
    let enc = Encounter {
        front: [
            Some(unit(
                EnemyId::GuardCruiser,
                80,
                5,
                0,
                vec![Step::Charge, Step::Fire],
            )),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::ChronoStrike),
        DeckCard::new(CardId::ChronoStrike),
    ];
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    let v = CardId::ChronoStrike.value(false);
    assert_eq!(b.enemies[0].intents[0], Action::Charge);
    play(&mut b, CardId::ChronoStrike, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 2 * v);
    b.end_turn();
    assert!(b.enemy(uid).unwrap().charged);
    assert_eq!(b.enemies[0].intents[0], Action::Fire);
    // In Round 2 enemy is already charged (even though intent[0] is Fire): check spec semantics!
    play(&mut b, CardId::ChronoStrike, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 4 * v);
}

#[test]
fn t2_f11_chrono_strike_normal_enemy_deals_10_upg_13() {
    let deck = vec![
        DeckCard::new(CardId::ChronoStrike),
        DeckCard {
            id: CardId::ChronoStrike,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let base_v = CardId::ChronoStrike.value(false);
    let upg_v = CardId::ChronoStrike.value(true);
    b.points = 10;
    play_upg(&mut b, CardId::ChronoStrike, false, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - base_v);
    play_upg(&mut b, CardId::ChronoStrike, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - base_v - upg_v);
}

#[test]
fn t2_f11_chrono_strike_silenced_and_evading_homing_piercing_behavior() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [
            Some(unit(EnemyId::Raiders, 50, 2, 0, vec![Step::Evade])),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::RadioSilence),
        DeckCard::new(CardId::ChronoStrike),
    ];
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    let v = CardId::ChronoStrike.value(false);
    b.points = 10;
    play(&mut b, CardId::RadioSilence, Target::Enemy(back_uid));
    play(&mut b, CardId::ChronoStrike, Target::Enemy(back_uid));
    // Homing + Piercing: full value damage to silenced, evading, covered back enemy
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - v);
}

#[test]
fn t2_f11_chrono_strike_stasis_and_charged_simultaneous_not_40() {
    let mut b = battle_with(
        AdmiralId::Kerr,
        vec![DeckCard::new(CardId::ChronoStrike)],
        encounter_single(unit(
            EnemyId::GuardCruiser,
            80,
            5,
            0,
            vec![Step::Charge, Step::Fire],
        )),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let v = CardId::ChronoStrike.value(false);
    b.enemies[0].charged = true;
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    play(&mut b, CardId::ChronoStrike, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 2 * v);
}

#[test]
fn t2_f11_clockwork_doctrine_first_tactic_1bp_to_0_and_second_tactic_full_cost() {
    let cd_cost = CardId::ClockworkDoctrine.cost(false);
    let discount = CardId::ClockworkDoctrine.extra(false) as u32;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::Scavenge),
        DeckCard::new(CardId::Countermeasures),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        &[
            CardId::ClockworkDoctrine,
            CardId::Scavenge,
            CardId::Countermeasures,
        ],
    );
    b.points = cd_cost + 1;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    assert_eq!(b.points, 1);
    let s_idx = find_card(&b, CardId::Scavenge);
    assert_eq!(b.card_cost(s_idx), 1u32.saturating_sub(discount));
    play(&mut b, CardId::Scavenge, Target::None);
    assert_eq!(b.points, 1);
    let c_idx = find_card(&b, CardId::Countermeasures);
    assert_eq!(b.card_cost(c_idx), 1);
    play(&mut b, CardId::Countermeasures, Target::None);
    assert_eq!(b.points, 0);
}

#[test]
fn t2_f11_clockwork_doctrine_first_debuff_3bp_to_2_and_second_debuff_full_cost() {
    let discount = CardId::ClockworkDoctrine.extra(false) as u32;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::RadioSilence),
        DeckCard::new(CardId::TargetingJammer),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 5, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    let pre = b.points;
    let r_idx = find_card(&b, CardId::RadioSilence);
    assert_eq!(b.card_cost(r_idx), 3u32.saturating_sub(discount));
    play(&mut b, CardId::RadioSilence, Target::Enemy(uid));
    assert_eq!(b.points, pre - 3u32.saturating_sub(discount));
    let j_idx = find_card(&b, CardId::TargetingJammer);
    assert_eq!(b.card_cost(j_idx), 1);
}

#[test]
fn t2_f11_clockwork_doctrine_0cost_interaction_and_attack_buff_doctrine_exempt() {
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::AblativeArmor),
        DeckCard::new(CardId::EmergencyPower),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 15;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    // Attack, Buff, Doctrine must NOT get discounted or consume discount
    assert_eq!(
        b.card_cost(find_card(&b, CardId::StandbyFire)),
        CardId::StandbyFire.cost(false)
    );
    assert_eq!(b.card_cost(find_card(&b, CardId::EmergencyShields)), 1);
    assert_eq!(b.card_cost(find_card(&b, CardId::AblativeArmor)), 1);
    play(&mut b, CardId::StandbyFire, Target::Enemy(uid));
    play(&mut b, CardId::EmergencyShields, Target::None);
    play(&mut b, CardId::AblativeArmor, Target::None);
    assert!(!b.flagship.tactic_discount_used);
    // 0-cost Tactic (EmergencyPower) floors at 0 and marks discount used
    assert_eq!(b.card_cost(find_card(&b, CardId::EmergencyPower)), 0);
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert!(b.flagship.tactic_discount_used);
}

#[test]
fn t2_f11_clockwork_doctrine_same_turn_activation_resets_next_round_and_stacks_supply_pod() {
    let cd_draw = CardId::ClockworkDoctrine.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::Scavenge),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![ModuleId::SupplyPod],
        &[CardId::ClockworkDoctrine, CardId::Scavenge],
    );
    // SupplyPod hand_size = 6
    assert_eq!(b.hand.len(), 6);
    b.points = CardId::ClockworkDoctrine.cost(false);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    assert_eq!(b.card_cost(find_card(&b, CardId::Scavenge)), 0);
    play(&mut b, CardId::Scavenge, Target::None);
    assert!(b.flagship.tactic_discount_used);
    b.end_turn();
    assert!(!b.flagship.tactic_discount_used);
    assert_eq!(b.hand.len(), 6 + cd_draw);
}

// --- F12 Boundaries: Reyes (8 tests) ---

#[test]
fn t2_f12_accelerant_on_non_burning_enemy_behavior_and_preview_parity() {
    let mut b = battle_with(
        AdmiralId::Reyes,
        vec![DeckCard::new(CardId::Accelerant)],
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    assert_preview_matches_execution(&mut b, 0, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 50);
    assert!(b.enemy(uid).unwrap().marked);
}

#[test]
fn t2_f12_accelerant_reyes_6_vs_non_reyes_2_and_ember_doctrine_boost() {
    let wv = CardId::Wildfire.value(false);
    let eb = CardId::EmberDoctrine.value(false);
    let mult = CardId::Accelerant.value(false);
    let deck = vec![
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Accelerant),
    ];
    let mut b_reyes = battle_with(
        AdmiralId::Reyes,
        deck.clone(),
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid_r = b_reyes.enemies[0].uid;
    b_reyes.points = 15;
    play(&mut b_reyes, CardId::Wildfire, Target::None);
    play(&mut b_reyes, CardId::EmberDoctrine, Target::None);
    play(&mut b_reyes, CardId::Accelerant, Target::Enemy(uid_r));
    assert_eq!(
        b_reyes.enemy(uid_r).unwrap().hull,
        80 - wv - mult * (rules().battle.ember_burn_damage + eb)
    );

    let mut b_osei = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(80, 1, 0)),
        vec![],
        1,
    );
    let uid_o = b_osei.enemies[0].uid;
    b_osei.points = 15;
    play(&mut b_osei, CardId::Wildfire, Target::None);
    play(&mut b_osei, CardId::EmberDoctrine, Target::None);
    play(&mut b_osei, CardId::Accelerant, Target::Enemy(uid_o));
    assert_eq!(
        b_osei.enemy(uid_o).unwrap().hull,
        80 - wv - mult * (rules().battle.burn_damage + eb)
    );
}

#[test]
fn t2_f12_accelerant_on_already_marked_enemy_matches_preview() {
    let wv = CardId::Wildfire.value(false);
    let mult = CardId::Accelerant.value(false);
    let deck = vec![
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::Accelerant),
    ];
    let mut b = battle_with(
        AdmiralId::Reyes,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 15;
    play(&mut b, CardId::Wildfire, Target::None);
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    let idx = find_card(&b, CardId::Accelerant);
    assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    // Direct burn burst is not multiplied by Marked
    assert_eq!(
        b.enemy(uid).unwrap().hull,
        50 - wv - mult * rules().battle.ember_burn_damage
    );
}

#[test]
fn t2_f12_accelerant_lethal_burn_burst_triggers_destroyed_and_ember_shield() {
    let wv = CardId::Wildfire.value(false);
    let eb = CardId::EmberDoctrine.value(false);
    let ek = CardId::EmberDoctrine.extra(false);
    let mult = CardId::Accelerant.value(false);
    let lethal_hp = wv + mult * (rules().battle.ember_burn_damage + eb);
    let enc = Encounter {
        front: [
            Some(dummy_front(lethal_hp, 1, 0)),
            Some(dummy_front(50, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Accelerant),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    b.points = 15;
    play(&mut b, CardId::Wildfire, Target::None);
    play(&mut b, CardId::EmberDoctrine, Target::None);
    let ev = play(&mut b, CardId::Accelerant, Target::Enemy(uid));
    assert!(b.enemy(uid).is_none());
    assert!(ev.iter().any(|e| matches!(
        e,
        BattleEvent::Destroyed {
            target: UnitRef::Enemy { uid: u, .. },
            cause: DestroyCause::Shot
        } if *u == uid
    )));
    assert_eq!(b.flagship.shield, ek);
}

#[test]
fn t2_f12_wildfire_piercing_aoe_ignites_front_back_and_evading_and_shielded() {
    let wv = CardId::Wildfire.value(false);
    let ev_dmg = wv - wv / 2;
    let enc = Encounter {
        front: [
            Some(dummy_front(40, 1, 0)),
            Some(unit(EnemyId::Raiders, 40, 1, 0, vec![Step::Evade])),
            None,
        ],
        back: [Some(dummy_front(40, 1, 0)), None, None],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Reyes,
        vec![DeckCard::new(CardId::Wildfire)],
        enc,
        vec![],
        1,
    );
    b.enemies[0].shield = 10;
    play(&mut b, CardId::Wildfire, Target::None);
    assert!(b.enemies[0].burning);
    assert_eq!(b.enemies[0].shield, 10 - wv);
    assert_eq!(b.enemies[1].hull, 40 - ev_dmg);
    assert!(b.enemies[1].burning);
    assert_eq!(b.enemies[2].hull, 40 - wv);
    assert!(b.enemies[2].burning);
}

#[test]
fn t2_f12_meltdown_salvo_0_1_2_4_burning_enemies_target_vs_others() {
    let mv = CardId::MeltdownSalvo.value(false);
    let me = CardId::MeltdownSalvo.extra(false);
    let mv_u = CardId::MeltdownSalvo.value(true);
    let me_u = CardId::MeltdownSalvo.extra(true);
    let enc = Encounter {
        front: [
            Some(dummy_front(80, 0, 0)),
            Some(dummy_front(80, 0, 0)),
            None,
        ],
        back: [
            Some(dummy_front(80, 0, 0)),
            Some(dummy_front(80, 0, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::MeltdownSalvo),
        DeckCard::new(CardId::MeltdownSalvo),
        DeckCard::new(CardId::MeltdownSalvo),
        DeckCard {
            id: CardId::MeltdownSalvo,
            upgraded: true,
        },
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    b.points = 20;
    let t0 = b.enemies[0].uid;

    // 0 burning
    play_upg(&mut b, CardId::MeltdownSalvo, false, Target::Enemy(t0));
    let mut expected_hull = 80 - mv;
    assert_eq!(b.enemy(t0).unwrap().hull, expected_hull);

    // 1 burning (another enemy, not target)
    b.enemies[1].burning = true;
    play_upg(&mut b, CardId::MeltdownSalvo, false, Target::Enemy(t0));
    expected_hull -= mv + me;
    assert_eq!(b.enemy(t0).unwrap().hull, expected_hull);

    // 2 burning
    b.enemies[2].burning = true;
    play_upg(&mut b, CardId::MeltdownSalvo, false, Target::Enemy(t0));
    expected_hull -= mv + 2 * me;
    assert_eq!(b.enemy(t0).unwrap().hull, expected_hull);

    // 4 burning, upgraded
    b.enemies[0].burning = true;
    b.enemies[3].burning = true;
    play_upg(&mut b, CardId::MeltdownSalvo, true, Target::Enemy(t0));
    expected_hull -= mv_u + 4 * me_u;
    assert_eq!(b.enemy(t0).unwrap().hull, expected_hull);
}

#[test]
fn t2_f12_ember_doctrine_burning_kill_grants_5_shields_non_burning_grants_0() {
    let ek = CardId::EmberDoctrine.extra(false);
    let enc = Encounter {
        front: [
            Some(dummy_front(5, 0, 0)),
            Some(dummy_front(5, 0, 0)),
            Some(dummy_front(50, 0, 0)),
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    let u0 = b.enemies[0].uid;
    let u1 = b.enemies[1].uid;
    b.points = 10;
    b.enemies[0].burning = true;
    play(&mut b, CardId::EmberDoctrine, Target::None);
    play(&mut b, CardId::Torpedo, Target::Enemy(u1));
    assert_eq!(b.flagship.shield, 0);
    play(&mut b, CardId::Torpedo, Target::Enemy(u0));
    assert_eq!(b.flagship.shield, ek);
}

#[test]
fn t2_f12_ember_doctrine_burn_tick_kill_and_reyes_vs_non_reyes_ticks() {
    let eb = CardId::EmberDoctrine.value(false);
    let reyes_tick = rules().battle.ember_burn_damage + eb;
    let osei_tick = rules().battle.burn_damage + eb;
    let enc = Encounter {
        front: [
            Some(dummy_front(reyes_tick, 0, 0)),
            Some(dummy_front(50, 0, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b_reyes = battle_with(
        AdmiralId::Reyes,
        vec![DeckCard::new(CardId::EmberDoctrine)],
        enc.clone(),
        vec![],
        1,
    );
    play(&mut b_reyes, CardId::EmberDoctrine, Target::None);
    b_reyes.enemies[0].burning = true;
    b_reyes.enemies[1].burning = true;
    let ev = b_reyes.end_turn();
    assert_eq!(b_reyes.enemies.len(), 1);
    assert_eq!(b_reyes.enemies[0].hull, 50 - reyes_tick);
    assert!(!ev.iter().any(|e| matches!(
        e,
        BattleEvent::Shielded {
            target: UnitRef::Flagship,
            ..
        }
    )));
    assert_eq!(b_reyes.flagship.shield, 0);

    let mut b_osei = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::EmberDoctrine)],
        enc,
        vec![],
        1,
    );
    play(&mut b_osei, CardId::EmberDoctrine, Target::None);
    b_osei.enemies[0].burning = true;
    b_osei.end_turn();
    assert_eq!(b_osei.enemies[0].hull, reyes_tick - osei_tick);
}

// --- F13 Boundaries: Neutral Cards (10 tests) ---

#[test]
fn t2_f13_standby_fire_round1_1bp_fails_at_0bp() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::StandbyFire)],
        encounter_single(dummy_front(40, 1, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 0;
    assert_eq!(b.card_cost(0), CardId::StandbyFire.cost(false));
    assert_eq!(
        b.play_card(0, Target::Enemy(uid)),
        Err(RuleError::NotEnoughPoints)
    );
}

#[test]
fn t2_f13_standby_fire_retained_round2_0bp_playable_at_0bp() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::StandbyFire)],
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let sf_val = CardId::StandbyFire.value(false);
    b.end_turn();
    b.points = 0;
    assert_eq!(b.card_cost(0), 0);
    play(&mut b, CardId::StandbyFire, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - sf_val);
}

#[test]
fn t2_f13_standby_fire_retained_across_multiple_rounds_remains_0bp() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::StandbyFire)],
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        1,
    );
    b.end_turn();
    assert_eq!(b.card_cost(0), 0);
    b.end_turn();
    assert!(b.is_retained(0));
    assert_eq!(b.card_cost(0), 0);
}

#[test]
fn t2_f13_standby_fire_round1_not_discounted_by_clockwork_doctrine() {
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::StandbyFire),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        1,
    );
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    let idx = find_card(&b, CardId::StandbyFire);
    assert_eq!(b.card_cost(idx), CardId::StandbyFire.cost(false));
}

#[test]
fn t2_f13_standby_fire_upgraded_1bp_then_0bp_deals_10_dmg() {
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard {
            id: CardId::StandbyFire,
            upgraded: true,
        }],
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let sf_upg = CardId::StandbyFire.value(true);
    assert!(sf_upg > CardId::StandbyFire.value(false));
    assert_eq!(b.card_cost(0), CardId::StandbyFire.cost(true));
    b.end_turn();
    assert_eq!(b.card_cost(0), 0);
    play_upg(&mut b, CardId::StandbyFire, true, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 40 - sf_upg);
}

#[test]
fn t2_f13_chain_command_before_station_0bp_gain_and_after_station_1bp_gain() {
    let cc_cost = CardId::ChainCommand.cost(false);
    let cc_gain = CardId::ChainCommand.extra(false) as u32;
    let cc_draw = CardId::ChainCommand.value(false) as usize;
    let mut deck = vec![
        DeckCard::new(CardId::ChainCommand),
        DeckCard::new(CardId::ChainCommand),
    ];
    deck.extend(vec![DeckCard::new(CardId::Torpedo); 3 + cc_draw]);
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        &[CardId::ChainCommand],
    );
    b.points = 2 * cc_cost + 2;
    let start = b.points;
    play(&mut b, CardId::ChainCommand, Target::None);
    assert_eq!(b.points, start - cc_cost);
    b.run_station(Station::Shields, Target::None).unwrap();
    // 5 initial + cc_draw draws the entire deck, so 2nd copy is guaranteed in hand
    play(&mut b, CardId::ChainCommand, Target::None);
    assert_eq!(b.points, start - 2 * cc_cost + cc_gain);
}

#[test]
fn t2_f13_chain_command_after_micro_jump_grants_0bp() {
    let cc_cost = CardId::ChainCommand.cost(false);
    let deck = vec![
        DeckCard::new(CardId::MicroJump),
        DeckCard::new(CardId::ChainCommand),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(40, 0, 0)),
        vec![],
        &[CardId::MicroJump, CardId::ChainCommand],
    );
    b.points = 3 + cc_cost + 1;
    play(&mut b, CardId::MicroJump, Target::None);
    play(&mut b, CardId::ChainCommand, Target::None);
    assert_eq!(b.points, 1);
}

#[test]
fn t2_f13_autocannon_salvo_3_hits_homing_per_hit_fire_control_and_armor_interaction() {
    let deck = vec![
        DeckCard::new(CardId::FireControlDoctrine),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(unit(EnemyId::Raiders, 50, 1, 1, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let ac_val = CardId::AutocannonSalvo.value(false);
    let ac_hits = CardId::AutocannonSalvo.extra(false);
    let per_hit_base = ac_val + 1;
    let per_hit_hull = (per_hit_base - 1).max(0);
    play(&mut b, CardId::FireControlDoctrine, Target::None);
    let idx = find_card(&b, CardId::AutocannonSalvo);
    let ev = assert_preview_matches_execution(&mut b, idx, Target::Enemy(uid));
    let bds = hit_breakdowns(&ev, uid);
    assert_eq!(bds.len(), ac_hits as usize);
    for bd in bds {
        assert_eq!(bd.base, per_hit_base);
        assert_eq!(bd.evaded, 0);
        assert_eq!(bd.armor, 1);
        assert_eq!(bd.hull, per_hit_hull);
    }
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - ac_hits * per_hit_hull);
}

#[test]
fn t2_f13_target_painter_on_covered_and_shielded_and_zero_armor_enemy() {
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [Some(dummy_front(50, 1, 0)), None, None],
        ..Encounter::default()
    };
    let mut b = battle_with(
        AdmiralId::Osei,
        vec![DeckCard::new(CardId::TargetPainter)],
        enc,
        vec![],
        1,
    );
    let back_uid = b.enemies[1].uid;
    b.enemies[1].shield = 10;
    play(&mut b, CardId::TargetPainter, Target::Enemy(back_uid));
    assert!(b.enemy(back_uid).unwrap().marked);
    assert_eq!(b.enemy(back_uid).unwrap().armor, 0);
    assert_eq!(b.enemy(back_uid).unwrap().shield, 10);
}

#[test]
fn t2_f13_emergency_power_at_0bp_enables_3bp_card_and_reshuffles_when_upgraded() {
    let gain = CardId::EmergencyPower.value(true) as u32;
    let deck = vec![
        DeckCard {
            id: CardId::EmergencyPower,
            upgraded: true,
        },
        DeckCard::new(CardId::OrbitalStrike),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 3 - gain;
    // Below 3 BP, OrbitalStrike (3 BP) is unplayable; EmergencyPower raises BP to 3
    play_upg(&mut b, CardId::EmergencyPower, true, Target::None);
    assert_eq!(b.points, 3);
    play(&mut b, CardId::OrbitalStrike, Target::Enemy(uid));
    assert_eq!(b.points, 0);
    assert_eq!(b.enemy(uid).unwrap().hull, 34);
}

// --- F14 Boundaries: Build Doctrines (8 tests) ---

#[test]
fn t2_f14_bridge_coordination_triggers_on_all_5_stations_and_red_alert_2nd_station() {
    let draw = CardId::BridgeCoordination.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::RedAlert),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(100, 0, 0)),
        vec![],
        &[CardId::BridgeCoordination, CardId::RedAlert],
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::BridgeCoordination, Target::None);
    play(&mut b, CardId::RedAlert, Target::None);
    let before = b.hand.len();
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.hand.len(), before + draw);
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), before + 2 * draw);
    b.end_turn();
    let before_r2 = b.hand.len();
    b.run_station(Station::DamageControl, Target::None).unwrap();
    assert_eq!(b.hand.len(), before_r2 + draw);
    b.end_turn();
    let before_r3 = b.hand.len();
    b.run_station(Station::Maneuver, Target::Column(0)).unwrap();
    assert_eq!(b.hand.len(), before_r3 + draw);
    b.end_turn();
    let before_r4 = b.hand.len();
    b.run_station(Station::Admiral, Target::None).unwrap();
    assert_eq!(b.hand.len(), before_r4 + draw);
}

#[test]
fn t2_f14_bridge_coordination_does_not_trigger_on_micro_jump() {
    let deck = vec![
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::MicroJump),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(100, 0, 0)),
        vec![],
        &[CardId::BridgeCoordination, CardId::MicroJump],
    );
    b.points = 10;
    play(&mut b, CardId::BridgeCoordination, Target::None);
    let before = b.hand.len();
    play(&mut b, CardId::MicroJump, Target::None);
    assert_eq!(b.hand.len(), before - 1);
}

#[test]
fn t2_f14_bridge_coordination_triggers_reshuffle_when_draw_pile_empty() {
    let deck = vec![
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::EmergencyShields),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    play(&mut b, CardId::BridgeCoordination, Target::None);
    play(&mut b, CardId::EmergencyShields, Target::None);
    assert!(b.hand.is_empty());
    assert!(b.draw_pile.is_empty());
    assert_eq!(b.discard_pile.len(), 1);
    let ev = b.run_station(Station::Shields, Target::None).unwrap();
    assert!(ev.iter().any(|e| matches!(e, BattleEvent::Shuffled)));
    assert_eq!(b.hand, vec![DeckCard::new(CardId::EmergencyShields)]);
}

#[test]
fn t2_f14_assault_doctrine_ignores_shields_dc_maneuver_non_vance_admiral_and_micro_jump() {
    for (admiral, station, tgt) in [
        (AdmiralId::Osei, Station::Admiral, Target::None),
        (AdmiralId::Kerr, Station::Admiral, Target::Enemy(1)),
        (AdmiralId::Reyes, Station::Admiral, Target::None),
    ] {
        let mut b = battle_with(
            admiral,
            vec![DeckCard::new(CardId::AssaultDoctrine)],
            encounter_single(dummy_front(50, 0, 0)),
            vec![],
            1,
        );
        play(&mut b, CardId::AssaultDoctrine, Target::None);
        b.run_station(station, tgt).unwrap();
        let expected_shield = if admiral == AdmiralId::Osei {
            rules().stations.counterfire_shield
        } else {
            0
        };
        assert_eq!(b.flagship.shield, expected_shield, "{admiral:?}");
    }
    let mut bk = battle_with(
        AdmiralId::Kerr,
        vec![
            DeckCard::new(CardId::AssaultDoctrine),
            DeckCard::new(CardId::MicroJump),
        ],
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    bk.points = 10;
    play(&mut bk, CardId::AssaultDoctrine, Target::None);
    play(&mut bk, CardId::MicroJump, Target::None);
    assert_eq!(bk.flagship.shield, 0);
}

#[test]
fn t2_f14_bastion_doctrine_upgraded_1bp_ignores_battle_stations_and_shields_station() {
    let upg_cost = CardId::BastionDoctrine.cost(true);
    assert!(upg_cost < CardId::BastionDoctrine.cost(false));
    let deck = vec![
        DeckCard {
            id: CardId::BastionDoctrine,
            upgraded: true,
        },
        DeckCard::new(CardId::BattleStations),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    b.points = upg_cost + 1;
    play_upg(&mut b, CardId::BastionDoctrine, true, Target::None);
    assert_eq!(b.points, 1);
    play(&mut b, CardId::BattleStations, Target::None);
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.flagship.shield, 11);
    assert_eq!(b.flagship.hold_the_line, 0);
}

#[test]
fn t2_f14_clockwork_doctrine_upgraded_1bp_multiple_copies_stack_draw_and_discount() {
    let deck = vec![
        DeckCard {
            id: CardId::ClockworkDoctrine,
            upgraded: true,
        },
        DeckCard {
            id: CardId::ClockworkDoctrine,
            upgraded: true,
        },
        DeckCard::new(CardId::RadioSilence),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = (1..20_000)
        .map(|seed| {
            battle_with(
                AdmiralId::Kerr,
                deck.clone(),
                encounter_single(dummy_front(50, 0, 0)),
                vec![],
                seed,
            )
        })
        .find(|b| {
            b.hand
                .iter()
                .filter(|c| c.id == CardId::ClockworkDoctrine)
                .count()
                == 2
                && b.hand.iter().any(|c| c.id == CardId::RadioSilence)
        })
        .expect("must draw 2 ClockworkDoctrine + RadioSilence in Round 1");
    b.points = 10;
    play_upg(&mut b, CardId::ClockworkDoctrine, true, Target::None);
    play_upg(&mut b, CardId::ClockworkDoctrine, true, Target::None);
    assert_eq!(
        b.flagship.clockwork_draw,
        2 * CardId::ClockworkDoctrine.value(true) as usize
    );
    let discount = 2 * CardId::ClockworkDoctrine.extra(true) as u32;
    let rs_idx = find_card(&b, CardId::RadioSilence);
    assert_eq!(b.card_cost(rs_idx), 3u32.saturating_sub(discount));
    b.end_turn();
    assert_eq!(b.hand.len(), 6);
}

#[test]
fn t2_f14_ember_doctrine_upgraded_1bp_multiple_copies_stack_burn_and_kill_shield() {
    let eb = CardId::EmberDoctrine.value(true);
    let ek = CardId::EmberDoctrine.extra(true);
    let enc = Encounter {
        front: [
            Some(dummy_front(4, 0, 0)),
            Some(dummy_front(50, 0, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard {
            id: CardId::EmberDoctrine,
            upgraded: true,
        },
        DeckCard {
            id: CardId::EmberDoctrine,
            upgraded: true,
        },
        DeckCard::new(CardId::Firebomb),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let uid = b.enemies[0].uid;
    b.enemies[0].burning = true;
    b.points = 10;
    play_upg(&mut b, CardId::EmberDoctrine, true, Target::None);
    play_upg(&mut b, CardId::EmberDoctrine, true, Target::None);
    assert_eq!(
        b.burn_damage(b.enemy(uid).unwrap().unit()),
        rules().battle.ember_burn_damage + 2 * eb
    );
    play(&mut b, CardId::Firebomb, Target::Enemy(uid));
    assert_eq!(b.flagship.shield, 2 * ek);
}

#[test]
fn t2_f14_all_5_doctrines_active_simultaneously_stack_cleanly() {
    let assault_s = CardId::AssaultDoctrine.value(false);
    let bridge_d = CardId::BridgeCoordination.value(false) as usize;
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::AssaultDoctrine),
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::EmergencyShields),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(80, 0, 0)),
        vec![],
        &[
            CardId::BastionDoctrine,
            CardId::AssaultDoctrine,
            CardId::ClockworkDoctrine,
            CardId::EmberDoctrine,
            CardId::BridgeCoordination,
        ],
    );
    let uid = b.enemies[0].uid;
    b.points = 20;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::AssaultDoctrine, Target::None);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    play(&mut b, CardId::EmberDoctrine, Target::None);
    play(&mut b, CardId::BridgeCoordination, Target::None);
    assert_eq!(b.doctrines.len(), 5);
    let before = b.hand.len();
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, assault_s);
    assert_eq!(b.hand.len(), before + bridge_d);
}

// ============================================================================
// TIER 3 — CROSS-FEATURE COMBINATIONS / PAIRWISE (18 tests)
// ============================================================================

#[test]
fn t3_time_window_plus_shield_bash() {
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::ShieldBash),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(unit(EnemyId::Raiders, 50, 1, 0, vec![Step::Evade])),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    let total_shield = CardId::Bulwark.value(false) + CardId::BastionDoctrine.value(false);
    b.points = 20;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::Bulwark, Target::None);
    assert_eq!(b.flagship.shield, total_shield);
    play(&mut b, CardId::TimeWindow, Target::None);
    let ev = play(&mut b, CardId::ShieldBash, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 2);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 2 * total_shield);
}

#[test]
fn t3_time_window_plus_flak_breaker() {
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::FlakBreaker),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(50, 1, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    let ev = play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 4);
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
}

#[test]
fn t3_time_window_plus_breakthrough_volley() {
    let bv_val = CardId::BreakthroughVolley.value(false);
    let bv_cost = CardId::BreakthroughVolley.cost(false);
    let bv_refund = CardId::BreakthroughVolley.extra(false) as u32;
    let tw_cost = CardId::TimeWindow.cost(false);
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::BreakthroughVolley),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
        DeckCard::new(CardId::Torpedo),
    ];
    // Survives hit 1 (bv_val), dies on hit 2 (2 * bv_val)
    let enc = Encounter {
        front: [
            Some(dummy_front(2 * bv_val - 1, 1, 0)),
            Some(dummy_front(40, 1, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_requiring_hand(
        AdmiralId::Vance,
        deck,
        enc,
        vec![],
        &[CardId::TimeWindow, CardId::BreakthroughVolley],
    );
    let uid = b.enemies[0].uid;
    b.points = tw_cost + bv_cost + 1;
    play(&mut b, CardId::TimeWindow, Target::None);
    let ev = play(&mut b, CardId::BreakthroughVolley, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), 2);
    assert!(b.enemy(uid).is_none());
    assert_eq!(b.points, 1 + bv_refund);
    assert_eq!(b.hand.len(), 4);
}

#[test]
fn t3_time_window_plus_precision_lock() {
    let draw = CardId::PrecisionLock.value(false) as usize;
    let mut deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::PrecisionLock),
    ];
    deck.extend(vec![DeckCard::new(CardId::Torpedo); 3 + 2 * draw]);
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 1, 0)),
        vec![],
        &[CardId::TimeWindow, CardId::PrecisionLock],
    );
    let uid = b.enemies[0].uid;
    let before = b.hand.len();
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    assert!(b.enemy(uid).unwrap().marked);
    assert_eq!(b.hand.len(), before - 2 + 2 * draw);
}

#[test]
fn t3_time_window_plus_wildfire() {
    let wv = CardId::Wildfire.value(false);
    let enc = Encounter {
        front: [Some(dummy_front(40, 1, 0)), None, None],
        back: [Some(dummy_front(40, 1, 0)), None, None],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::Wildfire),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    b.points = 10;
    play(&mut b, CardId::TimeWindow, Target::None);
    play(&mut b, CardId::Wildfire, Target::None);
    for e in &b.enemies {
        assert_eq!(e.hull, 40 - 2 * wv);
        assert!(e.burning);
    }
}

#[test]
fn t3_time_window_plus_standby_fire() {
    let tw_cost = CardId::TimeWindow.cost(false);
    let sf_upg = CardId::StandbyFire.value(true);
    let deck = vec![
        DeckCard::new(CardId::TimeWindow),
        DeckCard {
            id: CardId::StandbyFire,
            upgraded: true,
        },
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.end_turn();
    b.points = tw_cost + 1;
    play(&mut b, CardId::TimeWindow, Target::None);
    assert_eq!(b.points, 1);
    play_upg(&mut b, CardId::StandbyFire, true, Target::Enemy(uid));
    assert_eq!(b.points, 1);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - 2 * sf_upg);
}

#[test]
fn t3_marked_plus_autocannon_plus_vance_momentum_plus_fire_control() {
    let ac_val = CardId::AutocannonSalvo.value(false);
    let ac_hits = CardId::AutocannonSalvo.extra(false);
    let per_hit = ac_val + 1 + half_up(ac_val + 1);
    let deck = vec![
        DeckCard::new(CardId::FireControlDoctrine),
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(50, 0, 1)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.flagship.momentum = 3;
    play(&mut b, CardId::FireControlDoctrine, Target::None);
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
    assert!(b.enemy(uid).unwrap().marked);
    let ev = play(&mut b, CardId::AutocannonSalvo, Target::Enemy(uid));
    assert_eq!(hit_breakdowns(&ev, uid).len(), ac_hits as usize);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - ac_hits * per_hit);
}

#[test]
fn t3_marked_plus_follow_up_plus_burn_cards() {
    let mv = CardId::MeltdownSalvo.value(false);
    let me = CardId::MeltdownSalvo.extra(false);
    let follow_dmg = 2 * rules().battle.ember_burn_damage;
    let card_base = mv + me + rules().stations.follow_up_card_bonus;
    let marked_dmg = card_base + half_up(card_base);
    let deck = vec![
        DeckCard::new(CardId::Firebomb),
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::MeltdownSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Reyes,
        deck,
        encounter_single(dummy_front(80, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::Firebomb, Target::Enemy(uid));
    b.run_station(Station::Admiral, Target::None).unwrap();
    play(&mut b, CardId::PrecisionLock, Target::Enemy(uid));
    play(&mut b, CardId::MeltdownSalvo, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - 4 - follow_dmg - marked_dmg);
}

#[test]
fn t3_bastion_doctrine_plus_bulwark_plus_hold_the_line_plus_shield_bash() {
    let bs = CardId::BastionDoctrine.value(false);
    let bc = CardId::BastionDoctrine.extra(false);
    let bul = CardId::Bulwark.value(false);
    let expected_shields = (bul + bs) + (4 + bs);
    let expected_retal = bc + (2 + bc);
    let deck = vec![
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::HoldTheLine),
        DeckCard::new(CardId::ShieldBash),
    ];
    let enc = Encounter {
        front: [
            Some(dummy_front(50, 3, 0)),
            Some(dummy_front(50, 3, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    let u0 = b.enemies[0].uid;
    let u1 = b.enemies[1].uid;
    b.points = 15;
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::Bulwark, Target::None);
    play(&mut b, CardId::HoldTheLine, Target::None);
    assert_eq!(b.flagship.shield, expected_shields);
    assert_eq!(b.flagship.hold_the_line, expected_retal);
    play(&mut b, CardId::ShieldBash, Target::Enemy(u0));
    assert_eq!(b.enemy(u0).unwrap().hull, 50 - expected_shields);
    b.end_turn();
    assert_eq!(
        b.enemy(u0).unwrap().hull,
        50 - expected_shields - expected_retal
    );
    assert_eq!(b.enemy(u1).unwrap().hull, 50 - expected_retal);
    assert_eq!(b.flagship.hull, AdmiralId::Osei.hull());
}

#[test]
fn t3_ember_doctrine_plus_wildfire_plus_accelerant_plus_meltdown_salvo() {
    let wv = CardId::Wildfire.value(false);
    let eb = CardId::EmberDoctrine.value(false);
    let acc_mult = CardId::Accelerant.value(false);
    let acc_dmg = acc_mult * (rules().battle.ember_burn_damage + eb);
    let ms_base = CardId::MeltdownSalvo.value(false) + 2 * CardId::MeltdownSalvo.extra(false);
    let ms_dmg = ms_base + half_up(ms_base);
    let enc = Encounter {
        front: [
            Some(dummy_front(60, 0, 0)),
            Some(dummy_front(60, 0, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::Accelerant),
        DeckCard::new(CardId::MeltdownSalvo),
    ];
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 1);
    let u0 = b.enemies[0].uid;
    b.points = 15;
    play(&mut b, CardId::EmberDoctrine, Target::None);
    play(&mut b, CardId::Wildfire, Target::None);
    play(&mut b, CardId::Accelerant, Target::Enemy(u0));
    assert_eq!(b.enemy(u0).unwrap().hull, 60 - wv - acc_dmg);
    play(&mut b, CardId::MeltdownSalvo, Target::Enemy(u0));
    assert_eq!(b.enemy(u0).unwrap().hull, 60 - wv - acc_dmg - ms_dmg);
}

#[test]
fn t3_clockwork_doctrine_plus_chain_command_plus_bridge_coordination_plus_red_alert() {
    let cd_cost = CardId::ClockworkDoctrine.cost(false);
    let cd_disc = CardId::ClockworkDoctrine.extra(false) as u32;
    let bc_cost = CardId::BridgeCoordination.cost(false);
    let bc_draw = CardId::BridgeCoordination.value(false) as usize;
    let cc_cost = CardId::ChainCommand.cost(false);
    let cc_draw = CardId::ChainCommand.value(false) as usize;
    let cc_gain = CardId::ChainCommand.extra(false) as u32;
    let mut deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::BridgeCoordination),
        DeckCard::new(CardId::RedAlert),
        DeckCard::new(CardId::ChainCommand),
    ];
    deck.extend(vec![
        DeckCard::new(CardId::Torpedo);
        4 + 2 * bc_draw + cc_draw
    ]);
    let mut b = battle_requiring_hand(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(80, 0, 0)),
        vec![],
        &[
            CardId::ClockworkDoctrine,
            CardId::BridgeCoordination,
            CardId::RedAlert,
            CardId::ChainCommand,
        ],
    );
    let uid = b.enemies[0].uid;
    b.points = 20;
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    play(&mut b, CardId::BridgeCoordination, Target::None);
    let ra_cost = 2u32.saturating_sub(cd_disc);
    assert_eq!(b.card_cost(find_card(&b, CardId::RedAlert)), ra_cost);
    play(&mut b, CardId::RedAlert, Target::None);
    let after_ra = 20 - cd_cost - bc_cost - ra_cost;
    assert_eq!(b.points, after_ra);
    let before = b.hand.len();
    b.run_station(Station::Weapons, Target::Enemy(uid)).unwrap();
    b.run_station(Station::Shields, Target::None).unwrap();
    assert_eq!(b.hand.len(), before + 2 * bc_draw);
    play(&mut b, CardId::ChainCommand, Target::None);
    assert_eq!(b.points, after_ra - cc_cost + cc_gain);
    assert_eq!(b.hand.len(), before + 2 * bc_draw - 1 + cc_draw);
}

#[test]
fn t3_clockwork_doctrine_plus_emergency_power_plus_standby_fire() {
    let ep_gain = CardId::EmergencyPower.value(false) as u32;
    let sf_val = CardId::StandbyFire.value(false);
    let deck = vec![
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::EmergencyPower),
        DeckCard::new(CardId::StandbyFire),
    ];
    let mut b = battle_with(
        AdmiralId::Kerr,
        deck,
        encounter_single(dummy_front(50, 0, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = CardId::ClockworkDoctrine.cost(false);
    play(&mut b, CardId::ClockworkDoctrine, Target::None);
    b.end_turn();
    assert!(b.is_retained(find_card(&b, CardId::StandbyFire)));
    assert_eq!(b.card_cost(find_card(&b, CardId::StandbyFire)), 0);
    play(&mut b, CardId::EmergencyPower, Target::None);
    assert_eq!(b.points, 3 + ep_gain);
    assert_eq!(b.spent.len(), 1);
    play(&mut b, CardId::StandbyFire, Target::Enemy(uid));
    assert_eq!(b.points, 3 + ep_gain);
    assert_eq!(b.enemy(uid).unwrap().hull, 50 - sf_val);
}

#[test]
fn t3_panzerbruch_plus_autocannon_salvo_plus_marked() {
    let fb_val = CardId::FlakBreaker.value(false);
    let fb_hit = ((fb_val + half_up(fb_val)) - 1).max(0);
    let fb_dmg = 2 * fb_hit;
    let ac_val = CardId::AutocannonSalvo.value(false);
    let ac_hits = CardId::AutocannonSalvo.extra(false);
    let ac_dmg = ac_hits * (ac_val + half_up(ac_val));
    let deck = vec![
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::FlakBreaker),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(
        AdmiralId::Osei,
        deck,
        encounter_single(dummy_front(60, 0, 2)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.points = 10;
    play(&mut b, CardId::TargetPainter, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 1);
    assert!(b.enemy(uid).unwrap().marked);
    play(&mut b, CardId::FlakBreaker, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().armor, 0);
    assert_eq!(b.enemy(uid).unwrap().hull, 60 - fb_dmg);
    play(&mut b, CardId::AutocannonSalvo, Target::Enemy(uid));
    assert_eq!(b.enemy(uid).unwrap().hull, 60 - fb_dmg - ac_dmg);
}

#[test]
fn t3_assault_doctrine_plus_vance_focus_fire_plus_countermeasures() {
    let s = CardId::AssaultDoctrine.value(false);
    let cm_hit = half_up(10 + rules().stations.focus_fire_vulnerability);
    let expected_hull_loss = (cm_hit - s).max(0);
    let deck = vec![
        DeckCard::new(CardId::AssaultDoctrine),
        DeckCard::new(CardId::Countermeasures),
    ];
    let mut b = battle_with(
        AdmiralId::Vance,
        deck,
        encounter_single(dummy_front(80, 10, 0)),
        vec![],
        1,
    );
    let uid = b.enemies[0].uid;
    b.flagship.momentum = 2;
    play(&mut b, CardId::AssaultDoctrine, Target::None);
    play(&mut b, CardId::Countermeasures, Target::None);
    b.run_station(Station::Admiral, Target::Enemy(uid)).unwrap();
    assert_eq!(b.flagship.shield, s);
    assert_eq!(b.enemy(uid).unwrap().hull, 80 - (16 + 4));
    b.end_turn();
    assert_eq!(
        b.flagship.hull,
        AdmiralId::Vance.hull() - expected_hull_loss
    );
}

#[test]
fn t3_kerr_stasis_plus_chrono_strike_plus_micro_jump() {
    let cs_val = CardId::ChronoStrike.value(false);
    let enc = Encounter {
        front: [
            Some(unit(
                EnemyId::GuardCruiser,
                50,
                10,
                0,
                vec![Step::Charge, Step::Fire],
            )),
            Some(dummy_front(50, 8, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::ChronoStrike),
        DeckCard::new(CardId::MicroJump),
    ];
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 1);
    let u0 = b.enemies[0].uid;
    b.run_station(Station::Admiral, Target::Enemy(u0)).unwrap();
    assert!(b.enemy(u0).unwrap().stasis);
    play(&mut b, CardId::ChronoStrike, Target::Enemy(u0));
    assert_eq!(b.enemy(u0).unwrap().hull, 50 - 2 * cs_val);
    b.end_turn();
    assert_eq!(b.flagship.hull, AdmiralId::Kerr.hull() - 8);
    play(&mut b, CardId::MicroJump, Target::None);
    assert_eq!(b.spent, vec![DeckCard::new(CardId::MicroJump)]);
    b.end_turn();
    assert_eq!(b.flagship.hull, AdmiralId::Kerr.hull() - 8);
}

#[test]
fn t3_cover_plus_tractor_beam_plus_marked_plus_piercing() {
    let ap_val = CardId::ArmorPiercer.value(false);
    let ap_dmg = ap_val + half_up(ap_val);
    let enc = Encounter {
        front: [Some(dummy_front(50, 1, 0)), None, None],
        back: [Some(dummy_front(50, 1, 0)), None, None],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::ArmorPiercer),
        DeckCard::new(CardId::TractorBeam),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    b.points = 10;
    play(&mut b, CardId::PrecisionLock, Target::Enemy(back_uid));
    play(&mut b, CardId::ArmorPiercer, Target::Enemy(back_uid));
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - ap_dmg);
    play(&mut b, CardId::TractorBeam, Target::Enemy(back_uid));
    assert!(!b.covered(back_uid));
    play(&mut b, CardId::Torpedo, Target::Enemy(back_uid));
    assert_eq!(b.enemy(back_uid).unwrap().hull, 50 - ap_dmg - 14);
}

#[test]
fn t3_breach_persistence_exhaust_doctrines_marked() {
    let seal = unit(EnemyId::Seal, 10, 4, 0, vec![Step::Fire]);
    let uc = unit(EnemyId::UnknownContact, 90, 6, 1, vec![Step::Fire]);
    let enc = Encounter {
        front: [Some(seal), None, None],
        breach: Some(uc),
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::EmergencyPower),
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::Torpedo),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    let seal_uid = b.enemies[0].uid;
    b.round = 8;
    b.points = 10;
    play(&mut b, CardId::EmergencyPower, Target::None);
    play(&mut b, CardId::BastionDoctrine, Target::None);
    play(&mut b, CardId::TargetPainter, Target::Enemy(seal_uid));
    let ev = play(&mut b, CardId::Torpedo, Target::Enemy(seal_uid));
    assert!(ev.iter().any(|e| matches!(e, BattleEvent::Breach)));
    assert!(b.result.is_none());
    assert_eq!(b.enemies.len(), 1);
    assert_eq!(b.enemies[0].id, EnemyId::UnknownContact);
    assert!(!b.enemies[0].marked);
    assert_eq!(b.spent, vec![DeckCard::new(CardId::EmergencyPower)]);
    assert_eq!(b.doctrines, vec![DeckCard::new(CardId::BastionDoctrine)]);
    assert_eq!(b.clock(), 0);
}

#[test]
fn t3_preview_vs_execution_complex_combo() {
    let enc = Encounter {
        front: [Some(dummy_front(60, 1, 0)), None, None],
        back: [
            Some(unit(EnemyId::Raiders, 60, 2, 2, vec![Step::Evade])),
            None,
            None,
        ],
        ..Encounter::default()
    };
    let deck = vec![
        DeckCard::new(CardId::FireControlDoctrine),
        DeckCard::new(CardId::TargetPainter),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::StandbyFire),
        DeckCard::new(CardId::AutocannonSalvo),
    ];
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 1);
    let back_uid = b.enemies[1].uid;
    b.points = 10;
    play(&mut b, CardId::FireControlDoctrine, Target::None);
    play(&mut b, CardId::TargetPainter, Target::Enemy(back_uid));
    play(&mut b, CardId::TimeWindow, Target::None);
    let idx = find_card(&b, CardId::StandbyFire);
    assert_preview_matches_execution(&mut b, idx, Target::Enemy(back_uid));
    let idx2 = find_card(&b, CardId::AutocannonSalvo);
    assert_preview_matches_execution(&mut b, idx2, Target::Enemy(back_uid));
}

// ============================================================================
// TIER 4 — REAL-WORLD APPLICATION SCENARIOS (7 tests)
// ============================================================================

fn incoming_damage_at(battle: &Battle, column: usize) -> i32 {
    if battle.flagship.jumped {
        return 0;
    }
    let armor = battle.flagship_armor();
    let mut hits: Vec<i32> = battle
        .enemies
        .iter()
        .filter_map(|e| {
            let intent = battle.announced(e.uid)[0];
            let dmg = match intent.action {
                Action::ColumnFire { columns } if !columns[column] => 0,
                _ => intent.damage,
            };
            (dmg > 0).then_some(dmg)
        })
        .collect();
    hits.sort_unstable_by(|a, b| b.cmp(a));
    let mut cm = battle.flagship.countermeasures as usize;
    hits.into_iter()
        .map(|mut d| {
            if cm > 0 {
                cm -= 1;
                d -= d / 2;
            }
            (d - armor).max(0)
        })
        .sum()
}

fn eval_turn_state(battle: &Battle) -> i64 {
    if battle.result == Some(BattleResult::Won) {
        return 1_000_000;
    }
    if battle.result == Some(BattleResult::Lost) {
        return -1_000_000;
    }
    let incoming = incoming_damage_at(battle, battle.flagship.column);
    let effective_shield = battle.flagship.shield + battle.flagship.battle_stations;
    let unblocked = (incoming - effective_shield).max(0);
    let enemy_hull: i32 = battle.enemies.iter().map(|e| e.hull + e.shield).sum();
    let uc_regen_penalty = battle
        .enemies
        .iter()
        .filter(|e| e.id == EnemyId::UnknownContact && !e.hit && !e.burning)
        .count() as i64
        * 40;
    let doc_bonus = (battle.doctrines.len() as i64) * 25
        + (battle.flagship.fire_control as i64) * 20
        + (battle.flagship.ablative as i64) * 25
        + (battle.flagship.battle_stations as i64) * 18;
    let retal_bonus = (battle.flagship.hold_the_line as i64) * 4;
    let burn_bonus = battle.enemies.iter().filter(|e| e.burning).count() as i64 * 12;
    let mark_bonus = battle.enemies.iter().filter(|e| e.marked).count() as i64 * 12;
    let armor_pen = battle.enemies.iter().map(|e| e.armor).sum::<i32>() as i64 * 8;
    let mom_bonus = (battle.flagship.momentum as i64) * 8;
    (battle.flagship.hull as i64) * 10
        - (unblocked as i64) * 12
        - (battle.flagship.overcharge as i64) * 10
        + (effective_shield.min(incoming) as i64) * 3
        - (enemy_hull as i64) * 5
        - (battle.enemies.len() as i64) * 35
        - uc_regen_penalty
        - armor_pen
        + doc_bonus
        + retal_bonus
        + burn_bonus
        + mark_bonus
        + mom_bonus
        + (battle.flagship.attack_bonus as i64) * 6
        + (battle.flagship.double_next as i64) * 35
        + (battle.hand.len() as i64) * 3
        + (battle.points as i64) * 5
}

type TurnAction = (bool, usize, Station, Target);

fn enumerate_actions(battle: &Battle) -> Vec<TurnAction> {
    let mut actions = Vec::new();
    for st in [
        Station::Weapons,
        Station::Shields,
        Station::Admiral,
        Station::DamageControl,
        Station::Maneuver,
    ] {
        if battle.station_open(st).is_err() {
            continue;
        }
        if st == Station::DamageControl
            && !battle.flagship.burning
            && (battle.flagship.hull >= battle.flagship.max_hull - 2
                || battle.flagship.repaired >= rules().battle.repair_limit)
        {
            continue;
        }
        let targets: Vec<Target> = match battle.station_target(st) {
            CardTarget::None => vec![Target::None],
            CardTarget::Enemy => battle
                .enemies
                .iter()
                .map(|e| Target::Enemy(e.uid))
                .collect(),
            CardTarget::Column => {
                let cur_in = incoming_damage_at(battle, battle.flagship.column);
                (0..COLUMNS)
                    .filter(|&c| {
                        c != battle.flagship.column && incoming_damage_at(battle, c) < cur_in
                    })
                    .map(Target::Column)
                    .collect()
            }
        };
        for tgt in targets {
            actions.push((true, 0, st, tgt));
        }
    }

    let mut tried_ids = HashSet::new();
    for (i, c) in battle.hand.iter().enumerate() {
        if battle.card_cost(i) > battle.points || !tried_ids.insert((c.id, c.upgraded)) {
            continue;
        }
        if c.id == CardId::CollisionCourse && battle.flagship.hull <= 18 {
            continue;
        }
        if c.id == CardId::Overcharge && battle.flagship.hull <= 16 {
            continue;
        }
        if c.id == CardId::RepairCrew
            && !battle.flagship.burning
            && (battle.flagship.hull >= battle.flagship.max_hull - 2
                || battle.flagship.repaired >= rules().battle.repair_limit)
        {
            continue;
        }
        let targets: Vec<Target> = match c.id.target() {
            CardTarget::None => vec![Target::None],
            CardTarget::Enemy => {
                if let Some(best) = battle
                    .enemies
                    .iter()
                    .max_by_key(|e| (!battle.covered(e.uid), battle.announced(e.uid)[0].damage))
                {
                    let mut list = vec![Target::Enemy(best.uid)];
                    for e in &battle.enemies {
                        if e.uid != best.uid && e.hull <= 20 {
                            list.push(Target::Enemy(e.uid));
                        }
                    }
                    list
                } else {
                    vec![]
                }
            }
            CardTarget::Column => {
                let cur_in = incoming_damage_at(battle, battle.flagship.column);
                (0..COLUMNS)
                    .filter(|&col| {
                        col != battle.flagship.column && incoming_damage_at(battle, col) < cur_in
                    })
                    .map(Target::Column)
                    .collect()
            }
        };
        for tgt in targets {
            actions.push((false, i, Station::Weapons, tgt));
        }
    }
    actions
}

fn pick_best_action(battle: &Battle) -> Option<TurnAction> {
    let base_score = eval_turn_state(battle);
    let mut ply1: Vec<(i64, TurnAction, Battle)> = Vec::new();
    for act in enumerate_actions(battle) {
        let mut next = battle.clone();
        let ok = if act.0 {
            next.run_station(act.2, act.3).is_ok()
        } else {
            next.play_card(act.1, act.3).is_ok()
        };
        if ok {
            ply1.push((eval_turn_state(&next), act, next));
        }
    }
    ply1.sort_unstable_by_key(|a| std::cmp::Reverse(a.0));
    let mut best_score = base_score;
    let mut best_act = None;
    for (s1, act1, state1) in ply1.into_iter().take(6) {
        if s1 > best_score {
            best_score = s1;
            best_act = Some(act1);
        }
        if state1.result.is_some() {
            continue;
        }
        for act2 in enumerate_actions(&state1) {
            let mut state2 = state1.clone();
            let ok2 = if act2.0 {
                state2.run_station(act2.2, act2.3).is_ok()
            } else {
                state2.play_card(act2.1, act2.3).is_ok()
            };
            if ok2 {
                let s2 = eval_turn_state(&state2);
                if s2 > best_score {
                    best_score = s2;
                    best_act = Some(act1);
                }
            }
        }
    }
    best_act
}

fn drive_battle_to_completion(battle: &mut Battle, max_rounds: u32) {
    while battle.result.is_none() && battle.round <= max_rounds {
        for _ in 0..16 {
            if battle.result.is_some() {
                break;
            }
            let Some((is_station, idx, st, tgt)) = pick_best_action(battle) else {
                break;
            };
            if is_station {
                let _ = battle.run_station(st, tgt);
            } else {
                let _ = battle.play_card(idx, tgt);
            }
        }
        if battle.result.is_some() {
            break;
        }
        battle.end_turn();
    }
}

#[test]
fn t4_osei_fortress_multi_turn_battle() {
    let enc = Encounter {
        front: [
            Some(dummy_front(26, 4, 2)),
            Some(dummy_front(24, 4, 1)),
            None,
        ],
        back: [
            Some(dummy_front(20, 4, 1)),
            Some(dummy_front(20, 3, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut deck = starter_deck(AdmiralId::Osei);
    deck.extend([
        DeckCard::new(CardId::BastionDoctrine),
        DeckCard::new(CardId::Bulwark),
        DeckCard::new(CardId::FlakBreaker),
        DeckCard::new(CardId::ShieldBash),
    ]);
    let mut b = battle_with(AdmiralId::Osei, deck, enc, vec![], 101);
    drive_battle_to_completion(&mut b, 20);
    assert_eq!(b.result, Some(BattleResult::Won));
    assert_eq!(b.kills, 4);
    assert!(b.flagship.hull > 0);
}

#[test]
fn t4_vance_momentum_burst_multi_turn_battle() {
    let enc = Encounter {
        front: [
            Some(dummy_front(22, 4, 1)),
            Some(dummy_front(28, 4, 2)),
            None,
        ],
        back: [Some(dummy_front(24, 4, 1)), None, None],
        ..Encounter::default()
    };
    let mut deck = starter_deck(AdmiralId::Vance);
    deck.extend([
        DeckCard::new(CardId::AssaultDoctrine),
        DeckCard::new(CardId::Onslaught),
        DeckCard::new(CardId::ArmorPiercer),
        DeckCard::new(CardId::BreakthroughVolley),
    ]);
    let mut b = battle_with(AdmiralId::Vance, deck, enc, vec![], 202);
    drive_battle_to_completion(&mut b, 20);
    assert_eq!(b.result, Some(BattleResult::Won));
    assert_eq!(b.kills, 3);
}

#[test]
fn t4_kerr_clockwork_combo_multi_turn_battle() {
    let enc = Encounter {
        front: [
            Some(unit(
                EnemyId::GuardCruiser,
                30,
                5,
                1,
                vec![Step::Charge, Step::Fire],
            )),
            Some(unit(
                EnemyId::Raiders,
                18,
                4,
                0,
                vec![Step::Evade, Step::Fire],
            )),
            None,
        ],
        back: [Some(dummy_front(22, 4, 0)), None, None],
        ..Encounter::default()
    };
    let mut deck = starter_deck(AdmiralId::Kerr);
    deck.extend([
        DeckCard::new(CardId::ClockworkDoctrine),
        DeckCard::new(CardId::PrecisionLock),
        DeckCard::new(CardId::TimeWindow),
        DeckCard::new(CardId::ChronoStrike),
    ]);
    let mut b = battle_with(AdmiralId::Kerr, deck, enc, vec![], 303);
    drive_battle_to_completion(&mut b, 20);
    assert_eq!(b.result, Some(BattleResult::Won));
}

#[test]
fn t4_reyes_inferno_multi_turn_battle() {
    let enc = Encounter {
        front: [
            Some(dummy_front(26, 4, 1)),
            Some(dummy_front(26, 4, 1)),
            None,
        ],
        back: [
            Some(dummy_front(24, 3, 0)),
            Some(dummy_front(24, 3, 0)),
            None,
        ],
        ..Encounter::default()
    };
    let mut deck = starter_deck(AdmiralId::Reyes);
    deck.extend([
        DeckCard::new(CardId::EmberDoctrine),
        DeckCard::new(CardId::Wildfire),
        DeckCard::new(CardId::Accelerant),
        DeckCard::new(CardId::MeltdownSalvo),
    ]);
    let mut b = battle_with(AdmiralId::Reyes, deck, enc, vec![], 404);
    drive_battle_to_completion(&mut b, 20);
    assert_eq!(b.result, Some(BattleResult::Won));
    assert_eq!(b.kills, 4);
}

fn simulate_full_run(
    admiral: AdmiralId,
    seed: u64,
    maps: &[SectorMap; SECTORS as usize],
) -> (BattleResult, bool, HashSet<NodeKind>, u32) {
    let mut run = Run::new(admiral, seed, maps.clone());
    let mut saw_nodes = HashSet::new();
    let mut saw_breach = false;
    while run.result.is_none() {
        match &run.stage {
            Stage::Map => {
                let choices = run.choices();
                let row = run.current().map_or(0, |(r, _)| r + 1);
                let col = *choices
                    .iter()
                    .max_by_key(|&&c| {
                        let kind = run.map[row][c].kind;
                        let allow_elite = run.sector >= 2 && run.hull >= run.max_hull * 75 / 100;
                        let priority = match kind {
                            NodeKind::Shipyard if run.hull < run.max_hull * 80 / 100 => 7,
                            NodeKind::Elite if allow_elite && !saw_nodes.contains(&kind) => 6,
                            NodeKind::Market if run.scrap >= 120 => 5,
                            NodeKind::Event => 4,
                            NodeKind::Shipyard => 4,
                            NodeKind::Battle => 3,
                            NodeKind::Market => 2,
                            NodeKind::Elite => 1,
                            NodeKind::Boss => 0,
                        };
                        (priority, !saw_nodes.contains(&kind) as u8)
                    })
                    .unwrap();
                let kind = run.map[row][col].kind;
                saw_nodes.insert(kind);
                run.advance(col).unwrap();
            }
            Stage::Battle { .. } => {
                let mut battle = run.start_battle();
                let had_breach = battle.breach.is_some();
                drive_battle_to_completion(&mut battle, 40);
                if had_breach && battle.result == Some(BattleResult::Won) {
                    saw_breach = true;
                }
                if battle.result.is_none() {
                    return (BattleResult::Lost, saw_breach, saw_nodes, run.sector);
                }
                run.finish_battle(&battle);
            }
            Stage::Loot(loot) => {
                if !loot.cards.is_empty() {
                    let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
                    assert!(loot.cards.iter().any(|c| pool.contains(c)));
                    let pick = loot
                        .cards
                        .iter()
                        .position(|c| pool.contains(c))
                        .unwrap_or(0);
                    run.take_loot(Some(pick)).unwrap();
                } else if !loot.modules.is_empty() {
                    run.take_loot(Some(0)).unwrap();
                } else {
                    run.take_loot(None).unwrap();
                }
            }
            Stage::Shipyard => {
                if run.hull < run.max_hull * 85 / 100 {
                    run.repair_at_shipyard().unwrap();
                } else if let Some(idx) = run.deck.iter().position(|c| !c.upgraded) {
                    run.upgrade_at_shipyard(idx).unwrap();
                } else {
                    run.repair_at_shipyard().unwrap();
                }
            }
            Stage::Market(m) => {
                let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
                assert!(m.cards.iter().any(|(c, _)| pool.contains(c)));
                if let Some((_, price)) = m.modules.first().copied()
                    && run.scrap >= price
                {
                    run.buy_module(0).unwrap();
                }
                let cards = match &run.stage {
                    Stage::Market(mkt) => mkt.cards.clone(),
                    _ => vec![],
                };
                if let Some((idx, (_, price))) = cards
                    .iter()
                    .enumerate()
                    .find(|(_, (c, _))| pool.contains(c))
                    && run.scrap >= *price
                {
                    run.buy_card(idx).unwrap();
                }
                while run.hull <= run.max_hull - 10 && run.scrap >= rules().run.market_repair_price
                {
                    run.buy_repair().unwrap();
                }
                run.leave_market().unwrap();
            }
            Stage::Event(ev_id) => {
                let opt = match ev_id {
                    voidfleet_core::run::EventId::Resolute => 1,
                    voidfleet_core::run::EventId::Pilot => {
                        if run.scrap >= rules().run.pilot_price {
                            0
                        } else {
                            1
                        }
                    }
                    voidfleet_core::run::EventId::Flicker => 1,
                };
                let _ = run.choose(opt).unwrap();
            }
        }
    }
    (run.result.unwrap(), saw_breach, saw_nodes, run.sector)
}

#[test]
fn t4_full_3_sector_run_osei_and_vance() {
    let maps = load_sector_maps();
    for map in &maps {
        map.check().expect("sector map valid");
    }
    for admiral in [AdmiralId::Osei, AdmiralId::Vance] {
        let mut max_sector = 0;
        let mut wins = 0;
        let mut breaches = 0;
        let win = (1..=100).find_map(|seed| {
            let r = simulate_full_run(admiral, seed, &maps);
            max_sector = max_sector.max(r.3);
            if r.0 == BattleResult::Won {
                wins += 1;
            }
            if r.1 {
                breaches += 1;
            }
            (r.0 == BattleResult::Won
                && r.1
                && r.2.contains(&NodeKind::Elite)
                && r.2.contains(&NodeKind::Shipyard)
                && r.2.contains(&NodeKind::Market)
                && r.2.contains(&NodeKind::Event))
            .then_some(r)
        });
        let Some((_, _, nodes, _)) = win else {
            panic!(
                "{admiral:?} failed to win any 3-sector run covering all node types in seeds 1..=100 (max_sector={max_sector}, wins={wins}, breaches={breaches})"
            );
        };
        assert!(nodes.contains(&NodeKind::Battle));
        assert!(nodes.contains(&NodeKind::Elite));
        assert!(nodes.contains(&NodeKind::Boss));
        assert!(nodes.contains(&NodeKind::Shipyard));
        assert!(nodes.contains(&NodeKind::Market));
        assert!(nodes.contains(&NodeKind::Event));
    }
}

fn drive_battle_recording_cards(
    battle: &mut Battle,
    max_rounds: u32,
    priority_neutral: &[CardId],
    played: &mut HashSet<CardId>,
) {
    while battle.result.is_none() && battle.round <= max_rounds {
        for _ in 0..16 {
            if battle.result.is_some() {
                break;
            }
            if let Some((idx, tgt, cid)) = battle.hand.iter().enumerate().find_map(|(i, c)| {
                if !priority_neutral.contains(&c.id)
                    || played.contains(&c.id)
                    || battle.card_cost(i) > battle.points
                {
                    return None;
                }
                let tgt = match c.id.target() {
                    CardTarget::None => Target::None,
                    CardTarget::Enemy => Target::Enemy(battle.enemies.first()?.uid),
                    CardTarget::Column => return None,
                };
                let mut next = battle.clone();
                if next.play_card(i, tgt).is_ok()
                    && eval_turn_state(&next) >= eval_turn_state(battle) - 40
                {
                    Some((i, tgt, c.id))
                } else {
                    None
                }
            }) && battle.play_card(idx, tgt).is_ok()
            {
                played.insert(cid);
                continue;
            }
            let Some((is_station, idx, st, tgt)) = pick_best_action(battle) else {
                break;
            };
            if is_station {
                let _ = battle.run_station(st, tgt);
            } else {
                let cid = battle.hand[idx].id;
                if battle.play_card(idx, tgt).is_ok() {
                    played.insert(cid);
                }
            }
        }
        if battle.result.is_some() {
            break;
        }
        battle.end_turn();
    }
}

#[test]
fn t4_full_run_kerr_reyes_with_neutral_engine_and_loss_path() {
    const NEUTRAL_EXPANSION_6: [CardId; 6] = [
        CardId::BridgeCoordination,
        CardId::ChainCommand,
        CardId::StandbyFire,
        CardId::EmergencyPower,
        CardId::AutocannonSalvo,
        CardId::TargetPainter,
    ];
    let maps = load_sector_maps();

    // 1) Multi-battle progression across sectors + damaged hull carryover + all 6 neutral expansion cards played
    for admiral in [AdmiralId::Kerr, AdmiralId::Reyes] {
        let mut completed = false;
        for seed in 1..=50 {
            let mut run = Run::new(admiral, seed, maps.clone());
            run.deck
                .extend(NEUTRAL_EXPANSION_6.iter().copied().map(DeckCard::new));

            let mut played_cards: HashSet<CardId> = HashSet::new();
            let mut visited_nodes: HashSet<NodeKind> = HashSet::new();
            let mut battles_fought = 0usize;
            let mut last_post_battle_hull: Option<i32> = None;
            let mut repaired_since_battle = false;
            let mut saw_damaged_carryover = false;
            let mut lost = false;

            while run.result.is_none()
                && (run.sector == 1
                    || battles_fought < 6
                    || !NEUTRAL_EXPANSION_6.iter().all(|c| played_cards.contains(c))
                    || !saw_damaged_carryover)
            {
                match &run.stage {
                    Stage::Map => {
                        let choices = run.choices();
                        let row = run.current().map_or(0, |(r, _)| r + 1);
                        let col = *choices
                            .iter()
                            .max_by_key(|&&c| {
                                let kind = run.map[row][c].kind;
                                let priority = match kind {
                                    NodeKind::Shipyard if run.hull < run.max_hull * 75 / 100 => 6,
                                    NodeKind::Market if run.scrap >= 100 => 5,
                                    NodeKind::Battle => 4,
                                    NodeKind::Event => 3,
                                    NodeKind::Shipyard => 3,
                                    NodeKind::Market => 2,
                                    NodeKind::Elite => 1,
                                    NodeKind::Boss => 0,
                                };
                                (priority, !visited_nodes.contains(&kind) as u8)
                            })
                            .unwrap();
                        visited_nodes.insert(run.map[row][col].kind);
                        run.advance(col).unwrap();
                    }
                    Stage::Battle { .. } => {
                        let mut b = run.start_battle();
                        assert_eq!(
                            b.flagship.hull, run.hull,
                            "run.hull must carry over into Battle::flagship.hull"
                        );
                        if let Some(prev_hull) = last_post_battle_hull
                            && !repaired_since_battle
                            && prev_hull < run.max_hull
                        {
                            assert_eq!(b.flagship.hull, prev_hull);
                            saw_damaged_carryover = true;
                        }
                        drive_battle_recording_cards(
                            &mut b,
                            35,
                            &NEUTRAL_EXPANSION_6,
                            &mut played_cards,
                        );
                        if b.result != Some(BattleResult::Won) {
                            lost = true;
                            break;
                        }
                        let post_hull = b.flagship.hull;
                        let loot = run.finish_battle(&b);
                        battles_fought += 1;
                        assert_eq!(run.hull, post_hull + loot.repaired);
                        if loot.repaired == 0 {
                            last_post_battle_hull = Some(post_hull);
                            repaired_since_battle = false;
                        } else {
                            last_post_battle_hull = None;
                            repaired_since_battle = true;
                        }
                    }
                    Stage::Loot(loot) => {
                        if !loot.modules.is_empty() {
                            run.take_loot(Some(0)).unwrap();
                        } else {
                            run.take_loot(None).unwrap();
                        }
                    }
                    Stage::Shipyard => {
                        if run.hull < run.max_hull * 80 / 100 {
                            let before = run.hull;
                            run.repair_at_shipyard().unwrap();
                            if run.hull != before {
                                repaired_since_battle = true;
                            }
                        } else if let Some(idx) = run.deck.iter().position(|c| !c.upgraded) {
                            run.upgrade_at_shipyard(idx).unwrap();
                        } else {
                            let before = run.hull;
                            run.repair_at_shipyard().unwrap();
                            if run.hull != before {
                                repaired_since_battle = true;
                            }
                        }
                    }
                    Stage::Market(m) => {
                        if let Some((_, price)) = m.modules.first().copied()
                            && run.scrap >= price
                        {
                            run.buy_module(0).unwrap();
                        }
                        while run.hull <= run.max_hull - 10
                            && run.scrap >= rules().run.market_repair_price
                        {
                            let before = run.hull;
                            run.buy_repair().unwrap();
                            if run.hull != before {
                                repaired_since_battle = true;
                            }
                        }
                        run.leave_market().unwrap();
                    }
                    Stage::Event(ev_id) => {
                        let before = run.hull;
                        let opt = match ev_id {
                            voidfleet_core::run::EventId::Resolute => 1,
                            voidfleet_core::run::EventId::Pilot => 1,
                            voidfleet_core::run::EventId::Flicker => 1,
                        };
                        let _ = run.choose(opt).unwrap();
                        if run.hull != before {
                            repaired_since_battle = true;
                        }
                    }
                }
            }

            if !lost
                && battles_fought >= 6
                && run.sector >= 2
                && visited_nodes.contains(&NodeKind::Battle)
                && visited_nodes.contains(&NodeKind::Boss)
                && (visited_nodes.contains(&NodeKind::Shipyard)
                    || visited_nodes.contains(&NodeKind::Market)
                    || visited_nodes.contains(&NodeKind::Event))
                && saw_damaged_carryover
                && NEUTRAL_EXPANSION_6
                    .iter()
                    .all(|cid| played_cards.contains(cid))
            {
                completed = true;
                break;
            }
        }
        assert!(
            completed,
            "{admiral:?} must complete multi-battle Sector 1+ progression with damaged hull carryover and all 6 neutral expansion cards played"
        );
    }

    // 2) EmergencyGenerator + clean BattleResult::Lost -> RunResult::Lost path via run_loss.start_battle()
    let loss_seed = (1..200)
        .find(|&s| {
            let mut r = Run::new(AdmiralId::Kerr, s, maps.clone());
            r.advance(0).unwrap();
            let b = r.start_battle();
            b.hand.iter().any(|c| c.id == CardId::Torpedo)
        })
        .expect("must find seed with Torpedo in Round 1 hand");
    let mut run_loss = Run::new(AdmiralId::Kerr, loss_seed, maps);
    run_loss.modules.push(ModuleId::EmergencyGenerator);
    run_loss.hull = 3;
    run_loss.advance(0).unwrap();
    let mut b_loss = run_loss.start_battle();
    assert!(b_loss.emergency_generator_available);
    let r0 = b_loss.enemies[0].uid;
    play(&mut b_loss, CardId::Torpedo, Target::Enemy(r0));
    b_loss.run_station(Station::Shields, Target::None).unwrap();
    // Round 1: r1 consumes 6 of 8 shields; r2 breaks remaining 2 shields and deals 4 > 3 hull -> EmergencyGenerator saves at 1 hull
    let ev_save = b_loss.end_turn();
    assert!(ev_save.iter().any(|e| matches!(
        e,
        BattleEvent::EmergencyGenerator { after } if after.hull == 1
    )));
    assert!(b_loss.emergency_generator_used);
    assert_eq!(b_loss.flagship.hull, 1);
    assert_eq!(b_loss.result, None);
    // Round 2: next Raider hit drops 1 -> 0 with EmergencyGenerator already spent -> BattleResult::Lost
    while b_loss.result.is_none() {
        b_loss.end_turn();
    }
    assert_eq!(b_loss.result, Some(BattleResult::Lost));
    assert_eq!(b_loss.flagship.hull, 0);

    let loot = run_loss.finish_battle(&b_loss);
    assert_eq!(run_loss.result, Some(BattleResult::Lost));
    assert!(run_loss.emergency_generator_used);
    assert_eq!(run_loss.hull, 0);
    assert_eq!(loot, Default::default());
}

#[test]
fn t4_multi_seed_loot_and_market_invariants_all_admirals() {
    let battle_maps = synthetic_battle_maps(NodeKind::Battle);
    let market_maps = synthetic_battle_maps(NodeKind::Market);
    let mut seen_per_admiral: HashMap<AdmiralId, BTreeSet<String>> = HashMap::new();

    for admiral in ADMIRAL_IDS {
        let pool: HashSet<CardId> = admiral_pool(admiral).into_iter().collect();
        let allowed: HashSet<CardId> = admiral_pool(admiral)
            .into_iter()
            .chain(neutral_pool())
            .collect();
        let seen = seen_per_admiral.entry(admiral).or_default();

        for seed in 1..=60 {
            let mut r_loot = Run::new(admiral, seed, battle_maps.clone());
            r_loot.advance(0).unwrap();
            let mut b = r_loot.start_battle();
            b.result = Some(BattleResult::Won);
            let loot = r_loot.finish_battle(&b);
            assert_eq!(loot.cards.len(), 3);
            let uniq_l: HashSet<CardId> = loot.cards.iter().copied().collect();
            assert_eq!(uniq_l.len(), 3);
            assert!(uniq_l.is_subset(&allowed));
            assert!(loot.cards.iter().any(|c| pool.contains(c)));
            for c in loot.cards {
                seen.insert(format!("{c:?}"));
            }

            let mut r_mkt = Run::new(admiral, seed + 1000, market_maps.clone());
            r_mkt.advance(0).unwrap();
            let Stage::Market(mkt) = &r_mkt.stage else {
                panic!("expected market");
            };
            assert_eq!(mkt.cards.len(), 3);
            let ids: Vec<CardId> = mkt.cards.iter().map(|&(c, _)| c).collect();
            let uniq_m: HashSet<CardId> = ids.iter().copied().collect();
            assert_eq!(uniq_m.len(), 3);
            assert!(uniq_m.is_subset(&allowed));
            assert!(ids.iter().any(|c| pool.contains(c)));
            for c in ids {
                seen.insert(format!("{c:?}"));
            }
        }
        assert_eq!(
            seen.len(),
            33,
            "{admiral:?} must reach all 33 cards across 120 offers"
        );
    }

    // Verify all 48 CardId variants were reachable across the 4 admirals
    let all_seen: BTreeSet<String> = seen_per_admiral.into_values().flatten().collect();
    assert_eq!(all_seen.len(), 48);
    // Sanity-check unused public items exercised for complete coverage
    assert_eq!(FlagshipId::Bastion, AdmiralId::Osei.flagship());
    assert_eq!(AdmiralStation::Counterfire, AdmiralId::Osei.station());
    assert_eq!(Row::Front, EnemySlot::new(Row::Front, 0).row);
    assert_eq!(data().rules.battle.command_points, 3);
    let _r = Reinforcement {
        round: 2,
        unit: dummy_front(10, 1, 0),
    };
}
