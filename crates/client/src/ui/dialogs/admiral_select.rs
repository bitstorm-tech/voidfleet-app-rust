//! Die Admiralswahl (Port von `ui/admiralSelect.ts`): Nach „Neuer Run“ liegt über dem Brett ein Overlay
//! mit vier Karten — Porträt, Name und Beiname, das Flaggschiff mit Bild, Hülle, ATK und Waffe, Passiv,
//! Admiralsstation und Kennkarte, alles in der Akzentfarbe des Admirals. Ein Klick wählt ([`AdmiralChosen`]); bis der
//! Run steht, bleibt die Karte markiert und die übrigen sind gesperrt.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::LineHeight,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button, observe},
    window::PrimaryWindow,
};
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};

use super::{AdmiralChosen, Density, board_overlay, overlay_backdrop, overlay_hint, overlay_title};
use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    data::{
        admirals::AdmiralPortraits,
        card_art::{Card, CardArt},
    },
    i18n::Locale,
    palette,
    run::{ActiveRun, Screen},
    settings::Language,
    ui::{
        Fonts, UiIcons, button,
        card::ART_ASPECT,
        theme::{self, UiOpacity},
    },
};

/// Der gewählte Admiral, bis der Run steht.
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
pub(super) const CARD_FACE: Color = theme::rgba(22, 26, 34, 0.96);
const CARD_FACE_HOVER: Color = theme::rgba(26, 30, 40, 0.98);
/// Überschriften der Abschnitte (`h4`).
pub(super) const SECTION_LABEL: Color = theme::rgba(199, 219, 250, 0.6);

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

/// Überschrift eines Abschnitts, beim Flaggschiff mit drei Sternen davor.
fn section_label(fonts: &Fonts, label: &str, stars: Option<Handle<Image>>) -> impl Bundle + use<> {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(5),
            margin: UiRect::bottom(px(3)),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn((
            SpawnIter(stars.into_iter().map(|stars| {
                (
                    Node {
                        width: px(22),
                        height: px(22),
                        // Die Sterne füllen nur ein Drittel der Höhe; die Zeile bleibt so hoch wie die Schrift.
                        margin: UiRect::vertical(px(-4.5)),
                        ..default()
                    },
                    ImageNode::new(stars).with_color(palette::FLAGSHIP_TRIM),
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
    art: &CardArt,
    locale: &Locale,
    admiral: AdmiralId,
    density: Density,
) -> impl Bundle {
    let color = palette::admiral_color(admiral);
    let text = locale.admiral(admiral);
    let flagship = admiral.flagship();
    let signature = locale.card(voidfleet_core::cards::DeckCard::new(
        admiral.signature_card(),
    ));
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
                Node {
                    column_gap: px(10),
                    ..section()
                },
                Pickable::IGNORE,
                children![
                    (
                        Node {
                            width: px(46),
                            height: px(64),
                            flex_shrink: 0.0,
                            overflow: Overflow::clip(),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BoxShadow(vec![theme::ring(
                            palette::FLAGSHIP_TRIM.with_alpha(0.7),
                            1.0
                        )]),
                        Pickable::IGNORE,
                        children![theme::cover_image(
                            art.get(Card::Flagship(flagship)),
                            ART_ASPECT,
                            0.5
                        )],
                    ),
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![
                            section_label(
                                fonts,
                                locale.label_flagship(),
                                Some(icons.stars.clone())
                            ),
                            (
                                theme::text(
                                    fonts.body(ship_name),
                                    locale.flagship(flagship),
                                    palette::FLAGSHIP_TRIM
                                ),
                                Pickable::IGNORE,
                            ),
                            (
                                theme::text(
                                    fonts.body(14.0),
                                    format!(
                                        "{} {} · {} {}",
                                        locale.stats_hull(),
                                        admiral.hull(),
                                        locale.stats_atk(),
                                        admiral.attack()
                                    ),
                                    palette::TEXT_DIM
                                ),
                                Pickable::IGNORE,
                            ),
                        ],
                    ),
                ],
            ),
            (
                Node {
                    row_gap: px(4),
                    ..section()
                },
                Pickable::IGNORE,
                children![
                    lead_text(
                        fonts.body(body),
                        format!("{}:", locale.label_weapon()),
                        palette::TEXT,
                        text.weapon,
                        line_height,
                    ),
                    lead_text(
                        fonts.body(body),
                        format!("{}:", locale.label_passive()),
                        palette::TEXT,
                        text.passive,
                        line_height,
                    ),
                ],
            ),
            (
                section(),
                Pickable::IGNORE,
                children![
                    section_label(fonts, locale.label_station(), None),
                    lead_text(
                        fonts.body(body),
                        format!("{}:", text.station),
                        color,
                        text.station_text,
                        line_height,
                    ),
                ],
            ),
            (
                section(),
                Pickable::IGNORE,
                children![
                    section_label(fonts, locale.label_signature(), None),
                    lead_text(
                        fonts.body(body),
                        format!("{}:", signature.name),
                        color,
                        signature.effect,
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

/// Zeigt die Wahl auf ihrem Bildschirm; baut sie bei anderer Sprache oder Fensterhöhe neu.
fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    portraits: Res<AdmiralPortraits>,
    art: Res<CardArt>,
    window: Single<&Window, With<PrimaryWindow>>,
    overlays: Query<(Entity, &AdmiralOverlay)>,
) {
    let open = *screen == Screen::Admirals;
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
        .map(|admiral| admiral_card(&fonts, &icons, &portraits, &art, &locale, admiral, density));
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

/// Hebt die wartende Wahl auf, sobald der Run steht.
fn settle(run: Option<Res<ActiveRun>>, mut pending: ResMut<PendingAdmiral>) {
    if run.is_some() && pending.0.is_some() {
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
