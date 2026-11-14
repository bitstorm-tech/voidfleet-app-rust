//! 3D-Modelle der Gegner und Flaggschiffe, gebaut mit `tools/ships/` in Blender (`models/ships/<datei>.glb`). Die
//! Gegner tragen vorerst die Rümpfe der alten Schiffskarten ([`model_key`]); Entwürfe ([`DRAFTS`]) haben noch keine
//! Rolle im Spiel und stehen nur in der Modellschau (`dev/viewer.rs`). Zu jedem Modell gehört ein Wrack aus
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

/// Was auf dem Brett steht: ein Gegner oder ein Flaggschiff des Spielers; dazu Entwürfe für die Modellschau.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShipKind {
    Enemy(EnemyId),
    Flagship(FlagshipId),
    /// Ein Modell ohne Rolle im Spiel, mit seinem Dateinamen ([`DRAFTS`]).
    Draft(&'static str),
}

impl ShipKind {
    /// Klasse für Flugbahn und Triebwerksklang; Flaggschiffe sind eine eigene.
    pub fn class(self) -> EnemyClass {
        match self {
            ShipKind::Enemy(enemy) => enemy.class(),
            ShipKind::Flagship(_) => EnemyClass::Flagship,
            // shortcut: Entwürfe zählen als Schlachtschiff (ein Modell, keine Staffel); ein Jäger oder Kreuzer als
            // Entwurf bräuchte seine Klasse in `DRAFTS`.
            ShipKind::Draft(_) => EnemyClass::Battleship,
        }
    }

    /// Flaggschiff: das eigene und das Siegel.
    pub fn is_flagship(self) -> bool {
        self.class() == EnemyClass::Flagship
    }
}

pub const ENEMIES: [EnemyId; 13] = {
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
        UnknownContact,
        Brood,
    ]
};

pub const FLAGSHIPS: [FlagshipId; 4] = [
    FlagshipId::Bastion,
    FlagshipId::Vanguard,
    FlagshipId::Pendulum,
    FlagshipId::Pyre,
];

/// Entwürfe: fertige Modelle, die noch keinem Gegner gehören.
pub const DRAFTS: [&str; 1] = ["broadside"];

/// Alles mit Modell: Gegner, Flaggschiffe und Entwürfe.
pub fn ships() -> impl Iterator<Item = ShipKind> {
    ENEMIES
        .into_iter()
        .map(ShipKind::Enemy)
        .chain(FLAGSHIPS.into_iter().map(ShipKind::Flagship))
        .chain(DRAFTS.into_iter().map(ShipKind::Draft))
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
            EnemyId::UnknownContact => "unknown_contact",
            EnemyId::Brood => "brood",
        },
        ShipKind::Flagship(flagship) => match flagship {
            FlagshipId::Bastion => "bastion",
            FlagshipId::Vanguard => "vanguard",
            FlagshipId::Pendulum => "pendulum",
            FlagshipId::Pyre => "pyre",
        },
        ShipKind::Draft(key) => key,
    }
}

/// Ein Punkt aus dem Bauskript eines Schiffs (`tools/ships/fleet/<datei>.py`: x steuerbord, y achtern, z oben) im Raum
/// seines Modells auf dem Brett (Nase nach −Z, siehe [`crate::scene::coords`]).
const fn gun(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(-x, z, y)
}

/// Die Mündungen der Geschütze eines Schiffs im Raum seines Modells: Dort beginnt ein Laserstrahl. Die Werte stammen
/// aus den Bauskripten und ändern sich mit dem Modell; eingetragen sind die Schiffe, die Laser feuern (Flaggschiffe
/// über Karten und Waffenstation, Gegner über Ziel- und Spaltenfeuer), die übrigen feuern aus ihrer Mitte.
pub fn muzzles(kind: ShipKind) -> &'static [Vec3] {
    match model_key(kind) {
        // Mittlere Linse der Emitterbänke an den Tortürmen
        "bastion" => const { &[gun(3.02, -3.27, 1.75), gun(-3.02, -3.27, 1.75)] },
        // Rohre des vorderen Zwillingsturms
        "vanguard" => const { &[gun(0.175, -1.06, 1.41), gun(-0.175, -1.06, 1.41)] },
        // Mündungsjoch der Railgun
        "pendulum" => const { &[gun(0.0, -5.84, 2.0)] },
        // Glutkugel des Plasmawerfers
        "pyre" => const { &[gun(0.0, -5.7, 0.25)] },
        // Bugkanonen unter den Flügelwurzeln
        "void_interceptor" => const { &[gun(0.42, -3.08, -0.2), gun(-0.42, -3.08, -0.2)] },
        "seal" => const { &[gun(0.8, -2.82, -0.17), gun(-0.8, -2.82, -0.17)] },
        // Rohre des Zwillingsturms am Bug
        "horizon_cruiser" => const { &[gun(0.16, -3.23, 0.64), gun(-0.16, -3.23, 0.64)] },
        // Flak am Deckrand beiderseits des Bugs
        "titan_carrier" => const { &[gun(1.63, -3.16, 0.85), gun(-1.63, -3.16, 0.85)] },
        // Beutekanone unter dem Bug
        "scrap_king" => const { &[gun(0.0, -5.22, -1.0)] },
        // Mündungen der beiden Belagerungskanonen am Bug
        "iron_dreadnought" => const { &[gun(0.52, -4.94, 0.36), gun(-0.52, -4.94, 0.36)] },
        // Kalter Kern in der Mündungsbremse des Hauptgeschützes
        "warden" => const { &[gun(0.0, -6.8, 0.5)] },
        // Spitze der Lanze (der Rumpf ist danach auf 0,92 skaliert)
        "lance_bomber" => const { &[gun(0.0, -3.3 * 0.92, -0.78 * 0.92)] },
        // Der Schlund zwischen den oberen und unteren Zangen (`MAW`)
        "unknown_contact" => const { &[gun(0.0, -5.25, -0.5)] },
        _ => &[],
    }
}

/// Ob ein Schiff die Lackierung der Gegner trägt ([`crate::scene::toon::material::livery`]): die Gegner mit den alten
/// Rümpfen, nicht die Bosse mit eigenen, nicht das Fremde und seine Brut und nicht die Flaggschiffe.
pub fn livery(kind: ShipKind) -> bool {
    use EnemyId::*;
    matches!(
        kind,
        ShipKind::Enemy(
            Raiders
                | Fireship
                | TorpedoWing
                | GuardCruiser
                | PlasmaCruiser
                | Spotter
                | Besieger
                | Carrier
        )
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Die acht alten Rümpfe tragen die Lackierung, Bosse, das Fremde und Flaggschiffe nicht.
    #[test]
    fn livery_only_for_old_hulls() {
        let painted: Vec<ShipKind> = ships().filter(|kind| livery(*kind)).collect();
        assert_eq!(painted.len(), 8);
        use EnemyId::*;
        for boss in [ScrapKing, Warden, Seal, UnknownContact, Brood] {
            assert!(!livery(ShipKind::Enemy(boss)));
        }
        assert!(
            FLAGSHIPS
                .into_iter()
                .all(|id| !livery(ShipKind::Flagship(id)))
        );
    }
}
