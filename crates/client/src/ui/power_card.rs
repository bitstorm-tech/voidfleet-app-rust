//! Die Admiralsfähigkeit links neben den Befehlen (Port von `ui/powerCard.ts`): eine Karte mit dem Porträt, dem
//! Namen, der Zielangabe und dem Kurztext in der Farbe des Admirals ([`super::card::power_card`]). Ein Klick meldet
//! [`PowerPressed`]. Bei Admiral Kerr steht darüber der Knopf der Zielerfassung ([`MarkPressed`]), in der Runde, in
//! der die Railgun feuert, stattdessen ihr Ziel — mit einer Warnung, wenn die Pendulum geplant bewegt wird.
//!
//! Zustände der Fähigkeit und der Zielerfassung ([`PowerState`]) folgen aus [`Match`] und [`HudState`].

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::LineHeight,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::{
    admirals::{ABILITY_VALUES, AdmiralId},
    board::{Side, SlotRef},
};

use super::{
    Fonts, HudState, MarkPressed, PowerPressed, Selected, UiIcons,
    button::{self, Dimmed, Key, Lit, Pulsing},
    card::{CARD_HEIGHT, CARD_WIDTH, CardAssets, HAND_ZOOM, Marked, power_card},
    theme,
    tooltip::Tooltip,
};
use crate::{assets::AppState, game::Match, i18n::Locale, palette};

/// Stand der Fähigkeit bzw. der Zielerfassung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PowerState {
    /// Möglich.
    Open,
    /// Ihr Ziel wird gerade gewählt.
    Selecting,
    Planned,
    /// Gerade nicht möglich.
    Blocked,
}

impl PowerState {
    fn of(planned: bool, selecting: bool, possible: bool) -> Self {
        if planned {
            Self::Planned
        } else if selecting {
            Self::Selecting
        } else if possible {
            Self::Open
        } else {
            Self::Blocked
        }
    }
}

/// Die Spalte über der Hand mit Zielerfassung bzw. Railgun und der Fähigkeitskarte; die Hand rechnet mit ihrer Breite.
#[derive(Component)]
pub(super) struct AdmiralArea;

/// Die Fähigkeitskarte darin, mit dem Admiral, für den sie gebaut ist.
#[derive(Component)]
struct PowerCardOf(AdmiralId);

/// Abdunklung der Karte, wenn die Fähigkeit nicht möglich ist (CSS `filter: saturate(0.4) brightness(0.7)`).
#[derive(Component)]
struct BlockedShade;

#[derive(Component)]
struct LockButton;

#[derive(Component)]
struct RailgunStatus;

/// Beschriftung und Warnung von Zielerfassung bzw. Railgun.
#[derive(Component)]
struct LockLabel;

#[derive(Component)]
struct LockWarning;

/// Zeile mit Fadenkreuz, Beschriftung und Warnung (`.lock-button`, `.railgun-status`).
fn lock_content(fonts: &Fonts, icons: &UiIcons) -> impl Bundle {
    let s = |value: f32| px(value * HAND_ZOOM);
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: s(7.0),
            width: percent(100),
            ..default()
        },
        Pickable::IGNORE,
        children![
            button::icon_image(icons.crosshair.clone(), 18.0 * HAND_ZOOM),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        LockLabel,
                        theme::text(fonts.body(12.5 * HAND_ZOOM), "", palette::TEXT),
                        Pickable::IGNORE,
                    ),
                    (
                        LockWarning,
                        theme::text(fonts.body(10.5 * HAND_ZOOM), "", palette::DAMAGE),
                        LineHeight::RelativeToFont(1.2),
                        Pickable::IGNORE,
                    ),
                ],
            ),
        ],
    )
}

fn lock_node() -> Node {
    let s = |value: f32| px(value * HAND_ZOOM);
    Node {
        display: Display::None,
        flex_shrink: 0.0,
        min_height: s(38.0),
        padding: UiRect::axes(s(8.0), s(4.0)),
        justify_content: JustifyContent::FlexStart,
        border_radius: BorderRadius::all(s(4.0)),
        ..default()
    }
}

