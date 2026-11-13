//! Der Simulator: Ein Bot ([`bot`]) spielt Runs mit den Daten aus `assets/data/` und misst, was die Zahlen ergeben.
//!
//! - `voidfleet-sim [runs]`: Runs je Admiral; Siegquote, wo die Runs enden, je Knoten der Sternenkarten Verlust an
//!   Hülle, Runden und Niederlagen, je Karte, wie oft sie gespielt wird, wenn sie in der Hand ist.
//! - `voidfleet-sim cards`: je Karte dieselben Runs mit einer Karte mehr im Startdeck, gegen die Runs ohne sie.
//! - `voidfleet-sim modules`: ebenso mit einem Modul von Beginn an.
//! - `voidfleet-sim trace`: ein Run des ersten Admirals Zug für Zug, um dem Bot zuzusehen.
//!
//! Optionen: `-n <runs je Admiral>` (Vorgabe 300), `--admiral <id>` (nur dieser), `--seed <erster Seed>`,
//! `--data <ordner>` (eine Kopie von `assets/data` mit anderen Zahlen oder Karten, zum Probieren).

mod bot;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::battle::Encounter;
use voidfleet_core::cards::LOOT_POOL;
use voidfleet_core::data::{Data, REPO_DATA, install, read};
use voidfleet_core::run::{MODULES, NodeKind, SECTORS, SectorMap};

use bot::{RunLog, Start, play_run};

type Maps = [SectorMap; SECTORS as usize];

/// Die Id, wie sie in den Dateien steht (`guard_cruiser`).
fn name(id: impl serde::Serialize) -> String {
    match serde_json::to_value(id).expect("Id als JSON") {
        Value::String(name) => name,
        other => other.to_string(),
    }
}

fn load_maps(dir: &Path) -> Maps {
    std::array::from_fn(|index| {
        let file = format!("maps/{}", SectorMap::file(index as u32 + 1));
        let map: SectorMap = read(dir, &file).unwrap_or_else(|error| panic!("{error}"));
        map.check()
            .unwrap_or_else(|error| panic!("{file}: {error:?}"));
        map
    })
}

/// Spielt `runs` Runs je Admiral auf allen Kernen; dieselben Seeds für jeden Aufruf.
fn simulate(
    admirals: &[AdmiralId],
    runs: u64,
    seed: u64,
    maps: &Maps,
    start: &Start,
) -> Vec<RunLog> {
    let jobs: Vec<(AdmiralId, u64)> = (admirals.iter())
        .flat_map(|&admiral| (0..runs).map(move |run| (admiral, seed + run)))
        .collect();
    let next = AtomicUsize::new(0);
    let logs = Mutex::new(Vec::with_capacity(jobs.len()));
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(&(admiral, seed)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let log = play_run(admiral, seed, maps, start);
                    logs.lock().expect("Logs").push(log);
                }
            });
        }
    });
    logs.into_inner().expect("Logs")
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0.0), |(sum, count), value| (sum + value, count + 1.0));
    if count == 0.0 { f64::NAN } else { sum / count }
}

