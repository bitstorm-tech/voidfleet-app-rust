//! Was man vom Gegner weiß, oben rechts unter den Icon-Knöpfen (Port von `ui/opponentPanel.ts`): sein Admiral, sobald
//! beide gewählt haben (Porträt, Name, Flaggschiff und Admiralsfähigkeit, ihre Texte im Tooltip), ob er seinen Zug
//! schon beendet hat und wie viele Schiffs- und Befehlskarten er auf der Hand und noch im Deck hat. Welche Befehle ihm
//! noch bleiben, zeigt das Feld bewusst nicht — mitzuzählen ist Sache des Spielers.

use bevy::{prelude::*, text::LineHeight};
use voidfleet_core::{admirals::AdmiralId, protocol::OpponentView};

use super::{Fonts, UiIcons, theme, tooltip::Tooltip};
use crate::{
    assets::AppState, data::admirals::AdmiralPortraits, game::Match, i18n::Locale, palette,
};

#[derive(Component)]
struct OpponentPanel;

#[derive(Component)]
struct StateLabel;

/// Haken vor „Zug beendet“.
#[derive(Component)]
struct ReadyTick;

/// Der Bereich des Admirals; neu gebaut, wenn er bekannt wird.
#[derive(Component)]
struct AdmiralBlock(Option<AdmiralId>);

/// Eine der vier Kartenzahlen, in der Reihenfolge Hand Schiffe, Hand Befehle, Deck Schiffe, Deck Befehle.
#[derive(Component)]
struct Count(usize);

fn counts(opponent: &OpponentView) -> [usize; 4] {
    [
        opponent.ship_hand_size,
        opponent.command_hand_size,
        opponent.ship_deck_size,
        opponent.command_deck_size,
    ]
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>, icons: Res<UiIcons>, locale: Res<Locale>) {
    let dim = palette::TEXT_DIM;
    let heading = |text: &str| {
        (
            theme::caps(
                fonts.display_bold(12.5),
                text,
                theme::TRACKING,
                theme::rgba(199, 219, 250, 0.6),
            ),
            TextLayout::justify(Justify::Right),
            Pickable::IGNORE,
        )
    };
    let row = |text: &str| {
        (
            theme::text(fonts.body(13.0), text.to_owned(), dim),
            Pickable::IGNORE,
        )
    };
    let count = |index: usize| {
        (
            Count(index),
            theme::text(fonts.display(15.0), "0", palette::TEXT),
            TextLayout::justify(Justify::Right),
            Pickable::IGNORE,
        )
    };
    commands.spawn((
        Name::new("Gegner"),
        OpponentPanel,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: px(48),
            right: px(8),
            width: px(244),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(px(11), px(11), px(9), px(11)),
            border: UiRect::top(px(2)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme::rgba(28, 32, 40, 0.82)),
        BorderColor::all(palette::ENEMY.with_alpha(0.8)),
        children![
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Baseline,
                    margin: UiRect::bottom(px(8)),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        theme::caps(
                            fonts.display(17.0),
                            locale.opponent_title(),
                            theme::TRACKING,
                            palette::ENEMY
                        ),
                        Pickable::IGNORE,
                    ),
                    (
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(4),
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![
                            (
                                ReadyTick,
                                Node {
                                    display: Display::None,
                                    width: px(12),
                                    height: px(12),
                                    ..default()
                                },
                                ImageNode::new(icons.check.clone()).with_color(palette::ENEMY),
                                Pickable::IGNORE,
                            ),
                            (
                                StateLabel,
                                theme::text(fonts.body(13.0), "", dim),
                                Pickable::IGNORE
                            ),
                        ],
                    ),
                ],
            ),
            (
                AdmiralBlock(None),
                Node {
                    display: Display::None,
                    column_gap: px(9),
                    align_items: AlignItems::Center,
                    margin: UiRect::new(Val::ZERO, Val::ZERO, px(-2), px(10)),
                    padding: UiRect::bottom(px(9)),
                    border: UiRect::bottom(px(1)),
                    ..default()
                },
                BorderColor::all(Color::WHITE.with_alpha(0.08)),
                Pickable::IGNORE,
            ),
            (
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![
                        RepeatedGridTrack::fr(1, 1.0),
                        RepeatedGridTrack::auto(2),
                    ],
                    row_gap: px(2),
                    column_gap: px(12),
                    align_items: AlignItems::Baseline,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    row(""),
                    heading(locale.opponent_ships()),
                    heading(locale.opponent_commands()),
                    row(locale.opponent_hand()),
                    count(0),
                    count(1),
                    row(locale.opponent_deck()),
                    count(2),
                    count(3),
                ],
            ),
        ],
    ));
}

