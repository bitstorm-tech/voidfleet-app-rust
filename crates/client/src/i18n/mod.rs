//! Übersetzungen. [`Messages`] legt je sichtbarem Text eine Methode fest (`hud_end_turn`, `card_effect` …); Texte mit
//! Parametern nehmen sie als Argumente und regeln Plural und Vorzeichen selbst. Gruppen über Ids (Karten, Admirale,
//! Gegner, Stationen, Fehler …) sind je eine Methode mit dem Enum aus `voidfleet_core` als Parameter; die Zahlen kommen
//! aus dem Regelwerk. [`En`] ist die Ausgangssprache, [`De`] die zweite.
//!
//! Die Ressource [`Locale`] trägt die gewählte Sprache und dereferenziert auf ihre [`Messages`]:
//! `locale.hud_round(3)`, `locale.card(DeckCard::new(CardId::Torpedo)).name`. Gewählt wird beim Start: die Sprache aus
//! den Einstellungen, sonst die erste passende Systemsprache, sonst Englisch; ein Wechsel in den Einstellungen gilt
//! sofort.
//!
//! Die Funksprüche ([`Messages::radio_lines`]) sind die Ausnahme: Die Aufnahmen sind englisch, der englische Text ist
//! zugleich das Gesprochene; andere Sprachen übersetzen nur den Untertitel, mit gleich vielen Varianten je Spruch.

mod de;
mod en;

use std::{borrow::Cow, ops::Deref};

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, FlagshipId, Station},
    battle::{BattleResult, RuleError},
    board::{EnemySlot, Row},
    cards::{CardId, CardTarget, CardType, DeckCard},
    enemies::{Action, EnemyClass, EnemyId},
    events::{Breakdown, DestroyCause, IdleReason, Intent, MissReason},
    modules::ModuleId,
    run::{EventId, EventOption, NodeKind},
};

pub use de::De;
pub use en::En;

use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, CursorId, Language, Settings, WindowMode},
};

/// Zahl mit Vorzeichen, negativ mit echtem Minus.
pub(super) fn signed(value: i64) -> String {
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
    pub paragraphs: Vec<Cow<'static, str>>,
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
}

/// Alle sichtbaren Texte einer Sprache.
pub trait Messages: Sync {
    /// Name der Sprache in ihr selbst, für die Sprachwahl.
    fn language_name(&self) -> &'static str;

    // --- HUD ---
    fn hud_round(&self, round: u32) -> String;
    fn hud_new_round(&self) -> &'static str;
    fn hud_end_turn(&self) -> &'static str;
    fn hud_skip(&self) -> &'static str;
    fn hud_show_log(&self) -> &'static str;
    fn hud_hide_log(&self) -> &'static str;
    /// Gefechtsuhr ab Runde 8: Marke in der Kopfzeile und ihr Tooltip.
    fn hud_clock(&self, bonus: i32) -> String;
    fn hud_clock_hint(&self, bonus: i32) -> String;
    fn hud_hull(&self, hull: i32, max_hull: i32) -> String;
    fn hud_points(&self) -> &'static str;
    fn hud_draw_pile(&self) -> &'static str;
    fn hud_discard_pile(&self) -> &'static str;
    fn hud_stations(&self) -> &'static str;
    /// So viele Stationen dürfen in dieser Runde noch laufen.
    fn checklist_free_stations(&self, count: usize) -> String;
    fn checklist_station(&self, station: &str) -> String;
    fn checklist_points(&self, points: u32) -> String;

    // --- Admiralswahl ---
    fn admiral_select_title(&self) -> &'static str;
    fn admiral_select_hint(&self) -> &'static str;
    fn admiral_select_choose(&self) -> &'static str;
    fn label_flagship(&self) -> &'static str;
    fn label_weapon(&self) -> &'static str;
    fn label_passive(&self) -> &'static str;
    fn label_station(&self) -> &'static str;
    fn label_signature(&self) -> &'static str;

    // --- Einstellungen ---
    fn settings_title(&self) -> &'static str;
    fn settings_tab(&self, tab: SettingsTab) -> &'static str;
    fn settings_window_mode(&self) -> &'static str;
    fn window_mode(&self, mode: WindowMode) -> &'static str;
    fn settings_show_fps(&self) -> &'static str;
    fn on_off(&self, on: bool) -> &'static str;
    fn settings_music_volume(&self) -> &'static str;
    fn settings_effects_volume(&self) -> &'static str;
    fn settings_voice_volume(&self) -> &'static str;
    fn settings_attack_speed(&self) -> &'static str;
    fn settings_movement_speed(&self) -> &'static str;
    fn settings_animation_speed(&self) -> &'static str;
    fn settings_language(&self) -> &'static str;
    fn speed(&self, speed: AnimationSpeed) -> &'static str;
    fn settings_cursor(&self) -> &'static str;
    fn settings_cursor_size(&self) -> &'static str;
    fn cursor(&self, cursor: CursorId) -> &'static str;
    fn settings_models(&self) -> &'static str;
    fn models_hint(&self) -> &'static str;

