//! Befehlskarten als UI-Knoten (Port von `ui/cardView.ts`): [`command_card`] ist ein Datapad mit der Illustration im
//! Farbton des Kartentyps als Screen, den Kosten als Zahl oben links, Typ und Zielangabe, Effekttext und
//! Tastenleiste; aufgewertete Karten tragen „+“ am Namen und die erhöhten Werte im Text.
//!
//! Die Karte nimmt einen Maßstab (`scale`): 1 ist die Grundgröße (140 × 196), die Hand zoomt um 1,3 ([`HAND_ZOOM`]);
//! Schrift und Abstände wachsen mit, die Schrift bleibt scharf.
//!
//! Zustände als Komponenten an der Karte: [`Marked`] (gewählt oder eingeplant: Rahmen in der Auswahlfarbe),
//! [`CardHover`] (wie die Karte auf den Zeiger reagiert) und [`CardDraw`] (fliegt als neu gezogene Karte ein).
//! Für „gesperrt“ dimmt [`super::theme::UiOpacity`] die ganze Karte.

use bevy::{
    ecs::system::SystemParam,
    picking::{events::PointerEnter, hover::Hovered},
    prelude::*,
    text::LineHeight,
    time::Stopwatch,
};
use voidfleet_core::cards::DeckCard;

use super::{
    Fonts, theme,
    tooltip::{Tooltip, TooltipInstant},
};
use crate::{
    audio::{PlayUiSound, UiSound},
    data::card_art::{Card, CardArt},
    i18n::Locale,
    palette,
};

/// Größe einer Karte im Maßstab 1.
pub const CARD_WIDTH: f32 = 140.0;
pub const CARD_HEIGHT: f32 = 196.0;
/// Zoom der Karten auf der Hand.
pub const HAND_ZOOM: f32 = 1.3;
/// Seitenverhältnis der Illustrationen und Porträts (736 × 1024).
pub const ART_ASPECT: f32 = 736.0 / 1024.0;

/// Was eine Karte braucht: Schriften, Bilder und Texte.
#[derive(SystemParam)]
pub struct CardAssets<'w> {
    pub fonts: Res<'w, Fonts>,
    pub art: Res<'w, CardArt>,
    pub locale: Res<'w, Locale>,
}

/// Eine Karte; `scale` ist ihr Maßstab. Ihre Ebene ([`ZIndex`] von [`Node`]) hebt sie beim Hover über die Nachbarn.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(HoverProgress)]
pub struct CardView {
    pub card: DeckCard,
    pub scale: f32,
}

/// Die Karte ist gewählt: Rahmen in der Auswahlfarbe.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Marked;

/// Wie eine Karte auf den Zeiger reagiert.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(Hovered)]
pub enum CardHover {
    /// Wächst auf `scale` (Hand: 1,5), damit der Text lesbar wird; `origin` ist der feste Punkt, x von −1 (links) bis
    /// 1 (rechts), y 1 unten — die äußeren Karten der Hand wachsen zur Bildschirmmitte.
    Zoom { scale: f32, origin: Vec2 },
    /// Hebt sich nur ein wenig an und leuchtet.
    Raise,
}

/// Eine neu gezogene Karte fliegt nach einer Wartezeit von unten ein und leuchtet danach kurz nach.
#[derive(Component, Clone, Debug)]
pub struct CardDraw {
    /// Bis zum Einflug.
    delay: Timer,
    /// Seit dem Einflug.
    flight: Stopwatch,
}

