//! Schiffe und ihre Wracks (Grundlagen aus `scene/ship.ts`): Modell spawnen, Toon-Zustand je Schiff, Tausch gegen die
//! Bruchstücke.
//!
//! Ein Schiff ist eine Entity mit [`Ship`]; seine Nase zeigt nach `Transform::forward` (−Z), gedreht wird allein diese
//! Wurzel. Darunter hängt das Modell ([`ShipModel`]) als glTF-Szene. Sobald sie steht, bekommt das Schiff eigene Kopien
//! seiner Toon-Materialien, denn Seite, Glut und Lichter ([`ToonState`]) gelten je Schiff. [`BreakApart`] tauscht das Modell gegen die Bruchstücke, die exakt an seiner Stelle liegen, und meldet
//! sie mit [`WreckReady`]: je Stück eine Entity mit dem Ursprung im Schwerpunkt.
//!
//! ```ignore
//! let ship = spawn_ship(&mut commands, &models, ShipKind::Enemy(EnemyId::PlasmaCruiser));
//! commands.entity(ship).insert((
//!     Transform::from_translation(pad).looking_to(Vec3::X, Vec3::Y).with_scale(Vec3::splat(BOARD_SCALE)),
//!     ToonState::facing(Side::Player),
//! ));
//! ```

use bevy::{
    picking::Pickable,
    platform::collections::HashMap,
    prelude::*,
    world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady},
};

use super::{
    coords::{Side, facing},
    markers::ShipMarkers,
    toon::ToonMaterial,
};
use crate::data::ship_models::{ShipKind, ShipModels};

/// Schiffe sind für den Hangar modelliert und auf dem Brett zu groß: Maßstab der Wurzel …
pub const BOARD_SCALE: f32 = 0.6;
/// … und des Modells darunter.
pub const MODEL_SCALE: f32 = 0.5;
/// Flaggschiffe heben sich durch ihre Größe ab: Ihre Modelle sind schon gut ein Fünftel größer gebaut, dazu dieser
/// Faktor. Die Bastion, das breiteste, bleibt so knapp 2 breit (Slots liegen 2,2 auseinander).
pub const FLAGSHIP_SCALE: f32 = 1.1;

/// Ein Schiff auf dem Brett: ein Gegner (`uid` aus dem Gefecht) oder das Flaggschiff des Spielers (`uid` 0).
#[derive(Component, Clone, Debug)]
#[require(
    Transform::from_scale(Vec3::splat(BOARD_SCALE)),
    Visibility,
    ToonState,
    ShipMaterials,
    ShipMarkers
)]
pub struct Ship {
    pub kind: ShipKind,
    pub uid: u32,
}

/// Was Modell und Wrack eines Schiffs beim Zeichnen lesen: die Seite für das Hauptlicht (setzt der Landeplatz), die
/// Glut der Bruchflächen und die Helligkeit der Lichter (beides steuert die Zerstörung).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ToonState {
    /// +1 eigene Seite (Nase nach +x), −1 Gegner.
    pub facing: f32,
    /// Glut der Bruchflächen (linear); `None`: die Farbe aus dem Material-Vertrag.
    pub heat: Option<LinearRgba>,
    /// Helligkeit der Lichter: 1 an, 0 erloschen.
    pub lights: f32,
    /// Überlagerung in einer Farbe (rgb linear, a Anteil): Treffer-Blitz, Glühen beim Warp, Verdunkeln der Trümmer.
    pub overlay: Vec4,
    /// Faktor auf die Deckkraft (Flackern des Trugbilds).
    pub opacity: f32,
}

impl Default for ToonState {
    fn default() -> Self {
        Self {
            facing: 1.0,
            heat: None,
            lights: 1.0,
            overlay: Vec4::ZERO,
            opacity: 1.0,
        }
    }
}

impl ToonState {
    /// Zustand eines Schiffs auf dieser Seite.
    pub fn facing(side: Side) -> Self {
        Self {
            facing: facing(side),
            ..default()
        }
    }

    fn apply(&self, material: &mut ToonMaterial) {
        material.facing = self.facing;
        material.heat = self.heat;
        material.lights = self.lights;
        material.overlay = self.overlay;
        material.opacity = self.opacity;
    }
}

/// Die eigenen Toon-Materialien eines Schiffs, Modell und Wrack zusammen.
#[derive(Component, Default, Debug)]
pub struct ShipMaterials(Vec<Handle<ToonMaterial>>);

/// Träger der glTF-Szene unter dem Schiff: erst das Modell, nach [`BreakApart`] das Wrack.
#[derive(Component, Debug)]
pub struct ShipModel;

/// Markiert einen [`ShipModel`], der die Bruchstücke trägt.
#[derive(Component, Debug)]
pub struct Wreck;

