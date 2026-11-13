//! Die Ansicht des Befehlskarten-Editors ([`Screen::Cards`], Einstellungen → Dev) über [`CardEditor`]. Oben Titel,
//! Dateien und „Schließen“, darunter speichern, verwerfen und die Prüfung. Links alle Karten in der Farbe ihrer Art,
//! in der Mitte die gewählte je Sprache als Vorschau (Grundfassung und aufgewertet), rechts ihre Art, ihr Ziel, ihre
//! Zahlen mit −/+ (was die aufgewertete Fassung ändert, leuchtet) und je Sprache Name und Effekttext als Textfelder.
//!
//! Jede Änderung am [`CardEditor`] baut die Ansicht neu, nur das Tippen nicht: Das Textfeld behielte sonst weder Fokus
//! noch Schreibmarke. Es ändert den Editor an der Änderungserkennung vorbei und baut nur Vorschau und Prüfzeile neu
//! ([`Live`]).

use bevy::{
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::{EditableText, TextCursorStyle, TextEditChange},
    ui_widgets::{Activate, TextInput, observe},
    window::PrimaryWindow,
};
use voidfleet_core::cards::{CardTarget, CardType, DeckCard, LOOT_POOL};

use super::{
    map_editor::{bar, caption, frame, gap, grid, label, root},
    run_nodes::Enabled,
};
use crate::{
    assets::AppState,
    card_editor::{CardEdit, CardEditor, Field, TextField},
    data::card_art::CardArt,
    editor::Note,
    i18n::{LANGUAGES, Locale},
    palette,
    run::{RunView, Screen},
    settings::Language,
    ui::{
        Fonts,
        button::{self, Key, Lit},
        card::card_face,
        theme,
    },
};

/// Maßstab der Karten in der Vorschau.
const PREVIEW: f32 = 1.2;
/// Breite des Felds mit Art, Zahlen und Texten.
const PANEL: f32 = 560.0;

/// Der Editor; gebaut für diese Sprache und Fenstergröße.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct CardEditorOverlay(Language, Vec2);

/// Was beim Tippen neu entsteht.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Live {
    /// Speichern, verwerfen und die Prüfung.
    Status,
    Preview,
}

/// Getippt: Vorschau und Prüfzeile neu bauen.
#[derive(Message)]
struct Retyped;

const TYPES: [CardType; 5] = [
    CardType::Attack,
    CardType::Buff,
    CardType::Debuff,
    CardType::Tactic,
    CardType::Doctrine,
];
const TARGETS: [CardTarget; 3] = [CardTarget::None, CardTarget::Enemy, CardTarget::Column];

/// Eine Taste des Editors: führt beim Klick `edit` aus; `lit` leuchtet (die gewählte von mehreren).
fn key(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    look: Key,
    lit: bool,
    enabled: bool,
    edit: CardEdit,
) {
    let node = Node {
        padding: UiRect::axes(px(12), px(6)),
        ..default()
    };
    let mut key = parent.spawn((
        button::text_button(fonts.display_bold(14.0), label, look, node),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut editor: ResMut<CardEditor>| {
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
    editor: Option<Res<CardEditor>>,
    fonts: Res<Fonts>,
    art: Res<CardArt>,
    locale: Res<Locale>,
    window: Single<&Window, With<PrimaryWindow>>,
    shown: Query<(Entity, &CardEditorOverlay)>,
) {
    let wanted =
        (*screen == Screen::Cards).then(|| CardEditorOverlay(locale.language, window.size()));
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
        commands.insert_resource(CardEditor::open(crate::asset_root().join("data")));
        return;
    };
    let (fonts, art, locale, editor) = (&*fonts, &*art, &*locale, &*editor);
    let width = px((overlay.1.x - 32.0).min(1500.0));
    commands
        .spawn((Name::new("Befehlskarten-Editor"), overlay, root()))
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
                        locale.card_editor_title(),
                        0.04,
                        theme::ACCENT,
                    ),
                    Node {
                        margin: UiRect::right(px(12)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                bar.spawn(label(
                    fonts,
                    "cards.json · text/en.json · text/de.json",
                    palette::TEXT_DIM,
                ));
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
                    observe(|_: On<Activate>, mut view: ResMut<RunView>| view.cards = false),
                ));
            });
            root.spawn((bar(width), Live::Status))
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
                    padding: UiRect::all(px(12)),
                    ..default()
                }))
                .with_children(|list| card_list(list, fonts, locale, editor));
                main.spawn((
                    frame(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        padding: UiRect::all(px(14)),
                        ..default()
                    }),
                    Live::Preview,
                ))
                .with_children(|preview| previews(preview, fonts, art, editor));
                main.spawn(frame(Node {
                    width: px(PANEL),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(14)),
                    ..default()
                }))
                .with_children(|panel| card_panel(panel, fonts, locale, editor));
            });
        });
}

