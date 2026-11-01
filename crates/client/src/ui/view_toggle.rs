//! Ansicht (Port von `ui/viewToggle.ts`): Der Icon-Knopf oben rechts und die Taste V fahren mit einer kurzen
//! Kamerafahrt zur anderen Ansicht ([`CameraCommand::ToggleView`]); solange ← oder → gedrückt ist, dreht sich die
//! Kamera stufenlos dazwischen ([`CameraCommand::Orbit`]), ↑ und ↓ zoomen ([`CameraCommand::Zoom`]), außer bei offenem
//! Dialog ([`Modal`]). Die Tastatur der Kamera liegt allein hier; die Maus liest die Kamera selbst.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};

use super::{UiIcons, button, theme::Modal};
use crate::{assets::AppState, i18n::Locale, scene::camera::CameraCommand};

/// Der Icon-Knopf in der Leiste oben rechts.
pub(super) fn view_button(icons: &UiIcons, locale: &Locale) -> impl Bundle {
    (
        button::icon_button(icons.view_toggle.clone(), locale.camera_view_toggle()),
        observe(
            |_: On<Activate>, mut camera: MessageWriter<CameraCommand>| {
                camera.write(CameraCommand::ToggleView);
            },
        ),
    )
}

/// Ob eine Taste ohne Strg, Cmd und Alt gedrückt ist; mit ihnen gehört sie dem System.
pub(super) fn plain(keys: &ButtonInput<KeyCode>) -> bool {
    !keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
    ])
}

/// V wechselt die Ansicht; ← dreht nach hinten oben, → zurück zur Seite, ↑ zoomt hinein, ↓ heraus, Gegenrichtungen
/// heben sich auf. Verliert das Fenster den Fokus, lässt Bevy alle Tasten los, und Drehung und Zoom enden.
fn steer(
    keys: Res<ButtonInput<KeyCode>>,
    modals: Query<(), With<Modal>>,
    mut held: Local<(f32, f32)>,
    mut camera: MessageWriter<CameraCommand>,
) {
    let plain = plain(&keys);
    if plain && keys.just_pressed(KeyCode::KeyV) {
        camera.write(CameraCommand::ToggleView);
    }
    // In offenen Dialogen gehören die Pfeiltasten den Reglern und dem Scrollen.
    let free = plain && modals.is_empty();
    let axis = |positive: KeyCode, negative: KeyCode| {
        let pressed = |key: KeyCode| f32::from(u8::from(free && keys.pressed(key)));
        pressed(positive) - pressed(negative)
    };
    let next = (
        axis(KeyCode::ArrowLeft, KeyCode::ArrowRight),
        axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
    );
    if next.0 != held.0 {
        camera.write(CameraCommand::Orbit(next.0));
    }
    if next.1 != held.1 {
        camera.write(CameraCommand::Zoom(next.1));
    }
    *held = next;
}

pub(super) struct ViewTogglePlugin;

impl Plugin for ViewTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, steer.run_if(in_state(AppState::Match)));
    }
}
