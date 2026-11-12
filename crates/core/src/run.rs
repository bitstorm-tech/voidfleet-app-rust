//! Der Run (`docs/run.md`, `docs/gegner.md`): drei Sektoren mit je einer Sternenkarte, die Gefechte gegen die Verbände
//! des Sektors, Beute, Werft, Schwarzmarkt und Ereignisse. Der Zustand liegt in öffentlichen Feldern, jeder Schritt
//! ist eine Methode, die eine ungültige Wahl mit [`RunError`] ablehnt. Der Zufall hängt nur an `seed` und `rolls`,
//! also ist ein Run mit Seed wiederholbar und nach dem Laden derselbe.

use std::ops::RangeInclusive;

use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::admirals::AdmiralId;
use crate::battle::{Battle, BattleResult, BattleSetup, Encounter, Reinforcement};
use crate::board::{COLUMNS, EnemySlot, Row};
use crate::cards::{CardId, DeckCard, LOOT_POOL, starter_deck};
use crate::enemies::{EnemyClass, EnemyId};
use crate::modules::{ModuleId, REINFORCED_FRAMES_HULL, SALVAGE_CRANE_SCRAP};

pub const SECTORS: u32 = 3;
/// Reihen je Sektor; die letzte ist der Boss.
pub const ROWS: usize = 9;
pub const NODES_PER_ROW: RangeInclusive<usize> = 3..=4;
pub const BATTLE_SCRAP: RangeInclusive<u32> = 10..=20;
pub const ELITE_SCRAP: RangeInclusive<u32> = 25..=40;
pub const BOSS_SCRAP: u32 = 60;
pub const CARD_OFFER: usize = 3;
pub const COMBAT_SIMULATOR_OFFER: usize = 4;
pub const MODULE_OFFER: usize = 3;
/// Werft und Boss: so viel Prozent der maximalen Hülle, aufgerundet.
pub const SHIPYARD_REPAIR_PERCENT: i32 = 30;
pub const MARKET_CARDS: usize = 3;
pub const MARKET_MODULES: usize = 2;
pub const CARD_PRICE: RangeInclusive<u32> = 40..=70;
pub const MODULE_PRICE: RangeInclusive<u32> = 120..=180;
pub const REMOVAL_PRICE: u32 = 60;
/// Reparatur im Schwarzmarkt: so viel Hülle für so viel Schrott.
pub const MARKET_REPAIR: i32 = 10;
pub const MARKET_REPAIR_PRICE: u32 = 25;
pub const RESOLUTE_SCRAP: u32 = 40;
pub const PILOT_PRICE: u32 = 50;
pub const PILOT_DAMAGE: i32 = 6;
/// Ab Sektor 2 je Sektor ein Viertel mehr Hülle für die Gegner.
pub const SECTOR_HULL_PERCENT: u32 = 25;
/// Angesagte Verstärkung in Sektor 3 trifft in dieser Runde ein („in zwei Runden“).
pub const REINFORCEMENT_ROUND: u32 = 3;

/// Alle Module, aus denen Angebote und die Logbücher der Resolute ziehen.
pub const MODULES: [ModuleId; 9] = {
    use ModuleId::*;
    [
        FireControlComputer,
        FireSuppression,
        ReserveReactor,
        ArmorPlates,
        EmergencyGenerator,
        SalvageCrane,
        SupplyPod,
        CombatSimulator,
        ReinforcedFrames,
    ]
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Battle,
    Elite,
    Event,
    Shipyard,
    Market,
    Boss,
}

/// Ein Knoten der Sternenkarte; Reihe und Spalte sind seine Indizes in [`Run::map`], `next` die Spalten der
/// verbundenen Knoten in der nächsten Reihe (aufsteigend).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapNode {
    pub kind: NodeKind,
    pub next: Vec<usize>,
}