fn percent(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

fn victories(log: &RunLog) -> f64 {
    log.fights.iter().filter(|fight| fight.won).count() as f64
}

fn win_rate(logs: &[&RunLog]) -> f64 {
    percent(logs.iter().filter(|log| log.won).count(), logs.len())
}

fn describe(encounter: &Encounter) -> String {
    let unit = |unit: &voidfleet_core::enemies::Unit| {
        let armor = if unit.armor > 0 {
            format!("/{}", unit.armor)
        } else {
            String::new()
        };
        format!("{} {}/{}{armor}", name(unit.id), unit.hull, unit.attack)
    };
    let mut parts: Vec<String> = encounter.units().map(|(u, _)| unit(u)).collect();
    parts.extend(
        (encounter.reinforcements.iter()).map(|r| format!("+R{} {}", r.round, unit(&r.unit))),
    );
    parts.extend(
        encounter
            .replacements
            .iter()
            .map(|u| format!("+N {}", unit(u))),
    );
    parts.join(", ")
}

fn report_admirals(logs: &[RunLog], admirals: &[AdmiralId]) {
    println!("\n== Admirale ==");
    println!(
        "{:8} {:>5} {:>6} {:>9}  {:>6} {:>6} {:>6}  {:>17}  {:>5} {:>6} {:>7}",
        "Admiral",
        "Runs",
        "Sieg%",
        "Siege/Run",
        "Tod S1",
        "Tod S2",
        "Tod S3",
        "Hülle% vor Boss",
        "Deck",
        "Module",
        "Schrott"
    );
    for &admiral in admirals {
        let own: Vec<&RunLog> = logs.iter().filter(|log| log.admiral == admiral).collect();
        let died = |sector: u32| {
            let lost = |log: &&&RunLog| {
                log.fights
                    .last()
                    .is_some_and(|f| !f.won && f.sector == sector)
            };
            percent(own.iter().filter(lost).count(), own.len())
        };
        let before_boss = |sector: u32| {
            let boss = |log: &&RunLog| {
                let fight =
                    (log.fights.iter()).find(|f| f.kind == NodeKind::Boss && f.sector == sector)?;
                Some(100.0 * fight.hull_before as f64 / fight.max_hull as f64)
            };
            mean(own.iter().filter_map(boss))
        };
        println!(
            "{:8} {:>5} {:>6.1} {:>9.2}  {:>6.1} {:>6.1} {:>6.1}  {:>5.0} {:>5.0} {:>5.0}  {:>5.1} {:>6.1} {:>7.0}",
            name(admiral),
            own.len(),
            win_rate(&own),
            mean(own.iter().map(|log| victories(log))),
            died(1),
            died(2),
            died(3),
            before_boss(1),
            before_boss(2),
            before_boss(3),
            mean(own.iter().map(|log| log.deck.len() as f64)),
            mean(own.iter().map(|log| log.modules.len() as f64)),
            mean(own.iter().map(|log| log.scrap as f64)),
        );
    }
}

fn report_nodes(logs: &[RunLog], maps: &Maps) {
    println!("\n== Knoten (Gefechte) ==");
    println!(
        "{:1} {:1} {:1} {:6} {:>7} {:>5} {:>7} {:>6} {:>6}  Verband",
        "S", "R", "K", "Art", "Besuche", "Tod%", "-Hülle", "-%max", "Runden"
    );
    let mut nodes = BTreeMap::new();
    for fight in logs.iter().flat_map(|log| &log.fights) {
        let node: &mut Vec<_> = nodes
            .entry((fight.sector, fight.row, fight.column))
            .or_default();
        node.push(fight);
    }
    for ((sector, row, column), fights) in nodes {
        let node = &maps[sector as usize - 1].rows[row][column];
        let lost = (fights.iter()).map(|f| (f.hull_before - f.hull_after) as f64);
        let share = (fights.iter())
            .map(|f| 100.0 * (f.hull_before - f.hull_after) as f64 / f.max_hull as f64);
        println!(
            "{sector} {row} {column} {:6} {:>7} {:>5.1} {:>7.1} {:>6.1} {:>6.1}  {}",
            name(node.kind),
            fights.len(),
            percent(fights.iter().filter(|f| !f.won).count(), fights.len()),
            mean(lost),
            mean(share),
            mean(fights.iter().map(|f| f.rounds as f64)),
            node.encounter.as_ref().map(describe).unwrap_or_default(),
        );
    }
}

fn report_cards(logs: &[RunLog]) {
    println!("\n== Karten: gespielt, wenn in der Hand ==");
    println!(
        "{:22} {:>8} {:>9} {:>6}",
        "Karte", "gezogen", "gespielt", "%"
    );
    let total = |pick: fn(&RunLog) -> &HashMap<_, u32>, card| -> u32 {
        logs.iter().filter_map(|log| pick(log).get(&card)).sum()
    };
    for card in LOOT_POOL {
        let (drawn, played) = (
            total(|log| &log.drawn, card),
            total(|log| &log.played, card),
        );
        println!(
            "{:22} {drawn:>8} {played:>9} {:>6.1}",
            name(card),
            percent(played as usize, drawn as usize)
        );
    }
}

/// Dieselben Runs mit jedem Eintrag aus `variants` als Start, gegen die Runs ohne.
fn compare(
    title: &str,
    variants: Vec<(String, Start)>,
    admirals: &[AdmiralId],
    runs: u64,
    seed: u64,
    maps: &Maps,
) {
    println!("\n== {title} ({runs} Runs je Admiral) ==");
    print!(
        "{:22} {:>6} {:>6} {:>9} {:>6} ",
        "", "Sieg%", "Δ", "Siege/Run", "Δ"
    );
    for &admiral in admirals {
        print!(" {:>7}", format!("Δ {}", name(admiral)));
    }
    println!();
    let measure = |start: &Start| {
        let logs = simulate(admirals, runs, seed, maps, start);
        let all: Vec<&RunLog> = logs.iter().collect();
        let per_admiral: Vec<f64> = (admirals.iter())
            .map(|&admiral| {
                mean(
                    logs.iter()
                        .filter(|log| log.admiral == admiral)
                        .map(victories),
                )
            })
            .collect();
        (
            win_rate(&all),
            mean(logs.iter().map(victories)),
            per_admiral,
        )
    };
    let base = measure(&Start::default());
    let variants = std::iter::once(("(ohne)".to_string(), Start::default())).chain(variants);
    for (label, start) in variants {
        let (wins, won, per_admiral) = measure(&start);
        print!(
            "{label:22} {wins:>6.1} {:>+6.1} {won:>9.2} {:>+6.2} ",
            wins - base.0,
            won - base.1
        );
        for (own, base) in per_admiral.iter().zip(&base.2) {
            print!(" {:>+7.2}", own - base);
        }
        println!();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let option = |flag: &str| {
        let at = args.iter().position(|arg| arg == flag)?;
        Some(
            args.get(at + 1)
                .unwrap_or_else(|| panic!("{flag} braucht einen Wert"))
                .as_str(),
        )
    };
    let runs: u64 = option("-n").map_or(300, |n| n.parse().expect("-n <Zahl>"));
    let seed: u64 = option("--seed").map_or(1, |s| s.parse().expect("--seed <Zahl>"));
    let admirals: Vec<AdmiralId> = match option("--admiral") {
        Some(id) => {
            vec![serde_json::from_value(Value::from(id)).expect("--admiral osei|vance|kerr|reyes")]
        }
        None => ADMIRAL_IDS.to_vec(),
    };
    let dir = Path::new(option("--data").unwrap_or(REPO_DATA));
    install(Data::load(dir).unwrap_or_else(|error| panic!("{error}"))).expect("geprüft");
    let maps = load_maps(dir);
    match args
        .first()
        .map(String::as_str)
        .filter(|mode| !mode.starts_with('-'))
    {
        None | Some("runs") => {
            let logs = simulate(&admirals, runs, seed, &maps, &Start::default());
            report_admirals(&logs, &admirals);
            report_nodes(&logs, &maps);
            report_cards(&logs);
        }
        Some("trace") => {
            bot::TRACE.store(true, Ordering::Relaxed);
            let log = play_run(admirals[0], seed, &maps, &Start::default());
            println!(
                "{}",
                if log.won {
                    "Run gewonnen"
                } else {
                    "Run verloren"
                }
            );
        }
        Some("cards") => {
            let variants = LOOT_POOL.map(|card| {
                (
                    name(card),
                    Start {
                        cards: vec![card],
                        ..Start::default()
                    },
                )
            });
            compare(
                "Eine Karte mehr im Startdeck",
                variants.to_vec(),
                &admirals,
                runs,
                seed,
                &maps,
            );
        }
        Some("modules") => {
            let variants = MODULES.map(|module| {
                (
                    name(module),
                    Start {
                        modules: vec![module],
                        ..Start::default()
                    },
                )
            });
            compare(
                "Ein Modul von Beginn an",
                variants.to_vec(),
                &admirals,
                runs,
                seed,
                &maps,
            );
        }
        Some(mode) => panic!("unbekannter Modus {mode}: runs, cards, modules oder trace"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein Run endet, und derselbe Seed ergibt denselben Run.
    #[test]
    fn run_ends_and_repeats() {
        let maps = load_maps(Path::new(REPO_DATA));
        let play = || play_run(AdmiralId::Osei, 7, &maps, &Start::default());
        let (first, second) = (play(), play());
        assert!(!first.fights.is_empty());
        assert_eq!(first.won, second.won);
        assert_eq!(first.deck, second.deck);
        assert_eq!(first.fights.len(), second.fights.len());
    }
}
