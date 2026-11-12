//! Die Admirale im Client: ihre Porträts (`images/admirals/<id>.jpg`) für Admiralswahl, Kopfzeile und Banner. Regeln und Zahlen stehen im
//! Regelwerk, die Akzentfarben in [`crate::palette`].

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId, FlagshipId};

use super::FileKey;
use crate::assets::{Preload, gpu_image};

/// Asset-Pfad des Porträts.
pub fn portrait_path(admiral: AdmiralId) -> String {
    format!("images/admirals/{}.jpg", admiral.file_key())
}

/// Der Admiral, dem das Flaggschiff gehört.
pub fn flagship_admiral(flagship: FlagshipId) -> AdmiralId {
    ADMIRAL_IDS
        .into_iter()
        .find(|admiral| admiral.flagship() == flagship)
        .expect("jedes Flaggschiff gehört einem Admiral")
}

/// Die Porträts aller Admirale, beim Start geladen.
#[derive(Resource)]
pub struct AdmiralPortraits(HashMap<AdmiralId, Handle<Image>>);

impl AdmiralPortraits {
    pub fn get(&self, admiral: AdmiralId) -> Handle<Image> {
        self.0[&admiral].clone()
    }
}

impl FromWorld for AdmiralPortraits {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>().clone();
        let mut preload = world.resource_mut::<Preload>();
        Self(
            ADMIRAL_IDS
                .into_iter()
                .map(|admiral| {
                    (
                        admiral,
                        preload.track(gpu_image(&server, portrait_path(admiral))),
                    )
                })
                .collect(),
        )
    }
}
