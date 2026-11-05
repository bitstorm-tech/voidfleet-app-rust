//! Die Sternenkarte des Sektors als Overlay über dem leeren Brett: Titel „Sektor n: Name“, neun Reihen von unten nach
//! oben, Knoten als Tasten mit Zeichen und Farbe ihrer Art (Tooltip: Art und was dort geschieht), Kanten als dünne,
//! gedrehte Knoten dahinter. Der besuchte Pfad leuchtet, der aktuelle Knoten trägt einen Ring, die wählbaren Knoten
//! (`run.choices()`) pulsieren und melden [`RunAction::Advance`], alle anderen sind gesperrt.
//!
//! Die Lage der Knoten und Kanten rechnet [`layout`] ohne Bevy-Welt (getestet).

use bevy::{
    prelude::*,
    ui_widgets::{Activate, observe},
    window::PrimaryWindow,
};
use voidfleet_core::run::{MapNode, NodeKind, Run, Stage};

use super::{
    overlay_backdrop,
    run_nodes::{Enabled, screen_root},
};
use crate::{
    assets::AppState,
    i18n::Locale,
    palette,
    run::{ActiveRun, RunAction, Screen},
    settings::Language,
    ui::{
        Fonts, UiIcons,
        button::{self, Key, Lit, Pulsing},
        theme,
        tooltip::Tooltip,
    },
};

/// Abstand der äußeren Knoten vom Rand der Karte.
const MARGIN: f32 = 36.0;
const NODE: f32 = 44.0;
const BOSS_NODE: f32 = 62.0;

/// Lage der Knoten (Mitte, UI-Pixel in der Karte, y nach unten) und der Kanten.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLayout {
    pub nodes: Vec<Vec<Vec2>>,
    pub edges: Vec<MapEdge>,
}

/// Eine Kante von (Reihe, Spalte) zur nächsten Reihe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapEdge {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub a: Vec2,
    pub b: Vec2,
}

/// Reihe 0 unten, die letzte oben; die Knoten einer Reihe gleichmäßig über die Breite.
pub fn layout(map: &[Vec<MapNode>], size: Vec2) -> MapLayout {
    let rows = map.len().max(2);
    let step = (size.y - 2.0 * MARGIN) / (rows - 1) as f32;
    let nodes: Vec<Vec<Vec2>> = map
        .iter()
        .enumerate()
        .map(|(row, nodes)| {
            let y = size.y - MARGIN - row as f32 * step;
            (0..nodes.len())
                .map(|column| {
                    let x = MARGIN
                        + (size.x - 2.0 * MARGIN) * (column as f32 + 0.5) / nodes.len() as f32;
                    Vec2::new(x, y)
                })
                .collect()
        })
        .collect();
    let edges = map
        .iter()
        .enumerate()
        .flat_map(|(row, nodes)| {
            nodes.iter().enumerate().flat_map(move |(column, node)| {
                node.next.iter().map(move |&next| (row, column, next))
            })
        })
        .map(|(row, column, next)| MapEdge {
            from: (row, column),
            to: (row + 1, next),
            a: nodes[row][column],
            b: nodes[row + 1][next],
        })
        .collect();
    MapLayout { nodes, edges }
}

/// Wie ein Knoten gerade steht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    Current,
    Visited,
    Choice,
    Locked,
}

pub fn node_state(run: &Run, row: usize, column: usize) -> NodeState {
    if run.current() == Some((row, column)) {
        NodeState::Current
    } else if run.visited(row, column) {
        NodeState::Visited
    } else if run.stage == Stage::Map
        && run.result.is_none()
        && row == run.path.len()
        && run.choices().contains(&column)
    {
        NodeState::Choice
    } else {
        NodeState::Locked
    }
}

/// Wie eine Kante gerade steht: auf dem Pfad, offen (vom aktuellen Knoten zu einer Wahl) oder sonst.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeState {
    Path,
    Open,
    Rest,
}

pub fn edge_state(run: &Run, edge: &MapEdge) -> EdgeState {
    let ((row, column), (next_row, next)) = (edge.from, edge.to);
    if run.visited(row, column) && run.visited(next_row, next) {
        EdgeState::Path
    } else if run.current() == Some(edge.from)
        && node_state(run, next_row, next) == NodeState::Choice
    {
        EdgeState::Open
    } else {
        EdgeState::Rest
    }
}

