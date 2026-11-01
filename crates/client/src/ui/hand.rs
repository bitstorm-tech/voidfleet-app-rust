//! Die Hand (Port von `ui/hand.ts`): Schiffskarten links, in Runde 1 vorn das Flaggschiff, Aktionsknopf samt
//! Checkliste in der Mitte ([`super::hud`]), rechts die Admiralsfähigkeit ([`super::power_card`]) und die
//! Befehlskarten. Gleiche Karten liegen als ein Stapel mit Anzahl. Passt eine Seite nicht in ihre Hälfte der Leiste,
//! rücken ihre Karten zusammen und überlappen sich; die Karte unter dem Zeiger wächst und liegt oben.
//!
//! Karten werden nur neu aufgebaut, wenn sich die Hand ändert; neu gezogene fliegen dabei nacheinander ein. Ein Klick
//! meldet [`HandCardPressed`] mit dem Exemplar, das er meint; gewählt und eingeplant zeigt der Rahmen ([`Marked`]).

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use voidfleet_core::cards::{CommandId, ShipId};

use super::{
    HandCard, HandCardPressed, HudState, Selected,
    card::{
        CARD_WIDTH, CardAssets, CardDraw, CardHover, HAND_ZOOM, Marked, command_card, ship_card,
    },
    hud::{self, ActionArea},
    power_card::{self, AdmiralArea},
    theme,
};
use crate::{
    assets::AppState,
    game::{Match, ShipPick},
    palette,
};

/// Breite einer Karte auf der Hand, Abstand zweier Karten, Abstand zwischen Fähigkeitskarte und Befehlen und
/// Mindestabstand jeder Hand zum Aktionsknopf (logische Pixel).
const CARD: f32 = CARD_WIDTH * HAND_ZOOM;
const CARD_GAP: f32 = 10.0;
const SIDE_GAP: f32 = 16.0;
const CENTER_MARGIN: f32 = 20.0;
/// Versatz, mit dem neu gezogene Karten nacheinander einfliegen (Sekunden).
const DRAW_STAGGER: f32 = 0.18;

/// Die Leiste am unteren Rand.
#[derive(Component)]
pub struct HandBar;

/// Die Reihe der Schiffs- bzw. Befehlskarten.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HandRow {
    Ships,
    Commands,
}

/// Eine Karte der Hand: das Flaggschiff oder ein Stapel gleicher Karten mit ihren Hand-Indizes.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
enum HandStack {
    Flagship,
    Ships(Vec<usize>),
    Commands(Vec<usize>),
}

/// Was die Reihen gerade zeigen; nur eine Änderung baut sie neu.
#[derive(Resource, Default, PartialEq)]
struct ShownHand {
    ships: Vec<ShipId>,
    flagship: Option<ShipId>,
    commands: Vec<CommandId>,
}

