//! Der Bot: Er spielt ein Gefecht Zug für Zug und einen Run Knoten für Knoten.
//!
//! Im Gefecht probiert er jede Folge aus Karten und Stationen des Zugs auf einer Kopie aus ([`plan`]), lässt den
//! Gegnerzug laufen und bewertet, was danach steht ([`eval`]): die eigene Hülle minus den Schaden, den die übrigen
//! Gegner bis zu ihrem Ende noch anrichten ([`threat`]). Er rechnet also einen Zug genau und den Rest geschätzt; das
//! entspricht einem Spieler, der jede Vorschau liest, aber keinen Plan über mehrere Runden hat.
//!
//! Im Run entscheidet er nach einfachen festen Regeln ([`play_run`]): Beute zufällig, Werft nach Hülle. Das hält
//! die Messung frei von Vorlieben: Wie stark eine Karte ist, zeigt der Vergleich der Runs mit und ohne sie.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};

use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use voidfleet_core::admirals::{AdmiralId, Station};
use voidfleet_core::battle::{Battle, BattleResult, RuleError, Target};
use voidfleet_core::board::COLUMNS;
use voidfleet_core::cards::{CardId, CardTarget, CardType, DeckCard};
use voidfleet_core::data::{Step, rules};
use voidfleet_core::enemies::{Action, EnemyId, action};
use voidfleet_core::events::{BattleEvent, UnitRef};
use voidfleet_core::modules::ModuleId;
use voidfleet_core::run::{EventId, NodeKind, Run, SECTORS, SectorMap, Stage};

/// Ein Schritt des Spielerzugs.
#[derive(Debug, Clone, Copy)]
pub enum Move {
    Card(DeckCard, Target),
    Station(Station, Target),
}

const STATIONS: [Station; 5] = [
    Station::Weapons,
    Station::Shields,
    Station::DamageControl,
    Station::Maneuver,
    Station::Admiral,
];

/// So viele Stellungen sieht sich der Bot je Zug höchstens an.
const NODES: usize = 20_000;
/// Gewicht des geschätzten künftigen Schadens gegen die Hülle von jetzt. Geeicht: Mit diesem Wert gewinnt der Bot
/// am häufigsten (0,2 bis 2,5 probiert); darunter mauert er, bis die Gefechtsuhr ihn holt, darüber greift er an, wo
/// Schilde mehr gebracht hätten. Nach großen Änderungen an den Regeln neu eichen.
const THREAT: f64 = 0.5;
/// So viele Schritte hat ein Zug höchstens (Plündern für 0 Punkte zöge sonst das Deck im Kreis).
const STEPS: usize = 12;
/// So viele Gegnerzüge weit rechnet die Schätzung die Muster der Gegner aus.
const HORIZON: usize = 6;
/// Ein Gefecht, das so lange dauert, gilt als verloren (die Gefechtsuhr beendet jedes vorher).
const MAX_ROUNDS: u32 = 60;

fn apply(battle: &mut Battle, step: Move) -> Result<Vec<BattleEvent>, RuleError> {
    match step {
        Move::Card(card, target) => {
            let index = battle.hand.iter().position(|&held| held == card);
            battle.play_card(index.expect("Karte in der Hand"), target)
        }
        Move::Station(station, target) => battle.run_station(station, target),
    }
}

fn targets(battle: &Battle, kind: CardTarget) -> Vec<Target> {
    match kind {
        CardTarget::None => vec![Target::None],
        CardTarget::Enemy => (battle.enemies.iter())
            .map(|e| Target::Enemy(e.uid))
            .collect(),
        // Die Spalte zählt nur gegen Spaltenfeuer, das in diesem Gegnerzug fällt.
        CardTarget::Column => {
            let aimed =
                (battle.enemies.iter()).any(|e| matches!(e.intents[0], Action::ColumnFire { .. }));
            (0..COLUMNS)
                .filter(|&column| aimed && column != battle.flagship.column)
                .map(Target::Column)
                .collect()
        }
    }
}

fn moves(battle: &Battle) -> Vec<Move> {
    let mut moves = Vec::new();
    let mut cards: Vec<DeckCard> = Vec::new();
    for &card in &battle.hand {
        if card.cost() <= battle.points && !cards.contains(&card) {
            cards.push(card);
            let targets = targets(battle, card.id.target());
            moves.extend(targets.into_iter().map(|target| Move::Card(card, target)));
        }
    }
    for station in STATIONS {
        if battle.station_open(station).is_ok() {
            let targets = targets(battle, battle.station_target(station));
            moves.extend(
                targets
                    .into_iter()
                    .map(|target| Move::Station(station, target)),
            );
        }
    }
    moves
}

