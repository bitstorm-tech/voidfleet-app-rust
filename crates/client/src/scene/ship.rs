//! Schiffe und ihre Wracks (Grundlagen aus `scene/ship.ts`): Modell spawnen, Toon-Zustand je Schiff, Tausch gegen die
//! Bruchstücke.
//!
//! Ein Schiff ist eine Entity mit [`Ship`] (eine Einheit des Gefechts); seine Nase zeigt nach `Transform::forward` (−Z),
//! gedreht wird allein diese Wurzel. Darunter hängt die Gruppe ([`ShipModel`], die Flüge und Todeskampf bewegen) und in
//! ihr je Modell der Formation ([`super::formation`]) ein [`Hull`] mit der glTF-Szene. Sobald eine Szene steht, bekommt
//! sie eigene Kopien ihrer Toon- und Kontur-Materialien, denn Seite, Glut und Lichter ([`ToonState`]) gelten je Schiff, Verfall und
//! Treffer-Blitz ([`HullWear`]) je Modell, die Lackierung der Gegner ([`livery`]) je Art. [`BreakApart`] tauscht die
//! sichtbaren Modelle gegen ihre Bruchstücke, die exakt an ihrer Stelle liegen, und meldet sie je Modell mit
//! [`WreckReady`]: je Stück eine Entity mit dem Ursprung im Schwerpunkt.
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
    formation::{Hull, HullWear, formation},
    markers::ShipMarkers,
    toon::{OutlineMaterial, ToonMaterial},
};
use crate::data::ship_models::{ShipKind, ShipModels, livery};

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
    ShipMarkers,
    AimedHull
)]
pub struct Ship {
    pub kind: ShipKind,
    pub uid: u32,
}

/// Der Geist eines Schiffs: dasselbe Modell als bläuliches Hologramm, ohne Balken, Marken und Picking (kein
/// [`Ship`]); zeigt etwa, wohin das Flaggschiff manövrieren wird.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(
    Transform::from_scale(Vec3::splat(BOARD_SCALE)),
    Visibility,
    ToonState,
    ShipMaterials
)]
pub struct Ghost(pub ShipKind);

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

/// Die eigenen Toon-Materialien eines Schiffs, Modelle und Wracks zusammen, je mit dem [`Hull`], dessen Szene sie trägt.
#[derive(Component, Default, Debug)]
pub struct ShipMaterials(Vec<(Entity, Handle<ToonMaterial>)>);

/// Das Modell der Gruppe, auf das zuletzt geschossen wurde: Dort blitzt der Treffer auf.
#[derive(Component, Default, Debug)]
pub struct AimedHull(pub Option<Entity>);

/// Die Gruppe unter dem Schiff, Träger der [`Hull`]s: erst die Modelle, nach [`BreakApart`] die Wracks.
#[derive(Component, Debug)]
pub struct ShipModel;

/// Markiert einen [`ShipModel`], der die Bruchstücke trägt.
#[derive(Component, Debug)]
pub struct Wreck;

/// Die Szene unter einem [`Hull`] steht und hat ihre Materialien; ab dem nächsten Bild sind Lage und Grenzen ihrer
/// Teile gerechnet.
#[derive(Component, Debug)]
pub struct ModelReady;

/// Tauscht das Modell eines Schiffs gegen seine Bruchstücke.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct BreakApart {
    /// Das Schiff.
    pub entity: Entity,
}

/// Die Bruchstücke eines Modells stehen, exakt an seiner Stelle; je Stück eine Entity mit dem Ursprung im
/// Schwerpunkt, frei zu bewegen. Eine Gruppe meldet je Modell.
#[derive(EntityEvent, Clone, Debug)]
pub struct WreckReady {
    /// Das Schiff.
    pub entity: Entity,
    /// Der [`Hull`] des Wracks.
    pub hull: Entity,
    /// Wie viele Modelle die Gruppe zerlegt.
    pub hulls: usize,
    pub pieces: Vec<Entity>,
}

/// Maßstab des Modells unter der Wurzel.
/// Flaggschiffe (auch das Siegel) sind um [`FLAGSHIP_SCALE`] größer, wie in der Vorschau von `tools/ships`.
fn model_scale(kind: ShipKind) -> f32 {
    MODEL_SCALE
        * if kind.is_flagship() {
            FLAGSHIP_SCALE
        } else {
            1.0
        }
}

/// Hängt unter `ship` eine Gruppe mit je einem [`Hull`] für die Modelle `hulls` der Formation.
fn spawn_group(
    commands: &mut Commands,
    ship: Entity,
    scene: &Handle<WorldAsset>,
    kind: ShipKind,
    hulls: impl IntoIterator<Item = usize>,
) -> Entity {
    let (scale, offsets) = formation(kind.class());
    let group = commands
        .spawn((
            Name::new("Modelle"),
            ShipModel,
            Transform::default(),
            Visibility::default(),
            ChildOf(ship),
        ))
        .id();
    for index in hulls {
        commands.spawn((
            Name::new(format!("Modell {index}")),
            Hull { index },
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(offsets[index])
                .with_scale(Vec3::splat(model_scale(kind) * scale)),
            ChildOf(group),
        ));
    }
    group
}

/// Spawnt ein Schiff samt Modellen; Lage, Seite ([`ToonState::facing`]), Eltern und `uid` setzt der Aufrufer.
pub fn spawn_ship(commands: &mut Commands, models: &ShipModels, kind: ShipKind) -> Entity {
    let ship = commands
        .spawn((Name::new(format!("Schiff {kind:?}")), Ship { kind, uid: 0 }))
        .id();
    let count = formation(kind.class()).1.len();
    spawn_group(commands, ship, &models.get(kind).ship, kind, 0..count);
    ship
}

