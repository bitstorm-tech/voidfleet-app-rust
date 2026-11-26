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
use voidfleet_core::{
    admirals::AdmiralId,
    cards::{CardTarget, CardType, DeckCard},
};

use super::{
    Fonts, UiIcons, theme,
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

/// Was eine Karte braucht: Schriften, Icons, Bilder und Texte.
#[derive(SystemParam)]
pub struct CardAssets<'w> {
    pub fonts: Res<'w, Fonts>,
    pub icons: Res<'w, UiIcons>,
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

/// Stern-Abzeichen oben rechts auf Admiralskarten.
#[derive(Component, Clone, Copy, Debug)]
pub struct AdmiralBadge;

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

/// Ob `text` mit `keyword` beginnt (ohne Rücksicht auf Groß- und Kleinschreibung) und danach kein Wortzeichen folgt;
/// gibt die Byte-Länge des Treffers in `text` zurück.
fn match_keyword_at(text: &str, keyword: &str) -> Option<usize> {
    let mut text_chars = text.char_indices();
    for kw_char in keyword.chars() {
        let (_, t_char) = text_chars.next()?;
        if !t_char.to_lowercase().eq(kw_char.to_lowercase()) {
            return None;
        }
    }
    match text_chars.next() {
        Some((_, next)) if next.is_alphanumeric() => None,
        Some((end, _)) => Some(end),
        None => Some(text.len()),
    }
}

/// Zerlegt einen Effekttext an seinen Schlüsselwörtern: je Stück der Text und, bei einem Schlüsselwort, seine
/// Erklärung. Ein Schlüsselwort ist ein ganzes Wort (auch mehrteilig, etwa „Armor Break“), Groß- und Kleinschreibung
/// zählt nicht; bei mehreren Treffern gewinnt der längste.
fn keyword_chunks<'a>(
    effect: &'a str,
    keywords: &[(&'static str, &'static str)],
) -> Vec<(&'a str, Option<&'static str>)> {
    let mut chunks = Vec::new();
    let mut plain = 0;
    let mut at = 0;
    let mut prev_alnum = false;
    while let Some(c) = effect[at..].chars().next() {
        let matched = if !prev_alnum && c.is_alphanumeric() {
            keywords
                .iter()
                .filter_map(|&(name, explanation)| {
                    match_keyword_at(&effect[at..], name).map(|len| (len, explanation))
                })
                .max_by_key(|(len, _)| *len)
        } else {
            None
        };
        if let Some((len, explanation)) = matched {
            chunks.push((&effect[plain..at], None));
            chunks.push((&effect[at..at + len], Some(explanation)));
            plain = at + len;
            at = plain;
            prev_alnum = effect[..at]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
        } else {
            prev_alnum = c.is_alphanumeric();
            at += c.len_utf8();
        }
    }
    chunks.push((&effect[plain..], None));
    chunks
}

/// Farbe des Screens einer Befehlskarte (`--card-rgb`).
fn screen_color(card_type: CardType) -> Color {
    palette::darkened(palette::card_type_color(card_type), 0.25)
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
    command_card_in_hand(assets, card, bonus, card.cost(), false, scale)
}

/// Eine Befehlskarte auf der Hand mit effektivem `cost` und aktivem `retained`-Zustand.
pub fn command_card_in_hand(
    assets: &CardAssets,
    card: DeckCard,
    bonus: i32,
    cost: u32,
    retained: bool,
    scale: f32,
) -> impl Bundle {
    let text = assets.locale.card_with_bonus(card, bonus);
    let face = Face {
        name: text.name,
        effect: text.effect,
        cost,
        card_type: card.id.card_type(),
        target: card.id.target(),
        admiral: card.id.admiral(),
        retained,
    };
    card_face(
        &assets.fonts,
        &assets.icons,
        &assets.art,
        &assets.locale,
        card,
        face,
        bonus,
        scale,
    )
}

