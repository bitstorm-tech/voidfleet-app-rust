//! Ergebnis nach Gefechtsende (Port von `ui/resultScreen.ts`): Sieg oder Niederlage groß in der Bildmitte, darunter
//! die Abschüsse bzw. das verlorene Flaggschiff. „Brett ansehen“ blendet das Overlay aus und lässt ein Abzeichen oben
//! in der Mitte stehen, über das es wieder aufgeht. „Weiter“ räumt das Gefecht ab ([`LeaveBattle`]); der Run zeigt
//! dann die Beute bzw. sein Ende. Beim Erscheinen erklingt eine Fanfare, bei Niederlage eine düstere.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::battle::BattleResult;

use super::{FadeIn, PopIn, RESULT_LAYER};
use crate::{
    assets::AppState,
    audio::{PlaySound, SoundId},
    game::{Battle, LeaveBattle},
    i18n::Locale,
    palette,
    playback::PlaybackFinished,
    ui::{
        Fonts,
        button::{self, Key},
        theme,
    },
};

/// Das gezeigte Ergebnis und ob das Overlay offen ist (sonst steht das Abzeichen da).
#[derive(Resource, Default, Debug)]
struct ResultView {
    result: Option<BattleResult>,
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
fn result_color(result: BattleResult) -> Color {
    match result {
        BattleResult::Won => palette::WON,
        BattleResult::Lost => palette::LOST,
    }
}

/// Der Untertitel nennt die Abschüsse oder das verlorene Flaggschiff.
fn subtitle(battle: &Battle, locale: &Locale, result: BattleResult) -> String {
    match result {
        BattleResult::Won => locale.result_won_subtitle(battle.kills, battle.round),
        BattleResult::Lost => {
            locale.result_lost_subtitle(locale.flagship(battle.flagship.admiral.flagship()))
        }
    }
}

/// Overlay und Abzeichen für ein Ergebnis.
fn spawn_result(
    commands: &mut Commands,
    fonts: &Fonts,
    locale: &Locale,
    battle: &Battle,
    result: BattleResult,
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
                            subtitle(battle, locale, result),
                            palette::TEXT
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
                                    locale.result_continue(),
                                    Key::PRIMARY,
                                    action_node(),
                                ),
                                observe(
                                    |_: On<Activate>, mut leave: MessageWriter<LeaveBattle>| {
                                        leave.write(LeaveBattle);
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

/// Öffnet das Overlay, sobald ein Ergebnis feststeht und die Wiedergabe es gezeigt hat (mit Fanfare), und nimmt es
/// weg, wenn ein neues Gefecht beginnt oder der Run endet. Eine andere Sprache baut es neu auf.
fn follow_battle(
    mut commands: Commands,
    mut played: MessageReader<PlaybackFinished>,
    battle: Option<Res<Battle>>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    mut view: ResMut<ResultView>,
    shown: Query<Entity, With<ResultPart>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let played = played.read().count() > 0;
    let result = battle.as_ref().and_then(|battle| battle.result);
    if result.is_some() && view.result.is_none() && !played {
        return;
    }
    let changed = result != view.result;
    if !changed && !locale.is_changed() {
        return;
    }
    if changed {
        *view = ResultView {
            result,
            open: result.is_some(),
        };
        if let Some(result) = result {
            let fanfare = if result == BattleResult::Won {
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
    if let (Some(result), Some(battle)) = (view.result, &battle) {
        spawn_result(&mut commands, &fonts, &locale, battle, result, view.open);
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
            (follow_battle, toggle.run_if(resource_changed::<ResultView>))
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
