//! Die Admirale im Client: ihre Porträts (`images/admirals/<id>.jpg`) für Auswahlkarten, Fähigkeitskarte, Banner und
//! Gegner-Feld, und welcher Admiral zu einem Flaggschiff bzw. einer Fähigkeit gehört. Regeln und Zahlen stehen im
//! Regelwerk, die Akzentfarben in [`crate::palette`].

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::{
    admirals::{ADMIRAL_IDS, AdmiralId, PowerId},
    cards::ShipId,
};

use super::FileKey;
use crate::assets::Preload;

/// Asset-Pfad des Porträts.
pub fn portrait_path(admiral: AdmiralId) -> String {
    format!("images/admirals/{}.jpg", admiral.file_key())
}

/// Der Admiral, dem das Flaggschiff gehört; `None` für alle anderen Schiffe.
pub fn flagship_admiral(ship: ShipId) -> Option<AdmiralId> {
    ADMIRAL_IDS
        .into_iter()
        .find(|admiral| admiral.flagship() == ship)
}

/// Der Admiral mit dieser Fähigkeit.
pub fn power_admiral(power: PowerId) -> AdmiralId {
    ADMIRAL_IDS
        .into_iter()
        .find(|admiral| admiral.power() == power)
        .expect("jede Fähigkeit gehört einem Admiral")
}

/// Die Porträts aller Admirale, beim Start geladen.
#[derive(Resource)]
pub struct AdmiralPortraits(HashMap<AdmiralId, Handle<Image>>);

impl AdmiralPortraits {
    pub fn get(&self, admiral: AdmiralId) -> Handle<Image> {
        self.0[&admiral].clone()
    }
}

pub(super) fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let portraits = ADMIRAL_IDS
        .into_iter()
        .map(|admiral| (admiral, preload.track(server.load(portrait_path(admiral)))))
        .collect();
    commands.insert_resource(AdmiralPortraits(portraits));
}
