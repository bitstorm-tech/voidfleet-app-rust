//! Kataloge der Assets: welche Datei zu welcher Id gehört, und je Katalog eine Ressource mit den geladenen Handles.
//!
//! Karten-, Gegner-, Flaggschiff- und Admirals-Ids sind die Enums aus `voidfleet_core`; ihr Dateiname ist [`FileKey::file_key`].
//! Sounds, Musik und Bordfunk gehören nur dem Client und haben eigene Enums. Alle Kataloge laden beim Start und melden
//! ihre Handles bei [`crate::assets::Preload`] an; der Ladebildschirm wartet auf sie.

pub mod admirals;
pub mod backgrounds;
pub mod card_art;
pub mod radio;
pub mod ship_models;
pub mod sounds;

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, FlagshipId},
    cards::CardId,
    enemies::EnemyId,
};

/// Der Name einer Id in Dateinamen der Assets, wie im Regelwerk (`snake_case`).
pub trait FileKey: Copy {
    fn file_key(self) -> &'static str;
}

impl FileKey for AdmiralId {
    fn file_key(self) -> &'static str {
        match self {
            Self::Osei => "osei",
            Self::Vance => "vance",
            Self::Kerr => "kerr",
            Self::Reyes => "reyes",
        }
    }
}

impl FileKey for FlagshipId {
    fn file_key(self) -> &'static str {
        match self {
            Self::Bastion => "bastion",
            Self::Vanguard => "vanguard",
            Self::Pendulum => "pendulum",
            Self::Pyre => "pyre",
        }
    }
}

impl FileKey for EnemyId {
    fn file_key(self) -> &'static str {
        match self {
            Self::Raiders => "raiders",
            Self::Fireship => "fireship",
            Self::TorpedoWing => "torpedo_wing",
            Self::GuardCruiser => "guard_cruiser",
            Self::PlasmaCruiser => "plasma_cruiser",
            Self::Spotter => "spotter",
            Self::Besieger => "besieger",
            Self::Carrier => "carrier",
            Self::ScrapKing => "scrap_king",
            Self::Warden => "warden",
            Self::Seal => "seal",
            Self::UnknownContact => "unknown_contact",
            Self::Brood => "brood",
        }
    }
}

impl FileKey for CardId {
    fn file_key(self) -> &'static str {
        match self {
            Self::Torpedo => "torpedo",
            Self::OrbitalStrike => "orbital_strike",
            Self::VolleyFire => "volley_fire",
            Self::Barrage => "barrage",
            Self::HeatSeeker => "heat_seeker",
            Self::CoupDeGrace => "coup_de_grace",
            Self::Bombardment => "bombardment",
            Self::CollisionCourse => "collision_course",
            Self::Firebomb => "firebomb",
            Self::EmergencyShields => "emergency_shields",
            Self::HoldTheLine => "hold_the_line",
            Self::AblativeArmor => "ablative_armor",
            Self::Overcharge => "overcharge",
            Self::PowerSurge => "power_surge",
            Self::RepairCrew => "repair_crew",
            Self::EmpBlast => "emp_blast",
            Self::TargetingJammer => "targeting_jammer",
            Self::RadioSilence => "radio_silence",
            Self::TractorBeam => "tractor_beam",
            Self::MicroJump => "micro_jump",
            Self::EvasiveAction => "evasive_action",
            Self::Countermeasures => "countermeasures",
            Self::Scavenge => "scavenge",
            Self::RedAlert => "red_alert",
            Self::FireControlDoctrine => "fire_control_doctrine",
            Self::BattleStations => "battle_stations",
        }
    }
}

/// Lädt die Bild- und Modell-Kataloge (die Audio-Kataloge lädt [`crate::audio`] mit eigenen Asset-Typen); jeder
/// entsteht als `FromWorld` aus `AssetServer` und [`crate::assets::Preload`].
pub(crate) struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<admirals::AdmiralPortraits>()
            .init_resource::<backgrounds::Backgrounds>()
            .init_resource::<card_art::CardArt>()
            .init_resource::<ship_models::ShipModels>();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde::Serialize;

    use super::*;

    /// Der Dateiname ist der Name im Protokoll.
    fn same_as_serde(id: impl FileKey + Serialize) {
        assert_eq!(serde_json::to_value(id).unwrap(), id.file_key());
    }

    #[test]
    fn file_keys_match_protocol() {
        voidfleet_core::admirals::ADMIRAL_IDS
            .into_iter()
            .for_each(same_as_serde);
        voidfleet_core::cards::LOOT_POOL
            .into_iter()
            .for_each(same_as_serde);
        ship_models::ENEMIES.into_iter().for_each(same_as_serde);
        ship_models::FLAGSHIPS.into_iter().for_each(same_as_serde);
    }

    fn exists(path: &str) -> bool {
        Path::new(crate::REPO_ASSETS).join(path).is_file()
    }

    /// Jede Datei, die ein Katalog nennt, liegt unter `assets/`.
    #[test]
    fn catalog_files_exist() {
        let paths = voidfleet_core::admirals::ADMIRAL_IDS
            .into_iter()
            .map(admirals::portrait_path)
            .chain(
                backgrounds::BACKGROUNDS
                    .into_iter()
                    .map(backgrounds::background_path),
            )
            .chain(card_art::cards().map(card_art::card_art_path))
            .chain(ship_models::ships().map(ship_models::ship_model_path))
            .chain(ship_models::ships().map(ship_models::wreck_model_path))
            .chain(
                sounds::SoundId::ALL
                    .into_iter()
                    .flat_map(sounds::SoundId::paths),
            )
            .chain(
                sounds::MusicMood::ALL
                    .into_iter()
                    .map(|mood| mood.path().to_owned()),
            );
        let missing: Vec<String> = paths.filter(|path| !exists(path)).collect();
        assert!(missing.is_empty(), "fehlende Assets: {missing:?}");
    }
}