    // --- Regeln ---
    fn rules_title(&self) -> &'static str;
    fn rules_section(&self, chapter: RulesChapter) -> RulesSection;

    // --- Dialoge und Knöpfe ---
    fn dialog_close(&self) -> &'static str;
    fn fullscreen_enter(&self) -> &'static str;
    fn fullscreen_exit(&self) -> &'static str;
    fn camera_view_toggle(&self) -> &'static str;

    // --- Laden ---
    fn loading_assets(&self, percent: u32) -> String;
    fn loading_failed(&self) -> &'static str;

    // --- Bordfunk ---
    /// Absender im Untertitel: die eigene Flotte oder der abgehörte Gegnerfunk.
    fn radio_speaker(&self, intercepted: bool) -> &'static str;
    /// Je Funkspruch so viele Untertitel wie Aufnahmen ([`RadioCue::variants`]).
    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str];

    // --- Ergebnis ---
    fn result_title(&self, result: BattleResult) -> &'static str;
    fn result_won_subtitle(&self, kills: u32, rounds: u32) -> String;
    fn result_lost_subtitle(&self, flagship: &str) -> String;
    fn result_view_board(&self) -> &'static str;
    fn result_continue(&self) -> &'static str;
    fn result_new_run(&self) -> &'static str;

    // --- Run: Hauptmenü, Sternenkarte, Knoten, Ende ---
    fn menu_tagline(&self) -> &'static str;
    fn menu_new_run(&self) -> &'static str;
    fn menu_continue(&self) -> &'static str;
    fn menu_saved_run(&self, admiral: &str, sector: u32, row: usize) -> String;
    fn sector_name(&self, sector: u32) -> &'static str;
    fn map_title(&self, sector: u32) -> String;
    fn map_hint(&self) -> &'static str;
    fn sector_banner_hint(&self) -> &'static str;
    fn node_kind(&self, kind: NodeKind) -> &'static str;
    fn node_kind_hint(&self, kind: NodeKind) -> &'static str;
    fn run_scrap(&self, scrap: u32) -> String;
    fn run_deck(&self, cards: usize) -> String;
    fn run_show_deck(&self) -> &'static str;
    fn run_modules(&self, count: usize) -> String;
    fn run_no_modules(&self) -> &'static str;
    fn run_position(&self, sector: u32, row: usize) -> String;
    fn run_scrap_change(&self, scrap: i64) -> String;
    fn run_hull_change(&self, hull: i32) -> String;
    fn run_module_gained(&self, module: &str) -> String;
    fn run_card_upgraded(&self, card: &str) -> String;
    fn loot_title(&self) -> &'static str;
    fn loot_cards_hint(&self) -> &'static str;
    fn loot_modules_hint(&self) -> &'static str;
    fn loot_take(&self) -> &'static str;
    fn loot_skip(&self) -> &'static str;
    fn event_title(&self, event: EventId) -> &'static str;
    fn event_text(&self, event: EventId) -> &'static str;
    fn event_option(&self, option: EventOption) -> &'static str;
    /// Die Folgen einer Entscheidung; `flicker_damage` ist der Hüllenverlust im Flackern.
    fn event_consequence(&self, option: EventOption, flicker_damage: i32) -> String;
    /// Was nach der Entscheidung geschieht.
    fn event_outcome(&self, option: EventOption) -> &'static str;
    fn shipyard_title(&self) -> &'static str;
    fn shipyard_hint(&self) -> &'static str;
    fn shipyard_repair(&self) -> &'static str;
    fn shipyard_repair_text(&self, hull: i32) -> String;
    fn shipyard_upgrade(&self) -> &'static str;
    fn shipyard_upgrade_text(&self) -> &'static str;
    fn shipyard_remove(&self) -> &'static str;
    fn shipyard_remove_text(&self) -> &'static str;
    fn gallery_deck(&self) -> &'static str;
    fn gallery_upgrade_hint(&self) -> &'static str;
    fn gallery_remove_hint(&self) -> &'static str;
    fn gallery_back(&self) -> &'static str;
    fn market_title(&self) -> &'static str;
    fn market_hint(&self) -> &'static str;
    fn market_cards(&self) -> &'static str;
    fn market_modules(&self) -> &'static str;
    fn market_services(&self) -> &'static str;
    fn market_buy(&self, price: u32) -> String;
    fn market_removal(&self) -> &'static str;
    fn market_repair(&self, hull: i32) -> String;
    fn market_sold_out(&self) -> &'static str;
    fn market_leave(&self) -> &'static str;
    fn run_won_title(&self) -> &'static str;
    fn run_won_text(&self) -> &'static str;
    fn run_lost_text(&self, sector: u32, row: usize) -> String;
    fn run_abandon(&self) -> &'static str;
    fn run_abandon_title(&self) -> &'static str;
    fn run_abandon_text(&self) -> &'static str;
    fn run_abandon_confirm(&self) -> &'static str;
    fn run_abandon_back(&self) -> &'static str;

    // --- Ids aus dem Regelwerk ---
    fn rule_error(&self, error: RuleError) -> &'static str;
    fn stats_hull(&self) -> &'static str;
    fn stats_atk(&self) -> &'static str;
    /// Die Stationen; die Admiralsstation heißt je Admiral anders.
    fn station(&self, station: Station, admiral: AdmiralId) -> &'static str;
    fn station_text(&self, station: Station, admiral: AdmiralId) -> String;
    fn column_name(&self, column: usize) -> &'static str;
    fn row_name(&self, row: Row) -> &'static str;
    fn card_name(&self, card: CardId) -> &'static str;
    /// Effekttext mit den Zahlen der (aufgewerteten) Karte.
    fn card_effect(&self, card: DeckCard) -> String;
    fn card_type(&self, card_type: CardType) -> &'static str;
    fn card_target(&self, target: CardTarget) -> &'static str;
    fn admiral(&self, admiral: AdmiralId) -> AdmiralText;
    fn flagship(&self, flagship: FlagshipId) -> &'static str;
    fn enemy(&self, enemy: EnemyId) -> &'static str;
    fn enemy_class(&self, class: EnemyClass) -> &'static str;
    /// Eine angesagte Aktion ohne Schaden: „Zielfeuer“, „Spaltenfeuer Mitte“, „Schilde 6“.
    fn action(&self, action: Action) -> String;
    fn condition(&self, condition: Condition) -> String;
    /// Was eine Absicht bewirkt, für den Tooltip ihres Icons.
    fn action_hint(&self, action: Action) -> String;
    /// Was ein Zustand bewirkt, für den Tooltip seines Icons; der Brand mit seiner Zahl über [`Self::burn_hint`].
    fn condition_hint(&self, condition: Condition) -> &'static str;
    fn burn_hint(&self, amount: i32) -> String;
    /// Vor der zweiten Absicht im Tooltip (Uhrwerk).
    fn caption_next(&self) -> &'static str;
    fn module(&self, module: ModuleId) -> (&'static str, &'static str);
    /// Die Wörter der Schadensrechnung: Ausweichen, Gegenmaßnahmen, Panzerung, gedeckt, Schild, tödlich.
    fn breakdown_words(&self) -> [&'static str; 6];

    // --- Log, je Ereignis eine Zeile; Einheiten kommen schon benannt ---
    fn log_round_header(&self, round: u32) -> String;
    fn log_battle_started(&self, admiral: &str) -> String;
    fn log_card(&self, card: &str, target: Option<&str>) -> String;
    fn log_station(&self, station: &str) -> String;
    fn log_hit(&self, source: &str, target: &str, breakdown: &str) -> String;
    fn log_missed(&self, source: &str, reason: MissReason) -> String;
    fn log_shielded(&self, target: &str, amount: i32) -> String;
    fn log_healed(&self, target: &str, amount: i32) -> String;
    fn log_ignited(&self, target: &str) -> String;
    fn log_extinguished(&self, target: &str) -> String;
    fn log_emergency(&self, target: &str, active: bool) -> String;
    fn log_destroyed(&self, target: &str, cause: DestroyCause) -> String;
    fn log_fireship_spent(&self, unit: &str) -> String;
    fn log_rammed(&self, target: &str) -> String;
    fn log_jumped(&self) -> &'static str;
    fn log_returned(&self) -> &'static str;
    fn log_maneuvered(&self, from: &str, to: &str) -> String;
    fn log_tractored(&self, unit: &str, from: &str) -> String;
    fn log_arrived(&self, unit: &str) -> String;
    fn log_enemy_action(&self, unit: &str, action: &str) -> String;
    fn log_idle(&self, unit: &str, reason: IdleReason) -> String;
    fn log_counterfire(&self, target: &str, amount: i32) -> String;
    fn log_hold_the_line(&self, target: &str, amount: i32) -> String;
    fn log_self_damage(&self, amount: i32) -> String;
    fn log_burn(&self, target: &str, amount: i32) -> String;
    fn log_cannibalized(&self, unit: &str, amount: i32) -> String;
    fn log_emergency_generator(&self) -> &'static str;
    fn log_shuffled(&self) -> &'static str;
    fn log_drawn(&self, count: usize) -> String;
    fn log_intents(&self, unit: &str, intents: &str) -> String;
    fn log_clock(&self, bonus: i32) -> String;
    fn log_battle_ended(&self, result: BattleResult) -> &'static str;

    // --- Gefechtsbericht (Wiedergabe): Marken unter der Schadenszahl, Ansagen ---
    fn mark_evaded(&self, amount: i32) -> String;
    fn mark_countermeasures(&self, amount: i32) -> String;
    fn mark_armor(&self, amount: i32) -> String;
    fn mark_covered(&self, amount: i32, by: &str) -> String;
    fn mark_shielded(&self, amount: i32) -> String;
    /// Ein Schuss geht ins Leere (Kurzsprung, Manöver).
    fn report_missed(&self) -> &'static str;
    fn report_burned_out(&self, unit: &str) -> String;

    /// Name und Beiname, etwa „Admiral Osei – Die Mauer“.
    fn admiral_title(&self, admiral: AdmiralId) -> String {
        let AdmiralText { name, epithet, .. } = self.admiral(admiral);
        format!("{name} – {epithet}")
    }

    /// Name und Effekt einer Karte; aufgewertet mit „+“.
    fn card(&self, card: DeckCard) -> CardText {
        let name = self.card_name(card.id);
        CardText {
            name: if card.upgraded {
                format!("{name}+")
            } else {
                name.to_owned()
            },
            effect: self.card_effect(card),
        }
    }

    /// Eine Ansage samt Schaden: „Zielfeuer 4“, „Spaltenfeuer Mitte 16“.
    fn intent(&self, intent: Intent) -> String {
        let action = self.action(intent.action);
        if intent.damage > 0 {
            format!("{action} {}", intent.damage)
        } else {
            action
        }
    }

    /// Ein Slot der Gegnerseite: „Front Mitte“.
    fn slot_name(&self, slot: EnemySlot) -> String {
        format!(
            "{} {}",
            self.row_name(slot.row),
            self.column_name(slot.column)
        )
    }

    /// Die Rechnung eines Treffers in Kurzform: „18 · Panzerung −2 · gedeckt −4 · Schild −3 → 9“.
    fn breakdown(&self, breakdown: &Breakdown) -> String {
        let [evaded, countermeasures, armor, covered, shield, lethal] = self.breakdown_words();
        let mut parts = vec![breakdown.base.to_string()];
        for (word, amount) in [
            (evaded, breakdown.evaded),
            (countermeasures, breakdown.countermeasures),
            (armor, breakdown.armor),
            (covered, breakdown.covered.map_or(0, |(_, amount)| amount)),
            (shield, breakdown.shielded),
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
    messages: &'static dyn Messages,
}

impl Locale {
    pub fn new(language: Language) -> Self {
        let messages: &'static dyn Messages = match language {
            Language::En => &En,
            Language::De => &De,
        };
        Self { language, messages }
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
    type Target = dyn Messages;

    fn deref(&self) -> &Self::Target {
        self.messages
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
            "18 · Panzerung −2 · gedeckt −4 · Schild −3 → 9"
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

    /// Jede Karte, jeder Admiral, Gegner und Fehler hat in beiden Sprachen einen Text.
    #[test]
    fn every_id_has_text() {
        use voidfleet_core::admirals::ADMIRAL_IDS;
        for locale in all() {
            for card in voidfleet_core::cards::LOOT_POOL {
                for upgraded in [false, true] {
                    let text = locale.card(DeckCard { id: card, upgraded });
                    assert!(!text.name.is_empty() && !text.effect.is_empty(), "{card:?}");
                }
            }
            for admiral in ADMIRAL_IDS {
                assert!(!locale.admiral(admiral).station_text.is_empty());
                for station in [
                    Station::Weapons,
                    Station::Shields,
                    Station::DamageControl,
                    Station::Maneuver,
                    Station::Admiral,
                ] {
                    assert!(!locale.station_text(station, admiral).is_empty());
                }
            }
            for enemy in crate::data::ship_models::ENEMIES {
                assert!(!locale.enemy(enemy).is_empty());
            }
            for chapter in RulesChapter::ALL {
                assert!(!locale.rules_section(chapter).paragraphs.is_empty());
            }
            for sector in 1..=voidfleet_core::run::SECTORS {
                assert!(!locale.sector_name(sector).is_empty());
            }
            for event in EventId::ALL {
                assert!(!locale.event_text(event).is_empty());
                for &option in event.options() {
                    assert!(!locale.event_option(option).is_empty());
                    assert!(!locale.event_consequence(option, 18).is_empty());
                    assert!(!locale.event_outcome(option).is_empty());
                }
            }
        }
    }
}
