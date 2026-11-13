//! Die Ansicht des Karteneditors ([`Screen::Editor`], Einstellungen → Dev) über [`MapEditor`]. Zwei Zeilen über die
//! ganze Breite: die Karte je Sektor mit ihrer Datei (speichern, verwerfen) und rechts „Schließen“; darunter die Karte
//! („Reihe +“) und rechts das Probespiel (Admiral, „Probespielen“). Links die Karte wie im Spiel mit einem „+“ je
//! Reihe, rechts der gewählte Knoten: Titel und „Knoten löschen“, die Art, die Kanten zur nächsten Reihe als Tasten,
//! dann in zwei Schritten der Verband — erst das Ziel (ein Feld der zwei Reihen, Verstärkung oder Nachrücker), dann die
//! Einheit aus der Palette. Steht im gewählten Feld eine Einheit, folgen ihre Werte (Hülle, ATK, Panzerung mit −/+; was
//! von `enemies_defaults.json` abweicht, leuchtet) und ihr Ablauf: die Schritte in Folge, je mit −/+ für ihren Wert und
//! „x“, darunter die Schritte zum Anhängen. Linksklick wählt einen Knoten, Rechtsklick auf einen Knoten der nächsten
//! Reihe setzt oder löscht die Kante dorthin. Eine Zeile unter den Tasten nennt, was an der Karte nicht stimmt; der
//! Knoten dazu trägt einen roten Ring.
//!
//! Jede Änderung am [`MapEditor`] baut die Ansicht neu.

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};
use voidfleet_core::{
    admirals::ADMIRAL_IDS,
    battle::Encounter,
    board::Row,
    data::Step,
    enemies::{EnemyId, Unit, action},
    run::{MapError, NodeKind, SECTORS, SectorMap},
};

use super::{
    OVERLAY_LAYER,
    run_nodes::Enabled,
    star_map::{
        EdgeState, KINDS, current_ring, edge_node, kind_look, layout, node_size, spawn_glyph,
    },
};
use crate::{
    assets::AppState,
    editor::{Edit, MapEditor, Note, Stat, Target},
    game::{Battle, LeaveBattle},
    i18n::Locale,
    palette,
    run::{ActiveRun, RunView, Screen, time_seed},
    settings::Language,
    ui::{
        Fonts, UiIcons,
        button::{self, Key, Lit},
        theme,
        tooltip::Tooltip,
    },
};

/// Breite des Felds mit dem gewählten Knoten: drei Spalten, in die der längste Name einer Einheit passt.
const PANEL: f32 = 640.0;
/// Platz rechts neben der Karte für das „+“ je Reihe.
const GUTTER: f32 = 44.0;

/// Der Editor; gebaut für diese Sprache und Fenstergröße.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct EditorOverlay(Language, Vec2);

/// Die Wurzel unter der Kopfzeile. Der Inhalt hängt oben, damit nichts springt, wenn das Feld des Knotens wächst.
// shortcut: ohne Rollen; ein Verband mit vielen Verstärkungen ragt in einem kleinen Fenster unten hinaus.
pub(super) fn root() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: px(64),
            left: Val::ZERO,
            right: Val::ZERO,
            bottom: Val::ZERO,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(8),
            padding: UiRect::all(px(12)),
            ..default()
        },
        GlobalZIndex(OVERLAY_LAYER),
    )
}

