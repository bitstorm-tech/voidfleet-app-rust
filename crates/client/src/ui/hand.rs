//! Die Hand (Port von `ui/hand.ts`): links groß die Befehlspunkte und darunter Nachzieh- und Ablagestapel als Zahlen,
//! in der Mitte die Karten aus [`Battle::hand`](crate::game::Battle), rechts das Stationen-Feld ([`super::stations`]).
//! Gleiche Karten liegen als ein Stapel mit Anzahl. Passen die Karten nicht in ihre Reihe, rücken sie zusammen und
//! überlappen sich; die Karte unter dem Zeiger wächst und liegt oben. Karten, die mehr kosten, als Befehlspunkte
//! übrig sind, sind gedimmt.
//!
//! Karten werden nur neu aufgebaut, wenn sich die Hand ändert; neu gezogene fliegen dabei nacheinander ein. Nach dem
//! Zugende ist die Reihe leer, die neue Hand kommt erst, wenn Gegnerzug und Auflösung abgespielt sind. Ein Klick
//! meldet [`HandCardPressed`] mit dem Hand-Index; die gewählte Karte zeigt der Rahmen ([`Marked`]).

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use voidfleet_core::cards::DeckCard;

use super::{
    Fonts, HandCardPressed, HudState, UiIcons,
    card::{CARD_WIDTH, CardAssets, CardDraw, CardHover, HAND_ZOOM, Marked, command_card},
    stations, theme,
    theme::UiOpacity,
};
use crate::{assets::AppState, game::Battle, i18n::Locale, palette, playback::PlaybackFinished};

/// Breite einer Karte auf der Hand, Abstand zweier Karten (logische Pixel).
const CARD: f32 = CARD_WIDTH * HAND_ZOOM;
const CARD_GAP: f32 = 10.0;
/// Versatz, mit dem neu gezogene Karten nacheinander einfliegen (Sekunden).
const DRAW_STAGGER: f32 = 0.18;
/// Deckkraft einer Karte, die gerade nicht spielbar ist.
const UNPLAYABLE_ALPHA: f32 = 0.45;

/// Die Leiste am unteren Rand.
#[derive(Component)]
pub struct HandBar;

/// Die Reihe der Karten.
#[derive(Component)]
struct HandRow;

/// Befehlspunkte und Stapel links.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum PileText {
    Points,
    PointsLabel,
    Draw,
    Discard,
}

/// Eine Karte der Hand: ein Stapel gleicher Karten mit ihren Hand-Indizes.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct HandStack(Vec<usize>);

/// Was die Reihe gerade zeigt; nur eine Änderung baut sie neu.
#[derive(Resource, Default, PartialEq)]
struct ShownHand {
    cards: Vec<DeckCard>,
    /// Die Runde, zu der die Karten gehören; 0 ohne Gefecht.
    round: u32,
}

/// Die neue Runde ist gerechnet, aber noch nicht abgespielt: Die alte Hand ist abgeworfen, die neue wartet.
fn awaits_playback(shown_round: u32, round: u32, played: bool) -> bool {
    shown_round != 0 && round != shown_round && !played
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>, icons: Res<UiIcons>) {
    let pile = |kind: PileText, font: TextFont, color: Color| {
        (
            kind,
            theme::text(font, "", color),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        )
    };
    commands.spawn((
        Name::new("Hand"),
        HandBar,
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            right: px(20),
            bottom: px(12),
            height: px(266),
            align_items: AlignItems::FlexEnd,
            column_gap: px(20),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    align_self: AlignSelf::Center,
                    row_gap: px(6),
                    width: px(120),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    pile(PileText::Points, fonts.display_bold(72.0), theme::ACCENT),
                    pile(
                        PileText::PointsLabel,
                        fonts.display(16.0),
                        palette::TEXT_DIM
                    ),
                    pile(PileText::Draw, fonts.body(14.0), palette::TEXT_DIM),
                    pile(PileText::Discard, fonts.body(14.0), palette::TEXT_DIM),
                ],
            ),
            (
                HandRow,
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::ZERO,
                    min_width: Val::ZERO,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexEnd,
                    ..default()
                },
                Pickable::IGNORE,
            ),
            stations::stations_panel(&fonts, &icons),
        ],
    ));
}