/// Farbe und Zeichen einer Knotenart; Gefecht, Elite und Boss tragen ein Icon.
fn kind_look(kind: NodeKind, icons: &UiIcons) -> (Color, Result<Handle<Image>, &'static str>) {
    match kind {
        NodeKind::Battle => (palette::ENEMY, Ok(icons.crosshair.clone())),
        NodeKind::Elite => (palette::DEBUFF, Ok(icons.crosshair.clone())),
        NodeKind::Boss => (palette::DAMAGE, Ok(icons.crown.clone())),
        NodeKind::Event => (palette::BUFF, Err("?")),
        NodeKind::Shipyard => (palette::HEAL, Err("+")),
        NodeKind::Market => (palette::FLAGSHIP_TRIM, Err("$")),
    }
}

/// Die Karte; gebaut für diese Sprache und Größe.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct MapOverlay(Language, Vec2);

/// Größe der Karte im Fenster: so hoch wie möglich, etwas schmaler als hoch.
fn map_size(window: &Window) -> Vec2 {
    let height = (window.height() - 64.0 - 120.0).max(360.0);
    Vec2::new((height * 0.8).min(window.width() - 80.0), height)
}

fn show(
    mut commands: Commands,
    screen: Res<Screen>,
    run: Option<Res<ActiveRun>>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    locale: Res<Locale>,
    window: Single<&Window, With<PrimaryWindow>>,
    shown: Query<(Entity, &MapOverlay)>,
) {
    let wanted = (*screen == Screen::Map).then(|| MapOverlay(locale.language, map_size(&window)));
    let run_changed = run.as_ref().is_some_and(|run| run.is_changed());
    if !run_changed && shown.iter().map(|(_, o)| *o).eq(wanted) {
        return;
    }
    for (entity, _) in &shown {
        commands.entity(entity).despawn();
    }
    let (Some(overlay), Some(run)) = (wanted, run) else {
        return;
    };
    let size = overlay.1;
    let map = layout(&run.map, size);
    commands
        .spawn((Name::new("Sternenkarte"), overlay, screen_root()))
        .with_children(|root| {
            root.spawn(overlay_backdrop());
            root.spawn((
                theme::caps(
                    fonts.display_bold(34.0),
                    &locale.map_title(run.sector),
                    0.04,
                    theme::ACCENT,
                ),
                Pickable::IGNORE,
            ));
            root.spawn((
                theme::text(fonts.body(16.0), locale.map_hint(), palette::TEXT_DIM),
                Pickable::IGNORE,
            ));
            root.spawn((
                Node {
                    width: px(size.x),
                    height: px(size.y),
                    flex_shrink: 0.0,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(theme::rgba(8, 12, 20, 0.7)),
                BorderColor::all(palette::PLAYER.with_alpha(0.25)),
                Pickable::IGNORE,
            ))
            .with_children(|board| {
                for edge in &map.edges {
                    board.spawn(edge_node(edge, edge_state(&run, edge)));
                }
                for (row, nodes) in run.map.iter().enumerate() {
                    for (column, node) in nodes.iter().enumerate() {
                        let center = map.nodes[row][column];
                        let state = node_state(&run, row, column);
                        if state == NodeState::Current {
                            board.spawn(current_ring(center, node.kind));
                        }
                        spawn_node(
                            board, &fonts, &icons, &locale, node.kind, center, state, column,
                        );
                    }
                }
            });
        });
}

fn edge_node(edge: &MapEdge, state: EdgeState) -> impl Bundle {
    let (width, color) = match state {
        EdgeState::Path => (4.0, palette::SELECT.with_alpha(0.9)),
        EdgeState::Open => (3.0, palette::SELECT.with_alpha(0.55)),
        EdgeState::Rest => (2.0, palette::TEXT_DIM.with_alpha(0.18)),
    };
    let delta = edge.b - edge.a;
    let middle = (edge.a + edge.b) / 2.0;
    let length = delta.length();
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(middle.x - length / 2.0),
            top: px(middle.y - width / 2.0),
            width: px(length),
            height: px(width),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(color),
        // Im Uhrzeigersinn, y zeigt nach unten: der Winkel wie in UI-Koordinaten.
        UiTransform::from_rotation(Rot2::radians(delta.y.atan2(delta.x))),
        Pickable::IGNORE,
    )
}

fn node_size(kind: NodeKind) -> f32 {
    if kind == NodeKind::Boss {
        BOSS_NODE
    } else {
        NODE
    }
}

/// Ring um den aktuellen Knoten.
fn current_ring(center: Vec2, kind: NodeKind) -> impl Bundle {
    let size = node_size(kind) + 18.0;
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(center.x - size / 2.0),
            top: px(center.y - size / 2.0),
            width: px(size),
            height: px(size),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(palette::SELECT),
        BoxShadow(vec![theme::glow(palette::SELECT.with_alpha(0.6), 16.0)]),
        Pickable::IGNORE,
    )
}

