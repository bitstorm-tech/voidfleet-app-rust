use std::collections::HashSet;

use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::battle::{Battle, BattleResult, Encounter};
use voidfleet_core::board::{EnemySlot, Row};
use voidfleet_core::cards::{CardId, DeckCard};
use voidfleet_core::enemies::{EnemyClass, EnemyId};
use voidfleet_core::modules::ModuleId;
use voidfleet_core::run::{
    EventId, EventOption, Gain, MapNode, NodeKind, ROWS, Run, RunError, Stage,
};

/// Macht den ersten wählbaren Knoten zu `kind` und fliegt hin.
fn go(run: &mut Run, kind: NodeKind) {
    let column = run.choices()[0];
    run.map[run.path.len()][column].kind = kind;
    run.advance(column).expect("wählbar");
}

fn encounter(run: &Run) -> Encounter {
    let Stage::Battle { encounter, .. } = &run.stage else {
        panic!("kein Gefecht: {:?}", run.stage);
    };
    encounter.clone()
}

fn won(run: &Run) -> Battle {
    let mut battle = run.start_battle();
    battle.enemies.clear();
    battle.result = Some(BattleResult::Won);
    battle
}

fn loot_stage(run: &Run) -> voidfleet_core::run::Loot {
    let Stage::Loot(loot) = &run.stage else {
        panic!("keine Beute: {:?}", run.stage);
    };
    loot.clone()
}

/// Ein Run, der vor einem Ereignis `event` steht.
fn at_event(event: EventId) -> Run {
    let mut run = Run::new(AdmiralId::Osei, 3);
    go(&mut run, NodeKind::Event);
    run.stage = Stage::Event(event);
    run
}

fn check_map(map: &[Vec<MapNode>]) {
    assert_eq!(map.len(), ROWS);
    let last = &map[ROWS - 1];
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].kind, NodeKind::Boss);
    assert!(last[0].next.is_empty());
    assert!(map[0].iter().all(|n| n.kind == NodeKind::Battle));
    assert!(map[..2].iter().flatten().all(|n| n.kind != NodeKind::Elite));
    assert!(map.iter().flatten().any(|n| n.kind == NodeKind::Shipyard));
    for (r, row) in map[..ROWS - 1].iter().enumerate() {
        assert!((3..=4).contains(&row.len()));
        assert!(row.iter().all(|n| n.kind != NodeKind::Boss));
        let below = map[r + 1].len();
        let mut reached = HashSet::new();
        for (c, node) in row.iter().enumerate() {
            assert!(!node.next.is_empty(), "Knoten {r}/{c} ohne Kante hinaus");
            assert!(node.next.windows(2).all(|w| w[0] < w[1]));
            assert!(node.next.iter().all(|&n| n < below));
            reached.extend(node.next.iter().copied());
            // Keine Überkreuzung: rechts liegende Knoten führen nicht weiter nach links.
            if let Some(right) = row.get(c + 1) {
                assert!(node.next.last() <= right.next.first());
            }
        }
        assert_eq!(
            reached.len(),
            below,
            "jeder Knoten der Reihe {} erreichbar",
            r + 1
        );
    }
}

#[test]
fn sternenkarte() {
    let mut counts = [0usize; 6];
    for seed in 0..300 {
        let run = Run::new(AdmiralId::Kerr, seed);
        check_map(&run.map);
        for node in run.map[1..ROWS - 1].iter().flatten() {
            counts[node.kind as usize] += 1;
        }
    }
    let total: usize = counts.iter().sum();
    let share = |kind: NodeKind| counts[kind as usize] as f64 / total as f64;
    assert!((0.45..0.6).contains(&share(NodeKind::Battle)));
    assert!((0.07..0.14).contains(&share(NodeKind::Elite)));
    assert!((0.15..0.25).contains(&share(NodeKind::Event)));
    assert!((0.08..0.16).contains(&share(NodeKind::Shipyard)));
    assert!((0.03..0.09).contains(&share(NodeKind::Market)));
}