/// Was auf einer Karte steht; im Spiel aus den Daten, im Karteneditor aus dem, was dort gerade steht.
pub struct Face {
    pub name: String,
    pub effect: String,
    pub cost: u32,
    pub card_type: CardType,
    pub target: CardTarget,
    pub admiral: Option<AdmiralId>,
    pub retained: bool,
}

/// Ein Wort im Effekttext einer Karte mit seinen einzeln gefärbten Abschnitten und ob es einen Tooltip trägt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectWord<'a> {
    pub spans: Vec<(&'a str, bool)>,
    pub tooltip: bool,
}

/// Zerlegt den Effekttext in die erklärenden Tooltip-Zeilen der Schlüsselwörter und ihre Byte-Bereiche.
fn keyword_analysis(
    effect: &str,
    keywords: &[(&'static str, &'static str)],
) -> (Vec<String>, Vec<std::ops::Range<usize>>) {
    let mut explained: Vec<String> = Vec::new();
    let mut keyword_ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut cursor = 0;
    for (word, explanation) in keyword_chunks(effect, keywords) {
        let next = cursor + word.len();
        if let Some(explanation) = explanation {
            keyword_ranges.push(cursor..next);
            let line = format!("{}: {explanation}", capitalized(word));
            if !explained.contains(&line) {
                explained.push(line);
            }
        }
        cursor = next;
    }
    (explained, keyword_ranges)
}

/// Richtet die Wörter von `boosted` an `unboosted` per längster gemeinsamer Teilfolge aus; `true` bedeutet unverändert.
fn align_words(boosted: &[&str], unboosted: &[&str]) -> Vec<bool> {
    let n = boosted.len();
    let m = unboosted.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if boosted[i] == unboosted[j] {
                1 + dp[i + 1][j + 1]
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut matched = vec![false; n];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if boosted[i] == unboosted[j] {
            matched[i] = true;
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    matched
}

/// Byte-Bereiche der Zahlen (samt vorangestelltem `+`, `-` oder `−`) innerhalb eines durch Bonus veränderten Worts.
fn boosted_numeric_ranges(word: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let chars: Vec<(usize, char)> = word.char_indices().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let (byte_start, ch) = chars[idx];
        let prev_alnum = idx > 0 && chars[idx - 1].1.is_alphanumeric();
        let is_sign = matches!(ch, '+' | '-' | '−')
            && !prev_alnum
            && chars
                .get(idx + 1)
                .is_some_and(|(_, next)| next.is_ascii_digit());
        if ch.is_ascii_digit() || is_sign {
            idx += 1;
            while idx < chars.len() && chars[idx].1.is_ascii_digit() {
                idx += 1;
            }
            let byte_end = chars.get(idx).map_or(word.len(), |(pos, _)| *pos);
            ranges.push(byte_start..byte_end);
        } else {
            idx += 1;
        }
    }
    ranges
}

/// Zerlegt `effect` in Wörter samt Hervorhebungen (Schlüsselwörter aus `keyword_ranges` und geänderte Zahlen gegenüber `unboosted`).
fn effect_words_with_ranges<'a>(
    effect: &'a str,
    unboosted: &str,
    keyword_ranges: &[std::ops::Range<usize>],
) -> Vec<EffectWord<'a>> {
    let boosted_words: Vec<&str> = effect.split_whitespace().collect();
    let matched = if effect == unboosted {
        vec![true; boosted_words.len()]
    } else {
        let unboosted_words: Vec<&str> = unboosted.split_whitespace().collect();
        align_words(&boosted_words, &unboosted_words)
    };
    let base_ptr = effect.as_ptr() as usize;
    boosted_words
        .into_iter()
        .enumerate()
        .map(|(i, word)| {
            let start = word.as_ptr() as usize - base_ptr;
            let mut ranges: Vec<std::ops::Range<usize>> = keyword_ranges
                .iter()
                .filter_map(|range| {
                    let from = range.start.saturating_sub(start).min(word.len());
                    let to = range.end.saturating_sub(start).min(word.len());
                    (from < to).then_some(from..to)
                })
                .collect();
            if !matched[i] {
                ranges.extend(boosted_numeric_ranges(word));
            }
            ranges.sort_by_key(|range| (range.start, range.end));
            let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
            for range in ranges {
                if let Some(last) = merged.last_mut()
                    && range.start <= last.end
                {
                    last.end = last.end.max(range.end);
                } else {
                    merged.push(range);
                }
            }
            let tooltip = !merged.is_empty();
            let mut spans = Vec::new();
            let mut pos = 0;
            for range in merged {
                if pos < range.start {
                    spans.push((&word[pos..range.start], false));
                }
                spans.push((&word[range.start..range.end], true));
                pos = range.end;
            }
            if pos < word.len() {
                spans.push((&word[pos..], false));
            }
            EffectWord { spans, tooltip }
        })
        .collect()
}

