//! Vollbild (Port von `ui/fullscreen.ts`): Der Icon-Knopf oben rechts, die Taste F und [`ToggleFullscreen`] (etwa
//! der Doppelklick auf eine freie Stelle des Bretts) schalten zwischen Vollbild und maximiertem Fenster um, wie bei
//! einem Video. Der Modus steht in den [`Settings`]; dort setzt ihn auch der Einstellungsdialog, und die Einstellungen
//! schalten das Fenster und merken ihn für den nächsten Start.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};

use super::{UiIcons, button, tooltip::Tooltip, view_toggle::plain};
use crate::{
    assets::AppState,
    i18n::Locale,
    settings::{Settings, WindowMode},
};

/// Schaltet das Vollbild um; verlassen wird es maximiert, so startet die App auch ohne gespeicherte Wahl.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ToggleFullscreen;

#[derive(Component)]
struct FullscreenButton;

/// Der Icon-Knopf in der Leiste oben rechts.
pub(super) fn fullscreen_button(icons: &UiIcons, locale: &Locale) -> impl Bundle {
    (
        FullscreenButton,
        button::icon_button(icons.fullscreen_enter.clone(), locale.fullscreen_enter()),
        observe(
            |_: On<Activate>, mut toggle: MessageWriter<ToggleFullscreen>| {
                toggle.write(ToggleFullscreen);
            },
        ),
    )
}

fn toggle(mut messages: MessageReader<ToggleFullscreen>, mut settings: ResMut<Settings>) {
    // Zweimal im selben Bild hebt sich auf.
    if messages.read().count() % 2 == 1 {
        settings.window_mode = if settings.window_mode == WindowMode::Fullscreen {
            WindowMode::Maximized
        } else {
            WindowMode::Fullscreen
        };
    }
}

fn key_f(keys: Res<ButtonInput<KeyCode>>, mut toggle: MessageWriter<ToggleFullscreen>) {
    if keys.just_pressed(KeyCode::KeyF) && plain(&keys) {
        toggle.write(ToggleFullscreen);
    }
}

/// Icon und Tooltip folgen dem Modus.
fn update_button(
    settings: Res<Settings>,
    locale: Res<Locale>,
    icons: Res<UiIcons>,
    button: Single<(Entity, &mut Tooltip), With<FullscreenButton>>,
    children: Query<&Children>,
    mut images: Query<&mut ImageNode, With<button::KeyIcon>>,
) {
    if !settings.is_changed() && !locale.is_changed() {
        return;
    }
    let (entity, mut tooltip) = button.into_inner();
    let active = settings.window_mode == WindowMode::Fullscreen;
    let (icon, text) = if active {
        (&icons.fullscreen_exit, locale.fullscreen_exit())
    } else {
        (&icons.fullscreen_enter, locale.fullscreen_enter())
    };
    button::set_icon(entity, icon, &children, &mut images);
    if tooltip.0 != text {
        tooltip.0 = text.into();
    }
}

pub(super) struct FullscreenPlugin;

impl Plugin for FullscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToggleFullscreen>().add_systems(
            Update,
            (key_f, toggle, update_button)
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
