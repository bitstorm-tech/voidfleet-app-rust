//! Vollbild (Port von `ui/fullscreen.ts`): Die Taste F und [`ToggleFullscreen`] (etwa der Doppelklick auf eine freie
//! Stelle des Bretts) schalten zwischen Vollbild und maximiertem Fenster um, wie bei einem Video. Der Modus steht in
//! den [`Settings`]; dort setzt ihn auch der Einstellungsdialog, und die Einstellungen schalten das Fenster und merken
//! ihn für den nächsten Start.

use bevy::prelude::*;

use super::view_toggle::{plain, typing};
use crate::{
    assets::AppState,
    settings::{Settings, WindowMode},
};

/// Schaltet das Vollbild um; verlassen wird es maximiert, so startet die App auch ohne gespeicherte Wahl.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ToggleFullscreen;

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

pub(super) struct FullscreenPlugin;

impl Plugin for FullscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToggleFullscreen>().add_systems(
            Update,
            (key_f.run_if(not(typing)), toggle)
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
