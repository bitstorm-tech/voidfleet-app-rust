//! Spielregeln (Port von `ui/rulesDialog.ts`): Das Fragezeichen in der Icon-Leiste öffnet einen großen Dialog mit
//! Kapitelnavigation links und dem scrollenden Regeltext rechts; die Navigation markiert das Kapitel oben im
//! sichtbaren Bereich, ein Klick scrollt sanft dorthin (Mausrad, Pfeiltasten, Bild auf/ab, Pos1 und Ende scrollen
//! auch). Die Texte kommen aus [`Messages::rules_section`](crate::i18n::Messages::rules_section); jedes Kapitel bekommt
//! seine Illustration ([`rules_figures`](super::rules_figures)), die Screenshots aus `images/rules/`, die Admirale mit
//! Flaggschiff und Fähigkeitskarte oder die Galerie aller Karten samt Flaggschiffen und Tokens.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::LineHeight,
    ui_widgets::{Activate, Button, ScrollArea, observe},
};
use voidfleet_core::{
    admirals::{ADMIRAL_IDS, AdmiralId},
    cards::{COMMAND_ORDER, COMMANDS, CommandId, FLAGSHIPS, SHIPS, ShipId, TOKENS},
};

use super::{Dialog, dialog_console, footer_close, icon_dialog, rules_figures};
use crate::{
    assets::{AppState, Preload},
    audio::{PlayUiSound, UiSound},
    data::admirals::flagship_admiral,
    i18n::RulesChapter,
    palette,
    ui::{
        card::{ART_ASPECT, CardAssets, command_card, power_card, ship_card},
        theme::{self, rgba},
    },
};

/// Bilder des Regel-Dialogs: Screenshots und die gerasterten Figuren.
#[derive(Resource)]
pub(super) struct RulesImages {
    pub deploy: Handle<Image>,
    pub maneuver: Handle<Image>,
    pub board: Handle<Image>,
    pub fire_orders: Handle<Image>,
    pub turn_maneuver: Handle<Image>,
    pub turn_fire: Handle<Image>,
}

fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let mut image = |name: &str| preload.track(server.load(format!("images/rules/{name}")));
    commands.insert_resource(RulesImages {
        deploy: image("deploy.jpg"),
        maneuver: image("maneuver.jpg"),
        board: image("board.png"),
        fire_orders: image("fire_orders.png"),
        turn_maneuver: image("turn_maneuver.png"),
        turn_fire: image("turn_fire.png"),
    });
}

/// Eine Illustration.
#[derive(Clone, Copy, Debug)]
enum Figure {
    Overdrive,
    Turn,
    DeployScreenshot,
    Admirals,
    Board,
    ManeuverScreenshot,
    Phases,
    FireOrders,
    Ships,
    Commands,
}

/// Kapitel in Lesereihenfolge mit ihren Illustrationen; die stehen vor dem Absatz `after` (`None`: nach dem letzten).
const CHAPTERS: [(RulesChapter, &[Figure], Option<usize>); 12] = [
    (RulesChapter::Goal, &[Figure::Overdrive], None),
    (
        RulesChapter::Turn,
        &[Figure::Turn, Figure::DeployScreenshot],
        Some(2),
    ),
    (RulesChapter::Admirals, &[], None),
    (RulesChapter::Roster, &[Figure::Admirals], None),
    (RulesChapter::Board, &[Figure::Board], Some(2)),
    (
        RulesChapter::Movement,
        &[Figure::ManeuverScreenshot],
        Some(1),
    ),
    (RulesChapter::Resolution, &[Figure::Phases], None),
    (RulesChapter::Combat, &[Figure::FireOrders], Some(2)),
    (RulesChapter::Ships, &[Figure::Ships], Some(1)),
    (RulesChapter::Commands, &[Figure::Commands], None),
    (RulesChapter::Hand, &[], None),
    (RulesChapter::Controls, &[], None),
];

/// Der scrollende Regeltext; `target` ist, wohin er gerade sanft scrollt.
#[derive(Component, Default)]
struct RulesBody {
    target: Option<f32>,
}

