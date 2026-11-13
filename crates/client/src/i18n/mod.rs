//! Übersetzungen. Die Texte stehen je Sprache in `assets/data/text/<sprache>.json` (Format in `docs/daten.md`); hier
//! steht nur, wie sie gelesen werden. [`Messages`] hat je sichtbarem Text eine Methode (`hud_end_turn`, `card_effect`
//! …), die so heißt wie sein Schlüssel in der Datei; Gruppen über Ids (Karten, Admirale, Gegner, Stationen, Fehler …)
//! nehmen das Enum aus `voidfleet_core` und lesen `gruppe.id`. Im Text steht `{name}` für ein Argument der Methode und
//! `{rules.battle.clock_start}` für eine Zahl aus den Daten des Regelwerks (`enemies`, `cards`, `admirals`, `rules`).
//! Englisch ist die Ausgangssprache, Deutsch die zweite.
//!
//! Die Ressource [`Locale`] trägt die gewählte Sprache und dereferenziert auf ihre [`Messages`]:
//! `locale.hud_round(3)`, `locale.card(DeckCard::new(CardId::Torpedo)).name`. Gewählt wird beim Start: die Sprache aus
//! den Einstellungen, sonst die erste passende Systemsprache, sonst Englisch; ein Wechsel in den Einstellungen gilt
//! sofort.
//!
//! Die Funksprüche ([`Messages::radio_lines`]) sind die Ausnahme: Die Aufnahmen sind englisch, der englische Text ist
//! zugleich das Gesprochene; andere Sprachen übersetzen nur den Untertitel, mit gleich vielen Varianten je Spruch.

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    ops::Deref,
    sync::LazyLock,
};

use bevy::prelude::*;
use serde_json::Value;
use voidfleet_core::{
    admirals::{AdmiralId, FlagshipId, Station},
    battle::{BattleResult, RuleError},
    board::{EnemySlot, Row},
    cards::{CardId, CardTarget, CardType, DeckCard},
    data::{data, rules},
    enemies::{Action, EnemyClass, EnemyId},
    events::{Breakdown, DestroyCause, IdleReason, Intent, MissReason},
    modules::ModuleId,
    run::{EventId, EventOption, MapError, NodeKind},
};

use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, Antialiasing, CursorId, Language, Settings, WindowMode},
};

/// Die Texte je Sprache in der Reihenfolge von [`LANGUAGES`], flach nach Schlüssel (`card.torpedo.name`,
/// `radio_lines.victory.0`); die Zahlen aus den Daten sind schon eingesetzt.
// shortcut: eingebettet wie die Daten des Regelwerks, eine Änderung gilt also erst nach dem Neubauen.
static TEXTS: LazyLock<[HashMap<String, String>; 2]> = LazyLock::new(|| {
    let data = serde_json::to_value(data()).expect("Daten des Regelwerks als JSON");
    [
        (
            "en.json",
            include_str!("../../../../assets/data/text/en.json"),
        ),
        (
            "de.json",
            include_str!("../../../../assets/data/text/de.json"),
        ),
    ]
    .map(|(file, json)| {
        let tree: Value =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("assets/data/text/{file}: {e}"));
        let mut texts = HashMap::new();
        flatten(&tree, String::new(), &data, &mut texts);
        texts
    })
});

/// Legt den Baum einer Sprachdatei flach: Gruppen und Listen werden zu Punkten im Schlüssel.
fn flatten(value: &Value, key: String, data: &Value, texts: &mut HashMap<String, String>) {
    let child = |name: &str| match key.as_str() {
        "" => name.to_owned(),
        key => format!("{key}.{name}"),
    };
    match value {
        Value::String(text) => {
            let text = with_data(text, data).unwrap_or_else(|path| panic!("Text {key}: {path}"));
            texts.insert(key, text);
        }
        Value::Object(group) => {
            for (name, value) in group {
                flatten(value, child(name), data, texts);
            }
        }
        Value::Array(list) => {
            for (index, value) in list.iter().enumerate() {
                flatten(value, child(&index.to_string()), data, texts);
            }
        }
        _ => panic!("Text {key}: weder Text noch Gruppe"),
    }
}

/// Setzt die Zahlen aus den Daten ein: `{rules.battle.clock_start}`, `{admirals.osei.hull}`. Platzhalter ohne Punkt
/// sind Argumente und bleiben stehen. Der Fehler nennt den Pfad, den es in den Daten nicht gibt.
fn with_data(text: &str, data: &Value) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('{') {
        let (name, tail) = after
            .split_once('}')
            .ok_or("Platzhalter ohne schließende Klammer")?;
        out.push_str(before);
        if name.contains('.') {
            let number = data
                .pointer(&format!("/{}", name.replace('.', "/")))
                .and_then(Value::as_i64)
                .ok_or(format!("{{{name}}} ist keine Zahl in den Daten"))?;
            out.push_str(&number.to_string());
        } else {
            out.push_str(&format!("{{{name}}}"));
        }
        rest = tail;
    }
    Ok(out + rest)
}

/// Schlüssel einer Id wie in den Dateien: `CoupDeGrace` wird `coup_de_grace`; die Felder einer Variante zählen nicht.
fn snake(id: impl Debug) -> String {
    let mut key = String::new();
    for letter in format!("{id:?}")
        .chars()
        .take_while(char::is_ascii_alphanumeric)
    {
        if letter.is_ascii_uppercase() && !key.is_empty() {
            key.push('_');
        }
        key.push(letter.to_ascii_lowercase());
    }
    key
}

