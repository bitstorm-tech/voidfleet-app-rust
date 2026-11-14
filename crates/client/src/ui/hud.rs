//! Das HUD (Port von `ui/hud.ts`): Kopfzeile mit Runde, Gefechtsuhr ab Runde 8, der Hülle des Flaggschiffs samt
//! verbrauchter Reparaturgrenze und dem Stand des Runs ([`super::run_bar`]), darunter die aktiven Doktrinen als kleine Karten, das
//! Log hinter dem Icon-Knopf oben rechts, das Banner für kurze Ansagen (etwa abgelehnte Aktionen) und die Ankündigung
//! einer neuen Runde.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::LineHeight,
    ui_widgets::{Activate, ScrollArea, observe},
};

use super::{
    AppendLog, ClearLog, Fonts, HideBanner, ShowBanner, ShowRoundBanner, UiIcons,
    button::{self, Lit},
    card::{CardAssets, CardHover, HAND_ZOOM, command_card},
    hand::HandBar,
    theme::{self, UiOpacity},
    tooltip::Tooltip,
};
use voidfleet_core::cards::DeckCard;

use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    data::admirals::AdmiralPortraits,
    game::Battle,
    i18n::Locale,
    palette,
    run::Screen,
    scene::animation::AnimationScope,
    settings::Settings,
};

#[derive(Component)]
struct RoundLabel;

#[derive(Component)]
struct ClockLabel;

#[derive(Component)]
struct HullLabel;

/// Die verbrauchte Reparaturgrenze hinter der Hülle, `(x/6)`; der Tooltip sitzt am [`HullLabel`].
#[derive(Component)]
struct RepairLabel;

/// Die Leiste der aktiven Doktrinen unter der Kopfzeile.
#[derive(Component)]
struct DoctrineBar;

/// Eine kleine Doktrin der Leiste mit ihrer großen Ansicht, die beim Hover über ihr erscheint. Eigens gesetzt statt
/// hochskaliert, sonst wird die klein gerasterte Schrift unscharf.
#[derive(Component)]
struct DoctrinePreview(Entity);

/// Maßstab der Doktrinen in der Leiste.
const DOCTRINE_SCALE: f32 = 0.45;

/// Der Icon-Knopf des Logs.
#[derive(Component)]
pub(super) struct LogButton;

#[derive(Component)]
struct LogPanel;

#[derive(Component)]
struct LogText;

/// Das Banner für kurze Ansagen und die Ankündigung einer neuen Runde, mit der Uhr seines Auftritts.
#[derive(Component, Default)]
struct Banner(Timer);

#[derive(Component)]
struct RevealBanner;

#[derive(Component)]
struct RoundBanner;

/// Dauer der Banner ohne Streckung (Sekunden).
const REVEAL_TIME: f32 = 1.7;
const ROUND_TIME: f32 = 1.9;

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        Name::new("Kopfzeile"),
        Node {
            position_type: PositionType::Absolute,
            top: px(24),
            left: px(32),
            right: px(248),
            align_items: AlignItems::FlexStart,
            column_gap: px(24),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                RoundLabel,
                theme::caps(fonts.display_bold(30.0), "", theme::TRACKING, theme::ACCENT),
                Pickable::IGNORE,
            ),
            (
                HullLabel,
                theme::caps(fonts.display_bold(22.0), "", theme::TRACKING, palette::TEXT),
                Tooltip::default(),
                children![(
                    RepairLabel,
                    TextSpan::default(),
                    fonts.display_bold(22.0),
                    TextColor(palette::TEXT_DIM),
                )],
            ),
            (
                ClockLabel,
                Node {
                    display: Display::None,
                    padding: UiRect::axes(px(10), px(4)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BorderColor::all(palette::BUFF.with_alpha(0.6)),
                Tooltip::default(),
                children![(
                    theme::caps(fonts.display_bold(16.0), "", theme::TRACKING, palette::BUFF),
                    Pickable::IGNORE,
                )],
            ),
            super::run_bar::run_header(),
        ],
    ));
    commands.spawn((
        Name::new("Doktrinen"),
        DoctrineBar,
        Node {
            position_type: PositionType::Absolute,
            top: px(76),
            left: px(32),
            column_gap: px(8),
            ..default()
        },
        Pickable::IGNORE,
    ));
    commands.spawn((
        Name::new("Log"),
        LogPanel,
        ScrollArea,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: px(44),
            right: px(8),
            width: px(420),
            height: px(420),
            padding: UiRect::all(px(6)),
            overflow: Overflow::scroll_y(),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme::PANEL),
        GlobalZIndex(5),
        children![(
            LogText,
            theme::text(fonts.body(14.0), "", palette::TEXT_DIM),
            LineHeight::RelativeToFont(1.45),
            Pickable::IGNORE,
        )],
    ));
    // Banner: in voller Breite, damit sie mittig stehen; der Inhalt entsteht mit jeder Ankündigung neu.
    let centered = |top: Val| Node {
        position_type: PositionType::Absolute,
        top,
        left: Val::ZERO,
        right: Val::ZERO,
        justify_content: JustifyContent::Center,
        ..default()
    };
    commands.spawn((
        Name::new("Banner"),
        centered(px(120)),
        Pickable::IGNORE,
        children![(
            RevealBanner,
            Banner::default(),
            Node {
                align_items: AlignItems::Center,
                column_gap: px(14),
                min_width: px(280),
                padding: UiRect::axes(px(28), px(10)),
                border: UiRect::top(px(3)),
                border_radius: BorderRadius::all(px(3)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::PANEL),
            BorderColor::all(theme::ACCENT),
            UiOpacity(0.0),
            Visibility::Hidden,
            Pickable::IGNORE,
        )],
    ));
    commands.spawn((
        Name::new("Rundenbanner"),
        centered(percent(38)),
        Pickable::IGNORE,
        children![(
            RoundBanner,
            Banner::default(),
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                padding: UiRect::axes(px(72), px(14)),
                border: UiRect::vertical(px(1)),
                ..default()
            },
            theme::horizontal_gradient(&[
                (Color::NONE, 0.0),
                (theme::PANEL, 25.0),
                (theme::PANEL, 75.0),
                (Color::NONE, 100.0),
            ]),
            BorderColor::all(palette::PLAYER.with_alpha(0.45)),
            UiOpacity(0.0),
            Visibility::Hidden,
            Pickable::IGNORE,
        )],
    ));
}