/// Ein Kapitel im Regeltext.
#[derive(Component, Clone, Copy)]
struct ChapterSection(RulesChapter);

/// Ein Eintrag der Kapitelnavigation.
#[derive(Component, Clone, Copy)]
pub(in crate::ui) struct ChapterLink(pub RulesChapter);

/// Das Kapitel oben im sichtbaren Bereich.
#[derive(Component)]
struct ActiveLink;

/// Die Lampe vor einem Kapitel der Navigation.
#[derive(Component)]
struct LinkLamp;

/// Innenabstand des Regeltexts oben (samt Rand); so weit bleibt ein angesprungenes Kapitel unter der Kante.
const BODY_INSET: f32 = 19.0;

/// Ein eingelassenes Feld wie Navigation und Regeltext (`.rules-nav`, `.rules-body`).
fn well() -> impl Bundle {
    (
        theme::vertical_gradient(&[(rgba(9, 11, 16, 0.85), 0.0), (rgba(14, 17, 23, 0.7), 100.0)]),
        BorderColor::all(Color::BLACK.with_alpha(0.7)),
        BoxShadow(vec![theme::drop(
            Color::WHITE.with_alpha(0.08),
            0.0,
            1.0,
            0.0,
        )]),
    )
}

/// Fläche der Einträge in Galerien und Phasen.
const TILE: Color = rgba(255, 255, 255, 0.03);

