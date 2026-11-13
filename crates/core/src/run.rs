//! Der Run (`docs/run.md`, `docs/gegner.md`): drei Sektoren mit je einer Sternenkarte, die Gefechte gegen die Verbände
//! ihrer Knoten, Beute, Werft, Schwarzmarkt und Ereignisse. Die Sternenkarten sind entworfen ([`SectorMap`], eine
//! Datei je Sektor in `assets/data/maps/`). Der Zustand liegt in öffentlichen Feldern,
//! jeder Schritt ist eine Methode, die eine ungültige Wahl mit [`RunError`] ablehnt. Der Zufall hängt nur an `seed`
//! und `rolls`, also ist ein Run mit Seed und denselben Karten wiederholbar und nach dem Laden derselbe.

use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::admirals::AdmiralId;
use crate::battle::{Battle, BattleResult, BattleSetup, Encounter};
use crate::board::MAX_ENEMIES;
use crate::cards::{CardId, DeckCard, LOOT_POOL, starter_deck};
use crate::data::{rules, span};
use crate::modules::ModuleId;

pub const SECTORS: u32 = 3;

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
/// verbundenen Knoten in der nächsten Reihe (aufsteigend). Gefecht, Elite und Boss tragen ihren Verband.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapNode {
    pub kind: NodeKind,
    pub next: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Encounter>,
}

/// Eine entworfene Sternenkarte, Reihe für Reihe; die letzte Reihe ist der Boss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectorMap {
    pub rows: Vec<Vec<MapNode>>,
}

/// Was an einer Sternenkarte nicht stimmt; Knoten als (Reihe, Spalte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Die letzte Reihe ist nicht genau ein Boss, oder ein Boss steht in einer anderen Reihe.
    Boss,
    EmptyRow(usize),
    /// Keine Kante hinaus, eine Kante ins Leere oder Kanten nicht aufsteigend.
    Edges(usize, usize),
    /// Keine Kante führt zu diesem Knoten.
    Unreachable(usize, usize),
    /// Gefecht ohne Verband, Verband ohne Gefecht, Verband leer oder größer als das Brett erlaubt, oder eine
    /// Back-Einheit ohne Vordermann.
    Encounter(usize, usize),
}

impl SectorMap {
    /// Der Dateiname der Karte dieses Sektors (1 bis [`SECTORS`]) in `assets/data/maps/`.
    pub fn file(sector: u32) -> String {
        format!("sector{sector}.json")
    }

