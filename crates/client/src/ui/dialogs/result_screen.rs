//! Ergebnis nach Matchende (Port von `ui/resultScreen.ts`): Sieg, Niederlage oder Unentschieden groß in der Bildmitte,
//! darunter das zerstörte Flaggschiff, nach einer Aufgabe stattdessen die Aufgabe. „Brett ansehen“ blendet das Overlay
//! aus und lässt ein Abzeichen oben in der Mitte stehen, über das es wieder aufgeht; „Neues Match“ meldet
//! [`NewMatch`]. Beim Erscheinen erklingt eine Fanfare, bei Niederlage und Unentschieden eine düstere.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::{admirals::AdmiralId, protocol::MatchResult};

use super::{FadeIn, NewMatch, PopIn, RESULT_LAYER};
use crate::{
    assets::AppState,
    audio::{PlaySound, SoundId},
    game::Match,
    i18n::Locale,
    palette,
    ui::{
        Fonts,
        button::{self, Key},
        theme,
    },
};

/// Das gezeigte Ergebnis und ob das Overlay offen ist (sonst steht das Abzeichen da).
#[derive(Resource, Default, Debug)]
struct ResultView {
    result: Option<MatchResult>,
    open: bool,
}

/// Overlay und Abzeichen: alles, was ein neues Ergebnis ersetzt.
#[derive(Component, Default)]
struct ResultPart;

#[derive(Component)]
#[require(ResultPart)]
struct ResultOverlay;

#[derive(Component)]
#[require(ResultPart)]
struct ResultBadge;

/// „Brett ansehen“.
#[derive(Component)]
pub(in crate::ui) struct BoardButton;

/// Farbe des Ausgangs.
fn result_color(result: MatchResult) -> Color {
    match result {
        MatchResult::Won => palette::WON,
        MatchResult::Lost => palette::LOST,
        MatchResult::Draw => palette::DRAW,
    }
}

/// Der Untertitel nennt die Aufgabe oder das zerstörte Flaggschiff.
fn subtitle(game: &Match, locale: &Locale, result: MatchResult) -> String {
    if game.surrendered && result != MatchResult::Draw {
        return locale.result_surrendered(result == MatchResult::Won).into();
    }
    let flagship = |admiral: Option<AdmiralId>| {
        admiral.map_or("", |admiral| locale.ship(admiral.flagship()).name)
    };
    let own = flagship(game.admiral);
    let enemy = flagship(game.opponent.as_ref().and_then(|opponent| opponent.admiral));
    match result {
        MatchResult::Won => locale.result_won_subtitle(enemy),
        MatchResult::Lost => locale.result_lost_subtitle(own),
        MatchResult::Draw => locale.result_draw_subtitle(own, enemy),
    }
}