/// Eine Zeile mit Fähigkeit: Marke in ihrer Farbe, Name, Text im Tooltip.
fn ability_line(fonts: &Fonts, mark: Color, name: String, effect: String) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(5),
            ..default()
        },
        Tooltip(effect),
        children![
            (
                Node {
                    width: px(6),
                    height: px(6),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(mark),
                Pickable::IGNORE,
            ),
            (
                theme::text(fonts.body(12.5), name, palette::TEXT_DIM),
                LineHeight::RelativeToFont(1.25),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Porträt im Ring seiner Farbe, Name und Beiname, darunter Flaggschiff mit Fähigkeit und die Admiralsfähigkeit.
fn admiral_content(
    commands: &mut Commands,
    block: Entity,
    admiral: AdmiralId,
    fonts: &Fonts,
    portraits: &AdmiralPortraits,
    locale: &Locale,
) {
    let color = palette::admiral_color(admiral);
    let text = locale.admiral(admiral);
    let flagship = locale.ship(admiral.flagship());
    let power = locale.power(admiral.power());
    commands.spawn((
        Node {
            width: px(48),
            height: px(48),
            flex_shrink: 0.0,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        ImageNode::new(portraits.get(admiral)),
        BoxShadow(vec![
            theme::glow(color.with_alpha(0.5), 10.0),
            theme::ring(color, 2.0),
        ]),
        Pickable::IGNORE,
        ChildOf(block),
    ));
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            flex_shrink: 1.0,
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(block),
        children![
            (
                theme::text(fonts.display(15.0), text.name, palette::TEXT),
                LineHeight::RelativeToFont(1.2),
                Pickable::IGNORE,
            ),
            (
                theme::text(fonts.body(12.5), text.epithet, color),
                LineHeight::RelativeToFont(1.2),
                Pickable::IGNORE,
            ),
            ability_line(
                fonts,
                palette::FLAGSHIP_TRIM,
                format!("{} · {}", flagship.name, text.ability),
                flagship.effect.into_owned(),
            ),
            ability_line(fonts, color, power.name.into(), power.effect.into_owned()),
        ],
    ));
}

type StateOnly = (With<StateLabel>, Without<Count>);
type TickOnly = (
    With<ReadyTick>,
    Without<OpponentPanel>,
    Without<AdmiralBlock>,
);
type BlockOnly = (Without<OpponentPanel>, Without<ReadyTick>);

/// Folgt dem Spielzustand: ohne Gegner ausgeblendet; seinen Admiral nennt der Server erst, wenn beide gewählt haben.
fn update(
    mut commands: Commands,
    game: Res<Match>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    portraits: Res<AdmiralPortraits>,
    mut panel: Single<&mut Node, With<OpponentPanel>>,
    state: Single<(&mut Text, &mut TextColor), StateOnly>,
    mut tick: Single<&mut Node, TickOnly>,
    block: Single<(Entity, &mut AdmiralBlock, &mut Node), BlockOnly>,
    mut numbers: Query<(&Count, &mut Text), Without<StateLabel>>,
) {
    if !game.is_changed() && !locale.is_changed() {
        return;
    }
    let Some(opponent) = &game.opponent else {
        panel.display = Display::None;
        return;
    };
    panel.display = Display::Flex;
    let (mut label, mut color) = state.into_inner();
    let waiting = if game.admiral.is_some() {
        locale.opponent_choosing()
    } else {
        locale.opponent_hidden()
    };
    label.0 = match opponent.admiral {
        None => waiting,
        Some(_) if opponent.submitted => locale.opponent_ready(),
        Some(_) => locale.opponent_planning(),
    }
    .into();
    color.0 = if opponent.submitted {
        palette::TEXT
    } else {
        palette::TEXT_DIM
    };
    tick.display = if opponent.submitted {
        Display::Flex
    } else {
        Display::None
    };
    let values = counts(opponent);
    for (Count(index), mut text) in &mut numbers {
        text.0 = values[*index].to_string();
    }
    let (entity, mut shown, mut node) = block.into_inner();
    node.display = if opponent.admiral.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if shown.0 != opponent.admiral || locale.is_changed() {
        shown.0 = opponent.admiral;
        commands.entity(entity).despawn_related::<Children>();
        if let Some(admiral) = opponent.admiral {
            admiral_content(&mut commands, entity, admiral, &fonts, &portraits, &locale);
        }
    }
}

pub(super) struct OpponentPanelPlugin;

impl Plugin for OpponentPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Match), spawn)
            .add_systems(Update, update.run_if(in_state(AppState::Match)));
    }
}
