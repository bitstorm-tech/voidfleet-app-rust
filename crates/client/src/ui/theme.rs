//! Gemeinsame Optik der Oberfläche (Port der zentralen Stile aus `styles.css`): Farben der Flächen, Schrift-Helfer
//! für Fließtext und Display-Text (groß geschrieben und gesperrt wie in einem HUD), das Steuerpult der Dialoge
//! ([`console`], [`console_header`], [`console_inset`]), die Deckkraft ganzer Teilbäume ([`UiOpacity`]) und die
//! Markierung offener Dialoge ([`Modal`]).

use bevy::{ecs::spawn::SpawnableList, prelude::*, text::LetterSpacing};

use crate::palette;

/// Farbe aus sRGB-Bytes wie in CSS (`rgb(28 32 40)`).
pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    rgba(red, green, blue, 1.0)
}

/// Farbe aus sRGB-Bytes mit Deckkraft (`rgb(28 32 40 / 0.95)`).
pub const fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> Color {
    Color::srgba(
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        alpha,
    )
}

/// Eine Farbe der Palette mit anderer Deckkraft (`rgb(var(--player) / 0.6)`).
pub fn alpha(color: Color, alpha: f32) -> Color {
    color.with_alpha(alpha)
}

/// Fläche der Felder und Banner (`--panel`).
pub const PANEL: Color = rgba(28, 32, 40, 0.95);
/// Die dunkelste Kante: Ränder der Pulte und Fugen.
pub const INK: Color = rgb(4, 6, 9);
/// Grund unter Karten-Stapeln und Zählern.
pub const SHADE: Color = rgb(11, 14, 19);
/// Akzentfarbe der Oberfläche (`--accent`): die eigene Seite.
pub const ACCENT: Color = palette::PLAYER;

/// Sperrung des Display-Texts in em.
pub const TRACKING: f32 = 0.08;

/// Schriftgröße eines [`TextFont`] in logischen Pixeln.
pub fn font_px(font: &TextFont) -> f32 {
    match font.font_size {
        FontSize::Px(size) => size,
        _ => 16.0,
    }
}

/// Text in einer Schrift und Farbe.
pub fn text(font: TextFont, content: impl Into<String>, color: Color) -> impl Bundle {
    (Text::new(content), font, TextColor(color))
}

/// Display-Text: groß geschrieben und um `tracking` em gesperrt (`.round-label`, Knöpfe, Titel …).
pub fn caps(font: TextFont, content: &str, tracking: f32, color: Color) -> impl Bundle + use<> {
    let spacing = LetterSpacing::Px(font_px(&font) * tracking);
    (
        Text::new(content.to_uppercase()),
        font,
        spacing,
        TextColor(color),
    )
}

/// Schatten unter Text auf Bildern (`text-shadow: 0 1px 2px rgb(0 0 0 / 0.9)`).
pub fn text_shadow(opacity: f32) -> TextShadow {
    TextShadow {
        offset: Vec2::new(0.0, 1.0),
        color: Color::BLACK.with_alpha(opacity),
    }
}

/// Ein Schatten ohne Versatz und Unschärfe, nur verbreitert: ein Ring um die Fläche (`0 0 0 2px`).
pub fn ring(color: Color, width: f32) -> ShadowStyle {
    ShadowStyle {
        color,
        x_offset: Val::ZERO,
        y_offset: Val::ZERO,
        spread_radius: px(width),
        blur_radius: Val::ZERO,
    }
}

/// Ein weicher Schein um die Fläche (`0 0 14px color`).
pub fn glow(color: Color, blur: f32) -> ShadowStyle {
    ShadowStyle {
        color,
        x_offset: Val::ZERO,
        y_offset: Val::ZERO,
        spread_radius: Val::ZERO,
        blur_radius: px(blur),
    }
}

/// Ein Schatten mit Versatz (`x y blur color`).
pub fn drop(color: Color, x: f32, y: f32, blur: f32) -> ShadowStyle {
    ShadowStyle {
        color,
        x_offset: px(x),
        y_offset: px(y),
        spread_radius: Val::ZERO,
        blur_radius: px(blur),
    }
}

/// Ein Knoten, der den ganzen Elternknoten ausfüllt (`position: absolute; inset: 0`).
pub fn overlay() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::ZERO,
        right: Val::ZERO,
        top: Val::ZERO,
        bottom: Val::ZERO,
        ..default()
    }
}