/// Eine Zeile aus Tasten und Text, die bei Platzmangel umbricht; ein [`gap`] schiebt, was folgt, an den rechten Rand.
pub(super) fn bar(width: Val) -> impl Bundle {
    (
        Node {
            width,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(8),
            row_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Der freie Platz einer Zeile.
pub(super) fn gap() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Ein Raster aus gleich breiten Tasten.
pub(super) fn grid(columns: Vec<RepeatedGridTrack>) -> impl Bundle {
    (
        Node {
            display: Display::Grid,
            grid_template_columns: columns,
            align_items: AlignItems::Center,
            column_gap: px(8),
            row_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
    )
}

pub(super) fn label(fonts: &Fonts, text: impl Into<String>, color: Color) -> impl Bundle {
    (theme::text(fonts.body(15.0), text, color), Pickable::IGNORE)
}

/// Die Überschrift einer Gruppe von Tasten.
pub(super) fn caption(fonts: &Fonts, text: &str) -> impl Bundle {
    (
        theme::caps(
            fonts.display_bold(12.0),
            text,
            theme::TRACKING,
            palette::TEXT_DIM,
        ),
        Pickable::IGNORE,
    )
}

/// Eine Taste des Editors: führt beim Klick `edit` aus; `lit` leuchtet (die gewählte von mehreren).
fn key(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    look: Key,
    lit: bool,
    enabled: bool,
    edit: Edit,
) {
    let node = Node {
        padding: UiRect::axes(px(12), px(6)),
        ..default()
    };
    let mut key = parent.spawn((
        button::text_button(fonts.display_bold(14.0), label, look, node),
        Enabled(enabled),
        observe(move |_: On<Activate>, mut editor: ResMut<MapEditor>| {
            editor.apply(edit.clone());
        }),
    ));
    if lit {
        key.insert(Lit);
    }
}

/// Der Knoten, den ein Fehler der Prüfung meint.
fn error_node(error: MapError) -> Option<(usize, usize)> {
    match error {
        MapError::Edges(row, column)
        | MapError::Unreachable(row, column)
        | MapError::Encounter(row, column) => Some((row, column)),
        MapError::Boss | MapError::EmptyRow(_) => None,
    }
}

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    editor: Option<Res<MapEditor>>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    locale: Res<Locale>,
    window: Single<&Window, With<PrimaryWindow>>,
    shown: Query<(Entity, &EditorOverlay)>,
) {
    let wanted = (*screen == Screen::Editor).then(|| EditorOverlay(locale.language, window.size()));
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
        commands.insert_resource(MapEditor::open(crate::asset_root().join("data/maps")));
        return;
    };
    let (fonts, icons, locale, editor) = (&*fonts, &*icons, &*locale, &*editor);
    let check = editor.map.check();
    let height = (overlay.1.y - 64.0 - 150.0).max(360.0);
    let width = px(size_x(overlay.1, height) + GUTTER + 16.0 + PANEL);
    let size = Vec2::new(size_x(overlay.1, height), height);
    let (status, color) = match (&check, &editor.note) {
        (Err(error), _) => (locale.editor_error(*error), palette::LOST),
        (Ok(()), Some(Note::Failed(error))) => (locale.editor_failed(error), palette::LOST),
        (Ok(()), Some(Note::Saved(file))) => (locale.editor_saved(file), palette::HEAL),
        (Ok(()), None) => (locale.editor_valid().to_owned(), palette::TEXT_DIM),
    };
    commands
        .spawn((Name::new("Karteneditor"), overlay, root()))
        .with_children(|root| {
            // Deckt das Brett ab und fängt die Klicks daneben.
            root.spawn((
                theme::overlay(),
                BackgroundColor(palette::BACKGROUND.with_alpha(0.9)),
            ));
            root.spawn(bar(width))
                .with_children(|bar| file_bar(bar, fonts, locale, editor, check.is_ok()));
            root.spawn(bar(width))
                .with_children(|bar| map_bar(bar, fonts, locale, editor, check.is_ok()));
            root.spawn(bar(width))
                .with_child(label(fonts, status, color));
            root.spawn((
                Node {
                    column_gap: px(16),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|main| {
                main.spawn(frame(Node {
                    width: px(size.x + GUTTER),
                    height: px(size.y),
                    flex_shrink: 0.0,
                    ..default()
                }))
                .with_children(|board| {
                    map_board(
                        board,
                        fonts,
                        icons,
                        locale,
                        editor,
                        size,
                        check.err().and_then(error_node),
                    );
                });
                main.spawn(frame(Node {
                    width: px(PANEL),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    padding: UiRect::all(px(14)),
                    ..default()
                }))
                .with_children(|panel| node_panel(panel, fonts, icons, locale, editor));
            });
        });
}

/// Die Breite der Karte: vier Fünftel der Höhe, schmaler, wenn das Fenster es verlangt.
fn size_x(window: Vec2, height: f32) -> f32 {
    (height * 0.8).min(window.x - 80.0 - PANEL - GUTTER)
}

/// Ein gerahmtes Feld wie die Karte im Spiel.
pub(super) fn frame(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..node
        },
        BackgroundColor(theme::rgba(8, 12, 20, 0.7)),
        BorderColor::all(palette::PLAYER.with_alpha(0.25)),
        Pickable::IGNORE,
    )
}

/// Titel, die Karte je Sektor (mit Ungespeichertem bleibt die Karte offen, bis gespeichert oder verworfen ist), ihre
/// Datei, speichern, verwerfen und rechts schließen.
fn file_bar(
    bar: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &MapEditor,
    valid: bool,
) {
    let dirty = editor.dirty();
    let name = SectorMap::file(editor.sector);
    let name = if dirty {
        locale.editor_unsaved(&name)
    } else {
        name
    };
    bar.spawn((
        theme::caps(
            fonts.display_bold(26.0),
            locale.editor_title(),
            0.04,
            theme::ACCENT,
        ),
        Node {
            margin: UiRect::right(px(12)),
            ..default()
        },
        Pickable::IGNORE,
    ));
    for sector in 1..=SECTORS {
        let name = locale.editor_sector(sector);
        let lit = editor.sector == sector;
        key(
            bar,
            fonts,
            &name,
            Key::NORMAL,
            lit,
            !dirty,
            Edit::Sector(sector),
        );
    }
    // Feste Breite: Die Tasten dahinter bleiben stehen, wenn „ungespeichert“ dazukommt.
    bar.spawn((
        label(fonts, name, palette::TEXT),
        TextLayout::justify(Justify::Center),
        Node {
            width: px(220),
            ..default()
        },
    ));
    let save = locale.editor_save();
    key(
        bar,
        fonts,
        save,
        Key::PRIMARY,
        false,
        valid && dirty,
        Edit::Save,
    );
    let discard = locale.editor_discard();
    key(
        bar,
        fonts,
        discard,
        Key::DANGER,
        false,
        dirty,
        Edit::Discard,
    );
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
        observe(|_: On<Activate>, mut view: ResMut<RunView>| view.editor = false),
    ));
}

