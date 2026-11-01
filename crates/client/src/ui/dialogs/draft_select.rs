//! Der Nachschub nach jeder Runde (Port von `ui/draftSelect.ts`): Solange [`Match::draft`] offen ist, liegt über dem
//! Brett ein Overlay mit bis zu drei Schiffen und drei Befehlen so groß wie auf der Hand; ein Klick wählt je eines, ein
//! zweiter nimmt die Wahl zurück. Hat ein eigener Scrap Raider Schrott geborgen, ist einer der Befehle die
//! Schrottverwertung. „Übernehmen“ meldet die Wahl ([`DraftChosen`]); bis der Server bestätigt oder ablehnt, bleibt das
//! Overlay gesperrt. „Schlachtfeld ansehen“ klappt es bis auf einen Knopf ein, damit man das Brett sieht; ein neues
//! Angebot klappt es wieder auf.

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};
use voidfleet_core::protocol::{DraftOffer, DraftPick};

use super::{
    Density, DraftChosen, NewMatch, board_overlay, choice_cancelled, density, overlay_backdrop,
    overlay_hint, overlay_title,
};
use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    game::{Match, ServerError},
    palette,
    settings::Language,
    ui::{
        Fonts,
        button::{self, Key},
        card::{CardAssets, CardHover, HAND_ZOOM, Marked, command_card, ship_card},
        theme::{self, UiOpacity},
    },
};

/// Was im Nachschub gewählt ist.
#[derive(Resource, Default, Debug)]
struct DraftChoice {
    /// Das Angebot, zu dem die Wahl gehört; ein neues setzt sie zurück.
    offer: Option<DraftOffer>,
    pick: DraftPick,
    /// Gemeldet, bis der Server bestätigt oder ablehnt.
    pending: bool,
    /// Eingeklappt bis auf den Knopf zum Wiederaufklappen.
    peek: bool,
}

impl DraftChoice {
    /// Je Reihe ist etwas gewählt, sofern es dort etwas gibt, und nichts wartet.
    fn is_complete(&self) -> bool {
        self.offer.as_ref().is_some_and(|offer| {
            !self.pending
                && (offer.ships.is_empty() || self.pick.ship.is_some())
                && (offer.commands.is_empty() || self.pick.command.is_some())
        })
    }
}

/// Das Overlay; gebaut für dieses Angebot, diese Sprache und Fensterhöhe, auf- oder eingeklappt.
#[derive(Component, PartialEq, Debug)]
struct DraftOverlay {
    offer: DraftOffer,
    peek: bool,
    language: Language,
    density: Density,
}

/// Eine Karte des Angebots: Index in seiner Reihe.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum DraftCard {
    Ship(usize),
    Command(usize),
}

#[derive(Component)]
struct ConfirmButton;

/// Eine Reihe des Angebots mit Überschrift; leer sagt sie, dass es nichts zu wählen gibt.
fn draft_row(fonts: &Fonts, heading: &str, cards: Vec<impl Bundle>, none: &str) -> impl Bundle {
    let empty = cards
        .is_empty()
        .then(|| {
            (
                Node {
                    padding: UiRect::vertical(px(40)),
                    ..default()
                },
                theme::text(fonts.body(16.0), none, palette::TEXT_DIM),
                Pickable::IGNORE,
            )
        })
        .into_iter();
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
        Children::spawn((
            Spawn((
                Node {
                    margin: UiRect::top(px(4)),
                    ..default()
                },
                theme::caps(
                    fonts.display_bold(16.0),
                    heading,
                    theme::TRACKING,
                    palette::TEXT_DIM,
                ),
                Pickable::IGNORE,
            )),
            Spawn((
                Node {
                    align_items: AlignItems::FlexStart,
                    column_gap: px(18),
                    ..default()
                },
                Pickable::IGNORE,
                Children::spawn((SpawnIter(cards.into_iter()), SpawnIter(empty))),
            )),
        )),
    )
}