pub(super) fn spawn(mut commands: Commands, assets: CardAssets, images: Res<RulesImages>) {
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let body = commands
        .spawn((
            Name::new("Regeltext"),
            RulesBody::default(),
            ScrollArea,
            Node {
                flex_grow: 1.0,
                min_width: Val::ZERO,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(20), px(18)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            well(),
        ))
        .id();
    for (index, &(chapter, figures, after)) in CHAPTERS.iter().enumerate() {
        let text = locale.rules_section(chapter);
        let first = index == 0;
        let section = commands
            .spawn((
                ChapterSection(chapter),
                Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::top(px(if first { 0.0 } else { 26.0 })),
                    padding: UiRect::top(px(if first { 0.0 } else { 22.0 })),
                    border: UiRect::top(px(if first { 0.0 } else { 1.0 })),
                    ..default()
                },
                BorderColor::all(Color::WHITE.with_alpha(0.08)),
                ChildOf(body),
            ))
            .id();
        commands.spawn((
            Node {
                margin: UiRect::bottom(px(10)),
                ..default()
            },
            theme::caps(
                fonts.display_bold(24.0),
                text.heading,
                theme::TRACKING,
                palette::TEXT,
            ),
            ChildOf(section),
        ));
        let figure_at = after.unwrap_or(text.paragraphs.len());
        for (index, paragraph) in text.paragraphs.iter().enumerate() {
            if index == figure_at {
                spawn_figures(
                    &mut commands,
                    section,
                    figures,
                    text.caption,
                    &assets,
                    &images,
                );
            }
            commands.spawn((
                Node {
                    margin: UiRect::bottom(px(12)),
                    ..default()
                },
                theme::text(fonts.body(16.0), paragraph.as_ref(), palette::TEXT_DIM),
                LineHeight::RelativeToFont(1.55),
                ChildOf(section),
            ));
        }
        if figure_at >= text.paragraphs.len() {
            spawn_figures(
                &mut commands,
                section,
                figures,
                text.caption,
                &assets,
                &images,
            );
        }
    }
    let links: Vec<_> = CHAPTERS
        .iter()
        .map(|&(chapter, ..)| {
            (
                ChapterLink(chapter),
                Button,
                Hovered::default(),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    padding: UiRect::axes(px(8), px(7)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                observe(jump_to_chapter),
                children![
                    (
                        LinkLamp,
                        Node {
                            width: px(4),
                            height: px(16),
                            flex_shrink: 0.0,
                            border_radius: BorderRadius::all(px(2)),
                            ..default()
                        },
                        BackgroundColor(theme::rgb(6, 8, 11)),
                        BoxShadow::default(),
                        Pickable::IGNORE,
                    ),
                    (
                        theme::caps(
                            fonts.display_bold(16.0),
                            locale.rules_section(chapter).heading,
                            theme::TRACKING,
                            palette::TEXT_DIM
                        ),
                        Pickable::IGNORE,
                    ),
                ],
            )
        })
        .collect();
    commands.spawn(icon_dialog(
        Dialog::Rules,
        dialog_console(
            fonts,
            locale.rules_title(),
            Node {
                width: Val::Vw(94.0),
                max_width: px(1080),
                height: Val::Vh(92.0),
                max_height: px(900),
                ..default()
            },
            true,
            (
                Spawn((
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::ZERO,
                        column_gap: px(20),
                        margin: UiRect::bottom(px(14)),
                        ..default()
                    },
                    Children::spawn((
                        Spawn((
                            Node {
                                width: px(230),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                row_gap: px(2),
                                padding: UiRect::axes(px(6), px(8)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(8)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollArea,
                            well(),
                            Children::spawn(SpawnIter(links.into_iter())),
                        )),
                        WithOneRelated(body),
                    )),
                )),
                Spawn(footer_close(fonts, locale, 4.0)),
            ),
        ),
    ));
}

/// Hängt die Illustrationen eines Kapitels an; die Bildunterschrift steht unter der letzten.
fn spawn_figures(
    commands: &mut Commands,
    section: Entity,
    figures: &[Figure],
    caption: Option<&str>,
    assets: &CardAssets,
    images: &RulesImages,
) {
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    for (index, &figure) in figures.iter().enumerate() {
        let wrapper = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::new(Val::ZERO, Val::ZERO, px(6), px(16)),
                    ..default()
                },
                ChildOf(section),
            ))
            .id();
        let mut content = match figure {
            Figure::Overdrive => commands.spawn(rules_figures::overdrive(fonts, locale)),
            Figure::Turn => commands.spawn(rules_figures::turn(assets, images)),
            Figure::DeployScreenshot => commands.spawn(screenshot(images.deploy.clone())),
            Figure::Admirals => commands.spawn(admiral_gallery(assets)),
            Figure::Board => commands.spawn(rules_figures::board(fonts, locale, images)),
            Figure::ManeuverScreenshot => commands.spawn(screenshot(images.maneuver.clone())),
            Figure::Phases => commands.spawn(rules_figures::phases(fonts, locale)),
            Figure::FireOrders => commands.spawn(rules_figures::fire_orders(fonts, locale, images)),
            Figure::Ships => commands.spawn(ship_gallery(assets)),
            Figure::Commands => commands.spawn(command_gallery(assets)),
        };
        content.insert(ChildOf(wrapper));
        if let Some(caption) = caption.filter(|_| index == figures.len() - 1) {
            commands.spawn((
                Node {
                    margin: UiRect::top(px(6)),
                    ..default()
                },
                theme::text(fonts.body(14.0), caption, rgba(199, 219, 250, 0.7)),
                LineHeight::RelativeToFont(1.4),
                ChildOf(wrapper),
            ));
        }
    }
}

