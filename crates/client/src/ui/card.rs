//! Karten als UI-Knoten (Port von `ui/cardView.ts` und der Karte aus `ui/powerCard.ts`), für Hand, Nachschub,
//! Admiralswahl und Regeln:
//!
//! - [`ship_card`]: Illustration als Hintergrund, oben und unten in die Farbe der Schiffsklasse verlaufend, Name,
//!   Klasse und Werte; Flaggschiffe mit goldenem Rahmen, Kopfleiste in der Farbe ihres Admirals und Krone.
//! - [`command_card`]: Datapad mit der Illustration im Farbton des Befehlstyps als Screen, Zielangabe, Effekttext
//!   und Tastenleiste.
//! - [`power_card`]: die Admiralsfähigkeit mit dem Porträt, flacher als die übrigen Karten.
//!
//! Alle nehmen einen Maßstab (`scale`): 1 ist die Größe in Regeln und Galerie (140 × 196), die Hand und der
//! Nachschub zoomen um 1,3 ([`HAND_ZOOM`]); Schrift und Abstände wachsen mit, die Schrift bleibt scharf.
//!
//! Zustände als Komponenten an der Karte: [`Marked`] (gewählt oder eingeplant: Rahmen in der Auswahlfarbe),
//! [`CardHover`] (wie die Karte auf den Zeiger reagiert) und [`CardDraw`] (fliegt als neu gezogene Karte ein).
//! Für „gesperrt“ dimmt [`super::theme::UiOpacity`] die ganze Karte.

use bevy::{
    ecs::system::SystemParam,
    picking::{events::PointerEnter, hover::Hovered},
    prelude::*,
    text::LineHeight,
};
use voidfleet_core::{
    admirals::AdmiralId,
    cards::{CommandId, ShipId},
};

use super::{Fonts, UiIcons, theme};
use crate::{
    audio::{PlayUiSound, UiSound},
    data::{
        admirals::{AdmiralPortraits, flagship_admiral},
        card_art::{Card, CardArt},
    },
    i18n::Locale,
    palette,
};

/// Größe einer Karte im Maßstab 1 und Höhe der Fähigkeitskarte.
pub const CARD_WIDTH: f32 = 140.0;
pub const CARD_HEIGHT: f32 = 196.0;
pub const POWER_CARD_HEIGHT: f32 = 150.0;
/// Zoom der Karten auf der Hand und im Nachschub.
pub const HAND_ZOOM: f32 = 1.3;
/// Seitenverhältnis der Illustrationen und Porträts (736 × 1024).
pub const ART_ASPECT: f32 = 736.0 / 1024.0;

/// Was eine Karte braucht: Schriften, Bilder und Texte.
#[derive(SystemParam)]
pub struct CardAssets<'w> {
    pub fonts: Res<'w, Fonts>,
    pub art: Res<'w, CardArt>,
    pub portraits: Res<'w, AdmiralPortraits>,
    pub icons: Res<'w, UiIcons>,
    pub locale: Res<'w, Locale>,
}

/// Eine Karte; `scale` ist ihr Maßstab.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CardView {
    pub card: CardFace,
    pub scale: f32,
}

/// Was auf der Karte steht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardFace {
    Ship(ShipId),
    Command(CommandId),
    Power(AdmiralId),
}

/// Die Karte ist gewählt oder eingeplant: Rahmen in der Auswahlfarbe.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Marked;

/// Wie eine Karte auf den Zeiger reagiert.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(Hovered)]
pub enum CardHover {
    /// Wächst auf `scale` (Hand: 1,5), damit der Text lesbar wird; `origin` ist der feste Punkt, x von −1 (links) bis
    /// 1 (rechts), y 1 unten — die äußeren Karten der Hand wachsen zur Bildschirmmitte.
    Zoom { scale: f32, origin: Vec2 },
    /// Hebt sich nur ein wenig an und leuchtet (Nachschub).
    Raise,
}

