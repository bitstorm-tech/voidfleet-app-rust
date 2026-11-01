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

use crate::assets::Preload;

/// Alle Schiffe mit Modell.
///
/// TODO(core): aus den Kartendefinitionen von `voidfleet_core` ableiten.
pub const SHIPS: [&str; 15] = [
    "drone",
    "horizon_cruiser",
    "iron_dreadnought",
    "plasma_frigate",
    "repair_drone",
    "scrap_raider",
    "signal_booster",
    "titan_carrier",
    "void_interceptor",
    "decoy",
    "bastion",
    "vanguard",
    "hive",
    "mirage",
    "pendulum",
];

/// Die Flaggschiffe der fünf Admirale; sie stehen etwas größer auf dem Brett.
///
/// TODO(core): `voidfleet_core` sagt, ob eine Karte ein Flaggschiff ist.
pub fn is_flagship(ship: &str) -> bool {
    matches!(
        ship,
        "bastion" | "vanguard" | "hive" | "mirage" | "pendulum"
    )
}

/// Asset-Pfad des Modells.
pub fn ship_model_path(ship: &str) -> String {
    format!("models/ships/{ship}.glb")
}

/// Asset-Pfad der Bruchstücke.
pub fn wreck_model_path(ship: &str) -> String {
    format!("models/ships/{ship}_wreck.glb")
}

/// Szene eines Schiffs und seines Wracks.
#[derive(Clone, Debug)]
pub struct ShipModel {
    pub ship: Handle<WorldAsset>,
    pub wreck: Handle<WorldAsset>,
}

/// Modelle und Wracks aller Schiffe, beim Start geladen.
#[derive(Resource)]
pub struct ShipModels(HashMap<&'static str, ShipModel>);

impl ShipModels {
    /// Modell und Wrack eines Schiffs (TODO(core): Karten-Id statt `&str`).
    pub fn get(&self, ship: &str) -> &ShipModel {
        &self.0[ship]
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &ShipModel)> {
        self.0.iter().map(|(id, model)| (*id, model))
    }
}

pub(super) fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let mut scene =
        |path: String| preload.track(server.load(GltfAssetLabel::Scene(0).from_asset(path)));
    let models = SHIPS
        .into_iter()
        .map(|ship| {
            let model = ShipModel {
                ship: scene(ship_model_path(ship)),
                wreck: scene(wreck_model_path(ship)),
            };
            (ship, model)
        })
        .collect();
    commands.insert_resource(ShipModels(models));
}
