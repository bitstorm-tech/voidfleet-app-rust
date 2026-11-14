//! Die Zahlen des Spiels: Werte und Muster der Gegner, Karten (je Karte eine Datei, samt ihren Texten), Admirale und
//! alle übrigen Stellgrößen stehen in `assets/data/` (Format in `docs/daten.md`), nicht im Code. [`Data::load`] liest
//! und prüft sie zur Laufzeit; eine falsche Datei ist ein Fehler mit ihrem Namen. Es gilt, was zuletzt [`install`]
//! bekam (der Client beim Start und der Befehlskarten-Editor nach dem Speichern), sonst beim ersten Zugriff der Ordner
//! des Repositorys ([`REPO_DATA`], für Tests).

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::admirals::{ADMIRAL_IDS, AdmiralId};
use crate::cards::{CardId, CardTarget, CardType, EXTRA_CARDS, LOOT_POOL};
use crate::enemies::{EnemyClass, EnemyId, Unit, valid_pattern};

/// `enemies_defaults.json`: ein Gegner, seine Klasse und die Vorlage seiner Werte
/// ([`Unit::from`](crate::enemies::Unit)); im Gefecht gelten die Werte aus der Kartendatei.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyData {
    pub class: EnemyClass,
    pub hull: i32,
    pub attack: i32,
    pub armor: i32,
    /// Die Aktionsfolge; sie läuft von vorn, außer sie endet mit dem Rammkurs.
    pub pattern: Vec<Step>,
}

/// Ein Schritt eines Musters. Spaltenfeuer nennt seine Breite in Spalten, Schilde ihren Wert, Verstärkung die
/// gerufene Einheit mit allen Werten.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Fire,
    ColumnFire(usize),
    Ram,
    Shields(i32),
    Evade,
    Charge,
    TargetLock,
    Call(Unit),
    Bolt,
}