/// Die Spalte, noch ohne Karte; sie entsteht, sobald der Admiral feststeht.
pub(super) fn admiral_area(fonts: &Fonts, icons: &UiIcons, locale: &Locale) -> impl Bundle {
    (
        AdmiralArea,
        Node {
            display: Display::None,
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            row_gap: px(8.0 * HAND_ZOOM),
            width: px(CARD_WIDTH * HAND_ZOOM),
            height: px(CARD_HEIGHT * HAND_ZOOM),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                LockButton,
                button::button(
                    Key::tinted(palette::admiral_color(AdmiralId::Kerr)),
                    lock_node()
                ),
                Tooltip(locale.hud_mark_hint().into()),
                observe(|_: On<Activate>, mut pressed: MessageWriter<MarkPressed>| {
                    pressed.write(MarkPressed);
                }),
                children![lock_content(fonts, icons)],
            ),
            (
                RailgunStatus,
                Node {
                    border: UiRect::all(px(1)),
                    ..lock_node()
                },
                BackgroundColor::default(),
                BorderColor::default(),
                Tooltip(locale.hud_railgun_hint(ABILITY_VALUES.railgun)),
                children![lock_content(fonts, icons)],
            ),
        ],
    )
}

/// Baut die Fähigkeitskarte, sobald (bzw. wenn ein anderer) Admiral feststeht, und blendet die Spalte ohne ihn aus.
fn build_card(
    mut commands: Commands,
    game: Res<Match>,
    assets: CardAssets,
    area: Single<(Entity, &mut Node), With<AdmiralArea>>,
    cards: Query<(Entity, &PowerCardOf)>,
    mut lock: Single<&mut Key, With<LockButton>>,
    railgun: Single<(&mut BackgroundColor, &mut BorderColor), With<RailgunStatus>>,
) {
    let (entity, mut node) = area.into_inner();
    let display = if game.admiral.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    let current = cards.iter().next();
    if current.map(|(_, of)| of.0) == game.admiral {
        return;
    }
    if let Some((card, _)) = current {
        commands.entity(card).despawn();
    }
    let Some(admiral) = game.admiral else {
        return;
    };
    let color = palette::admiral_color(admiral);
    **lock = Key::tinted(color);
    let (mut background, mut border) = railgun.into_inner();
    background.0 = color.with_alpha(0.22);
    border.set_all(color.with_alpha(0.6));
    commands
        .spawn((
            power_card(&assets, admiral, HAND_ZOOM),
            PowerCardOf(admiral),
            ChildOf(entity),
        ))
        .with_child((
            BlockedShade,
            Node {
                border_radius: BorderRadius::all(px(4.0 * HAND_ZOOM)),
                ..theme::overlay()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.4)),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .observe(
            |click: On<PointerClick>, mut pressed: MessageWriter<PowerPressed>| {
                if click.button == PointerButton::Primary {
                    pressed.write(PowerPressed);
                }
            },
        );
}

/// Name des Schiffs in einem Slot.
fn ship_name(game: &Match, locale: &Locale, at: SlotRef) -> &'static str {
    locale.ship(game.board.ship(at).card).name
}

/// Der Stand der Fähigkeitskarte, für ihr Pulsieren.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct CardState(PowerState);

/// Zielerfassung bzw. Railgun: Anzeige, Art und Rand.
type LockOrRailgun = Or<(With<LockButton>, With<RailgunStatus>)>;
type LockParts<'a> = (Entity, &'a mut Node, Has<LockButton>, &'a mut BorderColor);

