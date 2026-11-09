//! Tooltips (Ersatz für das `title`-Attribut des Browsers): Eine Entity mit [`Tooltip`] zeigt ihren Text, sobald der
//! Zeiger eine Weile darüber ruht, in einem kleinen Feld neben dem Zeiger. Es gibt nur ein Feld; es liegt über allem.
//! Statt Text kann eine Entity [`TooltipGlyphs`] tragen: je Zeile ein [`Glyph`] und seine Erklärung;
//! solche Felder erscheinen sofort, ebenso Text mit [`TooltipInstant`].

use bevy::{
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    window::PrimaryWindow,
};

use voidfleet_core::board::COLUMNS;

use super::{Fonts, cursor, theme};
use crate::{palette, settings::Settings};

/// Der Text, der beim Überfahren erscheint; leer zeigt nichts.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct Tooltip(pub String);

/// Der [`Tooltip`] dieser Entity erscheint ohne Wartezeit (Schlüsselwörter auf Karten).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TooltipInstant;

/// Ein Icon in seiner Farbe, mit Zahl und, beim Spaltenfeuer, den angesagten Spalten.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub icon: Handle<Image>,
    pub color: Color,
    pub value: Option<i32>,
    pub columns: Option<[bool; COLUMNS]>,
}

/// Zeilen aus Icon und Erklärung statt Text.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct TooltipGlyphs(pub Vec<(Glyph, String)>);

/// So lange (Sekunden) muss der Zeiger ruhen, bis ein Text-Feld erscheint, und sein Abstand zum Bild des Zeigers.
const DELAY: f32 = 0.5;
const GAP: f32 = 6.0;
const MAX_WIDTH: f32 = 300.0;
/// Die Breitengrenze innerhalb von Rand und Polster. Sie steht auch an Text und Zeilen: Taffy misst den Text sonst
/// ungebrochen, das Feld bekäme die Höhe einer Zeile und der umbrochene Text hinge unten heraus.
const INNER_WIDTH: f32 = MAX_WIDTH - 2.0 * (8.0 + 1.0);

/// Das eine Feld.
#[derive(Component)]
struct TooltipBox;

#[derive(Component)]
struct TooltipText;

/// Die Zeilen der [`TooltipGlyphs`] im Feld.
#[derive(Component)]
struct TooltipRows;

/// Welche Entity das Feld zuletzt meinte und seit wann; dazu die Zeilen, die es zeigt.
#[derive(Resource, Default)]
struct TooltipHover {
    entity: Option<Entity>,
    since: f32,
    glyphs: TooltipGlyphs,
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        Name::new("Tooltip"),
        TooltipBox,
        Node {
            position_type: PositionType::Absolute,
            max_width: px(MAX_WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::axes(px(8), px(5)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme::PANEL.with_alpha(1.0)),
        BorderColor::all(Color::WHITE.with_alpha(0.12)),
        BoxShadow(vec![theme::drop(
            Color::BLACK.with_alpha(0.5),
            0.0,
            4.0,
            12.0,
        )]),
        GlobalZIndex(200),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            (
                TooltipText,
                theme::text(fonts.body(13.0), "", palette::TEXT),
                Node {
                    max_width: px(INNER_WIDTH),
                    ..default()
                },
                Pickable::IGNORE,
            ),
            (
                TooltipRows,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ),
        ],
    ));
}

