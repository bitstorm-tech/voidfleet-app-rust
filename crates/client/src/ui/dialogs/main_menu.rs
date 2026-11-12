//! Das Hauptmenü nach dem Laden: Titel und ein Satz zur Geschichte, darunter links die gespeicherten Runs als Karte
//! (Admiral mit Porträt, Flaggschiff, Sektor und Reihe, Hülle, Siege, Deck, Module, Schrott; die Pfeile daneben
//! blättern, der zuletzt gespielte steht vorn) und rechts das Pult mit „Neuer Run“ (öffnet die Admiralswahl), „Weiter“
//! (der gezeigte Run), „Spielregeln“, „Einstellungen“ und „Beenden“. Das Menü füllt das ganze Fenster; die
//! Icon-Knöpfe oben rechts fehlen hier.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::run::Run;

use super::{
    Dialog, OVERLAY_LAYER, OpenDialog,
    admiral_select::{CARD_FACE, SECTION_LABEL},
    overlay_backdrop,
    run_nodes::{Enabled, sized_message_button},
};
use crate::{
    assets::AppState,
    controller::Scripted,
    data::admirals::AdmiralPortraits,
    i18n::Locale,
    palette,
    run::{RunFlow, RunSave, Screen},
    settings::Language,
    ui::{
        Fonts, UiIcons,
        button::{self, Key, KeyIcon},
        card::ART_ASPECT,
        theme,
    },
};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct MainMenu(Language);

/// Die gespeicherten Runs, solange das Menü steht, und welchen die Karte zeigt.
#[derive(Resource, Default)]
struct Roster {
    runs: Vec<Run>,
    shown: usize,
}

/// Der Platz der Karte zwischen den Pfeilen; sein Inhalt wechselt beim Blättern.
#[derive(Component)]
struct CardSlot;

/// Ein Pfeil neben der Karte; rundherum, nach dem letzten Run kommt der erste.
fn arrow(icons: &UiIcons, forward: bool, enabled: bool) -> impl Bundle {
    (
        button::button(
            Key::NORMAL,
            Node {
                width: px(44),
                height: px(88),
                align_self: AlignSelf::Center,
                ..default()
            },
        ),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut roster: ResMut<Roster>| {
            let count = roster.runs.len().max(1);
            roster.shown = (roster.shown + if forward { 1 } else { count - 1 }) % count;
        }),
        children![(
            KeyIcon,
            Node {
                width: px(24),
                height: px(24),
                ..default()
            },
            ImageNode {
                flip_x: !forward,
                ..ImageNode::new(icons.arrow_right.clone()).with_color(palette::TEXT_DIM)
            },
            Pickable::IGNORE,
        )],
    )
}

/// Eine Taste des Pults über die ganze Breite.
fn menu_button<M: Message + Clone>(
    fonts: &Fonts,
    label: &str,
    key: Key,
    enabled: bool,
    message: M,
) -> impl Bundle {
    let node = Node {
        width: percent(100),
        padding: UiRect::vertical(px(11)),
        ..default()
    };
    sized_message_button(fonts, label, key, node, enabled, message)
}

