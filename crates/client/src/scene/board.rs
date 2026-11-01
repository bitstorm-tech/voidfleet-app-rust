//! Das Brett (Port von `slotPad.ts`, `laneLabel.ts`, `columnMarks.ts` und dem Aufbau in `matchScene.ts`): vier Lanes
//! mit je sechs Landeplätzen ([`Pad`]), die der Aufteilung der Kamera ([`BoardLayout`]) folgen, Beschriftungen der
//! Lanes und vor den Spalten der Gegnerseite die Pfeile fürs Spaltenfeuer ([`ColumnMark`]).
//!
//! Ein Pad zeigt seine Platte nur, solange ein Slot gesucht wird ([`Highlights::grid`]), einen leuchtenden Rahmen, wenn
//! er ein gültiges Ziel ist ([`Highlights::slots`]), und darüber eine helle Fläche, solange der Zeiger auf einem gültigen
//! Ziel liegt. Schiffe hängen an seinem Anker ([`PadAnchor`]): Die Nase zeigt zum Gegner, die Seite gibt das Hauptlicht
//! vor. Welches Schiff auf welchem Slot steht, trägt das Schiff selbst ([`OnSlot`]); [`attach`] setzt es dorthin.

use bevy::{
    picking::Pickable,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    transform::{TransformSystems, commands::BuildChildrenTransformExt},
    ui::UiSystems,
};
use voidfleet_core::board::{LaneType, SLOT_COUNT, Side, SlotRef, lane_slots};

use super::{
    camera::{BoardLayout, MainCamera},
    coords::{BOARD_CENTER, PAD_DEPTH, PAD_WIDTH, facing},
    fx::{FxAssets, FxMaterial, glowing, linear},
    paint::{Canvas, disc, polyline, ring},
    ship::{Ship, ToonState},
};
use crate::{i18n::Locale, palette, ui::Fonts};

/// Höhe des Ankers über dem Pad, an dem die Schiffe hängen.
pub const SHIP_ANCHOR_HEIGHT: f32 = 0.42;
/// Steht schon ein Schiff auf dem Pad (Phasentausch der Mirage), schwebt der Geist so hoch darüber.
pub const GHOST_LIFT: f32 = 0.85;
/// Breite des leuchtenden Rahmens um ein gültiges Ziel.
const RIM: f32 = 0.12;
/// Treffer-Körper: so hoch wie im Original; die Schiffe selbst sind eigene Ziele.
const PICK_HEIGHT: f32 = 0.5;
const PAD_COLOR: Color = Color::srgb(0.17, 0.33, 0.54);
const HOVER_COLOR: Color = Color::srgb(0.82, 0.98, 1.0);
/// Beschriftung: Höhe über dem Brett, Schriftgröße und Farbe je Seite.
const LABEL_HEIGHT: f32 = 0.35;
const LABEL_SIZE: f32 = 26.0;
const LABEL_COLORS: [Color; 2] = [
    Color::srgba(140.0 / 255.0, 217.0 / 255.0, 1.0, 0.7),
    Color::srgba(1.0, 140.0 / 255.0, 128.0 / 255.0, 0.7),
];
/// Pfeil vor der Spalte: Kantenlänge, Abstand vom Strike-Pad, Höhe, Deckkraft ohne Zeiger und Wahl, Größe unter dem
/// Zeiger.
const MARK_SIZE: f32 = 0.62;
const MARK_GAP: f32 = 0.22;
const MARK_LIFT: f32 = 0.07;
const MARK_IDLE_ALPHA: f32 = 0.7;
const MARK_HOVER_SCALE: f32 = 1.2;
const MARK_TEXTURE: u32 = 128;

/// Welche Slots gültige Ziele sind und ob das Gitter der Pads zu sehen ist; setzt der Controller je nach Auswahl.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Highlights {
    /// Slots mit leuchtendem Rahmen; unter dem Zeiger leuchtet zusätzlich die Fläche.
    pub slots: HashSet<SlotRef>,
    /// Die Platten der Pads und die Beschriftungen der Lanes: zu sehen, solange ein freier Slot gesucht wird.
    pub grid: bool,
}