/// Speichern (wenn etwas ungespeichert ist und alles stimmt), verwerfen und die Prüfung oder die letzte Meldung.
fn status(bar: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, editor: &CardEditor) {
    let check = editor.check();
    let dirty = editor.dirty();
    let save = locale.editor_save();
    key(
        bar,
        fonts,
        save,
        Key::PRIMARY,
        false,
        dirty && check.is_ok(),
        CardEdit::Save,
    );
    let discard = locale.editor_discard();
    key(
        bar,
        fonts,
        discard,
        Key::DANGER,
        false,
        dirty,
        CardEdit::Discard,
    );
    let (text, color) = match (check, &editor.note) {
        (Err((card, Some(language))), _) => {
            let card = editor.text(language, card, TextField::Name);
            let language = Locale::new(language).language_name();
            let text = locale.card_editor_error_text(card, language);
            (text, palette::LOST)
        }
        (Err((card, None)), _) => {
            let card = editor.text(locale.language, card, TextField::Name);
            (locale.card_editor_error_numbers(card), palette::LOST)
        }
        (Ok(()), Some(Note::Failed(error))) => (locale.editor_failed(error), palette::LOST),
        (Ok(()), Some(Note::Saved(files))) if !dirty => (locale.editor_saved(files), palette::HEAL),
        (Ok(()), _) => (locale.card_editor_valid().to_owned(), palette::TEXT_DIM),
    };
    bar.spawn(label(fonts, text, color));
}

/// Alle Karten in der Farbe ihrer Art, zwei Spalten.
fn card_list(list: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, editor: &CardEditor) {
    list.spawn(grid(vec![RepeatedGridTrack::flex(2, 1.0)]))
        .with_children(|grid| {
            for id in LOOT_POOL {
                let look = Key::tinted(palette::card_type_color(editor.cards[&id].card_type));
                let name = editor.text(locale.language, id, TextField::Name);
                let lit = editor.selected == id;
                key(grid, fonts, name, look, lit, true, CardEdit::Select(id));
            }
        });
}

