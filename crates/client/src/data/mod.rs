//! Kataloge der Assets: welche Datei zu welcher Id gehört, und je Katalog eine Ressource mit den geladenen Handles.
//!
//! Karten-, Schiffs- und Admirals-Ids sind die Enums aus `voidfleet_core`; ihr Dateiname ist [`FileKey::file_key`].
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
    admirals::AdmiralId,
    cards::{CommandId, ShipId},
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
            Self::Halvorsen => "halvorsen",
            Self::Moreau => "moreau",
            Self::Kerr => "kerr",
        }
    }
}

impl FileKey for ShipId {
    fn file_key(self) -> &'static str {
        match self {
            Self::HorizonCruiser => "horizon_cruiser",
            Self::IronDreadnought => "iron_dreadnought",
            Self::PlasmaFrigate => "plasma_frigate",
            Self::RepairDrone => "repair_drone",
            Self::ScrapRaider => "scrap_raider",
            Self::SignalBooster => "signal_booster",
            Self::TitanCarrier => "titan_carrier",
            Self::VoidInterceptor => "void_interceptor",
            Self::Drone => "drone",
            Self::Decoy => "decoy",
            Self::Bastion => "bastion",
            Self::Vanguard => "vanguard",
            Self::Hive => "hive",
            Self::Mirage => "mirage",
            Self::Pendulum => "pendulum",
        }
    }
}

impl FileKey for CommandId {
    fn file_key(self) -> &'static str {
        match self {
            Self::EmergencyShields => "emergency_shields",
            Self::EmpBlast => "emp_blast",
            Self::FlankingManeuver => "flanking_maneuver",
            Self::Intercept => "intercept",
            Self::OrbitalStrike => "orbital_strike",
            Self::Overcharge => "overcharge",
            Self::PowerSurge => "power_surge",
            Self::Recall => "recall",
            Self::Scavenge => "scavenge",
            Self::TargetingJammer => "targeting_jammer",
            Self::VolleyFire => "volley_fire",
            Self::Torpedo => "torpedo",
            Self::Barrage => "barrage",
            Self::EvasiveAction => "evasive_action",
            Self::MicroJump => "micro_jump",
            Self::RapidDeployment => "rapid_deployment",
            Self::DrawFire => "draw_fire",
            Self::ShieldGrid => "shield_grid",
            Self::AblativeArmor => "ablative_armor",
            Self::CoveringFire => "covering_fire",
            Self::DroneSquadron => "drone_squadron",
            Self::CollisionCourse => "collision_course",
            Self::Countermeasures => "countermeasures",
            Self::CarteBlanche => "carte_blanche",
            Self::RadioSilence => "radio_silence",
            Self::Minefield => "minefield",
            Self::TractorBeam => "tractor_beam",
            Self::HeatSeeker => "heat_seeker",
            Self::CoupDeGrace => "coup_de_grace",
            Self::PiercingShot => "piercing_shot",
            Self::Bombardment => "bombardment",
            Self::Crossfire => "crossfire",
            Self::ScrapSalvage => "scrap_salvage",
        }
    }
}

/// Lädt die Bild- und Modell-Kataloge (die Audio-Kataloge lädt [`crate::audio`] mit eigenen Asset-Typen).
pub(crate) struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                admirals::load,
                backgrounds::load,
                card_art::load,
                ship_models::load,
            ),
        );
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
        ship_models::ships().for_each(same_as_serde);
        voidfleet_core::cards::COMMANDS
            .into_iter()
            .for_each(same_as_serde);
    }

    fn exists(path: &str) -> bool {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
            .join(path)
            .is_file()
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