/// Der Icon-Knopf des Logs in der Leiste oben rechts.
pub(super) fn log_button(icons: &UiIcons, locale: &Locale) -> impl Bundle {
    (
        LogButton,
        button::icon_button(icons.log.clone(), locale.hud_show_log()),
        observe(toggle_log),
    )
}

/// Klappt das Log auf und zu.
fn toggle_log(
    activate: On<Activate>,
    mut commands: Commands,
    locale: Res<Locale>,
    mut buttons: Query<(Has<Lit>, &mut Tooltip)>,
    mut panel: Single<&mut Node, With<LogPanel>>,
) {
    let (expanded, mut tooltip) = buttons.get_mut(activate.entity).expect("Log-Knopf");
    let expanded = !expanded;
    if expanded {
        commands.entity(activate.entity).insert(Lit);
    } else {
        commands.entity(activate.entity).remove::<Lit>();
    }
    panel.display = if expanded {
        Display::Flex
    } else {
        Display::None
    };
    tooltip.0 = if expanded {
        locale.hud_hide_log()
    } else {
        locale.hud_show_log()
    }
    .into();
}

/// Hängt Zeilen ans Log und scrollt ans Ende; ein neues Gefecht leert es vorher.
fn append_log(
    mut messages: MessageReader<AppendLog>,
    mut clear: MessageReader<ClearLog>,
    mut text: Single<&mut Text, With<LogText>>,
    mut panel: Single<(&mut ScrollPosition, &ComputedNode), With<LogPanel>>,
) {
    if clear.read().last().is_some() {
        text.0.clear();
    }
    let mut appended = false;
    for AppendLog(lines) in messages.read() {
        for line in lines {
            if !text.0.is_empty() {
                text.0.push('\n');
            }
            text.0.push_str(line);
        }
        appended = true;
    }
    if appended {
        // Bis ans Ende; das Layout begrenzt auf den Inhalt.
        let (position, node) = &mut *panel;
        position.y = node.content_size().y * node.inverse_scale_factor() + 10_000.0;
    }
}