/// Ein Screenshot vom Brett (1080 × 515), so breit wie der Text.
fn screenshot(image: Handle<Image>) -> impl Bundle {
    (
        Node {
            width: percent(100),
            aspect_ratio: Some(1080.0 / 515.0),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BorderColor::all(Color::WHITE.with_alpha(0.08)),
        ImageNode::new(image),
    )
}

/// Ein Raster, das so viele Spalten von mindestens `min` Pixeln bildet, wie hineinpassen.
fn auto_grid(min: f32, gap: f32) -> Node {
    Node {
        display: Display::Grid,
        grid_template_columns: vec![RepeatedGridTrack::minmax(
            GridTrackRepetition::AutoFill,
            MinTrackSizingFunction::Px(min),
            MaxTrackSizingFunction::Fraction(1.0),
        )],
        row_gap: px(gap),
        column_gap: px(gap),
        ..default()
    }
}

/// Je Admiral Porträt, Name und Beiname, sein Flaggschiff und die Karte seiner Fähigkeit.
fn admiral_gallery(assets: &CardAssets) -> impl Bundle {
    let fonts = &assets.fonts;
    let entries: Vec<_> = ADMIRAL_IDS
        .into_iter()
        .map(|admiral: AdmiralId| {
            let color = palette::admiral_color(admiral);
            let text = assets.locale.admiral(admiral);
            (
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![GridTrack::auto(), GridTrack::fr(1.0)],
                    align_items: AlignItems::Center,
                    row_gap: px(10),
                    column_gap: px(12),
                    padding: UiRect::all(px(12)),
                    border: UiRect::left(px(3)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(TILE),
                BorderColor::all(color),
                children![
                    (
                        Node {
                            width: px(52),
                            height: px(52),
                            overflow: Overflow::clip(),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BoxShadow(vec![theme::ring(color, 2.0)]),
                        children![theme::cover_image(
                            assets.portraits.get(admiral),
                            ART_ASPECT,
                            0.5
                        )],
                    ),
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        children![
                            theme::text(fonts.body(17.0), text.name, palette::TEXT),
                            theme::text(fonts.body(15.0), text.epithet, color),
                        ],
                    ),
                    (
                        Node {
                            grid_column: GridPlacement::span(2),
                            align_items: AlignItems::FlexEnd,
                            column_gap: px(10),
                            ..default()
                        },
                        children![
                            ship_card(assets, admiral.flagship(), 1.0),
                            power_card(assets, admiral, 1.0),
                        ],
                    ),
                ],
            )
        })
        .collect();
    (
        auto_grid(300.0, 14.0),
        Children::spawn(SpawnIter(entries.into_iter())),
    )
}

/// Eine Karte in Originalgröße, daneben Name, Zusatz, Kopien im Deck und Zeilen wie Werte und Effekt.
fn gallery_entry(
    assets: &CardAssets,
    card: impl Bundle,
    name: &str,
    meta: String,
    lines: Vec<String>,
) -> impl Bundle {
    let fonts = &assets.fonts;
    let lines: Vec<_> = lines
        .into_iter()
        .map(|line| {
            (
                theme::text(fonts.body(14.0), line, palette::TEXT_DIM),
                LineHeight::RelativeToFont(1.45),
            )
        })
        .collect();
    (
        Node {
            column_gap: px(14),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(TILE),
        children![
            card,
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 1.0,
                    min_width: Val::ZERO,
                    row_gap: px(5),
                    ..default()
                },
                Children::spawn((
                    Spawn(theme::text(fonts.body(16.0), name, palette::TEXT)),
                    Spawn(theme::caps(fonts.display_bold(12.5), &meta, 0.04, NOTE)),
                    SpawnIter(lines.into_iter()),
                )),
            ),
        ],
    )
}

/// Gedämpfte Zusatzzeile (`.meta`).
const NOTE: Color = rgba(199, 219, 250, 0.6);

/// Zusatz einer Karte samt Kopien im Deck.
fn meta(parts: impl IntoIterator<Item = String>, copies: usize, assets: &CardAssets) -> String {
    parts
        .into_iter()
        .chain((copies > 0).then(|| assets.locale.rules_copies(copies)))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Alle Schiffe als Karte mit Klasse, Werten und Effekt, danach die Tokens wie die Drohne und die Flaggschiffe.
fn ship_gallery(assets: &CardAssets) -> impl Bundle {
    let locale = &assets.locale;
    let entries: Vec<_> = SHIPS
        .into_iter()
        .chain(TOKENS)
        .chain(FLAGSHIPS)
        .map(|ship: ShipId| {
            let card = ship.card();
            let admiral = flagship_admiral(ship);
            let origin = match admiral {
                Some(admiral) => Some(locale.rules_flagship(&locale.admiral_title(admiral))),
                None => ship.is_token().then(|| locale.rules_token().to_owned()),
            };
            let mut stats = format!(
                "{} {} · {} {}",
                locale.stats_hp(),
                card.hp,
                locale.stats_atk(),
                card.atk
            );
            if card.shield > 0 {
                stats.push_str(&format!(" · {} {}", locale.stats_shield(), card.shield));
            }
            let effect = locale.ship(ship).effect;
            let effect = match admiral {
                Some(admiral) => format!("{}: {effect}", locale.admiral(admiral).ability),
                None => effect.into_owned(),
            };
            gallery_entry(
                assets,
                ship_card(assets, ship, 1.0),
                locale.ship(ship).name,
                meta(
                    std::iter::once(locale.ship_class(card.class).to_owned()).chain(origin),
                    ship.deck_copies(),
                    assets,
                ),
                vec![stats, effect],
            )
        })
        .collect();
    (
        auto_grid(330.0, 14.0),
        Children::spawn(SpawnIter(entries.into_iter())),
    )
}

/// Alle Befehle, nach Typ in der Reihenfolge der Auflösung gruppiert.
fn command_gallery(assets: &CardAssets) -> impl Bundle {
    let locale = &assets.locale;
    let groups: Vec<_> = COMMAND_ORDER
        .into_iter()
        .enumerate()
        .map(|(index, command_type)| {
            let entries: Vec<_> = COMMANDS
                .into_iter()
                .filter(|command| command.command_type() == command_type)
                .map(|command: CommandId| {
                    let text = locale.command(command);
                    let target = text
                        .target
                        .unwrap_or_else(|| locale.target_mode(command.target_mode()));
                    gallery_entry(
                        assets,
                        command_card(assets, command, 1.0),
                        text.name,
                        meta([target.to_owned()], command.deck_copies(), assets),
                        vec![text.effect.into_owned()],
                    )
                })
                .collect();
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (
                        Node {
                            margin: UiRect::new(Val::ZERO, Val::ZERO, px(18), px(8)),
                            ..default()
                        },
                        theme::text(
                            assets.fonts.body(16.0),
                            format!("{}. {}", index + 1, locale.command_type(command_type)),
                            palette::TEXT,
                        ),
                    ),
                    (
                        auto_grid(330.0, 14.0),
                        Children::spawn(SpawnIter(entries.into_iter())),
                    ),
                ],
            )
        })
        .collect();
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Children::spawn(SpawnIter(groups.into_iter())),
    )
}