/// Was den Rest des Zugs bestimmt; gleiche Schlüssel, gleiche Stellung, also nur einmal ansehen.
fn key(battle: &Battle) -> u64 {
    let mut hasher = DefaultHasher::new();
    battle.flagship.hash(&mut hasher);
    battle.enemies.hash(&mut hasher);
    let mut hand = battle.hand.clone();
    hand.sort_by_key(|card| (card.id as u8, card.upgraded));
    hand.hash(&mut hasher);
    let piles = (battle.draw_pile.len(), battle.replacements.len());
    (battle.points, battle.kills, battle.result.is_some(), piles).hash(&mut hasher);
    battle.emergency_generator_used.hash(&mut hasher);
    hasher.finish()
}

struct Search {
    seen: HashSet<u64>,
    best: f64,
    line: Vec<Move>,
}

fn search(battle: &Battle, line: &mut Vec<Move>, search_state: &mut Search) {
    if search_state.seen.len() >= NODES || !search_state.seen.insert(key(battle)) {
        return;
    }
    let mut end = battle.clone();
    end.end_turn();
    let score = eval(&end);
    if score > search_state.best {
        search_state.best = score;
        search_state.line = line.clone();
    }
    if battle.result.is_some() || line.len() >= STEPS {
        return;
    }
    for step in moves(battle) {
        let mut next = battle.clone();
        if apply(&mut next, step).is_ok() {
            line.push(step);
            search(&next, line, search_state);
            line.pop();
        }
    }
}

/// Die beste Folge aus Karten und Stationen für diesen Zug.
// shortcut: Plündern zieht auf der Kopie dieselben Karten wie im Gefecht, der Bot kennt sie also vor dem Spielen;
// sobald Plündern auffällig stark misst, die Kopie vor dem Ziehen neu mischen.
pub fn plan(battle: &Battle) -> Vec<Move> {
    let mut state = Search {
        seen: HashSet::new(),
        best: f64::NEG_INFINITY,
        line: Vec::new(),
    };
    search(battle, &mut Vec::new(), &mut state);
    state.line
}

/// Der Wert einer Stellung zu Beginn eines Spielerzugs: die Hülle, mit der das Gefecht voraussichtlich endet.
fn eval(battle: &Battle) -> f64 {
    let hull = battle.flagship.hull as f64;
    match battle.result {
        Some(BattleResult::Lost) => -1000.0,
        Some(BattleResult::Won) => hull + 1.0,
        None => hull - THREAT * threat(battle),
    }
}

/// Eine gegnerische Einheit in der Schätzung: was sie aushält, was ihr Brand je Runde nimmt und was sie in den
/// nächsten Gegnerzügen am Flaggschiff anrichtet.
struct Foe {
    hull: f64,
    armor: i32,
    burn: f64,
    hits: [f64; HORIZON],
    seal: bool,
}

/// Was ein Gegner mit dieser ATK in den nächsten Gegnerzügen anrichtet. Spaltenfeuer kostet höchstens eine Station
/// (ausmanövriert), ein Ruf zählt pauschal.
fn hits(
    battle: &Battle,
    id: EnemyId,
    attack: i32,
    mut charged: bool,
    actions: [Action; HORIZON],
) -> [f64; HORIZON] {
    const DODGE: f64 = 6.0;
    const CALL: f64 = 3.0;
    let start = rules().battle.clock_start as i32 - 1;
    let armor = battle.flagship_armor();
    let mut out = [0.0; HORIZON];
    for (turn, action) in actions.into_iter().enumerate() {
        let clock = (battle.round as i32 + turn as i32 - start).max(0) - battle.clock();
        let attack = attack + clock;
        out[turn] = match action {
            Action::Fire => {
                let damage = attack * (1 + charged as i32) - armor;
                charged = false;
                damage.max(0) as f64
            }
            Action::ColumnFire { columns } => {
                let double = charged || id == EnemyId::TorpedoWing;
                charged = false;
                if turn < 2 && !columns[battle.flagship.column] {
                    0.0
                } else {
                    ((attack * (1 + double as i32) - armor).max(0) as f64).min(DODGE)
                }
            }
            Action::Ram => {
                out[turn] = (rules().enemies.ram_damage - armor).max(0) as f64;
                return out;
            }
            Action::Charge => {
                charged = true;
                0.0
            }
            Action::Call(_) => CALL,
            _ => 0.0,
        };
    }
    out
}

