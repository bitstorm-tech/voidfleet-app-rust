//! Illustrationen des Regel-Dialogs (Port von `ui/rulesFigures.ts`):
//!
//! - [`board`] und [`fire_orders`]: das Brett mit der Deckungsregel und die Feuerbefehle in vier Fällen. Die Geometrie
//!   ist gerastert (`images/rules/board.png`, `fire_orders.png` aus `tools/rasterize`, dort in `figures.rs`), die
//!   Beschriftung steht in der Sprache des Spielers an denselben Koordinaten des `viewBox` darüber und wächst mit der
//!   Breite der Figur wie im SVG.
//! - [`phases`]: die Phasen einer Runde, nummeriert.
//! - [`overdrive`]: Zeitleiste des Overdrives aus [`overdrive_bonus`].
//! - [`turn`]: die Teile eines Zugs aus verkleinerten Karten, das Manöver neben der Fähigkeitskarte, dazu die
//!   Feuerbefehle.

use bevy::{ecs::spawn::SpawnableList, prelude::*, text::LineHeight};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, overdrive_bonus},
    cards::{CommandId, ShipId},
};

use super::rules::RulesImages;
use crate::{
    i18n::{Locale, Phase},
    palette,
    ui::{
        Fonts,
        card::{CardAssets, command_card, power_card, ship_card},
        theme::{self, rgba},
    },
};

/// Grundlinie eines SVG-Texts unter der Oberkante seiner Zeile, in Schriftgrößen.
const BASELINE: f32 = 0.95;
/// Gedämpfte Beschriftung (`.note`, `.slot-label`).
const NOTE: Color = rgba(199, 219, 250, 0.6);

/// Wie ein SVG-Text an seinem Punkt hängt (`text-anchor`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Anchor {
    Start,
    Middle,
    End,
}

/// Eine Beschriftung im `viewBox` der Figur.
struct Label {
    x: f32,
    y: f32,
    text: String,
    size: f32,
    color: Color,
    anchor: Anchor,
}

impl Label {
    fn new(x: f32, y: f32, text: impl Into<String>, size: f32, color: Color) -> Self {
        Self {
            x,
            y,
            text: text.into(),
            size,
            color,
            anchor: Anchor::Start,
        }
    }

    fn anchored(self, anchor: Anchor) -> Self {
        Self { anchor, ..self }
    }
}

/// Eine gerasterte Figur; `width` ist die Breite ihres `viewBox`, an der die Schrift wächst.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct FigureCanvas {
    width: f32,
}

/// Eine Beschriftung einer Figur in ihrer Größe im `viewBox`.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct FigureLabel {
    size: f32,
}

/// Bild und Beschriftung; die Figur ist so breit wie der Text darüber.
fn labeled_figure(
    fonts: &Fonts,
    image: Handle<Image>,
    (width, height): (f32, f32),
    labels: Vec<Label>,
) -> impl Bundle {
    let labels: Vec<_> = labels
        .into_iter()
        .map(|label| {
            let horizontal = label.x / width * 100.0;
            let top = percent((label.y - BASELINE * label.size) / height * 100.0);
            let node = if label.anchor == Anchor::End {
                Node {
                    right: percent(100.0 - horizontal),
                    top,
                    ..default()
                }
            } else {
                Node {
                    left: percent(horizontal),
                    top,
                    ..default()
                }
            };
            // Mittig: um die halbe eigene Breite zurück.
            let shift = if label.anchor == Anchor::Middle {
                -50.0
            } else {
                0.0
            };
            (
                FigureLabel { size: label.size },
                Node {
                    position_type: PositionType::Absolute,
                    ..node
                },
                UiTransform::from_translation(Val2::percent(shift, 0.0)),
                theme::text(fonts.body(label.size), label.text, label.color),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            )
        })
        .collect();
    (
        FigureCanvas { width },
        Node {
            width: percent(100),
            aspect_ratio: Some(width / height),
            ..default()
        },
        ImageNode::new(image),
        Pickable::IGNORE,
        Children::spawn(SpawnIter(labels.into_iter())),
    )
}