/// Baut das Overlay, wenn sich Angebot, Einklappen, Sprache oder Fensterhöhe ändern, und nimmt es weg, sobald gewählt
/// ist.
fn show(
    mut commands: Commands,
    game: Res<Match>,
    mut choice: ResMut<DraftChoice>,
    assets: CardAssets,
    window: Query<&Window, With<PrimaryWindow>>,
    overlays: Query<(Entity, &DraftOverlay)>,
) {
    let offer = game.draft.clone().filter(|_| game.result.is_none());
    if offer != choice.offer {
        *choice = DraftChoice {
            offer: offer.clone(),
            ..default()
        };
    }
    let wanted = offer.map(|offer| DraftOverlay {
        offer,
        peek: choice.peek,
        language: assets.locale.language,
        density: density(&window),
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
    let fonts = &assets.fonts;
    let locale = &assets.locale;
    let peek_button = (
        button::text_button(
            fonts.display_bold(18.0),
            if overlay.peek {
                locale.draft_show()
            } else {
                locale.draft_peek()
            },
            Key::NORMAL,
            Node {
                height: px(52),
                padding: UiRect::horizontal(px(20)),
                ..default()
            },
        ),
        observe(|_: On<Activate>, mut choice: ResMut<DraftChoice>| choice.peek = !choice.peek),
    );
    if overlay.peek {
        // Eingeklappt bleibt nur der Knopf über der Hand; das Brett darüber ist frei.
        commands.spawn((
            Name::new("Nachschub (eingeklappt)"),
            overlay,
            Node {
                position_type: PositionType::Absolute,
                top: px(60),
                left: Val::ZERO,
                right: Val::ZERO,
                bottom: px(290),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::bottom(px(12)),
                ..default()
            },
            GlobalZIndex(super::OVERLAY_LAYER),
            Pickable::IGNORE,
            children![peek_button],
        ));
        return;
    }
    let ships = overlay
        .offer
        .ships
        .iter()
        .enumerate()
        .map(|(index, &ship)| {
            (
                ship_card(&assets, ship, HAND_ZOOM),
                DraftCard::Ship(index),
                CardHover::Raise,
                UiOpacity(1.0),
            )
        })
        .collect();
    let commands_row = overlay
        .offer
        .commands
        .iter()
        .enumerate()
        .map(|(index, &command)| {
            (
                command_card(&assets, command, HAND_ZOOM),
                DraftCard::Command(index),
                CardHover::Raise,
                UiOpacity(1.0),
            )
        })
        .collect();
    let density = overlay.density;
    commands.spawn((
        Name::new("Nachschub"),
        overlay,
        board_overlay(),
        children![
            overlay_backdrop(),
            overlay_title(fonts, locale.draft_title(), density),
            overlay_hint(fonts, locale.draft_hint(), density),
            (
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    row_gap: px(4),
                    column_gap: px(48),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    draft_row(fonts, locale.draft_ships(), ships, locale.draft_none()),
                    draft_row(
                        fonts,
                        locale.draft_commands(),
                        commands_row,
                        locale.draft_none()
                    ),
                ],
            ),
            (
                Node {
                    column_gap: px(16),
                    margin: UiRect::top(px(10)),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    peek_button,
                    (
                        ConfirmButton,
                        button::text_button(
                            fonts.display_bold(22.0),
                            locale.draft_confirm(),
                            Key::PRIMARY,
                            Node {
                                width: px(240),
                                height: px(52),
                                ..default()
                            },
                        ),
                        observe(confirm),
                    ),
                ],
            ),
        ],
    ));
}

/// Ein Klick wählt eine Karte bzw. nimmt die Wahl zurück; solange eine Wahl wartet, nicht.
fn toggle_card(click: On<PointerClick>, cards: Query<&DraftCard>, mut choice: ResMut<DraftChoice>) {
    if click.button != PointerButton::Primary || choice.pending {
        return;
    }
    let Ok(&card) = cards.get(click.entity) else {
        return;
    };
    let toggle = |current: Option<usize>, index: usize| (current != Some(index)).then_some(index);
    match card {
        DraftCard::Ship(index) => choice.pick.ship = toggle(choice.pick.ship, index),
        DraftCard::Command(index) => choice.pick.command = toggle(choice.pick.command, index),
    }
}

/// „Übernehmen“ meldet die Wahl und sperrt das Overlay bis zur Antwort des Servers.
fn confirm(
    _: On<Activate>,
    mut choice: ResMut<DraftChoice>,
    mut chosen: MessageWriter<DraftChosen>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if choice.is_complete() {
        choice.pending = true;
        chosen.write(DraftChosen(choice.pick));
        sounds.write(PlayUiSound(UiSound::Confirm));
    }
}

/// Hebt die wartende Wahl auf, wenn der Server ablehnt oder ein neues Match beginnt; ein neues Angebot setzt sie in
/// [`show`] ohnehin zurück.
fn settle(
    mut choice: ResMut<DraftChoice>,
    mut errors: MessageReader<ServerError>,
    mut new_match: MessageReader<NewMatch>,
) {
    if choice_cancelled(&mut errors, &mut new_match) {
        choice.pending = false;
    }
}

/// Rahmen um die gewählten Karten, gesperrte verblassen; „Übernehmen“ ist frei, sobald die Wahl vollständig ist.
fn mark_cards(
    mut commands: Commands,
    choice: Res<DraftChoice>,
    mut cards: Query<(Entity, &DraftCard, Has<Marked>, &mut UiOpacity)>,
    confirm: Query<(Entity, Has<InteractionDisabled>), With<ConfirmButton>>,
) {
    for (entity, &card, was, mut opacity) in &mut cards {
        let selected = match card {
            DraftCard::Ship(index) => choice.pick.ship == Some(index),
            DraftCard::Command(index) => choice.pick.command == Some(index),
        };
        if selected != was {
            if selected {
                commands.entity(entity).insert(Marked);
            } else {
                commands.entity(entity).remove::<Marked>();
            }
        }
        let locked = choice.pending && !selected;
        opacity.set_if_neq(UiOpacity(if locked { 0.45 } else { 1.0 }));
    }
    let complete = choice.is_complete();
    for (button, disabled) in &confirm {
        if disabled == complete {
            button::set_enabled(&mut commands, button, complete);
        }
    }
}

pub(super) struct DraftSelectPlugin;

impl Plugin for DraftSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DraftChoice>()
            .add_observer(toggle_card)
            .add_systems(
                Update,
                (settle, show, mark_cards)
                    .chain()
                    .run_if(in_state(AppState::Match)),
            );
    }
}
