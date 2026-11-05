//! Der Stand des Runs in der Kopfzeile des HUD. Außerhalb des Gefechts: Hülle, Schrott, Deckgröße (Klick öffnet die
//! Deckgalerie) und die Module als Marken mit Tooltip. Im Gefecht (Hülle und Runde zeigt das HUD): Schrott, Sektor
//! und Reihe, die Module als eine Marke mit allen als Tooltip-Liste. Ohne Run bleibt sie leer.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};

use super::{
    Fonts,
    button::{self, Key},
    dialogs::run_nodes::{Gallery, Purpose, modules_tooltip},
    theme,
    tooltip::Tooltip,
};
use crate::{assets::AppState, game::Battle, i18n::Locale, palette, run::ActiveRun};

/// Der Platz des Runs in der Kopfzeile.
#[derive(Component)]
pub struct RunHeader;

/// Der leere Platz; gehört in die Kopfzeile des HUD.
pub fn run_header() -> impl Bundle {
    (
        RunHeader,
        Node {
            align_items: AlignItems::Center,
            column_gap: px(18),
            margin: UiRect::top(px(2)),
            ..default()
        },
        Pickable::IGNORE,
    )
}

fn label(fonts: &Fonts, text: &str, color: Color) -> impl Bundle {
    (
        theme::caps(fonts.display_bold(22.0), text, theme::TRACKING, color),
        Pickable::IGNORE,
    )
}

/// Eine Marke mit Rahmen und Tooltip (Module).
fn chip(fonts: &Fonts, text: &str, tooltip: String) -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(px(10), px(4)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BorderColor::all(palette::BUFF.with_alpha(0.6)),
        BackgroundColor(theme::PANEL.with_alpha(0.6)),
        Tooltip(tooltip),
        children![(
            theme::caps(
                fonts.display_bold(15.0),
                text,
                theme::TRACKING,
                palette::BUFF
            ),
            Pickable::IGNORE,
        )],
    )
}

fn update(
    mut commands: Commands,
    run: Option<Res<ActiveRun>>,
    battle: Option<Res<Battle>>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    header: Single<Entity, With<RunHeader>>,
    mut shown: Local<Option<(bool, bool)>>,
) {
    let now = (run.is_some(), battle.is_some());
    let fighting = now.1;
    let changed = run.as_ref().is_some_and(|run| run.is_changed())
        || *shown != Some(now)
        || locale.is_changed();
    if !changed {
        return;
    }
    *shown = Some(now);
    commands.entity(*header).despawn_related::<Children>();
    let Some(run) = run else {
        return;
    };
    commands.entity(*header).with_children(|bar| {
        if fighting {
            bar.spawn(label(
                &fonts,
                &locale.run_scrap(run.scrap).to_uppercase(),
                palette::BUFF,
            ));
            bar.spawn(label(
                &fonts,
                &locale
                    .run_position(run.sector, run.path.len())
                    .to_uppercase(),
                palette::TEXT_DIM,
            ));
            bar.spawn(chip(
                &fonts,
                &locale.run_modules(run.modules.len()),
                modules_tooltip(&locale, &run.modules),
            ));
            return;
        }
        bar.spawn(label(
            &fonts,
            &locale.hud_hull(run.hull, run.max_hull).to_uppercase(),
            palette::TEXT,
        ));
        bar.spawn(label(
            &fonts,
            &locale.run_scrap(run.scrap).to_uppercase(),
            palette::BUFF,
        ));
        bar.spawn((
            button::text_button(
                fonts.display_bold(16.0),
                &locale.run_deck(run.deck.len()),
                Key::NORMAL,
                Node {
                    padding: UiRect::axes(px(14), px(5)),
                    ..default()
                },
            ),
            Tooltip(locale.run_show_deck().to_owned()),
            observe(|_: On<Activate>, mut commands: Commands| {
                commands.insert_resource(Gallery::open(Purpose::View));
            }),
        ));
        for &module in &run.modules {
            let (name, text) = locale.module(module);
            bar.spawn(chip(&fonts, name, text.to_owned()));
        }
    });
}

pub(super) struct RunBarPlugin;

impl Plugin for RunBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update.run_if(in_state(AppState::Match)));
    }
}