/// Ein Eintrag der Kapitelnavigation scrollt sanft zu seinem Kapitel.
fn jump_to_chapter(
    activate: On<Activate>,
    links: Query<&ChapterLink>,
    sections: Query<(&ChapterSection, &UiGlobalTransform, &ComputedNode)>,
    mut body: Single<(
        &mut RulesBody,
        &ScrollPosition,
        &UiGlobalTransform,
        &ComputedNode,
    )>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let Ok(&ChapterLink(chapter)) = links.get(activate.entity) else {
        return;
    };
    sounds.write(PlayUiSound(UiSound::Click));
    let (rules, position, body_transform, body_node) = &mut *body;
    let top = |transform: &UiGlobalTransform, node: &ComputedNode| {
        transform.translation.y - node.size().y / 2.0
    };
    if let Some((_, transform, node)) = sections.iter().find(|(section, ..)| section.0 == chapter) {
        let offset = (top(transform, node) - top(body_transform, body_node))
            * body_node.inverse_scale_factor();
        rules.target = Some(position.y + offset - BODY_INSET);
    }
}

/// Wie schnell der Text zum Ziel scrollt (1/s).
const SCROLL_FOLLOW: f32 = 12.0;

/// Scrollt sanft zum Ziel; Pfeiltasten, Bild auf/ab, Pos1 und Ende setzen ein neues.
fn scroll_body(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut bodies: Query<(&mut RulesBody, &mut ScrollPosition, &ComputedNode)>,
) {
    for (mut rules, mut position, node) in &mut bodies {
        let scale = node.inverse_scale_factor();
        let max = ((node.content_size().y - node.size().y) * scale).max(0.0);
        let page = node.size().y * scale * 0.9;
        let from = rules.target.unwrap_or(position.y);
        let key_target = [
            (KeyCode::ArrowDown, from + 40.0),
            (KeyCode::ArrowUp, from - 40.0),
            (KeyCode::PageDown, from + page),
            (KeyCode::PageUp, from - page),
            (KeyCode::End, max),
            (KeyCode::Home, 0.0),
        ]
        .into_iter()
        .find_map(|(key, target)| keys.just_pressed(key).then_some(target));
        if let Some(target) = key_target {
            rules.target = Some(target);
        }
        let Some(target) = rules.target else {
            continue;
        };
        let target = target.clamp(0.0, max);
        let follow = 1.0 - (-SCROLL_FOLLOW * time.delta_secs()).exp();
        let next = position.y.lerp(target, follow);
        if (next - target).abs() < 0.5 {
            position.y = target;
            rules.target = None;
        } else {
            position.y = next;
        }
    }
}