/// Was unter dem Zeiger liegt (aus dem Picking, [`crate::scene::picking`]).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoardHover {
    /// Slot, auch über ein Schiff oder einen Geist darauf.
    pub slot: Option<SlotRef>,
    /// Pfeil vor einer gegnerischen Spalte (nur, solange sie zu sehen sind).
    pub column: Option<usize>,
}

/// Die Pfeile vor den Spalten: zu sehen, solange ein eigenes Schiff zielt (`shown`), gefüllt der Pfeil der Spalte, auf
/// die sein Spaltenfeuer geht (`chosen`).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColumnMarks {
    pub shown: bool,
    pub chosen: Option<usize>,
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

/// Ein Schiff (oder Geist) auf einem Slot. Ohne diese Komponente gehört ein Schiff gerade niemandem (im Flug zwischen
/// zwei Slots ist es schon umgehängt; im Untergang hat es seinen Slot verloren).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnSlot(pub SlotRef);

/// Was ein Klick auf einen Treffer-Körper meint.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickTarget {
    Slot(SlotRef),
    Column(usize),
}

/// Ein Pfeil vor einer gegnerischen Spalte.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColumnMark {
    pub column: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum PadPart {
    Base,
    Frame,
    Hover,
}

/// Das Symbol eines Pfeils (der Treffer-Körper ist sein Geschwister).
#[derive(Component)]
struct ColumnIcon;

