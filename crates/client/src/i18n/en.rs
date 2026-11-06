//! Englisch, die Ausgangssprache: Die Funksprüche hier sind zugleich der gesprochene Text der Aufnahmen
//! (`tools/generateAudio.ts`).

use std::borrow::Cow;

use voidfleet_core::{
    admirals::{AdmiralId, DAMAGE_CONTROL_REPAIR, FlagshipId, Station},
    battle::{BattleResult, CLOCK_START, RuleError},
    board::Row,
    cards::{
        COUP_DE_GRACE_EMERGENCY_BONUS, CardId, CardTarget, CardType, DeckCard,
        HEAT_SEEKER_EVADING_BONUS, HOLD_THE_LINE_RETALIATION, OVERCHARGE_SELF_DAMAGE,
        SCAVENGE_DRAW,
    },
    enemies::{Action, BOLT_SHIELD, EnemyClass, EnemyId, RAM_DAMAGE},
    events::{DestroyCause, IdleReason, MissReason},
    modules::ModuleId,
    run::{EventId, EventOption, NodeKind},
};

use super::{AdmiralText, Condition, Messages, RulesChapter, RulesSection, SettingsTab, signed};
use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, CursorId, WindowMode},
};

pub struct En;

fn section(heading: &'static str, paragraphs: &[&'static str]) -> RulesSection {
    RulesSection {
        heading,
        paragraphs: paragraphs.iter().map(|text| Cow::Borrowed(*text)).collect(),
    }
}

