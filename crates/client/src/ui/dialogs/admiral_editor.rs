//! Die Ansicht des Admirals-Editors ([`Screen::AdmiralEditor`], Einstellungen → Dev) über [`AdmiralEditor`]. Oben
//! Titel, Datei und „Schließen“, darunter speichern, verwerfen und die Prüfung. Links die vier Admirale in ihrer
//! Farbe, in der Mitte der gewählte als Karte der Admiralswahl mit den Werten des Editors, rechts Hülle und ATK des
//! Flaggschiffs und die Schild-Station mit −/+, das Startdeck je Karte mit Anzahl (ein Klick auf den Namen macht sie
//! zur Kennkarte, sie leuchtet) und alle Karten zum Hinzufügen. Jede Änderung baut die Ansicht neu.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};
use voidfleet_core::{
    admirals::ADMIRAL_IDS,
    cards::{DeckCard, LOOT_POOL},
};

use super::{
    Density,
    admiral_select::admiral_card,
    map_editor::{bar, caption, frame, gap, grid, label, root},
    run_nodes::Enabled,
};
use crate::{
    admiral_editor::{AdmiralEdit, AdmiralEditor, Field},
    assets::AppState,
    data::{admirals::AdmiralPortraits, card_art::CardArt},
    editor::Note,
    i18n::Locale,
    palette,
    run::{RunView, Screen},
    settings::Language,
    ui::{
        Fonts, UiIcons,
        button::{self, Key, Lit},
        theme,
    },
};

/// Breite des Felds mit Zahlen und Startdeck.
const PANEL: f32 = 620.0;

/// Der Editor; gebaut für diese Sprache und Fenstergröße.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct AdmiralEditorOverlay(Language, Vec2);

