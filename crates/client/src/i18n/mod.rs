//! Übersetzungen (Port von `src/i18n/`). [`Messages`] legt je sichtbarem Text eine Methode fest, benannt nach
//! Gruppe und Schlüssel der Web-Fassung (`hud.endTurn` → [`Messages::hud_end_turn`]); Texte mit Parametern nehmen sie
//! als Argumente und regeln Plural und Vorzeichen selbst. Gruppen, die in TypeScript ein `Record` über Ids waren
//! (Karten, Admirale, Fehlercodes, Lanes …), sind je eine Methode mit dem Enum aus `voidfleet_core` bzw. den
//! Einstellungen als Parameter. [`En`] ist die Ausgangssprache, [`De`] die zweite.
//!
//! Die Ressource [`Locale`] trägt die gewählte Sprache und dereferenziert auf ihre [`Messages`]:
//! `locale.hud_round(3)`, `locale.ship(ShipId::Hive).name`. Gewählt wird beim Start: die Sprache aus den
//! Einstellungen, sonst die erste passende Systemsprache, sonst Englisch; ein Wechsel in den Einstellungen gilt sofort.
//!
//! Die Funksprüche ([`Messages::radio_lines`]) sind die Ausnahme: Die Aufnahmen sind englisch, der englische Text ist
//! zugleich das Gesprochene; andere Sprachen übersetzen nur den Untertitel, mit gleich vielen Varianten je Spruch.

mod de;
mod en;

use std::{borrow::Cow, ops::Deref};

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, PowerId, PowerTarget},
    board::{LaneType, Side},
    cards::{CommandId, CommandType, ShipClass, ShipId, TargetMode},
    protocol::{ErrorCode, MatchResult},
};

pub use de::De;
pub use en::En;

use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, CursorId, Language, Settings, WindowMode},
};

/// Alle Sprachen in der Reihenfolge der Sprachwahl.
pub const LANGUAGES: [Language; 2] = [Language::En, Language::De];

/// Name und Effekttext einer Karte (auch Admiralsfähigkeiten).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardText {
    pub name: &'static str,
    pub effect: Cow<'static, str>,
    /// Statt der Zielart ([`Messages::target_mode`]), wenn der Befehl nur bestimmte Schiffe zum Ziel haben darf.
    pub target: Option<&'static str>,
}

impl CardText {
    const fn new(name: &'static str, effect: &'static str) -> Self {
        Self {
            name,
            effect: Cow::Borrowed(effect),
            target: None,
        }
    }

    fn formatted(name: &'static str, effect: String) -> Self {
        Self {
            name,
            effect: Cow::Owned(effect),
            target: None,
        }
    }
}

/// Name und Beiname eines Admirals und der Name der Fähigkeit seines Flaggschiffs; deren Text steht beim Schiff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmiralText {
    pub name: &'static str,
    pub epithet: &'static str,
    pub ability: &'static str,
}

/// Ein Kapitel der Spielregeln; `caption` beschriftet seine Illustration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesSection {
    pub heading: &'static str,
    pub paragraphs: Vec<Cow<'static, str>>,
    pub caption: Option<&'static str>,
}

/// Name und Beschreibung einer Phase der Rundenauflösung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseText {
    pub name: &'static str,
    pub text: &'static str,
}

/// Die Kapitel der Spielregeln.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RulesChapter {
    Goal,
    Turn,
    Admirals,
    Roster,
    Board,
    Movement,
    Resolution,
    Combat,
    Ships,
    Commands,
    Hand,
    Controls,
}

impl RulesChapter {
    /// In der Reihenfolge der Navigation.
    pub const ALL: [Self; 12] = [
        Self::Goal,
        Self::Turn,
        Self::Admirals,
        Self::Roster,
        Self::Board,
        Self::Movement,
        Self::Resolution,
        Self::Combat,
        Self::Ships,
        Self::Commands,
        Self::Hand,
        Self::Controls,
    ];
}

/// Die Phasen einer Runde, wie der Regel-Dialog sie zeigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Deploy,
    Maneuver,
    Effects,
    Combat,
    Cleanup,
    Draw,
    Check,
}

impl Phase {
    pub const ALL: [Self; 7] = [
        Self::Deploy,
        Self::Maneuver,
        Self::Effects,
        Self::Combat,
        Self::Cleanup,
        Self::Draw,
        Self::Check,
    ];
}

