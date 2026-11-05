//! Die Knoten des Runs als Overlays über dem leeren Brett, je nach [`Screen`]: Beute (Schrott, Reparatur, Karten oder
//! Module zur Wahl), Ereignis (Text und Entscheidungen samt Folgen, Unbezahlbares gesperrt) und sein Ergebnis, Werft
//! (Reparieren, Aufwerten, Ausmustern), Schwarzmarkt (Karten, Module, Karte entfernen, Reparatur, Verlassen) und das
//! Run-Ende. Dazu zwei Dialoge darüber: die Deckgalerie ([`Gallery`]: ansehen, aufwerten mit vorher und nachher,
//! ausmustern, im Markt entfernen) und die Bestätigung „Run abbrechen“ ([`AbandonPrompt`]).
//!
//! Alles baut sich aus [`ActiveRun`] neu auf, sobald er sich ändert; die Tasten schreiben [`RunAction`] bzw.
//! [`RunFlow`], ausführen tut sie [`crate::run`].

use bevy::{
    ecs::message::Message,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    ui_widgets::{Activate, ScrollArea, observe},
};
use voidfleet_core::{
    battle::BattleResult,
    cards::DeckCard,
    modules::ModuleId,
    run::{
        EventOption, Gain, MARKET_REPAIR, MARKET_REPAIR_PRICE, PILOT_PRICE, REMOVAL_PRICE, Run,
        Stage,
    },
};

use super::{DIALOG_LAYER, FadeIn, OVERLAY_LAYER, PopIn, overlay_backdrop};
use crate::{
    assets::AppState,
    i18n::Locale,
    palette,
    run::{ActiveRun, RunAction, RunFlow, RunView, Screen},
    settings::Language,
    ui::{
        Fonts,
        button::{self, Key},
        card::{CardAssets, CardHover, command_card},
        theme::{self, Modal, UiOpacity},
    },
};

/// Wozu die Deckgalerie offen ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    View,
    Upgrade,
    Remove,
    BuyRemoval,
}

/// Die offene Deckgalerie; mit `picked` steht die gewählte Karte zur Bestätigung.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gallery {
    pub purpose: Purpose,
    pub picked: Option<usize>,
}

impl Gallery {
    pub fn open(purpose: Purpose) -> Self {
        Self {
            purpose,
            picked: None,
        }
    }

    /// Ob die Karte wählbar ist.
    fn selectable(self, card: DeckCard) -> bool {
        match self.purpose {
            Purpose::View => false,
            Purpose::Upgrade => !card.upgraded,
            Purpose::Remove | Purpose::BuyRemoval => true,
        }
    }

    fn action(self, index: usize) -> Option<RunAction> {
        match self.purpose {
            Purpose::View => None,
            Purpose::Upgrade => Some(RunAction::Upgrade(index)),
            Purpose::Remove => Some(RunAction::Remove(index)),
            Purpose::BuyRemoval => Some(RunAction::BuyRemoval(index)),
        }
    }
}

/// Die Bestätigung „Run abbrechen“ ist offen.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct AbandonPrompt;

/// Eine Taste, frei oder gesperrt; gesetzt wird das beim Erscheinen.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct Enabled(pub bool);

/// Das Overlay eines Knotens, gebaut für diesen Bildschirm und diese Sprache.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct NodeOverlay(Screen, Language);

#[derive(Component)]
struct GalleryDialog;

#[derive(Component)]
struct AbandonDialog;

/// Eine Taste mit Beschriftung, die beim Klick `message` schreibt.
pub(super) fn message_button<M: Message + Clone>(
    fonts: &Fonts,
    label: &str,
    key: Key,
    enabled: bool,
    message: M,
) -> impl Bundle {
    sized_message_button(fonts, label, key, key_node(), enabled, message)
}

/// Innenabstand der Tasten.
fn key_node() -> Node {
    Node {
        padding: UiRect::axes(px(22), px(9)),
        ..default()
    }
}

/// Wie [`message_button`] in der Größe `node`.
fn sized_message_button<M: Message + Clone>(
    fonts: &Fonts,
    label: &str,
    key: Key,
    node: Node,
    enabled: bool,
    message: M,
) -> impl Bundle {
    (
        button::text_button(fonts.display_bold(18.0), label, key, node),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut commands: Commands| {
            commands.write_message(message.clone());
        }),
    )
}