/// Ein Bild, das seinen Knoten wie `background-size: cover` füllt: volle Breite, Höhe nach Seitenverhältnis
/// (`aspect`, Breite durch Höhe), senkrecht bei `focus` (0 oben, 0,5 Mitte) angesetzt. Der Elternknoten schneidet ab.
pub fn cover_image(image: Handle<Image>, aspect: f32, focus: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            min_height: percent(100),
            aspect_ratio: Some(aspect),
            // Oben bei `focus` der Elternhöhe, um `focus` der eigenen Höhe zurück: Der Überstand verteilt sich nach `focus`.
            top: percent(focus * 100.0),
            ..default()
        },
        UiTransform::from_translation(Val2::percent(0.0, -focus * 100.0)),
        ImageNode::new(image).with_mode(NodeImageMode::Stretch),
        Pickable::IGNORE,
    )
}

/// Verlauf von oben nach unten mit Haltepunkten in Prozent.
pub fn vertical_gradient(stops: &[(Color, f32)]) -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_bottom(
        stops
            .iter()
            .map(|&(color, at)| ColorStop::percent(color, at))
            .collect(),
    ))
}

/// Verlauf von links nach rechts mit Haltepunkten in Prozent.
pub fn horizontal_gradient(stops: &[(Color, f32)]) -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_right(
        stops
            .iter()
            .map(|&(color, at)| ColorStop::percent(color, at))
            .collect(),
    ))
}

/// Ein offener Dialog: Solange es einen gibt, schweigen die Tastenkürzel des HUD (V, F, Pfeiltasten) — die Tasten
/// gehören dann dem Dialog, wie im Original `dialog[open]`.
#[derive(Component, Debug, Default)]
pub struct Modal;

/// Deckkraft eines ganzen Teilbaums (CSS `opacity`): multipliziert die Deckkraft aller Farben darunter
/// (Hintergrund, Verläufe, Rand, Schatten, Text, Bilder). Die Farben, die beim ersten Anwenden dastehen, gelten als
/// voll deckend; gedacht für Inhalte, deren Farben sich während des Ausblendens nicht ändern (Banner, Untertitel,
/// gesperrte Karten). Geschachtelte [`UiOpacity`] werden nicht verrechnet.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct UiOpacity(pub f32);

/// Die volle Deckkraft der Farben eines Knotens, bevor [`UiOpacity`] sie verändert hat.
#[derive(Component, Clone, Debug, Default)]
struct BaseAlpha {
    background: Option<f32>,
    text: Option<f32>,
    image: Option<f32>,
    border: Option<[f32; 4]>,
    shadows: Vec<f32>,
    gradients: Vec<Vec<f32>>,
}

/// Die Haltepunkte eines Verlaufs.
fn stops_mut(gradient: &mut Gradient) -> Vec<&mut Color> {
    match gradient {
        Gradient::Linear(linear) => linear
            .stops
            .iter_mut()
            .map(|stop| &mut stop.color)
            .collect(),
        Gradient::Radial(radial) => radial
            .stops
            .iter_mut()
            .map(|stop| &mut stop.color)
            .collect(),
        Gradient::Conic(conic) => conic.stops.iter_mut().map(|stop| &mut stop.color).collect(),
    }
}

type Paint<'a> = (
    Option<&'a mut BackgroundColor>,
    Option<&'a mut TextColor>,
    Option<&'a mut ImageNode>,
    Option<&'a mut BorderColor>,
    Option<&'a mut BoxShadow>,
    Option<&'a mut BackgroundGradient>,
    Option<&'a BaseAlpha>,
);