/// Befehlspunkte und Stapel.
fn update_piles(
    battle: Option<Res<Battle>>,
    locale: Res<Locale>,
    mut texts: Query<(&PileText, &mut Text)>,
) {
    for (kind, mut text) in &mut texts {
        let content = match (kind, battle.as_deref()) {
            (_, None) => String::new(),
            (PileText::Points, Some(battle)) => battle.points.to_string(),
            (PileText::PointsLabel, Some(_)) => locale.hud_points().to_uppercase(),
            (PileText::Draw, Some(battle)) => {
                format!("{} {}", locale.hud_draw_pile(), battle.draw_pile.len())
            }
            (PileText::Discard, Some(battle)) => {
                format!(
                    "{} {}",
                    locale.hud_discard_pile(),
                    battle.discard_pile.len()
                )
            }
        };
        if text.0 != content {
            text.0 = content;
        }
    }
}

/// Gleiche Karten der Hand als Stapel an der Stelle der ersten, mit den Hand-Indizes aller Exemplare.
fn stacks_of<T: PartialEq + Copy>(hand: &[T]) -> Vec<(T, Vec<usize>)> {
    let mut stacks: Vec<(T, Vec<usize>)> = Vec::new();
    for (index, &card) in hand.iter().enumerate() {
        match stacks.iter_mut().find(|(other, _)| *other == card) {
            Some((_, indices)) => indices.push(index),
            None => stacks.push((card, vec![index])),
        }
    }
    stacks
}

/// Indizes der Karten in `next`, die in `previous` noch nicht lagen (gleiche Karten zählen mehrfach).
fn drawn_indices<T: PartialEq>(previous: &[T], next: &[T]) -> Vec<usize> {
    let mut remaining: Vec<&T> = previous.iter().collect();
    next.iter()
        .enumerate()
        .filter_map(
            |(index, card)| match remaining.iter().position(|kept| *kept == card) {
                Some(kept) => {
                    remaining.remove(kept);
                    None
                }
                None => Some(index),
            },
        )
        .collect()
}

/// Ab wann eine Karte einfliegt: das früheste neue Exemplar ihres Stapels; `None`, wenn alle schon auf der Hand lagen.
fn draw_delay(indices: &[usize], drawn: &[usize]) -> Option<f32> {
    indices
        .iter()
        .filter_map(|index| drawn.iter().position(|new| new == index))
        .min()
        .map(|order| order as f32 * DRAW_STAGGER)
}

/// Zähler und Kanten eines Stapels ab zwei Exemplaren.
fn stack_marks(fonts: &Fonts, count: usize) -> (BoxShadow, impl Bundle) {
    let s = |value: f32| value * HAND_ZOOM;
    let edge = |offset: f32, alpha: f32| {
        [
            ShadowStyle {
                spread_radius: px(1),
                ..theme::drop(Color::WHITE.with_alpha(alpha), s(offset), -s(offset), 0.0)
            },
            theme::drop(theme::SHADE, s(offset), -s(offset), 0.0),
        ]
    };
    let mut shadow = Vec::new();
    if count > 2 {
        shadow.extend(edge(8.0, 0.2));
    }
    shadow.extend(edge(4.0, 0.3));
    let badge = (
        Node {
            position_type: PositionType::Absolute,
            bottom: px(-s(10.0)),
            right: px(-s(10.0)),
            min_width: px(s(26.0)),
            height: px(s(26.0)),
            padding: UiRect::horizontal(px(s(6.0))),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(s(2.0))),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(theme::SHADE),
        BorderColor::all(palette::SELECT),
        Pickable::IGNORE,
        children![(
            theme::text(
                fonts.display_bold(s(13.0)),
                format!("×{count}"),
                Color::WHITE
            ),
            Pickable::IGNORE,
        )],
    );
    (BoxShadow(shadow), badge)
}

/// Baut die Reihe neu, sobald sich die Hand ändert.
fn rebuild_row(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    mut shown: ResMut<ShownHand>,
    assets: CardAssets,
    row: Single<Entity, With<HandRow>>,
    mut finished: MessageReader<PlaybackFinished>,
) {
    let played = finished.read().count() > 0;
    let next = match battle {
        None => ShownHand::default(),
        Some(battle) if awaits_playback(shown.round, battle.round, played) => ShownHand {
            cards: Vec::new(),
            round: shown.round,
        },
        Some(battle) => ShownHand {
            cards: battle.hand.clone(),
            round: battle.round,
        },
    };
    if *shown == next && !assets.locale.is_changed() {
        return;
    }
    let drawn = drawn_indices(&shown.cards, &next.cards);
    commands.entity(*row).despawn_related::<Children>();
    for (card, indices) in stacks_of(&next.cards) {
        let mut entity = commands.spawn((
            command_card(&assets, card, HAND_ZOOM),
            HandStack(indices.clone()),
            UiOpacity(1.0),
            CardHover::Zoom {
                scale: 1.5,
                origin: Vec2::new(0.0, 1.0),
            },
            ChildOf(*row),
        ));
        if indices.len() > 1 {
            let (shadow, badge) = stack_marks(&assets.fonts, indices.len());
            entity.insert(shadow).with_child(badge);
        }
        if let Some(delay) = draw_delay(&indices, &drawn) {
            entity.insert(CardDraw::after(delay));
        }
    }
    *shown = next;
}