/// Links die Karte („Reihe +“), rechts das Probespiel: Admiral und „Probespielen“ (mit gewähltem Knoten auf einer
/// gültigen Karte).
fn map_bar(
    bar: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &MapEditor,
    valid: bool,
) {
    bar.spawn(caption(fonts, locale.editor_map()));
    let add_row = locale.editor_add_row();
    key(bar, fonts, add_row, Key::NORMAL, false, true, Edit::AddRow);
    bar.spawn(gap());
    bar.spawn(caption(fonts, locale.editor_admiral()));
    for admiral in ADMIRAL_IDS {
        let look = Key::tinted(palette::admiral_color(admiral));
        let lit = editor.admiral == admiral;
        let name = locale.admiral(admiral).name;
        key(bar, fonts, name, look, lit, true, Edit::Admiral(admiral));
    }
    bar.spawn((
        button::text_button(
            fonts.display_bold(14.0),
            locale.editor_play(),
            Key::PRIMARY,
            Node {
                padding: UiRect::axes(px(12), px(6)),
                ..default()
            },
        ),
        Enabled(valid && editor.selected.is_some()),
        observe(play),
    ));
}

/// Beginnt das Probespiel am gewählten Knoten; ein laufendes Gefecht endet dafür. Der Run davor bleibt gespeichert.
fn play(
    _: On<Activate>,
    mut commands: Commands,
    editor: Res<MapEditor>,
    battle: Option<Res<Battle>>,
    mut view: ResMut<RunView>,
    mut leave: MessageWriter<LeaveBattle>,
) {
    if let Some(run) = editor.test_run(time_seed()) {
        // Nur ein altes Gefecht abräumen: Ohne eines träfe die Meldung das neue, das im selben Bild beginnt.
        if battle.is_some() {
            leave.write(LeaveBattle);
        }
        commands.insert_resource(ActiveRun(run));
        *view = RunView {
            testing: true,
            ..RunView::default()
        };
    }
}