/// Eine Taste, die beim Klick eine Ressource setzt (Galerie, Bestätigung).
fn resource_button<R: Resource + Clone>(
    fonts: &Fonts,
    label: &str,
    key: Key,
    enabled: bool,
    resource: R,
) -> impl Bundle {
    (
        button::text_button(fonts.display_bold(18.0), label, key, key_node()),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut commands: Commands| {
            commands.insert_resource(resource.clone());
        }),
    )
}

/// Sperrt neue Tasten, die gesperrt erscheinen sollen.
fn apply_enabled(mut commands: Commands, added: Query<(Entity, &Enabled), Added<Enabled>>) {
    for (entity, &Enabled(enabled)) in &added {
        if !enabled {
            button::set_enabled(&mut commands, entity, false);
        }
    }
}

/// Wurzel eines Overlays über dem Brett, unter der Kopfzeile.
pub(super) fn screen_root() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: px(64),
            left: Val::ZERO,
            right: Val::ZERO,
            bottom: Val::ZERO,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(10),
            padding: UiRect::all(px(16)),
            ..default()
        },
        GlobalZIndex(OVERLAY_LAYER),
    )
}

/// Das Pult eines Knotens mit Titel; der Inhalt kommt als Kinder dazu.
fn node_console(fonts: &Fonts, title: &str) -> impl Bundle {
    (
        theme::console(
            Node {
                max_width: px(1100),
                align_items: AlignItems::Center,
                row_gap: px(14),
                ..default()
            },
            false,
            Spawn(theme::console_header(fonts.display_bold(28.0), title)),
        ),
        PopIn::new(0.96, 0.32, EaseFunction::CubicOut),
    )
}

fn body(fonts: &Fonts, text: impl Into<String>, color: Color) -> impl Bundle {
    (
        Node {
            max_width: px(760),
            ..default()
        },
        theme::text(fonts.body(17.0), text, color),
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
    )
}

fn row(gap: f32) -> impl Bundle {
    (
        Node {
            column_gap: px(gap),
            row_gap: px(gap),
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexEnd,
            ..default()
        },
        Pickable::IGNORE,
    )
}