/// Einzahl oder Mehrzahl einer Zahl, als Schlüssel in der Datei.
fn form(count: u64) -> &'static str {
    if count == 1 { "one" } else { "other" }
}

/// Zahl mit Vorzeichen, negativ mit echtem Minus.
fn signed(value: i64) -> String {
    if value < 0 {
        format!("−{}", -value)
    } else {
        format!("+{value}")
    }
}

/// Alle Sprachen in der Reihenfolge der Sprachwahl.
pub const LANGUAGES: [Language; 2] = [Language::En, Language::De];

/// Name und Effekttext einer Karte; aufgewertet mit „+“ am Namen und den erhöhten Werten im Text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardText {
    pub name: String,
    pub effect: String,
}

/// Was die Admiralswahl über einen Admiral sagt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmiralText {
    pub name: &'static str,
    pub epithet: &'static str,
    pub weapon: &'static str,
    pub passive: &'static str,
    /// Name der Admiralsstation.
    pub station: &'static str,
    pub station_text: &'static str,
}

/// Ein Kapitel der Spielregeln.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesSection {
    pub heading: &'static str,
    pub paragraphs: Vec<&'static str>,
}

/// Die Kapitel der Spielregeln.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RulesChapter {
    Board,
    Round,
    Stations,
    Damage,
    Conditions,
    Admirals,
    Cards,
    Enemies,
    Controls,
}

impl RulesChapter {
    /// In der Reihenfolge der Navigation.
    pub const ALL: [Self; 9] = [
        Self::Board,
        Self::Round,
        Self::Stations,
        Self::Damage,
        Self::Conditions,
        Self::Admirals,
        Self::Cards,
        Self::Enemies,
        Self::Controls,
    ];
}

/// Die Tabs des Einstellungsdialogs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    Game,
    Video,
    Sound,
    Dev,
}

impl SettingsTab {
    pub const ALL: [Self; 4] = [Self::Game, Self::Video, Self::Sound, Self::Dev];
}

/// Ein Zustand an einer Einheit, als Icon über ihr.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Burning,
    Emergency,
    Stasis,
    Silenced,
    /// Gedeckt: Ein Vordermann fängt die Hälfte ab.
    Covered,
}

/// Alle sichtbaren Texte einer Sprache.
#[derive(Clone, Copy)]
pub struct Messages {
    texts: &'static HashMap<String, String>,
}

/// Texte ohne Parameter.
macro_rules! plain {
    ($($(#[$doc:meta])* $name:ident)*) => {
        impl Messages {
            $($(#[$doc])* pub fn $name(&self) -> &'static str {
                self.text(stringify!($name))
            })*
        }

        #[cfg(test)]
        const PLAIN: &[&str] = &[$(stringify!($name)),*];
    };
}

/// Texte je Id: `window_mode(WindowMode::Windowed)` liest `window_mode.windowed`.
macro_rules! keyed {
    ($($(#[$doc:meta])* $name:ident($id:ty))*) => {
        impl Messages {
            $($(#[$doc])* pub fn $name(&self, id: $id) -> &'static str {
                self.text(&format!("{}.{}", stringify!($name), snake(id)))
            })*
        }
    };
}

/// Texte mit Parametern: `{name}` im Text wird das gleichnamige Argument.
macro_rules! filled {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),+))*) => {
        impl Messages {
            $($(#[$doc])* pub fn $name(&self, $($arg: $ty),+) -> String {
                self.fill(stringify!($name), &[$((stringify!($arg), &$arg)),+])
            })*
        }

        #[cfg(test)]
        const FILLED: &[(&str, &[&str])] = &[$((stringify!($name), &[$(stringify!($arg)),+])),*];
    };
}

plain! {
    /// Name der Sprache in ihr selbst, für die Sprachwahl.
    language_name

    // --- HUD ---
    hud_new_round hud_end_turn hud_skip hud_show_log hud_hide_log hud_points hud_draw_pile hud_discard_pile
    hud_stations

    // --- Admiralswahl ---
    admiral_select_title admiral_select_hint admiral_select_choose label_flagship label_weapon label_passive
    label_station label_signature

    // --- Einstellungen ---
    settings_title settings_window_mode settings_antialiasing settings_show_fps settings_music_volume
    settings_effects_volume settings_voice_volume settings_attack_speed settings_movement_speed
    settings_animation_speed settings_language settings_cursor settings_cursor_size settings_models models_hint

    // --- Karteneditor ---
    settings_editor editor_title editor_hint editor_save editor_discard editor_play editor_add_row
    editor_add_node editor_delete_node editor_empty editor_reinforcements editor_replacements editor_map editor_admiral
    editor_target editor_unit editor_valid editor_no_node editor_values editor_armor editor_reset
    editor_back editor_open_call
    editor_add_step

    // --- Regeln, Dialoge und Knöpfe, Laden ---
    rules_title dialog_close camera_view_toggle loading_failed

    // --- Ergebnis ---
    result_view_board result_continue result_new_run

    // --- Run: Hauptmenü, Sternenkarte, Knoten, Ende ---
    menu_tagline menu_new_run menu_continue menu_quit
    /// Der Weg zurück ins Hauptmenü (Einstellungen, Run-Ende).
    menu_main
    /// Statt eines Runs, solange keiner gespeichert ist.
    menu_no_runs
    map_hint sector_banner_hint run_show_deck run_no_modules loot_title loot_cards_hint loot_modules_hint loot_take
    loot_skip shipyard_title shipyard_hint shipyard_repair shipyard_upgrade shipyard_upgrade_text shipyard_remove
    shipyard_remove_text gallery_deck gallery_upgrade_hint gallery_remove_hint gallery_back market_title market_hint
    market_cards market_modules market_services market_removal market_sold_out market_leave run_won_title
    run_won_text run_abandon run_abandon_title run_abandon_text run_abandon_confirm run_abandon_back

    // --- Gefecht ---
    stats_hull stats_atk
    /// Vor der zweiten Absicht im Tooltip (Uhrwerk).
    caption_next
    log_jumped log_returned log_emergency_generator log_shuffled
    /// Ein Schuss geht ins Leere (Kurzsprung, Manöver).
    report_missed
}