fn pattern_actions(
    pattern: &[Step],
    first: &[Action],
    step: u32,
    column: usize,
) -> [Action; HORIZON] {
    std::array::from_fn(|turn| match first.get(turn) {
        Some(&announced) => announced,
        None => action(pattern, step + (turn - first.len()) as u32, column),
    })
}

/// Der Schaden, den das Flaggschiff bis zum Ende des Gefechts noch nimmt: Die Gegner fallen in der Reihenfolge
/// Schaden je Hülle, jeder feuert, bis er fällt.
fn threat(battle: &Battle) -> f64 {
    let flagship = &battle.flagship;
    let attack = battle.flagship_attack();
    let (card_damage, card_block) = card_power(battle);
    let ours = attack as f64 + card_damage;
    let block = card_block + flagship.battle_stations as f64;
    let shooters = battle.enemies.iter().filter(|e| !e.id.support()).count();
    let mut foes: Vec<Foe> = (battle.enemies.iter())
        .map(|enemy| {
            let actions =
                pattern_actions(&enemy.pattern, &enemy.intents, enemy.step, flagship.column);
            let covered = battle.covered(enemy.uid) && flagship.admiral != AdmiralId::Kerr;
            Foe {
                // Deckung verdoppelt, was das Siegel aushält, solange der Vordermann steht.
                hull: ((enemy.hull + enemy.shield)
                    * (1 + (covered && enemy.id == EnemyId::Seal) as i32))
                    as f64,
                armor: enemy.armor,
                burn: if enemy.burning {
                    battle.burn_damage(enemy.unit()) as f64
                } else {
                    0.0
                },
                hits: if enemy.id.support() {
                    [shooters as f64; HORIZON]
                } else {
                    hits(
                        battle,
                        enemy.id,
                        battle.enemy_attack(enemy),
                        enemy.charged,
                        actions,
                    )
                },
                seal: enemy.id == EnemyId::Seal,
            }
        })
        .collect();
    let pending = (battle.reinforcements.iter().map(|r| &r.unit)).chain(&battle.replacements);
    foes.extend(pending.map(|unit| {
        let actions = pattern_actions(&unit.pattern, &[], 0, flagship.column);
        Foe {
            hull: unit.hull as f64,
            armor: unit.armor,
            burn: 0.0,
            hits: hits(battle, unit.id, unit.attack, false, actions),
            seal: false,
        }
    }));
    let rate = |foe: &Foe| foe.hits[..3].iter().sum::<f64>() / foe.hull.max(1.0);
    foes.sort_by(|a, b| rate(b).total_cmp(&rate(a)));
    let mut best = schedule(&foes, ours, attack, block);
    // Mit dem Siegel fällt die Gegenflotte: auch der direkte Weg zählt.
    if let Some(seal) = foes.iter().position(|foe| foe.seal) {
        foes.swap(0, seal);
        let direct = schedule(&foes, ours, attack, block);
        if direct.0 < best.0 {
            best = direct;
        }
    }
    let (mut damage, rounds) = best;
    if flagship.burning {
        damage += battle.burn_damage(UnitRef::Flagship) as f64 * rounds.min(3.0);
    }
    damage
}

/// Schaden und Dauer, wenn die Gegner in dieser Reihenfolge fallen; mit dem Siegel endet das Gefecht. Von dem, was
/// je Gegnerzug ankommt, schlucken die Schildkarten `block`.
fn schedule(foes: &[Foe], ours: f64, attack: i32, block: f64) -> (f64, f64) {
    let mut time = 0.0;
    let mut incoming = [0.0; HORIZON];
    let mut late = 0.0;
    let mut over = false;
    for foe in foes {
        if !over {
            let shot = attack.max(foe.armor + 1) as f64;
            let through = ours * (shot - foe.armor as f64) / shot;
            let hull = (foe.hull - foe.burn * time).max(0.0);
            time += hull / (through + foe.burn);
            over = foe.seal;
        }
        for (turn, hit) in foe.hits.iter().enumerate() {
            incoming[turn] += hit * (time - turn as f64 - 0.5).clamp(0.0, 1.0);
        }
        let rounds = time - HORIZON as f64 - 0.5;
        if rounds > 0.0 {
            late += rounds * (foe.hits.iter().sum::<f64>() / HORIZON as f64 - block).max(0.0);
        }
    }
    let damage: f64 = incoming.iter().map(|hit| (hit - block).max(0.0)).sum();
    (damage + late, time)
}

