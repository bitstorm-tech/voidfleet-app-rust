//! Die Admirale im Client: ihre Porträts (`images/admirals/<id>.jpg`) für Auswahlkarten, Fähigkeitskarte, Banner und
//! Gegner-Feld. Regeln und Zahlen stehen im Regelwerk, die Akzentfarben in [`crate::palette`].

use bevy::{platform::collections::HashMap, prelude::*};

use crate::assets::Preload;

/// Die fünf Admirale.
///
/// TODO(core): gegen `voidfleet_core::admirals::AdmiralId` tauschen, sobald das Regelwerk portiert ist; dann
/// wandern auch `flagship_admiral` und `power_admiral` aus `data/admirals.ts` hierher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdmiralId {
    Osei,
    Vance,
    Halvorsen,
    Moreau,
    Kerr,
}

impl AdmiralId {
    pub const ALL: [Self; 5] = [
        Self::Osei,
        Self::Vance,
        Self::Halvorsen,
        Self::Moreau,
        Self::Kerr,
    ];

    /// Schlüssel wie im Regelwerk und in den Dateinamen.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Osei => "osei",
            Self::Vance => "vance",
            Self::Halvorsen => "halvorsen",
            Self::Moreau => "moreau",
            Self::Kerr => "kerr",
        }
    }
}

/// Asset-Pfad des Porträts.
pub fn portrait_path(admiral: AdmiralId) -> String {
    format!("images/admirals/{}.jpg", admiral.key())
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
    let portraits = AdmiralId::ALL
        .into_iter()
        .map(|admiral| (admiral, preload.track(server.load(portrait_path(admiral)))))
        .collect();
    commands.insert_resource(AdmiralPortraits(portraits));
}
