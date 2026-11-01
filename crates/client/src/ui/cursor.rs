//! Der Mauszeiger (Port von `ui/cursor.ts`): einer von zehn Zeigern im Toon-Stil mit dunkler Kontur
//! (`cursors/cursor_<id>.png`, 32 × 32, gerastert von `tools/rasterize`), gewählt in den Einstellungen; ein Wechsel
//! gilt sofort. Er gilt überall im Fenster, auch über Knöpfen und dem Brett.

use bevy::{
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow},
};

use crate::{
    assets::Preload,
    settings::{CursorId, Settings},
};

/// Alle wählbaren Zeiger in der Reihenfolge der Auswahl.
pub const CURSORS: [CursorId; 10] = [
    CursorId::Arrow,
    CursorId::Fighter,
    CursorId::Delta,
    CursorId::Flagship,
    CursorId::Needle,
    CursorId::Shard,
    CursorId::Formation,
    CursorId::Holo,
    CursorId::Thruster,
    CursorId::Crosshair,
];

/// Dateiname eines Zeigers.
pub const fn cursor_key(cursor: CursorId) -> &'static str {
    match cursor {
        CursorId::Arrow => "arrow",
        CursorId::Fighter => "fighter",
        CursorId::Delta => "delta",
        CursorId::Flagship => "flagship",
        CursorId::Needle => "needle",
        CursorId::Shard => "shard",
        CursorId::Formation => "formation",
        CursorId::Holo => "holo",
        CursorId::Thruster => "thruster",
        CursorId::Crosshair => "crosshair",
    }
}

/// Asset-Pfad des Bildes; der Einstellungsdialog zeigt es als Vorschau.
pub fn cursor_path(cursor: CursorId) -> String {
    format!("cursors/cursor_{}.png", cursor_key(cursor))
}

/// Der Punkt, der zeigt, in Pixeln des Bildes: die Spitze, beim Fadenkreuz die Mitte.
const fn hotspot(cursor: CursorId) -> (u16, u16) {
    match cursor {
        CursorId::Crosshair => (16, 16),
        _ => (2, 2),
    }
}

/// Die Bilder aller Zeiger in der Reihenfolge von [`CURSORS`], beim Start geladen.
#[derive(Resource)]
pub struct CursorImages(Vec<Handle<Image>>);

impl CursorImages {
    pub fn get(&self, cursor: CursorId) -> Handle<Image> {
        let index = CURSORS
            .iter()
            .position(|&other| other == cursor)
            .expect("jeder Zeiger steht in CURSORS");
        self.0[index].clone()
    }
}

fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let images = CURSORS
        .into_iter()
        .map(|cursor| preload.track(server.load(cursor_path(cursor))))
        .collect();
    commands.insert_resource(CursorImages(images));
}

/// Setzt den gewählten Zeiger als Fenster-Cursor, beim Start und nach jedem Wechsel.
fn apply(
    mut commands: Commands,
    settings: Res<Settings>,
    images: Res<CursorImages>,
    window: Single<Entity, With<PrimaryWindow>>,
    mut applied: Local<Option<CursorId>>,
) {
    if *applied == Some(settings.cursor) {
        return;
    }
    *applied = Some(settings.cursor);
    commands
        .entity(*window)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: images.get(settings.cursor),
            hotspot: hotspot(settings.cursor),
            ..default()
        })));
}

pub(super) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load).add_systems(Update, apply);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn every_cursor_has_an_image() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        for cursor in CURSORS {
            assert!(root.join(cursor_path(cursor)).is_file(), "{cursor:?}");
        }
    }
}