/// Die Tabs des Einstellungsdialogs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    Game,
    Video,
    Sound,
}

impl SettingsTab {
    pub const ALL: [Self; 3] = [Self::Game, Self::Video, Self::Sound];
}

/// Betrag und Vermerke eines Schusses im Log: „3, Fokusfeuer“.
fn amount_notes(amount: i32, notes: &[&str]) -> String {
    std::iter::once(amount.to_string())
        .chain(notes.iter().map(|note| (*note).to_owned()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Ein Rundenbonus mit Vorzeichen, nur das Plus ergänzt („+2“, „-1“).
fn signed(delta: i32) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

/// Eine ATK-Änderung als Schwebetext mit echtem Minuszeichen („+2“, „−1“).
fn float_signed(delta: i32) -> String {
    let sign = if delta > 0 { '+' } else { '−' };
    format!("{sign}{}", delta.abs())
}

/// HP und ATK eines Flaggschiffs aus den Kartendaten, für die Regeln.
fn stats(ship: ShipId) -> String {
    format!("{} HP, {} ATK", ship.hp(), ship.atk())
}

/// Alle sichtbaren Texte einer Sprache.
pub trait Messages: Sync {
    /// Name der Sprache in ihr selbst, für die Sprachwahl.
    fn language_name(&self) -> &'static str;

    // --- hud ---
    fn hud_round(&self, round: u32) -> String;
    fn hud_new_round(&self) -> &'static str;
    fn hud_end_turn(&self) -> &'static str;
    fn hud_waiting(&self) -> &'static str;
    fn hud_skip(&self) -> &'static str;
    fn hud_play_computer(&self) -> &'static str;
    fn hud_invite_player(&self) -> &'static str;
    fn hud_show_log(&self) -> &'static str;
    fn hud_hide_log(&self) -> &'static str;
    fn hud_overdrive(&self, bonus: i32) -> String;
    fn hud_overdrive_hint(&self, bonus: i32) -> String;
    fn hud_checklist_flagship(&self) -> &'static str;
    fn hud_checklist_ships(&self, planned: usize, max: usize) -> String;
    fn hud_checklist_command(&self) -> &'static str;
    fn hud_checklist_maneuver(&self) -> &'static str;
    /// Freie Hand: Grundmanöver und Fähigkeit getrennt.
    fn hud_checklist_maneuver_only(&self) -> &'static str;
    fn hud_checklist_power(&self) -> &'static str;
    fn hud_checklist_fire(&self, assigned: usize, shooters: usize) -> String;
    fn hud_checklist_mark(&self) -> &'static str;
    /// Die Fähigkeitskarte neben den Befehlen.
    fn hud_power(&self) -> &'static str;
    fn hud_power_hint(&self) -> &'static str;
    /// Der Knopf der Pendulum über der Fähigkeitskarte und was er in der Feuerrunde zeigt.
    fn hud_mark_button(&self) -> &'static str;
    fn hud_mark_hint(&self) -> &'static str;
    fn hud_marked(&self, ship: &str) -> String;
    fn hud_railgun(&self, ship: &str) -> String;
    fn hud_railgun_hint(&self, amount: i32) -> String;
    fn hud_shot_lost(&self) -> &'static str;
    fn hud_lock_lost(&self) -> &'static str;
    fn hud_under_fire(&self, ship: &str) -> String;

    // --- admiralSelect ---
    fn admiral_select_title(&self) -> &'static str;
    fn admiral_select_hint(&self) -> &'static str;
    fn admiral_select_flagship(&self) -> &'static str;
    fn admiral_select_power(&self) -> &'static str;
    fn admiral_select_choose(&self) -> &'static str;

    // --- draft ---
    fn draft_title(&self) -> &'static str;
    fn draft_hint(&self) -> &'static str;
    fn draft_ships(&self) -> &'static str;
    fn draft_commands(&self) -> &'static str;
    fn draft_none(&self) -> &'static str;
    fn draft_confirm(&self) -> &'static str;
    fn draft_peek(&self) -> &'static str;
    fn draft_show(&self) -> &'static str;

    // --- settings ---
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
    fn settings_install(&self) -> &'static str;
    fn settings_install_button(&self) -> &'static str;
    fn speed(&self, speed: AnimationSpeed) -> &'static str;
    fn settings_cursor(&self) -> &'static str;
    fn settings_cursor_size(&self) -> &'static str;
    fn cursor(&self, cursor: CursorId) -> &'static str;

    // --- rules ---
    fn rules_title(&self) -> &'static str;
    fn rules_section(&self, chapter: RulesChapter) -> RulesSection;
    fn phase(&self, phase: Phase) -> PhaseText;
    fn figures_you(&self) -> &'static str;
    fn figures_enemy(&self) -> &'static str;
    fn figures_slot(&self, index: usize) -> String;
    fn figures_covered(&self) -> &'static str;
    fn figures_exposed(&self) -> &'static str;
    fn figures_lane_limit(&self) -> &'static str;
    /// Die beiden Arten der Feuerbefehle mit je zwei Fällen.
    fn figures_column_fire(&self) -> &'static str;
    fn figures_tracking_fire(&self) -> &'static str;
    fn figures_column_through(&self) -> &'static str;
    fn figures_column_dodge(&self) -> &'static str;
    fn figures_tracking_follows(&self) -> &'static str;
    fn figures_tracking_covered(&self) -> &'static str;
    fn figures_round(&self, round: u32) -> String;
    fn figures_overdrive(&self, bonus: i32) -> String;
    fn figures_rounds(&self, from: u32, to: Option<u32>) -> String;
    fn figures_maneuver(&self) -> &'static str;
    fn figures_turn_parts(&self) -> [&'static str; 4];
    fn figures_or(&self) -> &'static str;
    fn figures_round_one(&self) -> &'static str;
    fn command_type(&self, command_type: CommandType) -> &'static str;
    fn rules_copies(&self, count: usize) -> String;
    fn rules_flagship(&self, admiral: &str) -> String;
    fn rules_token(&self) -> &'static str;

    // --- Dialoge und Knöpfe ---
    fn dialog_close(&self) -> &'static str;
    fn fullscreen_enter(&self) -> &'static str;
    fn fullscreen_exit(&self) -> &'static str;
    fn camera_view_toggle(&self) -> &'static str;
    fn surrender_title(&self) -> &'static str;
    fn surrender_question(&self) -> &'static str;
    fn surrender_confirm(&self) -> &'static str;

    // --- loading ---
    fn loading_assets(&self, percent: u32) -> String;
    fn loading_failed(&self) -> &'static str;

    // --- status ---
    fn status_disconnected(&self) -> &'static str;
    fn status_connecting(&self) -> &'static str;
    fn status_submitted(&self) -> &'static str;
    fn status_waiting_for_opponent(&self, match_id: &str) -> String;
    fn status_choose_admiral(&self, match_id: Option<&str>) -> String;
    fn status_opponent_choosing(&self) -> &'static str;
    fn status_draft(&self) -> &'static str;
    fn status_flagship_first(&self, flagship: &str) -> String;
    fn status_fleet(&self, count: usize, max: usize) -> String;
    fn status_no_free_slot(&self, ship: &str, per_lane: usize, per_side: usize) -> String;
    fn status_deploys_left(&self, count: usize) -> String;
    fn status_place_ship(&self, ship: &str) -> String;
    fn status_no_command_target(&self, command: &str) -> String;
    fn status_choose_command_target(&self, command: &str) -> String;
    fn status_choose_enemy_slot(&self, command: &str) -> String;
    fn status_choose_enemy_ship(&self, command: &str) -> String;
    fn status_choose_enemy_column(&self, command: &str) -> String;
    fn status_no_enemy_target(&self, command: &str) -> String;
    fn status_command_dropped(&self, command: &str, dropped: &str) -> String;
    fn status_choose_destination(&self, command: &str, ship: &str) -> String;
    /// Ein gewähltes eigenes Schiff: sein Feuerbefehl, sein Grundmanöver und was ein zweiter Klick zurücknimmt.
    fn status_choose_fire_target(&self, ship: &str) -> String;
    fn status_maneuver_instead(&self) -> &'static str;
    fn status_maneuver_or_swap(&self) -> &'static str;
    fn status_no_guns(&self, ship: &str) -> String;
    fn status_maneuvering(&self, ship: &str) -> String;
    fn status_command_holds_fire(&self, ship: &str, command: &str) -> String;
    fn status_maneuver_hint(&self) -> &'static str;
    fn status_change_maneuver(&self) -> &'static str;
    /// Der jetzige Feuerbefehl des gewählten Schiffs; unter dem Zeiger stattdessen, was ein Klick daraus macht.
    fn status_column_fire(&self, column: usize) -> String;
    fn status_track_fire(&self, target: &str) -> String;
    fn status_tap_column_fire(&self, column: usize) -> String;
    fn status_tap_track_fire(&self, target: &str) -> String;
    fn status_tap_clears_fire(&self) -> &'static str;
    /// Was der Schuss laut Vorschau trifft oder warum er verpufft.
    fn status_fire_hits(&self, target: &str) -> String;
    fn status_fire_through(&self, target: &str) -> String;
    fn status_column_empty(&self) -> &'static str;
    fn status_column_cleared(&self) -> &'static str;
    fn status_fire_covered(&self, target: &str) -> String;
    fn status_fire_fizzles(&self, target: &str) -> String;
    fn status_fire_gone(&self) -> &'static str;
    fn status_swarm_fire(&self, power: &str) -> String;
    fn status_again_unplan(&self) -> &'static str;
    fn status_again_maneuver(&self) -> &'static str;
    fn status_esc_cancels(&self) -> &'static str;
    fn status_fire_cleared(&self, ship: &str) -> String;
    fn status_fire_orders(&self, count: usize) -> String;
    fn status_fizzles(&self, count: usize) -> String;
    fn status_maneuver(&self, ship: &str) -> String;
    fn status_swap(&self, ship: &str, partner: &str) -> String;
    fn status_choose_power_target(&self, power: &str) -> String;
    fn status_no_power_target(&self, power: &str) -> String;
    fn status_no_drone(&self, power: &str) -> String;
    fn status_power(&self, power: &str, target: &str) -> String;
    fn status_power_replaces_maneuver(&self, power: &str, ship: &str) -> String;
    fn status_maneuver_replaces_power(&self, power: &str) -> String;
    fn status_choose_mark(&self) -> &'static str;
    fn status_mark(&self, ship: &str) -> String;
    fn status_planned(&self, cards: &str) -> String;
    fn status_idle(&self) -> &'static str;
    fn status_resolving(&self) -> &'static str;
    fn status_match_over(&self) -> &'static str;

    // --- opponent ---
    fn opponent_title(&self) -> &'static str;
    fn opponent_choosing(&self) -> &'static str;
    /// Solange man selbst noch wählt, bleibt die Wahl des Gegners verdeckt, auch ob er schon gewählt hat.
    fn opponent_hidden(&self) -> &'static str;
    fn opponent_planning(&self) -> &'static str;
    fn opponent_ready(&self) -> &'static str;
    fn opponent_ships(&self) -> &'static str;
    fn opponent_commands(&self) -> &'static str;
    fn opponent_hand(&self) -> &'static str;
    fn opponent_deck(&self) -> &'static str;

    // --- radio ---
    /// Absender im Untertitel.
    fn radio_speaker(&self) -> &'static str;
    /// Je Funkspruch so viele Untertitel wie Aufnahmen ([`RadioCue::variants`]).
    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str];

    // --- result ---
    fn result_title(&self, result: MatchResult) -> &'static str;
    fn result_won_subtitle(&self, flagship: &str) -> String;
    fn result_lost_subtitle(&self, flagship: &str) -> String;
    fn result_draw_subtitle(&self, own: &str, enemy: &str) -> String;
    /// Das Match endete durch Aufgabe: `won`, wenn der Gegner aufgegeben hat.
    fn result_surrendered(&self, won: bool) -> &'static str;
    fn result_rounds(&self, round: u32) -> String;
    fn result_view_board(&self) -> &'static str;
    fn result_new_match(&self) -> &'static str;

    // --- log ---
    fn log_round_header(&self, round: u32) -> String;
    fn log_who(&self, side: Side) -> &'static str;
    fn log_slot(&self, side: Side, lane: &str, slot: usize) -> String;
    /// Eine Spalte: beide Slots mit dieser Nummer.
    fn log_column(&self, side: Side, column: usize) -> String;
    fn log_ship(&self, ship: &str, slot: &str) -> String;
    fn log_deploy(&self, who: &str, ship: &str, slot: &str) -> String;
    fn log_deploy_flagship(&self, who: &str, ship: &str, slot: &str) -> String;
    fn log_power(&self, who: &str, power: &str, target: &str) -> String;
    fn log_decoy(&self, ship: &str) -> String;
    fn log_mark(&self, who: &str, ship: &str) -> String;
    fn log_swap(&self, who: &str, ship: &str, partner: &str) -> String;
    fn log_pass(&self, who: &str) -> String;
    fn log_maneuver(&self, who: &str, ship: &str, slot: &str) -> String;
    fn log_command(&self, who: &str, command: &str) -> String;
    fn log_command_on(&self, who: &str, command: &str, slot: &str) -> String;
    fn log_damage(&self, command: &str, ship: &str, amount: i32) -> String;
    fn log_attack_bonus(&self, command: &str, ship: &str, delta: i32) -> String;
    fn log_shield(&self, command: &str, ship: &str, amount: i32) -> String;
    fn log_disable(&self, command: &str, ship: &str) -> String;
    fn log_heal(&self, command: &str, ship: &str, amount: i32) -> String;
    fn log_move(&self, command: &str, ship: &str, slot: &str) -> String;
    fn log_shield_permanent(&self, command: &str, ship: &str, amount: i32) -> String;
    fn log_tow(&self, command: &str, ship: &str, slot: &str) -> String;
    fn log_jump(&self, command: &str, ship: &str) -> String;
    fn log_taunt(&self, command: &str, ship: &str) -> String;
    fn log_launch(&self, command: &str, ship: &str) -> String;
    fn log_ram(&self, command: &str, ship: &str, target: &str, dealt: i32, taken: i32) -> String;
    fn log_jam(&self, command: &str, ship: &str, power: &str) -> String;
    fn log_jam_decoy(&self, command: &str, ship: &str) -> String;
    fn log_countered(&self, command: &str, who: &str, countered: &str) -> String;
    fn log_rule(&self, command: &str, who: &str, detail: &str) -> String;
    fn log_rejoin(&self, ship: &str) -> String;
    fn log_repair(&self, source: &str, ship: &str, amount: i32) -> String;
    fn log_ability_damage(&self, source: &str, ship: &str, amount: i32) -> String;
    fn log_guardian_shield(&self, source: &str, ship: &str, amount: i32) -> String;
    fn log_spawn(&self, source: &str, ship: &str) -> String;
    fn log_brood(&self, source: &str, ship: &str) -> String;
    fn log_salvage(&self, ship: &str) -> String;
    fn log_draw(&self, command: &str, who: &str, ships: u32, commands: u32) -> String;
    fn log_no_effect(&self, command: &str, who: &str) -> String;
    fn log_miss(&self, command: &str, slot: &str) -> String;
    fn log_railgun(&self, source: &str, target: &str, amount: i32) -> String;
    fn log_railgun_blocked(&self, source: &str, target: &str) -> String;
    /// Ein Schuss im Kampf; `notes` nennt, was ihn verstärkt, gelenkt oder beschleunigt hat.
    fn log_attack(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String;
    /// Spaltenfeuer, das durchging: Das vordere Schiff der Spalte ist in diesem Kampf gefallen.
    fn log_through(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String;
    /// Spaltenfeuer verpufft: In der Spalte stand kein Schiff …
    fn log_fizzle_empty(&self, attacker: &str, column: &str) -> String;
    /// … bzw. die Schiffe dort sind in dieser Runde schon gefallen.
    fn log_fizzle_cleared(&self, attacker: &str, column: &str) -> String;
    /// Schiffsfeuer verpufft: Sein Ziel ist schon zerstört …
    fn log_fizzle(&self, attacker: &str, target: &str) -> String;
    /// … gedeckt …
    fn log_fizzle_covered(&self, attacker: &str, target: &str) -> String;
    /// … oder steht nicht mehr auf dem Brett (vor dem Kampf zerstört, Kurzsprung).
    fn log_fizzle_gone(&self, attacker: &str) -> String;
    fn log_silenced(&self, ship: &str) -> String;
    fn log_counterfire(&self, ship: &str, source: &str, amount: i32) -> String;
    fn log_self_damage(&self, ship: &str, amount: i32) -> String;
    fn log_destroyed(&self, ship: &str) -> String;
    fn log_flagship_destroyed(&self, ship: &str) -> String;
    fn log_vanish(&self, ship: &str) -> String;

    // --- banner ---
    fn banner_command(&self, who: &str, command: &str) -> String;
    fn banner_mark_target(&self, ship: &str) -> String;
    fn banner_blocked(&self) -> &'static str;
    fn banner_no_effect(&self) -> &'static str;
    fn banner_miss(&self, slot: &str) -> String;
    fn banner_draw(&self, ships: u32, commands: u32) -> String;
    fn banner_countered(&self, command: &str) -> String;
    fn banner_extra_ship(&self) -> &'static str;
    fn banner_maneuver_and_power(&self) -> &'static str;

    // --- float ---
    fn float_damage(&self, amount: i32) -> String;
    fn float_heal(&self, amount: i32) -> String;
    fn float_shield(&self, amount: i32) -> String;
    fn float_attack(&self, delta: i32) -> String;
    fn float_salvage(&self) -> &'static str;
    fn float_disabled(&self) -> &'static str;
    fn float_hold_fire(&self) -> &'static str;
    fn float_miss(&self) -> &'static str;
    fn float_silenced(&self) -> &'static str;
    fn float_locked(&self) -> &'static str;
    fn float_blocked(&self) -> &'static str;
    fn float_decoy(&self) -> &'static str;
    fn float_shield_permanent(&self, amount: i32) -> String;
    fn float_taunt(&self) -> &'static str;
    fn float_jammed(&self) -> &'static str;
    fn float_jump(&self) -> &'static str;
    /// Ein Schuss im Kampf verpufft.
    fn float_fizzle(&self) -> &'static str;
    /// Am Ziel von Schiffsfeuer, das an der Deckung davor verpufft.
    fn float_covered(&self) -> &'static str;

    // --- Ids aus dem Regelwerk ---
    fn error(&self, code: ErrorCode) -> &'static str;
    fn lane(&self, lane: LaneType) -> &'static str;
    fn stats_hp(&self) -> &'static str;
    fn stats_atk(&self) -> &'static str;
    fn stats_shield(&self) -> &'static str;
    fn ship_class(&self, class: ShipClass) -> &'static str;
    fn target_mode(&self, mode: TargetMode) -> &'static str;
    fn ship(&self, ship: ShipId) -> CardText;
    fn admiral(&self, admiral: AdmiralId) -> AdmiralText;
    fn power(&self, power: PowerId) -> CardText;
    fn power_target(&self, target: PowerTarget) -> &'static str;
    fn command(&self, command: CommandId) -> CardText;

    /// Name und Beiname, etwa „Admiral Osei – Die Mauer“.
    fn admiral_title(&self, admiral: AdmiralId) -> String {
        let AdmiralText { name, epithet, .. } = self.admiral(admiral);
        format!("{name} – {epithet}")
    }

    fn card_name(&self, ship: ShipId) -> &'static str {
        self.ship(ship).name
    }

    fn command_name(&self, command: CommandId) -> &'static str {
        self.command(command).name
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
    fn plurals_and_signs() {
        let en = Locale::new(Language::En);
        let de = Locale::new(Language::De);
        assert_eq!(en.status_deploys_left(0), "all ships planned");
        assert_eq!(en.status_deploys_left(1), "1 ship left to deploy");
        assert_eq!(de.status_deploys_left(2), "noch 2 Schiffe zu legen");
        assert_eq!(en.hud_checklist_ships(1, 1), "Deploy a ship");
        assert_eq!(de.hud_checklist_ships(1, 2), "Schiffe legen (1/2)");
        assert_eq!(en.float_attack(-1), "−1 ATK");
        assert_eq!(de.float_attack(2), "+2 ATK");
        assert_eq!(
            en.log_attack_bonus("X", "Y", -1),
            "Effect: X — Y -1 ATK this round"
        );
        assert_eq!(
            en.log_attack("A", "B", 3, &["Focus Fire"]),
            "Combat: A fires at B (3, Focus Fire)"
        );
        assert_eq!(de.banner_draw(0, 1), "+1 Befehl");
        assert_eq!(en.figures_rounds(8, None), "from round 8");
    }

    #[test]
    fn titles_and_numbers() {
        let de = Locale::new(Language::De);
        assert_eq!(
            de.admiral_title(AdmiralId::Osei),
            "Admiral Osei – Die Mauer"
        );
        assert_eq!(de.card_name(ShipId::TitanCarrier), "Titan-Träger");
        assert!(de.ship(ShipId::Hive).effect.contains("höchstens 2 Drohnen"));
        assert_eq!(
            Locale::new(Language::En)
                .command(CommandId::ScrapSalvage)
                .target,
            Some("Allied Scrap Raider")
        );
        assert_eq!(de.rules_section(RulesChapter::Roster).paragraphs.len(), 5);
    }
}