/// Zeigt den Tooltip der überfahrenen Entity nach der Wartezeit neben dem Zeiger.
fn follow_pointer(
    time: Res<Time>,
    mut hover: ResMut<TooltipHover>,
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    tooltips: Query<(
        Option<&Tooltip>,
        Option<&TooltipGlyphs>,
        Has<TooltipInstant>,
    )>,
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<Settings>,
    tooltip_box: Single<(&mut Node, &mut Visibility, &ComputedNode), With<TooltipBox>>,
    text: Single<(&mut Text, &mut Node), (With<TooltipText>, Without<TooltipBox>)>,
    rows: Single<Entity, With<TooltipRows>>,
    fonts: Res<Fonts>,
    mut commands: Commands,
) {
    // Der innerste Tooltip mit Text gewinnt: vom vordersten Treffer des Zeigers aus der erste nach außen.
    let hovered = hover_map
        .get(&PointerId::Mouse)
        .into_iter()
        .flatten()
        .filter_map(|(&hit, data)| {
            std::iter::once(hit)
                .chain(parents.iter_ancestors(hit))
                .find_map(|entity| {
                    let (text, glyphs, instant) = tooltips.get(entity).ok()?;
                    let text = text.filter(|text| !text.0.is_empty());
                    let glyphs = glyphs.filter(|glyphs| !glyphs.0.is_empty());
                    (text.is_some() || glyphs.is_some())
                        .then_some((data.depth, entity, text, glyphs, instant))
                })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let now = time.elapsed_secs();
    let current = hovered.map(|(_, entity, ..)| entity);
    if current != hover.entity {
        hover.entity = current;
        hover.since = now;
    }
    let (mut node, mut visibility, computed) = tooltip_box.into_inner();
    let (Some((_, _, tooltip, glyphs, instant)), Some(cursor)) =
        (hovered, window.cursor_position())
    else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    // Die Icons über den Schiffen (Glyphen) erklären sich sofort.
    if glyphs.is_none() && !instant && now - hover.since < DELAY {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    }
    let content = tooltip.map_or("", |tooltip| tooltip.0.as_str());
    let (mut text, mut text_node) = text.into_inner();
    if text.0 != content {
        text.0 = content.to_owned();
    }
    let display = if content.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if text_node.display != display {
        text_node.display = display;
    }
    let glyphs = glyphs.cloned().unwrap_or_default();
    if hover.glyphs != glyphs {
        let rows = rows.into_inner();
        commands.entity(rows).despawn_related::<Children>();
        commands.entity(rows).with_children(|rows| {
            for (glyph, line) in &glyphs.0 {
                rows.spawn((
                    Node {
                        max_width: px(INNER_WIDTH),
                        column_gap: px(8),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    spawn_glyph(row, glyph, 18.0, &fonts);
                    row.spawn((
                        theme::text(fonts.body(13.0), line.clone(), palette::TEXT),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
        hover.glyphs = glyphs;
    }
    // Das Feld steht über der Spitze des Zeigers: Das Bild reicht von dort fast nur nach unten, auch wenn das System
    // den Zeiger vergrößert. Am oberen Rand steht es unter dem Bild; am rechten Rand rückt es nach links, ohne den
    // Zeiger zu überdecken, da es senkrecht neben ihm steht.
    let size = computed.size() * computed.inverse_scale_factor();
    let (above, below) = cursor::reach(&settings);
    let mut at = Vec2::new(cursor.x, cursor.y + above.y - size.y - GAP);
    if at.y < 0.0 {
        at.y = cursor.y + below.y + GAP;
    }
    at.x = at.x.min(window.width() - size.x);
    node.left = px(at.x.max(0.0));
    node.top = px(at.y.max(0.0));
    visibility.set_if_neq(Visibility::Inherited);
}

/// Ein [`Glyph`] in `size` logischen Pixeln: Icon, Zahl daneben, die Spalten als drei Striche von links nach rechts.
pub fn spawn_glyph(parent: &mut ChildSpawnerCommands, glyph: &Glyph, size: f32, fonts: &Fonts) {
    parent
        .spawn((
            Node {
                column_gap: px(3),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|item| {
            item.spawn((
                ImageNode::new(glyph.icon.clone()).with_color(glyph.color),
                Node {
                    width: px(size),
                    height: px(size),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            if let Some(value) = glyph.value {
                item.spawn((
                    theme::text(
                        fonts.display_bold(size * 0.9),
                        value.to_string(),
                        glyph.color,
                    ),
                    theme::text_shadow(0.9),
                    Pickable::IGNORE,
                ));
            }
            if let Some(columns) = glyph.columns {
                item.spawn((
                    Node {
                        column_gap: px(2),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|pips| {
                    for on in columns {
                        pips.spawn((
                            Node {
                                width: px(size * 0.22),
                                height: px(size * 0.6),
                                ..default()
                            },
                            BackgroundColor(if on {
                                glyph.color
                            } else {
                                glyph.color.with_alpha(0.25)
                            }),
                            Pickable::IGNORE,
                        ));
                    }
                });
            }
        });
}

pub(super) struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipHover>()
            .add_systems(Startup, spawn)
            .add_systems(Update, follow_pointer);
    }
}