/// Beschriftung einer Lane, als Text der Oberfläche über ihrem Weltpunkt.
#[derive(Component, Clone, Copy, Debug)]
pub struct LaneLabel {
    side: Side,
    lane: LaneType,
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

/// Texturen der Pfeile: offen und gewählt.
#[derive(Resource)]
struct MarkTextures {
    open: Handle<Image>,
    chosen: Handle<Image>,
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
    Transform::from_translation(layout.slot_position(slot.side, slot.lane, slot.slot_index))
}

/// Weltpunkt des Ankers eines Slots in der jetzigen Aufteilung (ohne Wackeln und Flüge).
pub fn anchor_position(layout: &BoardLayout, slot: SlotRef) -> Vec3 {
    layout.slot_position(slot.side, slot.lane, slot.slot_index) + Vec3::Y * SHIP_ANCHOR_HEIGHT
}

pub(super) struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Highlights>()
            .init_resource::<BoardHover>()
            .init_resource::<ColumnMarks>()
            .init_resource::<BoardEntities>()
            .add_observer(face_side)
            .add_systems(Startup, (spawn_board, spawn_labels).chain())
            .add_systems(
                Update,
                (
                    follow_layout.run_if(resource_changed::<BoardLayout>),
                    show_pads,
                    show_column_marks,
                ),
            )
            .add_systems(
                PostUpdate,
                // Nach den Lagen dieses Bilds und vor dem UI-Layout: Die Beschriftung steht im selben Bild dort.
                place_labels
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
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

/// Ring mit zwei Winkeln, die nach +x in die Spalte zeigen, wahlweise mit schwachem Schein gefüllt.
fn mark_texture(filled: bool) -> Image {
    let mut canvas = Canvas::new(MARK_TEXTURE);
    if filled {
        canvas.fill(Srgba::WHITE.with_alpha(0.4), disc(Vec2::ZERO, 0.84));
    }
    canvas.fill(Srgba::WHITE, ring(0.84, 0.1));
    for tip in [-0.04, 0.3] {
        canvas.fill(
            Srgba::WHITE,
            polyline(
                vec![
                    Vec2::new(tip - 0.28, -0.34),
                    Vec2::new(tip, 0.0),
                    Vec2::new(tip - 0.28, 0.34),
                ],
                0.15,
            ),
        );
    }
    canvas.into_image()
}

fn spawn_board(
    mut commands: Commands,
    layout: Res<BoardLayout>,
    fx: Res<FxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FxMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut board: ResMut<BoardEntities>,
) {
    let base_mesh = meshes.add(Cuboid::new(PAD_WIDTH, 0.08, PAD_DEPTH));
    let frame = meshes.add(frame_mesh());
    let hover_mesh = meshes.add(Cuboid::new(PAD_WIDTH - 0.08, 0.02, PAD_DEPTH - 0.08));
    let pick_mesh = meshes.add(Cuboid::new(PAD_WIDTH, PICK_HEIGHT, PAD_DEPTH));
    let base_material = materials.add(FxMaterial::solid(linear(PAD_COLOR, 0.45)));
    let frame_material = materials.add(FxMaterial::light(glowing(palette::SELECT, 0.42)));
    let hover_material = materials.add(FxMaterial::solid(linear(HOVER_COLOR, 0.45)));
    let marks = MarkTextures {
        open: images.add(mark_texture(false)),
        chosen: images.add(mark_texture(true)),
    };
    let mark_mesh = fx.floor_quad.clone();

    for side in Side::ALL {
        for lane in LaneType::ALL {
            for slot in lane_slots(side, lane) {
                let name = format!("Pad {side:?} {lane:?} {}", slot.slot_index);
                let anchor = commands
                    .spawn((
                        Name::new("Anker"),
                        PadAnchor { slot },
                        Transform::from_xyz(0.0, SHIP_ANCHOR_HEIGHT, 0.0)
                            .looking_to(Vec3::X * facing(side), Vec3::Y),
                        Visibility::default(),
                    ))
                    .id();
                let pad = commands
                    .spawn((
                        Name::new(name),
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
                            PickTarget::Slot(slot),
                            Mesh3d(pick_mesh.clone()),
                            Pickable::default(),
                        ));
                        if side == Side::Enemy && lane == LaneType::Strike {
                            // Zur Brettmitte hin (−x von der Gegnerseite aus) vor dem Strike-Pad.
                            pad.spawn((
                                Name::new("Spaltenpfeil"),
                                ColumnMark {
                                    column: slot.slot_index,
                                },
                                Transform::from_xyz(
                                    -(PAD_WIDTH / 2.0 + MARK_GAP + MARK_SIZE / 2.0),
                                    0.0,
                                    0.0,
                                ),
                                Visibility::Hidden,
                            ))
                            .with_children(|mark| {
                                mark.spawn((
                                    ColumnIcon,
                                    Mesh3d(mark_mesh.clone()),
                                    MeshMaterial3d(
                                        materials.add(
                                            FxMaterial::light(glowing(
                                                palette::SELECT,
                                                MARK_IDLE_ALPHA,
                                            ))
                                            .with_texture(marks.open.clone()),
                                        ),
                                    ),
                                    Transform::from_xyz(0.0, MARK_LIFT, 0.0)
                                        .with_scale(Vec3::splat(MARK_SIZE)),
                                ));
                                mark.spawn((
                                    PickTarget::Column(slot.slot_index),
                                    Mesh3d(meshes.add(Cuboid::new(
                                        MARK_SIZE + 0.3,
                                        PICK_HEIGHT,
                                        MARK_SIZE + 0.4,
                                    ))),
                                    Pickable::default(),
                                ));
                            });
                        }
                    })
                    .add_child(anchor)
                    .id();
                board.pads.insert(slot, pad);
                board.anchors.insert(slot, anchor);
            }
        }
    }
    commands.insert_resource(marks);
}

fn spawn_labels(mut commands: Commands, fonts: Res<Fonts>, locale: Res<Locale>) {
    for side in Side::ALL {
        for lane in LaneType::ALL {
            commands.spawn((
                Name::new("Lane-Beschriftung"),
                LaneLabel { side, lane },
                Text::new(locale.lane(lane).to_uppercase()),
                fonts.display_bold(LABEL_SIZE),
                TextColor(LABEL_COLORS[side as usize]),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                UiTransform {
                    translation: Val2::percent(-50.0, -50.0),
                    ..default()
                },
                GlobalZIndex(-10),
                Pickable::IGNORE,
                Visibility::Hidden,
            ));
        }
    }
}