/// Das Wrack der Resolute, der Lotse, Flackern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventId {
    Resolute,
    Pilot,
    Flicker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOption {
    /// Resolute bergen: Schrott, das nächste Gefecht beginnt brennend.
    Salvage,
    /// Logbücher sichern: ein zufälliges noch nicht besessenes Modul.
    Logbooks,
    /// Weiterfliegen und die Crew bestatten: nichts.
    MoveOn,
    /// Den Lotsen bezahlen: der nächste gewählte Knoten (außer dem Boss) wird eine Werft.
    PayPilot,
    /// Selbst fliegen: Hülle verlieren, dafür ein Kartenangebot.
    FlyYourself,
    /// Ins Flackern: ein Viertel der maximalen Hülle verlieren, eine zufällige Karte wird aufgewertet.
    Embrace,
    /// Nichts geschieht.
    Listen,
}

impl EventId {
    pub const ALL: [EventId; 3] = [EventId::Resolute, EventId::Pilot, EventId::Flicker];

    pub fn options(self) -> &'static [EventOption] {
        use EventOption::*;
        match self {
            EventId::Resolute => &[Salvage, Logbooks, MoveOn],
            EventId::Pilot => &[PayPilot, FlyYourself],
            EventId::Flicker => &[Embrace, Listen],
        }
    }
}

/// Was eine Entscheidung im Ereignis außer Schrott und Hülle gebracht hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gain {
    Nothing,
    Module(ModuleId),
    Upgraded(CardId),
}

/// Beute nach einem Gefecht (oder das Kartenangebot des Lotsen-Ereignisses). Schrott und Reparatur sind schon
/// verbucht; zu wählen ist eine Karte aus `cards` oder ein Modul aus `modules`, nie beides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loot {
    pub scrap: u32,
    /// Hülle, die der Boss nach Sektor 1 und 2 repariert hat.
    pub repaired: i32,
    pub cards: Vec<CardId>,
    pub modules: Vec<ModuleId>,
}

/// Der Bestand eines Schwarzmarkts samt Preisen; Gekauftes verschwindet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Market {
    pub cards: Vec<(CardId, u32)>,
    pub modules: Vec<(ModuleId, u32)>,
    /// Karte entfernen ist noch zu haben (einmal je Markt).
    pub removal: bool,
}

/// Was am aktuellen Knoten ansteht.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Auf der Sternenkarte: der nächste Knoten ist zu wählen.
    Map,
    /// Ein Gefecht, Elite oder Boss: [`Run::start_battle`], danach [`Run::finish_battle`].
    Battle {
        encounter: Encounter,
        seed: u64,
    },
    Loot(Loot),
    Event(EventId),
    Shipyard,
    Market(Market),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    /// Der Schritt passt nicht zu dem, was gerade ansteht, oder der Run ist vorbei.
    WrongStage,
    /// Der Knoten ist vom aktuellen aus nicht erreichbar.
    InvalidNode,
    NotEnoughScrap,
    OutOfRange,
    AlreadyUpgraded,
    SoldOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub admiral: AdmiralId,
    pub seed: u64,
    /// Zahl der bisherigen Zufallsentscheidungen; jede zieht ihren eigenen RNG aus `seed` und `rolls`.
    pub rolls: u64,
    pub hull: i32,
    /// Samt Verstärkter Spanten.
    pub max_hull: i32,
    pub scrap: u32,
    pub deck: Vec<DeckCard>,
    pub modules: Vec<ModuleId>,
    pub emergency_generator_used: bool,
    /// 1 bis 3.
    pub sector: u32,
    /// Die Sternenkarte des Sektors, Reihe für Reihe; die letzte Reihe ist der Boss.
    pub map: Vec<Vec<MapNode>>,
    /// Die Spalte des besuchten Knotens je Reihe; der letzte ist der aktuelle, leer vor Reihe 1.
    pub path: Vec<usize>,
    pub stage: Stage,
    /// Wrack der Resolute geborgen: das nächste Gefecht beginnt brennend.
    pub burning: bool,
    /// Lotse bezahlt: der nächste gewählte Knoten außer dem Boss wird eine Werft.
    pub pilot: bool,
    /// Gewonnene Gefechte dieses Runs.
    pub victories: u32,
    pub result: Option<BattleResult>,
}

