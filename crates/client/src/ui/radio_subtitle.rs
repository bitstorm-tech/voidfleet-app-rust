//! Untertitel des Bordfunks über dem Aktionsknopf (Port von `ui/radioSubtitle.ts`): Absender mit zuckenden
//! Pegelbalken und der Text des laufenden Funkspruchs in der Sprache des Spielers. Er erscheint mit dem Spruch
//! ([`RadioTransmission`]) und blendet kurz nach dessen Ende aus. Außerhalb des Gefechts bleibt er verborgen; die Overlays
//! des Runs füllen dort den Bildschirm.

use bevy::prelude::*;

use super::{Fonts, theme, theme::UiOpacity};
use crate::{assets::AppState, audio::RadioTransmission, i18n::Locale, palette, run::Screen};

/// So lange bleibt der Untertitel nach dem Ende des Spruchs stehen, und so lange blendet er ein bzw. aus (Sekunden).
const LINGER: f32 = 0.9;
const FADE: f32 = 0.25;
/// Halbe Periode der Pegelbalken und ihr Versatz.
const LEVEL: f32 = 0.45;
const LEVEL_OFFSETS: [f32; 3] = [0.0, -0.2, -0.35];

/// Die Zeile, die den Untertitel trägt.
#[derive(Component)]
struct SubtitleRow;

#[derive(Component)]
struct Subtitle;

#[derive(Component)]
struct SubtitleText;

/// Der Absender: Flottenfunk oder abgehörter Gegnerfunk, je Spruch.
#[derive(Component)]
struct SpeakerText;

#[derive(Component)]
struct LevelBar(f32);

/// Wie lange der Untertitel steht (ab Beginn des Spruchs) und wie weit er eingeblendet ist.
#[derive(Resource, Default)]
struct OnAir {
    standing: Timer,
    shown: f32,
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>, locale: Res<Locale>) {
    let bar = |offset: f32, gap: f32| {
        (
            LevelBar(offset),
            Node {
                width: px(3),
                height: px(10),
                margin: UiRect::right(px(gap)),
                border_radius: BorderRadius::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::PLAYER),
            Pickable::IGNORE,
        )
    };
    commands.spawn((
        Name::new("Bordfunk"),
        SubtitleRow,
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            bottom: px(296),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Subtitle,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(3),
                max_width: px(640),
                padding: UiRect::new(px(18), px(18), px(7), px(9)),
                border: UiRect::bottom(px(2)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(theme::rgba(28, 32, 40, 0.78)),
            BorderColor::all(palette::PLAYER.with_alpha(0.7)),
            UiOpacity(0.0),
            Visibility::Hidden,
            Pickable::IGNORE,
            children![
                (
                    Node {
                        align_items: AlignItems::FlexEnd,
                        column_gap: px(2),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        bar(LEVEL_OFFSETS[0], 0.0),
                        bar(LEVEL_OFFSETS[1], 0.0),
                        bar(LEVEL_OFFSETS[2], 6.0),
                        (
                            SpeakerText,
                            theme::caps(
                                fonts.display(13.0),
                                locale.radio_speaker(false),
                                0.14,
                                palette::PLAYER
                            ),
                            Pickable::IGNORE,
                        ),
                    ],
                ),
                (
                    SubtitleText,
                    theme::text(fonts.body(20.0), "", palette::TEXT),
                    theme::text_shadow(0.8),
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ),
            ],
        )],
    ));
}

/// Ein neuer Spruch zeigt seinen Text; das Ende des Funks blendet aus.
fn receive(
    mut transmissions: MessageReader<RadioTransmission>,
    locale: Res<Locale>,
    mut on_air: ResMut<OnAir>,
    mut text: Single<&mut Text, With<SubtitleText>>,
    speaker: Single<(&mut Text, &mut TextColor), (With<SpeakerText>, Without<SubtitleText>)>,
) {
    let (mut speaker, mut speaker_color) = speaker.into_inner();
    for RadioTransmission(transmission) in transmissions.read() {
        match transmission {
            Some(transmission) => {
                let lines = locale.radio_lines(transmission.cue);
                text.0 = lines[transmission.variant.min(lines.len() - 1)].into();
                let intercepted = transmission.cue.intercepted();
                speaker.0 = locale.radio_speaker(intercepted).to_uppercase();
                speaker_color.0 = if intercepted {
                    palette::ENEMY
                } else {
                    palette::PLAYER
                };
                on_air.standing =
                    Timer::from_seconds(transmission.duration + LINGER, TimerMode::Once);
            }
            None => on_air.standing.finish(),
        }
    }
}

/// Nur im Gefecht zu sehen.
fn follow_screen(screen: Res<Screen>, mut row: Single<&mut Visibility, With<SubtitleRow>>) {
    row.set_if_neq(if *screen == Screen::Battle {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}

/// Ein- und Ausblenden samt leichtem Anheben, dazu die Pegelbalken.
fn animate(
    time: Res<Time>,
    mut on_air: ResMut<OnAir>,
    subtitle: Single<(&mut UiOpacity, &mut UiTransform, &mut Visibility), With<Subtitle>>,
    mut bars: Query<(&LevelBar, &mut UiTransform), Without<Subtitle>>,
) {
    let target = if on_air.standing.tick(time.delta()).is_finished() {
        0.0
    } else {
        1.0
    };
    let step = time.delta_secs() / FADE;
    on_air.shown = if target > on_air.shown {
        (on_air.shown + step).min(1.0)
    } else {
        (on_air.shown - step).max(0.0)
    };
    let (mut opacity, mut transform, mut visibility) = subtitle.into_inner();
    let shown = on_air.shown;
    visibility.set_if_neq(if shown > 0.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    if shown == 0.0 {
        return;
    }
    opacity.set_if_neq(UiOpacity(shown));
    let eased = EaseFunction::QuadraticOut.sample_clamped(shown);
    transform.translation = Val2::px(0.0, 8.0 * (1.0 - eased));
    // Pegel: von 30 % auf volle Höhe und zurück, je Balken versetzt; von unten aus.
    for (LevelBar(offset), mut bar) in &mut bars {
        let phase = ((time.elapsed_secs() + offset) / LEVEL).rem_euclid(2.0);
        let rise = if phase < 1.0 { phase } else { 2.0 - phase };
        let eased = 0.5 - 0.5 * (rise * std::f32::consts::PI).cos();
        let scale = 0.3 + 0.7 * eased;
        bar.scale = Vec2::new(1.0, scale);
        bar.translation = Val2::percent(0.0, (1.0 - scale) * 50.0);
    }
}

pub(super) struct RadioSubtitlePlugin;

impl Plugin for RadioSubtitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OnAir>()
            .add_systems(OnEnter(AppState::Match), spawn)
            .add_systems(
                Update,
                (follow_screen, receive, animate)
                    .chain()
                    .run_if(in_state(AppState::Match)),
            );
    }
}