/// Berechnet je Wort des Effekttexts die hervorgehobenen Teilabschnitte (Schlüsselwörter und erhöhte Zahlen) samt Tooltip-Flag.
pub fn effect_word_spans<'a>(
    effect: &'a str,
    unboosted: &str,
    keywords: &[(&'static str, &'static str)],
) -> Vec<EffectWord<'a>> {
    let (_, keyword_ranges) = keyword_analysis(effect, keywords);
    effect_words_with_ranges(effect, unboosted, &keyword_ranges)
}

/// Eine Befehlskarte mit dieser Aufschrift in der Sprache von `locale`; `bonus` wie bei [`command_card_with_bonus`].
pub fn card_face(
    fonts: &Fonts,
    icons: &UiIcons,
    art: &CardArt,
    locale: &Locale,
    card: DeckCard,
    face: Face,
    bonus: i32,
    scale: f32,
) -> impl Bundle {
    let s = |value: f32| px(value * scale);
    let text = face;
    let keywords = locale.keywords();
    // Jedes Schlüsselwort der Karte einmal, als Zeile „Wort: Erklärung“, samt Byte-Bereichen für mehrteilige Wörter.
    let (mut explained, keyword_ranges) = keyword_analysis(&text.effect, &keywords);
    if bonus != 0 {
        explained.insert(0, locale.card_bonus(bonus));
    }
    let explained = explained.join("\n");
    // Was sich gegenüber dem Text ohne Zuschlag ändert, ist eine erhöhte Zahl und leuchtet wie ein Schlüsselwort.
    let unboosted = (bonus != 0).then(|| locale.card(card).effect);
    let unboosted = unboosted.as_deref().unwrap_or(&text.effect);
    // Je Wort ein Knoten, damit ein Schlüsselwort den Zeiger spürt; es zeigt sofort alle Schlüsselwörter der Karte.
    let effect_font = fonts.body(8.5 * scale);
    let words: Vec<_> = effect_words_with_ranges(&text.effect, unboosted, &keyword_ranges)
        .into_iter()
        .map(|word| {
            let font = effect_font.clone();
            let spans: Vec<_> = word
                .spans
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
                Tooltip(if word.tooltip {
                    explained.clone()
                } else {
                    String::new()
                }),
                TooltipInstant,
                if word.tooltip {
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
        locale.card_type(text.card_type),
        locale.card_target(text.target)
    );
    let tint = screen_color(text.card_type);
    let type_color = palette::card_type_color(text.card_type);
    let discounted = text.cost < card.cost();
    let cost_border = if discounted {
        palette::HEAL
    } else {
        type_color
    };
    let cost_color = if discounted {
        palette::HEAL
    } else {
        palette::TEXT
    };
    let keyword_hint = |word: &str| {
        keywords
            .iter()
            .find(|(name, _)| name.to_lowercase() == word.to_lowercase())
            .map(|(_, hint)| format!("{word}: {hint}"))
            .unwrap_or_default()
    };
    let mut badges = Vec::new();
    if card.id.retain() {
        let label = if text.retained {
            locale.badge_retained()
        } else {
            locale.badge_retain()
        };
        let color = if text.retained {
            palette::SELECT
        } else {
            palette::PLAYER
        };
        badges.push((
            label.to_owned(),
            color,
            keyword_hint(locale.badge_retain()),
            text.retained,
        ));
    }
    if card.id.exhaust() {
        let label = locale.badge_exhaust();
        badges.push((
            label.to_owned(),
            palette::DEBUFF,
            keyword_hint(label),
            false,
        ));
    }
    let badge_nodes: Vec<_> = badges
        .into_iter()
        .map(|(label, color, hint, glow)| {
            let shadow = if glow {
                BoxShadow(vec![theme::glow(color.with_alpha(0.55), 8.0 * scale)])
            } else {
                BoxShadow::default()
            };
            (
                Node {
                    padding: UiRect::axes(s(5.0), s(1.5)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(s(3.0)),
                    ..default()
                },
                BackgroundColor(theme::SHADE.with_alpha(0.85)),
                BorderColor::all(color.with_alpha(if glow { 0.95 } else { 0.7 })),
                shadow,
                Tooltip(hint),
                TooltipInstant,
                children![(
                    theme::caps(fonts.display_bold(7.5 * scale), &label, 0.04, color),
                    Pickable::IGNORE,
                )],
            )
        })
        .collect();
    let badge_row = (!badge_nodes.is_empty()).then(|| {
        (
            Node {
                justify_content: JustifyContent::Center,
                column_gap: s(4.0),
                ..default()
            },
            Pickable::IGNORE,
            Children::spawn(SpawnIter(badge_nodes.into_iter())),
        )
    });
    let admiral_color = text.admiral.map(palette::admiral_color);
    let frame_border_width = if admiral_color.is_some() {
        s(1.5)
    } else {
        px(1)
    };
    let frame_border_color = admiral_color
        .map(|color| color.with_alpha(0.65))
        .unwrap_or(Color::WHITE.with_alpha(0.08));
    let admiral_badge = text.admiral.map(|admiral| {
        let color = palette::admiral_color(admiral);
        let hint = locale.badge_admiral_hint(locale.admiral(admiral).name);
        (
            AdmiralBadge,
            Node {
                position_type: PositionType::Absolute,
                top: s(-8.0),
                right: s(-6.0),
                height: s(20.0),
                padding: UiRect::axes(s(4.0), s(0.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(s(1.5)),
                border_radius: BorderRadius::all(s(4.0)),
                ..default()
            },
            BackgroundColor(theme::SHADE),
            BorderColor::all(color),
            Tooltip(hint),
            TooltipInstant,
            children![(
                ImageNode::new(icons.stars.clone()).with_color(color),
                Node {
                    width: s(18.0),
                    height: s(18.0),
                    ..default()
                },
                Pickable::IGNORE,
            )],
        )
    });
    let key = move |width: f32, accent: Option<Color>| {
        let (background, border, border_color) = match accent {
            Some(color) => (
                palette::darkened(color, 0.45),
                UiRect::all(px(1)),
                color.with_alpha(0.8),
            ),
            None => (theme::rgb(14, 16, 20), UiRect::ZERO, Color::NONE),
        };
        (
            Node {
                width: px(width * scale),
                height: px(7.0 * scale),
                border,
                border_radius: BorderRadius::all(px(scale)),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border_color),
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
            border: UiRect::all(frame_border_width),
            border_radius: BorderRadius::all(s(4.0)),
            ..default()
        },
        BackgroundColor(theme::rgb(26, 28, 34)),
        BorderColor::all(frame_border_color),
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
                Children::spawn((
                    Spawn(clipped_art(
                        art.get(Card::Command(card.id)),
                        0.5,
                        2.0 * scale,
                    )),
                    Spawn(shade(
                        &[
                            (tint.with_alpha(0.92), 0.0),
                            (tint.with_alpha(0.15), 30.0),
                            (tint.with_alpha(0.15), 52.0),
                            (tint.with_alpha(0.94), 72.0),
                        ],
                        2.0 * scale,
                    )),
                    Spawn(card_text(
                        fonts.display_bold(15.0 * scale),
                        text.name.clone(),
                        theme::rgb(247, 252, 255),
                        Justify::Center,
                    )),
                    Spawn(card_text(
                        fonts.body(9.0 * scale),
                        target,
                        theme::rgba(191, 224, 255, 0.9),
                        Justify::Center,
                    )),
                    SpawnIter(badge_row.into_iter()),
                    Spawn((
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
                    )),
                )),
            )),
            Spawn((
                Node {
                    justify_content: JustifyContent::Center,
                    column_gap: s(5.0),
                    margin: UiRect::top(s(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    key(14.0, None),
                    key(14.0, None),
                    key(24.0, admiral_color),
                    key(14.0, None),
                ],
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
                BorderColor::all(cost_border),
                Pickable::IGNORE,
                children![(
                    theme::text(
                        fonts.display_bold(15.0 * scale),
                        text.cost.to_string(),
                        cost_color
                    ),
                    Pickable::IGNORE,
                )],
            )),
            // Bei Admiralskarten das Stern-Abzeichen oben rechts.
            SpawnIter(admiral_badge.into_iter()),
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
    use voidfleet_core::cards::{CardId, DeckCard};

    use super::{EffectWord, effect_word_spans, keyword_chunks};
    use crate::{i18n::Locale, settings::Language};

    fn word<'a>(spans: Vec<(&'a str, bool)>, tooltip: bool) -> EffectWord<'a> {
        EffectWord { spans, tooltip }
    }

    fn plain(text: &str) -> EffectWord<'_> {
        word(vec![(text, false)], false)
    }

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

    #[test]
    fn multi_word_keywords_match_longest_phrase_across_spaces() {
        let keywords = [
            ("Armor", "reduces damage"),
            ("Armor Break", "lowers armor"),
            ("Retain", "stays in hand"),
        ];
        assert_eq!(
            keyword_chunks("Armor Break, armor 2. Retain.", &keywords),
            [
                ("", None),
                ("Armor Break", Some("lowers armor")),
                (", ", None),
                ("armor", Some("reduces damage")),
                (" 2. ", None),
                ("Retain", Some("stays in hand")),
                (".", None),
            ]
        );
    }

    #[test]
    fn shield_bash_bonus_zero_and_positive_en_and_de() {
        let card = DeckCard::new(CardId::ShieldBash);
        let en = Locale::new(Language::En);
        let en_kw = en.keywords();
        let en_base = en.card(card).effect;
        let en_boosted = en.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&en_base, &en_base, &en_kw),
            [
                plain("Damage"),
                plain("equal"),
                plain("to"),
                plain("your"),
                plain("flagship's"),
                plain("current"),
                word(vec![("shields", true)], true),
                plain("to"),
                plain("a"),
                plain("chosen"),
                plain("ship."),
                word(vec![("Homing", true), (".", false)], true),
            ]
        );
        assert_eq!(
            effect_word_spans(&en_boosted, &en_base, &en_kw),
            [
                plain("Damage"),
                plain("equal"),
                plain("to"),
                plain("your"),
                plain("flagship's"),
                plain("current"),
                word(vec![("shields", true)], true),
                word(vec![("+3", true)], true),
                plain("to"),
                plain("a"),
                plain("chosen"),
                plain("ship."),
                word(vec![("Homing", true), (".", false)], true),
            ]
        );

        let de = Locale::new(Language::De);
        let de_kw = de.keywords();
        let de_base = de.card(card).effect;
        let de_boosted = de.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&de_base, &de_base, &de_kw),
            [
                plain("Schaden"),
                plain("in"),
                plain("Höhe"),
                plain("der"),
                plain("aktuellen"),
                word(vec![("Schilde", true)], true),
                plain("deines"),
                plain("Flaggschiffs"),
                plain("auf"),
                plain("ein"),
                plain("gewähltes"),
                plain("Schiff."),
                word(vec![("Zielsuchend", true), (".", false)], true),
            ]
        );
        assert_eq!(
            effect_word_spans(&de_boosted, &de_base, &de_kw),
            [
                plain("Schaden"),
                plain("in"),
                plain("Höhe"),
                plain("der"),
                plain("aktuellen"),
                word(vec![("Schilde", true)], true),
                plain("deines"),
                plain("Flaggschiffs"),
                word(vec![("+3", true)], true),
                plain("auf"),
                plain("ein"),
                plain("gewähltes"),
                plain("Schiff."),
                word(vec![("Zielsuchend", true), (".", false)], true),
            ]
        );
    }

    #[test]
    fn collision_course_bonus_positive_en_and_de() {
        let card = DeckCard::new(CardId::CollisionCourse);
        let en = Locale::new(Language::En);
        let en_kw = en.keywords();
        let en_base = en.card(card).effect;
        let en_boosted = en.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&en_boosted, &en_base, &en_kw),
            [
                plain("Ram"),
                plain("a"),
                plain("chosen"),
                plain("ship:"),
                plain("it"),
                plain("takes"),
                plain("twice"),
                plain("your"),
                plain("ATK"),
                word(vec![("+3", true), (",", false)], true),
                plain("your"),
                plain("flagship"),
                plain("takes"),
                plain("its"),
                plain("ATK."),
                word(vec![("Homing", true), (",", false)], true),
                word(vec![("piercing", true), (".", false)], true),
            ]
        );

        let de = Locale::new(Language::De);
        let de_kw = de.keywords();
        let de_base = de.card(card).effect;
        let de_boosted = de.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&de_boosted, &de_base, &de_kw),
            [
                plain("Rammt"),
                plain("ein"),
                plain("gewähltes"),
                plain("Schiff:"),
                plain("Es"),
                plain("nimmt"),
                plain("das"),
                plain("Doppelte"),
                plain("deiner"),
                plain("ATK"),
                word(vec![("+3", true), (",", false)], true),
                plain("dein"),
                plain("Flaggschiff"),
                plain("seine"),
                plain("ATK."),
                word(vec![("Zielsuchend", true), (",", false)], true),
                word(vec![("durchschlagend", true), (".", false)], true),
            ]
        );
    }

    #[test]
    fn chrono_strike_bonus_positive_en_and_de() {
        let card = DeckCard::new(CardId::ChronoStrike);
        let boosted = (card.value() + 3).to_string();
        let doubled = (2 * card.value() + 3).to_string();

        let en = Locale::new(Language::En);
        let en_kw = en.keywords();
        let en_base = en.card(card).effect;
        let en_boosted = en.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&en_boosted, &en_base, &en_kw),
            [
                word(vec![(&boosted, true)], true),
                plain("damage"),
                plain("to"),
                plain("a"),
                plain("chosen"),
                plain("ship"),
                word(vec![("(", false), (&doubled, true)], true),
                plain("if"),
                plain("it"),
                plain("is"),
                plain("in"),
                plain("stasis"),
                plain("or"),
                plain("charging)."),
                word(vec![("Homing", true), (",", false)], true),
                word(vec![("piercing", true), (".", false)], true),
            ]
        );

        let de = Locale::new(Language::De);
        let de_kw = de.keywords();
        let de_base = de.card(card).effect;
        let de_boosted = de.card_with_bonus(card, 3).effect;

        assert_eq!(
            effect_word_spans(&de_boosted, &de_base, &de_kw),
            [
                word(vec![(&boosted, true)], true),
                plain("Schaden"),
                plain("auf"),
                plain("ein"),
                plain("gewähltes"),
                plain("Schiff"),
                word(vec![("(", false), (&doubled, true), (",", false)], true),
                plain("wenn"),
                plain("es"),
                plain("im"),
                plain("Stillstand"),
                plain("ist"),
                plain("oder"),
                plain("auflädt)."),
                word(vec![("Zielsuchend", true), (",", false)], true),
                word(vec![("durchschlagend", true), (".", false)], true),
            ]
        );
    }

    #[test]
    fn boosted_token_punctuation_edge_cases() {
        assert_eq!(
            effect_word_spans("(17) (17, +3, −5. (−5)", "(14) (14, −2. (−2)", &[]),
            [
                word(vec![("(", false), ("17", true), (")", false)], true),
                word(vec![("(", false), ("17", true), (",", false)], true),
                word(vec![("+3", true), (",", false)], true),
                word(vec![("−5", true), (".", false)], true),
                word(vec![("(", false), ("−5", true), (")", false)], true),
            ]
        );
    }

    #[test]
    fn admiral_cards_spawn_badge_frame_and_tooltip_while_neutral_do_not() {
        use bevy::prelude::*;
        use voidfleet_core::admirals::AdmiralId;

        use super::{AdmiralBadge, Face, card_face};
        use crate::{
            data::card_art::CardArt,
            palette,
            ui::{Fonts, UiIcons, tooltip::Tooltip},
        };

        let fonts = Fonts {
            display: Handle::default(),
            display_bold: Handle::default(),
            body: Handle::default(),
            symbols: Handle::default(),
        };
        let icons = UiIcons {
            log: Handle::default(),
            view_toggle: Handle::default(),
            crosshair: Handle::default(),
            check: Handle::default(),
            arrow_right: Handle::default(),
            stars: Handle::default(),
            settings: Handle::default(),
            rules: Handle::default(),
            column_fire: Handle::default(),
            ram: Handle::default(),
            shield: Handle::default(),
            evade: Handle::default(),
            charge: Handle::default(),
            target_lock: Handle::default(),
            call: Handle::default(),
            flame: Handle::default(),
            emergency: Handle::default(),
            stasis: Handle::default(),
            silenced: Handle::default(),
            marked: Handle::default(),
            cover: Handle::default(),
            armor: Handle::default(),
            repair: Handle::default(),
            maneuver: Handle::default(),
        };
        let art = CardArt::placeholder();
        let locale = Locale::new(Language::De);

        let mut world = World::new();
        let neutral_card = DeckCard::new(CardId::Torpedo);
        let neutral_text = locale.card(neutral_card);
        let neutral_root = world
            .spawn(card_face(
                &fonts,
                &icons,
                &art,
                &locale,
                neutral_card,
                Face {
                    name: neutral_text.name,
                    effect: neutral_text.effect,
                    cost: neutral_card.cost(),
                    card_type: neutral_card.id.card_type(),
                    target: neutral_card.id.target(),
                    admiral: neutral_card.id.admiral(),
                    retained: false,
                },
                0,
                1.0,
            ))
            .id();
        assert_eq!(
            world
                .query_filtered::<Entity, With<AdmiralBadge>>()
                .iter(&world)
                .count(),
            0
        );
        assert_eq!(
            *world.get::<BorderColor>(neutral_root).unwrap(),
            BorderColor::all(Color::WHITE.with_alpha(0.08))
        );

        let admiral_card = DeckCard::new(CardId::ShieldBash);
        let admiral_text = locale.card(admiral_card);
        let admiral_root = world
            .spawn(card_face(
                &fonts,
                &icons,
                &art,
                &locale,
                admiral_card,
                Face {
                    name: admiral_text.name,
                    effect: admiral_text.effect,
                    cost: admiral_card.cost(),
                    card_type: admiral_card.id.card_type(),
                    target: admiral_card.id.target(),
                    admiral: admiral_card.id.admiral(),
                    retained: false,
                },
                0,
                1.0,
            ))
            .id();
        let osei_color = palette::admiral_color(AdmiralId::Osei);
        assert_eq!(
            *world.get::<BorderColor>(admiral_root).unwrap(),
            BorderColor::all(osei_color.with_alpha(0.65))
        );
        let badges: Vec<_> = world
            .query_filtered::<(&BorderColor, &Tooltip), With<AdmiralBadge>>()
            .iter(&world)
            .collect();
        assert_eq!(badges.len(), 1);
        assert_eq!(*badges[0].0, BorderColor::all(osei_color));
        assert_eq!(
            badges[0].1.0,
            locale.badge_admiral_hint(locale.admiral(AdmiralId::Osei).name)
        );
    }
}
