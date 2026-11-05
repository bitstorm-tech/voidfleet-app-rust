//! Funksprüche des Bordfunks ([`crate::audio::radio`]), erzeugt mit `tools/generateAudio.ts` (ElevenLabs
//! Text-to-Speech) als `audio/radio/<id>_<n>.mp3`, je Variante eine Datei. Der gesprochene Text ist zugleich der
//! englische Untertitel (i18n).
//!
//! Die Priorität entscheidet, wer bei belegtem Funk warten und sich wiederholen darf: 3 ist der Ausgang des Gefechts, 2
//! Meldungen über den Verlauf.

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
    Victory => "victory", 2, 3;
    Defeat => "defeat", 2, 3;
    EnemyDestroyed => "enemy_destroyed", 4, 2;
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