/// Die Karte: Kanten (die des gewählten Knotens leuchten), Knoten und rechts je Reihe außer der des Bosses ein „+“.
fn map_board(
    board: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    icons: &UiIcons,
    locale: &Locale,
    editor: &MapEditor,
    size: Vec2,
    wrong: Option<(usize, usize)>,
) {
    let map = layout(&editor.map.rows, size);
    for edge in &map.edges {
        let state = if editor.selected == Some(edge.from) {
            EdgeState::Path
        } else {
            EdgeState::Open
        };
        board.spawn(edge_node(edge, state));
    }
    for (row, nodes) in editor.map.rows.iter().enumerate() {
        for (column, node) in nodes.iter().enumerate() {
            let center = map.nodes[row][column];
            let selected = editor.selected == Some((row, column));
            if selected {
                board.spawn(current_ring(center, node.kind));
            }
            if wrong == Some((row, column)) {
                board
                    .spawn(current_ring(center, node.kind))
                    .insert(BorderColor::all(palette::LOST));
            }
            let size = node_size(node.kind);
            let color = kind_look(node.kind, icons).0;
            let units: Vec<&str> = (node.encounter.iter())
                .flat_map(Encounter::units)
                .map(|(unit, _)| locale.enemy(unit.id))
                .collect();
            let mut key = board.spawn((
                button::button(
                    Key::tinted(color),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(center.x - size / 2.0),
                        top: px(center.y - size / 2.0),
                        width: px(size),
                        height: px(size),
                        ..default()
                    },
                ),
                Tooltip(format!(
                    "{}: {}",
                    locale.node_kind(node.kind),
                    if units.is_empty() {
                        locale.node_kind_hint(node.kind).to_owned()
                    } else {
                        units.join(", ")
                    }
                )),
                // Die Taste meldet jeden Klick gleich; welche Maustaste es war, weiß nur das Picking.
                observe(
                    move |click: On<PointerClick>, mut editor: ResMut<MapEditor>| match click.button
                    {
                        PointerButton::Primary => editor.apply(Edit::Select(row, column)),
                        PointerButton::Secondary => editor.apply(Edit::Edge(row, column)),
                        PointerButton::Middle => {}
                    },
                ),
            ));
            if selected {
                key.insert(Lit);
            }
            key.with_children(|key| spawn_glyph(key, fonts, icons, node.kind, size, color));
        }
        if row + 1 < editor.map.rows.len() {
            let y = map.nodes[row].first().map_or(0.0, |node| node.y);
            board.spawn((
                button::text_button(
                    fonts.display_bold(16.0),
                    "+",
                    Key::NORMAL,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(size.x + 4.0),
                        top: px(y - 15.0),
                        width: px(30),
                        height: px(30),
                        ..default()
                    },
                ),
                Tooltip(locale.editor_add_node().to_owned()),
                observe(move |_: On<Activate>, mut editor: ResMut<MapEditor>| {
                    editor.apply(Edit::AddNode(row));
                }),
            ));
        }
    }
}