impl Run {
    pub fn new(admiral: AdmiralId, seed: u64) -> Self {
        let mut run = Run {
            admiral,
            seed,
            rolls: 0,
            hull: admiral.hull(),
            max_hull: admiral.hull(),
            scrap: 0,
            deck: starter_deck(admiral),
            modules: Vec::new(),
            emergency_generator_used: false,
            sector: 1,
            map: Vec::new(),
            path: Vec::new(),
            stage: Stage::Map,
            burning: false,
            pilot: false,
            victories: 0,
            result: None,
        };
        run.map = generate_map(&mut run.rng());
        run
    }

    // --- Abfragen ---

    pub fn has(&self, module: ModuleId) -> bool {
        self.modules.contains(&module)
    }

    /// Reihe und Spalte des aktuellen Knotens; `None` vor Reihe 1.
    pub fn current(&self) -> Option<(usize, usize)> {
        self.path
            .last()
            .map(|&column| (self.path.len() - 1, column))
    }

    pub fn visited(&self, row: usize, column: usize) -> bool {
        self.path.get(row) == Some(&column)
    }

    /// Die wählbaren Spalten der nächsten Reihe (unabhängig davon, ob gerade gewählt werden darf).
    pub fn choices(&self) -> Vec<usize> {
        match self.current() {
            None => (0..self.map[0].len()).collect(),
            Some((row, column)) => self.map[row][column].next.clone(),
        }
    }

    // --- Sternenkarte ---

    /// Fliegt zum Knoten `column` der nächsten Reihe; dort steht dann ein Gefecht, ein Ereignis, die Werft oder der
    /// Schwarzmarkt an ([`Run::stage`]).
    pub fn advance(&mut self, column: usize) -> Result<(), RunError> {
        if self.result.is_some() || self.stage != Stage::Map {
            return Err(RunError::WrongStage);
        }
        if !self.choices().contains(&column) {
            return Err(RunError::InvalidNode);
        }
        self.path.push(column);
        let row = self.path.len() - 1;
        let node = &mut self.map[row][column];
        if self.pilot && node.kind != NodeKind::Boss {
            node.kind = NodeKind::Shipyard;
            self.pilot = false;
        }
        let kind = node.kind;
        let mut rng = self.rng();
        self.stage = match kind {
            NodeKind::Battle | NodeKind::Elite | NodeKind::Boss => Stage::Battle {
                encounter: encounter(kind, self.sector, &mut rng),
                seed: rng.random(),
            },
            NodeKind::Event => Stage::Event(*EventId::ALL.choose(&mut rng).expect("drei")),
            NodeKind::Shipyard => Stage::Shipyard,
            NodeKind::Market => Stage::Market(Market {
                cards: LOOT_POOL
                    .choose_multiple(&mut rng, MARKET_CARDS)
                    .map(|&card| (card, rng.random_range(CARD_PRICE)))
                    .collect(),
                modules: self
                    .missing_modules()
                    .choose_multiple(&mut rng, MARKET_MODULES)
                    .map(|&module| (module, rng.random_range(MODULE_PRICE)))
                    .collect(),
                removal: true,
            }),
        };
        Ok(())
    }

    // --- Gefecht ---

    /// Baut das Gefecht des aktuellen Knotens. Ändert den Run nicht, also liefert ein erneuter Aufruf (etwa nach
    /// dem Laden) dasselbe Gefecht. Panikt, wenn kein Gefecht ansteht.
    pub fn start_battle(&self) -> Battle {
        let Stage::Battle { encounter, seed } = &self.stage else {
            panic!("kein Gefecht an diesem Knoten");
        };
        let mut battle = Battle::new(
            BattleSetup {
                admiral: self.admiral,
                hull: self.hull,
                max_hull: self.max_hull,
                deck: self.deck.clone(),
                modules: self.modules.clone(),
                emergency_generator_available: self.has(ModuleId::EmergencyGenerator)
                    && !self.emergency_generator_used,
                encounter: encounter.clone(),
            },
            *seed,
        );
        battle.flagship.burning = self.burning;
        battle
    }