/// Spawnt den Geist eines Schiffs; Lage, Seite und Eltern setzt der Aufrufer.
pub fn spawn_ghost(commands: &mut Commands, models: &ShipModels, kind: ShipKind) -> Entity {
    let ghost = commands
        .spawn((Name::new(format!("Geist {kind:?}")), Ghost(kind)))
        .id();
    let count = formation(kind.class()).1.len();
    spawn_group(commands, ghost, &models.get(kind).ship, kind, 0..count);
    ghost
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
    groups: Query<&Children, With<ShipModel>>,
    hulls: Query<(&Hull, &Visibility)>,
    ship_models: Res<ShipModels>,
) {
    let Ok((ship, children, mut own)) = ships.get_mut(event.entity) else {
        return;
    };
    let mut standing = Vec::new();
    for group in children.iter().filter(|child| groups.contains(*child)) {
        standing.extend(
            groups
                .get(group)
                .into_iter()
                .flatten()
                .filter_map(|hull| hulls.get(*hull).ok())
                .filter(|(_, visibility)| **visibility != Visibility::Hidden)
                .map(|(hull, _)| hull.index),
        );
        commands.entity(group).despawn();
    }
    // Die Materialien der Modelle gehen mit ihnen; die Wracks bekommen eigene.
    own.0.clear();
    let wreck = &ship_models.get(ship.kind).wreck;
    let group = spawn_group(&mut commands, event.entity, wreck, ship.kind, standing);
    commands.entity(group).insert(Wreck);
}

/// Gibt einer frisch gespawnten Szene (Modell oder Wrack) die eigenen Materialien ihres Schiffs und meldet Wracks.
fn prepare_instance(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    hulls: Query<(&ChildOf, Option<&HullWear>), With<Hull>>,
    groups: Query<(&ChildOf, &Children, Has<Wreck>), With<ShipModel>>,
    mut ships: Query<(Option<&Ship>, Has<Ghost>, &ToonState, &mut ShipMaterials)>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<ToonMaterial>>,
    contours: Query<&MeshMaterial3d<OutlineMaterial>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
    mut outline_materials: ResMut<Assets<OutlineMaterial>>,
) {
    let Ok((hull_of, wear)) = hulls.get(ready.entity) else {
        return;
    };
    let Ok((group_of, group_hulls, is_wreck)) = groups.get(hull_of.parent()) else {
        return;
    };
    let ship_entity = group_of.parent();
    let Ok((ship, ghost, state, mut own)) = ships.get_mut(ship_entity) else {
        return;
    };
    let enemy = ship.is_some_and(|ship| livery(ship.kind));
    let mut copies: HashMap<AssetId<ToonMaterial>, Handle<ToonMaterial>> = HashMap::default();
    let mut outline_copies: HashMap<AssetId<OutlineMaterial>, Handle<OutlineMaterial>> =
        HashMap::default();
    commands.entity(ready.entity).insert(ModelReady);
    for part in children.iter_descendants(ready.entity) {
        if let Ok(template) = contours.get(part) {
            // Geister haben keine Konturen.
            if ghost {
                commands.entity(part).despawn();
                continue;
            }
            let copy = outline_copies.entry(template.id()).or_insert_with(|| {
                let mut material = outline_materials
                    .get(template.id())
                    .expect("Kontur-Material der Vorlage")
                    .clone();
                material.livery = enemy;
                outline_materials.add(material)
            });
            commands.entity(part).insert(MeshMaterial3d(copy.clone()));
            continue;
        }
        let Ok(template) = parts.get(part) else {
            continue;
        };
        // Schiffe lassen sich anklicken (`crate::scene::picking`), Trümmer und Geister nicht.
        if !is_wreck && !ghost {
            commands.entity(part).insert(Pickable::default());
        }
        let copy = copies.entry(template.id()).or_insert_with(|| {
            let mut material = materials
                .get(template.id())
                .expect("Toon-Material der Vorlage")
                .clone();
            material.livery = enemy;
            material.ghost = ghost;
            state.apply(&mut material);
            if let Some(wear) = wear {
                wear.apply(&mut material);
            }
            let handle = materials.add(material);
            own.0.push((ready.entity, handle.clone()));
            handle
        });
        commands.entity(part).insert(MeshMaterial3d(copy.clone()));
    }
    if is_wreck {
        // Unter dem Hull liegt die Szene, darunter je Bruchstück ein Knoten.
        let pieces = children
            .get(ready.entity)
            .ok()
            .and_then(|scene| scene.first())
            .and_then(|scene| children.get(*scene).ok())
            .map(|pieces| pieces.to_vec())
            .unwrap_or_default();
        commands.trigger(WreckReady {
            entity: ship_entity,
            hull: ready.entity,
            hulls: group_hulls.len(),
            pieces,
        });
    }
}

/// Gibt Änderungen an Seite, Glut und Lichter des Schiffs und am Verfall eines Modells an dessen Materialien weiter.
fn sync_toon_state(
    ships: Query<(Ref<ToonState>, &ShipMaterials)>,
    wear: Query<Ref<HullWear>>,
    mut materials: ResMut<Assets<ToonMaterial>>,
) {
    for (state, own) in &ships {
        for (hull, handle) in &own.0 {
            let wear = wear.get(*hull).ok();
            if !state.is_changed() && !wear.as_ref().is_some_and(Ref::is_changed) {
                continue;
            }
            if let Some(mut material) = materials.get_mut(handle) {
                state.apply(&mut material);
                if let Some(wear) = &wear {
                    wear.apply(&mut material);
                }
            }
        }
    }
}