fn spawn(mut commands: Commands, assets: CardAssets) {
    let half = || Node {
        flex_grow: 1.0,
        flex_basis: Val::ZERO,
        min_width: Val::ZERO,
        align_items: AlignItems::FlexEnd,
        ..default()
    };
    let row = |kind: HandRow| {
        (
            kind,
            Node {
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            Pickable::IGNORE,
        )
    };
    commands.spawn((
        Name::new("Hand"),
        HandBar,
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            right: px(28),
            bottom: px(12),
            height: px(266),
            align_items: AlignItems::FlexEnd,
            ..default()
        },
        Pickable::IGNORE,
        children![
            (half(), Pickable::IGNORE, children![row(HandRow::Ships)]),
            hud::action_area(&assets.fonts),
            (
                Node {
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(SIDE_GAP),
                    ..half()
                },
                Pickable::IGNORE,
                children![
                    power_card::admiral_area(&assets.fonts, &assets.icons, &assets.locale),
                    row(HandRow::Commands),
                ],
            ),
        ],
    ));
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
fn stack_marks(fonts: &super::Fonts, count: usize) -> (BoxShadow, impl Bundle) {
    let s = |value: f32| value * HAND_ZOOM;
    let edge = |offset: f32, alpha: f32| {
        [
            ShadowStyle {
                spread_radius: px(1),
                ..theme::drop(Color::WHITE.with_alpha(alpha), -s(offset), -s(offset), 0.0)
            },
            theme::drop(theme::SHADE, -s(offset), -s(offset), 0.0),
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
            top: px(-s(12.0)),
            left: px(-s(12.0)),
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

/// Baut eine Reihe neu, sobald sich ihre Karten ändern.
fn rebuild_rows(
    mut commands: Commands,
    game: Res<Match>,
    mut shown: ResMut<ShownHand>,
    assets: CardAssets,
    rows: Query<(Entity, &HandRow)>,
) {
    let flagship = game.flagship().filter(|_| game.needs_flagship());
    let next = ShownHand {
        ships: game.ship_hand.clone(),
        flagship,
        commands: game.command_hand.clone(),
    };
    if *shown == next {
        return;
    }
    for (row, kind) in &rows {
        match kind {
            HandRow::Ships if shown.ships != next.ships || shown.flagship != next.flagship => {
                let drawn = drawn_indices(&shown.ships, &next.ships);
                commands.entity(row).despawn_related::<Children>();
                if let Some(flagship) = flagship {
                    commands.spawn((
                        ship_card(&assets, flagship, HAND_ZOOM),
                        HandStack::Flagship,
                        ChildOf(row),
                    ));
                }
                for (ship, indices) in stacks_of(&next.ships) {
                    let card = commands
                        .spawn((ship_card(&assets, ship, HAND_ZOOM), ChildOf(row)))
                        .id();
                    decorate(&mut commands, card, &assets.fonts, &indices, &drawn);
                    commands.entity(card).insert(HandStack::Ships(indices));
                }
            }
            HandRow::Commands if shown.commands != next.commands => {
                let drawn = drawn_indices(&shown.commands, &next.commands);
                commands.entity(row).despawn_related::<Children>();
                for (command, indices) in stacks_of(&next.commands) {
                    let card = commands
                        .spawn((command_card(&assets, command, HAND_ZOOM), ChildOf(row)))
                        .id();
                    decorate(&mut commands, card, &assets.fonts, &indices, &drawn);
                    commands.entity(card).insert(HandStack::Commands(indices));
                }
            }
            _ => {}
        }
    }
    *shown = next;
}

/// Stapel-Kanten, Zähler und Einflug einer Karte.
fn decorate(
    commands: &mut Commands,
    card: Entity,
    fonts: &super::Fonts,
    indices: &[usize],
    drawn: &[usize],
) {
    if indices.len() > 1 {
        let (shadow, badge) = stack_marks(fonts, indices.len());
        commands.entity(card).insert(shadow).with_child(badge);
    }
    if let Some(delay) = draw_delay(indices, drawn) {
        commands.entity(card).insert(CardDraw::after(delay));
    }
}

/// Hover-Zoom zur Bildschirmmitte: die erste Schiffskarte wächst nach rechts, die letzte Befehlskarte nach links.
fn set_hover_origins(
    mut commands: Commands,
    rows: Query<(&HandRow, &Children), Changed<Children>>,
) {
    for (row, cards) in &rows {
        let last = cards.len().saturating_sub(1);
        for (index, card) in cards.iter().enumerate() {
            let x = match row {
                HandRow::Ships if index == 0 => -1.0,
                HandRow::Commands if index == last => 1.0,
                _ => 0.0,
            };
            commands.entity(card).insert(CardHover::Zoom {
                scale: 1.5,
                origin: Vec2::new(x, 1.0),
            });
        }
    }
}

/// Welches Exemplar ein Klick auf einen Stapel meint. Schiffe: das gewählte, sonst das nächste freie, sonst das
/// zuletzt eingeplante. Befehle: der eingeplante, sonst der gewählte, sonst das erste Exemplar.
fn pick(stack: &HandStack, game: &Match, hud: &HudState) -> HandCard {
    match stack {
        HandStack::Flagship => HandCard::Ship(ShipPick::Flagship),
        HandStack::Ships(indices) => {
            let planned: Vec<ShipPick> =
                game.arrivals().iter().map(|arrival| arrival.pick).collect();
            let picks = indices.iter().map(|&index| ShipPick::Hand(index));
            let selected = picks
                .clone()
                .find(|&pick| hud.selected == Some(Selected::Ship(pick)));
            let free = picks.clone().find(|pick| !planned.contains(pick));
            let last_planned = picks.clone().rfind(|pick| planned.contains(pick));
            HandCard::Ship(
                selected
                    .or(free)
                    .or(last_planned)
                    .unwrap_or(ShipPick::Hand(indices[0])),
            )
        }
        HandStack::Commands(indices) => {
            let planned = game.plan().command.map(|command| command.hand_index);
            let selected = match hud.selected {
                Some(Selected::Command(index)) => Some(index),
                _ => None,
            };
            let index = [planned, selected]
                .into_iter()
                .flatten()
                .find(|index| indices.contains(index))
                .unwrap_or(indices[0]);
            HandCard::Command(index)
        }
    }
}

fn click_card(
    click: On<PointerClick>,
    stacks: Query<&HandStack>,
    game: Res<Match>,
    hud: Res<HudState>,
    mut pressed: MessageWriter<HandCardPressed>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(stack) = stacks.get(click.entity) {
        pressed.write(HandCardPressed(pick(stack, &game, &hud)));
    }
}

/// Rahmen um gewählte und eingeplante Karten.
fn mark_cards(
    mut commands: Commands,
    game: Res<Match>,
    hud: Res<HudState>,
    stacks: Query<(Entity, &HandStack, Has<Marked>)>,
) {
    let planned_ships: Vec<ShipPick> = game.arrivals().iter().map(|arrival| arrival.pick).collect();
    let planned_command = game.plan().command.map(|command| command.hand_index);
    let ship_marked = |pick: ShipPick| {
        planned_ships.contains(&pick) || hud.selected == Some(Selected::Ship(pick))
    };
    for (card, stack, was) in &stacks {
        let marked = match stack {
            HandStack::Flagship => ship_marked(ShipPick::Flagship),
            HandStack::Ships(indices) => indices
                .iter()
                .any(|&index| ship_marked(ShipPick::Hand(index))),
            HandStack::Commands(indices) => indices.iter().any(|&index| {
                planned_command == Some(index) || hud.selected == Some(Selected::Command(index))
            }),
        };
        if marked && !was {
            commands.entity(card).insert(Marked);
        } else if !marked && was {
            commands.entity(card).remove::<Marked>();
        }
    }
}

/// Die Spalte der Fähigkeit, getrennt von den Reihen.
type AdmiralOnly = (With<AdmiralArea>, Without<HandRow>);

/// Die Hände teilen sich die Leiste neben dem Aktionsknopf: Jede bekommt genau die Hälfte, damit der Knopf in der
/// Mitte bleibt. Reicht der Platz nicht, rücken die Karten der Hand zusammen und überlappen sich.
fn fit_rows(
    bar: Single<&ComputedNode, With<HandBar>>,
    action: Single<&ComputedNode, With<ActionArea>>,
    admiral: Single<(&ComputedNode, &Node), AdmiralOnly>,
    rows: Query<(&HandRow, &Children)>,
    mut cards: Query<&mut Node, (Without<HandRow>, Without<AdmiralArea>)>,
) {
    let logical = |node: &ComputedNode| node.size().x * node.inverse_scale_factor();
    let room = logical(&bar) - logical(&action) - 2.0 * CENTER_MARGIN;
    let (admiral_node, admiral_style) = *admiral;
    let admiral_width = if admiral_style.display == Display::None {
        0.0
    } else {
        logical(admiral_node) + SIDE_GAP
    };
    for (row, children) in &rows {
        let count = children.len();
        let space = room / 2.0
            - match row {
                HandRow::Ships => 0.0,
                HandRow::Commands => admiral_width,
            };
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
}

pub(super) struct HandPlugin;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownHand>()
            .add_systems(OnEnter(AppState::Match), spawn)
            .add_observer(click_card)
            .add_systems(
                Update,
                (rebuild_rows, set_hover_origins, mark_cards, fit_rows)
                    .chain()
                    .run_if(in_state(AppState::Match)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacks_keep_first_position() {
        let hand = [ShipId::Drone, ShipId::Hive, ShipId::Drone, ShipId::Drone];
        assert_eq!(
            stacks_of(&hand),
            vec![(ShipId::Drone, vec![0, 2, 3]), (ShipId::Hive, vec![1])]
        );
    }

    #[test]
    fn drawn_counts_copies() {
        let before = [ShipId::Drone, ShipId::Hive];
        let after = [ShipId::Drone, ShipId::Drone, ShipId::Hive, ShipId::Mirage];
        assert_eq!(drawn_indices(&before, &after), vec![1, 3]);
        assert_eq!(draw_delay(&[0, 1], &[1, 3]), Some(0.0));
        assert_eq!(draw_delay(&[3], &[1, 3]), Some(DRAW_STAGGER));
        assert_eq!(draw_delay(&[2], &[1, 3]), None);
    }
}