/// Was die Karten des Decks je Runde etwa bringen, soweit die Befehlspunkte reichen: Schaden der Angriffskarten und
/// Schild der Schildkarten.
fn card_power(battle: &Battle) -> (f64, f64) {
    let cards = (battle.hand.iter())
        .chain(&battle.draw_pile)
        .chain(&battle.discard_pile);
    let (mut damage, mut block, mut cost, mut count) = (0.0, 0.0, 0.0, 0.0);
    for card in cards {
        count += 1.0;
        cost += card.cost() as f64;
        let value = (card.value() + battle.card_bonus(card.id)) as f64;
        match card.id {
            CardId::EmergencyShields | CardId::HoldTheLine => block += value,
            CardId::VolleyFire => damage += value * battle.enemies.len() as f64,
            CardId::Barrage => damage += value * 0.5,
            CardId::CollisionCourse => damage += value + 2.0 * battle.flagship_attack() as f64,
            CardId::Firebomb => damage += value + 2.0 * rules().battle.burn_damage as f64,
            _ if card.id.card_type() == CardType::Attack => damage += value,
            _ => {}
        }
    }
    if count == 0.0 {
        return (0.0, 0.0);
    }
    let hand = rules().battle.hand_size as f64 / count;
    let points = rules().battle.command_points as f64;
    let played = hand * (points / (cost * hand).max(points));
    (damage * played, block * played)
}

// --- Der Run ---

/// Womit ein Run abweichend beginnt: zusätzliche Karten im Startdeck und Module (für den Vergleich mit und ohne).
#[derive(Debug, Clone, Default)]
pub struct Start {
    pub cards: Vec<CardId>,
    pub modules: Vec<ModuleId>,
}

/// Ein Gefecht eines Runs.
#[derive(Debug, Clone)]
pub struct Fight {
    pub sector: u32,
    pub row: usize,
    pub column: usize,
    pub kind: NodeKind,
    pub hull_before: i32,
    pub hull_after: i32,
    pub max_hull: i32,
    pub rounds: u32,
    pub won: bool,
}

/// Was ein Run ergeben hat.
#[derive(Debug, Clone)]
pub struct RunLog {
    pub admiral: AdmiralId,
    pub won: bool,
    pub fights: Vec<Fight>,
    pub deck: Vec<DeckCard>,
    pub modules: Vec<ModuleId>,
    pub scrap: u32,
    /// Je Karte: wie oft sie in der Hand war und wie oft gespielt.
    pub drawn: HashMap<CardId, u32>,
    pub played: HashMap<CardId, u32>,
}

/// `voidfleet-sim trace`: jeder Zug des Bots auf der Konsole.
pub static TRACE: AtomicBool = AtomicBool::new(false);

fn trace(battle: &Battle, steps: &[Move]) {
    let flagship = &battle.flagship;
    println!(
        "  Runde {}: Hülle {}/{}{}, Spalte {}, Hand {:?}",
        battle.round,
        flagship.hull,
        flagship.max_hull,
        if flagship.burning { " brennt" } else { "" },
        flagship.column,
        (battle.hand.iter())
            .map(|card| (card.id, card.upgraded))
            .collect::<Vec<_>>()
    );
    for enemy in &battle.enemies {
        let [next, then] = battle.announced(enemy.uid);
        println!(
            "    #{} {:?} {:?}{} Hülle {} Schild {}: {:?} {}, dann {:?}",
            enemy.uid,
            enemy.id,
            enemy.slot.row,
            enemy.slot.column,
            enemy.hull,
            enemy.shield,
            next.action,
            next.damage,
            then.action
        );
    }
    println!("    -> {steps:?}");
}

/// Spielt das Gefecht zu Ende und zählt gezogene und gespielte Karten in `log`.
pub fn fight(battle: &mut Battle, log: &mut RunLog) {
    let count = |events: &[BattleEvent], log: &mut RunLog| {
        for event in events {
            if let BattleEvent::Drawn { cards } = event {
                for card in cards {
                    *log.drawn.entry(card.id).or_default() += 1;
                }
            }
        }
    };
    for card in &battle.hand {
        *log.drawn.entry(card.id).or_default() += 1;
    }
    while battle.result.is_none() {
        if battle.round > MAX_ROUNDS {
            battle.result = Some(BattleResult::Lost);
            break;
        }
        let steps = plan(battle);
        if TRACE.load(Ordering::Relaxed) {
            trace(battle, &steps);
        }
        for step in steps {
            if let Move::Card(card, _) = step {
                *log.played.entry(card.id).or_default() += 1;
            }
            let events = apply(battle, step).expect("geplanter Schritt gilt");
            count(&events, log);
        }
        let events = battle.end_turn();
        count(&events, log);
    }
}