    /// Übernimmt das beendete Gefecht: Hülle, Notaggregat, Schrott und Beute. Nach dem Boss von Sektor 3 ist der
    /// Run gewonnen, nach einer Niederlage verloren; dann ist die Beute leer.
    pub fn finish_battle(&mut self, battle: &Battle) -> Loot {
        let kind = self.current_kind();
        self.hull = battle.flagship.hull;
        self.burning = false;
        self.emergency_generator_used |= battle.emergency_generator_used;
        if battle.result == Some(BattleResult::Lost) {
            self.result = Some(BattleResult::Lost);
            self.stage = Stage::Map;
            return Loot::default();
        }
        self.victories += 1;
        let mut rng = self.rng();
        let mut loot = Loot {
            scrap: match kind {
                NodeKind::Elite => rng.random_range(ELITE_SCRAP),
                NodeKind::Boss => BOSS_SCRAP,
                _ => rng.random_range(BATTLE_SCRAP),
            },
            ..Loot::default()
        };
        if self.has(ModuleId::SalvageCrane) {
            loot.scrap += SALVAGE_CRANE_SCRAP * battle.kills;
        }
        self.scrap += loot.scrap;
        if kind == NodeKind::Boss && self.sector == SECTORS {
            self.result = Some(BattleResult::Won);
            self.stage = Stage::Map;
            return loot;
        }
        if kind == NodeKind::Battle {
            loot.cards = self.card_offer(&mut rng);
        } else {
            loot.modules = self
                .missing_modules()
                .choose_multiple(&mut rng, MODULE_OFFER)
                .copied()
                .collect();
        }
        if kind == NodeKind::Boss {
            let before = self.hull;
            self.repair(self.shipyard_repair());
            loot.repaired = self.hull - before;
        }
        self.stage = Stage::Loot(loot.clone());
        loot
    }

    /// Nimmt die Karte oder das Modul `index` der Beute, oder mit `None` nichts.
    pub fn take_loot(&mut self, index: Option<usize>) -> Result<(), RunError> {
        let Stage::Loot(loot) = &self.stage else {
            return Err(RunError::WrongStage);
        };
        if let Some(index) = index {
            if loot.cards.is_empty() {
                let module = *loot.modules.get(index).ok_or(RunError::OutOfRange)?;
                self.add_module(module);
            } else {
                let card = *loot.cards.get(index).ok_or(RunError::OutOfRange)?;
                self.deck.push(DeckCard::new(card));
            }
        }
        self.leave();
        Ok(())
    }

    // --- Werft ---

    /// +30 % der maximalen Hülle, aufgerundet.
    pub fn shipyard_repair(&self) -> i32 {
        (self.max_hull * SHIPYARD_REPAIR_PERCENT + 99) / 100
    }

    pub fn repair_at_shipyard(&mut self) -> Result<(), RunError> {
        self.at_shipyard()?;
        self.repair(self.shipyard_repair());
        self.leave();
        Ok(())
    }

    pub fn upgrade_at_shipyard(&mut self, index: usize) -> Result<(), RunError> {
        self.at_shipyard()?;
        let card = self.deck.get_mut(index).ok_or(RunError::OutOfRange)?;
        if card.upgraded {
            return Err(RunError::AlreadyUpgraded);
        }
        card.upgraded = true;
        self.leave();
        Ok(())
    }

    pub fn remove_at_shipyard(&mut self, index: usize) -> Result<(), RunError> {
        self.at_shipyard()?;
        if index >= self.deck.len() {
            return Err(RunError::OutOfRange);
        }
        self.deck.remove(index);
        self.leave();
        Ok(())
    }

    // --- Schwarzmarkt ---

    pub fn buy_card(&mut self, index: usize) -> Result<(), RunError> {
        let market = self.market()?;
        let (card, price) = *market.cards.get(index).ok_or(RunError::OutOfRange)?;
        self.pay(price)?;
        self.market()?.cards.remove(index);
        self.deck.push(DeckCard::new(card));
        Ok(())
    }