/// Welches Exemplar ein Klick auf einen Stapel meint: das gewählte, sonst das erste.
fn pick(stack: &HandStack, hud: &HudState) -> usize {
    hud.selected
        .filter(|index| stack.0.contains(index))
        .unwrap_or(stack.0[0])
}

fn click_card(
    click: On<PointerClick>,
    stacks: Query<&HandStack>,
    hud: Res<HudState>,
    mut pressed: MessageWriter<HandCardPressed>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(stack) = stacks.get(click.entity) {
        pressed.write(HandCardPressed(pick(stack, &hud)));
    }
}

/// Rahmen um die gewählte Karte; gedimmt, was gerade nicht spielbar ist.
fn mark_cards(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    hud: Res<HudState>,
    mut stacks: Query<(Entity, &HandStack, Has<Marked>, &mut UiOpacity)>,
) {
    let Some(battle) = battle else {
        return;
    };
    for (card, stack, was, mut opacity) in &mut stacks {
        let marked = hud.selected.is_some_and(|index| stack.0.contains(&index));
        if marked && !was {
            commands.entity(card).insert(Marked);
        } else if !marked && was {
            commands.entity(card).remove::<Marked>();
        }
        let affordable = battle
            .hand
            .get(stack.0[0])
            .is_some_and(|card| card.cost() <= battle.points);
        let alpha = if hud.planning && affordable {
            1.0
        } else {
            UNPLAYABLE_ALPHA
        };
        opacity.set_if_neq(UiOpacity(alpha));
    }
}

/// Passen die Karten nicht in die Reihe, rücken sie zusammen und überlappen sich.
fn fit_row(
    row: Single<(&ComputedNode, &Children), With<HandRow>>,
    mut cards: Query<&mut Node, Without<HandRow>>,
) {
    let (node, children) = *row;
    let space = node.size().x * node.inverse_scale_factor();
    let count = children.len();
    let width = count as f32 * CARD + count.saturating_sub(1) as f32 * CARD_GAP;
    let gap = if count > 1 && width > space {
        CARD_GAP - (width - space) / (count - 1) as f32
    } else {
        CARD_GAP
    };
    for (index, card) in children.iter().enumerate() {
        let margin = if index == 0 { Val::ZERO } else { px(gap) };
        if let Ok(mut node) = cards.get_mut(card)
            && node.margin.left != margin
        {
            node.margin.left = margin;
        }
    }
}

pub(super) struct HandPlugin;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownHand>()
            .add_systems(OnEnter(AppState::Match), spawn)
            .add_observer(click_card)
            .add_systems(
                Update,
                (rebuild_row, update_piles, mark_cards, fit_row)
                    .chain()
                    .run_if(in_state(AppState::Match)),
            );
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::cards::CardId;

    use super::*;

    #[test]
    fn new_hand_waits_for_the_playback() {
        assert!(awaits_playback(1, 2, false));
        assert!(!awaits_playback(1, 2, true));
        assert!(!awaits_playback(1, 1, false));
        assert!(!awaits_playback(0, 1, false));
    }

    #[test]
    fn stacks_keep_first_position() {
        let hand = [CardId::Torpedo, CardId::Scavenge, CardId::Torpedo].map(DeckCard::new);
        assert_eq!(
            stacks_of(&hand),
            vec![(hand[0], vec![0, 2]), (hand[1], vec![1])]
        );
    }

    #[test]
    fn drawn_counts_copies() {
        let before = [CardId::Torpedo, CardId::Scavenge];
        let after = [
            CardId::Torpedo,
            CardId::Torpedo,
            CardId::Scavenge,
            CardId::RedAlert,
        ];
        assert_eq!(drawn_indices(&before, &after), vec![1, 3]);
        assert_eq!(draw_delay(&[0, 1], &[1, 3]), Some(0.0));
        assert_eq!(draw_delay(&[3], &[1, 3]), Some(DRAW_STAGGER));
        assert_eq!(draw_delay(&[2], &[1, 3]), None);
    }
}
