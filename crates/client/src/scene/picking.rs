//! Zeiger auf dem Brett über Bevys Mesh-Picking: Getroffen werden nur die Treffer-Körper der Pads und die Modelle der
//! Schiffe, die für ihren Slot stehen. Was ein Klick bedeutet, entscheidet der Controller; die Szene meldet nur, worauf
//! er ging:
//!
//! - [`SlotPressed`] — ein Slot, auch über ein Schiff darauf.
//! - [`BoardMissed`] — daneben, ins Weltall (bricht die Auswahl ab). Klicks auf die Oberfläche zählen nicht.
//!
//! Der Slot unter dem Zeiger steht in [`BoardHover`].
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

use crate::run::Screen;

use super::{
    board::{BoardHover, OnSlot, PickTarget},
    camera::CameraInput,
    coords::SlotRef,
};

/// Ein Klick auf einen Slot (Pad oder Schiff darauf).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotPressed(pub SlotRef);

/// Ein Klick ins Leere neben dem Brett.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardMissed;

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
            .add_message::<BoardMissed>()
            .add_systems(
                Update,
                (
                    track_hover,
                    // Die Modellschau nimmt ihre Klicks selbst.
                    report_clicks.run_if(not(resource_equals(Screen::Models))),
                ),
            );
    }
}

/// Welchen Slot ein getroffenes Entity meint: seinen eigenen oder den seines nächsten Vorfahren (Teile eines Schiffs).
fn resolve(
    entity: Entity,
    parents: &Query<&ChildOf>,
    targets: &Query<&PickTarget>,
    ships: &Query<&OnSlot>,
) -> Option<SlotRef> {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find_map(|entity| {
            targets
                .get(entity)
                .ok()
                .map(|target| target.0)
                .or_else(|| ships.get(entity).ok().map(|on| on.0))
        })
}

fn track_hover(
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    targets: Query<&PickTarget>,
    ships: Query<&OnSlot>,
    mut hover: ResMut<BoardHover>,
) {
    let slot = hover_map
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hits| hits.iter())
        .filter_map(|(entity, hit)| {
            resolve(*entity, &parents, &targets, &ships).map(|target| (hit.depth, target))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, target)| target);
    hover.set_if_neq(BoardHover { slot });
}

fn report_clicks(
    mut clicks: MessageReader<PointerClick>,
    input: Res<CameraInput>,
    parents: Query<&ChildOf>,
    targets: Query<&PickTarget>,
    ships: Query<&OnSlot>,
    windows: Query<(), With<Window>>,
    mut slots: MessageWriter<SlotPressed>,
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
            Some(slot) => {
                slots.write(SlotPressed(slot));
            }
            // Nichts getroffen: Dann meldet das Picking das Fenster selbst.
            None if windows.contains(click.entity) => {
                missed.write(BoardMissed);
            }
            None => {}
        }
    }
}