/// Spielt einen Run bis zum Sieg oder zur Niederlage.
pub fn play_run(
    admiral: AdmiralId,
    seed: u64,
    maps: &[SectorMap; SECTORS as usize],
    start: &Start,
) -> RunLog {
    let mut run = Run::new(admiral, seed, maps.clone());
    run.deck
        .extend(start.cards.iter().copied().map(DeckCard::new));
    for &module in &start.modules {
        run.modules.push(module);
        if module == ModuleId::ReinforcedFrames {
            run.max_hull += rules().modules.reinforced_frames_hull;
            run.hull = run.max_hull;
        }
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5EED);
    let mut log = RunLog {
        admiral,
        won: false,
        fights: Vec::new(),
        deck: Vec::new(),
        modules: Vec::new(),
        scrap: 0,
        drawn: HashMap::new(),
        played: HashMap::new(),
    };
    let low = |run: &Run, percent: i32| run.hull * 100 < run.max_hull * percent;
    while run.result.is_none() {
        match run.stage.clone() {
            Stage::Map => {
                // Angeschlagen zur Werft und um die Elite herum, sonst zufällig.
                let row = run.path.len();
                let score = |&column: &usize| match run.map[row][column].kind {
                    NodeKind::Shipyard if low(&run, 60) => 1,
                    NodeKind::Elite if low(&run, 50) => -1,
                    _ => 0,
                };
                let choices = run.choices();
                let best = choices.iter().map(score).max().expect("Kante");
                let good: Vec<usize> = choices.into_iter().filter(|c| score(c) == best).collect();
                run.advance(*good.choose(&mut rng).expect("Kante"))
                    .expect("wählbar");
            }
            Stage::Battle { .. } => {
                let mut battle = run.start_battle();
                if TRACE.load(Ordering::Relaxed) {
                    let (row, column) = run.current().expect("auf der Karte");
                    println!(
                        "Sektor {} Reihe {row} Knoten {column}, Deck {}, Module {:?}",
                        run.sector,
                        run.deck.len(),
                        run.modules
                    );
                }
                fight(&mut battle, &mut log);
                let (row, column) = run.current().expect("auf der Karte");
                log.fights.push(Fight {
                    sector: run.sector,
                    row,
                    column,
                    kind: run.map[row][column].kind,
                    hull_before: run.hull,
                    hull_after: battle.flagship.hull,
                    max_hull: run.max_hull,
                    rounds: battle.round,
                    won: battle.result == Some(BattleResult::Won),
                });
                run.finish_battle(&battle);
            }
            // Eine Karte oder keine, gleich wahrscheinlich; ein Modul immer.
            Stage::Loot(loot) => {
                let pick = if loot.cards.is_empty() {
                    (!loot.modules.is_empty()).then(|| rng.random_range(0..loot.modules.len()))
                } else {
                    Some(rng.random_range(0..=loot.cards.len())).filter(|&i| i < loot.cards.len())
                };
                run.take_loot(pick).expect("Beute");
            }
            Stage::Event(event) => {
                let option = match event {
                    // Die Logbücher: ein Modul ohne Preis.
                    EventId::Resolute => 1,
                    EventId::Pilot if low(&run, 50) && run.scrap >= rules().run.pilot_price => 0,
                    EventId::Pilot => 1,
                    EventId::Flicker if low(&run, 60) => 1,
                    EventId::Flicker => 0,
                };
                run.choose(option).expect("Ereignis");
            }
            Stage::Shipyard => {
                let open: Vec<usize> = (0..run.deck.len())
                    .filter(|&i| !run.deck[i].upgraded)
                    .collect();
                match open.choose(&mut rng) {
                    Some(&index) if !low(&run, 60) => run.upgrade_at_shipyard(index),
                    _ => run.repair_at_shipyard(),
                }
                .expect("Werft");
            }
            Stage::Market(_) => {
                while low(&run, 50) && run.buy_repair().is_ok() {}
                while run.buy_module(0).is_ok() {}
                let _ = run.buy_card(0);
                run.leave_market().expect("Schwarzmarkt");
            }
        }
    }
    log.won = run.result == Some(BattleResult::Won);
    log.deck = run.deck;
    log.modules = run.modules;
    log.scrap = run.scrap;
    log
}
