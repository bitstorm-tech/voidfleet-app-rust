//! Der Mauszeiger (Port von `ui/cursor.ts`): einer von zehn Zeigern im Toon-Stil mit dunkler Kontur, gewählt in den
//! Einstellungen samt Größe; ein Wechsel gilt sofort, überall im Fenster. Steht der Zielpfeil ([`AimArrowShown`]),
//! ist seine Spitze der Zeiger und der Systemzeiger ein leeres Bild.
//!
//! Es ist der Systemzeiger mit eigenem Bild, den das Betriebssystem unabhängig vom Spielbild zeichnet, also ohne
//! Verzögerung. `resvg` rastert das SVG (`assets/cursors/cursor_<id>.svg`, 32 × 32, ins Binary eingebettet) so,
//! dass jedes Pixel einem des Bildschirms entspricht. Unter Windows und Linux geht das Bild als `CursorIcon::Custom`
//! an winit, das es in physischen Pixeln nimmt: Größe × Skalierungsfaktor, neu bei jedem Wechsel des Faktors. Unter
//! macOS nähme winit es in Punkten (auf Retina unscharf), darum setzt [`macos`] den Zeiger dort direkt über AppKit,
//! mit einem Bild je Auflösung.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(target_os = "macos"))]
use bevy::window::{
    CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, WindowScaleFactorChanged,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
#[cfg(target_os = "macos")]
use bevy::{ecs::system::NonSendMarker, window::WindowFocused};
use resvg::{tiny_skia, usvg};

use super::aim_arrow::AimArrowShown;
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

/// Kantenlänge der Vorschaubilder im Einstellungsdialog in logischen Pixeln; gerastert doppelt so groß, scharf auch
/// auf Retina-Bildschirmen.
pub const PREVIEW_SIZE: f32 = 32.0;

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

/// Der Zeiger, der gerade gilt: der gewählte; keiner, solange der Zielpfeil steht.
fn shown(settings: &Settings, arrow: AimArrowShown) -> Option<CursorId> {
    (!arrow.0).then_some(settings.cursor)
}

/// Die gewählte Größe in Punkten, im erlaubten Bereich.
fn chosen_size(settings: &Settings) -> f32 {
    settings.cursor_size.clamp(MIN_SIZE, MAX_SIZE)
}

/// Wie weit das Bild des gewählten Zeigers von seinem Hotspot aus reicht, in Punkten: nach links oben (negativ) und
/// nach rechts unten.
pub fn reach(settings: &Settings) -> (Vec2, Vec2) {
    let scale = chosen_size(settings) / ART_SIZE;
    let spot = hotspot(settings.cursor);
    (-spot * scale, (Vec2::splat(ART_SIZE) - spot) * scale)
}

/// Rastert einen Zeiger auf `pixels` × `pixels`; ohne Zeiger bleibt das Bild leer.
fn pixmap(cursor: Option<CursorId>, pixels: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(pixels, pixels).expect("Fläche");
    if let Some(cursor) = cursor {
        let tree = usvg::Tree::from_str(svg(cursor), &usvg::Options::default())
            .expect("eingebettetes SVG ist gültig");
        let scale = pixels as f32 / ART_SIZE;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
    }
    pixmap
}

/// Rastert einen Zeiger auf `pixels` × `pixels` als Bild für Bevy.
fn rasterize(cursor: Option<CursorId>, pixels: u32) -> Image {
    // tiny-skia rechnet mit vormultipliziertem Alpha, Bevy erwartet es gerade.
    let data = pixmap(cursor, pixels)
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
        // Die Hauptwelt behält die Pixel: Aus ihnen baut `bevy_winit` den Systemzeiger.
        RenderAssetUsages::default(),
    )
}

/// Die Vorschaubilder aller Zeiger in der Reihenfolge von [`CURSORS`], beim Aufbau des Plugins gerastert.
#[derive(Resource)]
pub struct CursorImages(Vec<Handle<Image>>);

impl FromWorld for CursorImages {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        Self(
            CURSORS
                .into_iter()
                .map(|cursor| images.add(rasterize(Some(cursor), (PREVIEW_SIZE * 2.0) as u32)))
                .collect(),
        )
    }
}

impl CursorImages {
    pub fn get(&self, cursor: CursorId) -> Handle<Image> {
        let index = CURSORS
            .iter()
            .position(|&other| other == cursor)
            .expect("jeder Zeiger steht in CURSORS");
        self.0[index].clone()
    }
}

/// Setzt den geltenden Zeiger als Systemzeiger: beim Start und wenn sich Zeiger, Größe oder Skalierungsfaktor ändern.
#[cfg(not(target_os = "macos"))]
fn apply(
    mut commands: Commands,
    settings: Res<Settings>,
    arrow: Res<AimArrowShown>,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut applied: Local<Option<(Option<CursorId>, u32)>>,
) {
    let (entity, window) = window.into_inner();
    let cursor = shown(&settings, *arrow);
    let pixels = (chosen_size(&settings) * window.scale_factor()).round() as u32;
    let wanted = (cursor, pixels);
    if *applied == Some(wanted) {
        return;
    }
    *applied = Some(wanted);
    let spot = (cursor.map_or(Vec2::ZERO, hotspot) * pixels as f32 / ART_SIZE).round();
    commands
        .entity(entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: images.add(rasterize(cursor, pixels)),
            hotspot: (spot.x as u16, spot.y as u16),
            ..default()
        })));
}

/// Setzt den geltenden Zeiger als Systemzeiger: beim Start und wenn sich Zeiger oder Größe ändern. Je ein Bild in
/// einfacher und doppelter Auflösung, AppKit wählt je Bildschirm das passende.
#[cfg(target_os = "macos")]
fn apply(
    _main_thread: NonSendMarker,
    settings: Res<Settings>,
    arrow: Res<AimArrowShown>,
    mut applied: Local<Option<(Option<CursorId>, f32)>>,
) {
    let cursor = shown(&settings, *arrow);
    let points = chosen_size(&settings);
    let wanted = (cursor, points);
    if *applied == Some(wanted) {
        return;
    }
    *applied = Some(wanted);
    let pngs = [1, 2].map(|scale| {
        pixmap(cursor, points.round() as u32 * scale)
            .encode_png()
            .expect("PNG aus dem Zeigerbild")
    });
    macos::install(
        pngs,
        points,
        cursor.map_or(Vec2::ZERO, hotspot) * points / ART_SIZE,
    );
}

pub(super) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorImages>();
        #[cfg(not(target_os = "macos"))]
        app.add_systems(
            Update,
            apply.run_if(
                resource_changed::<Settings>
                    .or_else(resource_changed::<AimArrowShown>)
                    .or_else(on_message::<WindowScaleFactorChanged>),
            ),
        );
        #[cfg(target_os = "macos")]
        app.add_systems(
            Update,
            (
                apply.run_if(
                    resource_changed::<Settings>.or_else(resource_changed::<AimArrowShown>),
                ),
                macos::reapply
                    .run_if(on_message::<CursorEntered>.or_else(on_message::<WindowFocused>)),
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jeder Zeiger lässt sich rastern und zeichnet seine Spitze an den Hotspot.
    #[test]
    fn every_cursor_rasterizes() {
        for cursor in CURSORS {
            let image = rasterize(Some(cursor), 64);
            let data = image.data.as_ref().expect("Pixel");
            let tip = hotspot(cursor) * 2.0;
            let alpha = data[(tip.y as usize * 64 + tip.x as usize) * 4 + 3];
            assert!(alpha > 0, "{cursor:?}: Hotspot ist durchsichtig");
        }
    }
}
