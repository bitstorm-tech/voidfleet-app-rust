//! Kataloge der Assets: welche Datei zu welcher Id gehört, und je Katalog eine Ressource mit den geladenen Handles.
//!
//! Karten-, Schiffs- und Admirals-Ids kommen später aus `voidfleet_core`; bis dahin nehmen die Zuordnungen einen
//! `&str`-Schlüssel bzw. ein eigenes kleines Enum (markiert mit `TODO(core)`). Sounds, Musik und Bordfunk gehören nur
//! dem Client und haben eigene Enums. Alle Kataloge laden beim Start und melden ihre Handles bei
//! [`crate::assets::Preload`] an; der Ladebildschirm wartet auf sie.

pub mod admirals;
pub mod backgrounds;
pub mod card_art;
pub mod radio;
pub mod ship_models;
pub mod sounds;

use bevy::prelude::*;

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

    use super::*;

    fn exists(path: &str) -> bool {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"))
            .join(path)
            .is_file()
    }

    /// Jede Datei, die ein Katalog nennt, liegt unter `assets/`.
    #[test]
    fn catalog_files_exist() {
        let paths = admirals::AdmiralId::ALL
            .into_iter()
            .map(admirals::portrait_path)
            .chain(
                backgrounds::BACKGROUNDS
                    .into_iter()
                    .map(backgrounds::background_path),
            )
            .chain(card_art::CARDS.into_iter().map(card_art::card_art_path))
            .chain(
                ship_models::SHIPS
                    .into_iter()
                    .map(ship_models::ship_model_path),
            )
            .chain(
                ship_models::SHIPS
                    .into_iter()
                    .map(ship_models::wreck_model_path),
            )
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
