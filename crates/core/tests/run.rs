use std::collections::HashSet;
use std::path::Path;
use std::sync::Once;

use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::battle::{Battle, BattleResult, Encounter};
use voidfleet_core::cards::{CardId, DeckCard};
use voidfleet_core::data::{Data, Step, install};
use voidfleet_core::enemies::{EnemyId, Unit};
use voidfleet_core::modules::ModuleId;
use voidfleet_core::run::{
    EventId, EventOption, Gain, MapError, NodeKind, Run, RunError, SECTORS, SectorMap, Stage,
};

/// Die Sternenkarten des Spiels, je Sektor eine. Die Zahlen daneben sind feste Prüfwerte aus `tests/data/` (wie in
/// `tests/battle.rs`), damit die Tests halten, wenn sich die Zahlen des Spiels ändern.
fn maps() -> [SectorMap; SECTORS as usize] {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"));
        install(Data::load(dir).expect("Prüfwerte")).expect("Prüfwerte gültig");
    });
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/maps");
    std::array::from_fn(|index| {
        let name = SectorMap::file(index as u32 + 1);
        let json = std::fs::read_to_string(format!("{dir}/{name}")).expect(&name);
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("{name}: {e}"))
    })
}

fn new_run(admiral: AdmiralId, seed: u64) -> Run {
    Run::new(admiral, seed, maps())
}

fn raiders() -> Encounter {
    Encounter {
        front: [None, Some(EnemyId::Raiders.into()), None],
        ..Encounter::default()
    }
}

/// Macht den ersten wählbaren Knoten zu `kind` und fliegt hin.
fn go(run: &mut Run, kind: NodeKind) {
    let column = run.choices()[0];
    let node = &mut run.map[run.path.len()][column];
    node.kind = kind;
    node.encounter.get_or_insert_with(raiders);
    run.advance(column).expect("wählbar");
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
    let mut run = new_run(AdmiralId::Osei, 3);
    go(&mut run, NodeKind::Event);
    run.stage = Stage::Event(event);
    run
}

/// Die Daten des Spiels in `assets/data/` lassen sich lesen und bestehen die Prüfung.
#[test]
fn spieldaten() {
    Data::load(Path::new(voidfleet_core::data::REPO_DATA)).expect("assets/data");
}

/// Jede Sternenkarte des Spiels ist gültig, und der Run fliegt sie in der Reihenfolge der Sektoren.
#[test]
fn sternenkarten() {
    let maps = maps();
    for (sector, map) in (1..).zip(&maps) {
        assert_eq!(map.check(), Ok(()), "Sektor {sector}");
    }
    let run = new_run(AdmiralId::Kerr, 0);
    assert_eq!(
        [&run.map, &run.upcoming[0], &run.upcoming[1]],
        maps.each_ref().map(|map| &map.rows)
    );
}

#[test]
fn karte_prüfen() {
    let json = r#"{ "rows": [
        [{ "kind": "battle", "next": [0, 1], "encounter": { "front": [null, { "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] }, null] } }],
        [{ "kind": "event", "next": [0] }, { "kind": "shipyard", "next": [0] }],
        [{ "kind": "boss", "next": [], "encounter": { "front": [{ "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] }, null, null], "back": [{ "id": "scrap_king", "hull": 60, "attack": 7, "armor": 0, "pattern": ["fire", { "call": { "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] } }] }, null, null] } }]
    ] }"#;
    let good: SectorMap = serde_json::from_str(json).unwrap();
    assert_eq!(good.check(), Ok(()));
    assert_eq!(
        good.rows[2][0].encounter.as_ref().unwrap().units().count(),
        2
    );
    let broken = |change: fn(&mut SectorMap)| {
        let mut map = good.clone();
        change(&mut map);
        map.check()
    };
    assert_eq!(
        broken(|m| m.rows[1][0].kind = NodeKind::Boss),
        Err(MapError::Boss)
    );
    assert_eq!(
        broken(|m| m.rows[2][0].kind = NodeKind::Elite),
        Err(MapError::Boss)
    );
    assert_eq!(broken(|m| m.rows[1].clear()), Err(MapError::EmptyRow(1)));
    assert_eq!(
        broken(|m| m.rows[1][1].next.clear()),
        Err(MapError::Edges(1, 1))
    );
    assert_eq!(
        broken(|m| m.rows[1][0].next = vec![1]),
        Err(MapError::Edges(1, 0))
    );
    assert_eq!(
        broken(|m| m.rows[0][0].next = vec![1, 0]),
        Err(MapError::Edges(0, 0))
    );
    assert_eq!(
        broken(|m| m.rows[0][0].next = vec![0]),
        Err(MapError::Unreachable(1, 1))
    );
    fn encounter(m: &mut SectorMap) -> &mut Encounter {
        m.rows[0][0].encounter.as_mut().unwrap()
    }
    assert_eq!(
        broken(|m| m.rows[0][0].encounter = None),
        Err(MapError::Encounter(0, 0))
    );
    assert_eq!(
        broken(|m| m.rows[1][0].encounter = Some(raiders())),
        Err(MapError::Encounter(1, 0))
    );
    assert_eq!(
        broken(|m| encounter(m).front = Default::default()),
        Err(MapError::Encounter(0, 0))
    );
    assert_eq!(
        broken(|m| encounter(m).back[0] = Some(EnemyId::Spotter.into())),
        Err(MapError::Encounter(0, 0))
    );
    assert_eq!(
        broken(|m| {
            encounter(m).front = std::array::from_fn(|_| Some(EnemyId::Raiders.into()));
            encounter(m).back = std::array::from_fn(|_| Some(EnemyId::Raiders.into()));
        }),
        Err(MapError::Encounter(0, 0))
    );
    // Die Werte einer Einheit: ein leeres Muster oder eine ungültige gerufene Einheit stimmt nicht.
    fn custom(pattern: Vec<Step>) -> Option<Unit> {
        Some(Unit {
            pattern,
            ..EnemyId::Raiders.into()
        })
    }
    assert_eq!(
        broken(|m| encounter(m).front[0] = custom(vec![Step::ColumnFire(2)])),
        Ok(())
    );
    assert_eq!(
        broken(|m| encounter(m).front[0] = custom(Vec::new())),
        Err(MapError::Encounter(0, 0))
    );
    assert_eq!(
        broken(|m| {
            let called = Unit {
                hull: 0,
                ..EnemyId::Raiders.into()
            };
            encounter(m).front[0] = custom(vec![Step::Call(called)]);
        }),
        Err(MapError::Encounter(0, 0))
    );
    // Eine Einheit nennt alle Werte; nur die Id reicht nicht.
    assert!(
        serde_json::from_str::<SectorMap>(&json.replacen(
            r#"{ "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] }"#,
            r#""raiders""#,
            1
        ))
        .is_err()
    );
    // Ein Tippfehler im Feldnamen fällt beim Lesen auf.
    assert!(serde_json::from_str::<SectorMap>(&json.replace("next", "nxt")).is_err());
}