keyed! {
    settings_tab(SettingsTab)
    window_mode(WindowMode)
    antialiasing(Antialiasing)
    speed(AnimationSpeed)
    cursor(CursorId)
    result_title(BattleResult)
    node_kind(NodeKind)
    node_kind_hint(NodeKind)
    rule_error(RuleError)
    row_name(Row)
    card_type(CardType)
    card_target(CardTarget)
    flagship(FlagshipId)
    enemy(EnemyId)
    enemy_class(EnemyClass)
    /// Was ein Zustand bewirkt, für den Tooltip seines Icons; der Brand mit seiner Zahl über [`Self::burn_hint`].
    condition_hint(Condition)
    log_battle_ended(BattleResult)
}

filled! {
    // --- HUD ---
    hud_round(round: u32)
    /// Gefechtsuhr: Marke in der Kopfzeile und ihr Tooltip.
    hud_clock(bonus: i32)
    hud_clock_hint(bonus: i32)
    hud_hull(hull: i32, max_hull: i32)
    checklist_station(station: &str)
    loading_assets(percent: u32)

    // --- Karteneditor; Reihe und Knoten zählen ab 1 ---
    editor_sector(sector: u32)
    editor_node(row: usize, column: usize, kind: &str)
    editor_edges(row: usize)
    editor_saved(file: &str)
    editor_failed(error: &str)
    editor_unsaved(file: &str)
    editor_column_fire(width: usize)

    // --- Run ---
    result_lost_subtitle(flagship: &str)
    menu_victories(battles: u32)
    run_scrap(scrap: u32)
    run_deck(cards: usize)
    run_modules(count: usize)
    run_position(sector: u32, row: usize)
    run_module_gained(module: &str)
    run_card_upgraded(card: &str)
    shipyard_repair_text(hull: i32)
    market_buy(price: u32)
    market_repair(hull: i32)
    run_lost_text(sector: u32, row: usize)

    // --- Gefecht ---
    /// Tooltip am Icon der Panzerung.
    armor_hint(amount: i32)

    // --- Log, je Ereignis eine Zeile; Einheiten kommen schon benannt ---
    log_round_header(round: u32)
    log_battle_started(admiral: &str)
    log_station(station: &str)
    log_hit(source: &str, target: &str, breakdown: &str)
    log_shielded(target: &str, amount: i32)
    log_healed(target: &str, amount: i32)
    log_ignited(target: &str)
    log_extinguished(target: &str)
    log_fireship_spent(unit: &str)
    log_rammed(target: &str)
    log_maneuvered(from: &str, to: &str)
    log_tractored(unit: &str, from: &str)
    log_arrived(unit: &str)
    log_enemy_action(unit: &str, action: &str)
    log_counterfire(target: &str, amount: i32)
    log_hold_the_line(target: &str, amount: i32)
    log_self_damage(amount: i32)
    log_burn(target: &str, amount: i32)
    log_cannibalized(unit: &str, amount: i32)
    log_intents(unit: &str, intents: &str)
    log_clock(bonus: i32)

    // --- Gefechtsbericht (Wiedergabe): Marken unter der Schadenszahl, Ansagen ---
    mark_evaded(amount: i32)
    mark_countermeasures(amount: i32)
    mark_armor(amount: i32)
    mark_covered(amount: i32, by: &str)
    mark_shielded(amount: i32)
    report_burned_out(unit: &str)
}

