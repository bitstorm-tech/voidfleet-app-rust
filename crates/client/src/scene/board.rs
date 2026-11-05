//! Das Brett (Port von `slotPad.ts` und dem Aufbau in `matchScene.ts`): drei Spalten, auf der Gegnerseite zwei Reihen
//! (Front und Back), auf der Spielerseite eine, je Slot ein Landeplatz ([`Pad`]), der der Aufteilung der Kamera
//! ([`BoardLayout`]) folgt.
//!
//! Ein Pad zeigt seine Platte nur, solange ein Slot gesucht wird ([`Highlights::grid`]), einen leuchtenden Rahmen, wenn
//! er ein gültiges Ziel ist ([`Highlights::slots`]), und darüber eine helle Fläche, solange der Zeiger auf einem gültigen
//! Ziel liegt. Schiffe hängen an seinem Anker ([`PadAnchor`]): Die Nase zeigt zum Gegner, die Seite gibt das Hauptlicht
//! vor. Welches Schiff auf welchem Slot steht, trägt das Schiff selbst ([`OnSlot`]); [`attach`] setzt es dorthin.

use bevy::{
    picking::Pickable,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    transform::commands::BuildChildrenTransformExt,
};

use super::{
    camera::BoardLayout,
    coords::{PAD_DEPTH, PAD_WIDTH, SlotRef, facing},
    fx::{FxMaterial, glowing, linear},
    ship::ToonState,
};
use crate::palette;

/// Höhe des Ankers über dem Pad, an dem die Schiffe hängen.
pub const SHIP_ANCHOR_HEIGHT: f32 = 0.42;
/// Breite des leuchtenden Rahmens um ein gültiges Ziel.
const RIM: f32 = 0.12;
/// Treffer-Körper: so hoch wie im Original; die Schiffe selbst sind eigene Ziele.
const PICK_HEIGHT: f32 = 0.5;
const PAD_COLOR: Color = Color::srgb(0.17, 0.33, 0.54);
const HOVER_COLOR: Color = Color::srgb(0.82, 0.98, 1.0);
/// Welche Slots gültige Ziele sind und ob das Gitter der Pads zu sehen ist; setzt der Controller je nach Auswahl.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Highlights {
    /// Slots mit leuchtendem Rahmen; unter dem Zeiger leuchtet zusätzlich die Fläche.
    pub slots: HashSet<SlotRef>,
    /// Die Platten der Pads: zu sehen, solange eine Spalte gesucht wird.
    pub grid: bool,
}

/// Was unter dem Zeiger liegt (aus dem Picking, [`crate::scene::picking`]).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoardHover {
    /// Slot, auch über ein Schiff darauf.
    pub slot: Option<SlotRef>,
}

/// Ein Landeplatz.
#[derive(Component, Clone, Copy, Debug)]
pub struct Pad {
    pub slot: SlotRef,
}

/// Der Anker über einem Pad, an dem Schiffe hängen; seine −Z-Achse zeigt zum Gegner.
#[derive(Component, Clone, Copy, Debug)]
pub struct PadAnchor {
    pub slot: SlotRef,
}

/// Ein Schiff auf einem Slot. Ohne diese Komponente gehört ein Schiff gerade niemandem (im Flug zwischen
/// zwei Slots ist es schon umgehängt; im Untergang hat es seinen Slot verloren).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnSlot(pub SlotRef);

/// Der Treffer-Körper eines Pads: Ein Klick darauf meint diesen Slot.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickTarget(pub SlotRef);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum PadPart {
    Base,
    Frame,
    Hover,
}

/// Die Entities des Bretts nach Slot.
#[derive(Resource, Debug, Default)]
pub struct BoardEntities {
    pub pads: HashMap<SlotRef, Entity>,
    pub anchors: HashMap<SlotRef, Entity>,
}

impl BoardEntities {
    pub fn anchor(&self, slot: SlotRef) -> Entity {
        self.anchors[&slot]
    }
}

/// Hängt ein Schiff an den Anker eines Slots, ohne seine Lage im Raum zu ändern (es fliegt von dort hinein), und stellt
/// sein Hauptlicht auf die Seite des Slots.
pub fn attach(commands: &mut Commands, board: &BoardEntities, ship: Entity, slot: SlotRef) {
    commands
        .entity(ship)
        .insert(OnSlot(slot))
        .set_parent_in_place(board.anchor(slot));
}

/// Hängt ein Schiff an den Anker eines Slots und stellt es genau darauf.
pub fn place(commands: &mut Commands, board: &BoardEntities, ship: Entity, slot: SlotRef) {
    commands.entity(ship).insert((
        OnSlot(slot),
        ChildOf(board.anchor(slot)),
        Transform::from_scale(Vec3::splat(super::ship::BOARD_SCALE)),
    ));
}

/// Lage eines Pads in der jetzigen Aufteilung.
fn pad_transform(layout: &BoardLayout, slot: SlotRef) -> Transform {
    Transform::from_translation(layout.slot_position(slot.side, slot.row, slot.column))
}

/// Weltpunkt des Ankers eines Slots in der jetzigen Aufteilung (ohne Wackeln und Flüge).
pub fn anchor_position(layout: &BoardLayout, slot: SlotRef) -> Vec3 {
    layout.slot_position(slot.side, slot.row, slot.column) + Vec3::Y * SHIP_ANCHOR_HEIGHT
}

pub(super) struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Highlights>()
            .init_resource::<BoardHover>()
            .init_resource::<BoardEntities>()
            .add_observer(face_side)
            .add_systems(Startup, spawn_board)
            .add_systems(
                Update,
                (
                    follow_layout.run_if(resource_changed::<BoardLayout>),
                    show_pads,
                ),
            );
    }
}