impl CardHover {
    /// Die Hand: von unten Mitte auf das Anderthalbfache.
    pub const HAND: Self = Self::Zoom {
        scale: 1.5,
        origin: Vec2::new(0.0, 1.0),
    };
}

/// Eine neu gezogene Karte fliegt nach `delay` Sekunden von unten ein und leuchtet danach kurz nach.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CardDraw {
    pub delay: f32,
    elapsed: f32,
}

impl CardDraw {
    pub fn after(delay: f32) -> Self {
        Self {
            delay,
            elapsed: 0.0,
        }
    }
}

/// Rahmen der Auswahl, sichtbar mit [`Marked`].
#[derive(Component)]
struct SelectionFrame;

/// Schein um die Karte: beim Einfliegen und beim Anheben.
#[derive(Component)]
struct Halo;

/// Laufender Stand des Hovers (0–1).
#[derive(Component, Default)]
struct HoverProgress(f32);

/// Schein um die Karte, beim Einfliegen und beim Anheben: als erstes Kind, damit der Inhalt das Innere des
/// Schattens verdeckt (Bevy zeichnet Schatten auch unter der Fläche, CSS nur außen).
fn halo(radius: f32) -> impl Bundle {
    (
        Halo,
        Node {
            border_radius: BorderRadius::all(px(radius)),
            ..theme::overlay()
        },
        BoxShadow::default(),
        Pickable::IGNORE,
    )
}

/// Rahmen der Auswahl, zuoberst.
fn selection_frame(scale: f32, radius: f32) -> impl Bundle {
    (
        SelectionFrame,
        Node {
            border: UiRect::all(px(3.0 * scale)),
            border_radius: BorderRadius::all(px(radius * scale)),
            ..theme::overlay()
        },
        BorderColor::all(palette::SELECT),
        Visibility::Hidden,
        Pickable::IGNORE,
    )
}

/// Das Bild einer Karte, auf ihre Fläche zugeschnitten (der Wrapper schneidet ab, nicht die Karte, damit Zähler
/// über den Rand ragen dürfen).
fn clipped_art(image: Handle<Image>, focus: f32, radius: f32) -> impl Bundle {
    (
        Node {
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(px(radius)),
            ..theme::overlay()
        },
        Pickable::IGNORE,
        children![theme::cover_image(image, ART_ASPECT, focus)],
    )
}

/// Ein Verlauf über der ganzen Fläche.
fn shade(stops: &[(Color, f32)], radius: f32) -> impl Bundle + use<> {
    (
        Node {
            border_radius: BorderRadius::all(px(radius)),
            ..theme::overlay()
        },
        theme::vertical_gradient(stops),
        Pickable::IGNORE,
    )
}

/// Ein Text auf einer Karte: Schatten fürs Bild dahinter, nicht anklickbar.
fn card_text(
    font: TextFont,
    content: impl Into<String>,
    color: Color,
    justify: Justify,
) -> impl Bundle {
    (
        theme::text(font, content, color),
        theme::text_shadow(0.9),
        TextLayout::justify(justify),
        Pickable::IGNORE,
    )
}

/// Ein Wert der Schiffskarte, Bezeichnung oder Zahl.
fn stat(font: TextFont, content: &str, color: Color, justify: Justify) -> impl Bundle + use<> {
    (
        theme::caps(font, content, 0.0, color),
        theme::text_shadow(0.9),
        TextLayout::justify(justify),
        Pickable::IGNORE,
    )
}