/// Ein Knoten als Taste in der Farbe seiner Art; nur wählbare Knoten sind frei.
fn spawn_node(
    board: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    icons: &UiIcons,
    locale: &Locale,
    kind: NodeKind,
    center: Vec2,
    state: NodeState,
    column: usize,
) {
    let size = node_size(kind);
    let (color, glyph) = kind_look(kind, icons);
    let tint = if state == NodeState::Locked {
        color.with_alpha(0.4)
    } else {
        color
    };
    let mut node = board.spawn((
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
            locale.node_kind(kind),
            locale.node_kind_hint(kind)
        )),
        Enabled(state == NodeState::Choice),
        observe(move |_: On<Activate>, mut commands: Commands| {
            commands.write_message(RunAction::Advance(column));
        }),
    ));
    match state {
        NodeState::Choice => {
            node.insert(Pulsing);
        }
        NodeState::Current | NodeState::Visited => {
            node.insert(Lit);
        }
        NodeState::Locked => {}
    }
    node.with_children(|key| match glyph {
        Ok(icon) => {
            key.spawn((
                Node {
                    width: px(size * 0.5),
                    height: px(size * 0.5),
                    ..default()
                },
                ImageNode::new(icon).with_color(tint),
                Pickable::IGNORE,
            ));
        }
        Err(text) => {
            key.spawn((
                theme::caps(fonts.display_bold(size * 0.6), text, 0.0, tint),
                Pickable::IGNORE,
            ));
        }
    });
}

pub(super) struct StarMapPlugin;

impl Plugin for StarMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show.run_if(in_state(AppState::Match)));
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::admirals::AdmiralId;

    use super::*;

    /// Reihe 0 liegt unten, der Boss oben in der Mitte; jede Kante verbindet benachbarte Reihen und zeigt nach oben.
    #[test]
    fn nodes_bottom_to_top() {
        let run = Run::new(AdmiralId::Kerr, 11);
        let size = Vec2::new(600.0, 800.0);
        let map = layout(&run.map, size);
        assert_eq!(map.nodes.len(), voidfleet_core::run::ROWS);
        assert_eq!(map.nodes[0][0].y, size.y - MARGIN);
        assert_eq!(map.nodes[8], vec![Vec2::new(size.x / 2.0, MARGIN)]);
        for row in &map.nodes {
            assert!(row.windows(2).all(|pair| pair[0].x < pair[1].x));
            assert!(row.iter().all(|p| p.x > 0.0 && p.x < size.x));
        }
        let count: usize = run.map.iter().flatten().map(|node| node.next.len()).sum();
        assert_eq!(map.edges.len(), count);
        for edge in &map.edges {
            assert_eq!(edge.to.0, edge.from.0 + 1);
            assert!(edge.b.y < edge.a.y);
            assert_eq!(edge.a, map.nodes[edge.from.0][edge.from.1]);
        }
    }

    /// Vor Reihe 1 ist die ganze erste Reihe wählbar; danach leuchtet der Pfad, die Kanten zu den Wahlen sind offen.
    #[test]
    fn states_follow_path() {
        let mut run = Run::new(AdmiralId::Osei, 5);
        assert!((0..run.map[0].len()).all(|c| node_state(&run, 0, c) == NodeState::Choice));
        assert_eq!(node_state(&run, 1, 0), NodeState::Locked);
        let column = run.choices()[0];
        run.advance(column).unwrap();
        assert_eq!(node_state(&run, 0, column), NodeState::Current);
        // Im Gefecht ist nichts wählbar.
        assert!(
            run.choices()
                .iter()
                .all(|&c| node_state(&run, 1, c) == NodeState::Locked)
        );
        run.stage = Stage::Map;
        let next = run.choices();
        assert!(
            next.iter()
                .all(|&c| node_state(&run, 1, c) == NodeState::Choice)
        );
        let map = layout(&run.map, Vec2::splat(500.0));
        for edge in &map.edges {
            let open = edge.from == (0, column);
            assert_eq!(edge_state(&run, edge) == EdgeState::Open, open);
        }
        run.advance(next[0]).unwrap();
        run.stage = Stage::Map;
        let path = map
            .edges
            .iter()
            .filter(|edge| edge_state(&run, edge) == EdgeState::Path)
            .count();
        assert_eq!(path, 1);
        assert_eq!(node_state(&run, 0, column), NodeState::Visited);
    }
}