/// Das Brett: oben der Gegner, unten die eigene Seite, darunter die Nummern der Slots, die zugleich die Spalten
/// zählen; ein gedecktes und ein ungedecktes Command-Schiff.
pub(super) fn board(fonts: &Fonts, locale: &Locale, images: &RulesImages) -> impl Bundle {
    let center = |index: usize| 100.0 + index as f32 * 86.0 + 40.0;
    let mut labels = vec![
        Label::new(0.0, 20.0, locale.figures_enemy(), 13.0, palette::ENEMY),
        Label::new(0.0, 242.0, locale.figures_you(), 13.0, palette::PLAYER),
    ];
    for (y, lane) in [
        (10.0, LaneType::Command),
        (64.0, LaneType::Strike),
        (150.0, LaneType::Strike),
        (204.0, LaneType::Command),
    ] {
        labels.push(Label::new(
            0.0,
            y + 27.0,
            locale.lane(lane),
            12.0,
            palette::TEXT,
        ));
    }
    labels.extend((0..6).map(|index| {
        Label::new(
            center(index),
            264.0,
            locale.figures_slot(index + 1),
            10.0,
            NOTE,
        )
        .anchored(Anchor::Middle)
    }));
    labels.extend([
        Label::new(
            center(1),
            280.0,
            locale.figures_covered(),
            11.0,
            palette::HEAL,
        )
        .anchored(Anchor::Middle),
        Label::new(
            center(4),
            280.0,
            locale.figures_exposed(),
            11.0,
            palette::DAMAGE,
        )
        .anchored(Anchor::Middle),
        Label::new(640.0, 310.0, locale.figures_lane_limit(), 10.0, NOTE).anchored(Anchor::End),
    ]);
    labeled_figure(fonts, images.board.clone(), (640.0, 316.0), labels)
}

/// Die Feuerbefehle in vier Fällen: oben Spaltenfeuer, unten Schiffsfeuer, je mit Titel und Kurztext; im ersten Fall
/// zählen Ziffern die Schüsse.
pub(super) fn fire_orders(fonts: &Fonts, locale: &Locale, images: &RulesImages) -> impl Bundle {
    let cases = [
        (
            locale.figures_column_fire(),
            locale.figures_column_through(),
        ),
        (locale.figures_column_fire(), locale.figures_column_dodge()),
        (
            locale.figures_tracking_fire(),
            locale.figures_tracking_follows(),
        ),
        (
            locale.figures_tracking_fire(),
            locale.figures_tracking_covered(),
        ),
    ];
    let mut labels = Vec::new();
    for (index, (title, note)) in cases.into_iter().enumerate() {
        let cx = 160.0 + (index % 2) as f32 * 320.0;
        let oy = (index / 2) as f32 * 176.0;
        labels.push(Label::new(cx, oy + 11.0, title, 12.0, palette::TEXT).anchored(Anchor::Middle));
        labels.push(Label::new(cx, oy + 24.0, note, 10.0, NOTE).anchored(Anchor::Middle));
    }
    // Die Reihenfolge der beiden Schüsse im ersten Fall, rechts neben den Schützen.
    labels.push(Label::new(112.0, 145.0, "1", 10.0, NOTE));
    labels.push(Label::new(240.0, 145.0, "2", 10.0, NOTE));
    labeled_figure(fonts, images.fire_orders.clone(), (640.0, 334.0), labels)
}

/// Die Phasen einer Runde in Reihenfolge, nummeriert, mit Kurztext.
pub(super) fn phases(fonts: &Fonts, locale: &Locale) -> impl Bundle {
    let items: Vec<_> = Phase::ALL
        .into_iter()
        .enumerate()
        .map(|(index, phase)| {
            let text = locale.phase(phase);
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    padding: UiRect::new(px(12), px(10), px(10), px(10)),
                    border: UiRect::top(px(2)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::WHITE.with_alpha(0.04)),
                BorderColor::all(palette::PLAYER.with_alpha(0.6)),
                Pickable::IGNORE,
                children![
                    (
                        Text::new(format!("{} · ", index + 1)),
                        fonts.body(15.0),
                        TextColor(palette::PLAYER),
                        Pickable::IGNORE,
                        children![(
                            TextSpan::new(text.name),
                            fonts.body(15.0),
                            TextColor(palette::TEXT)
                        )],
                    ),
                    (
                        theme::text(fonts.body(14.0), text.text, palette::TEXT_DIM),
                        LineHeight::RelativeToFont(1.4),
                        Pickable::IGNORE,
                    ),
                ],
            )
        })
        .collect();
    (
        Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::minmax(
                GridTrackRepetition::AutoFit,
                MinTrackSizingFunction::Px(130.0),
                MaxTrackSizingFunction::Fraction(1.0),
            )],
            row_gap: px(8),
            column_gap: px(8),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn(SpawnIter(items.into_iter())),
    )
}