impl Messages {
    fn text(&self, key: &str) -> &'static str {
        self.texts
            .get(key)
            .unwrap_or_else(|| panic!("Text {key} fehlt"))
    }

    /// Der Text mit eingesetzten Argumenten.
    fn fill(&self, key: &str, args: &[(&str, &dyn Display)]) -> String {
        args.iter()
            .fold(self.text(key).to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), &value.to_string())
            })
    }

    /// Die Einträge einer Liste: `key.0`, `key.1` …
    fn list(&self, key: &str) -> Vec<&'static str> {
        (0..)
            .map_while(|index| self.texts.get(&format!("{key}.{index}")))
            .map(String::as_str)
            .collect()
    }

    /// Was an einer Sternenkarte nicht stimmt, für den Karteneditor; Reihe und Knoten zählen ab 1.
    pub fn editor_error(&self, error: MapError) -> String {
        let (name, row, column) = match error {
            MapError::Boss => ("boss", 0, 0),
            MapError::EmptyRow(row) => ("empty_row", row, 0),
            MapError::Edges(row, column) => ("edges", row, column),
            MapError::Unreachable(row, column) => ("unreachable", row, column),
            MapError::Encounter(row, column) => ("encounter", row, column),
        };
        self.fill(
            &format!("editor_error.{name}"),
            &[("row", &(row + 1)), ("column", &(column + 1))],
        )
    }

    pub fn on_off(&self, on: bool) -> &'static str {
        self.text(if on { "on_off.on" } else { "on_off.off" })
    }

    /// So viele Stationen dürfen in dieser Runde noch laufen.
    pub fn checklist_free_stations(&self, count: usize) -> String {
        let key = format!("checklist_free_stations.{}", form(count as u64));
        self.fill(&key, &[("count", &count)])
    }

    pub fn checklist_points(&self, points: u32) -> String {
        let key = format!("checklist_points.{}", form(points.into()));
        self.fill(&key, &[("points", &points)])
    }

    pub fn rules_section(&self, chapter: RulesChapter) -> RulesSection {
        let key = format!("rules_section.{}", snake(chapter));
        RulesSection {
            heading: self.text(&format!("{key}.heading")),
            paragraphs: self.list(&format!("{key}.paragraphs")),
        }
    }

    /// Absender im Untertitel: die eigene Flotte oder der abgehörte Gegnerfunk.
    pub fn radio_speaker(&self, intercepted: bool) -> &'static str {
        self.text(if intercepted {
            "radio_speaker.intercepted"
        } else {
            "radio_speaker.fleet"
        })
    }

    /// Je Funkspruch so viele Untertitel wie Aufnahmen ([`RadioCue::variants`]).
    pub fn radio_lines(&self, cue: RadioCue) -> Vec<&'static str> {
        self.list(&format!("radio_lines.{}", snake(cue)))
    }

    pub fn result_won_subtitle(&self, kills: u32, rounds: u32) -> String {
        let word = |name: &str, count: u32| {
            self.text(&format!(
                "result_won_subtitle.{name}.{}",
                form(count.into())
            ))
        };
        self.fill(
            "result_won_subtitle.text",
            &[
                ("kills", &kills),
                ("units", &word("units", kills)),
                ("rounds", &rounds),
                ("round_word", &word("round_word", rounds)),
            ],
        )
    }

    pub fn sector_name(&self, sector: u32) -> &'static str {
        self.text(&format!("sector_name.{}", sector.clamp(1, 3) - 1))
    }

    pub fn map_title(&self, sector: u32) -> String {
        self.fill(
            "map_title",
            &[("sector", &sector), ("name", &self.sector_name(sector))],
        )
    }

    pub fn run_scrap_change(&self, scrap: i64) -> String {
        self.fill("run_scrap_change", &[("scrap", &signed(scrap))])
    }

    pub fn run_hull_change(&self, hull: i32) -> String {
        self.fill("run_hull_change", &[("hull", &signed(hull.into()))])
    }

    pub fn event_title(&self, event: EventId) -> &'static str {
        self.text(&format!("event.{}.title", snake(event)))
    }

    pub fn event_text(&self, event: EventId) -> &'static str {
        self.text(&format!("event.{}.text", snake(event)))
    }

    pub fn event_option(&self, option: EventOption) -> &'static str {
        self.text(&format!("event_option.{}.label", snake(option)))
    }

    /// Die Folgen einer Entscheidung; `flicker_damage` ist der Hüllenverlust im Flackern.
    pub fn event_consequence(&self, option: EventOption, flicker_damage: i32) -> String {
        self.fill(
            &format!("event_option.{}.consequence", snake(option)),
            &[("flicker_damage", &flicker_damage)],
        )
    }

    /// Was nach der Entscheidung geschieht.
    pub fn event_outcome(&self, option: EventOption) -> &'static str {
        self.text(&format!("event_option.{}.outcome", snake(option)))
    }

    /// Die Stationen; die Admiralsstation heißt je Admiral anders.
    pub fn station(&self, station: Station, admiral: AdmiralId) -> &'static str {
        match station {
            Station::Admiral => self.admiral(admiral).station,
            _ => self.text(&format!("station.{}", snake(station))),
        }
    }

    /// `attack` ist die ATK der Waffen-Station samt Boni.
    pub fn station_text(&self, station: Station, admiral: AdmiralId, attack: i32) -> String {
        match station {
            Station::Admiral => self.admiral(admiral).station_text.to_owned(),
            _ => self.fill(
                &format!("station_text.{}", snake(station)),
                &[("attack", &attack), ("shields", &admiral.shield_station())],
            ),
        }
    }

    pub fn column_name(&self, column: usize) -> &'static str {
        self.text(&format!("column_name.{}", column.min(2)))
    }

    pub fn card_name(&self, card: CardId) -> &'static str {
        self.text(&format!("card.{}.name", snake(card)))
    }

    /// Effekttext mit dem Hauptwert `v` der (aufgewerteten) Karte, samt Zuschlägen im Gefecht. Dazu `{evading}` und
    /// `{emergency}`, der Wert gegen ein ausweichendes Ziel und eines im Notfall, und `{bonus}`, der Zuschlag als
    /// „ +2“ oder nichts.
    pub fn card_effect(&self, card: DeckCard, v: i32) -> String {
        let bonus = if v > 0 {
            format!(" +{v}")
        } else {
            String::new()
        };
        self.fill(
            &format!("card.{}.effect", snake(card.id)),
            &[
                ("v", &v),
                ("evading", &(v + rules().cards.heat_seeker_evading_bonus)),
                (
                    "emergency",
                    &(v + rules().cards.coup_de_grace_emergency_bonus),
                ),
                ("bonus", &bonus),
            ],
        )
    }

    /// Die Schlüsselwörter (`docs/karten.md`): Wort, wie es im Effekttext steht, und seine Erklärung.
    pub fn keywords(&self) -> Vec<(&'static str, &'static str)> {
        (0..)
            .map_while(|index| {
                let word = self.texts.get(&format!("keywords.{index}.word"))?;
                Some((word.as_str(), self.text(&format!("keywords.{index}.text"))))
            })
            .collect()
    }

    /// Erklärung zum erhöhten Kartenschaden: `bonus` aus der Feuerleitdoktrin.
    pub fn card_bonus(&self, bonus: i32) -> String {
        let card = self.card_name(CardId::FireControlDoctrine);
        self.fill("card_bonus", &[("card", &card), ("bonus", &bonus)])
    }

    /// Was die Admiralswahl über einen Admiral sagt.
    pub fn admiral(&self, admiral: AdmiralId) -> AdmiralText {
        let text = |field: &str| self.text(&format!("admiral.{}.{field}", snake(admiral)));
        AdmiralText {
            name: text("name"),
            epithet: text("epithet"),
            weapon: text("weapon"),
            passive: text("passive"),
            station: text("station"),
            station_text: text("station_text"),
        }
    }

    /// Eine angesagte Aktion ohne Schaden: „Zielfeuer“, „Spaltenfeuer Mitte“, „Schilde 6“.
    pub fn action(&self, action: Action) -> String {
        self.action_text("action", action)
    }

    /// Was eine Absicht bewirkt, für den Tooltip ihres Icons.
    pub fn action_hint(&self, action: Action) -> String {
        self.action_text("action_hint", action)
    }

    /// Der Text einer Aktion mit dem, was sie mitbringt: Spalten, Schildwert oder gerufene Einheit.
    fn action_text(&self, group: &str, action: Action) -> String {
        let key = format!("{group}.{}", snake(action));
        match action {
            Action::ColumnFire { columns } => {
                let names: Vec<&str> = (0..columns.len())
                    .filter(|&column| columns[column])
                    .map(|column| self.column_name(column))
                    .collect();
                self.fill(&key, &[("columns", &names.join("+"))])
            }
            Action::Shields(amount) => self.fill(&key, &[("amount", &amount)]),
            Action::Call(enemy) => self.fill(&key, &[("enemy", &self.enemy(enemy))]),
            _ => self.text(&key).to_owned(),
        }
    }

    pub fn condition(&self, condition: Condition) -> String {
        self.text(&format!("condition.{}", snake(condition)))
            .to_owned()
    }

    pub fn burn_hint(&self, amount: i32) -> String {
        if amount > 0 {
            self.fill("burn_hint.ticking", &[("amount", &amount)])
        } else {
            self.text("burn_hint.suppressed").to_owned()
        }
    }

    /// Name und Wirkung eines Moduls.
    pub fn module(&self, module: ModuleId) -> (&'static str, &'static str) {
        let key = format!("module.{}", snake(module));
        (
            self.text(&format!("{key}.name")),
            self.text(&format!("{key}.text")),
        )
    }

    /// Die Wörter der Schadensrechnung: Ausweichen, Gegenmaßnahmen, Panzerung, gedeckt, Schild, tödlich.
    pub fn breakdown_words(&self) -> [&'static str; 6] {
        [
            "evaded",
            "countermeasures",
            "armor",
            "covered",
            "shield",
            "lethal",
        ]
        .map(|word| self.text(&format!("breakdown_words.{word}")))
    }

    pub fn log_card(&self, card: &str, target: Option<&str>) -> String {
        match target {
            Some(target) => self.fill("log_card.target", &[("card", &card), ("target", &target)]),
            None => self.fill("log_card.plain", &[("card", &card)]),
        }
    }

    pub fn log_missed(&self, source: &str, reason: MissReason) -> String {
        self.fill(
            &format!("log_missed.{}", snake(reason)),
            &[("source", &source)],
        )
    }

    pub fn log_emergency(&self, target: &str, active: bool) -> String {
        let key = if active {
            "log_emergency.on"
        } else {
            "log_emergency.off"
        };
        self.fill(key, &[("target", &target)])
    }

    pub fn log_destroyed(&self, target: &str, cause: DestroyCause) -> String {
        self.fill(
            &format!("log_destroyed.{}", snake(cause)),
            &[("target", &target)],
        )
    }

    pub fn log_idle(&self, unit: &str, reason: IdleReason) -> String {
        self.fill(&format!("log_idle.{}", snake(reason)), &[("unit", &unit)])
    }

    pub fn log_drawn(&self, count: usize) -> String {
        let key = format!("log_drawn.{}", form(count as u64));
        self.fill(&key, &[("count", &count)])
    }

    /// Name und Beiname, etwa „Admiral Osei – Die Mauer“.
    pub fn admiral_title(&self, admiral: AdmiralId) -> String {
        let AdmiralText { name, epithet, .. } = self.admiral(admiral);
        format!("{name} – {epithet}")
    }

    /// Name und Effekt einer Karte; aufgewertet mit „+“.
    pub fn card(&self, card: DeckCard) -> CardText {
        self.card_with_bonus(card, 0)
    }

    /// Wie `card`, mit `bonus` auf den Hauptwert (Feuerleitdoktrin im Gefecht).
    pub fn card_with_bonus(&self, card: DeckCard, bonus: i32) -> CardText {
        let name = self.card_name(card.id);
        CardText {
            name: if card.upgraded {
                format!("{name}+")
            } else {
                name.to_owned()
            },
            effect: self.card_effect(card, card.value() + bonus),
        }
    }

    /// Eine Ansage samt Schaden: „Zielfeuer 4“, „Spaltenfeuer Mitte 16“.
    pub fn intent(&self, intent: Intent) -> String {
        let action = self.action(intent.action);
        if intent.damage > 0 {
            format!("{action} {}", intent.damage)
        } else {
            action
        }
    }

    /// Ein Slot der Gegnerseite: „Front Mitte“.
    pub fn slot_name(&self, slot: EnemySlot) -> String {
        format!(
            "{} {}",
            self.row_name(slot.row),
            self.column_name(slot.column)
        )
    }

    /// Die Rechnung eines Treffers in Kurzform: „18 · gedeckt −4 · Schild −3 · Panzerung −2 → 9“.
    pub fn breakdown(&self, breakdown: &Breakdown) -> String {
        let [evaded, countermeasures, armor, covered, shield, lethal] = self.breakdown_words();
        let mut parts = vec![breakdown.base.to_string()];
        for (word, amount) in [
            (evaded, breakdown.evaded),
            (countermeasures, breakdown.countermeasures),
            (covered, breakdown.covered.map_or(0, |(_, amount)| amount)),
            (shield, breakdown.shielded),
            (armor, breakdown.armor),
        ] {
            if amount > 0 {
                parts.push(format!("{word} −{amount}"));
            }
        }
        let mut text = parts.join(" · ");
        if parts.len() > 1 {
            text.push_str(&format!(" → {}", breakdown.hull));
        }
        if breakdown.lethal {
            text.push_str(&format!(" · {lethal}"));
        }
        text
    }
}

