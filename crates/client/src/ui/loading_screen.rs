//! Ladebildschirm beim Start (Port von `ui/loadingScreen.ts`): deckt alles ab, bis die Assets geladen sind und die
//! Generalprobe steht, zeigt den Fortschritt und blendet dann aus.

use bevy::{prelude::*, text::LetterSpacing};

use super::Fonts;
use crate::{
    assets::{AppState, LoadingProgress},
    i18n::Locale,
    palette,
};

/// Ausblenden am Ende in Sekunden; der Titel pulsiert in diesem Takt.
const FADE: f32 = 0.6;
const PULSE: f32 = 2.4;
const BAR_WIDTH: f32 = 360.0;

/// Das Overlay; `fade` läuft nach dem Laden von 1 auf 0.
#[derive(Component)]
struct LoadingOverlay {
    fade: f32,
}

#[derive(Component)]
struct Title;

#[derive(Component)]
struct BarFill;

#[derive(Component)]
struct Label;

pub(super) struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(
                Update,
                (show_progress, pulse_title).run_if(in_state(AppState::Loading)),
            )
            .add_systems(Update, fade_out.run_if(in_state(AppState::Match)));
    }
}

/// Text unter dem Balken.
fn label(locale: &Locale, progress: LoadingProgress) -> String {
    if progress.failed {
        locale.loading_failed().into()
    } else {
        locale.loading_assets((progress.fraction * 100.0).round() as u32)
    }
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        Name::new("Ladebildschirm"),
        LoadingOverlay { fade: 1.0 },
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(28),
            ..default()
        },
        BackgroundColor(palette::BACKGROUND),
        GlobalZIndex(100),
        children![
            (
                Title,
                Text::new("VOIDFLEET"),
                fonts.display_bold(84.0),
                LetterSpacing::Px(84.0 * 0.35),
                TextColor(palette::PLAYER),
                TextShadow {
                    offset: Vec2::ZERO,
                    color: palette::PLAYER.with_alpha(0.55),
                },
            ),
            (
                Node {
                    width: px(BAR_WIDTH),
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                children![(
                    BarFill,
                    Node {
                        width: px(0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(palette::PLAYER),
                )],
            ),
            (
                Label,
                Text::default(),
                fonts.display(16.0),
                LetterSpacing::Px(16.0 * 0.06),
                TextColor(palette::TEXT_DIM)
            ),
        ],
    ));
}

fn show_progress(
    progress: Res<LoadingProgress>,
    locale: Res<Locale>,
    mut fill: Single<&mut Node, With<BarFill>>,
    label_text: Single<(&mut Text, &mut TextColor), With<Label>>,
) {
    if !progress.is_changed() && !locale.is_changed() {
        return;
    }
    fill.width = px(BAR_WIDTH * progress.fraction);
    let (mut text, mut color) = label_text.into_inner();
    text.0 = label(&locale, *progress);
    color.0 = if progress.failed {
        palette::DAMAGE
    } else {
        palette::TEXT_DIM
    };
}

/// Der Titel pulsiert langsam, solange geladen wird.
fn pulse_title(time: Res<Time>, mut title: Single<&mut TextColor, With<Title>>) {
    let phase = (time.elapsed_secs() / PULSE * std::f32::consts::TAU).cos();
    title.0.set_alpha(0.825 + 0.175 * phase);
}

/// Blendet das Overlay samt Inhalt aus und entfernt es.
fn fade_out(
    mut commands: Commands,
    time: Res<Time>,
    overlay: Single<(Entity, &mut LoadingOverlay)>,
    children: Query<&Children>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut texts: Query<&mut TextColor>,
) {
    let (entity, mut state) = overlay.into_inner();
    let previous = state.fade;
    state.fade = (state.fade - time.delta_secs() / FADE).max(0.0);
    if state.fade == 0.0 {
        commands.entity(entity).despawn();
        return;
    }
    let factor = state.fade / previous;
    for part in std::iter::once(entity).chain(children.iter_descendants(entity)) {
        if let Ok(mut background) = backgrounds.get_mut(part) {
            let alpha = background.0.alpha() * factor;
            background.0.set_alpha(alpha);
        }
        if let Ok(mut text) = texts.get_mut(part) {
            let alpha = text.0.alpha() * factor;
            text.0.set_alpha(alpha);
        }
    }
}