/// Runde, Gefechtsuhr, Hülle und Reparaturgrenze; ohne Gefecht bleibt die Kopfzeile leer.
fn update_top_bar(
    battle: Option<Res<Battle>>,
    locale: Res<Locale>,
    mut round: Single<&mut Text, (With<RoundLabel>, Without<HullLabel>)>,
    hull: Single<
        (&mut Text, &mut Tooltip),
        (With<HullLabel>, Without<RoundLabel>, Without<ClockLabel>),
    >,
    mut repair: Single<&mut TextSpan, With<RepairLabel>>,
    clock: Single<(&mut Node, &mut Tooltip, &Children), With<ClockLabel>>,
    mut texts: Query<&mut Text, (Without<RoundLabel>, Without<HullLabel>)>,
) {
    let changed = battle.as_ref().is_none_or(|battle| battle.is_changed());
    if !changed && !locale.is_changed() {
        return;
    }
    let (mut node, mut tooltip, children) = clock.into_inner();
    let (mut hull, mut hull_tooltip) = hull.into_inner();
    let Some(battle) = battle else {
        round.0.clear();
        hull.0.clear();
        repair.0.clear();
        hull_tooltip.0.clear();
        node.display = Display::None;
        return;
    };
    round.0 = locale.hud_round(battle.round).to_uppercase();
    hull.0 = locale
        .hud_hull(battle.flagship.hull, battle.flagship.max_hull)
        .to_uppercase();
    repair.0 = format!(" {}", locale.hud_repair(battle.flagship.repaired));
    hull_tooltip.0 = locale.hud_repair_hint(battle.flagship.repaired);
    let bonus = battle.clock();
    node.display = if bonus == 0 {
        Display::None
    } else {
        Display::Flex
    };
    tooltip.0 = locale.hud_clock_hint(bonus);
    for &child in children {
        if let Ok(mut text) = texts.get_mut(child) {
            text.0 = locale.hud_clock(bonus).to_uppercase();
        }
    }
}

/// Baut die Leiste der Doktrinen neu, sobald sich die gespielten Doktrinen ändern; ohne Gefecht bleibt sie leer.
fn update_doctrines(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    assets: CardAssets,
    bar: Single<Entity, With<DoctrineBar>>,
    mut shown: Local<Vec<DeckCard>>,
) {
    let doctrines = battle
        .map(|battle| battle.doctrines.clone())
        .unwrap_or_default();
    if *shown == doctrines && !assets.locale.is_changed() {
        return;
    }
    commands.entity(*bar).despawn_related::<Children>();
    commands.entity(*bar).with_children(|bar| {
        for &card in &doctrines {
            // Die große Ansicht ist ein Kind der kleinen Karte und deckt sie zu: Auch über ihr bleibt die kleine
            // Karte gehovert, und die Schlüsselwörter der großen Karte zeigen ihre Tooltips.
            let mut mini = bar.spawn((
                command_card(&assets, card, DOCTRINE_SCALE),
                CardHover::Raise,
            ));
            let mut preview = Entity::PLACEHOLDER;
            mini.with_children(|mini| {
                preview = mini
                    .spawn((
                        Node {
                            display: Display::None,
                            position_type: PositionType::Absolute,
                            top: Val::ZERO,
                            left: Val::ZERO,
                            ..default()
                        },
                        GlobalZIndex(10),
                        Pickable::IGNORE,
                        children![command_card(&assets, card, HAND_ZOOM)],
                    ))
                    .id();
            });
            mini.insert(DoctrinePreview(preview));
        }
    });
    *shown = doctrines;
}