/// Der gewählte Knoten: Titel und löschen, die Art, und an einem Gefecht der Verband in zwei Schritten — das Ziel (Feld,
/// Verstärkung, Nachrücker), dann die Einheit.
fn node_panel(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    icons: &UiIcons,
    locale: &Locale,
    editor: &MapEditor,
) {
    let (Some((row, column)), Some(node)) = (editor.selected, editor.node()) else {
        panel.spawn(label(fonts, locale.editor_no_node(), palette::TEXT));
        panel.spawn(label(fonts, locale.editor_hint(), palette::TEXT_DIM));
        return;
    };
    // Die letzte Reihe ist der Boss und bleibt es.
    let last = row + 1 == editor.map.rows.len();
    panel.spawn(bar(Val::Auto)).with_children(|head| {
        let title = locale.editor_node(row + 1, column + 1, locale.node_kind(node.kind));
        head.spawn(label(fonts, title, palette::TEXT));
        head.spawn(gap());
        let delete = locale.editor_delete_node();
        key(head, fonts, delete, Key::DANGER, false, !last, Edit::Delete);
    });
    // Eine Zeile: Jede Art ist so breit wie ihr Name, der Rest verteilt sich.
    panel
        .spawn(grid(RepeatedGridTrack::auto(5)))
        .with_children(|kinds| {
            for kind in KINDS
                .into_iter()
                .filter(|&kind| (kind == NodeKind::Boss) == last)
            {
                let look = Key::tinted(kind_look(kind, icons).0);
                let lit = node.kind == kind;
                key(
                    kinds,
                    fonts,
                    locale.node_kind(kind),
                    look,
                    lit,
                    true,
                    Edit::Kind(kind),
                );
            }
        });
    // Die Kanten zur nächsten Reihe: je Knoten dort eine Taste in der Farbe seiner Art; sie leuchtet, wenn die Kante
    // steht. Dasselbe wie der Rechtsklick auf der Karte, nur sichtbar.
    if let Some(next) = editor.map.rows.get(row + 1) {
        panel.spawn(bar(Val::Auto)).with_children(|edges| {
            edges.spawn(caption(fonts, &locale.editor_edges(row + 2)));
            for (to, target) in next.iter().enumerate() {
                let look = Key::tinted(kind_look(target.kind, icons).0);
                let lit = node.next.contains(&to);
                let edge = Edit::Edge(row + 1, to);
                key(edges, fonts, &(to + 1).to_string(), look, lit, true, edge);
            }
        });
    }
    let Some(encounter) = &node.encounter else {
        panel.spawn(label(
            fonts,
            locale.node_kind_hint(node.kind),
            palette::TEXT_DIM,
        ));
        return;
    };
    panel.spawn((
        caption(fonts, locale.editor_target()),
        Node {
            margin: UiRect::top(px(6)),
            ..default()
        },
    ));
    // Wie auf dem Brett vom Flaggschiff aus: hinten die Back-Reihe, davor die Front.
    panel
        .spawn(grid(vec![
            GridTrack::px(48.0),
            RepeatedGridTrack::fr(3, 1.0),
        ]))
        .with_children(|slots| {
            for (side, row) in [(Row::Back, &encounter.back), (Row::Front, &encounter.front)] {
                slots.spawn(label(fonts, locale.row_name(side), palette::TEXT_DIM));
                for (slot, unit) in row.iter().enumerate() {
                    let target = Target::Slot(side, slot);
                    let name = unit
                        .as_ref()
                        .map_or(locale.editor_empty().to_owned(), |unit| {
                            marked_name(locale, unit)
                        });
                    let look = unit.as_ref().map_or(Key::NORMAL, |unit| unit_look(unit.id));
                    key(
                        slots,
                        fonts,
                        &name,
                        look,
                        editor.target == target,
                        true,
                        Edit::Target(target),
                    );
                }
            }
        });
    // Die zwei Listen nebeneinander: oben die Taste, die die Liste zum Ziel macht, darunter ihre Einträge.
    panel
        .spawn((
            Node {
                align_items: AlignItems::FlexStart,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|lists| {
            lists.spawn(list()).with_children(|list| {
                let target = Target::Reinforcement;
                let lit = editor.target == target;
                let name = locale.editor_reinforcements();
                key(
                    list,
                    fonts,
                    name,
                    Key::NORMAL,
                    lit,
                    true,
                    Edit::Target(target),
                );
                for (index, entry) in encounter.reinforcements.iter().enumerate() {
                    list.spawn(entry_line()).with_children(|line| {
                        let target = Target::ReinforcementEntry(index);
                        entry_key(line, fonts, locale, editor, &entry.unit, target);
                        let earlier = entry.round > 1;
                        let round = Edit::Round(index, -1);
                        key(line, fonts, "-", Key::NORMAL, false, earlier, round);
                        line.spawn(label(fonts, locale.hud_round(entry.round), palette::TEXT));
                        let round = Edit::Round(index, 1);
                        key(line, fonts, "+", Key::NORMAL, false, true, round);
                        let drop = Edit::DropReinforcement(index);
                        key(line, fonts, "x", Key::DANGER, false, true, drop);
                    });
                }
            });
            lists.spawn(list()).with_children(|list| {
                let target = Target::Replacement;
                let lit = editor.target == target;
                let name = locale.editor_replacements();
                key(
                    list,
                    fonts,
                    name,
                    Key::NORMAL,
                    lit,
                    true,
                    Edit::Target(target),
                );
                for (index, unit) in encounter.replacements.iter().enumerate() {
                    list.spawn(entry_line()).with_children(|line| {
                        let target = Target::ReplacementEntry(index);
                        entry_key(line, fonts, locale, editor, unit, target);
                        let drop = Edit::DropReplacement(index);
                        key(line, fonts, "x", Key::DANGER, false, true, drop);
                    });
                }
            });
        });
    panel.spawn((
        caption(fonts, locale.editor_unit()),
        Node {
            margin: UiRect::top(px(6)),
            ..default()
        },
    ));
    panel
        .spawn(grid(RepeatedGridTrack::fr(3, 1.0)))
        .with_children(|units| {
            for unit in EnemyId::ALL {
                let name = locale.enemy(unit);
                key(
                    units,
                    fonts,
                    name,
                    unit_look(unit),
                    false,
                    true,
                    Edit::Place(Some(unit)),
                );
            }
            let empty = locale.editor_empty();
            key(
                units,
                fonts,
                empty,
                Key::NORMAL,
                false,
                true,
                Edit::Place(None),
            );
        });
    if let Some(unit) = editor.unit() {
        unit_panel(panel, fonts, locale, editor, unit);
    }
}

/// Der Name einer Einheit; ein Stern, wenn ihre Werte oder ihr Muster von der Vorlage in `enemies_defaults.json`
/// abweichen.
fn marked_name(locale: &Locale, unit: &Unit) -> String {
    let mark = if *unit == Unit::from(unit.id) {
        ""
    } else {
        "*"
    };
    format!("{}{mark}", locale.enemy(unit.id))
}

/// Ein Eintrag einer Liste als Taste: Sie macht ihn zum Ziel, und er leuchtet, solange er es ist.
fn entry_key(
    line: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &MapEditor,
    unit: &Unit,
    target: Target,
) {
    let name = marked_name(locale, unit);
    let lit = editor.target == target;
    key(
        line,
        fonts,
        &name,
        unit_look(unit.id),
        lit,
        true,
        Edit::Target(target),
    );
}

/// Die gezeigte Einheit (im Ziel oder von dort aus gerufen): der Weg dorthin mit „Zurück“ und „Zurücksetzen“, ihre
/// Werte mit −/+ (was von der Vorlage in `enemies_defaults.json` abweicht, leuchtet), ihr Ablauf als Folge von
/// Schritten (je −/+ für den Wert, am Ruf „Werte“ für die gerufene Einheit, und „x“; der letzte bleibt) und die
/// Schritte zum Anhängen.
fn unit_panel(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    locale: &Locale,
    editor: &MapEditor,
    unit: &Unit,
) {
    let base = Unit::from(unit.id);
    panel.spawn(bar(Val::Auto)).with_children(|head| {
        head.spawn(caption(fonts, locale.editor_values()));
        if !editor.calls.is_empty() {
            let names: Vec<&str> = (editor.chain().into_iter())
                .map(|unit| locale.enemy(unit.id))
                .collect();
            head.spawn(label(fonts, names.join(" › "), palette::TEXT));
            let back = locale.editor_back();
            key(head, fonts, back, Key::NORMAL, false, true, Edit::CloseCall);
        }
        head.spawn(gap());
        let reset = locale.editor_reset();
        key(
            head,
            fonts,
            reset,
            Key::DANGER,
            false,
            *unit != base,
            Edit::ResetUnit,
        );
    });
    panel.spawn(bar(Val::Auto)).with_children(|stats| {
        for (stat, name, value, base, steps) in [
            (
                Stat::Hull,
                locale.stats_hull(),
                unit.hull,
                base.hull,
                &[10, 1][..],
            ),
            (
                Stat::Attack,
                locale.stats_atk(),
                unit.attack,
                base.attack,
                &[1],
            ),
            (
                Stat::Armor,
                locale.editor_armor(),
                unit.armor,
                base.armor,
                &[1],
            ),
        ] {
            stats.spawn(entry_line()).with_children(|line| {
                line.spawn(label(fonts, name, palette::TEXT_DIM));
                for &by in steps {
                    let edit = Edit::Stat(stat, -by);
                    key(
                        line,
                        fonts,
                        &format!("-{by}"),
                        Key::NORMAL,
                        false,
                        true,
                        edit,
                    );
                }
                let color = if value == base {
                    palette::TEXT
                } else {
                    theme::ACCENT
                };
                line.spawn(label(fonts, value.to_string(), color));
                for &by in steps.iter().rev() {
                    let edit = Edit::Stat(stat, by);
                    key(
                        line,
                        fonts,
                        &format!("+{by}"),
                        Key::NORMAL,
                        false,
                        true,
                        edit,
                    );
                }
            });
        }
    });
    let pattern = &unit.pattern;
    panel.spawn(bar(Val::Auto)).with_children(|steps| {
        for (index, step) in pattern.iter().enumerate() {
            if index > 0 {
                steps.spawn(label(fonts, "→", palette::TEXT_DIM));
            }
            steps.spawn(entry_line()).with_children(|line| {
                let valued = matches!(step, Step::ColumnFire(_) | Step::Shields(_) | Step::Call(_));
                if valued {
                    let edit = Edit::StepValue(index, -1);
                    key(line, fonts, "-", Key::NORMAL, false, true, edit);
                }
                line.spawn(label(fonts, step_name(locale, step), palette::TEXT));
                if valued {
                    let edit = Edit::StepValue(index, 1);
                    key(line, fonts, "+", Key::NORMAL, false, true, edit);
                }
                if let Step::Call(called) = step {
                    let open = Edit::OpenCall(index);
                    let look = unit_look(called.id);
                    key(
                        line,
                        fonts,
                        locale.editor_open_call(),
                        look,
                        false,
                        true,
                        open,
                    );
                }
                let drop = Edit::DropStep(index);
                key(
                    line,
                    fonts,
                    "x",
                    Key::DANGER,
                    false,
                    pattern.len() > 1,
                    drop,
                );
            });
        }
    });
    panel.spawn(caption(fonts, locale.editor_add_step()));
    // Werte stellt danach −/+ am Schritt.
    let steps = [
        Step::Fire,
        Step::ColumnFire(1),
        Step::Ram,
        Step::Shields(6),
        Step::Evade,
        Step::Charge,
        Step::TargetLock,
        Step::Call(EnemyId::Raiders.into()),
        Step::Bolt,
    ];
    panel
        .spawn(grid(RepeatedGridTrack::fr(3, 1.0)))
        .with_children(|keys| {
            for step in steps {
                let name = step_name(locale, &step);
                key(
                    keys,
                    fonts,
                    &name,
                    Key::NORMAL,
                    false,
                    true,
                    Edit::AddStep(step),
                );
            }
        });
}

/// Der Name eines Schritts; Spaltenfeuer mit seiner Breite statt mit Spalten.
fn step_name(locale: &Locale, step: &Step) -> String {
    match step {
        Step::ColumnFire(width) => locale.editor_column_fire(*width),
        step => locale.action(action(std::slice::from_ref(step), 0, 0)),
    }
}

/// Eine der zwei Listen (Verstärkung, Nachrücker): halbe Breite, die Einträge untereinander.
fn list() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            flex_basis: px(0),
            flex_grow: 1.0,
            row_gap: px(6),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Ein Eintrag einer Liste: links die Einheit, rechts seine Tasten.
fn entry_line() -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(4),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Die Taste einer Einheit in der Farbe ihrer Klasse.
fn unit_look(unit: EnemyId) -> Key {
    Key::tinted(palette::enemy_class_color(unit.class()))
}

pub(super) struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show.run_if(in_state(AppState::Match)));
    }
}