/// Zustände der Karte, der Zielerfassung und der Railgun, sobald sich Spielzustand oder Auswahl ändern.
#[allow(clippy::too_many_arguments)]
fn update_states(
    mut commands: Commands,
    game: Res<Match>,
    hud: Res<HudState>,
    locale: Res<Locale>,
    new_card: Query<(), Added<PowerCardOf>>,
    card: Query<(Entity, &Children), With<PowerCardOf>>,
    mut shades: Query<&mut Visibility, With<BlockedShade>>,
    mut parts: Query<LockParts, LockOrRailgun>,
    keys: Query<(Has<Lit>, Has<Pulsing>, Has<Dimmed>)>,
    descendants: Query<&Children>,
    mut labels: Query<&mut Text, (With<LockLabel>, Without<LockWarning>)>,
    mut warnings: Query<&mut Text, (With<LockWarning>, Without<LockLabel>)>,
) {
    let changed =
        game.is_changed() || hud.is_changed() || locale.is_changed() || !new_card.is_empty();
    let Some(admiral) = game.admiral.filter(|_| changed) else {
        return;
    };
    let color = palette::admiral_color(admiral);
    let selecting = |what: Selected| hud.selected == Some(what);
    let possible = |targets: bool| hud.planning && targets;
    let power = PowerState::of(
        game.plan().power.is_some(),
        selecting(Selected::Power),
        possible(!game.power_targets().is_empty()),
    );
    if let Ok((entity, children)) = card.single() {
        set_marked(&mut commands, entity, power == PowerState::Planned);
        commands.entity(entity).insert(CardState(power));
        for &child in children {
            if let Ok(mut visibility) = shades.get_mut(child) {
                visibility.set_if_neq(if power == PowerState::Blocked {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }

    // Kerr: in der Feuerrunde das Ziel der Railgun, sonst die Zielerfassung, solange die Pendulum markieren darf.
    let railgun = game.railgun_target();
    let lock = (railgun.is_none() && game.can_mark()).then(|| {
        let mark = game.plan().mark;
        let state = PowerState::of(
            mark.is_some(),
            selecting(Selected::Mark),
            possible(!game.mark_targets().is_empty()),
        );
        let label = match mark {
            Some(target) => locale.hud_marked(ship_name(&game, &locale, target.on(Side::Enemy))),
            None => locale.hud_mark_button().to_owned(),
        };
        let warning = (mark.is_some() && game.pendulum_moves()).then(|| locale.hud_lock_lost());
        (state, label, warning)
    });
    let railgun = railgun.map(|target| {
        (
            locale.hud_railgun(ship_name(&game, &locale, target)),
            game.pendulum_moves().then(|| locale.hud_shot_lost()),
        )
    });
    for (entity, mut node, is_lock, mut border) in &mut parts {
        let content = if is_lock {
            lock.as_ref()
                .map(|(_, label, warning)| (label.clone(), *warning))
        } else {
            railgun.clone()
        };
        let display = if content.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let Some((label, warning)) = content else {
            continue;
        };
        for part in descendants.iter_descendants(entity) {
            if let Ok(mut text) = labels.get_mut(part) {
                text.0.clone_from(&label);
            }
            if let Ok(mut text) = warnings.get_mut(part) {
                warning.unwrap_or_default().clone_into(&mut text.0);
            }
        }
        let mut edge = if warning.is_some() {
            palette::DAMAGE.with_alpha(0.8)
        } else {
            color.with_alpha(0.6)
        };
        if is_lock && let Some((state, ..)) = &lock {
            if let Ok(current) = keys.get(entity) {
                set_lock_state(&mut commands, entity, current, *state);
            }
            if *state == PowerState::Planned {
                edge = palette::SELECT;
            }
        }
        border.set_all(edge);
    }
}

/// Während der Zielwahl pulsiert der Rahmen der Karte in der Farbe des Admirals.
fn pulse_card(
    time: Res<Time>,
    game: Res<Match>,
    mut cards: Query<(&CardState, &mut BoxShadow), With<PowerCardOf>>,
) {
    let Some(admiral) = game.admiral else {
        return;
    };
    let color = palette::admiral_color(admiral);
    for (&CardState(state), mut shadow) in &mut cards {
        let next = if state == PowerState::Selecting {
            let wave = 0.5 - 0.5 * (time.elapsed_secs() / 1.1 * std::f32::consts::PI).cos();
            vec![
                theme::glow(
                    color.with_alpha(0.4 + 0.5 * wave),
                    (10.0 + 16.0 * wave) * HAND_ZOOM,
                ),
                theme::ring(color.with_alpha(0.6 + 0.4 * wave), (2.0 + wave) * HAND_ZOOM),
            ]
        } else {
            vec![theme::glow(color.with_alpha(0.35), 14.0 * HAND_ZOOM)]
        };
        if shadow.0 != next {
            shadow.0 = next;
        }
    }
}

/// Rahmen der gewählten Karte.
fn set_marked(commands: &mut Commands, entity: Entity, marked: bool) {
    if marked {
        commands.entity(entity).insert(Marked);
    } else {
        commands.entity(entity).remove::<Marked>();
    }
}

/// Die Taste der Zielerfassung: eingeplant leuchtet sie, bei der Zielwahl pulsiert sie, ohne Ziel ist sie blass.
fn set_lock_state(
    commands: &mut Commands,
    entity: Entity,
    (lit, pulsing, dimmed): (bool, bool, bool),
    state: PowerState,
) {
    let wanted = (
        state == PowerState::Planned,
        state == PowerState::Selecting,
        state == PowerState::Blocked,
    );
    if wanted == (lit, pulsing, dimmed) {
        return;
    }
    let mut key = commands.entity(entity);
    key.remove::<(Lit, Pulsing, Dimmed)>();
    match state {
        PowerState::Open => {}
        PowerState::Selecting => {
            key.insert(Pulsing);
        }
        PowerState::Planned => {
            key.insert(Lit);
        }
        PowerState::Blocked => {
            key.insert(Dimmed);
        }
    }
}

pub(super) struct PowerCardPlugin;

impl Plugin for PowerCardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (build_card, update_states, pulse_card)
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