/// Die Szene unter einem [`ShipModel`] steht und hat ihre Materialien; ab dem nächsten Bild sind Lage und Grenzen
/// ihrer Teile gerechnet.
#[derive(Component, Debug)]
pub struct ModelReady;

/// Tauscht das Modell eines Schiffs gegen seine Bruchstücke.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct BreakApart {
    /// Das Schiff.
    pub entity: Entity,
}

/// Die Bruchstücke eines Schiffs stehen, exakt an der Stelle des Modells; je Stück eine Entity mit dem Ursprung im
/// Schwerpunkt, frei zu bewegen.
#[derive(EntityEvent, Clone, Debug)]
pub struct WreckReady {
    /// Das Schiff.
    pub entity: Entity,
    pub pieces: Vec<Entity>,
}

/// Maßstab des Modells unter der Wurzel.
fn model_scale(kind: ShipKind) -> f32 {
    MODEL_SCALE
        * if matches!(kind, ShipKind::Flagship(_)) {
            FLAGSHIP_SCALE
        } else {
            1.0
        }
}

fn model(scene: Handle<WorldAsset>, kind: ShipKind) -> impl Bundle {
    (
        Name::new("Modell"),
        ShipModel,
        WorldAssetRoot(scene),
        Transform::from_scale(Vec3::splat(model_scale(kind))),
    )
}

/// Spawnt ein Schiff samt Modell; Lage, Seite ([`ToonState::facing`]), Eltern und `uid` setzt der Aufrufer.
pub fn spawn_ship(commands: &mut Commands, models: &ShipModels, kind: ShipKind) -> Entity {
    commands
        .spawn((Name::new(format!("Schiff {kind:?}")), Ship { kind, uid: 0 }))
        .with_child(model(models.get(kind).ship.clone(), kind))
        .id()
}

pub(super) struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(break_apart)
            .add_observer(prepare_instance)
            .add_systems(Update, sync_toon_state);
    }
}

fn break_apart(
    event: On<BreakApart>,
    mut commands: Commands,
    mut ships: Query<(&Ship, &Children, &mut ShipMaterials)>,
    models: Query<(), With<ShipModel>>,
    ship_models: Res<ShipModels>,
) {
    let Ok((ship, children, mut own)) = ships.get_mut(event.entity) else {
        return;
    };
    for child in children.iter().filter(|child| models.contains(*child)) {
        commands.entity(child).despawn();
    }
    // Die Materialien des Modells gehen mit ihm; das Wrack bekommt eigene.
    own.0.clear();
    commands.entity(event.entity).with_child((
        model(ship_models.get(ship.kind).wreck.clone(), ship.kind),
        Wreck,
    ));
}

/// Gibt einer frisch gespawnten Szene (Modell oder Wrack) die eigenen Materialien ihres Schiffs und meldet Wracks.
fn prepare_instance(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<(&ChildOf, Has<Wreck>), With<ShipModel>>,
    mut ships: Query<(&ToonState, &mut ShipMaterials), With<Ship>>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<ToonMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
) {
    let Ok((child_of, is_wreck)) = roots.get(ready.entity) else {
        return;
    };
    let ship_entity = child_of.parent();
    let Ok((state, mut own)) = ships.get_mut(ship_entity) else {
        return;
    };
    let mut copies: HashMap<AssetId<ToonMaterial>, Handle<ToonMaterial>> = HashMap::default();
    commands.entity(ready.entity).insert(ModelReady);
    for part in children.iter_descendants(ready.entity) {
        let Ok(template) = parts.get(part) else {
            continue;
        };
        // Schiffe lassen sich anklicken (`crate::scene::picking`), Trümmer nicht.
        if !is_wreck {
            commands.entity(part).insert(Pickable::default());
        }
        let copy = copies.entry(template.id()).or_insert_with(|| {
            let mut material = materials
                .get(template.id())
                .expect("Toon-Material der Vorlage")
                .clone();
            state.apply(&mut material);
            let handle = materials.add(material);
            own.0.push(handle.clone());
            handle
        });
        commands.entity(part).insert(MeshMaterial3d(copy.clone()));
    }
    if is_wreck {
        // Unter dem Träger liegt die Szene, darunter je Bruchstück ein Knoten.
        let pieces = children
            .get(ready.entity)
            .ok()
            .and_then(|scene| scene.first())
            .and_then(|scene| children.get(*scene).ok())
            .map(|pieces| pieces.to_vec())
            .unwrap_or_default();
        commands.trigger(WreckReady {
            entity: ship_entity,
            pieces,
        });
    }
}

/// Gibt Änderungen an Seite, Glut und Lichter an die Materialien des Schiffs weiter.
fn sync_toon_state(
    ships: Query<(&ToonState, &ShipMaterials), Changed<ToonState>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
) {
    for (state, own) in &ships {
        for handle in &own.0 {
            if let Some(mut material) = materials.get_mut(handle) {
                state.apply(&mut material);
            }
        }
    }
}
