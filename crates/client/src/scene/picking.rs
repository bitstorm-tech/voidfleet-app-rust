//! Zeiger auf dem Brett über Bevys Mesh-Picking: Getroffen werden nur die Treffer-Körper der Pads und der Pfeile vor den
//! Spalten (diese nur, solange sie zu sehen sind) und die Modelle der Schiffe und Geister, die für ihren Slot stehen.
//! Was ein Klick bedeutet, entscheidet der Controller; die Szene meldet nur, worauf er ging:
//!
//! - [`SlotPressed`] — ein Slot, auch über ein Schiff oder einen Geist darauf.
//! - [`ColumnPressed`] — der Pfeil vor einer gegnerischen Spalte (Spaltenfeuer).
//! - [`BoardMissed`] — daneben, ins Weltall (bricht im Original die Auswahl ab). Klicks auf die Oberfläche zählen nicht.
//! - [`SlotHovered`], [`ColumnHovered`] — der Zeiger wechselt auf einen anderen Slot bzw. Pfeil; der Stand steht in
//!   [`BoardHover`].
//!
//! Wie im Original zählt ein Klick nur, wenn die Maus dabei nicht das Brett verschoben hat.

use bevy::{
    picking::{
        events::PointerClick,
        hover::HoverMap,
        mesh_picking::{MeshPickingPlugin, MeshPickingSettings},
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    window::Window,
};
use voidfleet_core::board::SlotRef;

use super::{
    board::{BoardHover, OnSlot, PickTarget},
    camera::CameraInput,
};

/// Ein Klick auf einen Slot (Pad, Schiff oder Geist darauf).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotPressed(pub SlotRef);

/// Ein Klick auf den Pfeil vor einer gegnerischen Spalte.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnPressed(pub usize);

/// Ein Klick ins Leere neben dem Brett.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardMissed;

/// Der Zeiger liegt jetzt über diesem Slot (oder keinem).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotHovered(pub Option<SlotRef>);

/// Der Zeiger liegt jetzt über diesem Spaltenpfeil (oder keinem).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnHovered(pub Option<usize>);

pub(super) struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MeshPickingPlugin)
            // Nur was `Pickable` trägt, wird getroffen: Effekte, Marken und Balken bleiben durchlässig.
            .insert_resource(MeshPickingSettings {
                require_markers: true,
                ..default()
            })
            .add_message::<SlotPressed>()
            .add_message::<ColumnPressed>()
            .add_message::<BoardMissed>()
            .add_message::<SlotHovered>()
            .add_message::<ColumnHovered>()
            .add_systems(Update, (track_hover, report_clicks));
    }
}

/// Was ein getroffenes Entity meint: sein eigenes Ziel oder das seines nächsten Vorfahren (Teile eines Schiffs).
fn resolve(
    entity: Entity,
    parents: &Query<&ChildOf>,
    targets: &Query<&PickTarget>,
    ships: &Query<&OnSlot>,
) -> Option<PickTarget> {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find_map(|entity| {
            targets
                .get(entity)
                .ok()
                .copied()
                .or_else(|| ships.get(entity).ok().map(|on| PickTarget::Slot(on.0)))
        })
}

fn track_hover(
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    targets: Query<&PickTarget>,
    ships: Query<&OnSlot>,
    mut hover: ResMut<BoardHover>,
    mut slot_hovered: MessageWriter<SlotHovered>,
    mut column_hovered: MessageWriter<ColumnHovered>,
) {
    let target = hover_map
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hits| hits.iter())
        .filter_map(|(entity, hit)| {
            resolve(*entity, &parents, &targets, &ships).map(|target| (hit.depth, target))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, target)| target);
    let next = BoardHover {
        slot: match target {
            Some(PickTarget::Slot(slot)) => Some(slot),
            _ => None,
        },
        column: match target {
            Some(PickTarget::Column(column)) => Some(column),
            _ => None,
        },
    };
    if next.slot != hover.slot {
        slot_hovered.write(SlotHovered(next.slot));
    }
    if next.column != hover.column {
        column_hovered.write(ColumnHovered(next.column));
    }
    hover.set_if_neq(next);
}

fn report_clicks(
    mut clicks: MessageReader<PointerClick>,
    input: Res<CameraInput>,
    parents: Query<&ChildOf>,
    targets: Query<&PickTarget>,
    ships: Query<&OnSlot>,
    windows: Query<(), With<Window>>,
    mut slots: MessageWriter<SlotPressed>,
    mut columns: MessageWriter<ColumnPressed>,
    mut missed: MessageWriter<BoardMissed>,
) {
    for click in clicks.read() {
        if click.button != PointerButton::Primary
            || click.pointer.id != PointerId::Mouse
            || input.panned()
        {
            continue;
        }
        match resolve(click.entity, &parents, &targets, &ships) {
            Some(PickTarget::Slot(slot)) => {
                slots.write(SlotPressed(slot));
            }
            Some(PickTarget::Column(column)) => {
                columns.write(ColumnPressed(column));
            }
            // Nichts getroffen: Dann meldet das Picking das Fenster selbst.
            None if windows.contains(click.entity) => {
                missed.write(BoardMissed);
            }
            None => {}
        }
    }
}
