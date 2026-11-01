//! Die Admiralswahl vor Runde 1 (Port von `ui/admiralSelect.ts`): Solange der eigene Admiral fehlt, liegt über dem
//! Brett ein Overlay mit fünf Karten — Porträt, Name und Beiname, das Flaggschiff mit Klasse, HP, ATK und Fähigkeit,
//! die Admiralsfähigkeit, alles in der Akzentfarbe des Admirals. Ein Klick wählt ([`AdmiralChosen`]); bis der Server
//! bestätigt (oder ablehnt), bleibt die Karte markiert und die übrigen sind gesperrt. Die Hand darunter bleibt frei.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::LineHeight,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button, observe},
    window::PrimaryWindow,
};
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};

use super::{
    AdmiralChosen, Density, NewMatch, board_overlay, choice_cancelled, overlay_backdrop,
    overlay_hint, overlay_title,
};
use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    data::admirals::AdmiralPortraits,
    game::{Match, ServerError},
    i18n::Locale,
    palette,
    settings::Language,
    ui::{
        Fonts, UiIcons, button,
        card::ART_ASPECT,
        theme::{self, UiOpacity},
    },
};

/// Der gewählte Admiral, bis der Server ihn bestätigt.
#[derive(Resource, Default, Debug)]
struct PendingAdmiral(Option<AdmiralId>);

/// Das Overlay; gebaut für diese Sprache und Fensterhöhe.
#[derive(Component, PartialEq, Eq, Debug)]
struct AdmiralOverlay {
    language: Language,
    density: Density,
}

/// Die Karte eines Admirals.
#[derive(Component, Clone, Copy, Debug)]
struct AdmiralCard(AdmiralId);

/// Das Feld „Wählen“ unten auf der Karte.
#[derive(Component)]
struct ChooseLabel;

/// Grund der Karte, beim Überfahren etwas heller.
const CARD_FACE: Color = theme::rgba(22, 26, 34, 0.96);
const CARD_FACE_HOVER: Color = theme::rgba(26, 30, 40, 0.98);
/// Überschriften der Abschnitte (`h4`).
const SECTION_LABEL: Color = theme::rgba(199, 219, 250, 0.6);

/// Text mit hervorgehobenem Anfang (`<b>…</b> …`).
fn lead_text(
    font: TextFont,
    lead: String,
    lead_color: Color,
    rest: impl Into<String>,
    line_height: f32,
) -> impl Bundle {
    (
        Text::new(lead),
        font.clone(),
        TextColor(lead_color),
        LineHeight::RelativeToFont(line_height),
        Pickable::IGNORE,
        children![(
            TextSpan::new(format!(" {}", rest.into())),
            font,
            TextColor(palette::TEXT_DIM),
            LineHeight::RelativeToFont(line_height),
        )],
    )
}

/// Überschrift eines Abschnitts, beim Flaggschiff mit Krone davor.
fn section_label(fonts: &Fonts, label: &str, crown: Option<Handle<Image>>) -> impl Bundle + use<> {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(5),
            margin: UiRect::bottom(px(3)),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn((
            SpawnIter(crown.into_iter().map(|crown| {
                (
                    Node {
                        width: px(13),
                        height: px(13),
                        ..default()
                    },
                    ImageNode::new(crown).with_color(palette::FLAGSHIP_TRIM),
                    Pickable::IGNORE,
                )
            })),
            Spawn((
                theme::caps(
                    fonts.display_bold(14.0),
                    label,
                    theme::TRACKING,
                    SECTION_LABEL,
                ),
                Pickable::IGNORE,
            )),
        )),
    )
}