/// Die Pads gleiten mit der Aufteilung, wenn die Kamera zwischen den Ansichten dreht.
fn follow_layout(layout: Res<BoardLayout>, mut pads: Query<(&Pad, &mut Transform)>) {
    for (pad, mut transform) in &mut pads {
        *transform = pad_transform(&layout, pad.slot);
    }
}

/// Platte, Rahmen und Fläche unter dem Zeiger je nach Gitter, gültigen Zielen und Zeiger. Liegt beim Platzieren schon
/// ein Geist unter dem Zeiger, zeigt er den Slot; die Fläche bleibt dann aus.
fn show_pads(
    highlights: Res<Highlights>,
    hover: Res<BoardHover>,
    pads: Query<(&Pad, &Children)>,
    mut parts: Query<(&PadPart, &mut Visibility)>,
    ghosts: Query<(&Ship, &OnSlot)>,
) {
    for (pad, children) in &pads {
        let valid = highlights.slots.contains(&pad.slot);
        let ghost_here = ghosts
            .iter()
            .any(|(ship, on)| ship.ghost && on.0 == pad.slot);
        let hovered = valid && hover.slot == Some(pad.slot) && !(highlights.grid && ghost_here);
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

/// Die Pfeile vor den Spalten: zu sehen, solange ein eigenes Schiff zielt, gefüllt der gewählte, unter dem Zeiger
/// größer und voll deckend.
fn show_column_marks(
    marks: Res<ColumnMarks>,
    hover: Res<BoardHover>,
    textures: Res<MarkTextures>,
    mut columns: Query<(&ColumnMark, &mut Visibility, &Children)>,
    mut icons: Query<(&mut Transform, &MeshMaterial3d<FxMaterial>), With<ColumnIcon>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    if !(marks.is_changed() || hover.is_changed()) {
        return;
    }
    for (mark, mut visibility, children) in &mut columns {
        visibility.set_if_neq(if marks.shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let chosen = marks.chosen == Some(mark.column);
        let hovered = hover.column == Some(mark.column);
        for child in children.iter() {
            if let Ok((mut transform, material)) = icons.get_mut(child) {
                transform.scale =
                    Vec3::splat(MARK_SIZE * if hovered { MARK_HOVER_SCALE } else { 1.0 });
                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.texture = Some(if chosen {
                        textures.chosen.clone()
                    } else {
                        textures.open.clone()
                    });
                    material.set_alpha(if chosen || hovered {
                        1.0
                    } else {
                        MARK_IDLE_ALPHA
                    });
                }
            }
        }
    }
}

/// Beschriftungen stehen jenseits von Slot 0 über ihrer Lane, nur solange das Gitter zu sehen ist.
fn place_labels(
    highlights: Res<Highlights>,
    layout: Res<BoardLayout>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut labels: Query<(&LaneLabel, &mut Node, &mut Visibility)>,
) {
    let (camera, camera_transform) = *camera;
    for (label, mut node, mut visibility) in &mut labels {
        visibility.set_if_neq(if highlights.grid {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let world = BOARD_CENTER
            + Vec3::new(
                layout.lane_x(label.side, label.lane),
                LABEL_HEIGHT,
                layout.label_z,
            );
        if let Ok(position) = camera.world_to_viewport(camera_transform, world) {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y);
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

/// Alle Slots des Bretts.
pub fn all_slots() -> impl Iterator<Item = SlotRef> {
    Side::ALL.into_iter().flat_map(|side| {
        LaneType::ALL
            .into_iter()
            .flat_map(move |lane| (0..SLOT_COUNT).map(move |index| SlotRef::new(side, lane, index)))
    })
}