/// Zeitleiste des Overdrives: je ein Abschnitt pro Bonus-Stufe, breit nach Rundenzahl, mit steigender Stufe kräftiger.
pub(super) fn overdrive(fonts: &Fonts, locale: &Locale) -> impl Bundle {
    const LAST_ROUND: u32 = 19;
    let mut segments = Vec::new();
    let mut from = 1;
    for round in 2..=LAST_ROUND + 1 {
        let bonus = overdrive_bonus(from);
        if round <= LAST_ROUND && overdrive_bonus(round) == bonus {
            continue;
        }
        let to = (round <= LAST_ROUND).then_some(round - 1);
        let level = bonus as f32;
        segments.push((
            Node {
                flex_grow: (round - from) as f32,
                flex_basis: Val::ZERO,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::top(px(2)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(palette::BUFF.with_alpha(0.06 + level * 0.14)),
            BorderColor::all(palette::BUFF.with_alpha(0.25 + level * 0.25)),
            Pickable::IGNORE,
            children![
                (
                    theme::text(
                        fonts.body(14.0),
                        locale.figures_rounds(from, to),
                        palette::TEXT_DIM
                    ),
                    Pickable::IGNORE,
                ),
                (
                    theme::text(
                        fonts.body(16.0),
                        locale.figures_overdrive(bonus),
                        palette::TEXT
                    ),
                    Pickable::IGNORE,
                ),
            ],
        ));
        from = round;
    }
    (
        Node {
            column_gap: px(3),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn(SpawnIter(segments.into_iter())),
    )
}

/// Maßstab der Karten im Bild eines Zugs (`.rules-turn .frame > * { scale: 0.7 }`).
const TURN_SCALE: f32 = 0.7;

/// Ein Teil des Zugs: Rahmen nebeneinander, unten bündig, darunter die Bezeichnung.
fn turn_part<F: SpawnableList<ChildOf> + Send + Sync + 'static>(
    fonts: &Fonts,
    frames: F,
    label: &str,
) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                Node {
                    align_items: AlignItems::FlexEnd,
                    column_gap: px(8),
                    ..default()
                },
                Pickable::IGNORE,
                Children::spawn(frames),
            ),
            (
                theme::text(fonts.body(15.0), label, palette::TEXT),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Ein Symbol im Bild eines Zugs, so groß wie die verkleinerten Karten.
fn turn_icon(image: Handle<Image>) -> impl Bundle {
    (
        Node {
            width: px(140.0 * TURN_SCALE),
            height: px(196.0 * TURN_SCALE),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(Color::WHITE.with_alpha(0.03)),
        BorderColor::all(Color::WHITE.with_alpha(0.1)),
        ImageNode::new(image),
        Pickable::IGNORE,
    )
}

fn plus(fonts: &Fonts) -> impl Bundle + use<> {
    (
        theme::text(fonts.body(26.0), "+", palette::TEXT_DIM),
        Pickable::IGNORE,
    )
}

/// Die Teile eines Zugs: Schiffskarte, Befehlskarte, das Manöver-Symbol oder die Admiralsfähigkeit und die
/// Feuerbefehle (zwei eigene Schiffe feuern über Kreuz auf zwei gegnerische).
pub(super) fn turn(assets: &CardAssets, images: &RulesImages) -> impl Bundle {
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let [ship_part, command_part, maneuver_part, fire_part] = locale.figures_turn_parts();
    (
        Node {
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            row_gap: px(14),
            column_gap: px(14),
            ..default()
        },
        Pickable::IGNORE,
        children![
            turn_part(
                fonts,
                Spawn(ship_card(assets, ShipId::VoidInterceptor, TURN_SCALE)),
                ship_part
            ),
            plus(fonts),
            turn_part(
                fonts,
                Spawn(command_card(assets, CommandId::Overcharge, TURN_SCALE)),
                command_part
            ),
            plus(fonts),
            turn_part(
                fonts,
                (
                    Spawn(turn_icon(images.turn_maneuver.clone())),
                    Spawn((
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        theme::text(fonts.body(14.0), locale.figures_or(), palette::TEXT_DIM),
                        Pickable::IGNORE,
                    )),
                    Spawn(power_card(assets, AdmiralId::Kerr, TURN_SCALE)),
                ),
                maneuver_part
            ),
            plus(fonts),
            turn_part(fonts, Spawn(turn_icon(images.turn_fire.clone())), fire_part),
            (
                Node {
                    margin: UiRect::left(Val::Auto),
                    ..default()
                },
                theme::text(
                    fonts.body(14.0),
                    locale.figures_round_one(),
                    palette::TEXT_DIM
                ),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Die Schrift der Beschriftung wächst mit der Breite ihrer Figur, wie im SVG.
pub(super) fn scale_labels(
    canvases: Query<(&FigureCanvas, &ComputedNode, &Children), Changed<ComputedNode>>,
    mut labels: Query<(&FigureLabel, &mut TextFont)>,
) {
    for (canvas, node, children) in &canvases {
        let scale = node.size().x * node.inverse_scale_factor() / canvas.width;
        for &child in children {
            if let Ok((label, mut font)) = labels.get_mut(child) {
                font.font_size = FontSize::Px(label.size * scale);
            }
        }
    }
}