/// Zeigt die große Ansicht einer Doktrin, solange der Zeiger auf ihrer kleinen Karte liegt.
fn show_doctrine_previews(cards: Query<(&Hovered, &DoctrinePreview)>, mut nodes: Query<&mut Node>) {
    for (hovered, preview) in &cards {
        if let Ok(mut node) = nodes.get_mut(preview.0) {
            let display = if hovered.get() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

/// Hand, Stationen und Log gehören zum Gefecht; auf Karte und Knoten liegen sie verborgen.
fn show_battle_hud(
    screen: Res<Screen>,
    mut parts: Query<
        &mut Visibility,
        Or<(
            With<HandBar>,
            With<LogButton>,
            With<LogPanel>,
            With<DoctrineBar>,
        )>,
    >,
) {
    let visibility = if *screen == Screen::Battle {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut part in &mut parts {
        part.set_if_neq(visibility);
    }
}

/// Zeigt eine kurze Ansage.
fn show_banner(
    mut commands: Commands,
    mut messages: MessageReader<ShowBanner>,
    fonts: Res<Fonts>,
    portraits: Res<AdmiralPortraits>,
    banner: Single<(Entity, &mut Banner, &mut BorderColor, &mut Visibility), With<RevealBanner>>,
    settings: Res<Settings>,
) {
    let Some(message) = messages.read().last() else {
        return;
    };
    let (entity, mut state, mut border, mut visibility) = banner.into_inner();
    let color = theme::ACCENT;
    border.set_all(color);
    *state = Banner(Timer::from_seconds(
        REVEAL_TIME * settings.time_scale(AnimationScope::General),
        TimerMode::Once,
    ));
    *visibility = Visibility::Inherited;
    let portrait = message.admiral.map(|admiral| {
        let ring = palette::admiral_color(admiral);
        (
            Node {
                width: px(48),
                height: px(48),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            ImageNode::new(portraits.get(admiral)),
            BoxShadow(vec![
                theme::glow(ring.with_alpha(0.6), 14.0),
                theme::ring(ring, 2.0),
            ]),
            Pickable::IGNORE,
        )
    });
    let subtitle = message.subtitle.clone().map(|subtitle| {
        (
            theme::text(fonts.body(16.0), subtitle, palette::TEXT_DIM),
            Pickable::IGNORE,
        )
    });
    commands
        .entity(entity)
        .despawn_related::<Children>()
        .insert(Children::spawn((
            SpawnIter(portrait.into_iter()),
            Spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(2),
                    ..default()
                },
                Pickable::IGNORE,
                Children::spawn((
                    Spawn((
                        theme::caps(fonts.display(30.0), &message.title, theme::TRACKING, color),
                        Pickable::IGNORE,
                    )),
                    SpawnIter(subtitle.into_iter()),
                )),
            )),
        )));
}

fn hide_banner(
    mut messages: MessageReader<HideBanner>,
    mut banner: Single<&mut Banner, With<RevealBanner>>,
) {
    if messages.read().last().is_some() {
        banner.0.finish();
    }
}

/// Kündigt eine neue Runde an, ab Runde 8 mit der Gefechtsuhr.
fn show_round_banner(
    mut commands: Commands,
    mut messages: MessageReader<ShowRoundBanner>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    banner: Single<(Entity, &mut Banner, &mut Visibility), With<RoundBanner>>,
    settings: Res<Settings>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let Some(&ShowRoundBanner(round)) = messages.read().last() else {
        return;
    };
    let (entity, mut state, mut visibility) = banner.into_inner();
    *state = Banner(Timer::from_seconds(
        ROUND_TIME * settings.time_scale(AnimationScope::General),
        TimerMode::Once,
    ));
    *visibility = Visibility::Inherited;
    let bonus = round.saturating_sub(voidfleet_core::data::rules().battle.clock_start - 1) as i32;
    let rising = (bonus > 0).then(|| {
        (
            theme::text(
                fonts.display(18.0),
                locale.hud_clock_hint(bonus),
                palette::BUFF,
            ),
            Pickable::IGNORE,
        )
    });
    commands
        .entity(entity)
        .despawn_related::<Children>()
        .insert(Children::spawn((
            Spawn((
                theme::caps(
                    fonts.display(16.0),
                    locale.hud_new_round(),
                    0.3,
                    palette::TEXT_DIM,
                ),
                Pickable::IGNORE,
            )),
            Spawn((
                theme::caps(
                    fonts.display_bold(60.0),
                    &locale.hud_round(round),
                    0.04,
                    theme::ACCENT,
                ),
                Pickable::IGNORE,
            )),
            SpawnIter(rising.into_iter()),
        )));
    sounds.write(PlayUiSound(UiSound::Round));
}

/// Blendet die Banner ein, hält sie und blendet sie wieder aus (Keyframes `banner` und `round-banner`).
fn animate_banners(
    time: Res<Time>,
    mut banners: Query<(
        &mut Banner,
        &mut UiOpacity,
        &mut UiTransform,
        &mut Visibility,
        Has<RoundBanner>,
    )>,
) {
    for (mut banner, mut opacity, mut transform, mut visibility, round) in &mut banners {
        if *visibility == Visibility::Hidden {
            continue;
        }
        let t = banner.0.tick(time.delta()).fraction();
        let (fade_in, hold) = if round { (0.15, 0.75) } else { (0.12, 0.8) };
        let eased = EaseFunction::QuadraticOut.sample_clamped(t / fade_in);
        let alpha = if t < fade_in {
            eased
        } else if t < hold {
            1.0
        } else {
            1.0 - (t - hold) / (1.0 - hold)
        };
        opacity.set_if_neq(UiOpacity(alpha));
        *transform = if round {
            UiTransform::from_scale(Vec2::splat(1.15 - 0.15 * eased))
        } else {
            UiTransform::from_translation(Val2::px(0.0, -10.0 * (1.0 - eased)))
        };
        if t >= 1.0 {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Match), spawn)
            .add_systems(
                Update,
                (
                    show_battle_hud,
                    update_top_bar,
                    update_doctrines,
                    show_doctrine_previews,
                    append_log,
                    show_banner,
                    hide_banner,
                    show_round_banner,
                    animate_banners,
                )
                    .chain()
                    .run_if(in_state(AppState::Match)),
            );
    }
}