/// Die Karte eines Admirals.
fn admiral_card(
    fonts: &Fonts,
    icons: &UiIcons,
    portraits: &AdmiralPortraits,
    locale: &Locale,
    admiral: AdmiralId,
    density: Density,
) -> impl Bundle {
    let color = palette::admiral_color(admiral);
    let text = locale.admiral(admiral);
    let flagship = admiral.flagship();
    let ship = flagship.card();
    let power = locale.power(admiral.power());
    let (portrait, name, epithet, ship_name, body, line_height, choose_size, choose_pad) =
        match density {
            Density::Regular => (120.0, 24.0, 17.0, 18.0, 14.5, 1.3, 18.0, 6.0),
            Density::Compact => (64.0, 20.0, 15.0, 16.0, 13.0, 1.28, 16.0, 3.0),
            Density::Tight => (52.0, 20.0, 15.0, 16.0, 12.5, 1.28, 16.0, 3.0),
        };
    let section = || Node {
        flex_direction: FlexDirection::Column,
        padding: UiRect::horizontal(px(14)),
        ..default()
    };
    (
        AdmiralCard(admiral),
        Button,
        Hovered::default(),
        Node {
            flex_basis: px(330),
            flex_shrink: 1.0,
            min_width: px(170),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::bottom(px(9)),
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
        BoxShadow::default(),
        UiOpacity(1.0),
        observe(choose),
        children![
            // Porträt, unten in die Karte auslaufend.
            (
                Node {
                    height: px(portrait),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    theme::cover_image(portraits.get(admiral), ART_ASPECT, 0.25),
                    (
                        theme::overlay(),
                        theme::vertical_gradient(&[
                            (CARD_FACE.with_alpha(0.0), 70.0),
                            (CARD_FACE, 100.0)
                        ]),
                        Pickable::IGNORE,
                    ),
                ],
            ),
            (
                Node {
                    margin: UiRect::top(px(-18)),
                    ..section()
                },
                Pickable::IGNORE,
                children![
                    (
                        theme::text(fonts.body(name), text.name, palette::TEXT),
                        theme::text_shadow(0.9),
                        Pickable::IGNORE,
                    ),
                    (
                        theme::text(fonts.body(epithet), text.epithet, color),
                        theme::text_shadow(0.9),
                        Pickable::IGNORE,
                    ),
                ],
            ),
            (
                section(),
                Pickable::IGNORE,
                children![
                    section_label(
                        fonts,
                        locale.admiral_select_flagship(),
                        Some(icons.crown.clone())
                    ),
                    (
                        theme::text(
                            fonts.body(ship_name),
                            locale.ship(flagship).name,
                            palette::FLAGSHIP_TRIM
                        ),
                        Pickable::IGNORE,
                    ),
                    (
                        Node {
                            margin: UiRect::bottom(px(3)),
                            ..default()
                        },
                        theme::text(
                            fonts.body(14.0),
                            format!(
                                "{} · {} {} · {} {}",
                                locale.ship_class(ship.class),
                                ship.hp,
                                locale.stats_hp(),
                                ship.atk,
                                locale.stats_atk()
                            ),
                            palette::TEXT_DIM
                        ),
                        Pickable::IGNORE,
                    ),
                    lead_text(
                        fonts.body(body),
                        format!("{}:", text.ability),
                        palette::TEXT,
                        locale.ship(flagship).effect,
                        line_height,
                    ),
                ],
            ),
            (
                section(),
                Pickable::IGNORE,
                children![
                    section_label(fonts, locale.admiral_select_power(), None),
                    lead_text(
                        fonts.body(body),
                        format!("{}:", power.name),
                        color,
                        power.effect,
                        line_height,
                    ),
                ],
            ),
            (
                ChooseLabel,
                Node {
                    margin: UiRect::new(px(14), px(14), Val::Auto, Val::ZERO),
                    padding: UiRect::vertical(px(choose_pad)),
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(color.with_alpha(0.6)),
                Pickable::IGNORE,
                children![(
                    theme::caps(
                        fonts.display_bold(choose_size),
                        locale.admiral_select_choose(),
                        theme::TRACKING,
                        color
                    ),
                    Pickable::IGNORE,
                )],
            ),
        ],
    )
}

/// Ein Klick wählt den Admiral, sofern noch keine Wahl wartet (gesperrte Karten melden keinen Klick).
fn choose(
    activate: On<Activate>,
    cards: Query<&AdmiralCard>,
    mut pending: ResMut<PendingAdmiral>,
    mut chosen: MessageWriter<AdmiralChosen>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let Ok(&AdmiralCard(admiral)) = cards.get(activate.entity) else {
        return;
    };
    if pending.0.is_none() {
        pending.0 = Some(admiral);
        chosen.write(AdmiralChosen(admiral));
        sounds.write(PlayUiSound(UiSound::Click));
        sounds.write(PlayUiSound(UiSound::Confirm));
    }
}

/// Zeigt die Wahl, solange der eigene Admiral fehlt; baut sie bei anderer Sprache oder Fensterhöhe neu.
fn show(
    mut commands: Commands,
    game: Res<Match>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    portraits: Res<AdmiralPortraits>,
    window: Single<&Window, With<PrimaryWindow>>,
    overlays: Query<(Entity, &AdmiralOverlay)>,
) {
    let open = game.round > 0 && game.admiral.is_none() && game.result.is_none();
    let wanted = open.then(|| AdmiralOverlay {
        language: locale.language,
        density: Density::of(&window),
    });
    if overlays
        .iter()
        .map(|(_, overlay)| overlay)
        .eq(wanted.iter())
    {
        return;
    }
    for (entity, _) in &overlays {
        commands.entity(entity).despawn();
    }
    let Some(overlay) = wanted else {
        return;
    };
    let density = overlay.density;
    let cards = ADMIRAL_IDS
        .map(|admiral| admiral_card(&fonts, &icons, &portraits, &locale, admiral, density));
    commands.spawn((
        Name::new("Admiralswahl"),
        overlay,
        board_overlay(),
        Children::spawn((
            Spawn(overlay_backdrop()),
            Spawn(overlay_title(
                &fonts,
                locale.admiral_select_title(),
                density,
            )),
            SpawnIter(
                (density != Density::Tight)
                    .then(|| overlay_hint(&fonts, locale.admiral_select_hint(), density))
                    .into_iter(),
            ),
            Spawn((
                Node {
                    max_width: percent(100),
                    column_gap: px(16),
                    ..default()
                },
                Pickable::IGNORE,
                Children::spawn(SpawnIter(cards.into_iter())),
            )),
        )),
    ));
}

/// Hebt die wartende Wahl auf, sobald der Admiral feststeht, der Server ablehnt oder ein neues Match beginnt.
fn settle(
    game: Res<Match>,
    mut pending: ResMut<PendingAdmiral>,
    mut errors: MessageReader<ServerError>,
    mut new_match: MessageReader<NewMatch>,
) {
    if choice_cancelled(&mut errors, &mut new_match) || game.admiral.is_some() {
        pending.0 = None;
    }
}

/// Sperrt die Karten, solange eine Wahl wartet; die gewählte bleibt markiert, die übrigen verblassen.
fn lock_cards(
    mut commands: Commands,
    pending: Res<PendingAdmiral>,
    mut cards: Query<(
        Entity,
        &AdmiralCard,
        Has<InteractionDisabled>,
        &mut UiOpacity,
    )>,
) {
    for (entity, &AdmiralCard(admiral), disabled, mut opacity) in &mut cards {
        let chosen = pending.0 == Some(admiral);
        let locked = pending.0.is_some();
        if disabled != locked {
            button::set_enabled(&mut commands, entity, !locked);
        }
        opacity.set_if_neq(UiOpacity(if locked && !chosen { 0.35 } else { 1.0 }));
    }
}

/// Wie schnell die Karte ihrem Ziel folgt (1/s): etwa die 0,15 s der CSS-Transition.
const FOLLOW: f32 = 20.0;

type CardLook<'a> = (
    &'a AdmiralCard,
    &'a Hovered,
    Has<InteractionDisabled>,
    &'a Children,
    &'a mut UiTransform,
    &'a mut BackgroundColor,
    &'a mut BoxShadow,
);

/// Beim Überfahren hebt sich die Karte und leuchtet, „Wählen“ füllt sich mit der Farbe des Admirals; die gewählte
/// Karte trägt einen Ring. Ein gesperrter Admiral kann nicht überfahren worden sein: Geklickt wurde der gewählte.
fn animate_cards(
    time: Res<Time>,
    pending: Res<PendingAdmiral>,
    mut cards: Query<CardLook, Without<ChooseLabel>>,
    mut labels: Query<(&mut BackgroundColor, &Children), With<ChooseLabel>>,
    mut texts: Query<&mut TextColor>,
) {
    for (&AdmiralCard(admiral), hovered, disabled, children, mut transform, mut face, mut shadow) in
        &mut cards
    {
        let color = palette::admiral_color(admiral);
        let chosen = pending.0 == Some(admiral);
        // Gesperrte Karten ruhen; ihre Farben dimmt `UiOpacity`.
        if disabled && !chosen {
            continue;
        }
        let lifted = hovered.get() && !disabled;
        let offset = match transform.translation.y {
            Val::Px(offset) => offset,
            _ => 0.0,
        };
        let target = if lifted { -6.0 } else { 0.0 };
        let mut next = offset;
        next.smooth_nudge(&target, FOLLOW, time.delta_secs());
        let next = if (next - target).abs() < 0.05 {
            target
        } else {
            next
        };
        if next != offset {
            transform.translation = Val2::px(0.0, next);
        }
        face.set_if_neq(BackgroundColor(if lifted {
            CARD_FACE_HOVER
        } else {
            CARD_FACE
        }));
        let glow = if chosen {
            vec![
                theme::glow(color.with_alpha(0.7), 30.0),
                theme::ring(color, 2.0),
            ]
        } else if lifted {
            vec![theme::glow(color.with_alpha(0.5), 26.0)]
        } else {
            Vec::new()
        };
        if shadow.0 != glow {
            shadow.0 = glow;
        }
        let filled = lifted || chosen;
        for &child in children {
            let Ok((mut background, label)) = labels.get_mut(child) else {
                continue;
            };
            background.set_if_neq(BackgroundColor(if filled { color } else { Color::NONE }));
            for &text in label {
                if let Ok(mut text) = texts.get_mut(text) {
                    text.set_if_neq(TextColor(if filled { palette::BACKGROUND } else { color }));
                }
            }
        }
    }
}

pub(super) struct AdmiralSelectPlugin;

impl Plugin for AdmiralSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingAdmiral>().add_systems(
            Update,
            (settle, show, lock_cards, animate_cards)
                .chain()
                .run_if(in_state(AppState::Match)),
        );
    }
}