/// Je Sprache die gewählte Karte, links die Grundfassung, rechts aufgewertet.
fn previews(preview: &mut ChildSpawnerCommands, fonts: &Fonts, art: &CardArt, editor: &CardEditor) {
    for language in LANGUAGES {
        let locale = Locale::new(language);
        preview.spawn(caption(fonts, locale.language_name()));
        preview
            .spawn((
                Node {
                    column_gap: px(16),
                    padding: UiRect::all(px(8)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|cards| {
                for upgraded in [false, true] {
                    let card = DeckCard {
                        id: editor.selected,
                        upgraded,
                    };
                    let face = editor.face(language, upgraded);
                    cards.spawn(card_face(fonts, art, &locale, card, face, 0, PREVIEW));
                }
            });
    }
}

/// Art, Ziel, Zahlen und Texte der gewählten Karte.
fn card_panel(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &CardEditor,
) {
    let card = editor.card();
    panel.spawn(caption(fonts, locale.card_editor_type()));
    panel.spawn(bar(percent(100))).with_children(|bar| {
        for card_type in TYPES {
            let look = Key::tinted(palette::card_type_color(card_type));
            let name = locale.card_type(card_type);
            let lit = card.card_type == card_type;
            key(bar, fonts, name, look, lit, true, CardEdit::Type(card_type));
        }
    });
    panel.spawn(caption(fonts, locale.card_editor_target()));
    panel.spawn(bar(percent(100))).with_children(|bar| {
        for target in TARGETS {
            let name = locale.card_target(target);
            let lit = card.target == target;
            key(
                bar,
                fonts,
                name,
                Key::NORMAL,
                lit,
                true,
                CardEdit::Target(target),
            );
        }
    });
    panel.spawn(caption(fonts, locale.card_editor_numbers()));
    let upgraded = locale.card_editor_upgraded();
    let cost = card.cost as i32;
    let upgraded_cost = card.upgraded.cost.map(|cost| cost as i32);
    let rows = [
        (
            locale.card_editor_cost(),
            (Field::Cost, cost),
            Some((Field::UpgradedCost, upgraded_cost)),
        ),
        (
            locale.card_editor_value(),
            (Field::Value, card.value),
            Some((Field::UpgradedValue, card.upgraded.value)),
        ),
    ];
    let extra = card
        .extra
        .map(|extra| (locale.card_editor_extra(), (Field::Extra, extra), None));
    panel
        .spawn(grid(vec![
            RepeatedGridTrack::auto(1),
            RepeatedGridTrack::auto(1),
            RepeatedGridTrack::auto(1),
        ]))
        .with_children(|grid| {
            for (name, (field, value), upgrade) in rows.into_iter().chain(extra) {
                grid.spawn(label(fonts, name, palette::TEXT));
                stepper(grid, fonts, None, field, value, false);
                match upgrade {
                    Some((field, changed)) => {
                        let shown = changed.unwrap_or(value);
                        stepper(grid, fonts, Some(upgraded), field, shown, changed.is_some());
                    }
                    None => {
                        grid.spawn(gap());
                    }
                }
            }
        });
    for language in LANGUAGES {
        let name = Locale::new(language).language_name();
        panel.spawn(caption(fonts, name));
        for (field, title, lines) in [
            (TextField::Name, locale.card_editor_name(), 1.0),
            (TextField::Effect, locale.card_editor_effect(), 3.0),
        ] {
            panel.spawn(label(fonts, title, palette::TEXT_DIM));
            let text = editor.text(language, editor.selected, field);
            panel.spawn(text_box(fonts, text, lines, language, field));
        }
    }
    panel.spawn((
        theme::text(
            fonts.body(13.0),
            locale.card_editor_hint(),
            palette::TEXT_DIM,
        ),
        Pickable::IGNORE,
    ));
}

/// Eine Zahl mit −/+, davor ein Titel; `lit` leuchtet, wenn die aufgewertete Fassung sie ändert.
fn stepper(
    grid: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    title: Option<&str>,
    field: Field,
    value: i32,
    lit: bool,
) {
    grid.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|row| {
        if let Some(title) = title {
            row.spawn(label(fonts, title, palette::TEXT_DIM));
        }
        key(
            row,
            fonts,
            "-",
            Key::NORMAL,
            false,
            value > 0,
            CardEdit::Number(field, -1),
        );
        let color = if lit { palette::BUFF } else { palette::TEXT };
        row.spawn((
            label(fonts, value.to_string(), color),
            TextLayout::justify(Justify::Center),
            Node {
                width: px(28),
                ..default()
            },
        ));
        key(
            row,
            fonts,
            "+",
            Key::NORMAL,
            false,
            true,
            CardEdit::Number(field, 1),
        );
    });
}

/// Ein Textfeld für Name oder Effekt der gewählten Karte in einer Sprache.
fn text_box(
    fonts: &Fonts,
    text: &str,
    lines: f32,
    language: Language,
    field: TextField,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            padding: UiRect::axes(px(8), px(5)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        TextInput,
        EditableText {
            visible_lines: Some(lines),
            ..EditableText::new(text)
        },
        TextLayout {
            linebreak: LineBreak::WordBoundary,
            ..default()
        },
        fonts.body(15.0),
        TextColor(palette::TEXT),
        TextCursorStyle {
            color: palette::TEXT,
            selection_color: palette::PLAYER.with_alpha(0.35),
            ..default()
        },
        BackgroundColor(theme::rgba(4, 8, 14, 0.9)),
        BorderColor::all(palette::PLAYER.with_alpha(0.4)),
        TabIndex(0),
        observe(
            move |change: On<TextEditChange>,
                  inputs: Query<&EditableText>,
                  mut editor: ResMut<CardEditor>,
                  mut retyped: MessageWriter<Retyped>| {
                let Ok(input) = inputs.get(change.event_target()) else {
                    return;
                };
                let text = input.value().to_string();
                // Auch eine bewegte Schreibmarke meldet sich.
                if editor.text(language, editor.selected, field) != text {
                    let edit = CardEdit::Text(language, field, text);
                    editor.bypass_change_detection().apply(edit);
                    retyped.write(Retyped);
                }
            },
        ),
    )
}

/// Baut nach dem Tippen Vorschau und Prüfzeile neu.
fn refresh(
    mut retyped: MessageReader<Retyped>,
    mut commands: Commands,
    editor: Option<Res<CardEditor>>,
    fonts: Res<Fonts>,
    art: Res<CardArt>,
    locale: Res<Locale>,
    live: Query<(Entity, &Live)>,
) {
    let (true, Some(editor)) = (retyped.read().count() > 0, editor) else {
        return;
    };
    for (entity, part) in &live {
        commands
            .entity(entity)
            .despawn_children()
            .with_children(|parent| match part {
                Live::Status => status(parent, &fonts, &locale, &editor),
                Live::Preview => previews(parent, &fonts, &art, &editor),
            });
    }
}

pub(super) struct CardEditorPlugin;

impl Plugin for CardEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Retyped>().add_systems(
            Update,
            (show, refresh).chain().run_if(in_state(AppState::Match)),
        );
    }
}