    pub fn buy_module(&mut self, index: usize) -> Result<(), RunError> {
        let market = self.market()?;
        let (module, price) = *market.modules.get(index).ok_or(RunError::OutOfRange)?;
        self.pay(price)?;
        self.market()?.modules.remove(index);
        self.add_module(module);
        Ok(())
    }

    /// Entfernt die Karte `index` des Decks, einmal je Markt.
    pub fn buy_removal(&mut self, index: usize) -> Result<(), RunError> {
        if !self.market()?.removal {
            return Err(RunError::SoldOut);
        }
        if index >= self.deck.len() {
            return Err(RunError::OutOfRange);
        }
        self.pay(REMOVAL_PRICE)?;
        self.market()?.removal = false;
        self.deck.remove(index);
        Ok(())
    }

    /// 10 Hülle für 25 Schrott, beliebig oft.
    pub fn buy_repair(&mut self) -> Result<(), RunError> {
        self.market()?;
        self.pay(MARKET_REPAIR_PRICE)?;
        self.repair(MARKET_REPAIR);
        Ok(())
    }

    pub fn leave_market(&mut self) -> Result<(), RunError> {
        self.market()?;
        self.leave();
        Ok(())
    }

    // --- Ereignisse ---

    /// Trifft die Entscheidung `option` (Index in [`EventId::options`]).
    pub fn choose(&mut self, option: usize) -> Result<Gain, RunError> {
        let Stage::Event(event) = self.stage else {
            return Err(RunError::WrongStage);
        };
        let option = *event.options().get(option).ok_or(RunError::OutOfRange)?;
        let mut gain = Gain::Nothing;
        match option {
            EventOption::Salvage => {
                self.scrap += RESOLUTE_SCRAP;
                self.burning = true;
            }
            EventOption::Logbooks => {
                if let Some(&module) = self.missing_modules().choose(&mut self.rng()) {
                    self.add_module(module);
                    gain = Gain::Module(module);
                }
            }
            EventOption::PayPilot => {
                self.pay(PILOT_PRICE)?;
                self.pilot = true;
            }
            EventOption::FlyYourself => {
                self.hull = (self.hull - PILOT_DAMAGE).max(1);
                let mut rng = self.rng();
                let cards = self.card_offer(&mut rng);
                self.stage = Stage::Loot(Loot {
                    cards,
                    ..Loot::default()
                });
                return Ok(gain);
            }
            EventOption::Embrace => {
                self.hull = (self.hull - (self.max_hull + 3) / 4).max(1);
                let open: Vec<usize> = (0..self.deck.len())
                    .filter(|&i| !self.deck[i].upgraded)
                    .collect();
                if let Some(&i) = open.choose(&mut self.rng()) {
                    self.deck[i].upgraded = true;
                    gain = Gain::Upgraded(self.deck[i].id);
                }
            }
            EventOption::MoveOn | EventOption::Listen => {}
        }
        self.leave();
        Ok(gain)
    }

    // --- Intern ---