impl CardDraw {
    /// Fliegt nach `delay` Sekunden ein.
    pub fn after(delay: f32) -> Self {
        Self {
            delay: Timer::from_seconds(delay, TimerMode::Once),
            flight: Stopwatch::new(),
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

/// Zerlegt einen Effekttext an seinen Schlüsselwörtern: je Stück der Text und, bei einem Schlüsselwort, seine
/// Erklärung. Ein Schlüsselwort ist ein ganzes Wort, Groß- und Kleinschreibung zählt nicht.
fn keyword_chunks<'a>(
    effect: &'a str,
    keywords: &[(&'static str, &'static str)],
) -> Vec<(&'a str, Option<&'static str>)> {
    let mut chunks = Vec::new();
    let mut plain = 0;
    let mut start = None;
    for (at, c) in effect.char_indices().chain([(effect.len(), ' ')]) {
        match (start, c.is_alphanumeric()) {
            (None, true) => start = Some(at),
            (Some(from), false) => {
                start = None;
                let word = effect[from..at].to_lowercase();
                if let Some((_, explanation)) = keywords
                    .iter()
                    .find(|(name, _)| name.to_lowercase() == word)
                {
                    chunks.push((&effect[plain..from], None));
                    chunks.push((&effect[from..at], Some(*explanation)));
                    plain = at;
                }
            }
            _ => {}
        }
    }
    chunks.push((&effect[plain..], None));
    chunks
}

/// Farbe des Screens einer Befehlskarte (`--card-rgb`).
fn screen_color(card: DeckCard) -> Color {
    palette::darkened(palette::card_type_color(card.id.card_type()), 0.25)
}

/// Das Wort mit großem Anfangsbuchstaben („durchschlagend“ → „Durchschlagend“).
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Eine Befehlskarte als Datapad.
pub fn command_card(assets: &CardAssets, card: DeckCard, scale: f32) -> impl Bundle {
    command_card_with_bonus(assets, card, 0, scale)
}

/// Eine Befehlskarte mit `bonus` auf ihren Hauptwert (Feuerleitdoktrin in der Hand).
pub fn command_card_with_bonus(
    assets: &CardAssets,
    card: DeckCard,
    bonus: i32,
    scale: f32,
) -> impl Bundle {
    let s = |value: f32| px(value * scale);
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let text = locale.card_with_bonus(card, bonus);
    let keywords = locale.keywords();
    // Jedes Schlüsselwort der Karte einmal, als Zeile „Wort: Erklärung“.
    let mut explained: Vec<String> = Vec::new();
    for (word, explanation) in keyword_chunks(&text.effect, keywords) {
        if let Some(explanation) = explanation {
            let line = format!("{}: {explanation}", capitalized(word));
            if !explained.contains(&line) {
                explained.push(line);
            }
        }
    }
    if bonus != 0 {
        explained.insert(0, locale.card_bonus(bonus));
    }
    let explained = explained.join("\n");
    // Was sich gegenüber dem Text ohne Zuschlag ändert, ist eine erhöhte Zahl und leuchtet wie ein Schlüsselwort.
    let unboosted = locale.card(card).effect;
    let unboosted: Vec<_> = unboosted.split_whitespace().collect();
    // Je Wort ein Knoten, damit ein Schlüsselwort den Zeiger spürt; es zeigt sofort alle Schlüsselwörter der Karte.
    let effect_font = fonts.body(8.5 * scale);
    let words: Vec<_> = text
        .effect
        .split_whitespace()
        .enumerate()
        .map(|(i, word)| {
            let chunks: Vec<_> = if unboosted.get(i) == Some(&word) {
                keyword_chunks(word, keywords)
                    .into_iter()
                    .map(|(chunk, explanation)| (chunk, explanation.is_some()))
                    .collect()
            } else {
                let end = word
                    .find(|c: char| !c.is_alphanumeric())
                    .unwrap_or(word.len());
                vec![(&word[..end], true), (&word[end..], false)]
            };
            let keyword = chunks.iter().any(|&(_, highlighted)| highlighted);
            let font = effect_font.clone();
            let spans: Vec<_> = chunks
                .into_iter()
                .map(|(chunk, highlighted)| {
                    let color = if highlighted {
                        palette::BUFF
                    } else {
                        theme::rgb(235, 242, 255)
                    };
                    (
                        TextSpan::new(chunk),
                        font.clone(),
                        TextColor(color),
                        LineHeight::RelativeToFont(1.35),
                    )
                })
                .collect();
            (
                Text::default(),
                theme::text_shadow(0.9),
                Tooltip(if keyword {
                    explained.clone()
                } else {
                    String::new()
                }),
                TooltipInstant,
                if keyword {
                    Pickable::default()
                } else {
                    Pickable::IGNORE
                },
                Children::spawn(SpawnIter(spans.into_iter())),
            )
        })
        .collect();
    let target = format!(
        "{} · {}",
        locale.card_type(card.id.card_type()),
        locale.card_target(card.id.target())
    );
    let tint = screen_color(card);
    let type_color = palette::card_type_color(card.id.card_type());
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
        CardView { card, scale },
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
                    clipped_art(assets.art.get(Card::Command(card.id)), 0.5, 2.0 * scale),
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
                        text.name.clone(),
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
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::Center,
                            // Der Wortabstand der Schrift.
                            column_gap: s(8.5 * 0.22),
                            ..default()
                        },
                        Pickable::IGNORE,
                        Children::spawn(SpawnIter(words.into_iter())),
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
            // Die Kosten als Zahl oben links.
            Spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: s(-8.0),
                    left: s(-8.0),
                    width: s(26.0),
                    height: s(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(s(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(theme::SHADE),
                BorderColor::all(type_color),
                Pickable::IGNORE,
                children![(
                    theme::text(
                        fonts.display_bold(15.0 * scale),
                        card.cost().to_string(),
                        palette::TEXT
                    ),
                    Pickable::IGNORE,
                )],
            )),
            Spawn(selection_frame(scale, 4.0)),
        )),
    )
}