/// Vier dünne Kästen als Rahmen außen um die Platte, in einem Mesh.
fn frame_mesh() -> Mesh {
    let height = 0.04;
    let boxes = [
        (
            Vec3::new(0.0, 0.0, PAD_DEPTH / 2.0 + RIM / 2.0),
            Vec3::new(PAD_WIDTH + 2.0 * RIM, height, RIM),
        ),
        (
            Vec3::new(0.0, 0.0, -PAD_DEPTH / 2.0 - RIM / 2.0),
            Vec3::new(PAD_WIDTH + 2.0 * RIM, height, RIM),
        ),
        (
            Vec3::new(PAD_WIDTH / 2.0 + RIM / 2.0, 0.0, 0.0),
            Vec3::new(RIM, height, PAD_DEPTH),
        ),
        (
            Vec3::new(-PAD_WIDTH / 2.0 - RIM / 2.0, 0.0, 0.0),
            Vec3::new(RIM, height, PAD_DEPTH),
        ),
    ];
    let part =
        |(center, size): (Vec3, Vec3)| Cuboid::from_size(size).mesh().build().translated_by(center);
    let mut mesh = part(boxes[0]);
    for other in &boxes[1..] {
        mesh.merge(&part(*other)).expect("gleiche Attribute");
    }
    mesh
}

fn spawn_board(
    mut commands: Commands,
    layout: Res<BoardLayout>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FxMaterial>>,
    mut board: ResMut<BoardEntities>,
) {
    let base_mesh = meshes.add(Cuboid::new(PAD_WIDTH, 0.08, PAD_DEPTH));
    let frame = meshes.add(frame_mesh());
    let hover_mesh = meshes.add(Cuboid::new(PAD_WIDTH - 0.08, 0.02, PAD_DEPTH - 0.08));
    let pick_mesh = meshes.add(Cuboid::new(PAD_WIDTH, PICK_HEIGHT, PAD_DEPTH));
    let base_material = materials.add(FxMaterial::solid(linear(PAD_COLOR, 0.45)));
    let frame_material = materials.add(FxMaterial::light(glowing(palette::SELECT, 0.42)));
    let hover_material = materials.add(FxMaterial::solid(linear(HOVER_COLOR, 0.45)));

    for slot in SlotRef::all() {
        let anchor = commands
            .spawn((
                Name::new("Anker"),
                PadAnchor { slot },
                Transform::from_xyz(0.0, SHIP_ANCHOR_HEIGHT, 0.0)
                    .looking_to(Vec3::X * facing(slot.side), Vec3::Y),
                Visibility::default(),
            ))
            .id();
        let pad = commands
            .spawn((
                Name::new(format!(
                    "Pad {:?} {:?} {}",
                    slot.side, slot.row, slot.column
                )),
                Pad { slot },
                pad_transform(&layout, slot),
                Visibility::default(),
            ))
            .with_children(|pad| {
                pad.spawn((
                    PadPart::Base,
                    Mesh3d(base_mesh.clone()),
                    MeshMaterial3d(base_material.clone()),
                    Visibility::Hidden,
                ));
                pad.spawn((
                    PadPart::Frame,
                    Mesh3d(frame.clone()),
                    MeshMaterial3d(frame_material.clone()),
                    Transform::from_xyz(0.0, -0.035, 0.0),
                    Visibility::Hidden,
                ));
                pad.spawn((
                    PadPart::Hover,
                    Mesh3d(hover_mesh.clone()),
                    MeshMaterial3d(hover_material.clone()),
                    Transform::from_xyz(0.0, 0.055, 0.0),
                    Visibility::Hidden,
                ));
                // Treffer-Körper ohne Material: unsichtbar, aber vom Picking getroffen.
                pad.spawn((
                    PickTarget(slot),
                    Mesh3d(pick_mesh.clone()),
                    Pickable::default(),
                ));
            })
            .add_child(anchor)
            .id();
        board.pads.insert(slot, pad);
        board.anchors.insert(slot, anchor);
    }
}

/// Die Pads gleiten mit der Aufteilung, wenn die Kamera zwischen den Ansichten dreht.
fn follow_layout(layout: Res<BoardLayout>, mut pads: Query<(&Pad, &mut Transform)>) {
    for (pad, mut transform) in &mut pads {
        *transform = pad_transform(&layout, pad.slot);
    }
}

/// Platte, Rahmen und Fläche unter dem Zeiger je nach Gitter, gültigen Zielen und Zeiger.
fn show_pads(
    highlights: Res<Highlights>,
    hover: Res<BoardHover>,
    pads: Query<(&Pad, &Children)>,
    mut parts: Query<(&PadPart, &mut Visibility)>,
) {
    for (pad, children) in &pads {
        let valid = highlights.slots.contains(&pad.slot);
        let hovered = valid && hover.slot == Some(pad.slot);
        for child in children.iter() {
            if let Ok((part, mut visibility)) = parts.get_mut(child) {
                let shown = match part {
                    PadPart::Base => highlights.grid,
                    PadPart::Frame => valid,
                    PadPart::Hover => hovered,
                };
                visibility.set_if_neq(if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
}

/// Ein Schiff auf einem Slot nimmt das Hauptlicht seiner Seite an.
fn face_side(add: On<Insert<OnSlot>>, mut ships: Query<(&OnSlot, &mut ToonState)>) {
    if let Ok((on, mut state)) = ships.get_mut(add.entity) {
        let facing = facing(on.0.side);
        if state.facing != facing {
            state.facing = facing;
        }
    }
}