/// Die Sprache der Oberfläche; dereferenziert auf ihre Texte.
#[derive(Resource, Clone, Copy)]
pub struct Locale {
    pub language: Language,
    messages: Messages,
}

impl Locale {
    pub fn new(language: Language) -> Self {
        let texts = &TEXTS[LANGUAGES
            .iter()
            .position(|&listed| listed == language)
            .expect("jede Sprache steht in LANGUAGES")];
        Self {
            language,
            messages: Messages { texts },
        }
    }

    /// Die gewählte Sprache, sonst die erste passende Systemsprache, sonst Englisch.
    pub fn detect(chosen: Option<Language>) -> Self {
        Self::new(chosen.unwrap_or_else(|| system_language(sys_locale::get_locales())))
    }
}

/// Beim Start: die Sprache aus den Einstellungen.
impl FromWorld for Locale {
    fn from_world(world: &mut World) -> Self {
        Self::detect(world.resource::<Settings>().language)
    }
}

impl Deref for Locale {
    type Target = Messages;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

/// Kürzel einer Sprache wie in Sprach-Tags (`de-AT`).
const fn tag(language: Language) -> &'static str {
    match language {
        Language::En => "en",
        Language::De => "de",
    }
}

/// Die erste unterstützte Sprache unter den Sprach-Tags des Systems, sonst Englisch.
fn system_language(tags: impl IntoIterator<Item = String>) -> Language {
    tags.into_iter()
        .find_map(|tag_name| {
            let primary = tag_name.split(['-', '_']).next()?.to_lowercase();
            LANGUAGES
                .into_iter()
                .find(|&language| tag(language) == primary)
        })
        .unwrap_or(Language::En)
}

