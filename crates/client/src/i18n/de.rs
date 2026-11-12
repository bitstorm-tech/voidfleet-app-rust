//! Deutsch: Die Funksprüche sind nur Untertitel zu den englischen Aufnahmen, je Spruch gleich viele Varianten.

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
    enemies::{Action, BOLT_SHIELD, EnemyClass, EnemyId},
    events::{DestroyCause, IdleReason, MissReason},
    modules::ModuleId,
    run::{EventId, EventOption, NodeKind},
};

use super::{AdmiralText, Condition, Messages, RulesChapter, RulesSection, SettingsTab, signed};
use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, Antialiasing, CursorId, WindowMode},
};

pub struct De;

fn section(heading: &'static str, paragraphs: &[&'static str]) -> RulesSection {
    RulesSection {
        heading,
        paragraphs: paragraphs.iter().map(|text| Cow::Borrowed(*text)).collect(),
    }
}

impl Messages for De {
    fn language_name(&self) -> &'static str {
        "Deutsch"
    }

    fn hud_round(&self, round: u32) -> String {
        format!("Runde {round}")
    }

    fn hud_new_round(&self) -> &'static str {
        "Neue Runde"
    }

    fn hud_end_turn(&self) -> &'static str {
        "Zug beenden"
    }

    fn hud_skip(&self) -> &'static str {
        "Animation überspringen"
    }

    fn hud_show_log(&self) -> &'static str {
        "Log einblenden"
    }

    fn hud_hide_log(&self) -> &'static str {
        "Log ausblenden"
    }

    fn admiral_select_title(&self) -> &'static str {
        "Wähle deinen Admiral"
    }

    fn admiral_select_choose(&self) -> &'static str {
        "Wählen"
    }

    fn settings_title(&self) -> &'static str {
        "Einstellungen"
    }

    fn settings_tab(&self, tab: SettingsTab) -> &'static str {
        match tab {
            SettingsTab::Game => "Spiel",
            SettingsTab::Video => "Video",
            SettingsTab::Sound => "Sound",
            SettingsTab::Dev => "Dev",
        }
    }

    fn settings_window_mode(&self) -> &'static str {
        "Darstellung"
    }

    fn window_mode(&self, mode: WindowMode) -> &'static str {
        match mode {
            WindowMode::Windowed => "Fenster",
            WindowMode::Maximized => "Maximiert",
            WindowMode::Fullscreen => "Vollbild",
        }
    }

    fn settings_antialiasing(&self) -> &'static str {
        "Kantenglättung"
    }

    fn antialiasing(&self, level: Antialiasing) -> &'static str {
        match level {
            Antialiasing::Off => "Aus",
            Antialiasing::X2 => "Zweifach",
            Antialiasing::X4 => "Vierfach",
            Antialiasing::X8 => "Achtfach",
        }
    }

    fn settings_models(&self) -> &'static str {
        "3D-Modelle"
    }

    fn models_hint(&self) -> &'static str {
        "Mausrad: Zoom · Linke Maustaste ziehen: verschieben · Rechte Maustaste ziehen: drehen · Klick auf ein Schiff: Zerstörung"
    }

    fn settings_show_fps(&self) -> &'static str {
        "FPS-Anzeige"
    }

    fn on_off(&self, on: bool) -> &'static str {
        if on { "An" } else { "Aus" }
    }

    fn settings_music_volume(&self) -> &'static str {
        "Musik"
    }

    fn settings_effects_volume(&self) -> &'static str {
        "Soundeffekte"
    }

    fn settings_voice_volume(&self) -> &'static str {
        "Bordfunk"
    }

    fn settings_attack_speed(&self) -> &'static str {
        "Geschwindigkeit der Angriffe"
    }

    fn settings_movement_speed(&self) -> &'static str {
        "Geschwindigkeit der Bewegungen"
    }

    fn settings_animation_speed(&self) -> &'static str {
        "Übrige Animationen"
    }

    fn settings_language(&self) -> &'static str {
        "Sprache"
    }

    fn speed(&self, speed: AnimationSpeed) -> &'static str {
        match speed {
            AnimationSpeed::Slow => "Langsam",
            AnimationSpeed::Normal => "Normal",
            AnimationSpeed::Fast => "Schnell",
            AnimationSpeed::VeryFast => "Sehr schnell",
        }
    }

    fn settings_cursor(&self) -> &'static str {
        "Mauszeiger"
    }

    fn settings_cursor_size(&self) -> &'static str {
        "Zeigergröße"
    }

    fn cursor(&self, cursor: CursorId) -> &'static str {
        match cursor {
            CursorId::Arrow => "Toon-Pfeil",
            CursorId::Fighter => "Jäger",
            CursorId::Delta => "Delta-Gleiter",
            CursorId::Flagship => "Flaggschiff",
            CursorId::Needle => "Navigationsnadel",
            CursorId::Shard => "Kristallsplitter",
            CursorId::Formation => "Formation",
            CursorId::Holo => "Holo-Pfeil",
            CursorId::Thruster => "Triebwerk",
            CursorId::Crosshair => "Zielkreuz",
        }
    }

    fn rules_title(&self) -> &'static str {
        "Spielregeln"
    }

    fn dialog_close(&self) -> &'static str {
        "Schließen"
    }

    fn fullscreen_enter(&self) -> &'static str {
        "Vollbild (F)"
    }

    fn fullscreen_exit(&self) -> &'static str {
        "Vollbild beenden (F)"
    }

    fn camera_view_toggle(&self) -> &'static str {
        "Ansicht wechseln (V); stufenlos drehen mit ← → oder der mittleren Maustaste"
    }

    fn loading_assets(&self, percent: u32) -> String {
        format!("Lade Assets … {percent} %")
    }

    fn loading_failed(&self) -> &'static str {
        "Laden fehlgeschlagen — starte das Spiel neu."
    }

    fn radio_speaker(&self, intercepted: bool) -> &'static str {
        if intercepted {
            "Abgehört"
        } else {
            "Flottenfunk"
        }
    }

    fn log_round_header(&self, round: u32) -> String {
        format!("— Runde {round} —")
    }

    fn stats_atk(&self) -> &'static str {
        "ATK"
    }

    fn hud_clock(&self, bonus: i32) -> String {
        format!("Gefechtsuhr +{bonus} ATK")
    }

    fn hud_clock_hint(&self, bonus: i32) -> String {
        format!(
            "Ab Runde {CLOCK_START} feuert jeder Gegner je Runde mit +1 ATK mehr; jetzt +{bonus}."
        )
    }

    fn hud_hull(&self, hull: i32, max_hull: i32) -> String {
        format!("Hülle {hull}/{max_hull}")
    }

    fn hud_points(&self) -> &'static str {
        "Befehlspunkte"
    }

    fn hud_draw_pile(&self) -> &'static str {
        "Nachziehstapel"
    }

    fn hud_discard_pile(&self) -> &'static str {
        "Ablagestapel"
    }

    fn hud_stations(&self) -> &'static str {
        "Stationen"
    }

    fn checklist_free_stations(&self, count: usize) -> String {
        match count {
            1 => "1 Station frei".to_owned(),
            _ => format!("{count} Stationen frei"),
        }
    }

    fn checklist_station(&self, station: &str) -> String {
        format!("Station: {station}")
    }

    fn checklist_points(&self, points: u32) -> String {
        match points {
            1 => "1 Befehlspunkt offen".to_owned(),
            _ => format!("{points} Befehlspunkte offen"),
        }
    }

    fn admiral_select_hint(&self) -> &'static str {
        "Ein Flaggschiff gegen ihre Flotte. Dein Admiral bringt das Schiff, ein Passiv, eine Admiralsstation und zwei Kennkarten im Startdeck mit."
    }

    fn label_flagship(&self) -> &'static str {
        "Flaggschiff"
    }

    fn label_weapon(&self) -> &'static str {
        "Waffe"
    }

    fn label_passive(&self) -> &'static str {
        "Passiv"
    }

    fn label_station(&self) -> &'static str {
        "Admiralsstation"
    }

    fn label_signature(&self) -> &'static str {
        "Kennkarte"
    }

    fn rules_section(&self, chapter: RulesChapter) -> RulesSection {
        match chapter {
            RulesChapter::Board => section(
                "Brett",
                &[
                    "Dein Flaggschiff steht allein in einer von drei Spalten. Gegenüber hält die gegnerische Flotte zwei Reihen, Front und Back, mit je drei Spalten und höchstens vier Einheiten.",
                    "Eine Einheit in der Back-Reihe ist gedeckt, solange in ihrer Spalte vorn eine Einheit steht: Der Vordermann fängt von jedem Treffer die aufgerundete Hälfte ab, das Ziel bekommt den Rest.",
                    "Gegner feuern auf zwei Arten. Zielfeuer folgt deinem Flaggschiff, wohin es auch fliegt. Spaltenfeuer wird mit Spalte angesagt und trifft, was im Gegnerzug dort steht.",
                ],
            ),
            RulesChapter::Round => section(
                "Runde",
                &[
                    "Eine Runde ist dein Zug, der Gegnerzug und die Auflösung. Vor Runde 1 siehst du Formation und Absichten und darfst mit einem Klick auf deine Seite deine Spalte wählen.",
                    "In deinem Zug wirken Karten und Station sofort, in deiner Reihenfolge. Karten kosten Befehlspunkte: 3 je Runde, was übrig bleibt, verfällt. Beende den Zug, wenn du fertig bist; eine Runde ohne Station ist erlaubt.",
                    "Im Gegnerzug tut jede Einheit, was sie angesagt hat, Front vor Back, links nach rechts. Dann ticken Brände, Schilde verfallen, die Hand wird abgeworfen und du ziehst fünf neue Karten.",
                    "Ab Runde 8 gibt die Gefechtsuhr allen Gegnern je Runde +1 ATK.",
                ],
            ),
            RulesChapter::Stations => section(
                "Stationen",
                &[
                    "Dein Flaggschiff löst eine Station je Runde aus, mit Alarmstufe Rot eine zweite, andere. Sie wird wie eine Karte ausgelöst und bekommt dabei ihr Ziel: Waffen, Fokusfeuer und Stillstand ein gegnerisches Schiff, Manöver eine andere Spalte; die übrigen Stationen brauchen keins. Sie wirkt sofort.",
                    "Waffen feuern deine ATK auf das Ziel. Schilde schlucken im nächsten Gegnerzug 8 (Osei 12). Schadenskontrolle löscht den Brand und repariert 5 Hülle. Manöver wechselt die Spalte und entgeht dem Spaltenfeuer auf die alte.",
                    "Die Admiralsstation hängt von deinem Admiral ab und geht nicht, solange die Brücke getroffen ist (Hülle bei 25 % oder weniger).",
                ],
            ),
            RulesChapter::Damage => section(
                "Schaden",
                &[
                    "Jeder Schuss trifft, und jede Zahl steht fest: Was die Vorschau zeigt, geschieht.",
                    "Vom Schaden eines Schusses geht in dieser Reihenfolge ab: Ausweichen halbiert, Panzerung zieht ab, Deckung gibt die Hälfte an den Vordermann, Schilde schlucken den Rest, bis sie aufgebraucht sind.",
                    "Spaltenfeuer und Rammkurs treffen voll oder gar nicht; Ausweichen zählt für sie nicht.",
                ],
            ),
            RulesChapter::Conditions => section(
                "Zustände",
                &[
                    "Brand: Ein Treffer, der eine Einheit auf 50 % Hülle oder weniger bringt, setzt sie in Brand. Wer brennt, verliert in jeder Auflösung 2 Hülle; den eigenen Brand löschen Schadenskontrolle und Reparaturtrupp.",
                    "Notfall: Bei 25 % Hülle oder weniger feuert eine Einheit mit halber ATK und weicht nicht mehr aus. Dein Flaggschiff kann dann seine Admiralsstation nicht auslösen.",
                    "Zerstört: Ein Gegner mit Hülle 0 fällt sofort und handelt nie mehr. Fällt dein Flaggschiff, ist das Gefecht verloren.",
                ],
            ),
            RulesChapter::Admirals => section(
                "Admirale",
                &[
                    "Osei, die Mauer: Bastion, 70 Hülle, 7 ATK. Schilde schlucken 12. Gegenfeuer: Schilde 6, Abwehrfeuer 3.",
                    "Vance, der Hammer: Vanguard, 50 Hülle, 11 ATK. Vorstoß: +1 ATK je Runde in Folge mit Waffen oder Fokusfeuer, höchstens +4. Fokusfeuer: doppelter Vorstoß, zielsuchend, dafür je Treffer 2 mehr.",
                    "Kerr, der Uhrmacher: Pendulum, 56 Hülle, 8 ATK. Die Railgun ist zielsuchend und durchschlagend; er sieht die Absichten zwei Runden voraus. Stillstand: Ein Gegner handelt in dieser Runde nicht.",
                    "Reyes, die Glut: Pyre, 56 Hülle, 9 ATK. Der Plasmawerfer setzt jedes Ziel in Brand, Brände bei Gegnern ticken 3. Nachsetzen: Jeder brennende Gegner nimmt sofort das Doppelte seines Brandschadens, Angriffskarten auf brennende Ziele +2.",
                ],
            ),
            RulesChapter::Cards => section(
                "Karten",
                &[
                    "Jede Runde ziehst du fünf Befehle. Angriffskarten treffen mit festem Schaden und den üblichen Abzügen; Verstärkungen stärken dein Flaggschiff, Schwächungen einen Gegner, Taktiken verändern das Brett. Doktrinen bleiben bis zum Gefechtsende liegen.",
                    "Jeder Admiral beginnt mit zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zwei Kennkarten. Aufgewertete Karten (+) machen oder schlucken 2 mehr oder kosten 1 weniger.",
                    "Schlüsselwörter: Zielsuchend – Ausweichen zählt nicht. Durchschlagend – Deckung zählt nicht. Schilde N – schluckt N Schaden im folgenden Gegnerzug. Panzerung N – jeder Treffer macht N weniger. Abwehrfeuer N – jeder Schütze, der dein Flaggschiff trifft, verliert N Hülle. Brand – wer brennt, verliert in jeder Auflösung 2 Hülle.",
                ],
            ),
            RulesChapter::Enemies => section(
                "Gegner und Absichten",
                &[
                    "Jeder Gegner folgt einem festen Muster und zeigt, was er als Nächstes tut: Zielfeuer mit Schaden, Spaltenfeuer mit Spalte und Schaden, Schilde, Ausweichen, Aufladen, Zielerfassung oder Verstärkung rufen.",
                    "Du handelst zuerst: Ein Gegner, der in deinem Zug fällt, führt seine Absicht nie aus, einer im Notfall mit halber ATK. Ein Kurzsprung lässt jedes Feuer ins Leere gehen, ein Manöver nur das Spaltenfeuer auf die alte Spalte.",
                    "Jäger weichen aus und schirmen ab, Kreuzer sind das Rückgrat, Schlachtschiffe laden ihre Hauptgeschütze auf und tragen Panzerung.",
                ],
            ),
            RulesChapter::Controls => section(
                "Steuerung",
                &[
                    "Klicke eine Karte, um sie zu spielen; eine Karte mit Ziel wartet auf einen Klick auf einen leuchtenden Gegner oder eine Spalte. Ein zweiter Klick auf die Karte, Rechtsklick oder Escape bricht ab.",
                    "Stationen gehen genauso: Klicke eine Station im Stationen-Feld an; ohne Ziel wirkt sie sofort, sonst leuchten die gültigen Ziele, und der Klick auf eines löst sie aus. Unter dem Zeiger zeigt ein leuchtender Gegner den Schaden der gewählten Karte oder Station.",
                    "Leertaste überspringt die Wiedergabe, V wechselt die Ansicht, ← → drehen, ↑ ↓ zoomen, F schaltet das Vollbild, Escape öffnet die Einstellungen.",
                ],
            ),
        }
    }

    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str] {
        match cue {
            RadioCue::BattleStart => &[
                "Alle Schiffe auf Gefechtsstation. Wir erwarten Ihre Befehle, Admiral.",
                "Flotte in Position. Bereit, sobald Sie es sind, Admiral.",
            ],
            RadioCue::Victory => &[
                "Feindliche Flotte vernichtet. Der Sektor gehört uns, Admiral!",
                "Das war der Letzte von ihnen. Sieg, Admiral!",
            ],
            RadioCue::Defeat => &[
                "Wir haben die Flotte verloren. Alle Mann von Bord!",
                "Unsere Flotte ist vernichtet. Alle Überlebenden: Rückzug!",
            ],
            RadioCue::EnemyDestroyed => &[
                "Feindschiff zerstört!",
                "Ziel ausgeschaltet!",
                "Einer weniger!",
                "Abschuss bestätigt!",
            ],
            RadioCue::HullCritical => &[
                "Hüllenintegrität kritisch!",
                "Das Schiff ist schwer getroffen, Admiral!",
            ],
            RadioCue::IncomingFire => &[
                "Feindfeuer im Anflug! Festhalten!",
                "Sie eröffnen das Feuer auf uns!",
            ],
            RadioCue::ColumnFireIncoming => &[
                "Strahlwaffen laden auf unsere Linie!",
                "Sie haben unsere Spalte im Visier! Bringt uns da raus!",
            ],
            RadioCue::FireOnboard => &[
                "Feuer auf Deck vier! Schadenskontrolle, los!",
                "Feuer im Hauptkorridor!",
            ],
            RadioCue::FireOut => &[
                "Feuer gelöscht. Wir lüften den Rauch ab.",
                "Brand eingedämmt, Admiral.",
            ],
            RadioCue::ShieldsUp => &["Schilde hoch!", "Alle Energie auf die Frontschilde!"],
            RadioCue::Maneuver => &[
                "Ruder hart Backbord! Wir wechseln die Bahn!",
                "Wir drehen bei, neuer Kurs!",
            ],
            RadioCue::MicroJump => &[
                "Antrieb läuft an. Sprung in drei, zwei, eins!",
                "Kurzsprung, jetzt!",
            ],
            RadioCue::JumpReturn => &[
                "Sprung beendet. Wir sind zurück im Gefecht.",
                "Zurück auf dem Feld. Waffen scharf.",
            ],
            RadioCue::RamIncoming => &[
                "Brander auf Rammkurs!",
                "Der rammt uns! Auf Aufprall gefasst machen!",
            ],
            RadioCue::ReinforcementsSighted => &[
                "Neue Kontakte! Feindliche Verstärkung springt ein!",
                "Es werden mehr, Admiral. Frische Feinde auf dem Feld.",
            ],
            RadioCue::EmergencyGenerator => &[
                "Hauptenergie weg! Das Notaggregat hält uns!",
                "Notstrom an! Wir atmen noch!",
            ],
            RadioCue::Repairs => &[
                "Reparaturtrupps an der Hüllenbresche.",
                "Schadenskontrolle, flickt sie zusammen!",
            ],
            RadioCue::TargetLocked => &[
                "Ziel erfasst. Feuer!",
                "Feuerlösung steht. Geschütze feuern!",
            ],
            RadioCue::DirectHit => &[
                "Volltreffer!",
                "Guter Treffer! Hülle des Ziels aufgerissen!",
            ],
            RadioCue::EnemyBurning => &["Ziel brennt!", "Der brennt! Weiter so!"],
            RadioCue::TorpedoAway => &["Torpedo los!", "Torpedo läuft!"],
            RadioCue::OrbitalStrike => &["Orbitalschlag kommt!", "Wir holen es von oben runter!"],
            RadioCue::RailgunFire => &["Railgun geladen. Feuer!", "Projektil raus!"],
            RadioCue::PlasmaFire => &[
                "Plasma bereit. Brennt sie nieder!",
                "Plasma ablassen, jetzt!",
            ],
            RadioCue::Counterfire => &[
                "Gegenfeuer! Zahlt es ihnen heim!",
                "Sie treffen uns, wir schlagen zurück!",
            ],
            RadioCue::EnemyPanic => &[
                "Mayday, Mayday! Wir brechen auseinander!",
                "Der Reaktor versagt! Helft uns doch!",
            ],
            RadioCue::EnemyCharging => &[
                "Hauptbatterie lädt. Feuer halten.",
                "Volle Energie aufs Hauptgeschütz. Auf mein Zeichen.",
            ],
            RadioCue::EnemyCallsReinforcements => &[
                "An alle Schiffe: Sammeln auf meiner Position!",
                "Wir brauchen Verstärkung! Schickt alles, was ihr habt!",
            ],
            RadioCue::EnemyBreaking => &[
                "Abdrehen! Abdrehen! Alle weg hier!",
                "Wir können sie nicht halten! Rückzug!",
            ],
        }
    }

    fn result_title(&self, result: BattleResult) -> &'static str {
        match result {
            BattleResult::Won => "Sieg",
            BattleResult::Lost => "Niederlage",
        }
    }

    fn result_won_subtitle(&self, kills: u32, rounds: u32) -> String {
        let units = if kills == 1 { "Einheit" } else { "Einheiten" };
        let round_word = if rounds == 1 { "Runde" } else { "Runden" };
        format!("{kills} feindliche {units} in {rounds} {round_word} zerstört.")
    }

    fn result_lost_subtitle(&self, flagship: &str) -> String {
        format!("Die {flagship} ist verloren. Der Run ist vorbei.")
    }

    fn result_view_board(&self) -> &'static str {
        "Brett ansehen"
    }

    fn result_continue(&self) -> &'static str {
        "Weiter"
    }

    fn result_new_run(&self) -> &'static str {
        "Neuer Run"
    }

    fn rule_error(&self, error: RuleError) -> &'static str {
        match error {
            RuleError::BattleOver => "Das Gefecht ist vorbei.",
            RuleError::NotEnoughPoints => "Nicht genug Befehlspunkte.",
            RuleError::TargetInvalid => "Kein gültiges Ziel.",
            RuleError::AdmiralStationInEmergency => "Brücke getroffen: keine Admiralsstation.",
            RuleError::Jumped => "Das Flaggschiff ist im Kurzsprung.",
            RuleError::StationUsed => "Diese Station lief in dieser Runde schon.",
            RuleError::NoStationSlot => "Keine Station mehr frei in dieser Runde.",
        }
    }

    fn stats_hull(&self) -> &'static str {
        "Hülle"
    }

    fn station(&self, station: Station, admiral: AdmiralId) -> &'static str {
        match station {
            Station::Weapons => "Waffen",
            Station::Shields => "Schilde",
            Station::DamageControl => "Schadenskontrolle",
            Station::Maneuver => "Manöver",
            Station::Admiral => self.admiral(admiral).station,
        }
    }

    fn station_text(&self, station: Station, admiral: AdmiralId, attack: i32) -> String {
        match station {
            Station::Weapons => format!(
                "Feuert sofort {} Schaden auf ein gegnerisches Schiff.",
                attack
            ),
            Station::Shields => format!(
                "Schluckt im nächsten Gegnerzug {}.",
                admiral.shield_station()
            ),
            Station::DamageControl => {
                format!("Löscht den Brand und repariert {DAMAGE_CONTROL_REPAIR} Hülle.")
            }
            Station::Maneuver => {
                "Wechselt sofort in eine andere Spalte und entgeht dem Spaltenfeuer auf die alte."
                    .to_owned()
            }
            Station::Admiral => self.admiral(admiral).station_text.to_owned(),
        }
    }

    fn column_name(&self, column: usize) -> &'static str {
        match column {
            0 => "links",
            1 => "Mitte",
            _ => "rechts",
        }
    }

    fn row_name(&self, row: Row) -> &'static str {
        match row {
            Row::Front => "Front",
            Row::Back => "Back",
        }
    }

    fn card_name(&self, card: CardId) -> &'static str {
        match card {
            CardId::Torpedo => "Torpedo",
            CardId::OrbitalStrike => "Orbitalschlag",
            CardId::VolleyFire => "Salvenfeuer",
            CardId::Barrage => "Sperrfeuer",
            CardId::HeatSeeker => "Wärmesucher",
            CardId::CoupDeGrace => "Gnadenstoß",
            CardId::Bombardment => "Bombardement",
            CardId::CollisionCourse => "Rammkurs",
            CardId::Firebomb => "Brandsatz",
            CardId::EmergencyShields => "Notschilde",
            CardId::HoldTheLine => "Standhalten",
            CardId::AblativeArmor => "Ablativpanzerung",
            CardId::Overcharge => "Überladung",
            CardId::PowerSurge => "Energiestoß",
            CardId::RepairCrew => "Reparaturtrupp",
            CardId::EmpBlast => "EMP-Stoß",
            CardId::TargetingJammer => "Zielstörer",
            CardId::RadioSilence => "Funkstille",
            CardId::TractorBeam => "Traktorstrahl",
            CardId::MicroJump => "Kurzsprung",
            CardId::EvasiveAction => "Ausweichmanöver",
            CardId::Countermeasures => "Gegenmaßnahmen",
            CardId::Scavenge => "Plündern",
            CardId::RedAlert => "Alarmstufe Rot",
            CardId::FireControlDoctrine => "Feuerleitdoktrin",
            CardId::BattleStations => "Alarmbereitschaft",
        }
    }

    fn card_bonus(&self, bonus: i32) -> String {
        format!(
            "{}: +{bonus} Schaden.",
            self.card_name(CardId::FireControlDoctrine)
        )
    }

    fn card_effect(&self, card: DeckCard, v: i32) -> String {
        match card.id {
            CardId::Torpedo => format!("{v} Schaden auf ein gewähltes Schiff. Zielsuchend."),
            CardId::OrbitalStrike => format!("{v} Schaden auf ein gewähltes Schiff. Durchschlagend."),
            CardId::VolleyFire => format!("{v} Schaden auf jede gegnerische Einheit. Durchschlagend."),
            CardId::Barrage => format!("{v} Schaden auf jedes gegnerische Schiff, das gerade ausweicht. Zielsuchend."),
            CardId::HeatSeeker => format!(
                "{v} Schaden auf ein gewähltes Schiff, {}, wenn es ausweicht. Zielsuchend, durchschlagend.",
                v + HEAT_SEEKER_EVADING_BONUS
            ),
            CardId::CoupDeGrace => format!(
                "{v} Schaden auf ein gewähltes Schiff, {}, wenn es im Notfall ist.",
                v + COUP_DE_GRACE_EMERGENCY_BONUS
            ),
            CardId::Bombardment => format!("{v} Schaden auf jede Einheit in der gegnerischen Back-Reihe. Durchschlagend."),
            CardId::CollisionCourse => {
                let bonus = if v > 0 { format!(" +{v}") } else { String::new() };
                format!("Rammt ein gewähltes Schiff: Es nimmt das Doppelte deiner ATK{bonus}, dein Flaggschiff seine ATK. Zielsuchend, durchschlagend.")
            }
            CardId::Firebomb => format!("{v} Schaden auf ein gewähltes Schiff, Brand."),
            CardId::EmergencyShields => format!("Schilde {v}."),
            CardId::HoldTheLine => format!("Schilde {v}, Abwehrfeuer {HOLD_THE_LINE_RETALIATION}."),
            CardId::AblativeArmor => format!("Doktrin: Panzerung {v}."),
            CardId::Overcharge => format!(
                "+{v} ATK in dieser Runde; dein Flaggschiff nimmt am Zugende {OVERCHARGE_SELF_DAMAGE} Schaden."
            ),
            CardId::PowerSurge => format!("+{v} ATK in dieser Runde."),
            CardId::RepairCrew => format!("Löscht den Brand und repariert {v} Hülle, zusätzlich zur Station."),
            CardId::EmpBlast => "Ein gewähltes Schiff verliert Schilde und Aufladung.".to_owned(),
            CardId::TargetingJammer => "Alle gegnerischen Schüsse im Gegnerzug −1 ATK.".to_owned(),
            CardId::RadioSilence => {
                "Ein gewähltes Schiff feuert in dieser Runde nicht und setzt seine Sonderaktion aus; eine Aufladung bricht ab.".to_owned()
            }
            CardId::TractorBeam => {
                "Zieht ein gewähltes Schiff (kein Flaggschiff) in einen freien Slot seiner Reihe; Deckung bricht.".to_owned()
            }
            CardId::MicroJump => {
                "Dein Flaggschiff verlässt das Brett: danach in dieser Runde keine Station mehr, kein Ziel im Gegnerzug; es kehrt in der Auflösung zurück.".to_owned()
            }
            CardId::EvasiveAction => "Dein Flaggschiff wechselt sofort die Spalte, ohne eine Station zu verbrauchen.".to_owned(),
            CardId::Countermeasures => "Der erste gegnerische Angriff im Gegnerzug macht halben Schaden.".to_owned(),
            CardId::Scavenge => format!("{SCAVENGE_DRAW} Befehle ziehen."),
            CardId::RedAlert => "Löse in dieser Runde eine zweite, andere Station aus.".to_owned(),
            CardId::FireControlDoctrine => {
                format!("Doktrin: Alle eigenen Schüsse und Angriffskarten +{v} Schaden.")
            }
            CardId::BattleStations => {
                format!("Doktrin: In jedem Gegnerzug Schilde {v}.")
            }
        }
    }

    fn keywords(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("Zielsuchend", "Ausweichen zählt nicht."),
            ("Durchschlagend", "Deckung zählt nicht."),
            (
                "Schilde",
                "Schluckt so viel Schaden im folgenden Gegnerzug.",
            ),
            ("Panzerung", "Jeder Treffer macht so viel weniger."),
            (
                "Abwehrfeuer",
                "Jeder Schütze, der dein Flaggschiff trifft, verliert so viel Hülle.",
            ),
            ("Brand", "Wer brennt, verliert in jeder Auflösung 2 Hülle."),
            (
                "Notfall",
                "Bei 25 % Hülle oder weniger: halbe ATK, kein Ausweichen.",
            ),
            ("Doktrin", "Gilt für den Rest des Gefechts."),
        ]
    }

    fn card_type(&self, card_type: CardType) -> &'static str {
        match card_type {
            CardType::Attack => "Angriff",
            CardType::Buff => "Verstärkung",
            CardType::Debuff => "Schwächung",
            CardType::Tactic => "Taktik",
            CardType::Doctrine => "Doktrin",
        }
    }

    fn card_target(&self, target: CardTarget) -> &'static str {
        match target {
            CardTarget::None => "Ohne Ziel",
            CardTarget::Enemy => "Gegnerisches Schiff",
            CardTarget::Column => "Eigene Spalte",
        }
    }

    fn admiral(&self, admiral: AdmiralId) -> AdmiralText {
        match admiral {
            AdmiralId::Osei => AdmiralText {
                name: "Osei",
                epithet: "Die Mauer",
                weapon: "Flakbatterien",
                passive: "Wächterschild: Schilde schlucken 12 statt 8.",
                station: "Gegenfeuer",
                station_text: "Feuert nicht. Schilde 6, Abwehrfeuer 3.",
            },
            AdmiralId::Vance => AdmiralText {
                name: "Vance",
                epithet: "Der Hammer",
                weapon: "Schwere Geschütze",
                passive: "Vorstoß: +1 ATK je Runde in Folge mit Waffen oder Fokusfeuer, höchstens +4; jede andere Station setzt zurück.",
                station: "Fokusfeuer",
                station_text: "Feuert mit doppeltem Vorstoß, zielsuchend; nimmt im nächsten Gegnerzug je Treffer 2 mehr.",
            },
            AdmiralId::Kerr => AdmiralText {
                name: "Kerr",
                epithet: "Der Uhrmacher",
                weapon: "Railgun: zielsuchend, durchschlagend",
                passive: "Uhrwerk: sieht die Absichten der Gegner zwei Runden voraus.",
                station: "Stillstand",
                station_text: "Feuert nicht. Ein gewählter Gegner handelt in dieser Runde nicht; seine Absicht verschiebt sich, eine Aufladung bleibt.",
            },
            AdmiralId::Reyes => AdmiralText {
                name: "Reyes",
                epithet: "Die Glut",
                weapon: "Plasmawerfer: jeder Treffer setzt das Ziel in Brand",
                passive: "Glutkern: Brände bei Gegnern ticken 3 statt 2.",
                station: "Nachsetzen",
                station_text: "Feuert nicht. Jeder brennende Gegner nimmt sofort das Doppelte seines Brandschadens; Angriffskarten auf brennende Ziele +2 für den Rest der Runde.",
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
            EnemyId::Raiders => "Plünderer-Staffel",
            EnemyId::Fireship => "Brander",
            EnemyId::TorpedoWing => "Torpedo-Staffel",
            EnemyId::GuardCruiser => "Wachkreuzer",
            EnemyId::PlasmaCruiser => "Plasmakreuzer",
            EnemyId::Spotter => "Spotter",
            EnemyId::Besieger => "Belagerer",
            EnemyId::Carrier => "Trägerschiff",
            EnemyId::ScrapKing => "Schrottkönig",
            EnemyId::Warden => "Wächter",
            EnemyId::Seal => "Siegel",
        }
    }

    fn enemy_class(&self, class: EnemyClass) -> &'static str {
        match class {
            EnemyClass::Fighter => "Jäger-Staffel",
            EnemyClass::Cruiser => "Kreuzer-Paar",
            EnemyClass::Battleship => "Schlachtschiff",
            EnemyClass::Flagship => "Flaggschiff",
        }
    }

    fn action(&self, action: Action) -> String {
        match action {
            Action::Fire => "Zielfeuer".to_owned(),
            Action::ColumnFire { columns } => {
                let names: Vec<&str> = (0..columns.len())
                    .filter(|&column| columns[column])
                    .map(|column| self.column_name(column))
                    .collect();
                format!("Spaltenfeuer {}", names.join("+"))
            }
            Action::Ram => "Rammkurs".to_owned(),
            Action::Shields(amount) => format!("Schilde {amount}"),
            Action::Evade => "Ausweichen".to_owned(),
            Action::Charge => "Aufladen".to_owned(),
            Action::TargetLock => "Zielerfassung".to_owned(),
            Action::Call(enemy) => format!("Ruft {}", self.enemy(enemy)),
            Action::Bolt => format!("Riegel: alle Schild {BOLT_SHIELD}"),
        }
    }

    fn condition(&self, condition: Condition) -> String {
        match condition {
            Condition::Burning => "Brand".to_owned(),
            Condition::Emergency => "Notfall".to_owned(),
            Condition::Stasis => "Stillstand".to_owned(),
            Condition::Silenced => "Funkstille".to_owned(),
            Condition::Covered => "Deckung".to_owned(),
        }
    }

    fn action_hint(&self, action: Action) -> String {
        match action {
            Action::Fire => "Feuert auf dein Flaggschiff, wo immer es steht.".to_owned(),
            Action::ColumnFire { .. } => {
                "Feuert auf die angesagten Spalten; ein Manöver heraus weicht aus.".to_owned()
            }
            Action::Ram => "Rammt dein Flaggschiff und vergeht dabei.".to_owned(),
            Action::Shields(amount) => format!("Erhält {amount} Schild."),
            Action::Evade => "Weicht aus: Deine Schüsse richten nur halben Schaden an.".to_owned(),
            Action::Charge => {
                "Lädt das Hauptgeschütz auf: Sein nächster Schuss richtet doppelten Schaden an."
                    .to_owned()
            }
            Action::TargetLock => "Alle gegnerischen Schüsse +1 ATK, solange er steht.".to_owned(),
            Action::Call(enemy) => {
                format!(
                    "Ruft {} als Verstärkung für die nächste Runde.",
                    self.enemy(enemy)
                )
            }
            Action::Bolt => format!("Alle ihre Einheiten erhalten {BOLT_SHIELD} Schild."),
        }
    }

    fn condition_hint(&self, condition: Condition) -> &'static str {
        match condition {
            Condition::Burning => "Verliert in jeder Auflösung Hülle.",
            Condition::Emergency => {
                "Hülle bei 25 % oder weniger: halbe ATK, kein Ausweichen; dein Flaggschiff verliert die Admiralsstation."
            }
            Condition::Stasis => "Handelt in dieser Runde nicht.",
            Condition::Silenced => "Feuert in dieser Runde nicht und setzt seine Sonderaktion aus.",
            Condition::Covered => {
                "Gedeckt: Der Vordermann in der Spalte fängt die Hälfte jedes Schadens ab (aufgerundet); durchschlagend übergeht das."
            }
        }
    }

    fn armor_hint(&self, amount: i32) -> String {
        format!("Panzerung: Jeder Treffer macht {amount} weniger Schaden.")
    }

    fn burn_hint(&self, amount: i32) -> String {
        if amount > 0 {
            format!("Verliert {amount} Hülle in jeder Auflösung.")
        } else {
            "Brandschutz: Der Brand tickt nicht.".to_owned()
        }
    }

    fn caption_next(&self) -> &'static str {
        "Nächste Runde"
    }

    fn module(&self, module: ModuleId) -> (&'static str, &'static str) {
        match module {
            ModuleId::FireControlComputer => (
                "Feuerleitrechner",
                "+1 ATK auf die Schüsse des Flaggschiffs.",
            ),
            ModuleId::FireSuppression => ("Löschanlage", "Der eigene Brand tickt nicht."),
            ModuleId::ReserveReactor => ("Reservereaktor", "4 Befehlspunkte in Runde 1."),
            ModuleId::ArmorPlates => ("Panzerplatten", "Panzerung +1."),
            ModuleId::EmergencyGenerator => (
                "Notaggregat",
                "Einmal im Run lässt ein tödlicher Schaden 1 Hülle.",
            ),
            ModuleId::SalvageCrane => (
                "Bergungskran",
                "Jede zerstörte gegnerische Einheit bringt 5 Schrott.",
            ),
            ModuleId::SupplyPod => ("Nachschubkapsel", "6 statt 5 Karten je Runde."),
            ModuleId::CombatSimulator => {
                ("Kampfsimulator", "Kartenangebote zeigen vier statt drei.")
            }
            ModuleId::ReinforcedFrames => ("Verstärkte Spanten", "+10 Hülle dauerhaft."),
        }
    }

    fn breakdown_words(&self) -> [&'static str; 6] {
        [
            "weicht aus",
            "Gegenmaßnahmen",
            "Panzerung",
            "gedeckt",
            "Schild",
            "tödlich",
        ]
    }

    fn log_battle_started(&self, admiral: &str) -> String {
        format!("Gefechtsstationen — {admiral} übernimmt das Kommando.")
    }

    fn log_card(&self, card: &str, target: Option<&str>) -> String {
        match target {
            Some(target) => format!("Karte: {card} auf {target}"),
            None => format!("Karte: {card}"),
        }
    }

    fn log_station(&self, station: &str) -> String {
        format!("Station: {station}")
    }

    fn log_hit(&self, source: &str, target: &str, breakdown: &str) -> String {
        format!("{source} trifft {target}: {breakdown}")
    }

    fn log_missed(&self, source: &str, reason: MissReason) -> String {
        match reason {
            MissReason::Jumped => {
                format!("{source} feuert ins Leere — das Flaggschiff ist gesprungen.")
            }
            MissReason::Maneuvered => format!("{source} feuert auf eine leere Spalte."),
        }
    }

    fn log_shielded(&self, target: &str, amount: i32) -> String {
        format!("{target} fährt Schilde hoch: {amount}")
    }

    fn log_healed(&self, target: &str, amount: i32) -> String {
        format!("{target} repariert {amount} Hülle")
    }

    fn log_ignited(&self, target: &str) -> String {
        format!("{target} brennt")
    }

    fn log_extinguished(&self, target: &str) -> String {
        format!("Brand auf {target} gelöscht")
    }

    fn log_emergency(&self, target: &str, active: bool) -> String {
        if active {
            format!("{target} im Notfall")
        } else {
            format!("{target} aus dem Notfall")
        }
    }

    fn log_destroyed(&self, target: &str, cause: DestroyCause) -> String {
        match cause {
            DestroyCause::Shot => format!("{target} zerstört"),
            DestroyCause::BurnedOut => format!("{target} ausgebrannt"),
            DestroyCause::Rammed => format!("{target} zerschellt am Rammstoß"),
            DestroyCause::Retaliation => format!("{target} am Gegenfeuer zerschellt"),
        }
    }

    fn log_fireship_spent(&self, unit: &str) -> String {
        format!("{unit} ist verbraucht")
    }

    fn log_rammed(&self, target: &str) -> String {
        format!("Das Flaggschiff rammt {target}")
    }

    fn log_jumped(&self) -> &'static str {
        "Das Flaggschiff springt"
    }

    fn log_returned(&self) -> &'static str {
        "Das Flaggschiff ist zurück"
    }

    fn log_maneuvered(&self, from: &str, to: &str) -> String {
        format!("Das Flaggschiff wechselt von {from} nach {to}")
    }

    fn log_tractored(&self, unit: &str, from: &str) -> String {
        format!("Traktorstrahl zieht {unit} aus {from}")
    }

    fn log_arrived(&self, unit: &str) -> String {
        format!("{unit} trifft ein")
    }

    fn log_enemy_action(&self, unit: &str, action: &str) -> String {
        format!("{unit}: {action}")
    }

    fn log_idle(&self, unit: &str, reason: IdleReason) -> String {
        match reason {
            IdleReason::Stasis => format!("{unit} steht still"),
            IdleReason::Silenced => format!("{unit} hält Funkstille"),
        }
    }

    fn log_counterfire(&self, target: &str, amount: i32) -> String {
        format!("Gegenfeuer trifft {target}: {amount}")
    }

    fn log_hold_the_line(&self, target: &str, amount: i32) -> String {
        format!("Standhalten trifft {target}: {amount}")
    }

    fn log_self_damage(&self, amount: i32) -> String {
        format!("Überladung belastet das Flaggschiff: {amount}")
    }

    fn log_burn(&self, target: &str, amount: i32) -> String {
        format!("{target} brennt: {amount}")
    }

    fn log_cannibalized(&self, unit: &str, amount: i32) -> String {
        format!("{unit} schlachtet das Wrack aus: +{amount} Hülle")
    }

    fn log_emergency_generator(&self) -> &'static str {
        "Das Notaggregat hält das Flaggschiff bei 1 Hülle"
    }

    fn log_shuffled(&self) -> &'static str {
        "Ablagestapel ins Deck gemischt"
    }

    fn log_drawn(&self, count: usize) -> String {
        match count {
            1 => "1 Karte gezogen".to_owned(),
            _ => format!("{count} Karten gezogen"),
        }
    }

    fn log_intents(&self, unit: &str, intents: &str) -> String {
        format!("{unit} sagt an: {intents}")
    }

    fn log_clock(&self, bonus: i32) -> String {
        format!("Gefechtsuhr: Gegner +{bonus} ATK")
    }

    fn log_battle_ended(&self, result: BattleResult) -> &'static str {
        match result {
            BattleResult::Won => "Die feindliche Flotte ist vernichtet.",
            BattleResult::Lost => "Das Flaggschiff ist verloren.",
        }
    }

    // --- Run ---

    fn menu_tagline(&self) -> &'static str {
        "Ein Wurmloch hat dein Flaggschiff verschluckt. Drei Sektoren trennen dich von zu Hause."
    }

    fn menu_new_run(&self) -> &'static str {
        "Neuer Run"
    }

    fn menu_continue(&self) -> &'static str {
        "Weiter"
    }

    fn menu_saved_run(&self, admiral: &str, sector: u32, row: usize) -> String {
        format!("{admiral} · Sektor {sector}, Reihe {row}")
    }

    fn sector_name(&self, sector: u32) -> &'static str {
        match sector {
            1 => "Das Trümmerfeld",
            2 => "Die Stille",
            _ => "Die Schwelle",
        }
    }

    fn map_title(&self, sector: u32) -> String {
        format!("Sektor {sector}: {}", self.sector_name(sector))
    }

    fn map_hint(&self) -> &'static str {
        "Wähle den nächsten Knoten. Oben wartet der Boss des Sektors."
    }

    fn sector_banner_hint(&self) -> &'static str {
        "Der Weg nach Hause führt weiter."
    }

    fn node_kind(&self, kind: NodeKind) -> &'static str {
        match kind {
            NodeKind::Battle => "Gefecht",
            NodeKind::Elite => "Elite",
            NodeKind::Event => "Ereignis",
            NodeKind::Shipyard => "Werft",
            NodeKind::Market => "Schwarzmarkt",
            NodeKind::Boss => "Boss",
        }
    }

    fn node_kind_hint(&self, kind: NodeKind) -> &'static str {
        match kind {
            NodeKind::Battle => "Ein Verband des Sektors. Beute: Schrott und eine Befehlskarte.",
            NodeKind::Elite => "Ein mächtiges Schlachtschiff. Beute: viel Schrott und ein Modul.",
            NodeKind::Event => "Eine Szene über Funk mit einer Entscheidung.",
            NodeKind::Shipyard => "Reparieren, eine Karte aufwerten oder eine ausmustern.",
            NodeKind::Market => "Karten, Module, Reparatur und Ausmustern für Schrott.",
            NodeKind::Boss => "Der Abschluss des Sektors. Beute: ein Modul und Reparatur.",
        }
    }

    fn run_scrap(&self, scrap: u32) -> String {
        format!("Schrott {scrap}")
    }

    fn run_deck(&self, cards: usize) -> String {
        format!("Deck {cards}")
    }

    fn run_show_deck(&self) -> &'static str {
        "Deck ansehen"
    }

    fn run_modules(&self, count: usize) -> String {
        format!("Module {count}")
    }

    fn run_no_modules(&self) -> &'static str {
        "Keine Module"
    }

    fn run_position(&self, sector: u32, row: usize) -> String {
        format!("Sektor {sector} · Reihe {row}")
    }

    fn run_scrap_change(&self, scrap: i64) -> String {
        format!("{} Schrott", signed(scrap))
    }

    fn run_hull_change(&self, hull: i32) -> String {
        format!("{} Hülle", signed(i64::from(hull)))
    }

    fn run_module_gained(&self, module: &str) -> String {
        format!("Neues Modul: {module}")
    }

    fn run_card_upgraded(&self, card: &str) -> String {
        format!("Aufgewertet: {card}")
    }

    fn loot_title(&self) -> &'static str {
        "Beute"
    }

    fn loot_cards_hint(&self) -> &'static str {
        "Eine Befehlskarte kommt ins Deck, oder keine."
    }

    fn loot_modules_hint(&self) -> &'static str {
        "Ein Modul wirkt bis zum Ende des Runs."
    }

    fn loot_take(&self) -> &'static str {
        "Nehmen"
    }

    fn loot_skip(&self) -> &'static str {
        "Keine"
    }

    fn event_title(&self, event: EventId) -> &'static str {
        match event {
            EventId::Resolute => "Das Wrack der Resolute",
            EventId::Pilot => "Der Lotse",
            EventId::Flicker => "Flackern",
        }
    }

    fn event_text(&self, event: EventId) -> &'static str {
        match event {
            EventId::Resolute => {
                "Ein Kreuzer der eigenen Flotte treibt ausgebrannt im Feld: die Resolute. Das Wurmloch hat auch sie \
                 verschluckt. Ihr Reaktor glimmt noch, ihre Funkboje sendet in einer Schleife die letzte Meldung."
            }
            EventId::Pilot => {
                "Ein Händler meldet sich auf einer offenen Frequenz. Vor dir liegt ein Minenfeld, und er kennt den \
                 sicheren Weg hindurch, für einen Preis."
            }
            EventId::Flicker => {
                "Das Wurmloch meldet sich. Für einen Augenblick flackert der Raum um das Flaggschiff, und zwischen dem \
                 Rauschen liegt etwas Vertrautes."
            }
        }
    }

    fn event_option(&self, option: EventOption) -> &'static str {
        match option {
            EventOption::Salvage => "Bergen",
            EventOption::Logbooks => "Die Logbücher sichern",
            EventOption::MoveOn => "Weiterfliegen und die Crew bestatten",
            EventOption::PayPilot => "Den Lotsen bezahlen",
            EventOption::FlyYourself => "Selbst fliegen",
            EventOption::Embrace => "Ins Flackern fliegen",
            EventOption::Listen => "Zuhören",
        }
    }

    fn event_consequence(&self, option: EventOption, flicker_damage: i32) -> String {
        use voidfleet_core::run::{PILOT_DAMAGE, PILOT_PRICE, RESOLUTE_SCRAP};
        match option {
            EventOption::Salvage => {
                format!("+{RESOLUTE_SCRAP} Schrott; das nächste Gefecht beginnt brennend.")
            }
            EventOption::Logbooks => "Ein zufälliges Modul.".to_owned(),
            EventOption::MoveOn => "Nichts.".to_owned(),
            EventOption::PayPilot => {
                format!("{PILOT_PRICE} Schrott; der nächste Knoten wird eine Werft.")
            }
            EventOption::FlyYourself => format!("−{PILOT_DAMAGE} Hülle, dafür ein Kartenangebot."),
            EventOption::Embrace => {
                format!("−{flicker_damage} Hülle; eine zufällige Karte wird aufgewertet.")
            }
            EventOption::Listen => "Nichts geschieht.".to_owned(),
        }
    }

    fn event_outcome(&self, option: EventOption) -> &'static str {
        match option {
            EventOption::Salvage => {
                "Die Bergungstrupps schneiden heraus, was sich tragen lässt. Beim Ablegen zündet ein Leck im Hangar."
            }
            EventOption::Logbooks => {
                "In den Logbüchern stecken die Baupläne, die die Resolute nicht mehr braucht."
            }
            EventOption::MoveOn => {
                "Die Crew bekommt ihr Begräbnis im All. Über Funk die letzte Meldung der Resolute: „Wir halten die \
                 Linie.“"
            }
            EventOption::PayPilot => "Der Händler überträgt seine Route. Am Ende wartet ein Dock.",
            EventOption::FlyYourself => {
                "Zwei Minen streifen die Hülle. Im Feld treiben Container mit Befehlssätzen."
            }
            EventOption::Embrace => "Das Flackern brennt durch die Systeme und lässt etwas zurück.",
            EventOption::Listen => {
                "Zum ersten Mal seit dem Sprung: der heimische Funk. Dann wieder Rauschen."
            }
        }
    }

    fn shipyard_title(&self) -> &'static str {
        "Werft"
    }

    fn shipyard_hint(&self) -> &'static str {
        "Ein Auftrag, dann geht es weiter."
    }

    fn shipyard_repair(&self) -> &'static str {
        "Reparieren"
    }

    fn shipyard_repair_text(&self, hull: i32) -> String {
        format!("+{hull} Hülle.")
    }

    fn shipyard_upgrade(&self) -> &'static str {
        "Aufwerten"
    }

    fn shipyard_upgrade_text(&self) -> &'static str {
        "Eine Karte des Decks wird stärker."
    }

    fn shipyard_remove(&self) -> &'static str {
        "Ausmustern"
    }

    fn shipyard_remove_text(&self) -> &'static str {
        "Eine Karte verlässt das Deck."
    }

    fn gallery_deck(&self) -> &'static str {
        "Deck"
    }

    fn gallery_upgrade_hint(&self) -> &'static str {
        "Wähle eine Karte, die noch nicht aufgewertet ist."
    }

    fn gallery_remove_hint(&self) -> &'static str {
        "Wähle die Karte, die das Deck verlässt."
    }

    fn gallery_back(&self) -> &'static str {
        "Zurück"
    }

    fn market_title(&self) -> &'static str {
        "Schwarzmarkt"
    }

    fn market_hint(&self) -> &'static str {
        "Bezahlt wird in Schrott. Fragen werden keine gestellt."
    }

    fn market_cards(&self) -> &'static str {
        "Befehlskarten"
    }

    fn market_modules(&self) -> &'static str {
        "Module"
    }

    fn market_services(&self) -> &'static str {
        "Dienste"
    }

    fn market_buy(&self, price: u32) -> String {
        format!("Kaufen · {price}")
    }

    fn market_removal(&self) -> &'static str {
        "Karte entfernen"
    }

    fn market_repair(&self, hull: i32) -> String {
        format!("{hull} Hülle reparieren")
    }

    fn market_sold_out(&self) -> &'static str {
        "Verkauft"
    }

    fn market_leave(&self) -> &'static str {
        "Verlassen"
    }

    fn run_won_title(&self) -> &'static str {
        "Das Wurmloch nach Hause"
    }

    fn run_won_text(&self) -> &'static str {
        "Kommandantin Vey ist gefallen, das Wurmloch steht offen. Auf der anderen Seite: die eigene Flotte."
    }

    fn run_lost_text(&self, sector: u32, row: usize) -> String {
        format!("Das Flaggschiff fiel in Sektor {sector}, Reihe {row}.")
    }

    fn run_abandon(&self) -> &'static str {
        "Run abbrechen"
    }

    fn run_abandon_title(&self) -> &'static str {
        "Run abbrechen?"
    }

    fn run_abandon_text(&self) -> &'static str {
        "Der Speicherstand wird gelöscht. Das lässt sich nicht rückgängig machen."
    }

    fn run_abandon_confirm(&self) -> &'static str {
        "Abbrechen"
    }

    fn run_abandon_back(&self) -> &'static str {
        "Weiterfliegen"
    }
    // --- Gefechtsbericht ---

    fn mark_evaded(&self, amount: i32) -> String {
        format!("weicht aus: {amount}")
    }

    fn mark_countermeasures(&self, amount: i32) -> String {
        format!("Gegenmaßnahmen −{amount}")
    }

    fn mark_armor(&self, amount: i32) -> String {
        format!("Panzerung −{amount}")
    }

    fn mark_covered(&self, amount: i32, by: &str) -> String {
        format!("gedeckt: {amount} → {by}")
    }

    fn mark_shielded(&self, amount: i32) -> String {
        format!("Schild schluckt {amount}")
    }

    fn report_missed(&self) -> &'static str {
        "ins Leere"
    }

    fn report_burned_out(&self, unit: &str) -> String {
        format!("{unit} ausgebrannt")
    }
}