/// Eine Schiffskarte (auch Flaggschiffe und Tokens).
pub fn ship_card(assets: &CardAssets, ship: ShipId, scale: f32) -> impl Bundle {
    let s = |value: f32| px(value * scale);
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let card = ship.card();
    let color = palette::ship_class_color(card.class);
    let background = palette::darkened(color, 0.72);
    let admiral = flagship_admiral(ship);
    let accent = admiral.map_or(color, palette::admiral_color);
    let mut stats = vec![
        (locale.stats_atk(), card.atk.to_string()),
        (locale.stats_hp(), card.hp.to_string()),
    ];
    if card.shield > 0 {
        stats.push((locale.stats_shield(), card.shield.to_string()));
    }
    let label_font = fonts.body(13.0 * scale);
    let value_font = fonts.display_bold(16.0 * scale);
    let stat_cells = stats.into_iter().flat_map(move |(label, value)| {
        [
            stat(
                label_font.clone(),
                label,
                theme::rgba(158, 184, 224, 0.9),
                Justify::Left,
            ),
            stat(value_font.clone(), &value, palette::TEXT, Justify::Right),
        ]
    });
    // Flaggschiff: goldener Rahmen innen und Krone oben rechts.
    let crown = assets.icons.crown.clone();
    let flagship = admiral.map(move |_| {
        (
            Node {
                border: UiRect::all(px(2.0 * scale)),
                ..theme::overlay()
            },
            BorderColor::all(palette::FLAGSHIP_TRIM.with_alpha(0.85)),
            Pickable::IGNORE,
            children![(
                Node {
                    position_type: PositionType::Absolute,
                    top: px(4.0 * scale),
                    right: px(5.0 * scale),
                    width: px(20.0 * scale),
                    height: px(20.0 * scale),
                    ..default()
                },
                ImageNode::new(crown).with_color(palette::FLAGSHIP_TRIM),
                BoxShadow(vec![theme::glow(Color::BLACK.with_alpha(0.5), 4.0 * scale)]),
                Pickable::IGNORE,
            )],
        )
    });
    (
        CardView {
            card: CardFace::Ship(ship),
            scale,
        },
        Node {
            width: s(CARD_WIDTH),
            height: s(CARD_HEIGHT),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(s(10.0), s(10.0), s(16.0), s(12.0)),
            border: UiRect::top(s(7.0)),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(accent),
        BoxShadow(if admiral.is_some() {
            vec![theme::glow(
                palette::FLAGSHIP_TRIM.with_alpha(0.35),
                16.0 * scale,
            )]
        } else {
            Vec::new()
        }),
        UiTransform::default(),
        Children::spawn((
            Spawn(halo(0.0)),
            Spawn(clipped_art(assets.art.get(Card::Ship(ship)), 0.5, 0.0)),
            Spawn(shade(
                &[
                    (background, 0.0),
                    (Color::NONE, 34.0),
                    (Color::NONE, 56.0),
                    (background, 92.0),
                ],
                0.0,
            )),
            SpawnIter(flagship.into_iter()),
            Spawn((
                Node {
                    padding: UiRect::right(s(if admiral.is_some() { 18.0 } else { 0.0 })),
                    ..default()
                },
                Pickable::IGNORE,
                children![card_text(
                    fonts.display_bold(16.0 * scale),
                    locale.ship(ship).name,
                    palette::TEXT,
                    Justify::Left,
                )],
            )),
            Spawn((
                Node {
                    margin: UiRect::top(s(4.0)),
                    ..default()
                },
                Pickable::IGNORE,
                children![stat(
                    fonts.body(11.0 * scale),
                    locale.ship_class(card.class),
                    theme::rgba(168, 199, 242, 0.85),
                    Justify::Left,
                )],
            )),
            Spawn((
                Node {
                    margin: UiRect::top(Val::Auto),
                    display: Display::Grid,
                    grid_template_columns: vec![
                        RepeatedGridTrack::fr(1, 1.0),
                        RepeatedGridTrack::auto(1),
                    ],
                    row_gap: s(4.0),
                    align_items: AlignItems::Baseline,
                    ..default()
                },
                Pickable::IGNORE,
                Children::spawn(SpawnIter(stat_cells)),
            )),
            Spawn(selection_frame(scale, 0.0)),
        )),
    )
}

/// Farbe des Screens einer Befehlskarte (`--card-rgb`).
fn screen_color(command: CommandId) -> Color {
    palette::darkened(palette::command_type_color(command.command_type()), 0.25)
}

/// Eine Befehlskarte als Datapad (auch die Schrottverwertung mit dem Bild des Scrap Raiders).
pub fn command_card(assets: &CardAssets, command: CommandId, scale: f32) -> impl Bundle {
    let s = |value: f32| px(value * scale);
    let fonts = &assets.fonts;
    let text = assets.locale.command(command);
    let target = text
        .target
        .unwrap_or_else(|| assets.locale.target_mode(command.target_mode()));
    let tint = screen_color(command);
    let key = move |width: f32| {
        (
            Node {
                width: px(width * scale),
                height: px(7.0 * scale),
                border_radius: BorderRadius::all(px(scale)),
                ..default()
            },
            BackgroundColor(theme::rgb(14, 16, 20)),
            Pickable::IGNORE,
        )
    };
    (
        CardView {
            card: CardFace::Command(command),
            scale,
        },
        Node {
            width: s(CARD_WIDTH),
            height: s(CARD_HEIGHT),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(s(6.0), s(6.0), s(8.0), s(6.0)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(s(4.0)),
            ..default()
        },
        BackgroundColor(theme::rgb(26, 28, 34)),
        BorderColor::all(Color::WHITE.with_alpha(0.08)),
        BoxShadow::default(),
        UiTransform::default(),
        Children::spawn((
            Spawn(halo(4.0 * scale)),
            Spawn((
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: s(5.0),
                    padding: UiRect::axes(s(7.0), s(10.0)),
                    border_radius: BorderRadius::all(s(2.0)),
                    ..default()
                },
                BoxShadow(vec![theme::ring(Color::BLACK.with_alpha(0.35), 1.0)]),
                Pickable::IGNORE,
                children![
                    clipped_art(assets.art.get(Card::Command(command)), 0.5, 2.0 * scale),
                    shade(
                        &[
                            (tint.with_alpha(0.92), 0.0),
                            (tint.with_alpha(0.15), 30.0),
                            (tint.with_alpha(0.15), 52.0),
                            (tint.with_alpha(0.94), 72.0),
                        ],
                        2.0 * scale,
                    ),
                    card_text(
                        fonts.display_bold(15.0 * scale),
                        text.name,
                        theme::rgb(247, 252, 255),
                        Justify::Center
                    ),
                    card_text(
                        fonts.body(9.0 * scale),
                        target,
                        theme::rgba(191, 224, 255, 0.9),
                        Justify::Center
                    ),
                    (
                        Node {
                            margin: UiRect::top(Val::Auto),
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![(
                            card_text(
                                fonts.body(8.5 * scale),
                                text.effect,
                                theme::rgb(235, 242, 255),
                                Justify::Center
                            ),
                            LineHeight::RelativeToFont(1.35),
                        )],
                    ),
                ],
            )),
            Spawn((
                Node {
                    justify_content: JustifyContent::Center,
                    column_gap: s(5.0),
                    margin: UiRect::top(s(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
                children![key(14.0), key(14.0), key(24.0), key(14.0)],
            )),
            Spawn(selection_frame(scale, 4.0)),
        )),
    )
}

/// Die Karte der Admiralsfähigkeit: Porträt als Hintergrund, Rahmen und Schrift in der Farbe des Admirals.
pub fn power_card(assets: &CardAssets, admiral: AdmiralId, scale: f32) -> impl Bundle {
    let s = |value: f32| px(value * scale);
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let color = palette::admiral_color(admiral);
    let power = admiral.power();
    let text = locale.power(power);
    let ink = theme::rgb(14, 16, 22);
    (
        CardView {
            card: CardFace::Power(admiral),
            scale,
        },
        Node {
            width: s(CARD_WIDTH),
            height: s(POWER_CARD_HEIGHT),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: s(3.0),
            padding: UiRect::new(s(8.0), s(8.0), s(8.0), s(7.0)),
            border: UiRect::new(px(1), px(1), s(5.0), px(1)),
            border_radius: BorderRadius::all(s(4.0)),
            ..default()
        },
        BackgroundColor(ink),
        BorderColor {
            top: color,
            ..BorderColor::all(color.with_alpha(0.75))
        },
        BoxShadow(vec![theme::glow(color.with_alpha(0.35), 14.0 * scale)]),
        UiTransform::default(),
        Children::spawn((
            Spawn(halo(4.0 * scale)),
            Spawn(clipped_art(assets.portraits.get(admiral), 0.22, 0.0)),
            Spawn(shade(
                &[
                    (ink.with_alpha(0.9), 0.0),
                    (ink.with_alpha(0.35), 34.0),
                    (ink.with_alpha(0.55), 55.0),
                    (ink.with_alpha(0.95), 80.0),
                ],
                0.0,
            )),
            Spawn((
                theme::caps(fonts.display(9.5 * scale), locale.hud_power(), 0.1, color),
                theme::text_shadow(0.9),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            )),
            Spawn(card_text(
                fonts.display_bold(15.0 * scale),
                text.name,
                palette::TEXT,
                Justify::Center,
            )),
            Spawn(card_text(
                fonts.body(9.0 * scale),
                locale.power_target(power.target()),
                theme::rgba(191, 224, 255, 0.9),
                Justify::Center,
            )),
            Spawn((
                Node {
                    margin: UiRect::top(Val::Auto),
                    ..default()
                },
                Pickable::IGNORE,
                children![(
                    card_text(
                        fonts.body(8.5 * scale),
                        text.effect,
                        theme::rgb(235, 242, 255),
                        Justify::Center
                    ),
                    LineHeight::RelativeToFont(1.3),
                )],
            )),
            Spawn(card_text(
                fonts.body(8.0 * scale),
                locale.hud_power_hint(),
                color,
                Justify::Center,
            )),
            Spawn(selection_frame(scale, 4.0)),
        )),
    )
}

/// Zeigt den Rahmen gewählter und eingeplanter Karten.
fn show_marks(
    cards: Query<(&Children, Has<Marked>), With<CardView>>,
    mut frames: Query<&mut Visibility, With<SelectionFrame>>,
) {
    for (children, marked) in &cards {
        for &child in children {
            if let Ok(mut visibility) = frames.get_mut(child) {
                visibility.set_if_neq(if marked {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
}

/// Dauer des Einflugs, des Nachleuchtens und des Hover-Zooms (Sekunden).
const DRAW_FLIGHT: f32 = 0.45;
const DRAW_GLOW: f32 = 2.4;
const HOVER_TIME: f32 = 0.12;

/// `cubic-bezier(x1, y1, x2, y2)` aus CSS bei `t` (0–1): erst x nach dem Kurvenparameter auflösen, dann y.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let curve = |a: f32, b: f32, u: f32| {
        3.0 * a * u * (1.0 - u).powi(2) + 3.0 * b * u.powi(2) * (1.0 - u) + u.powi(3)
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        if curve(x1, x2, mid) < t {
            low = mid;
        } else {
            high = mid;
        }
    }
    curve(y1, y2, (low + high) / 2.0)
}

type CardMotion<'a> = (
    Entity,
    &'a CardView,
    Option<&'a CardHover>,
    Option<&'a Hovered>,
    Option<&'a mut CardDraw>,
    &'a mut HoverProgress,
    &'a mut UiTransform,
    &'a mut ZIndex,
    &'a Children,
);

/// Hover-Zoom, Anheben und Einflug als Transformation der Karte, dazu der Schein.
fn animate_cards(
    mut commands: Commands,
    time: Res<Time>,
    mut cards: Query<CardMotion>,
    mut halos: Query<&mut BoxShadow, With<Halo>>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let delta = time.delta_secs();
    for (entity, view, hover, hovered, draw, mut progress, mut transform, mut z, children) in
        &mut cards
    {
        let hovering = hovered.is_some_and(Hovered::get) && hover.is_some();
        let step = delta / HOVER_TIME;
        progress.0 = if hovering {
            (progress.0 + step).min(1.0)
        } else {
            (progress.0 - step).max(0.0)
        };
        let eased = 1.0 - (1.0 - progress.0).powi(2);
        let (mut scale, mut shift, mut halo) = (1.0, Vec2::ZERO, Vec::new());
        match hover {
            Some(CardHover::Zoom {
                scale: grown,
                origin,
            }) => {
                scale = 1.0 + (grown - 1.0) * eased;
                // Um den festen Punkt wachsen: verschieben um den halben Zuwachs in seine Richtung (Prozent der Größe).
                shift = -origin * (scale - 1.0) * 50.0;
            }
            Some(CardHover::Raise) => {
                shift.y = -4.0 * eased / (CARD_HEIGHT * view.scale) * 100.0;
                if eased > 0.0 {
                    halo.push(theme::glow(palette::SELECT.with_alpha(0.45 * eased), 22.0));
                }
            }
            None => {}
        }
        if let Some(mut draw) = draw {
            let before = draw.elapsed;
            draw.elapsed += delta;
            if before <= draw.delay && draw.elapsed > draw.delay {
                sounds.write(PlayUiSound(UiSound::Draw));
            }
            let since = draw.elapsed - draw.delay;
            let flight = (since / DRAW_FLIGHT).clamp(0.0, 1.0);
            shift.y += 120.0 * (1.0 - cubic_bezier(0.2, 0.9, 0.3, 1.15, flight));
            // Nachleuchten: ein Drittel voll, dann ausblenden.
            let glow = 1.0 - ((since / DRAW_GLOW - 0.3) / 0.7).clamp(0.0, 1.0);
            if since >= 0.0 && glow > 0.0 {
                halo.push(theme::glow(
                    palette::SELECT.with_alpha(0.8 * glow),
                    24.0 * view.scale,
                ));
                halo.push(theme::ring(
                    palette::SELECT.with_alpha(glow),
                    2.0 * view.scale,
                ));
            }
            if since > DRAW_GLOW {
                commands.entity(entity).remove::<CardDraw>();
            }
        }
        let next = UiTransform {
            translation: Val2::percent(shift.x, shift.y),
            scale: Vec2::splat(scale),
            ..default()
        };
        if *transform != next {
            *transform = next;
        }
        z.set_if_neq(ZIndex(i32::from(progress.0 > 0.0)));
        for &child in children {
            if let Ok(mut shadow) = halos.get_mut(child)
                && shadow.0 != halo
            {
                shadow.0.clone_from(&halo);
            }
        }
    }
}

/// Beim Überfahren einer Karte tickt es leise.
fn hover_sound(
    enter: On<PointerEnter>,
    cards: Query<(), (With<CardView>, With<CardHover>)>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if cards.contains(enter.entity) {
        sounds.write(PlayUiSound(UiSound::Hover));
    }
}

/// Jede Karte bekommt ihren Hover-Stand und eine Ebene, auf der sie sich über die Nachbarn heben kann.
fn prepare_cards(mut commands: Commands, cards: Query<Entity, Added<CardView>>) {
    for card in &cards {
        commands
            .entity(card)
            .insert((HoverProgress::default(), ZIndex::default()));
    }
}

pub(super) struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(hover_sound)
            .add_systems(Update, (prepare_cards, show_marks, animate_cards).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_matches_css_endpoints() {
        assert!(cubic_bezier(0.2, 0.9, 0.3, 1.15, 0.0).abs() < 1e-4);
        assert!((cubic_bezier(0.2, 0.9, 0.3, 1.15, 1.0) - 1.0).abs() < 1e-4);
        // Die Kurve schießt über das Ziel hinaus, bevor sie sich setzt.
        assert!((10..20).any(|i| cubic_bezier(0.2, 0.9, 0.3, 1.15, i as f32 / 20.0) > 1.0));
    }
}