#[test]
fn wählbare_knoten() {
    let mut run = Run::new(AdmiralId::Osei, 1);
    assert_eq!(run.choices(), (0..run.map[0].len()).collect::<Vec<_>>());
    assert_eq!(run.current(), None);
    assert_eq!(run.advance(9), Err(RunError::InvalidNode));
    run.advance(0).unwrap();
    assert_eq!(run.current(), Some((0, 0)));
    assert!(run.visited(0, 0) && !run.visited(0, 1));
    assert_eq!(run.choices(), run.map[0][0].next);
    // Erst das Gefecht, dann weiter.
    assert_eq!(run.advance(run.choices()[0]), Err(RunError::WrongStage));
    run.finish_battle(&won(&run));
    run.take_loot(None).unwrap();
    let unreachable = (0..run.map[1].len()).find(|c| !run.map[0][0].next.contains(c));
    if let Some(column) = unreachable {
        assert_eq!(run.advance(column), Err(RunError::InvalidNode));
    }
    run.advance(run.choices()[0]).unwrap();
    assert_eq!(run.current().unwrap().0, 1);
}

/// Eine Einheit der erwarteten Aufstellung.
type Unit = (EnemyId, Row, usize);

fn check_formation(units: &[(EnemyId, EnemySlot)]) {
    let slots: HashSet<_> = units.iter().map(|&(_, s)| s).collect();
    assert_eq!(slots.len(), units.len(), "doppelte Slots: {units:?}");
    for &(id, slot) in units {
        if let Some(front) = slot.front() {
            assert!(slots.contains(&front), "{id:?} hinten ungedeckt: {units:?}");
        } else {
            assert_ne!(id, EnemyId::Spotter, "Spotter vorn: {units:?}");
        }
    }
}

#[test]
fn verbände_je_sektor() {
    for sector in 1..=3 {
        let mut reinforced = false;
        for seed in 0..200 {
            let mut run = Run::new(AdmiralId::Vance, seed);
            run.sector = sector;
            go(&mut run, NodeKind::Battle);
            let enc = encounter(&run);
            check_formation(&enc.units);
            let total = enc.units.len() + enc.reinforcements.len();
            assert!(total <= 4);
            assert!(enc.replacements.is_empty());
            assert_eq!(enc.hull_percent, [100, 125, 150][sector as usize - 1]);
            let ids: Vec<EnemyId> = enc.units.iter().map(|&(id, _)| id).collect();
            assert!(ids.iter().filter(|&&id| id == EnemyId::Spotter).count() <= 1);
            assert!(
                ids.iter()
                    .filter(|id| id.class() == EnemyClass::Battleship)
                    .count()
                    <= 1
            );
            match sector {
                1 => {
                    assert!((1..=2).contains(&total));
                    assert!(ids.iter().all(|id| matches!(
                        id,
                        EnemyId::Raiders | EnemyId::GuardCruiser | EnemyId::Fireship
                    )));
                }
                2 => assert!((2..=3).contains(&total)),
                _ => {
                    assert!((3..=4).contains(&total));
                    assert!(enc.units.len() >= 2);
                    reinforced |= !enc.reinforcements.is_empty();
                }
            }
            assert!(ids.iter().all(|id| id.class() != EnemyClass::Flagship));
        }
        assert_eq!(reinforced, sector == 3);
    }
}