    pub fn check(&self) -> Result<(), MapError> {
        let Some((last, rows)) = self.rows.split_last() else {
            return Err(MapError::Boss);
        };
        let boss = |node: &MapNode| node.kind == NodeKind::Boss;
        if last.len() != 1 || !boss(&last[0]) || rows.iter().flatten().any(boss) {
            return Err(MapError::Boss);
        }
        if let Some(r) = self.rows.iter().position(Vec::is_empty) {
            return Err(MapError::EmptyRow(r));
        }
        for (r, row) in self.rows.iter().enumerate() {
            let below = self.rows.get(r + 1).map_or(0, Vec::len);
            for (c, node) in row.iter().enumerate() {
                let ascending = node.next.windows(2).all(|pair| pair[0] < pair[1]);
                let dangling = node.next.iter().any(|&next| next >= below);
                if !ascending || dangling || (node.next.is_empty() && below > 0) {
                    return Err(MapError::Edges(r, c));
                }
                if r > 0 && !self.rows[r - 1].iter().any(|from| from.next.contains(&c)) {
                    return Err(MapError::Unreachable(r, c));
                }
                let battle = matches!(
                    node.kind,
                    NodeKind::Battle | NodeKind::Elite | NodeKind::Boss
                );
                let valid = match &node.encounter {
                    None => !battle,
                    Some(encounter) => {
                        let covered = (encounter.back.iter().zip(&encounter.front))
                            .all(|(back, front)| back.is_none() || front.is_some());
                        let count = encounter.units().count();
                        battle
                            && covered
                            && (1..=MAX_ENEMIES).contains(&count)
                            && encounter.units().all(|(unit, _)| unit.valid())
                    }
                };
                if !valid {
                    return Err(MapError::Encounter(r, c));
                }
            }
        }
        Ok(())
    }
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
    /// Ein Gefecht, Elite oder Boss gegen den Verband des Knotens: [`Run::start_battle`], danach
    /// [`Run::finish_battle`].
    Battle {
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
    /// Die Sternenkarten der folgenden Sektoren, die nächste zuerst.
    pub upcoming: Vec<Vec<Vec<MapNode>>>,
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
    /// Ein Run über die geprüften ([`SectorMap::check`]) Karten der Sektoren, in ihrer Reihenfolge.
    pub fn new(admiral: AdmiralId, seed: u64, maps: [SectorMap; SECTORS as usize]) -> Self {
        let [first, upcoming @ ..] = maps.map(|map| map.rows);
        Run {
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
            map: first,
            upcoming: upcoming.to_vec(),
            path: Vec::new(),
            stage: Stage::Map,
            burning: false,
            pilot: false,
            victories: 0,
            result: None,
        }
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
            node.encounter = None;
            self.pilot = false;
        }
        let kind = node.kind;
        let mut rng = self.rng();
        self.stage = match kind {
            NodeKind::Battle | NodeKind::Elite | NodeKind::Boss => {
                Stage::Battle { seed: rng.random() }
            }
            NodeKind::Event => Stage::Event(*EventId::ALL.choose(&mut rng).expect("drei")),
            NodeKind::Shipyard => Stage::Shipyard,
            NodeKind::Market => Stage::Market(Market {
                cards: LOOT_POOL
                    .choose_multiple(&mut rng, rules().run.market_cards)
                    .map(|&card| (card, rng.random_range(span(rules().run.card_price))))
                    .collect(),
                modules: self
                    .missing_modules()
                    .choose_multiple(&mut rng, rules().run.market_modules)
                    .map(|&module| (module, rng.random_range(span(rules().run.module_price))))
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
        let Stage::Battle { seed } = self.stage else {
            panic!("kein Gefecht an diesem Knoten");
        };
        let (row, column) = self.current().expect("auf der Karte");
        let encounter = self.map[row][column].encounter.clone();
        let mut battle = Battle::new(
            BattleSetup {
                admiral: self.admiral,
                hull: self.hull,
                max_hull: self.max_hull,
                deck: self.deck.clone(),
                modules: self.modules.clone(),
                emergency_generator_available: self.has(ModuleId::EmergencyGenerator)
                    && !self.emergency_generator_used,
                encounter: encounter.expect("Gefechtsknoten mit Verband"),
            },
            seed,
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
                NodeKind::Elite => rng.random_range(span(rules().run.elite_scrap)),
                NodeKind::Boss => rules().run.boss_scrap,
                _ => rng.random_range(span(rules().run.battle_scrap)),
            },
            ..Loot::default()
        };
        if self.has(ModuleId::SalvageCrane) {
            loot.scrap += rules().modules.salvage_crane_scrap * battle.kills;
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
                .choose_multiple(&mut rng, rules().run.module_offer)
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

    /// Flackern: so viel Hülle kostet es.
    pub fn flicker_damage(&self) -> i32 {
        (self.max_hull * rules().run.flicker_hull_percent + 99) / 100
    }

    /// Ein Anteil der maximalen Hülle, aufgerundet.
    pub fn shipyard_repair(&self) -> i32 {
        (self.max_hull * rules().run.shipyard_repair_percent + 99) / 100
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
        self.pay(rules().run.removal_price)?;
        self.market()?.removal = false;
        self.deck.remove(index);
        Ok(())
    }

    /// Hülle für Schrott, beliebig oft.
    pub fn buy_repair(&mut self) -> Result<(), RunError> {
        self.market()?;
        self.pay(rules().run.market_repair_price)?;
        self.repair(rules().run.market_repair);
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
                self.scrap += rules().run.resolute_scrap;
                self.burning = true;
            }
            EventOption::Logbooks => {
                if let Some(&module) = self.missing_modules().choose(&mut self.rng()) {
                    self.add_module(module);
                    gain = Gain::Module(module);
                }
            }
            EventOption::PayPilot => {
                self.pay(rules().run.pilot_price)?;
                self.pilot = true;
            }
            EventOption::FlyYourself => {
                self.hull = (self.hull - rules().run.pilot_damage).max(1);
                let mut rng = self.rng();
                let cards = self.card_offer(&mut rng);
                self.stage = Stage::Loot(Loot {
                    cards,
                    ..Loot::default()
                });
                return Ok(gain);
            }
            EventOption::Embrace => {
                self.hull = (self.hull - self.flicker_damage()).max(1);
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
            rules().run.combat_simulator_offer
        } else {
            rules().run.card_offer
        };
        LOOT_POOL.choose_multiple(rng, size).copied().collect()
    }

    fn add_module(&mut self, module: ModuleId) {
        self.modules.push(module);
        if module == ModuleId::ReinforcedFrames {
            self.max_hull += rules().modules.reinforced_frames_hull;
            self.hull += rules().modules.reinforced_frames_hull;
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
            self.map = self.upcoming.remove(0);
        }
    }
}