/// Markiert das Kapitel, das oben im sichtbaren Bereich steht.
fn mark_active(
    mut commands: Commands,
    body: Single<(&UiGlobalTransform, &ComputedNode), With<RulesBody>>,
    sections: Query<(&ChapterSection, &UiGlobalTransform, &ComputedNode)>,
    links: Query<(Entity, &ChapterLink, Has<ActiveLink>)>,
) {
    let (body_transform, body_node) = *body;
    let line = body_transform.translation.y - body_node.size().y / 2.0
        + 40.0 / body_node.inverse_scale_factor();
    let active = sections
        .iter()
        .filter(|(_, transform, node)| transform.translation.y - node.size().y / 2.0 <= line)
        .map(|(section, ..)| section.0)
        .last()
        .unwrap_or(CHAPTERS[0].0);
    for (entity, link, marked) in &links {
        let wanted = link.0 == active;
        if wanted && !marked {
            commands.entity(entity).insert(ActiveLink);
        } else if !wanted && marked {
            commands.entity(entity).remove::<ActiveLink>();
        }
    }
}

/// Die Lampen der Navigation.
type LinkLamps<'w, 's> = Query<
    'w,
    's,
    (&'static mut BackgroundColor, &'static mut BoxShadow),
    (With<LinkLamp>, Without<ChapterLink>),
>;

/// Das aktive Kapitel hat eine leuchtende Lampe und hellen Grund, beim Überfahren hellt der Eintrag auf.
fn style_links(
    mut links: Query<
        (&Hovered, Has<ActiveLink>, &Children, &mut BackgroundColor),
        With<ChapterLink>,
    >,
    mut lamps: LinkLamps,
    mut labels: Query<&mut TextColor>,
) {
    for (hovered, active, children, mut background) in &mut links {
        let hovered = hovered.get();
        background.set_if_neq(BackgroundColor(if active {
            palette::PLAYER.with_alpha(0.1)
        } else if hovered {
            Color::WHITE.with_alpha(0.04)
        } else {
            Color::NONE
        }));
        for &child in children {
            if let Ok((mut lamp, mut glow)) = lamps.get_mut(child) {
                lamp.set_if_neq(BackgroundColor(if active {
                    palette::SELECT
                } else {
                    theme::rgb(6, 8, 11)
                }));
                let wanted = if active {
                    vec![theme::glow(palette::SELECT, 8.0)]
                } else {
                    Vec::new()
                };
                if glow.0 != wanted {
                    glow.0 = wanted;
                }
            } else if let Ok(mut label) = labels.get_mut(child) {
                label.set_if_neq(TextColor(if active || hovered {
                    palette::TEXT
                } else {
                    palette::TEXT_DIM
                }));
            }
        }
    }
}

pub(super) struct RulesPlugin;

impl Plugin for RulesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load).add_systems(
            Update,
            (
                scroll_body,
                mark_active.run_if(any_with_component::<RulesBody>),
                style_links,
                rules_figures::scale_labels,
            )
                .run_if(in_state(AppState::Match)),
        );
    }
}