#[test]
fn elite_und_bosse() {
    use EnemyId::*;
    use Row::*;
    let expected: [(NodeKind, u32, &[Unit]); 6] = [
        (NodeKind::Elite, 1, &[(Besieger, Front, 1)]),
        (
            NodeKind::Elite,
            2,
            &[
                (GuardCruiser, Front, 1),
                (GuardCruiser, Front, 0),
                (Besieger, Back, 1),
            ],
        ),
        (
            NodeKind::Elite,
            3,
            &[
                (Besieger, Front, 1),
                (Besieger, Front, 0),
                (Spotter, Back, 1),
            ],
        ),
        (
            NodeKind::Boss,
            1,
            &[
                (Raiders, Front, 1),
                (Raiders, Front, 0),
                (ScrapKing, Back, 1),
            ],
        ),
        (
            NodeKind::Boss,
            2,
            &[
                (GuardCruiser, Front, 1),
                (GuardCruiser, Front, 0),
                (Warden, Back, 1),
            ],
        ),
        (
            NodeKind::Boss,
            3,
            &[
                (GuardCruiser, Front, 1),
                (PlasmaCruiser, Front, 0),
                (Seal, Back, 1),
                (Spotter, Back, 0),
            ],
        ),
    ];
    for (kind, sector, units) in expected {
        let mut run = Run::new(AdmiralId::Reyes, 5);
        run.sector = sector;
        go(&mut run, kind);
        let enc = encounter(&run);
        let want: Vec<_> = units
            .iter()
            .map(|&(id, row, c)| (id, EnemySlot::new(row, c)))
            .collect();
        assert_eq!(enc.units, want);
        check_formation(&enc.units);
        let replacements = if kind == NodeKind::Boss && sector == 3 {
            vec![GuardCruiser, GuardCruiser]
        } else {
            vec![]
        };
        assert_eq!(enc.replacements, replacements);
    }
}

#[test]
fn gefecht_starten() {
    let mut run = Run::new(AdmiralId::Kerr, 2);
    run.hull = 30;
    run.modules = vec![ModuleId::EmergencyGenerator, ModuleId::ReinforcedFrames];
    run.max_hull = 66;
    run.burning = true;
    go(&mut run, NodeKind::Battle);
    let battle = run.start_battle();
    assert_eq!(battle.flagship.hull, 30);
    assert_eq!(battle.flagship.max_hull, 66);
    assert!(battle.flagship.burning);
    assert!(battle.emergency_generator_available);
    assert_eq!(battle.hand.len() + battle.draw_pile.len(), run.deck.len());
    // Wiederholbar, etwa nach dem Laden.
    assert_eq!(run.start_battle().hand, battle.hand);

    let mut battle = won(&run);
    battle.flagship.hull = 12;
    battle.flagship.burning = true;
    battle.emergency_generator_used = true;
    run.finish_battle(&battle);
    assert_eq!(run.hull, 12);
    assert!(!run.burning && run.emergency_generator_used);
    run.take_loot(None).unwrap();
    go(&mut run, NodeKind::Battle);
    let battle = run.start_battle();
    assert!(!battle.flagship.burning && !battle.emergency_generator_available);
}

#[test]
fn beute_nach_gefecht() {
    let mut run = Run::new(AdmiralId::Osei, 4);
    go(&mut run, NodeKind::Battle);
    let loot = run.finish_battle(&won(&run));
    assert!((10..=20).contains(&loot.scrap));
    assert_eq!(run.scrap, loot.scrap);
    assert_eq!(loot.cards.len(), 3);
    assert_eq!(loot.cards.iter().collect::<HashSet<_>>().len(), 3);
    assert!(loot.modules.is_empty());
    assert_eq!(loot_stage(&run), loot);
    assert_eq!(run.take_loot(Some(3)), Err(RunError::OutOfRange));
    run.take_loot(Some(1)).unwrap();
    assert_eq!(run.deck.len(), 11);
    assert_eq!(run.deck[10], DeckCard::new(loot.cards[1]));
    assert_eq!(run.stage, Stage::Map);

    // Kampfsimulator zeigt vier, Bergungskran bringt 5 je Abschuss.
    run.modules = vec![ModuleId::CombatSimulator, ModuleId::SalvageCrane];
    go(&mut run, NodeKind::Battle);
    let mut battle = won(&run);
    battle.kills = 3;
    let before = run.scrap;
    let loot = run.finish_battle(&battle);
    assert_eq!(loot.cards.len(), 4);
    assert!((25..=35).contains(&loot.scrap));
    assert_eq!(run.scrap, before + loot.scrap);
}

