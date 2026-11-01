//! Funksprüche des Bordfunks ([`crate::audio::radio`]), erzeugt mit `tools/generateAudio.ts` (ElevenLabs
//! Text-to-Speech) als `audio/radio/<id>_<n>.mp3`, je Variante eine Datei. Der gesprochene Text ist zugleich der
//! englische Untertitel (i18n).
//!
//! Die Priorität entscheidet, wer bei belegtem Funk warten und sich wiederholen darf: 3 sind Verluste, Angriffe auf die
//! eigene Flotte und der Ausgang des Matches, 2 Meldungen über den Gegner und den Verlauf, 1 Bestätigungen eigener
//! Befehle und Fähigkeiten.

macro_rules! radio_cues {
    ($($cue:ident => $key:literal, $variants:literal, $priority:literal;)*) => {
        /// Ein Funkspruch.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RadioCue {
            $($cue,)*
        }

        impl RadioCue {
            pub const ALL: &[Self] = &[$(Self::$cue,)*];

            /// Dateiname ohne Variante, zugleich Schlüssel der Untertitel.
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$cue => $key,)*
                }
            }

            /// Anzahl der Aufnahmen; genauso viele Untertitel hat der Spruch.
            pub const fn variants(self) -> usize {
                match self {
                    $(Self::$cue => $variants,)*
                }
            }

            /// Priorität 1 (Bestätigung) bis 3 (Verlust, Angriff, Ausgang).
            pub const fn priority(self) -> u8 {
                match self {
                    $(Self::$cue => $priority,)*
                }
            }
        }
    };
}

radio_cues! {
    BattleStart => "battle_start", 2, 2;
    OrdersSent => "orders_sent", 3, 1;
    EnemyReady => "enemy_ready", 2, 2;
    Overdrive => "overdrive", 2, 2;
    OverdriveUp => "overdrive_up", 2, 2;
    LastShip => "last_ship", 2, 3;
    Victory => "victory", 2, 3;
    Defeat => "defeat", 2, 3;
    Draw => "draw", 1, 3;
    EnemySurrender => "enemy_surrender", 1, 3;
    Retreat => "retreat", 1, 3;
    TorpedoAway => "torpedo_away", 2, 1;
    TorpedoHit => "torpedo_hit", 2, 2;
    TorpedoMissed => "torpedo_missed", 2, 2;
    TorpedoIncoming => "torpedo_incoming", 2, 3;
    TorpedoDodged => "torpedo_dodged", 2, 2;
    BarrageHit => "barrage_hit", 2, 2;
    BarrageMissed => "barrage_missed", 1, 2;
    BarrageCaught => "barrage_caught", 2, 2;
    OrbitalStrike => "orbital_strike", 1, 1;
    OrbitalIncoming => "orbital_incoming", 1, 2;
    Volley => "volley", 1, 1;
    VolleyIncoming => "volley_incoming", 1, 2;
    EmpHit => "emp_hit", 1, 2;
    EmpIncoming => "emp_incoming", 1, 2;
    Jammer => "jammer", 1, 1;
    Jammed => "jammed", 1, 2;
    ShieldsUp => "shields_up", 2, 1;
    Overcharge => "overcharge", 1, 1;
    PowerSurge => "power_surge", 1, 1;
    Scavenge => "scavenge", 1, 1;
    Intercept => "intercept", 1, 1;
    Recall => "recall", 1, 1;
    Flanking => "flanking", 1, 1;
    DroneLaunch => "drone_launch", 1, 1;
    FrigateFire => "frigate_fire", 1, 1;
    Repair => "repair", 1, 1;
    Scrap => "scrap", 1, 1;
    EnemyDestroyed => "enemy_destroyed", 4, 2;
    ShipLost => "ship_lost", 3, 3;
    HullCritical => "hull_critical", 2, 2;
}

impl RadioCue {
    /// Asset-Pfade aller Aufnahmen.
    pub fn paths(self) -> impl Iterator<Item = String> {
        (1..=self.variants()).map(move |n| format!("audio/radio/{}_{n}.mp3", self.key()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jede Aufnahme des Katalogs liegt in `assets/audio/radio/`, und keine Datei dort fehlt im Katalog.
    #[test]
    fn catalog_matches_files() {
        let dir = std::path::Path::new(crate::REPO_ASSETS).join("audio/radio");
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .expect("Ordner des Bordfunks")
            .map(|entry| {
                format!(
                    "audio/radio/{}",
                    entry.unwrap().file_name().to_string_lossy()
                )
            })
            .collect();
        let mut catalog: Vec<String> = RadioCue::ALL.iter().flat_map(|cue| cue.paths()).collect();
        files.sort();
        catalog.sort();
        assert_eq!(files, catalog);
    }
}