/// Eine Taste des Editors: führt beim Klick `edit` aus; `lit` leuchtet (die gewählte von mehreren).
fn key(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    look: Key,
    lit: bool,
    enabled: bool,
    edit: AdmiralEdit,
) {
    let node = Node {
        padding: UiRect::axes(px(12), px(6)),
        ..default()
    };
    let mut key = parent.spawn((
        button::text_button(fonts.display_bold(14.0), label, look, node),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut editor: ResMut<AdmiralEditor>| {
            editor.apply(edit.clone());
        }),
    ));
    if lit {
        key.insert(Lit);
    }
}

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    editor: Option<Res<AdmiralEditor>>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    portraits: Res<AdmiralPortraits>,
    art: Res<CardArt>,
    locale: Res<Locale>,
    window: Single<&Window, With<PrimaryWindow>>,
    shown: Query<(Entity, &AdmiralEditorOverlay)>,
) {
    let wanted = (*screen == Screen::AdmiralEditor)
        .then(|| AdmiralEditorOverlay(locale.language, window.size()));
    let changed = editor.as_ref().is_some_and(|editor| editor.is_changed());
    if !changed && shown.iter().map(|(_, overlay)| *overlay).eq(wanted) {
        return;
    }
    for (entity, _) in &shown {
        commands.entity(entity).despawn();
    }
    let Some(overlay) = wanted else {
        return;
    };
    let Some(editor) = editor else {
        // Beim ersten Öffnen; das nächste Bild baut die Ansicht.
        commands.insert_resource(AdmiralEditor::open(crate::asset_root().join("data")));
        return;
    };
    let (fonts, locale, editor) = (&*fonts, &*locale, &*editor);
    let width = px((overlay.1.x - 32.0).min(1500.0));
    commands
        .spawn((Name::new("Admirals-Editor"), overlay, root()))
        .with_children(|root| {
            // Deckt das Brett ab und fängt die Klicks daneben.
            root.spawn((
                theme::overlay(),
                BackgroundColor(palette::BACKGROUND.with_alpha(0.9)),
            ));
            root.spawn(bar(width)).with_children(|bar| {
                bar.spawn((
                    theme::caps(
                        fonts.display_bold(26.0),
                        locale.admiral_editor_title(),
                        0.04,
                        theme::ACCENT,
                    ),
                    Node {
                        margin: UiRect::right(px(12)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                bar.spawn(label(fonts, "assets/data/admirals.json", palette::TEXT_DIM));
                bar.spawn(gap());
                bar.spawn((
                    button::text_button(
                        fonts.display_bold(14.0),
                        locale.dialog_close(),
                        Key::NORMAL,
                        Node {
                            padding: UiRect::axes(px(12), px(6)),
                            ..default()
                        },
                    ),
                    observe(|_: On<Activate>, mut view: ResMut<RunView>| {
                        view.admiral_editor = false;
                    }),
                ));
            });
            root.spawn(bar(width))
                .with_children(|bar| status(bar, fonts, locale, editor));
            root.spawn((
                Node {
                    column_gap: px(16),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|main| {
                main.spawn(frame(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(12)),
                    ..default()
                }))
                .with_children(|list| {
                    for id in ADMIRAL_IDS {
                        let look = Key::tinted(palette::admiral_color(id));
                        let name = locale.admiral(id).name;
                        let lit = editor.selected == id;
                        key(list, fonts, name, look, lit, true, AdmiralEdit::Select(id));
                    }
                });
                main.spawn(admiral_card(
                    fonts,
                    &icons,
                    &portraits,
                    &art,
                    locale,
                    editor.selected,
                    editor.admiral(),
                    Density::Regular,
                ));
                main.spawn(frame(Node {
                    width: px(PANEL),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(14)),
                    ..default()
                }))
                .with_children(|panel| admiral_panel(panel, fonts, locale, editor));
            });
        });
}

/// Speichern (wenn etwas ungespeichert ist und alles stimmt), verwerfen und die Prüfung oder die letzte Meldung.
fn status(bar: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, editor: &AdmiralEditor) {
    let check = editor.check();
    let dirty = editor.dirty();
    let save = locale.editor_save();
    let can_save = dirty && check.is_ok();
    key(
        bar,
        fonts,
        save,
        Key::PRIMARY,
        false,
        can_save,
        AdmiralEdit::Save,
    );
    let discard = locale.editor_discard();
    key(
        bar,
        fonts,
        discard,
        Key::DANGER,
        false,
        dirty,
        AdmiralEdit::Discard,
    );
    let (text, color) = match (check, &editor.note) {
        (Err(admiral), _) => {
            let name = locale.admiral(admiral).name;
            (locale.admiral_editor_error(name), palette::LOST)
        }
        (Ok(()), Some(Note::Failed(error))) => (locale.editor_failed(error), palette::LOST),
        (Ok(()), Some(Note::Saved(file))) if !dirty => (locale.editor_saved(file), palette::HEAL),
        (Ok(()), _) => (locale.admiral_editor_valid().to_owned(), palette::TEXT_DIM),
    };
    bar.spawn(label(fonts, text, color));
}

/// Zahlen, Startdeck und Kennkarte des gewählten Admirals, darunter alle Karten zum Hinzufügen.
fn admiral_panel(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &AdmiralEditor,
) {
    let admiral = editor.admiral();
    let name = |id| locale.card(DeckCard::new(id)).name;
    panel.spawn(caption(fonts, locale.admiral_editor_numbers()));
    panel.spawn(bar(percent(100))).with_children(|bar| {
        for (title, field, value) in [
            (locale.stats_hull(), Field::Hull, admiral.hull),
            (locale.stats_atk(), Field::Attack, admiral.attack),
            (
                locale.admiral_editor_shields(),
                Field::ShieldStation,
                admiral.shield_station,
            ),
        ] {
            bar.spawn(label(fonts, title, palette::TEXT_DIM));
            stepper(
                bar,
                fonts,
                value.to_string(),
                value > 0,
                AdmiralEdit::Number(field, -1),
                AdmiralEdit::Number(field, 1),
            );
        }
    });
    panel.spawn(caption(fonts, locale.admiral_editor_deck()));
    // Je Karte eine Zeile, in der Reihenfolge des Decks.
    let mut cards = admiral.deck.clone();
    cards.dedup();
    panel
        .spawn(grid(vec![RepeatedGridTrack::flex(2, 1.0)]))
        .with_children(|grid| {
            for card in cards {
                let count = admiral.deck.iter().filter(|&&c| c == card).count();
                grid.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    let look = Key::tinted(palette::card_type_color(card.card_type()));
                    let lit = admiral.signature_card == card;
                    key(
                        row,
                        fonts,
                        &name(card),
                        look,
                        lit,
                        true,
                        AdmiralEdit::Signature(card),
                    );
                    row.spawn(gap());
                    stepper(
                        row,
                        fonts,
                        count.to_string(),
                        true,
                        AdmiralEdit::RemoveCard(card),
                        AdmiralEdit::AddCard(card),
                    );
                });
            }
        });
    panel.spawn(caption(fonts, locale.admiral_editor_add()));
    panel
        .spawn(grid(vec![RepeatedGridTrack::flex(3, 1.0)]))
        .with_children(|grid| {
            for card in LOOT_POOL {
                let look = Key::tinted(palette::card_type_color(card.card_type()));
                key(
                    grid,
                    fonts,
                    &name(card),
                    look,
                    false,
                    true,
                    AdmiralEdit::AddCard(card),
                );
            }
        });
    panel.spawn((
        theme::text(
            fonts.body(13.0),
            locale.admiral_editor_hint(),
            palette::TEXT_DIM,
        ),
        Pickable::IGNORE,
    ));
}

/// Eine Zahl mit −/+.
fn stepper(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    value: String,
    can_lower: bool,
    lower: AdmiralEdit,
    raise: AdmiralEdit,
) {
    parent
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            key(row, fonts, "-", Key::NORMAL, false, can_lower, lower);
            row.spawn((
                label(fonts, value, palette::TEXT),
                TextLayout::justify(Justify::Center),
                Node {
                    width: px(28),
                    ..default()
                },
            ));
            key(row, fonts, "+", Key::NORMAL, false, true, raise);
        });
}

pub(super) struct AdmiralEditorPlugin;

impl Plugin for AdmiralEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show.run_if(in_state(AppState::Match)));
    }
}
