//! Der Mauszeiger (Port von `ui/cursor.ts`): einer von zehn Zeigern im Toon-Stil mit dunkler Kontur, gewählt in den
//! Einstellungen samt Größe; ein Wechsel gilt sofort, überall im Fenster.
//!
//! Der Systemzeiger ist ausgeblendet, stattdessen folgt ein UI-Knoten über allem der Maus. Denn ein eigener
//! Systemzeiger (`CursorIcon::Custom`) wäre ein festes Bild: unter macOS zählt winit dessen Pixel als Punkte (auf
//! Retina doppelt hochgezogen, unscharf), anderswo als physische Pixel (bei hoher Skalierung winzig). Hier rastert
//! `resvg` das SVG (`assets/cursors/cursor_<id>.svg`, 32 × 32, ins Binary eingebettet) genau auf Größe ×
//! Skalierungsfaktor, neu bei jedem Wechsel von Zeiger, Größe oder Bildschirm. Der Preis: Der Zeiger kommt mit dem
//! Spielbild und hinkt dem Systemzeiger um ein bis zwei Bilder hinterher.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::UiSystems,
    window::{CursorOptions, PrimaryWindow},
};
use resvg::{tiny_skia, usvg};

use crate::settings::{CursorId, Settings};

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

/// Wählbare Größe in logischen Pixeln (Punkten): kleinste, größte, Schritt des Reglers.
pub const MIN_SIZE: f32 = 16.0;
pub const MAX_SIZE: f32 = 64.0;
pub const SIZE_STEP: f32 = 4.0;

/// Kantenlänge der SVGs (`viewBox`).
const ART_SIZE: f32 = 32.0;

/// Kantenlänge der Vorschaubilder im Einstellungsdialog in Pixeln, scharf auch auf Retina-Bildschirmen.
const PREVIEW_PIXELS: u32 = 128;

fn svg(cursor: CursorId) -> &'static str {
    match cursor {
        CursorId::Arrow => include_str!("../../../../assets/cursors/cursor_arrow.svg"),
        CursorId::Fighter => include_str!("../../../../assets/cursors/cursor_fighter.svg"),
        CursorId::Delta => include_str!("../../../../assets/cursors/cursor_delta.svg"),
        CursorId::Flagship => include_str!("../../../../assets/cursors/cursor_flagship.svg"),
        CursorId::Needle => include_str!("../../../../assets/cursors/cursor_needle.svg"),
        CursorId::Shard => include_str!("../../../../assets/cursors/cursor_shard.svg"),
        CursorId::Formation => include_str!("../../../../assets/cursors/cursor_formation.svg"),
        CursorId::Holo => include_str!("../../../../assets/cursors/cursor_holo.svg"),
        CursorId::Thruster => include_str!("../../../../assets/cursors/cursor_thruster.svg"),
        CursorId::Crosshair => include_str!("../../../../assets/cursors/cursor_crosshair.svg"),
    }
}

/// Der Punkt, der zeigt, in Koordinaten des SVGs: die Spitze, beim Fadenkreuz die Mitte.
const fn hotspot(cursor: CursorId) -> Vec2 {
    match cursor {
        CursorId::Crosshair => Vec2::splat(16.0),
        _ => Vec2::splat(2.0),
    }
}

/// Rastert einen Zeiger auf `pixels` × `pixels`.
fn rasterize(cursor: CursorId, pixels: u32) -> Image {
    let tree = usvg::Tree::from_str(svg(cursor), &usvg::Options::default())
        .expect("eingebettetes SVG ist gültig");
    let mut pixmap = tiny_skia::Pixmap::new(pixels, pixels).expect("Fläche");
    let scale = pixels as f32 / ART_SIZE;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia rechnet mit vormultipliziertem Alpha, Bevy erwartet es gerade.
    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Image::new(
        Extent3d {
            width: pixels,
            height: pixels,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Die Vorschaubilder aller Zeiger in der Reihenfolge von [`CURSORS`], beim Start gerastert.
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

/// Der Knoten, der der Maus folgt, mit dem Abstand der Spitze von seiner oberen linken Ecke in logischen Pixeln.
#[derive(Component, Default)]
struct Pointer {
    hotspot: Vec2,
}

/// Rastert die Vorschaubilder, blendet den Systemzeiger aus und legt den Zeiger an.
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut window: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let previews = CURSORS
        .into_iter()
        .map(|cursor| images.add(rasterize(cursor, PREVIEW_PIXELS)))
        .collect();
    commands.insert_resource(CursorImages(previews));
    window.visible = false;
    commands.spawn((
        Name::new("Mauszeiger"),
        Pointer::default(),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::default(),
        GlobalZIndex(i32::MAX),
        Pickable::IGNORE,
        Visibility::Hidden,
    ));
}

/// Rastert den Zeiger neu, wenn sich Wahl, Größe oder Skalierungsfaktor ändern; so deckt sich jedes Pixel des Bildes
/// mit einem des Bildschirms.
fn render(
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    pointer: Single<(&mut Pointer, &mut Node, &mut ImageNode)>,
    mut rendered: Local<Option<(CursorId, u32)>>,
) {
    let scale = window.scale_factor();
    let pixels = (settings.cursor_size.clamp(MIN_SIZE, MAX_SIZE) * scale).round() as u32;
    let wanted = (settings.cursor, pixels);
    if *rendered == Some(wanted) {
        return;
    }
    *rendered = Some(wanted);
    let (mut pointer, mut node, mut image) = pointer.into_inner();
    let size = pixels as f32 / scale;
    pointer.hotspot = hotspot(settings.cursor) * size / ART_SIZE;
    node.width = px(size);
    node.height = px(size);
    // Das alte Bild gibt Bevy frei, sobald niemand mehr seinen Handle hält.
    image.image = images.add(rasterize(settings.cursor, pixels));
}

/// Setzt den Zeiger an die Maus; außerhalb des Fensters ist er verborgen. Unter macOS zeigt das System seinen Zeiger
/// über einem Fenster ohne Fokus wieder, dann verbirgt sich unserer.
fn follow(
    window: Single<&Window, With<PrimaryWindow>>,
    pointer: Single<(&Pointer, &mut Node, &mut Visibility)>,
) {
    let (pointer, mut node, mut visibility) = pointer.into_inner();
    let shown = cfg!(not(target_os = "macos")) || window.focused;
    let Some(at) = window.cursor_position().filter(|_| shown) else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    visibility.set_if_neq(Visibility::Inherited);
    let corner = at - pointer.hotspot;
    if node.left != px(corner.x) || node.top != px(corner.y) {
        node.left = px(corner.x);
        node.top = px(corner.y);
    }
}

pub(super) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup)
            .add_systems(PostUpdate, (render, follow).before(UiSystems::Prepare));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jeder Zeiger lässt sich rastern und zeichnet seine Spitze an den Hotspot.
    #[test]
    fn every_cursor_rasterizes() {
        for cursor in CURSORS {
            let image = rasterize(cursor, 64);
            let data = image.data.as_ref().expect("Pixel");
            let tip = hotspot(cursor) * 2.0;
            let alpha = data[(tip.y as usize * 64 + tip.x as usize) * 4 + 3];
            assert!(alpha > 0, "{cursor:?}: Hotspot ist durchsichtig");
        }
    }
}
