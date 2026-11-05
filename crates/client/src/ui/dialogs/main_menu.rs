//! Das Hauptmenü nach dem Laden: Titel, ein Satz zur Geschichte, „Neuer Run“ (öffnet die Admiralswahl) und, wenn ein
//! nicht beendeter Speicherstand da ist, „Weiter“ mit Admiral, Sektor und Reihe. Die Icon-Knöpfe oben rechts bleiben.

use bevy::prelude::*;

use super::{
    overlay_backdrop,
    run_nodes::{message_button, screen_root},
};
use crate::{
    assets::AppState,
    controller::Scripted,
    i18n::Locale,
    palette,
    run::{RunFlow, RunSave, Screen},
    settings::Language,
    ui::{Fonts, button::Key, theme},
};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MainMenu(Language);

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    save: Res<RunSave>,
    shown: Query<(Entity, &MainMenu)>,
) {
    let wanted = (*screen == Screen::Menu).then_some(MainMenu(locale.language));
    if !screen.is_changed() && shown.iter().map(|(_, menu)| *menu).eq(wanted) {
        return;
    }
    for (entity, _) in &shown {
        commands.entity(entity).despawn();
    }
    let Some(menu) = wanted else {
        return;
    };
    let saved = save.load();
    commands
        .spawn((Name::new("Hauptmenü"), menu, screen_root()))
        .with_children(|root| {
            root.spawn(overlay_backdrop());
            root.spawn((
                theme::caps(fonts.display_bold(110.0), "Voidfleet", 0.16, theme::ACCENT),
                TextShadow {
                    offset: Vec2::ZERO,
                    color: palette::PLAYER.with_alpha(0.5),
                },
                Pickable::IGNORE,
            ));
            root.spawn((
                Node {
                    max_width: px(640),
                    margin: UiRect::bottom(px(28)),
                    ..default()
                },
                theme::text(fonts.body(18.0), locale.menu_tagline(), palette::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
            if let Some(run) = &saved {
                root.spawn(message_button(
                    &fonts,
                    locale.menu_continue(),
                    Key::PRIMARY,
                    true,
                    RunFlow::Continue,
                ));
                root.spawn((
                    theme::text(
                        fonts.body(15.0),
                        locale.menu_saved_run(
                            &locale.admiral_title(run.admiral),
                            run.sector,
                            run.path.len(),
                        ),
                        palette::TEXT_DIM,
                    ),
                    Node {
                        margin: UiRect::bottom(px(12)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            root.spawn(message_button(
                &fonts,
                locale.menu_new_run(),
                if saved.is_some() {
                    Key::NORMAL
                } else {
                    Key::PRIMARY
                },
                true,
                RunFlow::NewRun,
            ));
        });
}

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        // Dev-Szenen stellen ihr Bild selbst, ohne Run; das Menü hätte sonst nichts zu tun als es zu verdecken.
        app.add_systems(
            Update,
            show.run_if(in_state(AppState::Match))
                .run_if(not(resource_exists::<Scripted>)),
        );
    }
}