/// Overlay und Abzeichen für ein Ergebnis.
fn spawn_result(
    commands: &mut Commands,
    fonts: &Fonts,
    locale: &Locale,
    game: &Match,
    result: MatchResult,
    open: bool,
) {
    let color = result_color(result);
    let visible = |shown: bool| {
        if shown { Display::Flex } else { Display::None }
    };
    commands.spawn((
        Name::new("Ergebnis"),
        ResultOverlay,
        Node {
            display: visible(open),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..theme::overlay()
        },
        GlobalZIndex(RESULT_LAYER),
        children![
            (
                theme::overlay(),
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::FarthestCorner,
                    vec![
                        ColorStop::percent(color.with_alpha(0.18), 0.0),
                        ColorStop::percent(palette::BACKGROUND.with_alpha(0.82), 70.0),
                    ],
                )),
                FadeIn::new(0.6),
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(10),
                    min_width: px(480),
                    padding: UiRect::new(px(56), px(56), px(36), px(32)),
                    border: UiRect::top(px(4)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(theme::PANEL),
                BorderColor::all(color),
                BoxShadow(vec![theme::glow(color.with_alpha(0.25), 80.0)]),
                PopIn::new(0.85, 0.7, EaseFunction::BackOut),
                children![
                    (
                        theme::caps(
                            fonts.display_bold(100.0),
                            locale.result_title(result),
                            0.12,
                            color
                        ),
                        TextShadow {
                            offset: Vec2::ZERO,
                            color: color.with_alpha(0.6),
                        },
                        Pickable::IGNORE,
                    ),
                    (
                        theme::text(
                            fonts.body(24.0),
                            subtitle(game, locale, result),
                            palette::TEXT
                        ),
                        Pickable::IGNORE,
                    ),
                    (
                        theme::text(
                            fonts.body(16.0),
                            locale.result_rounds(game.round),
                            palette::TEXT_DIM
                        ),
                        Pickable::IGNORE,
                    ),
                    (
                        Node {
                            column_gap: px(16),
                            margin: UiRect::top(px(20)),
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![
                            (
                                BoardButton,
                                button::text_button(
                                    fonts.display_bold(22.0),
                                    locale.result_view_board(),
                                    Key::NORMAL,
                                    action_node(),
                                ),
                                observe(|_: On<Activate>, mut view: ResMut<ResultView>| {
                                    view.open = false;
                                }),
                            ),
                            (
                                button::text_button(
                                    fonts.display_bold(22.0),
                                    locale.result_new_match(),
                                    Key::PRIMARY,
                                    action_node(),
                                ),
                                observe(
                                    |_: On<Activate>, mut new_match: MessageWriter<NewMatch>| {
                                        new_match.write(NewMatch);
                                    }
                                ),
                            ),
                        ],
                    ),
                ],
            ),
        ],
    ));
    // Das Abzeichen in der Farbe des Ausgangs: Rahmen und Schrift statt der grauen Taste.
    commands.spawn((
        Name::new("Ergebnis-Abzeichen"),
        ResultBadge,
        Node {
            display: visible(!open),
            position_type: PositionType::Absolute,
            top: px(18),
            left: Val::ZERO,
            right: Val::ZERO,
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(RESULT_LAYER),
        Pickable::IGNORE,
        children![(
            button::button(
                Key::NORMAL,
                Node {
                    padding: UiRect::axes(px(28), px(8)),
                    ..default()
                },
            ),
            observe(|_: On<Activate>, mut view: ResMut<ResultView>| {
                view.open = true;
            }),
            children![
                (
                    theme::caps(
                        fonts.display_bold(24.0),
                        locale.result_title(result),
                        0.1,
                        color
                    ),
                    theme::text_shadow(0.55),
                    Pickable::IGNORE,
                ),
                (
                    Node {
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(5)),
                        ..theme::overlay()
                    },
                    BorderColor::all(color.with_alpha(0.7)),
                    Pickable::IGNORE,
                ),
            ],
        )],
    ));
}

/// Größe der Knöpfe im Ergebnis.
fn action_node() -> Node {
    Node {
        width: px(200),
        height: px(52),
        ..default()
    }
}

/// Öffnet das Overlay, sobald ein Ergebnis feststeht (mit Fanfare), und nimmt es weg, wenn ein neues Match beginnt.
/// Eine andere Sprache baut es neu auf.
#[allow(clippy::too_many_arguments)]
fn follow_match(
    mut commands: Commands,
    game: Res<Match>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    mut view: ResMut<ResultView>,
    shown: Query<Entity, With<ResultPart>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let changed = game.result != view.result;
    if !changed && !locale.is_changed() {
        return;
    }
    if changed {
        *view = ResultView {
            result: game.result,
            open: game.result.is_some(),
        };
        if let Some(result) = game.result {
            let fanfare = if result == MatchResult::Won {
                SoundId::Victory
            } else {
                SoundId::Defeat
            };
            sounds.write(PlaySound::new(fanfare));
        }
    }
    for entity in &shown {
        commands.entity(entity).despawn();
    }
    if let Some(result) = view.result {
        spawn_result(&mut commands, &fonts, &locale, &game, result, view.open);
    }
}

/// „Brett ansehen“ und das Abzeichen schalten zwischen Overlay und Abzeichen um.
fn toggle(
    view: Res<ResultView>,
    mut overlay: Query<&mut Node, (With<ResultOverlay>, Without<ResultBadge>)>,
    mut badge: Query<&mut Node, With<ResultBadge>>,
) {
    let display = |shown: bool| if shown { Display::Flex } else { Display::None };
    for mut node in &mut overlay {
        node.display = display(view.open);
    }
    for mut node in &mut badge {
        node.display = display(!view.open);
    }
}

pub(super) struct ResultScreenPlugin;

impl Plugin for ResultScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ResultView>().add_systems(
            Update,
            (follow_match, toggle.run_if(resource_changed::<ResultView>))
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
