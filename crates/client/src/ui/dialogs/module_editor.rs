//! Die Ansicht des Modul-Editors ([`Screen::ModuleEditor`], Einstellungen → Dev) über [`ModuleEditor`]. Oben Titel,
//! Datei und „Schließen“, darunter speichern, verwerfen und die letzte Meldung. Links die neun Module, in der Mitte
//! das gewählte als Tafel wie in Beute und Schwarzmarkt mit den Zahlen des Editors, rechts seine Zahl mit −/+ und
//! ihr Pfad in `rules.json`. Jede Änderung baut die Ansicht neu.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};
use voidfleet_core::run::MODULES;

use super::{
    map_editor::{bar, caption, frame, gap, label, root},
    run_nodes::{Enabled, plate},
};
use crate::{
    assets::AppState,
    editor::Note,
    i18n::Locale,
    module_editor::{ModuleEdit, ModuleEditor, number},
    palette,
    run::{RunView, Screen},
    settings::Language,
    ui::{
        Fonts,
        button::{self, Key, Lit},
        theme,
    },
};

/// Breite des Felds mit der Zahl.
const PANEL: f32 = 420.0;

/// Der Editor; gebaut für diese Sprache und Fenstergröße.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct ModuleEditorOverlay(Language, Vec2);

/// Eine Taste des Editors: führt beim Klick `edit` aus; `lit` leuchtet (die gewählte von mehreren).
fn key(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    look: Key,
    lit: bool,
    enabled: bool,
    edit: ModuleEdit,
) {
    let node = Node {
        padding: UiRect::axes(px(12), px(6)),
        ..default()
    };
    let mut key = parent.spawn((
        button::text_button(fonts.display_bold(14.0), label, look, node),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut editor: ResMut<ModuleEditor>| {
            editor.apply(edit);
        }),
    ));
    if lit {
        key.insert(Lit);
    }
}

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    editor: Option<Res<ModuleEditor>>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    window: Single<&Window, With<PrimaryWindow>>,
    shown: Query<(Entity, &ModuleEditorOverlay)>,
) {
    let wanted = (*screen == Screen::ModuleEditor)
        .then(|| ModuleEditorOverlay(locale.language, window.size()));
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
        commands.insert_resource(ModuleEditor::open(crate::asset_root().join("data")));
        return;
    };
    let (fonts, locale, editor) = (&*fonts, &*locale, &*editor);
    let width = px((overlay.1.x - 32.0).min(1500.0));
    commands
        .spawn((Name::new("Modul-Editor"), overlay, root()))
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
                        locale.module_editor_title(),
                        0.04,
                        theme::ACCENT,
                    ),
                    Node {
                        margin: UiRect::right(px(12)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                bar.spawn(label(fonts, "assets/data/rules.json", palette::TEXT_DIM));
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
                        view.module_editor = false;
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
                    for module in MODULES {
                        let name = locale.module(module).0;
                        let lit = editor.selected == module;
                        let edit = ModuleEdit::Select(module);
                        key(list, fonts, name, Key::NORMAL, lit, true, edit);
                    }
                });
                let name = locale.module(editor.selected).0;
                let text = locale.module_text_with(editor.selected, &editor.rules);
                main.spawn(plate(fonts, name, &text));
                main.spawn(frame(Node {
                    width: px(PANEL),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(14)),
                    ..default()
                }))
                .with_children(|panel| module_panel(panel, fonts, locale, editor));
            });
        });
}

/// Speichern (wenn etwas ungespeichert ist), verwerfen und die letzte Meldung.
fn status(bar: &mut ChildSpawnerCommands, fonts: &Fonts, locale: &Locale, editor: &ModuleEditor) {
    let dirty = editor.dirty();
    let save = locale.editor_save();
    key(
        bar,
        fonts,
        save,
        Key::PRIMARY,
        false,
        dirty,
        ModuleEdit::Save,
    );
    let discard = locale.editor_discard();
    let edit = ModuleEdit::Discard;
    key(bar, fonts, discard, Key::DANGER, false, dirty, edit);
    let (text, color) = match &editor.note {
        Some(Note::Failed(error)) => (locale.editor_failed(error), palette::LOST),
        Some(Note::Saved(file)) if !dirty => (locale.editor_saved(file), palette::HEAL),
        _ => (locale.module_editor_valid().to_owned(), palette::TEXT_DIM),
    };
    bar.spawn(label(fonts, text, color));
}

/// Die Zahl des gewählten Moduls mit −/+ und ihr Pfad in `rules.json`, oder dass es keine hat.
fn module_panel(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &ModuleEditor,
) {
    panel.spawn(caption(fonts, locale.module_editor_number()));
    match (editor.value(), number(editor.selected)) {
        (Some(value), Some(path)) => {
            panel.spawn(bar(percent(100))).with_children(|row| {
                key(
                    row,
                    fonts,
                    "-",
                    Key::NORMAL,
                    false,
                    value > 0,
                    ModuleEdit::Number(-1),
                );
                row.spawn((
                    label(fonts, value.to_string(), palette::TEXT),
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
                    ModuleEdit::Number(1),
                );
                row.spawn(label(
                    fonts,
                    path.replace('/', " › ").trim_start_matches(" › "),
                    palette::TEXT_DIM,
                ));
            });
        }
        _ => {
            panel.spawn(label(fonts, locale.module_editor_none(), palette::TEXT_DIM));
        }
    }
    panel.spawn((
        theme::text(
            fonts.body(13.0),
            locale.module_editor_hint(),
            palette::TEXT_DIM,
        ),
        Pickable::IGNORE,
    ));
}

pub(super) struct ModuleEditorPlugin;

impl Plugin for ModuleEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show.run_if(in_state(AppState::Match)));
    }
}
