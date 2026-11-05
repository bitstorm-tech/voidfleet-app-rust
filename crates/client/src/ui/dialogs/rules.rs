//! Spielregeln (Port von `ui/rulesDialog.ts`): Das Fragezeichen in der Icon-Leiste öffnet einen großen Dialog mit
//! Kapitelnavigation links und dem scrollenden Regeltext rechts; die Navigation markiert das Kapitel oben im
//! sichtbaren Bereich, ein Klick scrollt sanft dorthin (Mausrad, Pfeiltasten, Bild auf/ab, Pos1 und Ende scrollen
//! auch). Die Texte kommen aus [`Messages::rules_section`](crate::i18n::Messages::rules_section): Brett, Runde,
//! Stationen, Schaden, Zustände, Admirale, Karten, Gegner und Steuerung.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::LineHeight,
    ui_widgets::{Activate, Button, ScrollArea, observe},
};

use super::{Dialog, dialog_console, footer_close, icon_dialog};
use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    i18n::RulesChapter,
    palette,
    ui::{
        card::CardAssets,
        theme::{self, rgba},
    },
};

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

pub(super) fn spawn(mut commands: Commands, assets: CardAssets) {
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
    for (index, chapter) in RulesChapter::ALL.into_iter().enumerate() {
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
        for paragraph in &text.paragraphs {
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
    }
    let links: Vec<_> = RulesChapter::ALL
        .into_iter()
        .map(|chapter| {
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
                width: vw(94),
                max_width: px(1080),
                height: vh(92),
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
        let mut next = position.y;
        next.smooth_nudge(&target, SCROLL_FOLLOW, time.delta_secs());
        if (next - target).abs() < 0.5 {
            position.y = target;
            rules.target = None;
        } else {
            position.y = next;
        }
    }
}

/// Ein Kapitel ist gewandert oder neu: Es wurde gescrollt, das Fenster hat sich geändert, oder der Dialog entsteht.
type SectionMoved = (
    With<ChapterSection>,
    Or<(Changed<UiGlobalTransform>, Changed<ComputedNode>)>,
);

/// Markiert das Kapitel, das oben im sichtbaren Bereich steht; läuft nur, wenn ein Kapitel gewandert ist (die Lage im
/// Regeltext folgt aus der Lage der Kapitel).
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
        .unwrap_or(RulesChapter::ALL[0]);
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
        app.add_systems(
            Update,
            (
                scroll_body,
                mark_active.run_if(any_match_filter::<SectionMoved>),
                style_links,
            )
                .run_if(in_state(AppState::Match)),
        );
    }
}
