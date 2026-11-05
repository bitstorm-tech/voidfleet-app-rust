//! Funksprüche des Bordfunks ([`crate::audio::radio`]) als `audio/radio/<id>_<n>.mp3`, je Variante eine Datei, trocken
//! aufgenommen (die Funkstrecke legt der Client darüber). Die ersten fünf stammen aus ElevenLabs, die übrigen aus
//! ElevenLabs v3 über fal.ai (`fal-ai/elevenlabs/tts/eleven-v3`) mit drei festen Stimmen: Brücke „Roger“,
//! Geschützleitung „Callum“, abgehörter Gegnerfunk „Bill“. Der gesprochene Text ist zugleich der englische Untertitel
//! (i18n).
//!
//! Die Priorität entscheidet, wer bei belegtem Funk warten und sich wiederholen darf: 3 sind Ausgang des Gefechts und
//! Gefahr fürs Flaggschiff, 2 Meldungen über den Verlauf, 1 Bestätigungen der Geschützleitung.

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
    HullCritical => "hull_critical", 2, 3;
    // Brücke
    IncomingFire => "incoming_fire", 2, 2;
    ColumnFireIncoming => "column_fire_incoming", 2, 3;
    FireOnboard => "fire_onboard", 2, 3;
    FireOut => "fire_out", 2, 2;
    ShieldsUp => "shields_up", 2, 2;
    Maneuver => "maneuver", 2, 2;
    MicroJump => "micro_jump", 2, 2;
    JumpReturn => "jump_return", 2, 2;
    RamIncoming => "ram_incoming", 2, 3;
    ReinforcementsSighted => "reinforcements_sighted", 2, 2;
    EmergencyGenerator => "emergency_generator", 2, 3;
    Repairs => "repairs", 2, 2;
    // Geschützleitung
    TargetLocked => "target_locked", 2, 1;
    DirectHit => "direct_hit", 2, 1;
    EnemyBurning => "enemy_burning", 2, 2;
    TorpedoAway => "torpedo_away", 2, 1;
    OrbitalStrike => "orbital_strike", 2, 1;
    RailgunFire => "railgun_fire", 2, 1;
    PlasmaFire => "plasma_fire", 2, 1;
    Counterfire => "counterfire", 2, 1;
    // Gegnerfunk, abgehört
    EnemyPanic => "enemy_panic", 2, 2;
    EnemyCharging => "enemy_charging", 2, 2;
    EnemyCallsReinforcements => "enemy_calls_reinforcements", 2, 2;
    EnemyBreaking => "enemy_breaking", 2, 2;
}

impl RadioCue {
    /// Abgehörter Gegnerfunk: fremde Stimme, stärker gestört.
    pub const fn intercepted(self) -> bool {
        matches!(
            self,
            Self::EnemyPanic
                | Self::EnemyCharging
                | Self::EnemyCallsReinforcements
                | Self::EnemyBreaking
        )
    }

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