#[test]
fn beute_nach_elite() {
    let mut run = Run::new(AdmiralId::Vance, 6);
    run.modules = vec![ModuleId::ArmorPlates];
    go(&mut run, NodeKind::Elite);
    let loot = run.finish_battle(&won(&run));
    assert!((25..=40).contains(&loot.scrap));
    assert!(loot.cards.is_empty());
    assert_eq!(loot.modules.len(), 3);
    assert!(!loot.modules.contains(&ModuleId::ArmorPlates));
    assert_eq!(loot.modules.iter().collect::<HashSet<_>>().len(), 3);
    // Verstärkte Spanten wirken sofort.
    run.stage = Stage::Loot(voidfleet_core::run::Loot {
        modules: vec![ModuleId::ReinforcedFrames],
        ..loot
    });
    run.hull = 20;
    run.take_loot(Some(0)).unwrap();
    assert_eq!((run.hull, run.max_hull), (30, 60));
    assert!(run.has(ModuleId::ReinforcedFrames));
}

#[test]
fn boss_und_sektorwechsel() {
    let mut run = Run::new(AdmiralId::Osei, 8);
    run.path = vec![0; ROWS - 1];
    run.hull = 10;
    run.advance(0).unwrap();
    assert_eq!(encounter(&run).units[2].0, EnemyId::ScrapKing);
    let loot = run.finish_battle(&won(&run));
    assert_eq!(loot.scrap, 60);
    assert_eq!(loot.modules.len(), 3);
    assert_eq!(loot.repaired, 21);
    assert_eq!(run.hull, 31);
    let old_map = run.map.clone();
    run.take_loot(None).unwrap();
    assert_eq!(run.sector, 2);
    assert!(run.path.is_empty());
    assert_ne!(run.map, old_map);
    check_map(&run.map);

    // Sektor 3: der Boss beendet den Run.
    run.sector = 3;
    run.path = vec![0; ROWS - 1];
    run.advance(0).unwrap();
    let loot = run.finish_battle(&won(&run));
    assert_eq!(run.result, Some(BattleResult::Won));
    assert!(loot.cards.is_empty() && loot.modules.is_empty());
    assert_eq!(run.advance(0), Err(RunError::WrongStage));
}

#[test]
fn niederlage() {
    let mut run = Run::new(AdmiralId::Reyes, 9);
    go(&mut run, NodeKind::Battle);
    let mut battle = run.start_battle();
    battle.flagship.hull = 0;
    battle.result = Some(BattleResult::Lost);
    let loot = run.finish_battle(&battle);
    assert_eq!(loot, Default::default());
    assert_eq!(run.result, Some(BattleResult::Lost));
    assert_eq!(run.scrap, 0);
    assert_eq!(run.advance(0), Err(RunError::WrongStage));
}

#[test]
fn werft() {
    let mut run = Run::new(AdmiralId::Kerr, 10);
    assert_eq!(run.repair_at_shipyard(), Err(RunError::WrongStage));
    go(&mut run, NodeKind::Shipyard);
    run.hull = 20;
    run.repair_at_shipyard().unwrap();
    assert_eq!(run.hull, 37); // 30 % von 56 = 16,8, aufgerundet 17.
    assert_eq!(run.repair_at_shipyard(), Err(RunError::WrongStage));

    run.stage = Stage::Shipyard;
    run.hull = 50;
    run.repair_at_shipyard().unwrap();
    assert_eq!(run.hull, 56);

    run.stage = Stage::Shipyard;
    run.deck[0].upgraded = true;
    assert_eq!(run.upgrade_at_shipyard(0), Err(RunError::AlreadyUpgraded));
    assert_eq!(run.upgrade_at_shipyard(10), Err(RunError::OutOfRange));
    run.upgrade_at_shipyard(9).unwrap();
    assert!(run.deck[9].upgraded);

    run.stage = Stage::Shipyard;
    assert_eq!(run.remove_at_shipyard(10), Err(RunError::OutOfRange));
    run.remove_at_shipyard(0).unwrap();
    assert_eq!(run.deck.len(), 9);
    assert!(
        run.deck
            .iter()
            .all(|c| c.id != CardId::Torpedo || !c.upgraded)
    );
}

