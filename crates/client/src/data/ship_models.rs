//! 3D-Modelle der Schiffskarten samt Flaggschiffen und Tokens (Drohne, Trugbild), gebaut mit `tools/ships/` in Blender
//! (`models/ships/<id>.glb`). Zu jedem Schiff gehört ein Wrack aus Bruchstücken (`<id>_wreck.glb`) im selben
//! Modellraum, das bei der Zerstörung an die Stelle des Schiffs tritt.
//!
//! Geladen wird die erste Szene jeder Datei; ihre Materialien sind beim Laden schon Toon-Materialien
//! ([`crate::scene::toon`]).

use bevy::{
    gltf::GltfAssetLabel, platform::collections::HashMap, prelude::*,
    world_serialization::WorldAsset,
};

use voidfleet_core::cards::{FLAGSHIPS, SHIPS, ShipId, TOKENS};

use super::FileKey;
use crate::assets::Preload;

/// Alle Schiffe mit Modell: Deck-Schiffe, Tokens und Flaggschiffe.
pub fn ships() -> impl Iterator<Item = ShipId> {
    SHIPS.into_iter().chain(TOKENS).chain(FLAGSHIPS)
}

/// Asset-Pfad des Modells.
pub fn ship_model_path(ship: ShipId) -> String {
    format!("models/ships/{}.glb", ship.file_key())
}

/// Asset-Pfad der Bruchstücke.
pub fn wreck_model_path(ship: ShipId) -> String {
    format!("models/ships/{}_wreck.glb", ship.file_key())
}

/// Szene eines Schiffs und seines Wracks.
#[derive(Clone, Debug)]
pub struct ShipModel {
    pub ship: Handle<WorldAsset>,
    pub wreck: Handle<WorldAsset>,
}

/// Modelle und Wracks aller Schiffe, beim Start geladen.
#[derive(Resource)]
pub struct ShipModels(HashMap<ShipId, ShipModel>);

impl ShipModels {
    /// Modell und Wrack eines Schiffs.
    pub fn get(&self, ship: ShipId) -> &ShipModel {
        &self.0[&ship]
    }

    pub fn iter(&self) -> impl Iterator<Item = (ShipId, &ShipModel)> {
        self.0.iter().map(|(id, model)| (*id, model))
    }
}

impl FromWorld for ShipModels {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>().clone();
        let mut preload = world.resource_mut::<Preload>();
        let mut scene =
            |path: String| preload.track(server.load(GltfAssetLabel::Scene(0).from_asset(path)));
        Self(
            ships()
                .map(|ship| {
                    let model = ShipModel {
                        ship: scene(ship_model_path(ship)),
                        wreck: scene(wreck_model_path(ship)),
                    };
                    (ship, model)
                })
                .collect(),
        )
    }
}