/// Zeigt den Rahmen, sobald eine Karte gewählt ist ([`Marked`] kommt immer erst nach dem Bau der
/// Karte dazu, ihre Kinder stehen also schon).
fn show_mark(
    add: On<Add<Marked>>,
    cards: Query<&Children>,
    mut frames: Query<&mut Visibility, With<SelectionFrame>>,
) {
    set_frame(add.entity, Visibility::Inherited, &cards, &mut frames);
}

/// Verbirgt den Rahmen wieder.
fn hide_mark(
    remove: On<Remove<Marked>>,
    cards: Query<&Children>,
    mut frames: Query<&mut Visibility, With<SelectionFrame>>,
) {
    set_frame(remove.entity, Visibility::Hidden, &cards, &mut frames);
}

fn set_frame(
    card: Entity,
    visibility: Visibility,
    cards: &Query<&Children>,
    frames: &mut Query<&mut Visibility, With<SelectionFrame>>,
) {
    for &child in cards.get(card).into_iter().flatten() {
        if let Ok(mut frame) = frames.get_mut(child) {
            frame.set_if_neq(visibility);
        }
    }
}

/// Dauer des Einflugs, des Nachleuchtens und des Hover-Zooms (Sekunden).
const DRAW_FLIGHT: f32 = 0.45;
const DRAW_GLOW: f32 = 2.4;
const HOVER_TIME: f32 = 0.12;

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
    // Einflug wie `cubic-bezier(0.2, 0.9, 0.3, 1.15)`: schießt über das Ziel hinaus, bevor er sich setzt.
    let flight_curve = CubicSegment::new_bezier_easing((0.2, 0.9), (0.3, 1.15));
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
        let eased = EaseFunction::QuadraticOut.sample_clamped(progress.0);
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
            let waiting = draw.delay.remaining();
            if draw.delay.tick(time.delta()).just_finished() {
                sounds.write(PlayUiSound(UiSound::Draw));
            }
            let flying = draw.delay.is_finished();
            if flying {
                // Im Bild, in dem die Wartezeit abläuft, fliegt sie nur den Rest des Bildes.
                draw.flight.tick(time.delta().saturating_sub(waiting));
            }
            let since = draw.flight.elapsed_secs();
            shift.y += 120.0 * (1.0 - flight_curve.ease(since / DRAW_FLIGHT));
            // Nachleuchten: ein Drittel voll, dann ausblenden.
            let glow = 1.0 - ((since / DRAW_GLOW - 0.3) / 0.7).clamp(0.0, 1.0);
            if flying && glow > 0.0 {
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

pub(super) struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(hover_sound)
            .add_observer(show_mark)
            .add_observer(hide_mark)
            .add_systems(Update, animate_cards);
    }
}

#[cfg(test)]
mod tests {
    use super::keyword_chunks;

    #[test]
    fn keywords_are_whole_words_in_any_case() {
        let keywords = [("Brand", "brennt"), ("Schilde", "schluckt")];
        assert_eq!(
            keyword_chunks("Schilde 5, brand. Brandsatz", &keywords),
            [
                ("", None),
                ("Schilde", Some("schluckt")),
                (" 5, ", None),
                ("brand", Some("brennt")),
                (". Brandsatz", None),
            ]
        );
    }
}