fn apply_opacity(
    mut commands: Commands,
    roots: Query<(Entity, &UiOpacity), Changed<UiOpacity>>,
    children: Query<&Children>,
    mut paint: Query<Paint>,
) {
    for (root, &UiOpacity(opacity)) in &roots {
        for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
            let Ok((background, text, image, border, shadow, gradient, base)) =
                paint.get_mut(entity)
            else {
                continue;
            };
            let base = base.cloned().unwrap_or_else(|| {
                let base = BaseAlpha {
                    background: background.as_ref().map(|color| color.0.alpha()),
                    text: text.as_ref().map(|color| color.0.alpha()),
                    image: image.as_ref().map(|image| image.color.alpha()),
                    border: border.as_ref().map(|border| {
                        [border.top, border.right, border.bottom, border.left]
                            .map(|color| color.alpha())
                    }),
                    shadows: shadow
                        .as_ref()
                        .map(|shadow| shadow.iter().map(|style| style.color.alpha()).collect())
                        .unwrap_or_default(),
                    gradients: gradient
                        .as_ref()
                        .map(|gradients| {
                            gradients
                                .0
                                .iter()
                                .map(|gradient| {
                                    stops_mut(&mut gradient.clone())
                                        .into_iter()
                                        .map(|color| color.alpha())
                                        .collect()
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                commands.entity(entity).insert(base.clone());
                base
            });
            if let (Some(mut color), Some(full)) = (background, base.background) {
                color.0.set_alpha(full * opacity);
            }
            if let (Some(mut color), Some(full)) = (text, base.text) {
                color.0.set_alpha(full * opacity);
            }
            if let (Some(mut image), Some(full)) = (image, base.image) {
                image.color.set_alpha(full * opacity);
            }
            if let (Some(mut border), Some(full)) = (border, base.border) {
                let border = &mut *border;
                for (color, full) in [
                    &mut border.top,
                    &mut border.right,
                    &mut border.bottom,
                    &mut border.left,
                ]
                .into_iter()
                .zip(full)
                {
                    color.set_alpha(full * opacity);
                }
            }
            if let Some(mut shadow) = shadow {
                for (style, full) in shadow.iter_mut().zip(&base.shadows) {
                    style.color.set_alpha(full * opacity);
                }
            }
            if let Some(mut gradients) = gradient {
                for (gradient, fulls) in gradients.0.iter_mut().zip(&base.gradients) {
                    for (color, full) in stops_mut(gradient).into_iter().zip(fulls) {
                        color.set_alpha(full * opacity);
                    }
                }
            }
        }
    }
}

// --- Steuerpult der Dialoge (`.dialog`) ---

/// Schraubenkopf für die Ecken des Pults.
fn screw() -> impl Bundle {
    (
        Node {
            width: px(11),
            height: px(11),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundGradient::from(RadialGradient::new(
            UiPosition::new(Vec2::ZERO, percent(38), percent(34)),
            RadialGradientShape::FarthestCorner,
            vec![
                ColorStop::percent(rgb(214, 222, 234), 0.0),
                ColorStop::percent(rgb(108, 118, 134), 38.0),
                ColorStop::percent(rgb(34, 39, 48), 62.0),
                ColorStop::percent(rgba(0, 0, 0, 0.7), 70.0),
            ],
        )),
        Pickable::IGNORE,
    )
}

/// Ein Schraubenkopf an einer Stelle des Elternknotens.
fn screw_at(
    left: Option<f32>,
    right: Option<f32>,
    top: Option<f32>,
    bottom: Option<f32>,
) -> impl Bundle {
    let at = |value: Option<f32>| value.map_or(Val::Auto, px);
    (
        Node {
            position_type: PositionType::Absolute,
            left: at(left),
            right: at(right),
            top: at(top),
            bottom: at(bottom),
            ..default()
        },
        Pickable::IGNORE,
        children![screw()],
    )
}

/// Lüftungsschlitze unten links im Pult, auf Höhe der Knöpfe.
fn vents() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(32),
            bottom: px(30),
            column_gap: px(4),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn(SpawnIter((0..10).map(|_| {
            (
                Node {
                    width: px(3),
                    height: px(12),
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(rgb(3, 4, 6)),
            )
        }))),
    )
}

/// Das Steuerpult (`.dialog`): Metallgehäuse mit Rahmen aus Ringen, Schrauben unten in den Ecken und, mit `vents`,
/// Lüftungsschlitzen unten links. `node` legt die Größe fest (Breite, Höhe, Grenzen), `content` ist der Inhalt, als
/// erstes [`console_header`]. Abgeschrägte Ecken (`corner-shape: bevel`) kennt Bevy nicht; die Ecken sind leicht
/// gerundet.
pub fn console<C: SpawnableList<ChildOf> + Send + Sync + 'static>(
    node: Node,
    vents: bool,
    content: C,
) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(px(28), px(28), px(22), px(26)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..node
        },
        BackgroundGradient::from(LinearGradient::degrees(
            165.0,
            vec![
                ColorStop::percent(rgb(40, 47, 60), 0.0),
                ColorStop::percent(rgb(25, 29, 38), 50.0),
                ColorStop::percent(rgb(17, 20, 27), 100.0),
            ],
        )),
        BorderColor::all(INK),
        BoxShadow(vec![
            glow(alpha(palette::PLAYER, 0.12), 60.0),
            drop(rgba(0, 0, 0, 0.75), 0.0, 26.0, 80.0),
            ring(INK, 8.0),
            ring(rgba(255, 255, 255, 0.08), 6.0),
            ring(rgb(14, 17, 23), 5.0),
        ]),
        Children::spawn((
            Spawn(screw_at(Some(9.0), None, None, Some(9.0))),
            Spawn(screw_at(None, Some(9.0), None, Some(9.0))),
            SpawnIter(vents.then(self::vents).into_iter()),
            content,
        )),
    )
}

/// Kopfleiste des Pults (`.dialog h2`): eigenes Blech mit Schrauben, Kontrolllampe vor dem Titel, Statuslampen rechts
/// und Lichtkante darunter. Gehört als erstes Kind in [`console`] und reicht über dessen Innenabstand hinaus.
pub fn console_header(font: TextFont, title: &str) -> impl Bundle + use<> {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(14),
            margin: UiRect::new(px(-28), px(-28), px(-22), px(24)),
            padding: UiRect::new(px(34), px(34), px(14), px(12)),
            border: UiRect::bottom(px(1)),
            ..default()
        },
        vertical_gradient(&[
            (rgb(54, 62, 78), 0.0),
            (rgb(34, 40, 51), 55.0),
            (rgb(28, 33, 42), 100.0),
        ]),
        BorderColor::all(INK),
        BoxShadow(vec![
            drop(alpha(palette::PLAYER, 0.22), 0.0, 3.0, 16.0),
            drop(alpha(palette::PLAYER, 0.6), 0.0, 1.0, 0.0),
        ]),
        children![
            screw_at(Some(10.0), None, Some(15.0), None),
            screw_at(None, Some(10.0), Some(15.0), None),
            // Kontrolllampe.
            (
                Node {
                    width: px(10),
                    height: px(10),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::new(Vec2::ZERO, percent(35), percent(35)),
                    RadialGradientShape::FarthestCorner,
                    vec![
                        ColorStop::percent(Color::WHITE, 0.0),
                        ColorStop::percent(palette::SELECT, 45.0),
                        ColorStop::percent(alpha(palette::PLAYER, 0.7), 100.0),
                    ],
                )),
                BoxShadow(vec![
                    glow(palette::SELECT, 10.0),
                    ring(rgba(255, 255, 255, 0.1), 3.0),
                    ring(rgb(8, 10, 14), 2.0),
                ]),
            ),
            (
                caps(font.clone(), title, TRACKING, ACCENT),
                TextShadow {
                    offset: Vec2::ZERO,
                    color: alpha(palette::PLAYER, 0.55),
                },
            ),
            // Statuslampen rechts.
            (
                Node {
                    margin: UiRect::left(Val::Auto),
                    width: px(52),
                    height: px(8),
                    padding: UiRect::horizontal(px(5)),
                    column_gap: px(11),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(rgb(6, 8, 11)),
                Children::spawn(SpawnIter((0..3).map(|_| {
                    (
                        Node {
                            width: px(5),
                            height: px(5),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(palette::HEAL),
                        BoxShadow(vec![glow(alpha(palette::HEAL, 0.6), 3.0)]),
                    )
                }))),
            ),
        ],
    )
}

/// Ein eingelassenes Feld im Pult (`.console-panel`), das Regler und Listen trägt; `node` legt etwa die Lage im
/// Raster fest.
pub fn console_inset(node: Node) -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(px(16), px(16), px(16), px(8)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..node
        },
        BackgroundColor(rgba(0, 0, 0, 0.28)),
        BorderColor {
            top: INK,
            left: INK,
            right: rgba(255, 255, 255, 0.07),
            bottom: rgba(255, 255, 255, 0.07),
        },
    )
}

pub(super) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            apply_opacity.before(bevy::ui::UiSystems::Layout),
        );
    }
}