fn column(gap: f32) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(gap),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Eine Tafel (Modul, Werft-Auftrag, Dienst): Name, Text, darunter die Taste.
fn plate(fonts: &Fonts, name: &str, text: &str) -> impl Bundle {
    (
        theme::console_inset(Node {
            width: px(240),
            min_height: px(150),
            row_gap: px(8),
            align_items: AlignItems::Center,
            padding: UiRect::all(px(14)),
            ..default()
        }),
        Pickable::IGNORE,
        children![
            (
                theme::caps(
                    fonts.display_bold(20.0),
                    name,
                    theme::TRACKING,
                    palette::BUFF
                ),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ),
            (
                theme::text(fonts.body(15.0), text.to_owned(), palette::TEXT_DIM),
                TextLayout::justify(Justify::Center),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Hüllenverlust im Flackern: ein Viertel der maximalen Hülle, aufgerundet (wie `Run::choose`).
fn flicker_damage(run: &Run) -> i32 {
    (run.max_hull + 3) / 4
}

/// Baut das Overlay des Knotens neu, wenn Bildschirm, Sprache oder Run sich ändern.
fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    run: Option<Res<ActiveRun>>,
    view: Res<RunView>,
    assets: CardAssets,
    shown: Query<(Entity, &NodeOverlay)>,
) {
    let wanted = matches!(
        *screen,
        Screen::Loot
            | Screen::Event
            | Screen::EventOutcome
            | Screen::Shipyard
            | Screen::Market
            | Screen::Ended
    )
    .then_some(NodeOverlay(*screen, assets.locale.language));
    let run_changed = run.as_ref().is_some_and(|run| run.is_changed());
    if !run_changed && shown.iter().map(|(_, o)| *o).eq(wanted) {
        return;
    }
    for (entity, _) in &shown {
        commands.entity(entity).despawn();
    }
    let (Some(overlay), Some(run)) = (wanted, run) else {
        return;
    };
    let fonts = &*assets.fonts;
    let locale = &*assets.locale;
    let mut root = commands.spawn((Name::new("Knoten"), overlay, screen_root()));
    root.with_children(|parent| {
        parent.spawn(overlay_backdrop());
        match *screen {
            Screen::Loot => loot(parent, &assets, &run),
            Screen::Event => event(parent, fonts, locale, &run),
            Screen::EventOutcome => {
                if let Some(outcome) = view.outcome {
                    event_outcome(parent, fonts, locale, outcome);
                }
            }
            Screen::Shipyard => shipyard(parent, fonts, locale, &run),
            Screen::Market => market(parent, &assets, &run),
            _ => ended(parent, fonts, locale, &run),
        }
    });
}

fn loot(parent: &mut ChildSpawnerCommands, assets: &CardAssets, run: &Run) {
    let Stage::Loot(loot) = &run.stage else {
        return;
    };
    let (fonts, locale) = (&*assets.fonts, &*assets.locale);
    let mut lines = Vec::new();
    if loot.scrap > 0 {
        lines.push(locale.run_scrap_change(i64::from(loot.scrap)));
    }
    if loot.repaired > 0 {
        lines.push(locale.run_hull_change(loot.repaired));
    }
    parent
        .spawn(node_console(fonts, locale.loot_title()))
        .with_children(|console| {
            if !lines.is_empty() {
                console.spawn((
                    theme::caps(
                        fonts.display_bold(22.0),
                        &lines.join("   "),
                        theme::TRACKING,
                        palette::BUFF,
                    ),
                    Pickable::IGNORE,
                ));
            }
            if !loot.cards.is_empty() {
                console.spawn(body(fonts, locale.loot_cards_hint(), palette::TEXT_DIM));
                console.spawn(row(18.0)).with_children(|cards| {
                    for (index, &card) in loot.cards.iter().enumerate() {
                        cards.spawn(column(10.0)).with_children(|offer| {
                            offer.spawn(command_card(assets, DeckCard::new(card), 1.2));
                            offer.spawn(message_button(
                                fonts,
                                locale.loot_take(),
                                Key::PRIMARY,
                                true,
                                RunAction::TakeLoot(Some(index)),
                            ));
                        });
                    }
                });
            }
            if !loot.modules.is_empty() {
                console.spawn(body(fonts, locale.loot_modules_hint(), palette::TEXT_DIM));
                console.spawn(row(14.0)).with_children(|plates| {
                    for (index, &module) in loot.modules.iter().enumerate() {
                        let (name, text) = locale.module(module);
                        plates
                            .spawn(plate(fonts, name, text))
                            .with_children(|plate| {
                                plate.spawn(message_button(
                                    fonts,
                                    locale.loot_take(),
                                    Key::PRIMARY,
                                    true,
                                    RunAction::TakeLoot(Some(index)),
                                ));
                            });
                    }
                });
            }
            if !loot.cards.is_empty() || !loot.modules.is_empty() {
                console.spawn(message_button(
                    fonts,
                    locale.loot_skip(),
                    Key::NORMAL,
                    true,
                    RunAction::TakeLoot(None),
                ));
            } else {
                console.spawn(message_button(
                    fonts,
                    locale.result_continue(),
                    Key::PRIMARY,
                    true,
                    RunAction::TakeLoot(None),
                ));
            }
        });
}

fn event(parent: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, run: &Run) {
    let Stage::Event(event) = run.stage else {
        return;
    };
    parent
        .spawn(node_console(fonts, locale.event_title(event)))
        .with_children(|console| {
            console.spawn(body(fonts, locale.event_text(event), palette::TEXT));
            console.spawn(column(10.0)).with_children(|options| {
                for (index, &option) in event.options().iter().enumerate() {
                    let affordable = option != EventOption::PayPilot || run.scrap >= PILOT_PRICE;
                    options
                        .spawn((
                            Node {
                                width: px(640),
                                align_items: AlignItems::Center,
                                column_gap: px(16),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|line| {
                            line.spawn(sized_message_button(
                                fonts,
                                locale.event_option(option),
                                Key::NORMAL,
                                Node {
                                    width: px(300),
                                    flex_shrink: 0.0,
                                    padding: UiRect::axes(px(16), px(9)),
                                    ..default()
                                },
                                affordable,
                                RunAction::Choose(index),
                            ));
                            line.spawn((
                                theme::text(
                                    fonts.body(15.0),
                                    locale.event_consequence(option, flicker_damage(run)),
                                    if affordable {
                                        palette::TEXT_DIM
                                    } else {
                                        palette::LOST
                                    },
                                ),
                                Node {
                                    flex_shrink: 1.0,
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        });
                }
            });
        });
}

fn event_outcome(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    outcome: crate::run::EventOutcome,
) {
    let mut lines = Vec::new();
    if outcome.scrap != 0 {
        lines.push(locale.run_scrap_change(outcome.scrap));
    }
    if outcome.hull != 0 {
        lines.push(locale.run_hull_change(outcome.hull));
    }
    match outcome.gain {
        Gain::Nothing => {}
        Gain::Module(module) => lines.push(locale.run_module_gained(locale.module(module).0)),
        Gain::Upgraded(card) => lines.push(
            locale.run_card_upgraded(
                &locale
                    .card(DeckCard {
                        id: card,
                        upgraded: true,
                    })
                    .name,
            ),
        ),
    }
    parent
        .spawn(node_console(fonts, locale.event_option(outcome.option)))
        .with_children(|console| {
            console.spawn(body(
                fonts,
                locale.event_outcome(outcome.option),
                palette::TEXT,
            ));
            for line in lines {
                console.spawn((
                    theme::caps(
                        fonts.display_bold(20.0),
                        &line,
                        theme::TRACKING,
                        palette::BUFF,
                    ),
                    Pickable::IGNORE,
                ));
            }
            console.spawn(message_button(
                fonts,
                locale.result_continue(),
                Key::PRIMARY,
                true,
                RunFlow::Dismiss,
            ));
        });
}

fn shipyard(parent: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, run: &Run) {
    let can_upgrade = run.deck.iter().any(|card| !card.upgraded);
    parent
        .spawn(node_console(fonts, locale.shipyard_title()))
        .with_children(|console| {
            console.spawn(body(fonts, locale.shipyard_hint(), palette::TEXT_DIM));
            console.spawn(row(14.0)).with_children(|plates| {
                plates
                    .spawn(plate(
                        fonts,
                        locale.shipyard_repair(),
                        &locale.shipyard_repair_text(run.shipyard_repair()),
                    ))
                    .with_children(|plate| {
                        plate.spawn(message_button(
                            fonts,
                            locale.shipyard_repair(),
                            Key::PRIMARY,
                            true,
                            RunAction::Repair,
                        ));
                    });
                plates
                    .spawn(plate(
                        fonts,
                        locale.shipyard_upgrade(),
                        locale.shipyard_upgrade_text(),
                    ))
                    .with_children(|plate| {
                        plate.spawn(resource_button(
                            fonts,
                            locale.shipyard_upgrade(),
                            Key::PRIMARY,
                            can_upgrade,
                            Gallery::open(Purpose::Upgrade),
                        ));
                    });
                plates
                    .spawn(plate(
                        fonts,
                        locale.shipyard_remove(),
                        locale.shipyard_remove_text(),
                    ))
                    .with_children(|plate| {
                        plate.spawn(resource_button(
                            fonts,
                            locale.shipyard_remove(),
                            Key::PRIMARY,
                            !run.deck.is_empty(),
                            Gallery::open(Purpose::Remove),
                        ));
                    });
            });
        });
}

fn market(parent: &mut ChildSpawnerCommands, assets: &CardAssets, run: &Run) {
    let Stage::Market(market) = &run.stage else {
        return;
    };
    let (fonts, locale) = (&*assets.fonts, &*assets.locale);
    let heading = |text: &str| {
        (
            theme::caps(
                fonts.display_bold(18.0),
                text,
                theme::TRACKING,
                theme::ACCENT,
            ),
            Pickable::IGNORE,
        )
    };
    parent
        .spawn(node_console(fonts, locale.market_title()))
        .with_children(|console| {
            console.spawn(body(
                fonts,
                format!("{} · {}", locale.market_hint(), locale.run_scrap(run.scrap)),
                palette::TEXT_DIM,
            ));
            console.spawn(row(28.0)).with_children(|sections| {
                sections.spawn(column(8.0)).with_children(|section| {
                    section.spawn(heading(locale.market_cards()));
                    section.spawn(row(12.0)).with_children(|cards| {
                        for (index, &(card, price)) in market.cards.iter().enumerate() {
                            cards.spawn(column(8.0)).with_children(|offer| {
                                offer.spawn(command_card(assets, DeckCard::new(card), 1.0));
                                offer.spawn(message_button(
                                    fonts,
                                    &locale.market_buy(price),
                                    Key::PRIMARY,
                                    run.scrap >= price,
                                    RunAction::BuyCard(index),
                                ));
                            });
                        }
                        if market.cards.is_empty() {
                            cards.spawn(body(fonts, locale.market_sold_out(), palette::TEXT_DIM));
                        }
                    });
                });
                sections.spawn(column(8.0)).with_children(|section| {
                    section.spawn(heading(locale.market_modules()));
                    section.spawn(row(12.0)).with_children(|plates| {
                        for (index, &(module, price)) in market.modules.iter().enumerate() {
                            let (name, text) = locale.module(module);
                            plates
                                .spawn(plate(fonts, name, text))
                                .with_children(|plate| {
                                    plate.spawn(message_button(
                                        fonts,
                                        &locale.market_buy(price),
                                        Key::PRIMARY,
                                        run.scrap >= price,
                                        RunAction::BuyModule(index),
                                    ));
                                });
                        }
                        if market.modules.is_empty() {
                            plates.spawn(body(fonts, locale.market_sold_out(), palette::TEXT_DIM));
                        }
                    });
                });
            });
            console.spawn(heading(locale.market_services()));
            console.spawn(row(12.0)).with_children(|services| {
                services.spawn(resource_button(
                    fonts,
                    &format!("{} · {REMOVAL_PRICE}", locale.market_removal()),
                    Key::NORMAL,
                    market.removal && run.scrap >= REMOVAL_PRICE && !run.deck.is_empty(),
                    Gallery::open(Purpose::BuyRemoval),
                ));
                services.spawn(message_button(
                    fonts,
                    &format!(
                        "{} · {MARKET_REPAIR_PRICE}",
                        locale.market_repair(MARKET_REPAIR)
                    ),
                    Key::NORMAL,
                    run.scrap >= MARKET_REPAIR_PRICE && run.hull < run.max_hull,
                    RunAction::BuyRepair,
                ));
                services.spawn(message_button(
                    fonts,
                    locale.market_leave(),
                    Key::PRIMARY,
                    true,
                    RunAction::LeaveMarket,
                ));
            });
        });
}

fn ended(parent: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, run: &Run) {
    let won = run.result == Some(BattleResult::Won);
    let color = if won { palette::WON } else { palette::LOST };
    let (title, text) = if won {
        (locale.run_won_title(), locale.run_won_text().to_owned())
    } else {
        (
            locale.result_title(BattleResult::Lost),
            locale.run_lost_text(run.sector, run.path.len()),
        )
    };
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(14),
                padding: UiRect::new(px(56), px(56), px(36), px(32)),
                border: UiRect::top(px(4)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(theme::PANEL),
            BorderColor::all(color),
            BoxShadow(vec![theme::glow(color.with_alpha(0.25), 80.0)]),
            PopIn::new(0.85, 0.7, EaseFunction::BackOut),
        ))
        .with_children(|panel| {
            panel.spawn((
                theme::caps(fonts.display_bold(72.0), title, 0.08, color),
                TextShadow {
                    offset: Vec2::ZERO,
                    color: color.with_alpha(0.6),
                },
                Pickable::IGNORE,
            ));
            panel.spawn(body(fonts, text, palette::TEXT));
            panel.spawn(message_button(
                fonts,
                locale.result_new_run(),
                Key::PRIMARY,
                true,
                RunFlow::NewRun,
            ));
        });
}

// --- Deckgalerie ---

/// Wählt eine Karte der Galerie zur Bestätigung.
fn pick_card(click: On<PointerClick>, cards: Query<&GalleryCard>, mut gallery: ResMut<Gallery>) {
    if click.button == PointerButton::Primary
        && let Ok(&GalleryCard(index)) = cards.get(click.entity)
    {
        gallery.picked = Some(index);
    }
}

/// Eine wählbare Karte der Galerie (Index im Deck).
#[derive(Component, Clone, Copy, Debug)]
struct GalleryCard(usize);

/// Baut die Galerie neu, wenn sie aufgeht, sich ändert oder der Run sich ändert; schließt sie mit ihrem Bildschirm.
fn show_gallery(
    mut commands: Commands,
    gallery: Option<Res<Gallery>>,
    run: Option<Res<ActiveRun>>,
    screen: Res<Screen>,
    assets: CardAssets,
    shown: Query<Entity, With<GalleryDialog>>,
) {
    if screen.is_changed() && !screen.is_added() && gallery.is_some() {
        commands.remove_resource::<Gallery>();
        return;
    }
    let changed = gallery.as_ref().is_none_or(|gallery| gallery.is_changed())
        || run.as_ref().is_some_and(|run| run.is_changed())
        || assets.locale.is_changed();
    if !changed {
        return;
    }
    for entity in &shown {
        commands.entity(entity).despawn();
    }
    let (Some(gallery), Some(run)) = (gallery, run) else {
        return;
    };
    let gallery = *gallery;
    let (fonts, locale) = (&*assets.fonts, &*assets.locale);
    let (title, hint) = match gallery.purpose {
        Purpose::View => (locale.gallery_deck(), None),
        Purpose::Upgrade => (
            locale.shipyard_upgrade(),
            Some(locale.gallery_upgrade_hint()),
        ),
        Purpose::Remove => (locale.shipyard_remove(), Some(locale.gallery_remove_hint())),
        Purpose::BuyRemoval => (locale.market_removal(), Some(locale.gallery_remove_hint())),
    };
    commands
        .spawn((
            Name::new("Deckgalerie"),
            GalleryDialog,
            Modal,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..theme::overlay()
            },
            GlobalZIndex(DIALOG_LAYER - 1),
        ))
        .with_children(|root| {
            root.spawn((
                theme::overlay(),
                BackgroundColor(palette::BACKGROUND.with_alpha(0.8)),
                FadeIn::new(0.2),
            ));
            root.spawn((
                theme::console(
                    Node {
                        max_width: px(1080),
                        max_height: percent(90),
                        align_items: AlignItems::Center,
                        row_gap: px(12),
                        ..default()
                    },
                    false,
                    Spawn(theme::console_header(fonts.display_bold(28.0), title)),
                ),
                PopIn::new(0.96, 0.32, EaseFunction::CubicOut),
            ))
            .with_children(|console| {
                match gallery
                    .picked
                    .and_then(|index| Some((index, *run.deck.get(index)?)))
                {
                    Some((index, card)) => {
                        console.spawn(row(24.0)).with_children(|pair| {
                            pair.spawn(command_card(&assets, card, 1.4));
                            if gallery.purpose == Purpose::Upgrade {
                                pair.spawn((
                                    theme::caps(fonts.display_bold(48.0), "→", 0.0, palette::BUFF),
                                    Node {
                                        align_self: AlignSelf::Center,
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                pair.spawn(command_card(
                                    &assets,
                                    DeckCard {
                                        upgraded: true,
                                        ..card
                                    },
                                    1.4,
                                ));
                            }
                        });
                        console.spawn(row(12.0)).with_children(|keys| {
                            if let Some(action) = gallery.action(index) {
                                keys.spawn((
                                    message_button(fonts, title, Key::PRIMARY, true, action),
                                    observe(|_: On<Activate>, mut commands: Commands| {
                                        commands.remove_resource::<Gallery>();
                                    }),
                                ));
                            }
                            keys.spawn(resource_button(
                                fonts,
                                locale.gallery_back(),
                                Key::NORMAL,
                                true,
                                Gallery::open(gallery.purpose),
                            ));
                        });
                    }
                    None => {
                        if let Some(hint) = hint {
                            console.spawn(body(fonts, hint, palette::TEXT_DIM));
                        }
                        console
                            .spawn((
                                Node {
                                    max_width: px(1020),
                                    flex_wrap: FlexWrap::Wrap,
                                    justify_content: JustifyContent::Center,
                                    column_gap: px(10),
                                    row_gap: px(10),
                                    padding: UiRect::all(px(6)),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                ScrollArea,
                            ))
                            .with_children(|grid| {
                                for (index, &card) in run.deck.iter().enumerate() {
                                    let mut entry = grid.spawn(command_card(&assets, card, 0.95));
                                    if gallery.selectable(card) {
                                        entry.insert((
                                            GalleryCard(index),
                                            CardHover::Raise,
                                            observe(pick_card),
                                        ));
                                    } else if gallery.purpose != Purpose::View {
                                        entry.insert(UiOpacity(0.35));
                                    }
                                }
                            });
                        console.spawn((
                            button::text_button(
                                fonts.display_bold(18.0),
                                locale.dialog_close(),
                                Key::NORMAL,
                                key_node(),
                            ),
                            observe(|_: On<Activate>, mut commands: Commands| {
                                commands.remove_resource::<Gallery>();
                            }),
                        ));
                    }
                }
            });
        });
}

// --- Run abbrechen ---

fn show_abandon(
    mut commands: Commands,
    prompt: Option<Res<AbandonPrompt>>,
    run: Option<Res<ActiveRun>>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    shown: Query<Entity, With<AbandonDialog>>,
) {
    let open = prompt.is_some() && run.is_some();
    if open == !shown.is_empty() && !locale.is_changed() {
        return;
    }
    for entity in &shown {
        commands.entity(entity).despawn();
    }
    if !open {
        commands.remove_resource::<AbandonPrompt>();
        return;
    }
    let close = |_: On<Activate>, mut commands: Commands| {
        commands.remove_resource::<AbandonPrompt>();
    };
    commands
        .spawn((
            Name::new("Run abbrechen"),
            AbandonDialog,
            Modal,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..theme::overlay()
            },
            GlobalZIndex(DIALOG_LAYER + 1),
        ))
        .with_children(|root| {
            root.spawn((
                theme::overlay(),
                BackgroundColor(palette::BACKGROUND.with_alpha(0.75)),
                FadeIn::new(0.2),
            ));
            root.spawn((
                theme::console(
                    Node {
                        width: px(480),
                        align_items: AlignItems::Center,
                        row_gap: px(16),
                        ..default()
                    },
                    false,
                    Spawn(theme::console_header(
                        fonts.display_bold(28.0),
                        locale.run_abandon_title(),
                    )),
                ),
                PopIn::new(0.96, 0.32, EaseFunction::CubicOut),
            ))
            .with_children(|console| {
                console.spawn(body(&fonts, locale.run_abandon_text(), palette::TEXT));
                console.spawn(row(12.0)).with_children(|keys| {
                    keys.spawn((
                        message_button(
                            &fonts,
                            locale.run_abandon_confirm(),
                            Key::DANGER,
                            true,
                            RunFlow::Abandon,
                        ),
                        observe(close),
                    ));
                    keys.spawn((
                        button::text_button(
                            fonts.display_bold(18.0),
                            locale.run_abandon_back(),
                            Key::PRIMARY,
                            key_node(),
                        ),
                        observe(close),
                    ));
                });
            });
        });
}

/// Die Module des Runs als Tooltip-Zeilen „Name: Text“.
pub fn modules_tooltip(locale: &Locale, modules: &[ModuleId]) -> String {
    if modules.is_empty() {
        return locale.run_no_modules().to_owned();
    }
    modules
        .iter()
        .map(|&module| {
            let (name, text) = locale.module(module);
            format!("{name}: {text}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) struct RunNodesPlugin;

impl Plugin for RunNodesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (show, show_gallery, show_abandon, apply_enabled)
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::cards::CardId;

    use super::*;

    #[test]
    fn gallery_choices() {
        let plain = DeckCard::new(CardId::Torpedo);
        let upgraded = DeckCard {
            upgraded: true,
            ..plain
        };
        let upgrade = Gallery::open(Purpose::Upgrade);
        assert!(upgrade.selectable(plain) && !upgrade.selectable(upgraded));
        assert!(!Gallery::open(Purpose::View).selectable(plain));
        assert_eq!(upgrade.action(2), Some(RunAction::Upgrade(2)));
        assert_eq!(
            Gallery::open(Purpose::BuyRemoval).action(0),
            Some(RunAction::BuyRemoval(0))
        );
        assert_eq!(Gallery::open(Purpose::View).action(0), None);
    }
}
