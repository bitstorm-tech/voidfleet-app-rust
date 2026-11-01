//! Aufgeben (Port von `ui/surrenderDialog.ts`): Die Fahne in der Icon-Leiste öffnet eine Rückfrage; erst „Aufgeben“
//! darin meldet [`SurrenderConfirmed`]. Der Knopf steht nur da, solange gegen einen Gegner gespielt wird und das Match
//! nicht entschieden ist; fällt das weg, schließt auch die Rückfrage.

use bevy::{
    prelude::*,
    text::LineHeight,
    ui_widgets::{Activate, observe},
};

use super::{
    Dialog, DialogButton, IconDialog, SurrenderConfirmed, close_button, close_dialogs,
    dialog_console, icon_dialog,
};
use crate::{
    assets::AppState,
    game::Match,
    i18n::Locale,
    palette,
    ui::{
        Fonts,
        button::{self, Key},
        theme,
    },
};

pub(super) fn spawn(mut commands: Commands, fonts: Res<Fonts>, locale: Res<Locale>) {
    let action = || Node {
        padding: UiRect::axes(px(24), px(8)),
        ..default()
    };
    commands.spawn(icon_dialog(
        Dialog::Surrender,
        dialog_console(
            &fonts,
            locale.surrender_title(),
            Node::default(),
            false,
            (
                Spawn((
                    Node {
                        max_width: px(360),
                        margin: UiRect::bottom(px(20)),
                        ..default()
                    },
                    theme::text(fonts.body(17.0), locale.surrender_question(), palette::TEXT_DIM),
                    LineHeight::RelativeToFont(1.5),
                    Pickable::IGNORE,
                )),
                Spawn((
                    Node {
                        justify_content: JustifyContent::FlexEnd,
                        column_gap: px(12),
                        ..default()
                    },
                    children![
                        close_button(&fonts, &locale, action()),
                        (
                            button::text_button(
                                fonts.display_bold(18.0),
                                locale.surrender_confirm(),
                                Key::DANGER,
                                action(),
                            ),
                            observe(close_dialogs),
                            observe(
                                |_: On<Activate>, mut confirmed: MessageWriter<SurrenderConfirmed>| {
                                    confirmed.write(SurrenderConfirmed);
                                }
                            ),
                        ),
                    ],
                )),
            ),
        ),
    ));
}

/// Der Knopf steht nur da, solange gegen einen Gegner gespielt wird; sonst schließt auch die Rückfrage.
fn availability(
    mut commands: Commands,
    game: Res<Match>,
    mut buttons: Query<(&DialogButton, &mut Node)>,
    dialogs: Query<(Entity, &IconDialog)>,
) {
    let available = game.opponent.is_some() && game.result.is_none();
    for (button, mut node) in &mut buttons {
        if button.0 == Dialog::Surrender {
            let display = if available {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    if !available {
        for (entity, dialog) in &dialogs {
            if dialog.0 == Dialog::Surrender {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub(super) struct SurrenderPlugin;

impl Plugin for SurrenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            availability
                .after(super::DialogSystems)
                .run_if(in_state(AppState::Match)),
        );
    }
}