#[test]
fn wählbare_knoten() {
    let mut run = new_run(AdmiralId::Osei, 1);
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

#[test]
fn gefecht_starten() {
    let mut run = new_run(AdmiralId::Kerr, 2);
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

    // Der Verband des Knotens steht auf dem Brett, in jedem Sektor mit genau den Werten aus der Karte.
    for (sector, hull) in [(1, 14), (2, 14), (3, 14)] {
        let mut run = new_run(AdmiralId::Kerr, 2);
        run.sector = sector;
        let column = run.choices()[0];
        run.map[0][column].encounter = Some(Encounter {
            front: [Some(EnemyId::PlasmaCruiser.into()), None, None],
            ..Encounter::default()
        });
        run.advance(column).unwrap();
        let battle = run.start_battle();
        assert_eq!(battle.enemies.len(), 1);
        let enemy = &battle.enemies[0];
        assert_eq!((enemy.id, enemy.slot.column), (EnemyId::PlasmaCruiser, 0));
        assert_eq!((enemy.hull, enemy.max_hull), (hull, hull));
    }
}

#[test]
fn beute_nach_gefecht() {
    let mut run = new_run(AdmiralId::Osei, 4);
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
    let mut run = new_run(AdmiralId::Vance, 6);
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
    let mut run = new_run(AdmiralId::Osei, 8);
    run.path = vec![0; run.map.len() - 1];
    run.hull = 10;
    run.advance(0).unwrap();
    let loot = run.finish_battle(&won(&run));
    assert_eq!(loot.scrap, 60);
    assert_eq!(loot.modules.len(), 3);
    assert_eq!(loot.repaired, 21);
    assert_eq!(run.hull, 31);
    let next_map = run.upcoming[0].clone();
    run.take_loot(None).unwrap();
    assert_eq!(run.sector, 2);
    assert!(run.path.is_empty());
    assert_eq!(run.map, next_map);
    assert_eq!(run.upcoming.len(), 1);

    // Sektor 3: der Boss beendet den Run.
    run.sector = 3;
    run.path = vec![0; run.map.len() - 1];
    run.advance(0).unwrap();
    let loot = run.finish_battle(&won(&run));
    assert_eq!(run.result, Some(BattleResult::Won));
    assert!(loot.cards.is_empty() && loot.modules.is_empty());
    assert_eq!(run.advance(0), Err(RunError::WrongStage));
}

#[test]
fn niederlage() {
    let mut run = new_run(AdmiralId::Reyes, 9);
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
    let mut run = new_run(AdmiralId::Kerr, 10);
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
    let mut run = new_run(AdmiralId::Osei, 11);
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
    assert_eq!(run.map[1][column].encounter, None);
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
        let mut run = new_run(AdmiralId::Vance, seed);
        go(&mut run, NodeKind::Battle);
        let loot = run.finish_battle(&won(&run));
        run.take_loot(None).unwrap();
        go(&mut run, NodeKind::Market);
        (run, loot)
    };
    assert_eq!(play(42), play(42));
}

#[test]
fn speichern_und_laden() {
    let mut run = new_run(AdmiralId::Reyes, 77);
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
        let mut run = new_run(ADMIRAL_IDS[seed as usize % 4], seed);
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
