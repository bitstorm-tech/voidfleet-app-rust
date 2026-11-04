//! Deutsch. Die Funksprüche sind hier nur Untertitel; gesprochen wird englisch.

use std::borrow::Cow;

use voidfleet_core::{
    admirals::{ABILITY_VALUES as V, AdmiralId, PowerId, PowerTarget},
    board::{LaneType, Side},
    cards::{CommandId, CommandType, ShipClass, ShipId, TargetMode},
    protocol::{ErrorCode, MatchResult},
};

use super::{
    AdmiralText, CardText, Messages, Phase, PhaseText, RulesChapter, RulesSection, SettingsTab,
    amount_notes, float_signed, signed, stats,
};
use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, CursorId, WindowMode},
};

pub struct De;

fn section(
    heading: &'static str,
    paragraphs: Vec<Cow<'static, str>>,
    caption: Option<&'static str>,
) -> RulesSection {
    RulesSection {
        heading,
        paragraphs,
        caption,
    }
}

impl Messages for De {
    fn language_name(&self) -> &'static str {
        "Deutsch"
    }

    // --- hud ---
    fn hud_round(&self, round: u32) -> String {
        format!("Runde {round}")
    }
    fn hud_new_round(&self) -> &'static str {
        "Neue Runde"
    }
    fn hud_end_turn(&self) -> &'static str {
        "Zug beenden"
    }
    fn hud_waiting(&self) -> &'static str {
        "Warte auf Gegner"
    }
    fn hud_skip(&self) -> &'static str {
        "Animation überspringen"
    }
    fn hud_play_computer(&self) -> &'static str {
        "Gegen Computer spielen"
    }
    fn hud_invite_player(&self) -> &'static str {
        "Mitspieler einladen"
    }
    fn hud_show_log(&self) -> &'static str {
        "Log einblenden"
    }
    fn hud_hide_log(&self) -> &'static str {
        "Log ausblenden"
    }
    fn hud_overdrive(&self, bonus: i32) -> String {
        format!("Overdrive +{bonus}")
    }
    fn hud_overdrive_hint(&self, bonus: i32) -> String {
        format!("Overdrive: alle Kampfschiffe +{bonus} ATK")
    }
    fn hud_checklist_flagship(&self) -> &'static str {
        "Flaggschiff legen"
    }
    fn hud_checklist_ships(&self, planned: usize, max: usize) -> String {
        if max > 1 {
            format!("Schiffe legen ({planned}/{max})")
        } else {
            "Schiff legen".into()
        }
    }
    fn hud_checklist_command(&self) -> &'static str {
        "Befehl spielen"
    }
    fn hud_checklist_maneuver(&self) -> &'static str {
        "Grundmanöver oder Fähigkeit"
    }
    fn hud_checklist_maneuver_only(&self) -> &'static str {
        "Grundmanöver"
    }
    fn hud_checklist_power(&self) -> &'static str {
        "Admiralsfähigkeit"
    }
    fn hud_checklist_fire(&self, assigned: usize, shooters: usize) -> String {
        if shooters > 0 {
            format!("Ziele zuweisen ({assigned}/{shooters})")
        } else {
            "Ziele zuweisen".into()
        }
    }
    fn hud_checklist_mark(&self) -> &'static str {
        "Zielerfassung"
    }
    fn hud_power(&self) -> &'static str {
        "Admiralsfähigkeit"
    }
    fn hud_power_hint(&self) -> &'static str {
        "statt des Grundmanövers"
    }
    fn hud_mark_button(&self) -> &'static str {
        "Zielerfassung"
    }
    fn hud_mark_hint(&self) -> &'static str {
        "Pendulum: ein gegnerisches Schiff erfassen. In der nächsten Runde trifft es die Railgun zu Kampfbeginn, wenn es ungedeckt ist."
    }
    fn hud_marked(&self, ship: &str) -> String {
        format!("Erfasst: {ship}")
    }
    fn hud_railgun(&self, ship: &str) -> String {
        format!("Railgun → {ship}")
    }
    fn hud_railgun_hint(&self, amount: i32) -> String {
        format!(
            "Die Railgun trifft das erfasste Schiff zu Kampfbeginn mit {amount}, wenn es ungedeckt ist."
        )
    }
    fn hud_shot_lost(&self) -> &'static str {
        "Die Pendulum bewegt sich — der Schuss fällt aus."
    }
    fn hud_lock_lost(&self) -> &'static str {
        "Die Pendulum bewegt sich — die Zielerfassung verfällt."
    }
    fn hud_under_fire(&self, ship: &str) -> String {
        format!("Gegnerische Railgun auf {ship}")
    }

    // --- admiralSelect ---
    fn admiral_select_title(&self) -> &'static str {
        "Wähle deinen Admiral"
    }
    fn admiral_select_hint(&self) -> &'static str {
        "Deine Wahl bleibt verdeckt, bis beide gewählt haben. Die Admiralsfähigkeit ersetzt jede Runde das Grundmanöver. Fällt dein Flaggschiff, hast du verloren."
    }
    fn admiral_select_flagship(&self) -> &'static str {
        "Flaggschiff"
    }
    fn admiral_select_power(&self) -> &'static str {
        "Admiralsfähigkeit"
    }
    fn admiral_select_choose(&self) -> &'static str {
        "Wählen"
    }

    // --- draft ---
    fn draft_title(&self) -> &'static str {
        "Nachschub"
    }
    fn draft_hint(&self) -> &'static str {
        "Nimm ein Schiff und einen Befehl auf die Hand; der Rest geht zurück ins Deck."
    }
    fn draft_ships(&self) -> &'static str {
        "Schiffe"
    }
    fn draft_commands(&self) -> &'static str {
        "Befehle"
    }
    fn draft_none(&self) -> &'static str {
        "Nichts zur Wahl — Hand voll oder Deck leer."
    }
    fn draft_confirm(&self) -> &'static str {
        "Übernehmen"
    }
    fn draft_peek(&self) -> &'static str {
        "Schlachtfeld ansehen"
    }
    fn draft_show(&self) -> &'static str {
        "Nachschub zeigen"
    }

    // --- settings ---
    fn settings_title(&self) -> &'static str {
        "Einstellungen"
    }
    fn settings_tab(&self, tab: SettingsTab) -> &'static str {
        match tab {
            SettingsTab::Game => "Spiel",
            SettingsTab::Video => "Video",
            SettingsTab::Sound => "Sound",
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
    fn settings_install(&self) -> &'static str {
        "App"
    }
    fn settings_install_button(&self) -> &'static str {
        "Voidfleet als App installieren"
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

    // --- rules ---
    fn rules_title(&self) -> &'static str {
        "Spielregeln"
    }
    fn rules_section(&self, chapter: RulesChapter) -> RulesSection {
        match chapter {
            RulesChapter::Goal => section(
                "Ziel des Spiels",
                vec![
                    "Voidfleet ist ein Duell zweier Flotten, jede geführt von einem Admiral und seinem Flaggschiff. Du gewinnst, indem du das gegnerische Flaggschiff zerstörst: Wer sein Flaggschiff verliert, verliert das Match, und zwar sofort: Der Rest der Runde entfällt. Fallen beide Flaggschiffe gleichzeitig, etwa im selben Kampf, endet es unentschieden.".into(),
                    "Vor Runde 1 wählt jeder verdeckt einen von fünf Admiralen. Der Admiral bringt ein Flaggschiff mit — ein Schiff außerhalb der Decks mit eigener Fähigkeit — und eine Admiralsfähigkeit, die du jede Runde statt des Grundmanövers einsetzen kannst (siehe „Admirale und Flaggschiffe“).".into(),
                    "Beide Spieler planen ihren Zug gleichzeitig, ohne die Wahl des anderen zu sehen, und ohne Zeitlimit: Ein Zug darf beliebig lange dauern. Es gibt keine Energie und kein Mana — jede Karte auf deiner Hand ist sofort spielbar.".into(),
                    "Damit sich keine Partie endlos hinzieht, beginnt ab Runde 8 der Overdrive: Alle Kampfschiffe beider Seiten bekommen +1 ATK, und der Bonus wächst alle 4 Runden um eins (Runde 12, 16, …). Dagegen kommen Reparaturen auf Dauer nicht an.".into(),
                ],
                Some("Der Overdrive gibt allen Kampfschiffen beider Seiten Angriff dazu, je länger das Match dauert."),
            ),
            RulesChapter::Turn => section(
                "Dein Zug",
                vec![
                    "Jede Runde planst du bis zu drei Dinge und klickst dann auf „Zug beenden“: ein Schiff aus deiner Hand auf einen freien Slot deiner Seite, einen Befehl aus deiner Hand und entweder ein Manöver eines Schiffs, das schon auf dem Brett steht, in einen anderen freien Slot oder deine Admiralsfähigkeit. Alle drei sind freiwillig — du kannst auch mit leerem Zug passen. Dazu gibst du deinen Kampfschiffen ihre Feuerbefehle: worauf jedes von ihnen im Kampf feuert, eine gegnerische Spalte oder ein bestimmtes gegnerisches Schiff (siehe „Kampf“). Admiral Kerr darf außerdem mit der Pendulum ein Ziel erfassen.".into(),
                    "In Runde 1 stellst du deine Flotte auf: dein Flaggschiff, dazu bis zu zwei Schiffe statt einem und ein Befehl. Das Flaggschiff ist Pflicht — „Zug beenden“ geht erst, wenn es eingeplant ist. Ab Runde 2 ist es ein Schiff pro Runde. Zwei Befehle verändern den Zug selbst, solange sie eingeplant sind: Mit Schnellverlegung legst du ein Schiff mehr, mit Freier Hand bekommst du Grundmanöver und Admiralsfähigkeit zusammen. Nimmst du den Befehl zurück, fällt das zusätzliche Schiff bzw. die Fähigkeit wieder weg.".into(),
                    "Ein Befehl darf auch ein Schiff treffen, das du in derselben Runde legst — auch dein Flaggschiff —, etwa um es sofort zu überladen oder abzuschirmen. Geplante Schiffe stehen als durchscheinende Geister auf dem Brett; ein Befehl auf einen Geist wird so aufgelöst, als stünde das Schiff schon dort, und auch ein Geist kann einen Feuerbefehl bekommen.".into(),
                    "Nichts ist endgültig, bis du den Zug beendest: Alles, was du geplant hast, kannst du zurücknehmen oder ändern (siehe „Bedienung“). Dein Gegner sieht deinen Zug erst, wenn beide abgegeben haben.".into(),
                ],
                Some("Schiff legen: Karte antippen, dann einen der leuchtenden Slots auf deiner Seite."),
            ),
            RulesChapter::Admirals => section(
                "Admirale und Flaggschiffe",
                vec![
                    "Vor Runde 1 wählen beide Spieler gleichzeitig und verdeckt ihren Admiral; die Wahl des Gegners siehst du erst, wenn beide gewählt haben, dann beginnt Runde 1. Jeder Admiral steht für einen Spielstil und bringt ein Flaggschiff und eine Admiralsfähigkeit mit.".into(),
                    "Das Flaggschiff liegt in keinem Deck. In Runde 1 legst du es zusätzlich zu deinen bis zu zwei Schiffen, nach den normalen Platzregeln; es zählt zur Lane- und Flottengrenze. Sonst ist es ein normales Kampfschiff: Es feuert auf das Ziel, das du ihm gibst, manövriert und kann deine Befehle bekommen. Wird es zerstört, hast du verloren. Auf dem Brett heben sich Flaggschiffe ab: etwas größer, mit einem Ring in der Farbe ihres Admirals darunter und einer Krone neben dem Hüllenbalken.".into(),
                    "Die Admiralsfähigkeit steht dir jede Runde zur Verfügung, aber sie tritt an die Stelle des Grundmanövers: In jedem Zug manövrierst du entweder ein Schiff oder setzt die Fähigkeit ein, nie beides — wer das eine plant, verliert das andere. Nur der Befehl Freie Hand erlaubt beides im selben Zug. Tippe die Fähigkeitskarte neben deinen Befehlen an, dann eines der leuchtenden Ziele; nochmal auf die Karte oder Escape nimmt sie zurück. Aufgedeckt werden die Fähigkeiten gleich nach den Deploys. Ihre Marke folgt einem Schiff, das sich bewegt; eine Fähigkeit auf ein eigenes Schiff darf auch ein Schiff treffen, das du in diesem Zug legst.".into(),
                ],
                None,
            ),
            RulesChapter::Roster => section(
                "Die fünf Admirale",
                vec![
                    format!("Admiral Osei, die Mauer, führt die Bastion (Schlachtschiff, {}). Wächterschild: Nach den Manövern erhält die Bastion +{} Schild für die Runde, das Schiff im Strike-Slot direkt vor ihr +{}. Gegenfeuer: Wähle ein eigenes Schiff; jedes gegnerische Schiff außer einem Flaggschiff, das es im Kampf trifft, nimmt danach je Treffer {} Schaden.", stats(ShipId::Bastion), V.guardian_shield, V.guardian_shield_front, V.counterfire).into(),
                    format!("Admiralin Vance, der Hammer, führt die Vanguard (Kreuzer, {}). Speerspitze: Das Kampfschiff im Strike-Slot direkt vor ihr hat +{} ATK. Fokusfeuer: Wähle ein gegnerisches Schiff, auch ein gedecktes; in dieser Runde macht jeder Treffer darauf im Kampf +{} Schaden. Es lenkt keinen Schuss — wer darauf feuert, bestimmen deine Feuerbefehle.", stats(ShipId::Vanguard), V.spearhead, V.focus_fire).into(),
                    format!("Admiralin Halvorsen, der Schwarm, führt die Hive (Schlachtschiff, {}). Brutkammer: Am Ende jeder Runde startet die Hive eine Drohne in den nächsten freien Strike-Slot — höchstens {} deiner Drohnen gleichzeitig und {} pro Match. Der Schwarmangriff braucht eine eigene Drohne auf dem Brett: Wähle ein gegnerisches Schiff außer dem Flaggschiff, gedeckt oder nicht; alle deine Drohnen feuern statt auf ihre eigenen Ziele darauf — durch Deckung hindurch, ohne Feuerbefehl, auch frisch gestartete —, jeder Treffer mit +{} Schaden.", stats(ShipId::Hive), V.max_drones, V.hive_budget, V.swarm_strike).into(),
                    format!("Admiral Moreau, das Phantom, führt die Mirage (Jäger, {}). Phasentausch: Das Grundmanöver der Mirage darf auch auf einen belegten eigenen Slot gehen — dann tauschen beide Schiffe die Plätze und feuern beide. Trugbild: Wähle einen freien eigenen Strike-Slot; dort steht für diese Runde ein Trugbild (1 HP). Es greift nie an, ist für den Gegner aber ein Schiff vorn in seiner Spalte: Spaltenfeuer trifft zuerst das Trugbild, und Schiffsfeuer auf das Schiff dahinter verpufft, solange das Trugbild steht. Flaggschiffe durchschauen es: Gegen sie deckt es nicht, und ihr Spaltenfeuer geht daran vorbei auf das Schiff dahinter — nur wenn dort keins steht, treffen sie das Trugbild. Am Rundenende löst es sich auf.", stats(ShipId::Mirage)).into(),
                    format!("Admiral Kerr, der Uhrmacher, führt die Pendulum (Schlachtschiff, {}). Railgun: In einer Runde, in der ihre Railgun nicht feuert, darf Kerr ein gegnerisches Schiff erfassen. In der nächsten Runde trifft es die Railgun zu Kampfbeginn mit {} Schaden, wenn es ungedeckt ist; ist es gedeckt, ist der Schuss blockiert. Bewegt sich die Pendulum in einer der beiden Runden, fällt der Schuss aus. Erfasste Schiffe tragen auf beiden Seiten ein Fadenkreuz. Taktvorsprung: Wähle ein eigenes Kampfschiff; es feuert vor allen anderen, und was es zerstört, feuert nicht mehr.", stats(ShipId::Pendulum), V.railgun).into(),
                ],
                Some("Die Admirale mit Flaggschiff, seiner Fähigkeit und der Admiralsfähigkeit."),
            ),
            RulesChapter::Board => section(
                "Das Brett",
                vec![
                    "Jede Seite hat zwei Lanes mit je 6 Slots: vorne, dem Gegner zugewandt, die Strike-Lane, dahinter die Command-Lane. In eine Lane passen höchstens 4 Schiffe, auf eine Seite höchstens 6 (Drohnen zählen bei beidem nicht mit); mindestens zwei Slots je Lane bleiben also frei.".into(),
                    "Die beiden Slots mit derselben Nummer bilden eine Spalte: vorne der Strike-Slot, dahinter der Command-Slot. Die Command-Lane bietet Deckung: Ein Schiff dort ist gedeckt, solange in seiner Spalte ein Schiff davor steht. Geschützfeuer erreicht ein gedecktes Schiff erst, wenn das Schiff davor gefallen ist (siehe „Kampf“). Ohne Schiff davor ist ein Command-Schiff genauso ungedeckt wie eines in der Strike-Lane.".into(),
                    "Die Deckung zählt im Moment jedes Schusses: Ist das Schiff davor zerstört — auch durch einen früheren Schuss desselben Kampfs —, steht das Schiff dahinter für alle folgenden Schüsse ungedeckt. Schaden durch Befehle ignoriert die Deckung: Orbitalschlag, Torpedo, Wärmesucher, Bombardement und alle anderen Schadensbefehle treffen auch gedeckte Schiffe. Nur der Rammkurs braucht ein ungedecktes Schiff gegenüber.".into(),
                    "Schiffe feuern in beiden Lanes mit voller Stärke; die Lane zählt nur für die Deckung, für die Reihenfolge der Schüsse im Kampf (erst die Strike-Lane), für Befehle auf eine bestimmte Lane (Energiestoß, Zielstörer, Abfangen, Rückruf, Rammkurs, Bombardement) und für den Signalverstärker, der die Schiffe seiner eigenen Lane verstärkt.".into(),
                ],
                Some("Jede Seite: Strike vorne, Command dahinter; Slots mit derselben Nummer bilden eine Spalte. Feuer in eine Spalte trifft zuerst das vordere Schiff: Das Command-Schiff hinter einem Strike-Schiff ist gedeckt, das ohne ist ungedeckt."),
            ),
            RulesChapter::Movement => section(
                "Schiffe bewegen",
                vec![
                    "Einmal pro Runde — sofern du nicht stattdessen deine Admiralsfähigkeit einsetzt — darfst du eines deiner Schiffe, das schon auf dem Brett steht, in einen beliebigen freien Slot deiner Seite manövrieren, in der anderen Lane oder in derselben: Schiff antippen, dann einen leuchtenden Slot. Innerhalb der Lane bringst du so etwa eine Reparaturdrohne hinter ein Strike-Schiff in Deckung. Der Preis: Ein manövrierendes Schiff feuert in dieser Runde nicht. Es kann weiterhin beschossen werden, deckt und ist gedeckt schon von seinem neuen Platz aus, und ein Befehl auf das Schiff folgt ihm in den neuen Slot. Beim gegnerischen Geschützfeuer kommt es darauf an, worauf es zielt: Schiffsfeuer folgt dem Schiff, Spaltenfeuer bleibt auf seiner Spalte — wer sie verlässt, weicht ihm aus, wer in eine Spalte hineinmanövriert, läuft hinein. Nur die Mirage darf auf einen belegten eigenen Slot manövrieren: Beide Schiffe tauschen die Plätze und feuern weiter.".into(),
                    "Auch Befehle verlegen Schiffe: Flankenmanöver wechselt ein Schiff in die andere Lane, Abfangen holt ein Command-Schiff mit +2 ATK nach vorne in die Strike-Lane, Rückruf zieht ein Strike-Schiff in die Command-Lane zurück und repariert 2 HP, Ausweichmanöver versetzt ein Schiff mit Schild 2 in einen freien Slot derselben Lane. Per Befehl verlegte Schiffe feuern weiter. Der Kurzsprung nimmt ein eigenes Schiff nach allen Bewegungen bis zum Rundenende aus dem Gefecht: Nichts trifft es — Schiffsfeuer darauf verpufft, für Spaltenfeuer ist sein Slot leer —, aber es feuert und deckt auch nicht. Der Traktorstrahl verlegt dagegen ein gegnerisches Schiff: Nach allen Bewegungen zieht er es in den nächsten freien Slot seiner anderen Lane.".into(),
                    "Das Ausweichmanöver bleibt in der Lane, die übrigen Bewegungsbefehle wechseln in die andere. Manöver und Bewegungsbefehl können nicht dasselbe Schiff verlegen und nicht im selben Slot landen. Der Slot, den ein Schiff verlässt, bleibt für den Rest der Runde blockiert — er wird erst in der nächsten Runde frei, auch für Schiffe, die du legst. Gelegte Schiffe und beide Ziele zählen zusammen zur Grenze von 4 Schiffen je Lane.".into(),
                    "Bewegung hat ein Risiko: Sperrfeuer macht 3 Schaden an jedem gegnerischen Schiff, das in dieser Runde seinen Slot gewechselt hat, egal wie, und der Wärmesucher folgt seinem Ziel und trifft ein Schiff, das den Slot gewechselt hat, mit 5 statt 3. Ein Torpedo zielt auf einen Slot, nicht auf ein Schiff — der Durchschlag auf eine Spalte: Er trifft, wer nach allen Bewegungen dort steht, ähnlich wie Spaltenfeuer — ihm auszuweichen oder hineinzulaufen gehört zum Spiel.".into(),
                ],
                Some("Ein geplantes Manöver: Der Schrott-Raider zieht sich aus der Strike-Lane in die Command-Lane zurück und feuert diese Runde nicht. Ein Manöver darf auch in der Lane bleiben."),
            ),
            RulesChapter::Resolution => section(
                "Die Rundenauflösung",
                vec![
                    "Sobald beide Spieler ihren Zug beendet haben, werden beide Züge aufgedeckt und in festen Phasen aufgelöst. Innerhalb einer Phase handeln beide Seiten gleichzeitig; keine Seite ist je zuerst dran.".into(),
                ],
                Some("Befehle wirken in der Effects-Phase nach Typ, nicht nach Spieler: erst Umgruppierung, dann Taktik, Verstärkung, Spezial, Schwächung und zuletzt Schaden."),
            ),
            RulesChapter::Combat => section(
                "Kampf",
                vec![
                    "Jede Runde gibst du deinen Kampfschiffen ihre Feuerbefehle: Für jedes Schiff mit Angriff über 0 wählst du, worauf es feuert, und zwar auf eine von zwei Arten. Spaltenfeuer zielt auf eine Spalte der Gegnerseite, jede der sechs, auch eine leere. Schiffsfeuer zielt auf ein bestimmtes gegnerisches Schiff, das beim Planen auf dem Brett steht, auch auf gedeckte und das gegnerische Flaggschiff. Ein Schiff ohne Feuerbefehl feuert nicht — es gibt keine automatische Zielwahl, und jede Runde beginnt ohne Feuerbefehle. Auch Schiffe, die du in dieser Runde legst, können einen Feuerbefehl bekommen; ein Schiff, das manövriert, feuert nicht (Ausnahme ist der Phasentausch der Mirage: Beide Schiffe feuern weiter), und eines, das der EMP-Stoß gelähmt hat, kann nicht feuern.".into(),
                    "Spaltenfeuer bleibt auf seiner Spalte: Der Schuss trifft das vorderste Schiff, das dort im Moment des Schusses noch steht — das Strike-Schiff oder, wenn der Strike-Slot leer oder sein Schiff in diesem Kampf schon gefallen ist, das Command-Schiff dahinter. Steht dort keins (mehr), verpufft der Schuss. Wer aus der Spalte manövriert, weicht ihm also aus, und wer hineinmanövriert oder dort gelegt wird, läuft hinein. Schiffsfeuer folgt seinem Schiff über Manöver, Befehle, Traktorstrahl und Phasentausch und trifft es, wo es gerade steht — es verpufft aber, wenn das Schiff im Moment des Schusses gedeckt, schon zerstört (der Schuss geht nie auf ein anderes Schiff über) oder nicht mehr auf dem Brett ist (vor dem Kampf zerstört, Kurzsprung).".into(),
                    "Innerhalb einer Seite fallen die Schüsse nacheinander: erst die Strike-Lane von Slot 1 bis 6, dann die Command-Lane. Die beiden Seiten feuern gleichzeitig; ein Schiff, das in diesem Kampf zerstört wird, feuert also trotzdem — die Reihenfolge zählt nur unter den Schüssen einer Seite. Weil die Deckung im Moment jedes Schusses zählt, kann Spaltenfeuer aus deiner Strike-Lane erst die Deckung brechen, und Schiffsfeuer aus deiner Command-Lane trifft danach das Schiff dahinter. Nichts verhindert Overkill: Beim Planen zeigt die Vorschau, welches Schiff jeder Schuss trifft und wie viel Schaden jedes gegnerische Schiff nimmt — verteile also dein Feuer.".into(),
                    "Weil sich auch eine leere Spalte beschießen lässt, fallen schon ab Runde 1 Schüsse: blind in die Spalten, in denen du die Landung des Gegners erwartest. Ein Schiff, das in dieser Runde landet, kann von Spaltenfeuer getroffen werden (und mit Ablenkung Schüsse auf sich ziehen), aber nicht von Schiffsfeuer, das ein Schiff beim Planen braucht. Drohnen, die während der Runde starten, feuern erst in der nächsten — außer der Schwarmangriff schickt sie los.".into(),
                    "Schaden trifft zuerst den Schild und erst dann den Rumpf. Der eigene Schild eines Schiffs (Eisen-Dreadnought: 2) regeneriert nicht; Zusatzschild aus Befehlen wie den Notschilden gilt nur für die Runde, außer bei der Ablativpanzerung, die bleibt, bis sie weggeschossen ist. Ein Schiff mit 0 HP ist zerstört und verlässt das Brett — nach der Effects-Phase, nach dem Kampf und nach dem Eigenschaden im Cleanup.".into(),
                    "Der Angriffswert enthält schon alles: Rundenboni aus Befehlen, den Bonus des Schrott-Raiders, die Aura des Signalverstärkers, die Speerspitze der Vanguard und den Overdrive. Support-Schiffe greifen nie an, egal mit welchem Bonus.".into(),
                    format!("Die Admirale verbiegen diese Regeln: Die Railgun der Pendulum feuert vor allen anderen, der Taktvorsprung lässt ein Schiff vor dem Rest schießen, das Fokusfeuer gibt jedem Treffer auf sein Ziel +{} Schaden, der Schwarmangriff schickt alle deine Drohnen auf sein Ziel — durch Deckung und ohne Feuerbefehle —, und das Gegenfeuer schlägt zurück, wenn alle Schüsse gefallen sind. Das Trugbild von Admiral Moreau gilt als Schiff vorn in seiner Spalte, aber Flaggschiffe durchschauen es. Der Befehl Ablenkung zieht die gegnerischen Schüsse beider Arten auf dein gewähltes Schiff, solange es steht und für den Schützen ungedeckt ist; nur Schwarmangriff und Railgun behalten ihr Ziel.", V.focus_fire).into(),
                ],
                Some("Spaltenfeuer (gepunkteter Rahmen) trifft das vordere Schiff seiner Spalte, Schiffsfeuer (gepunkteter Ring) folgt seinem Schiff. Rot der Schuss, blass mit Kreuz ein Schuss, der verpufft; die Ziffern geben die Reihenfolge der Schüsse an."),
            ),
            RulesChapter::Ships => section(
                "Schiffe",
                vec![
                    "Schiffe haben Rumpfpunkte (HP), Angriff (ATK) und manchmal einen Schild. Es gibt vier Klassen: Jäger, Kreuzer und Schlachtschiffe sind Kampfschiffe, Support-Schiffe greifen nie an (0 ATK). Dein Schiffsdeck enthält jedes Schiff zweimal.".into(),
                    "Manche Schiffe haben eine Fähigkeit. Beim Deploy, sobald beide Seiten gelegt haben: Ein Titan-Träger schickt seine Drohne in den nächsten freien Slot seiner Lane — sie zählt weder zur Lane- noch zur Flottengrenze und entfällt nur, wenn kein Slot der Lane frei ist —, danach richtet jede Plasma-Fregatte 3 Schaden an einem zufälligen gegnerischen Schiff der gleichnamigen Lane an (Strike gegen Strike, Command gegen Command), ohne Rücksicht auf Deckung — auch eine frische Drohne kann es treffen. Ein so zerstörtes Schiff manövriert nicht mehr und bekommt deinen Befehl nicht.".into(),
                    "Die Drohne (1 HP, 1 ATK, Jäger) liegt in keinem Deck; sie zählt als Kampfschiff, aber weder zur Lane- noch zur Flottengrenze. Titan-Träger schicken sie beim Deploy los, die Drohnenstaffel zwei in der Effects-Phase, die Hive am Rundenende. Sie startet nur, wenn Platz ist — auch für die Ziele deines Manövers und Bewegungsbefehls dieser Runde. Eine Drohne, die während der Runde startet, feuert erst ab der nächsten Runde, mit dem Schwarmangriff sofort. Der Schrott-Raider bekommt dauerhaft +1 ATK, sobald ein gegnerisches Schiff in der gleichnamigen Lane zerstört wird, egal wodurch; den Bonus behält er auch, wenn er sich bewegt. Die Reparaturdrohne repariert im Cleanup, der Signalverstärker verstärkt seine Lane.".into(),
                    "Die Galerie zeigt auch die fünf Flaggschiffe und das Trugbild von Admiral Moreau; beide liegen in keinem Deck.".into(),
                ],
                None,
            ),
            RulesChapter::Commands => section(
                "Befehle",
                vec![
                    "Befehle werden einmal gespielt und in der Effects-Phase aufgelöst, vor dem Kampf. Manche brauchen beim Planen ein Ziel — ein eigenes Schiff, ein gegnerisches Schiff (Wärmesucher, Traktorstrahl), einen Slot der Gegnerseite (Torpedo) oder eine gegnerische Spalte (Durchschlag) —, die übrigen wählen ihr Ziel automatisch. Rundeneffekte wie ATK-Boni, Zusatzschild und EMP enden im Cleanup.".into(),
                    "Befehle wirken nach Typ, in dieser Reihenfolge: Umgruppierung, Taktik, Verstärkung, Spezial, Schwächung, Angriff. Gegenmaßnahmen (Spezial) lassen den Angriffs- oder Schwächungsbefehl des Gegners in derselben Runde verpuffen, bevor er wirkt. Dein Befehlsdeck enthält jeden Befehl einmal, nur Torpedo und Sperrfeuer zweimal.".into(),
                ],
                None,
            ),
            RulesChapter::Hand => section(
                "Hand und Decks",
                vec![
                    "Du beginnst mit 4 Schiffen — immer 1 Support- und 3 Kampfschiffe — und 3 Befehlen. Nach jeder Runde kommt Nachschub: 3 Schiffe und 3 Befehle aus deinen Decks, davon nimmst du ein Schiff und einen Befehl auf die Hand; der Rest wird zurück ins Deck gemischt. Erst danach planst du deinen Zug. Jede Hand fasst höchstens 10 Karten: Eine volle Hand bekommt keinen Nachschub, also spiel deine Karten. Hat einer deiner Schrott-Raider in der Runde Schrott geborgen und steht noch einer auf dem Brett, ist einer der drei Befehle die Schrottverwertung: Du spielst sie wie jeden Befehl, und der gewählte Schrott-Raider erhält dauerhaft +1 ATK. Nimmst du sie nicht, verfällt sie. Gleiche Karten liegen als Stapel, jede zählt aber einzeln. Plündern zieht sofort 1 Schiff und 1 Befehl zusätzlich, Gegenmaßnahmen 1 Befehl, wenn nichts verpufft ist, wieder bis zum Handlimit.".into(),
                    "Dein Schiffsdeck hat 16 Karten (jedes Schiff zweimal), dein Befehlsdeck 34 (Torpedo und Sperrfeuer zweimal, jeder andere Befehl einmal). Geht ein Deck zur Neige, zeigt der Nachschub nur noch, was übrig ist.".into(),
                ],
                None,
            ),
            RulesChapter::Controls => section(
                "Bedienung",
                vec![
                    "Tippe eine Karte auf deiner Hand an, dann einen leuchtenden Slot oder ein Schiff auf dem Brett. Nochmal auf die Karte tippen nimmt sie zurück. In Runde 1 steht dein Flaggschiff mit goldenem Rahmen vorn in der Schiffshand. Ohne gewählte Karte wählst du mit einem Tipp eines deiner Schiffe oder einen Geist: Vor jeder gegnerischen Spalte erscheint ein Pfeil, und die gegnerischen Schiffe leuchten auf. Ein Tipp auf einen Pfeil gibt deinem Schiff Spaltenfeuer auf diese Spalte, ein Tipp auf ein gegnerisches Schiff Schiffsfeuer auf genau dieses Schiff; nochmal auf dasselbe Ziel hebt den Feuerbefehl auf. Ein Schiff, das schon auf dem Brett steht, zeigt dazu die Slots, in die es manövrieren kann; ein Manöver hebt seinen Feuerbefehl auf. Ein weiterer Tipp auf das gewählte Schiff nimmt sein Legen bzw. sein Manöver zurück; Escape, ein Rechtsklick oder ein Tipp neben das Brett beendet die Auswahl. Deine Admiralsfähigkeit ist die Karte links neben deinen Befehlen, die Zielerfassung von Admiral Kerr der Knopf darüber. Die Checkliste unter „Zug beenden“ zeigt, welche Teile deines Zugs eingeplant, noch offen oder nicht möglich sind.".into(),
                    "Während du planst, zeigt das Brett die Runde so, als täte dein Gegner nichts: Je Feuerbefehl zeigt ein Bogen mit seiner Spitze auf das Schiff, das der Schuss treffen würde, bei Schiffsfeuer mit einem Ring um die Spitze; ein Schuss, der verpuffen würde, ist blass und gestrichelt, mit einem Kreuz statt der Spitze. Ist ein Schiff gewählt, zeigt die Vorschau schon, was ein Tipp auf den Pfeil oder das Schiff unter dem Zeiger bewirken würde. Die Hüllenbalken blinken mit dem Schaden, den ein Schiff nehmen würde, und ein Totenkopf markiert jedes Schiff, das zerstört würde. Zufall wie der Schuss der Plasma-Fregatte ist nicht eingerechnet. Das Feld oben rechts zeigt, was du über deinen Gegner weißt: ob er seinen Zug schon beendet hat und wie viele Karten er auf der Hand und noch in den Decks hat.".into(),
                    "„Zug beenden“ schickt deinen Plan ab. Während die Runde aufgelöst wird, kannst du die Animation überspringen; der Log-Knopf oben rechts listet jedes Ereignis des Matches. Mit dem Mausrad oder gehaltenem ↑ und ↓ zoomst du ins Brett, herangezoomt verschiebst du es durch Ziehen. Der Kamera-Knopf oben rechts oder die Taste V wechselt zwischen dem Blick von der Seite und von hinten oben; hältst du ← oder → gedrückt oder ziehst mit gedrückter mittlerer Maustaste, dreht sich die Kamera stufenlos dazwischen. Drehung und Zoom bleiben fürs nächste Match erhalten. Ton, Bordfunk, Animationstempo und Sprache findest du in den Einstellungen neben diesem Dialog; die Fahne daneben gibt das Match nach Rückfrage auf, der Gegner gewinnt sofort.".into(),
                ],
                None,
            ),
        }
    }
    fn phase(&self, phase: Phase) -> PhaseText {
        let (name, text) = match phase {
            Phase::Deploy => (
                "Deploy",
                "Neue Schiffe landen, in Runde 1 auch die Flaggschiffe. Die Admiralsfähigkeiten werden aufgedeckt (ein Trugbild erscheint), Titan-Träger schicken Drohnen, Plasma-Fregatten feuern, die Pendulum erfasst ihr Ziel.",
            ),
            Phase::Maneuver => (
                "Manöver",
                "Manövrierende Schiffe wechseln den Slot und halten das Feuer, die Mirage tauscht die Plätze. Dann hebt die Bastion ihren Wächterschild.",
            ),
            Phase::Effects => (
                "Effects",
                "Befehle wirken, nach Typ; Schaden zuletzt. Zerstörte Schiffe verschwinden.",
            ),
            Phase::Combat => (
                "Combat",
                "Erst die Railgun, dann der Taktvorsprung, dann die übrigen Schiffe mit Feuerbefehl: Strike-Lane vor Command-Lane, je nach Slot. Spaltenfeuer trifft das vordere Schiff seiner Spalte, Schiffsfeuer sein Ziel, wenn es ungedeckt ist — sonst verpufft der Schuss. Zuletzt das Gegenfeuer. Zerstörte verschwinden.",
            ),
            Phase::Cleanup => (
                "Cleanup",
                "Schiffe im Kurzsprung kehren zurück, Eigenschaden der Überladung, Reparaturdrohnen reparieren, Rundeneffekte enden, das Trugbild löst sich auf, die Hive startet eine Drohne.",
            ),
            Phase::Draw => (
                "Nachschub",
                "Jeder wählt ein Schiff und einen Befehl aus je dreien (Handlimit 10).",
            ),
            Phase::Check => (
                "Prüfung",
                "Wer sein Flaggschiff verloren hat, verliert; fielen beide, ist es unentschieden.",
            ),
        };
        PhaseText { name, text }
    }
    fn figures_you(&self) -> &'static str {
        "Du"
    }
    fn figures_enemy(&self) -> &'static str {
        "Gegner"
    }
    fn figures_slot(&self, index: usize) -> String {
        format!("Slot {index}")
    }
    fn figures_covered(&self) -> &'static str {
        "gedeckt"
    }
    fn figures_exposed(&self) -> &'static str {
        "ungedeckt"
    }
    fn figures_lane_limit(&self) -> &'static str {
        "max. 4 Schiffe je Lane, 6 je Seite"
    }
    fn figures_column_fire(&self) -> &'static str {
        "Spaltenfeuer"
    }
    fn figures_tracking_fire(&self) -> &'static str {
        "Schiffsfeuer"
    }
    fn figures_column_through(&self) -> &'static str {
        "trifft das vordere Schiff, nach dessen Fall das dahinter"
    }
    fn figures_column_dodge(&self) -> &'static str {
        "verpufft, wenn das Schiff die Spalte verlässt"
    }
    fn figures_tracking_follows(&self) -> &'static str {
        "folgt seinem Schiff in einen anderen Slot"
    }
    fn figures_tracking_covered(&self) -> &'static str {
        "verpufft, solange sein Schiff gedeckt ist"
    }
    fn figures_round(&self, round: u32) -> String {
        format!("Runde {round}")
    }
    fn figures_overdrive(&self, bonus: i32) -> String {
        if bonus == 0 {
            "kein Overdrive".into()
        } else {
            format!("+{bonus} ATK")
        }
    }
    fn figures_rounds(&self, from: u32, to: Option<u32>) -> String {
        match to {
            None => format!("ab Runde {from}"),
            Some(to) => format!("Runde {from}–{to}"),
        }
    }
    fn figures_maneuver(&self) -> &'static str {
        "Manöver"
    }
    fn figures_turn_parts(&self) -> [&'static str; 4] {
        [
            "1 Schiff",
            "1 Befehl",
            "Manöver oder Fähigkeit",
            "Feuerbefehle",
        ]
    }
    fn figures_or(&self) -> &'static str {
        "oder"
    }
    fn figures_round_one(&self) -> &'static str {
        "Runde 1: Flaggschiff + 2 Schiffe"
    }
    fn command_type(&self, command_type: CommandType) -> &'static str {
        match command_type {
            CommandType::Reposition => "Umgruppierung",
            CommandType::Tactic => "Taktik",
            CommandType::Buff => "Verstärkung",
            CommandType::Special => "Spezial",
            CommandType::Debuff => "Schwächung",
            CommandType::Attack => "Angriff",
        }
    }
    fn rules_copies(&self, count: usize) -> String {
        format!("{count}× im Deck")
    }
    fn rules_flagship(&self, admiral: &str) -> String {
        format!("Flaggschiff von {admiral}")
    }
    fn rules_token(&self) -> &'static str {
        "Token, in keinem Deck"
    }

    // --- Dialoge und Knöpfe ---
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
    fn surrender_title(&self) -> &'static str {
        "Aufgeben"
    }
    fn surrender_question(&self) -> &'static str {
        "Dieses Match aufgeben? Dein Gegner gewinnt sofort."
    }
    fn surrender_confirm(&self) -> &'static str {
        "Aufgeben"
    }

    // --- loading ---
    fn loading_assets(&self, percent: u32) -> String {
        format!("Lade Assets … {percent} %")
    }
    fn loading_failed(&self) -> &'static str {
        "Laden fehlgeschlagen — starte das Spiel neu."
    }

    // --- opponent ---
    fn opponent_title(&self) -> &'static str {
        "Gegner"
    }
    fn opponent_choosing(&self) -> &'static str {
        "wählt seinen Admiral …"
    }
    fn opponent_hidden(&self) -> &'static str {
        "Wahl verdeckt"
    }
    fn opponent_planning(&self) -> &'static str {
        "plant …"
    }
    fn opponent_ready(&self) -> &'static str {
        "Zug beendet"
    }
    fn opponent_ships(&self) -> &'static str {
        "Schiffe"
    }
    fn opponent_commands(&self) -> &'static str {
        "Befehle"
    }
    fn opponent_hand(&self) -> &'static str {
        "Hand"
    }
    fn opponent_deck(&self) -> &'static str {
        "Deck"
    }

    // --- radio ---
    fn radio_speaker(&self) -> &'static str {
        "Flottenfunk"
    }
    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str] {
        match cue {
            RadioCue::BattleStart => &[
                "Alle Schiffe auf Gefechtsstation. Wir erwarten Ihre Befehle, Admiral.",
                "Flotte in Position. Bereit, sobald Sie es sind, Admiral.",
            ],
            RadioCue::OrdersSent => &[
                "Befehle bestätigt. Ausführung läuft.",
                "Verstanden, Admiral.",
                "Verstanden. Alle Schiffe: ausführen.",
            ],
            RadioCue::EnemyReady => &[
                "Die feindliche Flotte hat ihre Züge festgelegt.",
                "Der Feind ist bereit, Admiral. Ihre Entscheidung.",
            ],
            RadioCue::Overdrive => &[
                "Reaktoren im Overdrive. Alle Waffen frei!",
                "Overdrive aktiv! Alle Geschütze auf volle Leistung!",
            ],
            RadioCue::OverdriveUp => &[
                "Overdrive steigt! Die Reaktoren laufen heiß!",
                "Wir fahren die Reaktoren weiter hoch. Rechnen Sie mit schwererem Feuer!",
            ],
            RadioCue::LastShip => &[
                "Das ist unser letztes Kriegsschiff, Admiral!",
                "Wir haben nur noch ein Kriegsschiff!",
            ],
            RadioCue::Victory => &[
                "Feindliche Flotte vernichtet. Der Sektor gehört uns, Admiral!",
                "Das war der Letzte von ihnen. Sieg, Admiral!",
            ],
            RadioCue::Defeat => &[
                "Wir haben die Flotte verloren. Alle Mann von Bord!",
                "Unsere Flotte ist vernichtet. Alle Überlebenden: Rückzug!",
            ],
            RadioCue::Draw => &["Beide Flotten sind vernichtet. Heute gewinnt niemand."],
            RadioCue::EnemySurrender => &["Der Feind zieht sich zurück. Er hat kapituliert!"],
            RadioCue::Retreat => &["Rückzug signalisieren. Alle Schiffe: Gefecht abbrechen."],
            RadioCue::TorpedoAway => &["Torpedo abgefeuert!", "Torpedo läuft!"],
            RadioCue::TorpedoHit => &["Volltreffer!", "Torpedotreffer bestätigt!"],
            RadioCue::TorpedoMissed => &[
                "Torpedo verfehlt! Der Feind ist ausgewichen.",
                "Kein Treffer. Das Ziel ist entwischt.",
            ],
            RadioCue::TorpedoIncoming => &[
                "Torpedo im Anflug!",
                "Feindlicher Torpedo! Auf Einschlag vorbereiten!",
            ],
            RadioCue::TorpedoDodged => &[
                "Der feindliche Torpedo hat verfehlt!",
                "Torpedo ging daneben. Gut geflogen!",
            ],
            RadioCue::BarrageHit => &[
                "Wir haben sie in der Bewegung erwischt!",
                "Sperrfeuer im Ziel! Der Feind ist direkt hineingeflogen!",
            ],
            RadioCue::BarrageMissed => {
                &["Sperrfeuer ins Leere. Der Feind hat seine Position gehalten."]
            }
            RadioCue::BarrageCaught => &[
                "Sperrfeuer! Wir sind direkt hineingeflogen!",
                "Der Feind hat nur darauf gewartet, dass wir uns bewegen!",
            ],
            RadioCue::OrbitalStrike => &["Orbitalschlag ist unterwegs ins Ziel!"],
            RadioCue::OrbitalIncoming => &["Orbitalschlag im Anflug! Festhalten!"],
            RadioCue::Volley => &["Alle Batterien: Feuer!"],
            RadioCue::VolleyIncoming => &["Raketen im Anflug!"],
            RadioCue::EmpHit => &["EMP-Treffer! Die feindlichen Waffen schweigen."],
            RadioCue::EmpIncoming => &["EMP! Unsere Waffen sind ausgefallen!"],
            RadioCue::Jammer => &["Wir stören ihre Zielerfassung."],
            RadioCue::Jammed => &["Der Feind stört unsere Zielerfassung!"],
            RadioCue::ShieldsUp => &["Schilde hoch!", "Energie auf die Schilde!"],
            RadioCue::Overcharge => {
                &["Waffen werden überladen. Lange hält das Schiff das nicht aus!"]
            }
            RadioCue::PowerSurge => &["Energiestoß für die Strike-Linie!"],
            RadioCue::Scavenge => &["Die Bergungsteams melden neuen Nachschub."],
            RadioCue::Intercept => &["Gehen auf Abfangkurs!"],
            RadioCue::Recall => &["Zurückziehen und zusammenflicken!"],
            RadioCue::Flanking => &["Flankenmanöver, los!"],
            RadioCue::DroneLaunch => &["Drohne gestartet!"],
            RadioCue::FrigateFire => &["Fregatte feuert beim Eintreffen!"],
            RadioCue::Repair => &["Reparaturdrohnen bei der Arbeit."],
            RadioCue::Scrap => &["Der Raider plündert das Wrack!"],
            RadioCue::EnemyDestroyed => &[
                "Feindschiff zerstört!",
                "Ziel ausgeschaltet!",
                "Einer weniger!",
                "Abschuss bestätigt!",
            ],
            RadioCue::ShipLost => &[
                "Wir haben ein Schiff verloren!",
                "Schiff verloren! Schiff verloren!",
                "Eines unserer Schiffe ist vernichtet!",
            ],
            RadioCue::HullCritical => &[
                "Hüllenintegrität kritisch!",
                "Das Schiff ist schwer getroffen, Admiral!",
            ],
        }
    }

    // --- result ---
    fn result_title(&self, result: MatchResult) -> &'static str {
        match result {
            MatchResult::Won => "Sieg",
            MatchResult::Lost => "Niederlage",
            MatchResult::Draw => "Unentschieden",
        }
    }
    fn result_won_subtitle(&self, flagship: &str) -> String {
        format!("Das gegnerische Flaggschiff {flagship} ist zerstört.")
    }
    fn result_lost_subtitle(&self, flagship: &str) -> String {
        format!("Dein Flaggschiff {flagship} ist zerstört.")
    }
    fn result_draw_subtitle(&self, own: &str, enemy: &str) -> String {
        format!("Beide Flaggschiffe sind zerstört: deine {own} und die gegnerische {enemy}.")
    }
    fn result_surrendered(&self, won: bool) -> &'static str {
        if won {
            "Der Gegner hat aufgegeben."
        } else {
            "Du hast aufgegeben."
        }
    }
    fn result_rounds(&self, round: u32) -> String {
        format!("Entschieden in Runde {round}")
    }
    fn result_view_board(&self) -> &'static str {
        "Brett ansehen"
    }
    fn result_new_match(&self) -> &'static str {
        "Neues Match"
    }

    // --- log ---
    fn log_round_header(&self, round: u32) -> String {
        format!("— Runde {round} —")
    }
    fn log_who(&self, side: Side) -> &'static str {
        match side {
            Side::Player => "Du",
            Side::Enemy => "Gegner",
        }
    }
    fn log_slot(&self, side: Side, lane: &str, slot: usize) -> String {
        let owner = match side {
            Side::Player => "eigene",
            Side::Enemy => "gegnerische",
        };
        format!("{owner} {lane} Slot {slot}")
    }
    fn log_column(&self, side: Side, column: usize) -> String {
        let owner = match side {
            Side::Player => "eigener",
            Side::Enemy => "gegnerischer",
        };
        format!("{owner} Spalte {column}")
    }
    fn log_ship(&self, ship: &str, slot: &str) -> String {
        format!("{ship} ({slot})")
    }
    fn log_deploy(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Deploy: {who} — {ship} auf {slot}")
    }
    fn log_deploy_flagship(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Deploy: {who} — Flaggschiff {ship} auf {slot}")
    }
    fn log_power(&self, who: &str, power: &str, target: &str) -> String {
        format!("Fähigkeit: {who} — {power} auf {target}")
    }
    fn log_decoy(&self, ship: &str) -> String {
        format!("Fähigkeit: {ship} erscheint")
    }
    fn log_mark(&self, who: &str, ship: &str) -> String {
        format!("Zielerfassung: {who} — die Pendulum erfasst {ship}")
    }
    fn log_swap(&self, who: &str, ship: &str, partner: &str) -> String {
        format!("Phasentausch: {who} — {ship} und {partner} tauschen die Plätze und feuern weiter")
    }
    fn log_pass(&self, who: &str) -> String {
        format!("Deploy: {who} — kein Schiff")
    }
    fn log_maneuver(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Manöver: {who} — {ship} wechselt auf {slot} und feuert diese Runde nicht")
    }
    fn log_command(&self, who: &str, command: &str) -> String {
        format!("Befehl: {who} — {command}")
    }
    fn log_command_on(&self, who: &str, command: &str, slot: &str) -> String {
        format!("Befehl: {who} — {command} auf {slot}")
    }
    fn log_damage(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Wirkung: {command} trifft {ship} mit {amount}")
    }
    fn log_attack_bonus(&self, command: &str, ship: &str, delta: i32) -> String {
        format!(
            "Wirkung: {command} — {ship} diese Runde {} ATK",
            signed(delta)
        )
    }
    fn log_shield(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Wirkung: {command} — {ship} diese Runde +{amount} Schild")
    }
    fn log_disable(&self, command: &str, ship: &str) -> String {
        format!("Wirkung: {command} — {ship} kann diese Runde nicht angreifen")
    }
    fn log_heal(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Wirkung: {command} heilt {ship} um {amount}")
    }
    fn log_move(&self, command: &str, ship: &str, slot: &str) -> String {
        format!("Wirkung: {command} — {ship} wechselt auf {slot}")
    }
    fn log_shield_permanent(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Wirkung: {command} — {ship} dauerhaft +{amount} Schild")
    }
    fn log_tow(&self, command: &str, ship: &str, slot: &str) -> String {
        format!("Wirkung: {command} zieht {ship} auf {slot}")
    }
    fn log_jump(&self, command: &str, ship: &str) -> String {
        format!("Wirkung: {command} — {ship} springt bis zum Cleanup vom Brett")
    }
    fn log_taunt(&self, command: &str, ship: &str) -> String {
        format!("Wirkung: {command} — {ship} zieht diese Runde das Feuer des Gegners auf sich")
    }
    fn log_launch(&self, command: &str, ship: &str) -> String {
        format!("Wirkung: {command} schickt {ship} los")
    }
    fn log_ram(&self, command: &str, ship: &str, target: &str, dealt: i32, taken: i32) -> String {
        format!(
            "Wirkung: {command} — {ship} rammt {target}: {dealt} ausgeteilt, {taken} eingesteckt"
        )
    }
    fn log_jam(&self, command: &str, ship: &str, power: &str) -> String {
        format!("Wirkung: {command} — {power} auf {ship} fällt weg")
    }
    fn log_jam_decoy(&self, command: &str, ship: &str) -> String {
        format!("Wirkung: {command} — {ship} löst sich auf")
    }
    fn log_countered(&self, command: &str, who: &str, countered: &str) -> String {
        format!("Wirkung: {command} ({who}) fängt {countered} ab")
    }
    fn log_rule(&self, command: &str, who: &str, detail: &str) -> String {
        format!("Wirkung: {command} ({who}) — diese Runde {detail}")
    }
    fn log_rejoin(&self, ship: &str) -> String {
        format!("Cleanup: {ship} kehrt aus dem Sprung zurück")
    }
    fn log_repair(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Cleanup: {source} repariert {ship} um {amount}")
    }
    fn log_ability_damage(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Deploy: {source} trifft {ship} mit {amount}")
    }
    fn log_guardian_shield(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Wächterschild: {source} gibt {ship} diese Runde +{amount} Schild")
    }
    fn log_spawn(&self, source: &str, ship: &str) -> String {
        format!("Deploy: {source} schickt {ship} los")
    }
    fn log_brood(&self, source: &str, ship: &str) -> String {
        format!("Brutkammer: {source} schickt {ship} los")
    }
    fn log_salvage(&self, ship: &str) -> String {
        format!(
            "Beute: {ship} birgt Schrott — die Schrottverwertung kommt mit dem nächsten Nachschub"
        )
    }
    fn log_draw(&self, command: &str, who: &str, ships: u32, commands: u32) -> String {
        format!(
            "Wirkung: {command} — {who} {}",
            self.banner_draw(ships, commands)
        )
    }
    fn log_no_effect(&self, command: &str, who: &str) -> String {
        format!("Wirkung: {command} ({who}) bleibt ohne Wirkung")
    }
    fn log_miss(&self, command: &str, slot: &str) -> String {
        format!("Wirkung: {command} verpufft auf {slot}")
    }
    fn log_railgun(&self, source: &str, target: &str, amount: i32) -> String {
        format!("Railgun: {source} trifft {target} mit {amount}")
    }
    fn log_railgun_blocked(&self, source: &str, target: &str) -> String {
        format!(
            "Railgun: Der Schuss von {source} auf {target} ist blockiert — das Ziel ist gedeckt"
        )
    }
    fn log_attack(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String {
        format!(
            "Kampf: {attacker} feuert auf {target} ({})",
            amount_notes(amount, notes)
        )
    }
    fn log_through(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String {
        format!(
            "Kampf: {attacker} feuert am gefallenen Vordermann vorbei auf {target} ({})",
            amount_notes(amount, notes)
        )
    }
    fn log_fizzle_empty(&self, attacker: &str, column: &str) -> String {
        format!("Kampf: Der Schuss von {attacker} verpufft — in {column} steht kein Schiff")
    }
    fn log_fizzle_cleared(&self, attacker: &str, column: &str) -> String {
        format!(
            "Kampf: Der Schuss von {attacker} verpufft — die Schiffe in {column} sind schon zerstört"
        )
    }
    fn log_fizzle(&self, attacker: &str, target: &str) -> String {
        format!("Kampf: Der Schuss von {attacker} verpufft — {target} ist schon zerstört")
    }
    fn log_fizzle_covered(&self, attacker: &str, target: &str) -> String {
        format!("Kampf: Der Schuss von {attacker} verpufft — {target} ist gedeckt")
    }
    fn log_fizzle_gone(&self, attacker: &str) -> String {
        format!(
            "Kampf: Der Schuss von {attacker} verpufft — sein Ziel steht nicht mehr auf dem Brett"
        )
    }
    fn log_silenced(&self, ship: &str) -> String {
        format!("Taktvorsprung: {ship} fällt, bevor es feuern kann")
    }
    fn log_counterfire(&self, ship: &str, source: &str, amount: i32) -> String {
        format!("Gegenfeuer: {ship} nimmt {amount} Schaden für den Treffer auf {source}")
    }
    fn log_self_damage(&self, ship: &str, amount: i32) -> String {
        format!("Cleanup: {ship} nimmt {amount} Eigenschaden")
    }
    fn log_destroyed(&self, ship: &str) -> String {
        format!("Zerstört: {ship}")
    }
    fn log_flagship_destroyed(&self, ship: &str) -> String {
        format!("Zerstört: Flaggschiff {ship}!")
    }
    fn log_vanish(&self, ship: &str) -> String {
        format!("Cleanup: {ship} löst sich auf")
    }

    // --- banner ---
    fn banner_command(&self, who: &str, command: &str) -> String {
        format!("{who}: {command}")
    }
    fn banner_mark_target(&self, ship: &str) -> String {
        format!("erfasst {ship}")
    }
    fn banner_blocked(&self) -> &'static str {
        "blockiert — das Ziel ist gedeckt"
    }
    fn banner_no_effect(&self) -> &'static str {
        "ohne Wirkung"
    }
    fn banner_miss(&self, slot: &str) -> String {
        format!("verpufft auf {slot}")
    }
    fn banner_draw(&self, ships: u32, commands: u32) -> String {
        if ships > 0 {
            format!("+{ships} Schiff, +{commands} Befehl")
        } else {
            format!("+{commands} Befehl")
        }
    }
    fn banner_countered(&self, command: &str) -> String {
        format!("fängt {command} ab")
    }
    fn banner_extra_ship(&self) -> &'static str {
        "+1 Schiff"
    }
    fn banner_maneuver_and_power(&self) -> &'static str {
        "Manöver und Fähigkeit"
    }

    // --- float ---
    fn float_damage(&self, amount: i32) -> String {
        format!("−{amount}")
    }
    fn float_heal(&self, amount: i32) -> String {
        format!("+{amount} HP")
    }
    fn float_shield(&self, amount: i32) -> String {
        format!("+{amount} Schild")
    }
    fn float_attack(&self, delta: i32) -> String {
        format!("{} ATK", float_signed(delta))
    }
    fn float_salvage(&self) -> &'static str {
        "Schrott"
    }
    fn float_disabled(&self) -> &'static str {
        "Lahmgelegt"
    }
    fn float_hold_fire(&self) -> &'static str {
        "Feuert nicht"
    }
    fn float_miss(&self) -> &'static str {
        "Daneben"
    }
    fn float_silenced(&self) -> &'static str {
        "Ausgeschaltet"
    }
    fn float_locked(&self) -> &'static str {
        "Erfasst"
    }
    fn float_blocked(&self) -> &'static str {
        "Blockiert"
    }
    fn float_decoy(&self) -> &'static str {
        "Trugbild"
    }
    fn float_shield_permanent(&self, amount: i32) -> String {
        format!("+{amount} Schild (dauerhaft)")
    }
    fn float_taunt(&self) -> &'static str {
        "Zieht Feuer"
    }
    fn float_jammed(&self) -> &'static str {
        "Funkstille"
    }
    fn float_jump(&self) -> &'static str {
        "Sprung"
    }
    fn float_fizzle(&self) -> &'static str {
        "Verpufft"
    }
    fn float_covered(&self) -> &'static str {
        "Gedeckt"
    }

    // --- Ids aus dem Regelwerk ---
    fn error(&self, code: ErrorCode) -> &'static str {
        use ErrorCode as E;
        match code {
            E::MatchFull => "Dieses Match ist bereits voll.",
            E::MatchOver => "Das Match ist vorbei.",
            E::MatchStarted => "Das Match hat schon begonnen.",
            E::OpponentMissing => "Der Gegner ist noch nicht beigetreten.",
            E::AdmiralRequired => "Wähle zuerst deinen Admiral.",
            E::AdmiralAlreadyChosen => "Du hast deinen Admiral schon gewählt.",
            E::OpponentChoosing => "Der Gegner wählt noch seinen Admiral.",
            E::DraftPending => "Wähle zuerst deinen Nachschub.",
            E::DraftUnavailable => "Gerade gibt es keinen Nachschub zur Wahl.",
            E::DraftInvalid => "Wähle ein Schiff und einen Befehl aus dem Nachschub.",
            E::FlagshipRequired => "Lege in Runde 1 dein Flaggschiff.",
            E::FlagshipNotAllowed => "Das Flaggschiff wird nur in Runde 1 gelegt.",
            E::PowerAndManeuver => {
                "Entweder das Grundmanöver oder die Admiralsfähigkeit, nicht beides."
            }
            E::PowerTargetInvalid => "Das ist kein gültiges Ziel für deine Admiralsfähigkeit.",
            E::MarkUnavailable => "Die Pendulum kann in dieser Runde kein Ziel erfassen.",
            E::MarkTargetInvalid => "Die Pendulum kann nur ein gegnerisches Schiff erfassen.",
            E::SwapNotAllowed => {
                "Nur die Mirage kann mit einem eigenen Schiff die Plätze tauschen."
            }
            E::AlreadySubmitted => "Der Zug dieser Runde ist bereits beendet.",
            E::ShipNotInHand => "Diese Schiffskarte ist nicht auf der Hand.",
            E::TooManyShips => "So viele Schiffe darfst du in dieser Runde nicht legen.",
            E::SlotUnavailable => "Auf diesem Slot kann kein Schiff landen.",
            E::CommandNotInHand => "Diese Befehlskarte ist nicht auf der Hand.",
            E::TargetRequired => "Dieser Befehl braucht ein Ziel.",
            E::TargetNotAllowed => "Dieser Befehl braucht kein Ziel.",
            E::TargetNotOwnShip => "Das Befehlsziel ist kein eigenes Schiff.",
            E::TargetWrongLane => "Dieser Befehl kann kein Schiff in dieser Lane zum Ziel haben.",
            E::TargetNotEnemySlot => "Das Befehlsziel ist kein Slot auf der Gegnerseite.",
            E::TargetNotEnemyShip => "Das Befehlsziel ist kein gegnerisches Schiff.",
            E::TargetWrongShip => "Dieser Befehl kann dieses Schiff nicht zum Ziel haben.",
            E::DestinationRequired => {
                "Dieser Befehl braucht einen Slot, in den das Schiff wechselt."
            }
            E::DestinationNotAllowed => "Dieser Befehl verlegt kein Schiff.",
            E::DestinationUnavailable => "In diesen Slot kann das Schiff nicht wechseln.",
            E::ManeuverNotOwnShip => {
                "Manövrieren kann nur ein eigenes Schiff, das schon auf dem Brett steht."
            }
            E::ManeuverDestinationUnavailable => {
                "In diesen Slot kann das Schiff nicht manövrieren."
            }
            E::ManeuverConflict => {
                "Ein Befehl kann das Schiff nicht verlegen, das in dieser Runde manövriert."
            }
            E::FireShipInvalid => {
                "Einen Feuerbefehl bekommt nur ein eigenes Kampfschiff, je Schiff einer — nicht das Schiff, das in dieser Runde manövriert."
            }
            E::FireTargetInvalid => {
                "Ziel eines Feuerbefehls ist eine gegnerische Spalte oder ein gegnerisches Schiff auf dem Brett."
            }
        }
    }
    fn lane(&self, lane: LaneType) -> &'static str {
        match lane {
            LaneType::Strike => "Strike",
            LaneType::Command => "Command",
        }
    }
    fn stats_hp(&self) -> &'static str {
        "HP"
    }
    fn stats_atk(&self) -> &'static str {
        "ATK"
    }
    fn stats_shield(&self) -> &'static str {
        "Schild"
    }
    fn ship_class(&self, class: ShipClass) -> &'static str {
        match class {
            ShipClass::Fighter => "Jäger",
            ShipClass::Cruiser => "Kreuzer",
            ShipClass::Battleship => "Schlachtschiff",
            ShipClass::Support => "Support",
        }
    }
    fn target_mode(&self, mode: TargetMode) -> &'static str {
        use TargetMode as M;
        match mode {
            M::None => "",
            M::AutoHighestHp => "Gegner mit den meisten HP",
            M::AutoLowestHp => "Gegner mit den wenigsten HP",
            M::AutoHighestAtk => "Gegner mit der höchsten ATK",
            M::AutoAll => "Alle Ziele",
            M::AutoMoved => "Bewegte Gegner",
            M::AutoArrived => "Neue Gegner",
            M::AutoUnpaired => "Gegner ohne Partner",
            M::AllFriendly => "Alle Verbündeten",
            M::AllFriendlyStrike => "Verbündete in Strike",
            M::AllEnemyStrike => "Gegner in Strike",
            M::AllEnemyCommand => "Gegner in Command",
            M::EnemyPower => "Gegnerische Fähigkeit",
            M::ChooseFriendly => "Verbündeten wählen",
            M::ChooseEnemy => "Gegner wählen",
            M::ChooseEnemySlot => "Gegner-Slot wählen",
            M::ChooseEnemyColumn => "Gegner-Spalte wählen",
        }
    }
    fn ship(&self, ship: ShipId) -> CardText {
        use ShipId as S;
        match ship {
            S::HorizonCruiser => CardText::new("Horizon-Kreuzer", "Solide Werte, zuverlässig."),
            S::IronDreadnought => CardText::new(
                "Eisen-Dreadnought",
                "Schild 2: Absorbiert die ersten 2 Schaden vor dem Rumpf.",
            ),
            S::PlasmaFrigate => CardText::new(
                "Plasma-Fregatte",
                "Beim Deploy: 3 Schaden an ein zufälliges gegnerisches Schiff in derselben Lane.",
            ),
            S::RepairDrone => CardText::new(
                "Reparaturdrohne",
                "Am Ende jeder Runde: Repariert 2 HP beim beschädigten verbündeten Kampfschiff mit den wenigsten HP.",
            ),
            S::ScrapRaider => CardText::new(
                "Schrott-Raider",
                "Wird ein gegnerisches Schiff in derselben Lane zerstört, ist im nächsten Nachschub die Schrottverwertung: dauerhaft +1 ATK für einen Schrott-Raider.",
            ),
            S::SignalBooster => CardText::new(
                "Signalverstärker",
                "Alle anderen verbündeten Schiffe in derselben Lane haben +1 ATK, solange der Signalverstärker dort steht.",
            ),
            S::TitanCarrier => CardText::new(
                "Titan-Träger",
                "Beim Deploy: Erzeugt eine Drohne (1 HP, 1 ATK) in derselben Lane.",
            ),
            S::Drone => CardText::new(
                "Drohne",
                "Startet beim Deploy vom Titan-Träger, mit der Drohnenstaffel und am Rundenende von der Hive; liegt in keinem Deck.",
            ),
            S::VoidInterceptor => {
                CardText::new("Void-Abfangjäger", "Reiner Schaden, keine Tricks.")
            }
            S::Decoy => CardText::new(
                "Trugbild",
                "Hologramm von Admiral Moreau: steht eine Runde und greift nie an. Spaltenfeuer trifft zuerst das Trugbild, Schiffsfeuer auf das Schiff dahinter verpufft, solange es steht. Flaggschiffe durchschauen es; zerstört hinterlässt es keinen Schrott.",
            ),
            S::Bastion => CardText::formatted(
                "Bastion",
                format!(
                    "Nach den Manövern erhält sie +{} Schild für die Runde, das Schiff im Strike-Slot direkt vor ihr +{}.",
                    V.guardian_shield, V.guardian_shield_front
                ),
            ),
            S::Vanguard => CardText::formatted(
                "Vanguard",
                format!(
                    "Das Kampfschiff im Strike-Slot direkt vor ihr hat +{} ATK.",
                    V.spearhead
                ),
            ),
            S::Hive => CardText::formatted(
                "Hive",
                format!(
                    "Startet am Ende jeder Runde eine Drohne in den nächsten freien Strike-Slot — höchstens {} Drohnen gleichzeitig und {} pro Match.",
                    V.max_drones, V.hive_budget
                ),
            ),
            S::Mirage => CardText::new(
                "Mirage",
                "Ihr Grundmanöver darf auch auf einen belegten eigenen Slot gehen: Beide Schiffe tauschen die Plätze und feuern beide.",
            ),
            S::Pendulum => CardText::formatted(
                "Pendulum",
                format!(
                    "In einer Runde, in der ihre Railgun nicht feuert, darf sie ein gegnerisches Schiff erfassen. In der nächsten Runde trifft es die Railgun mit {}, wenn es ungedeckt ist. Bewegt sie sich in einer der beiden Runden, fällt der Schuss aus.",
                    V.railgun
                ),
            ),
        }
    }
    fn admiral(&self, admiral: AdmiralId) -> AdmiralText {
        let (name, epithet, ability) = match admiral {
            AdmiralId::Osei => ("Admiral Osei", "Die Mauer", "Wächterschild"),
            AdmiralId::Vance => ("Admiralin Vance", "Der Hammer", "Speerspitze"),
            AdmiralId::Halvorsen => ("Admiralin Halvorsen", "Der Schwarm", "Brutkammer"),
            AdmiralId::Moreau => ("Admiral Moreau", "Das Phantom", "Phasentausch"),
            AdmiralId::Kerr => ("Admiral Kerr", "Der Uhrmacher", "Railgun"),
        };
        AdmiralText {
            name,
            epithet,
            ability,
        }
    }
    fn power(&self, power: PowerId) -> CardText {
        match power {
            PowerId::Counterfire => CardText::formatted(
                "Gegenfeuer",
                format!(
                    "Wähle ein eigenes Schiff. Jedes gegnerische Schiff außer einem Flaggschiff, das es im Kampf trifft, nimmt danach je Treffer {} Schaden.",
                    V.counterfire
                ),
            ),
            PowerId::FocusFire => CardText::formatted(
                "Fokusfeuer",
                format!(
                    "Wähle ein gegnerisches Schiff, auch ein gedecktes. In dieser Runde macht jeder Treffer darauf im Kampf +{} Schaden.",
                    V.focus_fire
                ),
            ),
            PowerId::SwarmStrike => CardText::formatted(
                "Schwarmangriff",
                format!(
                    "Wähle ein gegnerisches Schiff außer dem Flaggschiff, auch ein gedecktes. Alle deine Drohnen feuern ohne Feuerbefehl darauf, jeder Treffer +{} Schaden.",
                    V.swarm_strike
                ),
            ),
            PowerId::Decoy => CardText::new(
                "Trugbild",
                "Wähle einen freien Strike-Slot. Dort steht diese Runde ein Trugbild (1 HP): Spaltenfeuer trifft zuerst das Trugbild, Schiffsfeuer auf das Schiff dahinter verpufft. Flaggschiffe durchschauen es.",
            ),
            PowerId::HeadStart => CardText::new(
                "Taktvorsprung",
                "Wähle ein eigenes Kampfschiff. Es feuert vor allen anderen, und was es zerstört, feuert nicht mehr.",
            ),
        }
    }
    fn power_target(&self, target: PowerTarget) -> &'static str {
        match target {
            PowerTarget::OwnShip => "Verbündeten wählen",
            PowerTarget::OwnCombatShip => "Kampfschiff wählen",
            PowerTarget::EnemyShip | PowerTarget::EnemyShipNotFlagship => "Gegner wählen",
            PowerTarget::FreeOwnStrikeSlot => "Strike-Slot wählen",
        }
    }
    fn command(&self, command: CommandId) -> CardText {
        use CommandId as C;
        let (name, effect) = match command {
            C::EmergencyShields => (
                "Notschilde",
                "Verbündetes Schiff erhält diese Runde Schild 3.",
            ),
            C::EmpBlast => (
                "EMP-Stoß",
                "Gegner mit höchster ATK feuert diese Runde nicht.",
            ),
            C::FlankingManeuver => (
                "Flankenmanöver",
                "Schiff wechselt in freien Slot der anderen Lane.",
            ),
            C::Intercept => (
                "Abfangen",
                "Command-Schiff rückt nach vorn, diese Runde +2 ATK.",
            ),
            C::OrbitalStrike => ("Orbitalschlag", "4 Schaden an Gegner mit den meisten HP."),
            C::Overcharge => ("Überladung", "Diese Runde +3 ATK, danach 2 Schaden."),
            C::PowerSurge => (
                "Energiestoß",
                "Eigene Strike-Schiffe erhalten diese Runde +1 ATK.",
            ),
            C::Recall => ("Rückruf", "Strike-Schiff rückt nach hinten, heilt 2 HP."),
            C::Scavenge => ("Plündern", "Ziehe sofort 1 Schiff und 1 Befehl."),
            C::TargetingJammer => (
                "Zielstörer",
                "Gegnerische Strike-Schiffe erhalten diese Runde -1 ATK.",
            ),
            C::VolleyFire => ("Salvenfeuer", "1 Schaden an ALLE gegnerischen Schiffe."),
            C::Torpedo => (
                "Torpedo",
                "Nach Bewegungen: 5 Schaden im Slot, trotz Deckung.",
            ),
            C::Barrage => ("Sperrfeuer", "3 Schaden an jeden Gegner mit Slotwechsel."),
            C::EvasiveAction => (
                "Ausweichmanöver",
                "Schiff wechselt in derselben Lane, erhält Schild 2.",
            ),
            C::MicroJump => (
                "Kurzsprung",
                "Verlässt bis Rundenende das Gefecht: unangreifbar, aber wirkungslos.",
            ),
            C::RapidDeployment => (
                "Schnellverlegung",
                "Du darfst diese Runde ein Schiff mehr legen.",
            ),
            C::DrawFire => (
                "Ablenkung",
                "Lenkt alle Feuerbefehle auf sich, solange ungedeckt.",
            ),
            C::ShieldGrid => (
                "Schildgitter",
                "Alle verbündeten Schiffe erhalten diese Runde Schild 1.",
            ),
            C::AblativeArmor => ("Ablativpanzerung", "Schiff erhält dauerhaft Schild 2."),
            C::CoveringFire => (
                "Feuerschutz",
                "Schiff und Vorder- oder Hintermann: +1 ATK, Schild 1.",
            ),
            C::DroneSquadron => (
                "Drohnenstaffel",
                "Zwei Drohnen starten und decken sofort ungedeckte Command-Schiffe.",
            ),
            C::CollisionCourse => (
                "Rammkurs",
                "Rammt Gegner gegenüber: Jeder nimmt die HP des anderen als Schaden. Feuert nicht.",
            ),
            C::Countermeasures => (
                "Gegenmaßnahmen",
                "Gegnerischer Angriffs-/Schwächungsbefehl verpufft, sonst ziehe einen Befehl.",
            ),
            C::CarteBlanche => (
                "Freie Hand",
                "Diese Runde Grundmanöver und Admiralsfähigkeit.",
            ),
            C::RadioSilence => (
                "Funkstille",
                "Gegnerische Admiralsfähigkeit verpufft, Trugbild löst sich auf.",
            ),
            C::Minefield => (
                "Minenfeld",
                "3 Schaden an jeden gegnerischen Neuankömmling.",
            ),
            C::TractorBeam => (
                "Traktorstrahl",
                "Zieht Gegner in andere Lane, gilt als bewegt.",
            ),
            C::HeatSeeker => ("Wärmesucher", "3 Schaden trotz Deckung, 5 bei Slotwechsel."),
            C::CoupDeGrace => (
                "Gnadenstoß",
                "4 Schaden trotz Deckung an Gegner mit wenigsten HP (keine Drohnen/Trugbilder).",
            ),
            C::PiercingShot => (
                "Durchschlag",
                "Nach Bewegungen: 3 Schaden vorn, 2 dahinter.",
            ),
            C::Bombardment => (
                "Bombardement",
                "2 Schaden an jedes Schiff der gegnerischen Command-Lane.",
            ),
            C::Crossfire => (
                "Kreuzfeuer",
                "3 Schaden an jeden Gegner allein in seiner Spalte.",
            ),
            C::ScrapSalvage => {
                return CardText {
                    target: Some("Eigener Schrott-Raider"),
                    ..CardText::new(
                        "Schrottverwertung",
                        "Der gewählte Schrott-Raider erhält dauerhaft +1 ATK.",
                    )
                };
            }
        };
        CardText::new(name, effect)
    }
}
