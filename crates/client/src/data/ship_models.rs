//! 3D-Modelle der Gegner und Flaggschiffe, gebaut mit `tools/ships/` in Blender (`models/ships/<datei>.glb`). Die
//! Gegner tragen vorerst die Rümpfe der alten Schiffskarten ([`model_key`]). Zu jedem Modell gehört ein Wrack aus
//! Bruchstücken (`<datei>_wreck.glb`) im selben Modellraum, das bei der Zerstörung an die Stelle des Schiffs tritt.
//!
//! Geladen wird die erste Szene jeder Datei; ihre Materialien sind beim Laden schon Toon-Materialien
//! ([`crate::scene::toon`]).

use bevy::{
    gltf::GltfAssetLabel, platform::collections::HashMap, prelude::*,
    world_serialization::WorldAsset,
};
use voidfleet_core::{
    admirals::FlagshipId,
    enemies::{EnemyClass, EnemyId},
};

use crate::assets::Preload;

/// Was auf dem Brett steht: ein Gegner oder ein Flaggschiff des Spielers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShipKind {
    Enemy(EnemyId),
    Flagship(FlagshipId),
}

impl ShipKind {
    /// Klasse für Flugbahn und Triebwerksklang; Flaggschiffe sind eine eigene.
    pub fn class(self) -> EnemyClass {
        match self {
            ShipKind::Enemy(enemy) => enemy.class(),
            ShipKind::Flagship(_) => EnemyClass::Flagship,
        }
    }

    /// Flaggschiffe tragen eine Krone: das eigene und das Siegel.
    pub fn is_flagship(self) -> bool {
        self.class() == EnemyClass::Flagship
    }
}

pub const ENEMIES: [EnemyId; 11] = {
    use EnemyId::*;
    [
        Raiders,
        Fireship,
        TorpedoWing,
        GuardCruiser,
        PlasmaCruiser,
        Spotter,
        Besieger,
        Carrier,
        ScrapKing,
        Warden,
        Seal,
    ]
};

pub const FLAGSHIPS: [FlagshipId; 4] = [
    FlagshipId::Bastion,
    FlagshipId::Vanguard,
    FlagshipId::Pendulum,
    FlagshipId::Pyre,
];

/// Alles mit Modell: Gegner und Flaggschiffe.
pub fn ships() -> impl Iterator<Item = ShipKind> {
    ENEMIES
        .into_iter()
        .map(ShipKind::Enemy)
        .chain(FLAGSHIPS.into_iter().map(ShipKind::Flagship))
}

/// Der Dateiname des Modells.
pub fn model_key(kind: ShipKind) -> &'static str {
    match kind {
        ShipKind::Enemy(enemy) => match enemy {
            EnemyId::Raiders => "void_interceptor",
            EnemyId::Fireship => "scrap_raider",
            EnemyId::TorpedoWing => "lance_bomber",
            EnemyId::GuardCruiser => "horizon_cruiser",
            EnemyId::PlasmaCruiser => "plasma_frigate",
            EnemyId::Spotter => "signal_booster",
            EnemyId::Besieger => "iron_dreadnought",
            EnemyId::Carrier => "titan_carrier",
            EnemyId::ScrapKing => "scrap_king",
            EnemyId::Warden => "warden",
            EnemyId::Seal => "seal",
        },
        ShipKind::Flagship(flagship) => match flagship {
            FlagshipId::Bastion => "bastion",
            FlagshipId::Vanguard => "vanguard",
            FlagshipId::Pendulum => "pendulum",
            FlagshipId::Pyre => "pyre",
        },
    }
}

/// Asset-Pfad des Modells.
pub fn ship_model_path(kind: ShipKind) -> String {
    format!("models/ships/{}.glb", model_key(kind))
}

/// Asset-Pfad der Bruchstücke.
pub fn wreck_model_path(kind: ShipKind) -> String {
    format!("models/ships/{}_wreck.glb", model_key(kind))
}

/// Szene eines Schiffs und seines Wracks.
#[derive(Clone, Debug)]
pub struct ShipModel {
    pub ship: Handle<WorldAsset>,
    pub wreck: Handle<WorldAsset>,
}

/// Modelle und Wracks aller Schiffe, beim Start geladen.
#[derive(Resource)]
pub struct ShipModels(HashMap<ShipKind, ShipModel>);

impl ShipModels {
    /// Modell und Wrack eines Schiffs.
    pub fn get(&self, kind: ShipKind) -> &ShipModel {
        &self.0[&kind]
    }

    pub fn iter(&self) -> impl Iterator<Item = (ShipKind, &ShipModel)> {
        self.0.iter().map(|(kind, model)| (*kind, model))
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
                .map(|kind| {
                    let model = ShipModel {
                        ship: scene(ship_model_path(kind)),
                        wreck: scene(wreck_model_path(kind)),
                    };
                    (kind, model)
                })
                .collect(),
        )
    }
}