/// Wählt die Sprache beim Start und folgt der Einstellung.
pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Locale>()
            .add_systems(Update, follow_settings.run_if(resource_changed::<Settings>));
    }
}

/// Eine andere Sprache in den Einstellungen gilt sofort.
fn follow_settings(settings: Res<Settings>, mut locale: ResMut<Locale>) {
    let next = Locale::detect(settings.language);
    if next.language != locale.language {
        *locale = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> [Locale; 2] {
        LANGUAGES.map(Locale::new)
    }

    /// Je Sprache genau so viele Untertitel wie Aufnahmen.
    #[test]
    fn radio_lines_match_recordings() {
        for locale in all() {
            for &cue in RadioCue::ALL {
                assert_eq!(
                    locale.radio_lines(cue).len(),
                    cue.variants(),
                    "{:?} in {:?}",
                    cue,
                    locale.language
                );
            }
        }
    }

    #[test]
    fn detect_prefers_setting_then_system() {
        assert_eq!(Locale::detect(Some(Language::De)).language, Language::De);
        assert_eq!(
            system_language(["fr-FR".into(), "de_AT".into()]),
            Language::De
        );
        assert_eq!(system_language(["EN-us".into()]), Language::En);
        assert_eq!(system_language(["fr".into()]), Language::En);
    }

    #[test]
    fn numbers_and_upgrades() {
        let de = Locale::new(Language::De);
        let en = Locale::new(Language::En);
        assert_eq!(de.admiral_title(AdmiralId::Osei), "Osei – Die Mauer");
        let torpedo = DeckCard::new(CardId::Torpedo);
        assert!(de.card(torpedo).effect.contains('6'));
        let upgraded = DeckCard {
            upgraded: true,
            ..torpedo
        };
        assert_eq!(en.card(upgraded).name, "Torpedo+");
        assert!(en.card(upgraded).effect.contains('8'));
        let breakdown = Breakdown {
            base: 18,
            armor: 2,
            covered: Some((1, 4)),
            shielded: 3,
            hull: 9,
            ..Breakdown::default()
        };
        assert_eq!(
            de.breakdown(&breakdown),
            "18 · gedeckt −4 · Schild −3 · Panzerung −2 → 9"
        );
        assert_eq!(
            en.breakdown(&Breakdown {
                base: 7,
                hull: 7,
                ..Breakdown::default()
            }),
            "7"
        );
    }

    fn placeholders(text: &str) -> Vec<&str> {
        let mut names: Vec<&str> = text
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name)
            .collect();
        names.sort_unstable();
        names
    }

    /// Beide Dateien haben dieselben Schlüssel mit denselben Platzhaltern; jeder Text ohne Id steht darin und nennt
    /// nur Argumente seiner Methode.
    #[test]
    fn files_match() {
        let [en, de] = &*TEXTS;
        for (key, text) in en {
            let other = de
                .get(key)
                .unwrap_or_else(|| panic!("de.json: {key} fehlt"));
            assert_eq!(placeholders(text), placeholders(other), "{key}");
        }
        for key in de.keys() {
            assert!(en.contains_key(key), "en.json: {key} fehlt");
        }
        for key in PLAIN {
            assert!(en.contains_key(*key), "{key} fehlt");
        }
        for (key, args) in FILLED {
            let text = en.get(*key).unwrap_or_else(|| panic!("{key} fehlt"));
            for name in placeholders(text) {
                assert!(args.contains(&name), "{key}: {{{name}}} ist kein Argument");
            }
        }
    }

    #[test]
    fn data_in_texts() {
        let data = serde_json::json!({ "rules": { "battle": { "clock_start": 8 } } });
        assert_eq!(
            with_data("Ab {rules.battle.clock_start}: +{bonus}", &data).unwrap(),
            "Ab 8: +{bonus}"
        );
        assert!(with_data("{rules.battle.clock}", &data).is_err());
        assert!(with_data("{offen", &data).is_err());
        assert_eq!(snake(CardId::CoupDeGrace), "coup_de_grace");
        assert_eq!(snake(Action::Shields(6)), "shields");
        assert_eq!(snake(Antialiasing::X2), "x2");
    }

    /// Jede Id jeder Gruppe hat in beiden Sprachen ihren Text, und kein Platzhalter bleibt stehen. Eine neue Variante
    /// eines Enums ohne eigene Liste gehört hier dazu.
    #[test]
    fn every_id_has_text() {
        use voidfleet_core::{admirals::ADMIRAL_IDS, run::MODULES};
        for locale in all() {
            let mut texts: Vec<String> = Vec::new();
            let mut plain = |text: &str| texts.push(text.to_owned());
            SettingsTab::ALL
                .into_iter()
                .for_each(|id| plain(locale.settings_tab(id)));
            Antialiasing::ALL
                .into_iter()
                .for_each(|id| plain(locale.antialiasing(id)));
            [
                WindowMode::Windowed,
                WindowMode::Maximized,
                WindowMode::Fullscreen,
            ]
            .into_iter()
            .for_each(|id| plain(locale.window_mode(id)));
            [
                AnimationSpeed::Slow,
                AnimationSpeed::Normal,
                AnimationSpeed::Fast,
                AnimationSpeed::VeryFast,
            ]
            .into_iter()
            .for_each(|id| plain(locale.speed(id)));
            [
                CursorId::Arrow,
                CursorId::Fighter,
                CursorId::Delta,
                CursorId::Flagship,
                CursorId::Needle,
                CursorId::Shard,
                CursorId::Formation,
                CursorId::Holo,
                CursorId::Thruster,
                CursorId::Crosshair,
            ]
            .into_iter()
            .for_each(|id| plain(locale.cursor(id)));
            for result in [BattleResult::Won, BattleResult::Lost] {
                plain(locale.result_title(result));
                plain(locale.log_battle_ended(result));
            }
            for kind in [
                NodeKind::Battle,
                NodeKind::Elite,
                NodeKind::Event,
                NodeKind::Shipyard,
                NodeKind::Market,
                NodeKind::Boss,
            ] {
                plain(locale.node_kind(kind));
                plain(locale.node_kind_hint(kind));
            }
            [
                RuleError::BattleOver,
                RuleError::NotEnoughPoints,
                RuleError::TargetInvalid,
                RuleError::AdmiralStationInEmergency,
                RuleError::Jumped,
                RuleError::StationUsed,
                RuleError::NoStationSlot,
            ]
            .into_iter()
            .for_each(|id| plain(locale.rule_error(id)));
            [
                MapError::Boss,
                MapError::EmptyRow(0),
                MapError::Edges(0, 0),
                MapError::Unreachable(0, 0),
                MapError::Encounter(0, 0),
            ]
            .into_iter()
            .for_each(|error| plain(&locale.editor_error(error)));
            [Row::Front, Row::Back]
                .into_iter()
                .for_each(|id| plain(locale.row_name(id)));
            [
                CardType::Attack,
                CardType::Buff,
                CardType::Debuff,
                CardType::Tactic,
                CardType::Doctrine,
            ]
            .into_iter()
            .for_each(|id| plain(locale.card_type(id)));
            [CardTarget::None, CardTarget::Enemy, CardTarget::Column]
                .into_iter()
                .for_each(|id| plain(locale.card_target(id)));
            [
                FlagshipId::Bastion,
                FlagshipId::Vanguard,
                FlagshipId::Pendulum,
                FlagshipId::Pyre,
            ]
            .into_iter()
            .for_each(|id| plain(locale.flagship(id)));
            [
                EnemyClass::Fighter,
                EnemyClass::Cruiser,
                EnemyClass::Battleship,
                EnemyClass::Flagship,
            ]
            .into_iter()
            .for_each(|id| plain(locale.enemy_class(id)));
            for condition in [
                Condition::Burning,
                Condition::Emergency,
                Condition::Stasis,
                Condition::Silenced,
                Condition::Covered,
            ] {
                plain(locale.condition_hint(condition));
                plain(&locale.condition(condition));
            }
            for enemy in EnemyId::ALL {
                plain(locale.enemy(enemy));
            }
            for action in [
                Action::Fire,
                Action::ColumnFire {
                    columns: [true, false, true],
                },
                Action::Ram,
                Action::Shields(6),
                Action::Evade,
                Action::Charge,
                Action::TargetLock,
                Action::Call(EnemyId::Raiders),
                Action::Bolt,
            ] {
                plain(&locale.action(action));
                plain(&locale.action_hint(action));
            }
            for card in voidfleet_core::cards::LOOT_POOL {
                for upgraded in [false, true] {
                    let text = locale.card(DeckCard { id: card, upgraded });
                    plain(&text.name);
                    plain(&text.effect);
                }
            }
            for admiral in ADMIRAL_IDS {
                let text = locale.admiral(admiral);
                [text.name, text.epithet, text.weapon, text.passive]
                    .into_iter()
                    .for_each(&mut plain);
                for station in [
                    Station::Weapons,
                    Station::Shields,
                    Station::DamageControl,
                    Station::Maneuver,
                    Station::Admiral,
                ] {
                    plain(locale.station(station, admiral));
                    plain(&locale.station_text(station, admiral, 3));
                }
            }
            for module in MODULES {
                let (name, text) = locale.module(module);
                plain(name);
                plain(text);
            }
            for chapter in RulesChapter::ALL {
                let section = locale.rules_section(chapter);
                assert!(!section.paragraphs.is_empty(), "{chapter:?}");
                plain(section.heading);
                section.paragraphs.into_iter().for_each(&mut plain);
            }
            for sector in 1..=voidfleet_core::run::SECTORS {
                plain(&locale.map_title(sector));
            }
            for event in EventId::ALL {
                plain(locale.event_title(event));
                plain(locale.event_text(event));
                for &option in event.options() {
                    plain(locale.event_option(option));
                    plain(&locale.event_consequence(option, 18));
                    plain(locale.event_outcome(option));
                }
            }
            assert_eq!(locale.keywords().len(), 8);
            for (word, text) in locale.keywords() {
                plain(word);
                plain(text);
            }
            locale.breakdown_words().into_iter().for_each(&mut plain);
            (0..3).for_each(|column| plain(locale.column_name(column)));
            for flag in [false, true] {
                plain(locale.on_off(flag));
                plain(locale.radio_speaker(flag));
                plain(&locale.log_emergency("A", flag));
            }
            for count in [1, 2] {
                plain(&locale.checklist_free_stations(count));
                plain(&locale.checklist_points(count as u32));
                plain(&locale.log_drawn(count));
                plain(&locale.result_won_subtitle(count as u32, count as u32));
                plain(&locale.burn_hint(count as i32 - 1));
            }
            plain(&locale.card_bonus(1));
            plain(&locale.run_scrap_change(-5));
            plain(&locale.run_hull_change(5));
            plain(&locale.log_card("A", None));
            plain(&locale.log_card("A", Some("B")));
            [MissReason::Jumped, MissReason::Maneuvered]
                .into_iter()
                .for_each(|reason| plain(&locale.log_missed("A", reason)));
            [
                DestroyCause::Shot,
                DestroyCause::BurnedOut,
                DestroyCause::Rammed,
                DestroyCause::Retaliation,
            ]
            .into_iter()
            .for_each(|cause| plain(&locale.log_destroyed("A", cause)));
            [IdleReason::Stasis, IdleReason::Silenced]
                .into_iter()
                .for_each(|reason| plain(&locale.log_idle("A", reason)));
            for text in texts {
                assert!(!text.is_empty() && !text.contains('{'), "{text}");
            }
        }
    }
}