/// Eine Angabe zum Run als eingelassene Marke.
fn stat(fonts: &Fonts, text: &str) -> impl Bundle + use<> {
    (
        Node {
            padding: UiRect::axes(px(10), px(4)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme::rgba(0, 0, 0, 0.28)),
        BorderColor::all(theme::rgba(255, 255, 255, 0.07)),
        Pickable::IGNORE,
        children![(
            theme::caps(
                fonts.display_bold(15.0),
                text,
                theme::TRACKING,
                palette::TEXT
            ),
            Pickable::IGNORE,
        )],
    )
}

/// Der Rahmen der Karte in der Farbe ihres Admirals.
fn card_frame(color: Color) -> impl Bundle {
    (
        Node {
            width: px(360),
            min_height: px(430),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::bottom(px(12)),
            overflow: Overflow::clip(),
            border: UiRect::new(px(1), px(1), px(4), px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(CARD_FACE),
        BorderColor {
            top: color,
            ..BorderColor::all(color.with_alpha(0.55))
        },
    )
}

/// Die Karte des gezeigten Runs; ohne Run ein Hinweis auf „Neuer Run“.
fn fill_slot(
    mut commands: Commands,
    slot: Single<(Entity, Ref<CardSlot>)>,
    roster: Res<Roster>,
    fonts: Res<Fonts>,
    portraits: Res<AdmiralPortraits>,
    locale: Res<Locale>,
) {
    let (slot, added) = &*slot;
    if !roster.is_changed() && !added.is_added() {
        return;
    }
    let mut slot = commands.entity(*slot);
    slot.despawn_related::<Children>();
    let Some(run) = roster.runs.get(roster.shown) else {
        slot.with_child((
            card_frame(palette::TEXT_DIM),
            children![(
                Node {
                    margin: UiRect::all(Val::Auto),
                    max_width: px(260),
                    ..default()
                },
                theme::text(fonts.body(17.0), locale.menu_no_runs(), palette::TEXT_DIM),
                TextLayout::justify(Justify::Center),
            )],
        ));
        return;
    };
    let color = palette::admiral_color(run.admiral);
    let text = locale.admiral(run.admiral);
    let section = || Node {
        padding: UiRect::horizontal(px(16)),
        ..default()
    };
    let stats = [
        locale.run_position(run.sector, run.path.len()),
        format!("{} {}/{}", locale.stats_hull(), run.hull, run.max_hull),
        locale.menu_victories(run.victories),
        locale.run_deck(run.deck.len()),
        locale.run_modules(run.modules.len()),
        locale.run_scrap(run.scrap),
    ];
    let modules: Vec<_> = run
        .modules
        .iter()
        .map(|&module| locale.module(module).0)
        .collect();
    slot.with_children(|slot| {
        slot.spawn(card_frame(color)).with_children(|card| {
            // Porträt, unten in die Karte auslaufend.
            card.spawn((
                Node {
                    height: px(200),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![
                    theme::cover_image(portraits.get(run.admiral), ART_ASPECT, 0.25),
                    (
                        theme::overlay(),
                        theme::vertical_gradient(&[
                            (CARD_FACE.with_alpha(0.0), 70.0),
                            (CARD_FACE, 100.0)
                        ]),
                    ),
                ],
            ));
            card.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::top(px(-28)),
                    ..section()
                },
                children![
                    (
                        theme::text(fonts.body(26.0), text.name, palette::TEXT),
                        theme::text_shadow(0.9),
                    ),
                    (
                        theme::text(fonts.body(17.0), text.epithet, color),
                        theme::text_shadow(0.9),
                    ),
                ],
            ));
            card.spawn((
                Node {
                    align_items: AlignItems::Baseline,
                    column_gap: px(8),
                    ..section()
                },
                children![
                    theme::caps(
                        fonts.display_bold(14.0),
                        locale.label_flagship(),
                        theme::TRACKING,
                        SECTION_LABEL,
                    ),
                    theme::text(
                        fonts.body(18.0),
                        locale.flagship(run.admiral.flagship()),
                        palette::FLAGSHIP_TRIM,
                    ),
                ],
            ));
            card.spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(6),
                    row_gap: px(6),
                    ..section()
                },
                Children::spawn(SpawnIter(stats.map(|text| stat(&fonts, &text)).into_iter())),
            ));
            if !modules.is_empty() {
                card.spawn((
                    section(),
                    children![theme::text(
                        fonts.body(14.0),
                        modules.join(" · "),
                        palette::TEXT_DIM
                    )],
                ));
            }
            card.spawn((
                Node {
                    margin: UiRect::new(Val::Auto, Val::Auto, Val::Auto, Val::ZERO),
                    ..default()
                },
                theme::text(
                    fonts.display_bold(15.0),
                    format!("{} / {}", roster.shown + 1, roster.runs.len()),
                    palette::TEXT_DIM,
                ),
            ));
        });
    });
}

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    locale: Res<Locale>,
    save: Res<RunSave>,
    mut roster: ResMut<Roster>,
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
    if screen.is_changed() {
        *roster = Roster {
            runs: save.list(),
            shown: 0,
        };
    }
    let saved = !roster.runs.is_empty();
    let browsable = roster.runs.len() > 1;
    commands
        .spawn((
            Name::new("Hauptmenü"),
            menu,
            Node {
                position_type: PositionType::Absolute,
                top: Val::ZERO,
                left: Val::ZERO,
                right: Val::ZERO,
                bottom: Val::ZERO,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(16)),
                ..default()
            },
            GlobalZIndex(OVERLAY_LAYER),
        ))
        .with_children(|root| {
            root.spawn(overlay_backdrop());
            root.spawn((
                theme::caps(fonts.display_bold(76.0), "Voidfleet", 0.16, theme::ACCENT),
                TextShadow {
                    offset: Vec2::ZERO,
                    color: palette::PLAYER.with_alpha(0.5),
                },
                Pickable::IGNORE,
            ));
            root.spawn((
                Node {
                    max_width: px(820),
                    margin: UiRect::bottom(px(24)),
                    ..default()
                },
                theme::text(fonts.body(18.0), locale.menu_tagline(), palette::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
            root.spawn(Node {
                column_gap: px(56),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        column_gap: px(18),
                        ..default()
                    },
                    children![
                        arrow(&icons, false, browsable),
                        (CardSlot, Node::default()),
                        arrow(&icons, true, browsable),
                    ],
                ));
                row.spawn(theme::console(
                    Node {
                        width: px(340),
                        row_gap: px(12),
                        ..default()
                    },
                    false,
                    Spawn(theme::console_header(
                        fonts.display_bold(28.0),
                        locale.menu_main(),
                    )),
                ))
                .with_children(|console| {
                    console.spawn(menu_button(
                        &fonts,
                        locale.menu_new_run(),
                        if saved { Key::NORMAL } else { Key::PRIMARY },
                        true,
                        RunFlow::NewRun,
                    ));
                    console.spawn((
                        button::text_button(
                            fonts.display_bold(18.0),
                            locale.menu_continue(),
                            Key::PRIMARY,
                            Node {
                                width: percent(100),
                                padding: UiRect::vertical(px(11)),
                                ..default()
                            },
                        ),
                        Enabled(saved),
                        observe(
                            |_: On<Activate>,
                             roster: Res<Roster>,
                             mut flow: MessageWriter<RunFlow>| {
                                if let Some(run) = roster.runs.get(roster.shown) {
                                    flow.write(RunFlow::Continue(run.seed));
                                }
                            },
                        ),
                    ));
                    console.spawn(menu_button(
                        &fonts,
                        locale.rules_title(),
                        Key::NORMAL,
                        true,
                        OpenDialog(Dialog::Rules),
                    ));
                    console.spawn(menu_button(
                        &fonts,
                        locale.settings_title(),
                        Key::NORMAL,
                        true,
                        OpenDialog(Dialog::Settings),
                    ));
                    // Der Abstand rückt „Beenden“ an den unteren Rand, weg von den übrigen Tasten.
                    console.spawn(Node {
                        flex_grow: 1.0,
                        min_height: px(24),
                        ..default()
                    });
                    console.spawn(menu_button(
                        &fonts,
                        locale.menu_quit(),
                        Key::NORMAL,
                        true,
                        AppExit::Success,
                    ));
                });
            });
        });
}

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        // Dev-Szenen stellen ihr Bild selbst, ohne Run; das Menü hätte sonst nichts zu tun als es zu verdecken.
        app.init_resource::<Roster>().add_systems(
            Update,
            (show, fill_slot)
                .chain()
                .run_if(in_state(AppState::Match))
                .run_if(not(resource_exists::<Scripted>)),
        );
    }
}