impl Messages for En {
    fn language_name(&self) -> &'static str {
        "English"
    }

    fn hud_round(&self, round: u32) -> String {
        format!("Round {round}")
    }

    fn hud_new_round(&self) -> &'static str {
        "New round"
    }

    fn hud_end_turn(&self) -> &'static str {
        "End turn"
    }

    fn hud_skip(&self) -> &'static str {
        "Skip animation"
    }

    fn hud_show_log(&self) -> &'static str {
        "Show log"
    }

    fn hud_hide_log(&self) -> &'static str {
        "Hide log"
    }

    fn admiral_select_title(&self) -> &'static str {
        "Choose your admiral"
    }

    fn admiral_select_choose(&self) -> &'static str {
        "Choose"
    }

    fn settings_title(&self) -> &'static str {
        "Settings"
    }

    fn settings_tab(&self, tab: SettingsTab) -> &'static str {
        match tab {
            SettingsTab::Game => "Game",
            SettingsTab::Video => "Video",
            SettingsTab::Sound => "Sound",
        }
    }

    fn settings_window_mode(&self) -> &'static str {
        "Display"
    }

    fn window_mode(&self, mode: WindowMode) -> &'static str {
        match mode {
            WindowMode::Windowed => "Window",
            WindowMode::Maximized => "Maximized",
            WindowMode::Fullscreen => "Fullscreen",
        }
    }

    fn settings_show_fps(&self) -> &'static str {
        "FPS counter"
    }

    fn on_off(&self, on: bool) -> &'static str {
        if on { "On" } else { "Off" }
    }

    fn settings_music_volume(&self) -> &'static str {
        "Music"
    }

    fn settings_effects_volume(&self) -> &'static str {
        "Sound effects"
    }

    fn settings_voice_volume(&self) -> &'static str {
        "Radio chatter"
    }

    fn settings_attack_speed(&self) -> &'static str {
        "Attack speed"
    }

    fn settings_movement_speed(&self) -> &'static str {
        "Movement speed"
    }

    fn settings_animation_speed(&self) -> &'static str {
        "Other animations"
    }

    fn settings_language(&self) -> &'static str {
        "Language"
    }

    fn speed(&self, speed: AnimationSpeed) -> &'static str {
        match speed {
            AnimationSpeed::Slow => "Slow",
            AnimationSpeed::Normal => "Normal",
            AnimationSpeed::Fast => "Fast",
            AnimationSpeed::VeryFast => "Very fast",
        }
    }

    fn settings_cursor(&self) -> &'static str {
        "Mouse pointer"
    }

    fn settings_cursor_size(&self) -> &'static str {
        "Pointer size"
    }

    fn cursor(&self, cursor: CursorId) -> &'static str {
        match cursor {
            CursorId::Arrow => "Toon arrow",
            CursorId::Fighter => "Fighter",
            CursorId::Delta => "Delta glider",
            CursorId::Flagship => "Flagship",
            CursorId::Needle => "Navigation needle",
            CursorId::Shard => "Crystal shard",
            CursorId::Formation => "Formation",
            CursorId::Holo => "Holo arrow",
            CursorId::Thruster => "Thruster",
            CursorId::Crosshair => "Crosshair",
        }
    }

    fn rules_title(&self) -> &'static str {
        "Rules"
    }

    fn dialog_close(&self) -> &'static str {
        "Close"
    }

    fn fullscreen_enter(&self) -> &'static str {
        "Fullscreen (F)"
    }

    fn fullscreen_exit(&self) -> &'static str {
        "Exit fullscreen (F)"
    }

    fn camera_view_toggle(&self) -> &'static str {
        "Switch view (V); turn freely with ← → or the middle mouse button"
    }

    fn loading_assets(&self, percent: u32) -> String {
        format!("Loading assets … {percent} %")
    }

    fn loading_failed(&self) -> &'static str {
        "Loading failed — restart the game."
    }

    fn radio_speaker(&self, intercepted: bool) -> &'static str {
        if intercepted {
            "Intercepted"
        } else {
            "Fleet Comms"
        }
    }

    fn log_round_header(&self, round: u32) -> String {
        format!("— Round {round} —")
    }

    fn stats_atk(&self) -> &'static str {
        "ATK"
    }

    fn hud_trigger(&self) -> &'static str {
        "Fire station"
    }

    fn hud_clock(&self, bonus: i32) -> String {
        format!("Battle clock +{bonus} ATK")
    }

    fn hud_clock_hint(&self, bonus: i32) -> String {
        format!(
            "From round {CLOCK_START} every enemy fires with +1 ATK more each round; now +{bonus}."
        )
    }

    fn hud_hull(&self, hull: i32, max_hull: i32) -> String {
        format!("Hull {hull}/{max_hull}")
    }

    fn hud_points(&self) -> &'static str {
        "Command points"
    }

    fn hud_draw_pile(&self) -> &'static str {
        "Draw pile"
    }

    fn hud_discard_pile(&self) -> &'static str {
        "Discard pile"
    }

    fn hud_stations(&self) -> &'static str {
        "Stations"
    }

    fn hud_target(&self, target: &str) -> String {
        format!("Target: {target}")
    }

    fn hud_no_target(&self) -> &'static str {
        "No target — click an enemy"
    }

    fn hud_column(&self, column: usize) -> String {
        format!("Move to: {}", self.column_name(column))
    }

    fn hud_station_done(&self) -> &'static str {
        "Station done this round"
    }

    fn checklist_place(&self) -> &'static str {
        "Choose your column (optional)"
    }

    fn checklist_station(&self, station: &str) -> String {
        format!("Station: {station}")
    }

    fn checklist_target(&self) -> &'static str {
        "Choose a target"
    }

    fn checklist_column(&self) -> &'static str {
        "Choose a column"
    }

    fn checklist_points(&self, points: u32) -> String {
        match points {
            1 => "1 command point left".to_owned(),
            _ => format!("{points} command points left"),
        }
    }

    fn admiral_select_hint(&self) -> &'static str {
        "One flagship against their fleet. Your admiral brings the ship, a passive, an admiral station and two signature cards in the starter deck."
    }

    fn label_flagship(&self) -> &'static str {
        "Flagship"
    }

    fn label_weapon(&self) -> &'static str {
        "Weapon"
    }

    fn label_passive(&self) -> &'static str {
        "Passive"
    }

    fn label_station(&self) -> &'static str {
        "Admiral station"
    }

    fn label_signature(&self) -> &'static str {
        "Signature card"
    }

    fn rules_section(&self, chapter: RulesChapter) -> RulesSection {
        match chapter {
            RulesChapter::Board => section(
                "Board",
                &[
                    "Your flagship stands alone in one of three columns. Across from it the enemy fleet holds two rows, front and back, of three columns each, with at most four units.",
                    "A unit in the back row is covered while a unit stands in front of it in its column: the front unit takes half of every hit (rounded up), the target the rest.",
                    "Enemies fire in two ways. Targeted fire follows your flagship wherever it flies. Column fire is announced for a column and hits whatever stands there during the enemy turn.",
                ],
            ),
            RulesChapter::Round => section(
                "Round",
                &[
                    "A round is your turn, the enemy turn and the resolution. Before round 1 you see the formation and its intents and may pick your column with a click on your side.",
                    "In your turn every card and your station take effect at once, in the order you choose. Cards cost command points: 3 each round, unused points are lost. End turn when you are done; a station you have not fired yet fires then, if it can.",
                    "In the enemy turn each unit does what it announced, front before back, left to right. Then burns tick, shields expire, your hand is discarded and you draw five new cards.",
                    "From round 8 the battle clock gives every enemy +1 ATK per round.",
                ],
            ),
            RulesChapter::Stations => section(
                "Stations",
                &[
                    "Your flagship mans exactly one station per round. Station, target and column stay set until you change them.",
                    "Weapons fire your ATK at the chosen target. Shields absorb 8 in the next enemy turn (Osei 12). Damage control puts out the fire and repairs 5 hull. Maneuver moves to another column and dodges column fire aimed at the old one.",
                    "The admiral station depends on your admiral and is not available while your bridge is hit (hull at 25 % or less).",
                ],
            ),
            RulesChapter::Damage => section(
                "Damage",
                &[
                    "Every shot hits and every number is fixed: what the preview shows is what happens.",
                    "From the damage of a shot come off, in this order: evading halves it, armor subtracts, cover passes half to the unit in front, shields absorb the rest until they are spent.",
                    "Column fire and ramming hit fully or not at all; evading does not count against them.",
                ],
            ),
            RulesChapter::Conditions => section(
                "Conditions",
                &[
                    "Burning: a hit that brings a unit to 50 % hull or less sets it on fire. A burning unit loses 2 hull in each resolution; your own fire is put out by damage control or the repair crew.",
                    "Emergency: at 25 % hull or less a unit fires with half ATK and no longer evades. Your flagship cannot man its admiral station.",
                    "Destroyed: an enemy at 0 hull falls at once and never acts again. If your flagship falls, the battle is lost.",
                ],
            ),
            RulesChapter::Admirals => section(
                "Admirals",
                &[
                    "Osei, the Wall: Bastion, 70 hull, 7 ATK. Shields absorb 12. Counterfire: absorbs 6 and every shooter that hits her loses 3 hull.",
                    "Vance, the Hammer: Vanguard, 50 hull, 11 ATK. Momentum: +1 ATK for each round in a row with weapons or focus fire, up to +4. Focus fire: double momentum, ignores evading, but takes 2 more per hit.",
                    "Kerr, the Clockmaker: Pendulum, 56 hull, 8 ATK. The railgun ignores cover and evading; he sees intents two rounds ahead. Stasis: one enemy does not act this round.",
                    "Reyes, the Ember: Pyre, 56 hull, 9 ATK. The plasma thrower sets every target on fire, enemy burns tick 3. Follow-up: every burning enemy takes double burn damage at once, attack cards on burning targets +2.",
                ],
            ),
            RulesChapter::Cards => section(
                "Cards",
                &[
                    "Every round you draw five commands. Attack cards hit with fixed damage and the usual deductions; buffs strengthen your flagship, debuffs weaken an enemy, tactics change the board. Doctrines stay in play until the battle ends.",
                    "Each admiral starts with ten cards: three torpedoes, three emergency shields, a repair crew, scavenge and two signature cards. Upgraded cards (+) deal or absorb 2 more or cost 1 less.",
                ],
            ),
            RulesChapter::Enemies => section(
                "Enemies and intents",
                &[
                    "Each enemy follows a fixed pattern and shows what it will do next: targeted fire with its damage, column fire with column and damage, shields, evading, charging, target lock or calling reinforcements.",
                    "You act first: an enemy that falls in your turn never carries out its intent, one in emergency acts with half ATK. A micro jump makes every shot miss, a maneuver only the column fire on the old column.",
                    "Fighters evade and screen, cruisers are the backbone, battleships charge their main guns and carry armor.",
                ],
            ),
            RulesChapter::Controls => section(
                "Controls",
                &[
                    "Click a card to play it; a card with a target waits for a click on a glowing enemy or column. Clicking the card again, right-clicking or Escape cancels.",
                    "Pick a station in the panel; click an enemy to set the weapon target, a glowing column of your side for a maneuver. Hover an enemy to see the damage of your card or station.",
                    "Space skips the playback, V switches the view, ← → orbit, ↑ ↓ zoom, F toggles fullscreen, Escape opens the settings.",
                ],
            ),
        }
    }

    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str] {
        match cue {
            RadioCue::BattleStart => &[
                "All ships at battle stations. Awaiting your orders, Admiral.",
                "Fleet in position. Ready when you are, Admiral.",
            ],
            RadioCue::Victory => &[
                "Enemy fleet destroyed. The sector is ours, Admiral!",
                "That's the last of them. Victory, Admiral!",
            ],
            RadioCue::Defeat => &[
                "We've lost the fleet. All hands, abandon ship.",
                "Our fleet is gone. All survivors, fall back.",
            ],
            RadioCue::EnemyDestroyed => &[
                "Enemy ship destroyed!",
                "Target down!",
                "Splash one!",
                "Kill confirmed!",
            ],
            RadioCue::HullCritical => &["Hull integrity critical!", "She's badly hurt, Admiral!"],
            RadioCue::IncomingFire => &[
                "Incoming fire! Brace, brace, brace!",
                "They're opening up on us!",
            ],
            RadioCue::ColumnFireIncoming => &[
                "Beam weapons charging on our line!",
                "They've got our column bracketed! Get us out of there!",
            ],
            RadioCue::FireOnboard => &[
                "Fire on deck four! Damage control, move!",
                "We've got fire in the main corridor!",
            ],
            RadioCue::FireOut => &[
                "Fire's out. Venting the smoke now.",
                "Fire suppressed, Admiral.",
            ],
            RadioCue::ShieldsUp => &["Shields up!", "All power to forward shields!"],
            RadioCue::Maneuver => &[
                "Helm, hard to port! Changing lanes!",
                "Coming about, new heading!",
            ],
            RadioCue::MicroJump => &[
                "Spinning up the drive. Jump in three, two, one!",
                "Micro jump, now!",
            ],
            RadioCue::JumpReturn => &[
                "Jump complete. We're back in the fight.",
                "Back on the grid. Weapons hot.",
            ],
            RadioCue::RamIncoming => &[
                "Fireship on a collision course!",
                "That one's gonna ram us! Brace for impact!",
            ],
            RadioCue::ReinforcementsSighted => &[
                "New contacts! Enemy reinforcements jumping in!",
                "More of them, Admiral. Fresh hostiles on the board.",
            ],
            RadioCue::EmergencyGenerator => &[
                "Main power's gone! Emergency generator is holding us!",
                "Backup power online! We're still breathing!",
            ],
            RadioCue::Repairs => &[
                "Repair crews on the hull breach.",
                "Damage control teams, patch her up!",
            ],
            RadioCue::TargetLocked => {
                &["Target locked. Firing!", "Firing solution set. Guns away!"]
            }
            RadioCue::DirectHit => &["Direct hit!", "Good hit, good hit! Hull breach on target!"],
            RadioCue::EnemyBurning => &["Target's burning!", "She's on fire! Keep it up!"],
            RadioCue::TorpedoAway => &["Torpedo away!", "Torpedo running hot!"],
            RadioCue::OrbitalStrike => &["Orbital strike inbound!", "Calling it down from above!"],
            RadioCue::RailgunFire => &["Rail's charged. Fire!", "Slug away!"],
            RadioCue::PlasmaFire => &["Plasma ready. Burn them!", "Venting plasma, now!"],
            RadioCue::Counterfire => &[
                "Counterfire! Return the favor!",
                "They hit us, we hit back!",
            ],
            RadioCue::EnemyPanic => &[
                "Mayday, mayday! We're breaking up out here!",
                "Reactor's failing! Somebody help us!",
            ],
            RadioCue::EnemyCharging => &[
                "Main battery charging. Hold your fire.",
                "Full power to the big gun. On my mark.",
            ],
            RadioCue::EnemyCallsReinforcements => &[
                "All ships, all ships, converge on my position!",
                "We need backup! Send everything you've got!",
            ],
            RadioCue::EnemyBreaking => &[
                "Break off! Break off! Everyone run!",
                "We can't hold them! Pull back!",
            ],
        }
    }

    fn result_title(&self, result: BattleResult) -> &'static str {
        match result {
            BattleResult::Won => "Victory",
            BattleResult::Lost => "Defeat",
        }
    }

    fn result_won_subtitle(&self, kills: u32, rounds: u32) -> String {
        let units = if kills == 1 { "unit" } else { "units" };
        let round_word = if rounds == 1 { "round" } else { "rounds" };
        format!("{kills} enemy {units} destroyed in {rounds} {round_word}.")
    }

    fn result_lost_subtitle(&self, flagship: &str) -> String {
        format!("The {flagship} is lost. The run is over.")
    }

    fn result_view_board(&self) -> &'static str {
        "View board"
    }

    fn result_continue(&self) -> &'static str {
        "Continue"
    }

    fn result_new_run(&self) -> &'static str {
        "New run"
    }

    fn rule_error(&self, error: RuleError) -> &'static str {
        match error {
            RuleError::BattleOver => "The battle is over.",
            RuleError::PlacementOver => "Your column is set once you act.",
            RuleError::NotEnoughPoints => "Not enough command points.",
            RuleError::TargetRequired => "Choose a target first.",
            RuleError::TargetInvalid => "Not a valid target.",
            RuleError::NoStationLeft => "No station left this round.",
            RuleError::StationUsed => "This station already ran this round.",
            RuleError::AdmiralStationInEmergency => "Bridge hit: no admiral station.",
            RuleError::Jumped => "The flagship is in micro jump.",
            RuleError::SameColumn => "The flagship is already in this column.",
        }
    }

    fn stats_hull(&self) -> &'static str {
        "Hull"
    }

    fn station(&self, station: Station, admiral: AdmiralId) -> &'static str {
        match station {
            Station::Weapons => "Weapons",
            Station::Shields => "Shields",
            Station::DamageControl => "Damage control",
            Station::Maneuver => "Maneuver",
            Station::Admiral => self.admiral(admiral).station,
        }
    }

    fn station_text(&self, station: Station, admiral: AdmiralId) -> String {
        match station {
            Station::Weapons => "Fires your ATK at the chosen target.".to_owned(),
            Station::Shields => format!(
                "Absorbs {} in the next enemy turn.",
                admiral.shield_station()
            ),
            Station::DamageControl => {
                format!("Puts out the fire and repairs {DAMAGE_CONTROL_REPAIR} hull.")
            }
            Station::Maneuver => {
                "Moves to another column and dodges column fire on the old one.".to_owned()
            }
            Station::Admiral => self.admiral(admiral).station_text.to_owned(),
        }
    }

    fn column_name(&self, column: usize) -> &'static str {
        match column {
            0 => "left",
            1 => "center",
            _ => "right",
        }
    }

    fn row_name(&self, row: Row) -> &'static str {
        match row {
            Row::Front => "front",
            Row::Back => "back",
        }
    }

    fn card_name(&self, card: CardId) -> &'static str {
        match card {
            CardId::Torpedo => "Torpedo",
            CardId::OrbitalStrike => "Orbital Strike",
            CardId::VolleyFire => "Volley Fire",
            CardId::Barrage => "Barrage",
            CardId::HeatSeeker => "Heat Seeker",
            CardId::CoupDeGrace => "Coup de Grâce",
            CardId::Bombardment => "Bombardment",
            CardId::CollisionCourse => "Collision Course",
            CardId::Firebomb => "Firebomb",
            CardId::EmergencyShields => "Emergency Shields",
            CardId::HoldTheLine => "Hold the Line",
            CardId::AblativeArmor => "Ablative Armor",
            CardId::Overcharge => "Overcharge",
            CardId::PowerSurge => "Power Surge",
            CardId::RepairCrew => "Repair Crew",
            CardId::EmpBlast => "EMP Blast",
            CardId::TargetingJammer => "Targeting Jammer",
            CardId::RadioSilence => "Radio Silence",
            CardId::TractorBeam => "Tractor Beam",
            CardId::MicroJump => "Micro Jump",
            CardId::EvasiveAction => "Evasive Action",
            CardId::Countermeasures => "Countermeasures",
            CardId::Scavenge => "Scavenge",
            CardId::RedAlert => "Red Alert",
            CardId::FireControlDoctrine => "Fire Control Doctrine",
            CardId::BattleStations => "Battle Stations",
        }
    }

    fn card_effect(&self, card: DeckCard) -> String {
        let v = card.value();
        match card.id {
            CardId::Torpedo => format!("{v} damage to a chosen ship; evading does not count."),
            CardId::OrbitalStrike => format!("{v} damage to a chosen ship; ignores cover."),
            CardId::VolleyFire => format!("{v} damage to every enemy unit; ignores cover."),
            CardId::Barrage => format!("{v} damage to every enemy ship that is evading."),
            CardId::HeatSeeker => format!(
                "{v} damage to a chosen ship, {} if it is evading; ignores cover.",
                v + HEAT_SEEKER_EVADING_BONUS
            ),
            CardId::CoupDeGrace => format!(
                "{v} damage to a chosen ship, {} if it is in emergency.",
                v + COUP_DE_GRACE_EMERGENCY_BONUS
            ),
            CardId::Bombardment => format!("{v} damage to every unit in the enemy back row."),
            CardId::CollisionCourse => {
                let bonus = if v > 0 { format!(" +{v}") } else { String::new() };
                format!("Ram a chosen ship: it takes twice your ATK{bonus}, your flagship takes its ATK.")
            }
            CardId::Firebomb => format!("{v} damage to a chosen ship and sets it on fire."),
            CardId::EmergencyShields => format!("Your flagship absorbs {v} more in the enemy turn."),
            CardId::HoldTheLine => format!(
                "Absorbs {v} more in the enemy turn; every shooter that hits you loses {HOLD_THE_LINE_RETALIATION} hull."
            ),
            CardId::AblativeArmor => format!("For the rest of the battle you take {v} less per hit."),
            CardId::Overcharge => format!(
                "+{v} ATK this round; your flagship takes {OVERCHARGE_SELF_DAMAGE} damage at the end of your turn."
            ),
            CardId::PowerSurge => format!("+{v} ATK this round."),
            CardId::RepairCrew => format!("Puts out the fire and repairs {v} hull, on top of the station."),
            CardId::EmpBlast => "A chosen ship loses its shields and its charge.".to_owned(),
            CardId::TargetingJammer => "Every enemy shot in the enemy turn −1 ATK.".to_owned(),
            CardId::RadioSilence => {
                "A chosen ship neither fires nor uses its special action this round; a charge breaks.".to_owned()
            }
            CardId::TractorBeam => {
                "Pulls a chosen ship (not a flagship) into a free slot of its row; cover breaks.".to_owned()
            }
            CardId::MicroJump => {
                "Your flagship leaves the board: no station this round, no target in the enemy turn; it returns in the resolution.".to_owned()
            }
            CardId::EvasiveAction => "Your flagship moves to another column at once, keeping its station.".to_owned(),
            CardId::Countermeasures => "The first enemy attack in the enemy turn deals half damage.".to_owned(),
            CardId::Scavenge => format!("Draw {SCAVENGE_DRAW} commands."),
            CardId::RedAlert => "Man two different stations this round.".to_owned(),
            CardId::FireControlDoctrine => {
                format!("Doctrine: for the rest of the battle all your shots and attack cards +{v} damage.")
            }
            CardId::BattleStations => {
                format!("Doctrine: for the rest of the battle your flagship absorbs {v} in each enemy turn.")
            }
        }
    }

    fn card_type(&self, card_type: CardType) -> &'static str {
        match card_type {
            CardType::Attack => "Attack",
            CardType::Buff => "Buff",
            CardType::Debuff => "Debuff",
            CardType::Tactic => "Tactic",
            CardType::Doctrine => "Doctrine",
        }
    }

    fn card_target(&self, target: CardTarget) -> &'static str {
        match target {
            CardTarget::None => "No target",
            CardTarget::Enemy => "Enemy ship",
            CardTarget::Column => "Own column",
        }
    }

    fn admiral(&self, admiral: AdmiralId) -> AdmiralText {
        match admiral {
            AdmiralId::Osei => AdmiralText {
                name: "Osei",
                epithet: "The Wall",
                weapon: "Flak batteries",
                passive: "Guardian shield: shields absorb 12 instead of 8.",
                station: "Counterfire",
                station_text: "Does not fire. Absorbs 6; every shooter that hits the Bastion loses 3 hull.",
            },
            AdmiralId::Vance => AdmiralText {
                name: "Vance",
                epithet: "The Hammer",
                weapon: "Heavy guns",
                passive: "Momentum: +1 ATK for each round in a row with weapons or focus fire, up to +4; any other station resets it.",
                station: "Focus fire",
                station_text: "Fires with double momentum, evading does not count; takes 2 more per hit in the next enemy turn.",
            },
            AdmiralId::Kerr => AdmiralText {
                name: "Kerr",
                epithet: "The Clockmaker",
                weapon: "Railgun: ignores cover and evading",
                passive: "Clockwork: sees enemy intents two rounds ahead.",
                station: "Stasis",
                station_text: "Does not fire. A chosen enemy does not act this round; its intent moves to the next, a charge is kept.",
            },
            AdmiralId::Reyes => AdmiralText {
                name: "Reyes",
                epithet: "The Ember",
                weapon: "Plasma thrower: every hit sets the target on fire",
                passive: "Ember core: enemy burns tick 3 instead of 2.",
                station: "Follow-up",
                station_text: "Does not fire. Every burning enemy takes double its burn damage at once; attack cards on burning targets +2 this round.",
            },
        }
    }

    fn flagship(&self, flagship: FlagshipId) -> &'static str {
        match flagship {
            FlagshipId::Bastion => "Bastion",
            FlagshipId::Vanguard => "Vanguard",
            FlagshipId::Pendulum => "Pendulum",
            FlagshipId::Pyre => "Pyre",
        }
    }

    fn enemy(&self, enemy: EnemyId) -> &'static str {
        match enemy {
            EnemyId::Raiders => "Raider Squadron",
            EnemyId::Fireship => "Fireship",
            EnemyId::TorpedoWing => "Torpedo Wing",
            EnemyId::GuardCruiser => "Guard Cruiser",
            EnemyId::PlasmaCruiser => "Plasma Cruiser",
            EnemyId::Spotter => "Spotter",
            EnemyId::Besieger => "Besieger",
            EnemyId::Carrier => "Carrier",
            EnemyId::ScrapKing => "Scrap King",
            EnemyId::Warden => "Warden",
            EnemyId::Seal => "Seal",
        }
    }

    fn enemy_class(&self, class: EnemyClass) -> &'static str {
        match class {
            EnemyClass::Fighter => "Fighter squadron",
            EnemyClass::Cruiser => "Cruiser pair",
            EnemyClass::Battleship => "Battleship",
            EnemyClass::Flagship => "Flagship",
        }
    }

    fn action(&self, action: Action) -> String {
        match action {
            Action::Fire => "Fire".to_owned(),
            Action::ColumnFire { columns } => {
                let names: Vec<&str> = (0..columns.len())
                    .filter(|&column| columns[column])
                    .map(|column| self.column_name(column))
                    .collect();
                format!("Column fire {}", names.join("+"))
            }
            Action::Ram => format!("Ram {RAM_DAMAGE}"),
            Action::Shields(amount) => format!("Shields {amount}"),
            Action::Evade => "Evade".to_owned(),
            Action::Charge => "Charge".to_owned(),
            Action::TargetLock => "Target lock".to_owned(),
            Action::Call(enemy) => format!("Call {}", self.enemy(enemy)),
            Action::Bolt => format!("Bolt: all shield {BOLT_SHIELD}"),
        }
    }

    fn condition(&self, condition: Condition) -> String {
        match condition {
            Condition::Burning => "Burning".to_owned(),
            Condition::Emergency => "Emergency".to_owned(),
            Condition::Shield(amount) => format!("Shield {amount}"),
            Condition::Evading => "Evading".to_owned(),
            Condition::Charged => "Charged".to_owned(),
            Condition::Stasis => "Stasis".to_owned(),
            Condition::Silenced => "Silenced".to_owned(),
            Condition::Jumped => "Jumped".to_owned(),
        }
    }

    fn module(&self, module: ModuleId) -> (&'static str, &'static str) {
        match module {
            ModuleId::FireControlComputer => {
                ("Fire control computer", "+1 ATK on your flagship's shots.")
            }
            ModuleId::FireSuppression => ("Fire suppression", "Your own fire does not tick."),
            ModuleId::ReserveReactor => ("Reserve reactor", "4 command points in round 1."),
            ModuleId::ArmorPlates => ("Armor plates", "Armor +1."),
            ModuleId::EmergencyGenerator => (
                "Emergency generator",
                "Once per run a lethal hit leaves 1 hull.",
            ),
            ModuleId::SalvageCrane => ("Salvage crane", "Every destroyed enemy brings 5 scrap."),
            ModuleId::SupplyPod => ("Supply pod", "Draw 6 instead of 5 cards each round."),
            ModuleId::CombatSimulator => (
                "Combat simulator",
                "Card offers show four instead of three.",
            ),
            ModuleId::ReinforcedFrames => ("Reinforced frames", "+10 hull for good."),
        }
    }

    fn breakdown_words(&self) -> [&'static str; 6] {
        [
            "evades",
            "countermeasures",
            "armor",
            "covered",
            "shield",
            "lethal",
        ]
    }

    fn log_battle_started(&self, admiral: &str) -> String {
        format!("Battle stations — {admiral} takes command.")
    }

    fn log_card(&self, card: &str, target: Option<&str>) -> String {
        match target {
            Some(target) => format!("Card: {card} on {target}"),
            None => format!("Card: {card}"),
        }
    }

    fn log_station(&self, station: &str) -> String {
        format!("Station: {station}")
    }

    fn log_hit(&self, source: &str, target: &str, breakdown: &str) -> String {
        format!("{source} hits {target}: {breakdown}")
    }

    fn log_missed(&self, source: &str, reason: MissReason) -> String {
        match reason {
            MissReason::Jumped => {
                format!("{source} fires into the void — the flagship has jumped.")
            }
            MissReason::Maneuvered => format!("{source} fires at an empty column."),
        }
    }

    fn log_shielded(&self, target: &str, amount: i32) -> String {
        format!("{target} raises shields: {amount}")
    }

    fn log_healed(&self, target: &str, amount: i32) -> String {
        format!("{target} repairs {amount} hull")
    }

    fn log_ignited(&self, target: &str) -> String {
        format!("{target} is burning")
    }

    fn log_extinguished(&self, target: &str) -> String {
        format!("Fire on {target} is out")
    }

    fn log_emergency(&self, target: &str, active: bool) -> String {
        if active {
            format!("{target} in emergency")
        } else {
            format!("{target} out of emergency")
        }
    }

    fn log_destroyed(&self, target: &str, cause: DestroyCause) -> String {
        match cause {
            DestroyCause::Shot => format!("{target} destroyed"),
            DestroyCause::BurnedOut => format!("{target} burned out"),
            DestroyCause::Rammed => format!("{target} rammed to pieces"),
            DestroyCause::Retaliation => format!("{target} destroyed by return fire"),
        }
    }

    fn log_fireship_spent(&self, unit: &str) -> String {
        format!("{unit} is spent")
    }

    fn log_rammed(&self, target: &str) -> String {
        format!("The flagship rams {target}")
    }

    fn log_jumped(&self) -> &'static str {
        "The flagship jumps out"
    }

    fn log_returned(&self) -> &'static str {
        "The flagship is back"
    }

    fn log_maneuvered(&self, from: &str, to: &str) -> String {
        format!("The flagship moves from {from} to {to}")
    }

    fn log_tractored(&self, unit: &str, from: &str) -> String {
        format!("Tractor beam pulls {unit} from {from}")
    }

    fn log_arrived(&self, unit: &str) -> String {
        format!("{unit} arrives")
    }

    fn log_enemy_action(&self, unit: &str, action: &str) -> String {
        format!("{unit}: {action}")
    }

    fn log_idle(&self, unit: &str, reason: IdleReason) -> String {
        match reason {
            IdleReason::Stasis => format!("{unit} is held in stasis"),
            IdleReason::Silenced => format!("{unit} keeps radio silence"),
        }
    }

    fn log_counterfire(&self, target: &str, amount: i32) -> String {
        format!("Counterfire hits {target}: {amount}")
    }

    fn log_hold_the_line(&self, target: &str, amount: i32) -> String {
        format!("Hold the line hits {target}: {amount}")
    }

    fn log_self_damage(&self, amount: i32) -> String {
        format!("Overcharge strains the flagship: {amount}")
    }

    fn log_burn(&self, target: &str, amount: i32) -> String {
        format!("{target} burns: {amount}")
    }

    fn log_cannibalized(&self, unit: &str, amount: i32) -> String {
        format!("{unit} cannibalizes the wreck: +{amount} hull")
    }

    fn log_emergency_generator(&self) -> &'static str {
        "The emergency generator holds the flagship at 1 hull"
    }

    fn log_shuffled(&self) -> &'static str {
        "Discard pile shuffled into the deck"
    }

    fn log_drawn(&self, count: usize) -> String {
        match count {
            1 => "Drew 1 card".to_owned(),
            _ => format!("Drew {count} cards"),
        }
    }

    fn log_intents(&self, unit: &str, intents: &str) -> String {
        format!("{unit} announces: {intents}")
    }

    fn log_clock(&self, bonus: i32) -> String {
        format!("Battle clock: enemies +{bonus} ATK")
    }

    fn log_battle_ended(&self, result: BattleResult) -> &'static str {
        match result {
            BattleResult::Won => "The enemy fleet is destroyed.",
            BattleResult::Lost => "The flagship is lost.",
        }
    }

    // --- Run ---

    fn menu_tagline(&self) -> &'static str {
        "A wormhole swallowed your flagship. Three sectors lie between you and home."
    }

    fn menu_new_run(&self) -> &'static str {
        "New run"
    }

    fn menu_continue(&self) -> &'static str {
        "Continue"
    }

    fn menu_saved_run(&self, admiral: &str, sector: u32, row: usize) -> String {
        format!("{admiral} · Sector {sector}, row {row}")
    }

    fn sector_name(&self, sector: u32) -> &'static str {
        match sector {
            1 => "The Debris Field",
            2 => "The Silence",
            _ => "The Threshold",
        }
    }

    fn map_title(&self, sector: u32) -> String {
        format!("Sector {sector}: {}", self.sector_name(sector))
    }

    fn map_hint(&self) -> &'static str {
        "Choose the next node. The sector's boss waits at the top."
    }

    fn sector_banner_hint(&self) -> &'static str {
        "The way home goes on."
    }

    fn node_kind(&self, kind: NodeKind) -> &'static str {
        match kind {
            NodeKind::Battle => "Battle",
            NodeKind::Elite => "Elite",
            NodeKind::Event => "Event",
            NodeKind::Shipyard => "Shipyard",
            NodeKind::Market => "Black market",
            NodeKind::Boss => "Boss",
        }
    }

    fn node_kind_hint(&self, kind: NodeKind) -> &'static str {
        match kind {
            NodeKind::Battle => "A formation of this sector. Loot: scrap and a command card.",
            NodeKind::Elite => "A mighty battleship. Loot: lots of scrap and a module.",
            NodeKind::Event => "A scene over the radio with a decision.",
            NodeKind::Shipyard => "Repair, upgrade a card or remove one.",
            NodeKind::Market => "Cards, modules, repairs and removals for scrap.",
            NodeKind::Boss => "The end of the sector. Loot: a module and repairs.",
        }
    }

    fn run_scrap(&self, scrap: u32) -> String {
        format!("Scrap {scrap}")
    }

    fn run_deck(&self, cards: usize) -> String {
        format!("Deck {cards}")
    }

    fn run_show_deck(&self) -> &'static str {
        "View deck"
    }

    fn run_modules(&self, count: usize) -> String {
        format!("Modules {count}")
    }

    fn run_no_modules(&self) -> &'static str {
        "No modules"
    }

    fn run_position(&self, sector: u32, row: usize) -> String {
        format!("Sector {sector} · Row {row}")
    }

    fn run_scrap_change(&self, scrap: i64) -> String {
        format!("{} scrap", signed(scrap))
    }

    fn run_hull_change(&self, hull: i32) -> String {
        format!("{} hull", signed(i64::from(hull)))
    }

    fn run_module_gained(&self, module: &str) -> String {
        format!("New module: {module}")
    }

    fn run_card_upgraded(&self, card: &str) -> String {
        format!("Upgraded: {card}")
    }

    fn loot_title(&self) -> &'static str {
        "Loot"
    }

    fn loot_cards_hint(&self) -> &'static str {
        "One command card joins the deck, or none."
    }

    fn loot_modules_hint(&self) -> &'static str {
        "A module works until the end of the run."
    }

    fn loot_take(&self) -> &'static str {
        "Take"
    }

    fn loot_skip(&self) -> &'static str {
        "None"
    }

    fn event_title(&self, event: EventId) -> &'static str {
        match event {
            EventId::Resolute => "The Wreck of the Resolute",
            EventId::Pilot => "The Pilot",
            EventId::Flicker => "Flicker",
        }
    }

    fn event_text(&self, event: EventId) -> &'static str {
        match event {
            EventId::Resolute => {
                "A cruiser of your own fleet drifts burnt out in the field: the Resolute. The wormhole swallowed her \
                 too. Her reactor still glows, her beacon repeats the last report in a loop."
            }
            EventId::Pilot => {
                "A trader hails you on an open channel. A minefield lies ahead, and he knows the safe way through, \
                 for a price."
            }
            EventId::Flicker => {
                "The wormhole stirs. For a moment space flickers around the flagship, and in the static lies something \
                 familiar."
            }
        }
    }

    fn event_option(&self, option: EventOption) -> &'static str {
        match option {
            EventOption::Salvage => "Salvage",
            EventOption::Logbooks => "Secure the logbooks",
            EventOption::MoveOn => "Move on and bury the crew",
            EventOption::PayPilot => "Pay the pilot",
            EventOption::FlyYourself => "Fly yourself",
            EventOption::Embrace => "Fly into the flicker",
            EventOption::Listen => "Listen",
        }
    }

    fn event_consequence(&self, option: EventOption, flicker_damage: i32) -> String {
        use voidfleet_core::run::{PILOT_DAMAGE, PILOT_PRICE, RESOLUTE_SCRAP};
        match option {
            EventOption::Salvage => {
                format!("+{RESOLUTE_SCRAP} scrap; the next battle starts burning.")
            }
            EventOption::Logbooks => "A random module.".to_owned(),
            EventOption::MoveOn => "Nothing.".to_owned(),
            EventOption::PayPilot => {
                format!("{PILOT_PRICE} scrap; the next node becomes a shipyard.")
            }
            EventOption::FlyYourself => format!("−{PILOT_DAMAGE} hull, but a card offer."),
            EventOption::Embrace => {
                format!("−{flicker_damage} hull; a random card is upgraded.")
            }
            EventOption::Listen => "Nothing happens.".to_owned(),
        }
    }

    fn event_outcome(&self, option: EventOption) -> &'static str {
        match option {
            EventOption::Salvage => {
                "The salvage crews cut out whatever they can carry. As you cast off, a leak in the hangar ignites."
            }
            EventOption::Logbooks => "The logbooks hold blueprints the Resolute no longer needs.",
            EventOption::MoveOn => {
                "The crew gets their burial in space. Over the radio, the Resolute's last report: \"We hold the \
                 line.\""
            }
            EventOption::PayPilot => "The trader sends his route. A dock waits at its end.",
            EventOption::FlyYourself => {
                "Two mines graze the hull. Containers of command sets drift in the field."
            }
            EventOption::Embrace => {
                "The flicker burns through the systems and leaves something behind."
            }
            EventOption::Listen => {
                "For the first time since the jump: the radio from home. Then static again."
            }
        }
    }

    fn shipyard_title(&self) -> &'static str {
        "Shipyard"
    }

    fn shipyard_hint(&self) -> &'static str {
        "One job, then you move on."
    }

    fn shipyard_repair(&self) -> &'static str {
        "Repair"
    }

    fn shipyard_repair_text(&self, hull: i32) -> String {
        format!("+{hull} hull.")
    }

    fn shipyard_upgrade(&self) -> &'static str {
        "Upgrade"
    }

    fn shipyard_upgrade_text(&self) -> &'static str {
        "One card of the deck grows stronger."
    }

    fn shipyard_remove(&self) -> &'static str {
        "Remove"
    }

    fn shipyard_remove_text(&self) -> &'static str {
        "One card leaves the deck."
    }

    fn gallery_deck(&self) -> &'static str {
        "Deck"
    }

    fn gallery_upgrade_hint(&self) -> &'static str {
        "Choose a card that is not upgraded yet."
    }

    fn gallery_remove_hint(&self) -> &'static str {
        "Choose the card that leaves the deck."
    }

    fn gallery_back(&self) -> &'static str {
        "Back"
    }

    fn market_title(&self) -> &'static str {
        "Black market"
    }

    fn market_hint(&self) -> &'static str {
        "Paid in scrap. No questions asked."
    }

    fn market_cards(&self) -> &'static str {
        "Command cards"
    }

    fn market_modules(&self) -> &'static str {
        "Modules"
    }

    fn market_services(&self) -> &'static str {
        "Services"
    }

    fn market_buy(&self, price: u32) -> String {
        format!("Buy · {price}")
    }

    fn market_removal(&self) -> &'static str {
        "Remove a card"
    }

    fn market_repair(&self, hull: i32) -> String {
        format!("Repair {hull} hull")
    }

    fn market_sold_out(&self) -> &'static str {
        "Sold"
    }

    fn market_leave(&self) -> &'static str {
        "Leave"
    }

    fn run_won_title(&self) -> &'static str {
        "The Wormhole Home"
    }

    fn run_won_text(&self) -> &'static str {
        "Commander Vey has fallen, the wormhole stands open. On the other side: your own fleet."
    }

    fn run_lost_text(&self, sector: u32, row: usize) -> String {
        format!("The flagship fell in sector {sector}, row {row}.")
    }

    fn run_abandon(&self) -> &'static str {
        "Abandon run"
    }

    fn run_abandon_title(&self) -> &'static str {
        "Abandon the run?"
    }

    fn run_abandon_text(&self) -> &'static str {
        "The save will be deleted. This cannot be undone."
    }

    fn run_abandon_confirm(&self) -> &'static str {
        "Abandon"
    }

    fn run_abandon_back(&self) -> &'static str {
        "Keep flying"
    }
    // --- Gefechtsbericht ---

    fn mark_evaded(&self, amount: i32) -> String {
        format!("evades: {amount}")
    }

    fn mark_countermeasures(&self, amount: i32) -> String {
        format!("countermeasures −{amount}")
    }

    fn mark_armor(&self, amount: i32) -> String {
        format!("armor −{amount}")
    }

    fn mark_covered(&self, amount: i32, by: &str) -> String {
        format!("covered: {amount} → {by}")
    }

    fn mark_shielded(&self, amount: i32) -> String {
        format!("shield absorbs {amount}")
    }

    fn report_missed(&self) -> &'static str {
        "into the void"
    }

    fn report_burned_out(&self, unit: &str) -> String {
        format!("{unit} burned out")
    }
}
