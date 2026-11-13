//! Die Zahlen des Spiels: Werte und Muster der Gegner, Karten, Admirale und alle übrigen Stellgrößen stehen in
//! `assets/data/*.json` (Format in `docs/daten.md`), nicht im Code. Die Dateien sind in das Crate eingebettet und
//! werden beim ersten Zugriff gelesen und geprüft ([`data`]); eine falsche Datei bricht dort mit ihrem Namen ab.
// shortcut: eingebettet, eine Änderung gilt also erst nach dem Neubauen; fürs Neuladen im laufenden Spiel (Editor)
// die Dateien im Client lesen und hier einsetzen.

use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::LazyLock;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::admirals::{ADMIRAL_IDS, AdmiralId};
use crate::cards::{CardId, CardTarget, CardType, EXTRA_CARDS, LOOT_POOL};
use crate::enemies::{EnemyClass, EnemyId, Unit, valid_pattern};

/// `enemies_defaults.json`: ein Gegner, seine Klasse und die Vorlage seiner Werte
/// ([`Unit::from`](crate::enemies::Unit)); im Gefecht gelten die Werte aus der Kartendatei.
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

/// `cards.json`: eine Karte. `value` ist der Hauptwert (Schaden, Schild, Reparatur, ATK-Bonus, Panzerung; 0 bei
/// Karten ohne Zahl), `extra` der Zweitwert der Karten aus [`EXTRA_CARDS`], `upgraded` nennt, was die aufgewertete
/// Fassung anders hat.
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmiralData {
    pub hull: i32,
    pub attack: i32,
    pub shield_station: i32,
    pub signature_card: CardId,
    pub deck: Vec<CardId>,
}

/// `rules.json`: alle Stellgrößen, die nicht an einem Gegner, einer Karte oder einem Admiral hängen.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub battle: BattleRules,
    pub stations: StationRules,
    pub enemies: EnemyRules,
    pub modules: ModuleRules,
    pub run: RunRules,
}

#[derive(Debug, Deserialize, Serialize)]
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
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyRules {
    pub ram_damage: i32,
    /// Riegel: Schild für alle gegnerischen Einheiten.
    pub bolt_shield: i32,
    /// Kannibalisieren: so viel repariert jedes zerstörte Schiff den Schrottkönig.
    pub cannibalize_heal: i32,
    /// Treffer des Plasmakreuzers zünden den Brand schon bei so viel Prozent Hülle.
    pub plasma_burn_percent: i32,
}

#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
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

/// `[von, bis]` als Spanne für den Zufall.
pub fn span([from, to]: [u32; 2]) -> RangeInclusive<u32> {
    from..=to
}

#[derive(Serialize)]
pub struct Data {
    pub enemies: HashMap<EnemyId, EnemyData>,
    pub cards: HashMap<CardId, CardData>,
    pub admirals: HashMap<AdmiralId, AdmiralData>,
    pub rules: Rules,
}

pub fn data() -> &'static Data {
    static DATA: LazyLock<Data> = LazyLock::new(|| {
        let data = Data {
            enemies: parse(
                "enemies_defaults.json",
                include_str!("../../../assets/data/enemies_defaults.json"),
            ),
            cards: parse(
                "cards.json",
                include_str!("../../../assets/data/cards.json"),
            ),
            admirals: parse(
                "admirals.json",
                include_str!("../../../assets/data/admirals.json"),
            ),
            rules: parse(
                "rules.json",
                include_str!("../../../assets/data/rules.json"),
            ),
        };
        data.check();
        data
    });
    &DATA
}

pub fn rules() -> &'static Rules {
    &data().rules
}

fn parse<T: DeserializeOwned>(file: &str, json: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("assets/data/{file}: {e}"))
}

impl Data {
    /// Jede Id hat ihren Eintrag, und was das Regelwerk voraussetzt, stimmt.
    fn check(&self) {
        for id in EnemyId::ALL {
            let enemy = self
                .enemies
                .get(&id)
                .unwrap_or_else(|| panic!("enemies_defaults.json: {id:?} fehlt"));
            let values = enemy.hull > 0 && enemy.attack >= 0 && enemy.armor >= 0;
            assert!(
                values && valid_pattern(&enemy.pattern),
                "enemies_defaults.json: {id:?}: Werte negativ, Muster leer, Spaltenfeuer zu breit, Schild \
                 nicht positiv oder gerufene Einheit ungültig"
            );
        }
        for id in LOOT_POOL {
            let card = self
                .cards
                .get(&id)
                .unwrap_or_else(|| panic!("cards.json: {id:?} fehlt"));
            assert!(
                card.check(EXTRA_CARDS.contains(&id)),
                "cards.json: {id:?}: Wert negativ oder `extra` fehlt bzw. steht, wo es keins gibt"
            );
        }
        for id in ADMIRAL_IDS {
            assert!(
                self.admirals.contains_key(&id),
                "admirals.json: {id:?} fehlt"
            );
        }
        let run = &self.rules.run;
        for [from, to] in [
            run.battle_scrap,
            run.elite_scrap,
            run.card_price,
            run.module_price,
        ] {
            assert!(from <= to, "rules.json: Spanne [{from}, {to}] ist leer");
        }
    }
}