/// `cards/<id>.json`: eine Karte. `value` ist der Hauptwert (Schaden, Schild, Reparatur, ATK-Bonus, Panzerung; 0 bei
/// Karten ohne Zahl), `extra` der Zweitwert der Karten aus [`EXTRA_CARDS`], `upgraded` nennt, was die aufgewertete
/// Fassung anders hat, `text` je Sprachkürzel (`en`, `de`) Name und Effekttext; welche Sprachen es gibt und was in den
/// Texten steht, weiß der Client.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardData {
    #[serde(rename = "type")]
    pub card_type: CardType,
    pub target: CardTarget,
    pub cost: u32,
    pub value: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<i32>,
    pub upgraded: Upgrade,
    pub text: BTreeMap<String, CardText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardText {
    pub name: String,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Upgrade {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

/// `admirals.json`: ein Admiral mit Hülle und ATK seines Flaggschiffs, dem Wert seiner Schild-Station, der
/// Kennkarte und dem Startdeck.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmiralData {
    pub hull: i32,
    pub attack: i32,
    pub shield_station: i32,
    pub signature_card: CardId,
    pub deck: Vec<CardId>,
}

/// `rules.json`: alle Stellgrößen, die nicht an einem Gegner, einer Karte oder einem Admiral hängen.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub battle: BattleRules,
    pub stations: StationRules,
    pub enemies: EnemyRules,
    pub modules: ModuleRules,
    pub run: RunRules,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRules {
    pub hand_size: usize,
    pub command_points: u32,
    /// Brandschaden je Auflösung; bei Gegnern von Reyes (Glutkern) `ember_burn_damage`.
    pub burn_damage: i32,
    pub ember_burn_damage: i32,
    /// Ein Treffer zündet den Brand, wenn die Hülle danach höchstens so viel Prozent hat.
    pub burn_percent: i32,
    /// Notfall bei höchstens so viel Prozent Hülle.
    pub emergency_percent: i32,
    /// Ab dieser Runde bekommen alle Gegner +1 ATK je Runde (Gefechtsuhr).
    pub clock_start: u32,
    /// So viel Hülle geben Schadenskontrolle und Reparaturtrupp je Gefecht zusammen höchstens zurück.
    pub repair_limit: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StationRules {
    /// Schadenskontrolle: Hülle zurück.
    pub damage_control_repair: i32,
    /// Vorstoß: höchstens so viel ATK.
    pub max_momentum: i32,
    /// Gegenfeuer: Schild und Hülle, die jeder Schütze verliert, der die Bastion trifft.
    pub counterfire_shield: i32,
    pub counterfire_damage: i32,
    /// Fokusfeuer: Mehrschaden je gegnerischem Treffer im folgenden Gegnerzug.
    pub focus_fire_vulnerability: i32,
    /// Nachsetzen: Zuschlag für Angriffskarten auf brennende Ziele.
    pub follow_up_card_bonus: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyRules {
    pub ram_damage: i32,
    /// Riegel: Schild für alle gegnerischen Einheiten.
    pub bolt_shield: i32,
    /// Kannibalisieren: so viel repariert jedes zerstörte Schiff den Schrottkönig.
    pub cannibalize_heal: i32,
    /// Treffer des Plasmakreuzers zünden den Brand schon bei so viel Prozent Hülle.
    pub plasma_burn_percent: i32,
    /// Lebende Hülle: so viel wächst der Unbekannte Kontakt in einer Auflösung nach, wenn ihn in der Runde nichts
    /// getroffen hat und er nicht brennt.
    pub regeneration: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleRules {
    pub fire_control_computer_attack: i32,
    pub armor_plates_armor: i32,
    pub reserve_reactor_points: u32,
    pub supply_pod_hand: usize,
    pub salvage_crane_scrap: u32,
    pub reinforced_frames_hull: i32,
}

/// Spannen stehen als `[von, bis]`, beide Enden eingeschlossen.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunRules {
    pub battle_scrap: [u32; 2],
    pub elite_scrap: [u32; 2],
    pub boss_scrap: u32,
    pub card_offer: usize,
    pub combat_simulator_offer: usize,
    pub module_offer: usize,
    /// Werft und Boss: so viel Prozent der maximalen Hülle, aufgerundet.
    pub shipyard_repair_percent: i32,
    pub market_cards: usize,
    pub market_modules: usize,
    pub card_price: [u32; 2],
    pub module_price: [u32; 2],
    pub removal_price: u32,
    /// Reparatur im Schwarzmarkt: so viel Hülle für so viel Schrott.
    pub market_repair: i32,
    pub market_repair_price: u32,
    pub resolute_scrap: u32,
    pub pilot_price: u32,
    pub pilot_damage: i32,
    /// Flackern: so viel Prozent der maximalen Hülle gehen verloren, aufgerundet.
    pub flicker_hull_percent: i32,
}

impl CardData {
    /// Keine Zahl negativ, und `extra` steht genau dann, wenn die Karte einen Zweitwert hat.
    pub fn check(&self, has_extra: bool) -> bool {
        let values = [Some(self.value), self.extra, self.upgraded.value];
        values.into_iter().flatten().all(|value| value >= 0) && self.extra.is_some() == has_extra
    }
}

impl AdmiralData {
    /// Hülle positiv, ATK und Schild nicht negativ, ein Startdeck, in dem die Kennkarte steckt.
    pub fn check(&self) -> bool {
        self.hull > 0
            && self.attack >= 0
            && self.shield_station >= 0
            && self.deck.contains(&self.signature_card)
    }
}

/// `[von, bis]` als Spanne für den Zufall.
pub fn span([from, to]: [u32; 2]) -> RangeInclusive<u32> {
    from..=to
}

#[derive(Clone, Serialize)]
pub struct Data {
    pub enemies: HashMap<EnemyId, EnemyData>,
    pub cards: HashMap<CardId, CardData>,
    pub admirals: HashMap<AdmiralId, AdmiralData>,
    pub rules: Rules,
}

/// `assets/data` des Repositorys.
pub const REPO_DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data");

static DATA: RwLock<Option<&'static Data>> = RwLock::new(None);
/// Zählt jedes [`install`].
static VERSION: AtomicU64 = AtomicU64::new(0);

thread_local! {
    // Die Fassung, die dieser Thread zuletzt sah: Der Simulator fragt aus allen Kernen zugleich, und das Schloss
    // bremst dann jeden Zugriff auf ein Zehntel.
    static SEEN: Cell<Option<(u64, &'static Data)>> = const { Cell::new(None) };
}

/// Die geltenden Daten: was zuletzt [`install`] bekam, sonst die aus [`REPO_DATA`].
pub fn data() -> &'static Data {
    let version = VERSION.load(Ordering::Acquire);
    if let Some((seen, data)) = SEEN.get()
        && seen == version
    {
        return data;
    }
    let installed = *DATA.read().expect("Daten");
    let data = installed.unwrap_or_else(|| {
        DATA.write().expect("Daten").get_or_insert_with(|| {
            let data = Data::load(Path::new(REPO_DATA)).unwrap_or_else(|error| panic!("{error}"));
            Box::leak(Box::new(data))
        })
    });
    SEEN.set(Some((version, data)));
    data
}

/// Setzt geprüfte Daten ein; von da an gelten sie überall.
// Jede Fassung bleibt im Speicher, damit `&'static` hält: ein paar KB je Speichern im Editor.
pub fn install(data: Data) -> Result<(), String> {
    data.check()?;
    *DATA.write().expect("Daten") = Some(Box::leak(Box::new(data)));
    VERSION.fetch_add(1, Ordering::Release);
    Ok(())
}

pub fn rules() -> &'static Rules {
    &data().rules
}

/// Liest eine Datei aus `dir` (`assets/data`); der Fehler nennt sie.
pub fn read<T: DeserializeOwned>(dir: &Path, file: &str) -> Result<T, String> {
    let json = std::fs::read_to_string(dir.join(file));
    json.map_err(|e| e.to_string())
        .and_then(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
        .map_err(|e| format!("assets/data/{file}: {e}"))
}

impl Data {
    /// Liest alle Dateien aus `dir` (`assets/data`) und prüft sie.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let cards = LOOT_POOL
            .iter()
            .map(|&id| Ok((id, read(dir, &id.file())?)))
            .collect::<Result<_, String>>()?;
        let data = Self {
            enemies: read(dir, "enemies_defaults.json")?,
            cards,
            admirals: read(dir, "admirals.json")?,
            rules: read(dir, "rules.json")?,
        };
        data.check()?;
        Ok(data)
    }

    /// Jede Id hat ihren Eintrag, und was das Regelwerk voraussetzt, stimmt.
    pub fn check(&self) -> Result<(), String> {
        for id in EnemyId::ALL {
            let enemy = self
                .enemies
                .get(&id)
                .ok_or(format!("enemies_defaults.json: {id:?} fehlt"))?;
            let values = enemy.hull > 0 && enemy.attack >= 0 && enemy.armor >= 0;
            if !values || !valid_pattern(&enemy.pattern) {
                return Err(format!(
                    "enemies_defaults.json: {id:?}: Werte negativ, Muster leer, Spaltenfeuer zu breit, Schild \
                     nicht positiv oder gerufene Einheit ungültig"
                ));
            }
        }
        for id in LOOT_POOL {
            let card = self.cards.get(&id).ok_or(format!("{}: fehlt", id.file()))?;
            if !card.check(EXTRA_CARDS.contains(&id)) {
                return Err(format!(
                    "{}: Wert negativ oder `extra` fehlt bzw. steht, wo es keins gibt",
                    id.file()
                ));
            }
        }
        for id in ADMIRAL_IDS {
            let admiral = self
                .admirals
                .get(&id)
                .ok_or(format!("admirals.json: {id:?} fehlt"))?;
            if !admiral.check() {
                return Err(format!(
                    "admirals.json: {id:?}: Hülle nicht positiv, ATK oder Schild negativ oder Kennkarte nicht im \
                     Startdeck"
                ));
            }
        }
        let run = &self.rules.run;
        for [from, to] in [
            run.battle_scrap,
            run.elite_scrap,
            run.card_price,
            run.module_price,
        ] {
            if from > to {
                return Err(format!("rules.json: Spanne [{from}, {to}] ist leer"));
            }
        }
        Ok(())
    }
}