    fn rng(&mut self) -> StdRng {
        let rng = StdRng::seed_from_u64(self.seed ^ self.rolls.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        self.rolls += 1;
        rng
    }

    fn current_kind(&self) -> NodeKind {
        let (row, column) = self.current().expect("auf der Karte");
        self.map[row][column].kind
    }

    fn missing_modules(&self) -> Vec<ModuleId> {
        MODULES.into_iter().filter(|&m| !self.has(m)).collect()
    }

    fn card_offer(&self, rng: &mut StdRng) -> Vec<CardId> {
        let size = if self.has(ModuleId::CombatSimulator) {
            COMBAT_SIMULATOR_OFFER
        } else {
            CARD_OFFER
        };
        LOOT_POOL.choose_multiple(rng, size).copied().collect()
    }

    fn add_module(&mut self, module: ModuleId) {
        self.modules.push(module);
        if module == ModuleId::ReinforcedFrames {
            self.max_hull += REINFORCED_FRAMES_HULL;
            self.hull += REINFORCED_FRAMES_HULL;
        }
    }

    fn repair(&mut self, amount: i32) {
        self.hull = (self.hull + amount).min(self.max_hull);
    }

    fn pay(&mut self, price: u32) -> Result<(), RunError> {
        self.scrap = self
            .scrap
            .checked_sub(price)
            .ok_or(RunError::NotEnoughScrap)?;
        Ok(())
    }

    fn at_shipyard(&self) -> Result<(), RunError> {
        match self.stage {
            Stage::Shipyard => Ok(()),
            _ => Err(RunError::WrongStage),
        }
    }

    fn market(&mut self) -> Result<&mut Market, RunError> {
        match &mut self.stage {
            Stage::Market(market) => Ok(market),
            _ => Err(RunError::WrongStage),
        }
    }

    /// Zurück auf die Karte; nach dem Boss beginnt der nächste Sektor.
    fn leave(&mut self) {
        self.stage = Stage::Map;
        if self.current_kind() == NodeKind::Boss {
            self.sector += 1;
            self.path.clear();
            self.map = generate_map(&mut self.rng());
        }
    }
}

/// Neun Reihen: Reihe 1 nur Gefechte, Reihen 2 bis 8 nach Anteilen (Elite erst ab Reihe 3), mindestens eine Werft,
/// zuletzt der Boss. Kanten verbinden benachbarte Reihen als Treppe ohne Überkreuzung: Jeder Knoten hat mindestens
/// eine Kante hinein und hinaus.
fn generate_map(rng: &mut StdRng) -> Vec<Vec<MapNode>> {
    let mut kinds: Vec<Vec<NodeKind>> = (0..ROWS - 1)
        .map(|row| {
            (0..rng.random_range(NODES_PER_ROW))
                .map(|_| match (row, rng.random_range(0..100)) {
                    (0, _) => NodeKind::Battle,
                    (_, 0..50) => NodeKind::Battle,
                    (1, 50..62) => NodeKind::Battle,
                    (_, 50..62) => NodeKind::Elite,
                    (_, 62..82) => NodeKind::Event,
                    (_, 82..94) => NodeKind::Shipyard,
                    _ => NodeKind::Market,
                })
                .collect()
        })
        .collect();
    if !kinds.iter().flatten().any(|&k| k == NodeKind::Shipyard) {
        let row = rng.random_range(1..ROWS - 1);
        let column = rng.random_range(0..kinds[row].len());
        kinds[row][column] = NodeKind::Shipyard;
    }
    kinds.push(vec![NodeKind::Boss]);
    let mut map: Vec<Vec<MapNode>> = kinds
        .iter()
        .map(|row| {
            row.iter()
                .map(|&kind| MapNode {
                    kind,
                    next: Vec::new(),
                })
                .collect()
        })
        .collect();
    for row in 0..ROWS - 1 {
        let (from, to) = (map[row].len() - 1, map[row + 1].len() - 1);
        let (mut i, mut j) = (0, 0);
        map[row][0].next.push(0);
        while (i, j) != (from, to) {
            match (i == from, j == to, rng.random_range(0..3)) {
                (true, _, _) | (false, false, 0) => j += 1,
                (_, true, _) | (false, false, 1) => i += 1,
                _ => (i, j) = (i + 1, j + 1),
            }
            map[row][i].next.push(j);
        }
    }
    map
}

/// Der Verband eines Knotens: Gefecht nach Sektor, Elite und Boss fest.
fn encounter(kind: NodeKind, sector: u32, rng: &mut StdRng) -> Encounter {
    use EnemyId::*;
    use Row::*;
    let fixed = |units: &[(EnemyId, Row, usize)]| -> Vec<(EnemyId, EnemySlot)> {
        units
            .iter()
            .map(|&(id, row, column)| (id, EnemySlot::new(row, column)))
            .collect()
    };
    let mut reinforcements = Vec::new();
    let mut replacements = Vec::new();
    let units = match (kind, sector) {
        (NodeKind::Elite, 1) => fixed(&[(Besieger, Front, 1)]),
        (NodeKind::Elite, 2) => fixed(&[
            (GuardCruiser, Front, 1),
            (GuardCruiser, Front, 0),
            (Besieger, Back, 1),
        ]),
        (NodeKind::Elite, _) => fixed(&[
            (Besieger, Front, 1),
            (Besieger, Front, 0),
            (Spotter, Back, 1),
        ]),
        (NodeKind::Boss, 1) => fixed(&[
            (Raiders, Front, 1),
            (Raiders, Front, 0),
            (ScrapKing, Back, 1),
        ]),
        (NodeKind::Boss, 2) => fixed(&[
            (GuardCruiser, Front, 1),
            (GuardCruiser, Front, 0),
            (Warden, Back, 1),
        ]),
        (NodeKind::Boss, _) => {
            replacements = vec![GuardCruiser, GuardCruiser];
            fixed(&[
                (GuardCruiser, Front, 1),
                (PlasmaCruiser, Front, 0),
                (Seal, Back, 1),
                (Spotter, Back, 0),
            ])
        }
        _ => {
            let (pool, sizes): (&[EnemyId], RangeInclusive<usize>) = match sector {
                1 => (&[Raiders, GuardCruiser, Fireship], 1..=2),
                2 => (
                    &[
                        Raiders,
                        Fireship,
                        TorpedoWing,
                        GuardCruiser,
                        PlasmaCruiser,
                        Spotter,
                        Besieger,
                    ],
                    2..=3,
                ),
                _ => (
                    &[
                        Raiders,
                        TorpedoWing,
                        GuardCruiser,
                        PlasmaCruiser,
                        Spotter,
                        Besieger,
                        Carrier,
                    ],
                    3..=4,
                ),
            };
            let mut size = rng.random_range(sizes);
            // Sektor 3: bei der Hälfte der Verbände kommen ein oder zwei Wachkreuzer als angesagte Verstärkung.
            if sector == SECTORS && rng.random_bool(0.5) {
                let count = rng.random_range(1..=size - 2);
                size -= count;
                reinforcements = vec![
                    Reinforcement {
                        round: REINFORCEMENT_ROUND,
                        unit: GuardCruiser,
                    };
                    count
                ];
            }
            // Höchstens ein Spotter und ein Schlachtschiff je Verband.
            let limited = |id: EnemyId| id == Spotter || id.class() == EnemyClass::Battleship;
            let mut units: Vec<EnemyId> = Vec::new();
            while units.len() < size {
                let id = *pool.choose(rng).expect("nicht leer");
                if !limited(id) || !units.iter().any(|&u| limited(u) && u.class() == id.class()) {
                    units.push(id);
                }
            }
            formation(&units)
        }
    };
    Encounter {
        units,
        hull_percent: 100 + SECTOR_HULL_PERCENT * (sector - 1),
        reinforcements,
        replacements,
    }
}

/// Die Aufstellung eines Verbands: Jäger und Schlachtschiffe vorn, Spotter hinten, Kreuzer hinter Jägern. Hinten
/// steht nur, wer einen Vordermann hat (gedeckt); vorn höchstens drei. Die Mitte zuerst, dann links, dann rechts.
pub fn formation(units: &[EnemyId]) -> Vec<(EnemyId, EnemySlot)> {
    const ORDER: [usize; COLUMNS] = [1, 0, 2];
    let fighters = units.iter().any(|u| u.class() == EnemyClass::Fighter);
    let (mut back, mut front): (Vec<EnemyId>, Vec<EnemyId>) = units
        .iter()
        .copied()
        .partition(|&u| u == EnemyId::Spotter || (fighters && u.class() == EnemyClass::Cruiser));
    while front.len() > COLUMNS {
        back.push(front.pop().expect("vier vorn"));
    }
    while back.len() > front.len() {
        let i = back
            .iter()
            .rposition(|&u| u != EnemyId::Spotter)
            .expect("höchstens ein Spotter");
        front.push(back.remove(i));
    }
    let place = |row, units: Vec<EnemyId>| {
        units
            .into_iter()
            .zip(ORDER)
            .map(move |(id, column)| (id, EnemySlot::new(row, column)))
    };
    place(Row::Front, front)
        .chain(place(Row::Back, back))
        .collect()
}