#[test]
fn schwarzmarkt() {
    let mut run = Run::new(AdmiralId::Osei, 11);
    run.modules = vec![ModuleId::SupplyPod];
    go(&mut run, NodeKind::Market);
    let Stage::Market(market) = run.stage.clone() else {
        panic!("kein Markt");
    };
    assert_eq!(market.cards.len(), 3);
    assert_eq!(
        market
            .cards
            .iter()
            .map(|c| c.0)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert!(market.cards.iter().all(|&(_, p)| (40..=70).contains(&p)));
    assert_eq!(market.modules.len(), 2);
    assert!(
        market
            .modules
            .iter()
            .all(|&(m, p)| (120..=180).contains(&p) && m != ModuleId::SupplyPod)
    );
    assert!(market.removal);

    let (card, price) = market.cards[1];
    run.scrap = price - 1;
    assert_eq!(run.buy_card(1), Err(RunError::NotEnoughScrap));
    assert_eq!(run.scrap, price - 1);
    run.scrap = 1000;
    assert_eq!(run.buy_card(3), Err(RunError::OutOfRange));
    run.buy_card(1).unwrap();
    assert_eq!(run.scrap, 1000 - price);
    assert_eq!(*run.deck.last().unwrap(), DeckCard::new(card));

    let (module, price) = market.modules[0];
    let scrap = run.scrap;
    run.buy_module(0).unwrap();
    assert!(run.has(module));
    assert_eq!(run.scrap, scrap - price);

    let scrap = run.scrap;
    let deck = run.deck.len();
    run.buy_removal(0).unwrap();
    assert_eq!((run.deck.len(), run.scrap), (deck - 1, scrap - 60));
    assert_eq!(run.buy_removal(0), Err(RunError::SoldOut));

    run.hull = 40;
    run.buy_repair().unwrap();
    run.buy_repair().unwrap();
    assert_eq!(run.hull, 60);
    run.scrap = 24;
    assert_eq!(run.buy_repair(), Err(RunError::NotEnoughScrap));

    let Stage::Market(left) = &run.stage else {
        panic!("kein Markt");
    };
    assert_eq!(
        (left.cards.len(), left.modules.len(), left.removal),
        (2, 1, false)
    );
    run.leave_market().unwrap();
    assert_eq!(run.stage, Stage::Map);
    assert_eq!(run.buy_repair(), Err(RunError::WrongStage));
}

#[test]
fn wrack_der_resolute() {
    use EventOption::*;
    assert_eq!(EventId::Resolute.options(), &[Salvage, Logbooks, MoveOn]);

    let mut run = at_event(EventId::Resolute);
    assert_eq!(run.choose(3), Err(RunError::OutOfRange));
    assert_eq!(run.choose(0), Ok(Gain::Nothing));
    assert_eq!(run.scrap, 40);
    assert!(run.burning);
    assert_eq!(run.stage, Stage::Map);
    go(&mut run, NodeKind::Battle);
    assert!(run.start_battle().flagship.burning);

    let mut run = at_event(EventId::Resolute);
    run.modules = vec![ModuleId::SupplyPod];
    let Ok(Gain::Module(module)) = run.choose(1) else {
        panic!("kein Modul");
    };
    assert_ne!(module, ModuleId::SupplyPod);
    assert_eq!(run.modules, vec![ModuleId::SupplyPod, module]);

    let mut run = at_event(EventId::Resolute);
    let before = run.clone();
    assert_eq!(run.choose(2), Ok(Gain::Nothing));
    assert_eq!(
        (run.scrap, run.hull, &run.modules),
        (before.scrap, before.hull, &before.modules)
    );
}

#[test]
fn der_lotse() {
    let mut run = at_event(EventId::Pilot);
    run.scrap = 49;
    assert_eq!(run.choose(0), Err(RunError::NotEnoughScrap));
    run.scrap = 70;
    run.choose(0).unwrap();
    assert_eq!(run.scrap, 20);
    let column = run.choices()[0];
    run.advance(column).unwrap();
    assert_eq!(run.stage, Stage::Shipyard);
    assert_eq!(run.map[1][column].kind, NodeKind::Shipyard);
    assert!(!run.pilot);

    let mut run = at_event(EventId::Pilot);
    run.choose(1).unwrap();
    assert_eq!(run.hull, 64);
    let loot = loot_stage(&run);
    assert_eq!((loot.cards.len(), loot.scrap), (3, 0));
    run.take_loot(Some(0)).unwrap();
    assert_eq!(run.deck.len(), 11);
}

#[test]
fn flackern() {
    let mut run = at_event(EventId::Flicker);
    run.hull = 30;
    let Ok(Gain::Upgraded(card)) = run.choose(0) else {
        panic!("keine Aufwertung");
    };
    assert_eq!(run.hull, 12); // Ein Viertel von 70, aufgerundet 18.
    assert_eq!(run.deck.iter().filter(|c| c.upgraded).count(), 1);
    assert!(run.deck.iter().any(|c| c.upgraded && c.id == card));

    let mut run = at_event(EventId::Flicker);
    run.hull = 5;
    run.choose(0).unwrap();
    assert_eq!(run.hull, 1);

    let mut run = at_event(EventId::Flicker);
    assert_eq!(run.choose(1), Ok(Gain::Nothing));
    assert_eq!(run.hull, 70);
    assert!(run.deck.iter().all(|c| !c.upgraded));
}

#[test]
fn gleicher_seed_gleicher_run() {
    let play = |seed| {
        let mut run = Run::new(AdmiralId::Vance, seed);
        go(&mut run, NodeKind::Battle);
        let loot = run.finish_battle(&won(&run));
        run.take_loot(None).unwrap();
        go(&mut run, NodeKind::Market);
        (run, loot)
    };
    assert_eq!(play(42), play(42));
    assert_ne!(play(42).0.map, play(43).0.map);
}

#[test]
fn speichern_und_laden() {
    let mut run = Run::new(AdmiralId::Reyes, 77);
    go(&mut run, NodeKind::Battle);
    run.finish_battle(&won(&run));
    run.take_loot(Some(0)).unwrap();
    go(&mut run, NodeKind::Battle);
    let json = serde_json::to_string(&run).unwrap();
    let mut loaded: Run = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, run);
    assert_eq!(loaded.start_battle().hand, run.start_battle().hand);
    let loot = run.finish_battle(&won(&run));
    assert_eq!(loaded.finish_battle(&won(&loaded)), loot);
    assert_eq!(loaded, run);
}

/// Immer der erste wählbare Knoten, jedes Gefecht gewonnen: Der Run endet ohne Panik im Sieg.
#[test]
fn durchgespielter_run() {
    for seed in 0..50 {
        let mut run = Run::new(ADMIRAL_IDS[seed as usize % 4], seed);
        let mut steps = 0;
        while run.result.is_none() {
            steps += 1;
            assert!(steps < 200);
            match run.stage.clone() {
                Stage::Map => run.advance(run.choices()[0]).unwrap(),
                Stage::Battle { .. } => {
                    run.finish_battle(&won(&run));
                }
                Stage::Loot(_) => run.take_loot(Some(0)).unwrap(),
                Stage::Event(event) => {
                    let last = event.options().len() - 1;
                    run.choose(last).unwrap();
                }
                Stage::Shipyard => run.repair_at_shipyard().unwrap(),
                Stage::Market(_) => run.leave_market().unwrap(),
            }
        }
        assert_eq!(run.result, Some(BattleResult::Won));
        assert_eq!(run.sector, 3);
    }
}
