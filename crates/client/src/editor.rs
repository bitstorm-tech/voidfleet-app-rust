//! Der Karteneditor ohne Oberfläche (Einstellungen → Dev, [`Screen::Editor`](crate::run::Screen)): eine Sternenkarte
//! aus `assets/data/maps/` laden, Schritt für Schritt bearbeiten ([`Edit`]), prüfen ([`SectorMap::check`]), im Format
//! der Dateien schreiben ([`map_json`]) und einen Knoten probespielen ([`MapEditor::test_run`]). Die Ansicht steht
//! in `ui/dialogs/map_editor.rs`; sie legt die Ressource [`MapEditor`] beim ersten Öffnen an.

use std::{io, path::PathBuf};

use bevy::prelude::*;
use serde::Serialize;
use voidfleet_core::{
    admirals::AdmiralId,
    battle::{Encounter, Reinforcement},
    board::Row,
    enemies::EnemyId,
    run::{MapNode, NodeKind, Run, SECTORS, SectorMap},
};

/// Wohin die nächste Einheit aus der Palette geht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Slot(Row, usize),
    Reinforcement,
    Replacement,
}

/// Ein Schritt im Editor. Knoten sind (Reihe, Spalte); was einen Knoten ändert, meint den gewählten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Select(usize, usize),
    /// Setzt oder löscht die Kante vom gewählten Knoten zu diesem, wenn er in der nächsten Reihe steht.
    Edge(usize, usize),
    /// Ein neuer Knoten am Ende dieser Reihe.
    AddNode(usize),
    /// Eine neue Reihe vor dem Boss.
    AddRow,
    Delete,
    Kind(NodeKind),
    Sector(u32),
    Target(Target),
    /// Die Einheit der Palette (`None`: leer) geht an das Ziel ([`Target`]).
    Place(Option<EnemyId>),
    /// Verschiebt die Runde der Verstärkung mit diesem Index.
    Round(usize, i32),
    DropReinforcement(usize),
    DropReplacement(usize),
    Admiral(AdmiralId),
    /// Blättert durch die Dateien.
    File(isize),
    New,
    Save,
    Discard,
}

/// Die letzte Meldung über eine Datei.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Saved(String),
    Failed(String),
}

#[derive(Resource, Debug)]
pub struct MapEditor {
    dir: PathBuf,
    /// Die Kartendateien des Ordners, nach Namen sortiert.
    pub files: Vec<String>,
    /// Die offene Datei; `None` ist eine neue Karte, die ihren Namen beim ersten Speichern bekommt.
    pub file: Option<String>,
    pub map: SectorMap,
    /// Der Stand der Datei, zum Verwerfen und um Ungespeichertes zu erkennen.
    saved: SectorMap,
    pub selected: Option<(usize, usize)>,
    pub target: Target,
    /// Mit diesem Admiral und seinem Startdeck läuft das Probespiel.
    pub admiral: AdmiralId,
    pub note: Option<Note>,
}

fn encounter(unit: EnemyId) -> Encounter {
    Encounter {
        front: [None, Some(unit), None],
        ..default()
    }
}

fn battle_node(next: Vec<usize>) -> MapNode {
    MapNode {
        kind: NodeKind::Battle,
        next,
        encounter: Some(encounter(EnemyId::Raiders)),
    }
}

/// Die kleinste gültige Karte: ein Gefecht, dann der Boss.
fn blank() -> SectorMap {
    let boss = MapNode {
        kind: NodeKind::Boss,
        next: Vec::new(),
        encounter: Some(encounter(EnemyId::ScrapKing)),
    };
    SectorMap {
        sector: 1,
        rows: vec![vec![battle_node(vec![0])], vec![boss]],
    }
}

impl MapEditor {
    /// Öffnet die erste Karte des Ordners, ohne Dateien eine neue.
    pub fn open(dir: PathBuf) -> Self {
        let mut editor = Self {
            dir,
            files: Vec::new(),
            file: None,
            map: blank(),
            saved: blank(),
            selected: None,
            target: Target::Slot(Row::Front, 1),
            admiral: AdmiralId::Osei,
            note: None,
        };
        editor.list();
        if let Some(name) = editor.files.first().cloned() {
            editor.load(name);
        }
        editor
    }

    /// Ungespeicherte Änderungen; solange es sie gibt, wechselt der Editor die Karte nicht.
    pub fn dirty(&self) -> bool {
        self.map != self.saved
    }

    pub fn node(&self) -> Option<&MapNode> {
        let (row, column) = self.selected?;
        self.map.rows.get(row)?.get(column)
    }

    fn node_mut(&mut self) -> Option<&mut MapNode> {
        let (row, column) = self.selected?;
        self.map.rows.get_mut(row)?.get_mut(column)
    }

    pub fn apply(&mut self, edit: Edit) {
        self.note = None;
        match edit {
            Edit::Select(row, column) => self.selected = Some((row, column)),
            Edit::Edge(row, column) => {
                if let Some((from, _)) = self.selected
                    && row == from + 1
                    && let Some(node) = self.node_mut()
                {
                    match node.next.binary_search(&column) {
                        Ok(at) => {
                            node.next.remove(at);
                        }
                        Err(at) => node.next.insert(at, column),
                    }
                }
            }
            // Der neue Knoten hängt am letzten der Reihe davor und führt zum letzten der Reihe danach: Die Karte
            // bleibt gültig.
            Edit::AddNode(row) => {
                let column = self.map.rows[row].len();
                let last = (self.map.rows.get(row + 1)).and_then(|row| row.len().checked_sub(1));
                let next = last.into_iter().collect();
                self.map.rows[row].push(battle_node(next));
                if let Some(last) =
                    (row.checked_sub(1)).and_then(|row| self.map.rows[row].last_mut())
                {
                    last.next.push(column);
                }
                self.selected = Some((row, column));
            }
            // Was zum Boss führte, führt jetzt zum einen Knoten der neuen Reihe.
            Edit::AddRow => {
                let row = self.map.rows.len().saturating_sub(1);
                self.map.rows.insert(row, vec![battle_node(vec![0])]);
                self.selected = Some((row, 0));
            }
            Edit::Delete => self.delete(),
            Edit::Kind(kind) => {
                if let Some(node) = self.node_mut() {
                    node.kind = kind;
                    if !matches!(kind, NodeKind::Battle | NodeKind::Elite | NodeKind::Boss) {
                        node.encounter = None;
                    } else if node.encounter.is_none() {
                        node.encounter = Some(encounter(EnemyId::Raiders));
                    }
                }
            }
            Edit::Sector(sector) => self.map.sector = sector,
            Edit::Target(target) => self.target = target,
            Edit::Place(unit) => {
                let target = self.target;
                let Some(encounter) = self.node_mut().and_then(|node| node.encounter.as_mut())
                else {
                    return;
                };
                match (target, unit) {
                    (Target::Slot(Row::Front, column), unit) => encounter.front[column] = unit,
                    (Target::Slot(Row::Back, column), unit) => encounter.back[column] = unit,
                    (Target::Reinforcement, Some(unit)) => {
                        encounter
                            .reinforcements
                            .push(Reinforcement { round: 3, unit });
                    }
                    (Target::Replacement, Some(unit)) => encounter.replacements.push(unit),
                    (_, None) => {}
                }
            }
            Edit::Round(index, by) => {
                if let Some(entry) = (self.node_mut())
                    .and_then(|node| node.encounter.as_mut()?.reinforcements.get_mut(index))
                {
                    entry.round = entry.round.saturating_add_signed(by).max(1);
                }
            }
            Edit::DropReinforcement(index) => {
                if let Some(encounter) = self.node_mut().and_then(|node| node.encounter.as_mut())
                    && index < encounter.reinforcements.len()
                {
                    encounter.reinforcements.remove(index);
                }
            }
            Edit::DropReplacement(index) => {
                if let Some(encounter) = self.node_mut().and_then(|node| node.encounter.as_mut())
                    && index < encounter.replacements.len()
                {
                    encounter.replacements.remove(index);
                }
            }
            Edit::Admiral(admiral) => self.admiral = admiral,
            Edit::File(step) => {
                if self.dirty() || self.files.is_empty() {
                    return;
                }
                let at = (self.file.as_ref())
                    .and_then(|file| self.files.iter().position(|name| name == file));
                let next = at.map_or(0, |at| {
                    (at as isize + step).rem_euclid(self.files.len() as isize) as usize
                });
                self.load(self.files[next].clone());
            }
            Edit::New => {
                if !self.dirty() {
                    (self.map, self.saved, self.file, self.selected) =
                        (blank(), blank(), None, None);
                }
            }
            Edit::Save => self.save(),
            Edit::Discard => (self.map, self.selected) = (self.saved.clone(), None),
        }
    }

    /// Löscht den gewählten Knoten (nie den Boss) und zieht die Kanten der Reihe davor nach. Mit dem letzten Knoten
    /// fällt die Reihe; was zu ihm führte, führt dann dorthin, wohin er führte.
    fn delete(&mut self) {
        let Some((row, column)) = self.selected else {
            return;
        };
        if row + 1 >= self.map.rows.len() || column >= self.map.rows[row].len() {
            return;
        }
        self.selected = None;
        let removed = self.map.rows[row].remove(column);
        let gone = self.map.rows[row].is_empty();
        if gone {
            self.map.rows.remove(row);
        }
        let Some(before) = row.checked_sub(1) else {
            return;
        };
        for node in &mut self.map.rows[before] {
            if gone {
                node.next = removed.next.clone();
                continue;
            }
            node.next.retain(|&next| next != column);
            for next in node.next.iter_mut().filter(|next| **next > column) {
                *next -= 1;
            }
        }
    }

    fn list(&mut self) {
        let entries = std::fs::read_dir(&self.dir).into_iter().flatten().flatten();
        self.files = entries
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".json"))
            .collect();
        self.files.sort();
    }

    /// Lädt eine Karte, auch eine, die die Prüfung nicht besteht; unlesbar bleibt die bisherige stehen.
    fn load(&mut self, name: String) {
        let read = std::fs::read_to_string(self.dir.join(&name))
            .map_err(|error| error.to_string())
            .and_then(|json| {
                serde_json::from_str::<SectorMap>(&json).map_err(|error| error.to_string())
            });
        match read {
            Ok(map) => {
                (self.saved, self.map) = (map.clone(), map);
                (self.file, self.selected, self.note) = (Some(name), None, None);
            }
            Err(error) => self.note = Some(Note::Failed(format!("{name}: {error}"))),
        }
    }

    /// Schreibt die Karte, wenn sie stimmt; eine neue heißt `sector<n>_<x>.json` mit dem ersten freien Buchstaben,
    /// danach mit Zahlen.
    fn save(&mut self) {
        if self.map.check().is_err() {
            return;
        }
        self.list();
        let name = self.file.clone().unwrap_or_else(|| {
            let letters = ('a'..='z').map(String::from);
            letters
                .chain((1..).map(|number: u32| number.to_string()))
                .map(|tag| format!("sector{}_{tag}.json", self.map.sector))
                .find(|name| !self.files.contains(name))
                .expect("die Zahlen enden nicht")
        });
        self.note = Some(
            match std::fs::write(self.dir.join(&name), map_json(&self.map)) {
                Ok(()) => {
                    self.saved = self.map.clone();
                    self.file = Some(name.clone());
                    self.list();
                    Note::Saved(name)
                }
                Err(error) => Note::Failed(format!("{name}: {error}")),
            },
        );
    }

    /// Ein Run, der am gewählten Knoten steht (über irgendeinen Weg dorthin), mit dem Startdeck des Admirals, im
    /// Sektor der Karte. `None` ohne Wahl oder solange die Karte nicht stimmt.
    pub fn test_run(&self, seed: u64) -> Option<Run> {
        let (row, column) = self.selected?;
        self.map.check().ok()?;
        let rows = &self.map.rows;
        let maps: Vec<SectorMap> = (1..=SECTORS)
            .map(|sector| SectorMap {
                sector,
                rows: rows.clone(),
            })
            .collect();
        let mut run = Run::new(self.admiral, seed, &maps);
        run.sector = self.map.sector;
        run.upcoming.truncate((SECTORS - run.sector) as usize);
        let mut path = vec![column];
        for before in (0..row).rev() {
            let to = path[path.len() - 1];
            path.push(
                rows[before]
                    .iter()
                    .position(|node| node.next.contains(&to))?,
            );
        }
        path.reverse();
        path.pop();
        run.path = path;
        run.advance(column).ok()?;
        Some(run)
    }
}

/// Die Karte im Format der Dateien (`docs/daten.md`): je Knoten eine Zeile, Listen und Verbände in der Zeile.
pub fn map_json(map: &SectorMap) -> String {
    let rows: Vec<String> = (map.rows.iter())
        .map(|row| {
            let nodes: Vec<String> = row
                .iter()
                .map(|node| format!("      {}", inline(node)))
                .collect();
            format!("    [\n{}\n    ]", nodes.join(",\n"))
        })
        .collect();
    format!(
        "{{\n  \"sector\": {},\n  \"rows\": [\n{}\n  ]\n}}\n",
        map.sector,
        rows.join(",\n")
    )
}

/// JSON in einer Zeile mit Leerzeichen, wie die Dateien es schreiben: `{ "kind": "event", "next": [0, 1] }`.
fn inline(value: &impl Serialize) -> String {
    let mut json = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut json, Inline);
    value.serialize(&mut serializer).expect("Knoten als JSON");
    String::from_utf8(json).expect("JSON ist UTF-8")
}

// shortcut: ein leeres Objekt würde `{ }`, die Karten haben keins.
struct Inline;

impl serde_json::ser::Formatter for Inline {
    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        out: &mut W,
        first: bool,
    ) -> io::Result<()> {
        out.write_all(if first { "" } else { ", " }.as_bytes())
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        out: &mut W,
        first: bool,
    ) -> io::Result<()> {
        out.write_all(if first { " " } else { ", " }.as_bytes())
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, out: &mut W) -> io::Result<()> {
        out.write_all(b": ")
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, out: &mut W) -> io::Result<()> {
        out.write_all(b" }")
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::run::{MapError, Stage};

    use super::*;

    fn maps_dir() -> PathBuf {
        PathBuf::from(crate::REPO_ASSETS).join("data/maps")
    }

    /// Ein Editor ohne Ordner: die leere Karte, nichts wird geschrieben.
    fn blank_editor() -> MapEditor {
        MapEditor::open(PathBuf::from("/kein/ordner"))
    }

    /// Der Schreiber gibt jede Kartendatei Zeichen für Zeichen wieder: Speichern ändert nur, was bearbeitet wurde.
    #[test]
    fn files_keep_their_format() {
        let editor = MapEditor::open(maps_dir());
        assert!(editor.files.len() >= 9);
        for name in &editor.files {
            let json = std::fs::read_to_string(maps_dir().join(name)).unwrap();
            let map: SectorMap = serde_json::from_str(&json).unwrap();
            assert_eq!(map_json(&map), json, "{name}");
        }
    }

    /// Neue Knoten und Reihen lassen die Karte gültig; Kanten, Art und Verband folgen den Schritten.
    #[test]
    fn edits() {
        let mut editor = blank_editor();
        assert_eq!(editor.map.check(), Ok(()));
        assert!(!editor.dirty());
        editor.apply(Edit::AddRow);
        editor.apply(Edit::AddNode(1));
        editor.apply(Edit::AddNode(0));
        assert_eq!(editor.map.check(), Ok(()));
        assert!(editor.dirty());
        assert_eq!(editor.selected, Some((0, 1)));
        let lens: Vec<usize> = editor.map.rows.iter().map(Vec::len).collect();
        assert_eq!(lens, [2, 2, 1]);

        // Von (0, 1) führt eine Kante zu (1, 1); die zweite kommt dazu und geht wieder.
        assert_eq!(editor.map.rows[0][1].next, [1]);
        editor.apply(Edit::Edge(1, 0));
        assert_eq!(editor.map.rows[0][1].next, [0, 1]);
        editor.apply(Edit::Edge(1, 0));
        editor.apply(Edit::Edge(2, 0));
        assert_eq!(editor.map.rows[0][1].next, [1]);

        // Ein Ereignis trägt keinen Verband, ein Gefecht bekommt wieder einen.
        editor.apply(Edit::Kind(NodeKind::Event));
        assert_eq!(editor.node().unwrap().encounter, None);
        assert_eq!(editor.map.check(), Ok(()));
        editor.apply(Edit::Kind(NodeKind::Elite));
        editor.apply(Edit::Target(Target::Slot(Row::Back, 1)));
        editor.apply(Edit::Place(Some(EnemyId::Spotter)));
        editor.apply(Edit::Target(Target::Reinforcement));
        editor.apply(Edit::Place(Some(EnemyId::GuardCruiser)));
        editor.apply(Edit::Round(0, -5));
        editor.apply(Edit::Target(Target::Replacement));
        editor.apply(Edit::Place(Some(EnemyId::Fireship)));
        let encounter = editor.node().unwrap().encounter.clone().unwrap();
        assert_eq!(encounter.back[1], Some(EnemyId::Spotter));
        assert_eq!(encounter.reinforcements[0].round, 1);
        assert_eq!(encounter.replacements, [EnemyId::Fireship]);
        editor.apply(Edit::DropReinforcement(0));
        editor.apply(Edit::DropReplacement(0));
        editor.apply(Edit::Target(Target::Slot(Row::Front, 1)));
        editor.apply(Edit::Place(None));
        assert_eq!(editor.map.check(), Err(MapError::Encounter(0, 1)));

        // Verwerfen stellt den Stand der Datei wieder her.
        editor.apply(Edit::Discard);
        assert!(!editor.dirty());
    }

    /// Löschen zieht die Kanten der Reihe davor nach; der Boss bleibt.
    #[test]
    fn delete() {
        let mut editor = blank_editor();
        editor.apply(Edit::AddRow);
        editor.apply(Edit::AddNode(1));
        editor.apply(Edit::AddNode(1));
        assert_eq!(editor.map.rows[0][0].next, [0, 1, 2]);
        editor.apply(Edit::Select(1, 1));
        editor.apply(Edit::Delete);
        assert_eq!(editor.map.rows[0][0].next, [0, 1]);
        assert_eq!(editor.map.check(), Ok(()));

        // Mit dem letzten Knoten fällt die Reihe, die Kanten erbt die Reihe davor.
        editor.apply(Edit::Select(1, 1));
        editor.apply(Edit::Delete);
        editor.apply(Edit::Select(1, 0));
        editor.apply(Edit::Delete);
        assert_eq!(editor.map, blank());

        editor.apply(Edit::Select(1, 0));
        editor.apply(Edit::Delete);
        assert_eq!(editor.map, blank());
    }

    /// Das Probespiel steht an jedem Knoten einer echten Karte, mit dem, was dort ansteht.
    #[test]
    fn test_run_reaches_every_node() {
        let mut editor = MapEditor::open(maps_dir());
        assert_eq!(editor.test_run(1), None);
        let kinds: Vec<Vec<NodeKind>> = (editor.map.rows.iter())
            .map(|row| row.iter().map(|node| node.kind).collect())
            .collect();
        for (row, kinds) in kinds.iter().enumerate() {
            for (column, kind) in kinds.iter().enumerate() {
                editor.apply(Edit::Select(row, column));
                let run = editor.test_run(1).unwrap();
                assert_eq!(run.current(), Some((row, column)));
                assert_eq!(run.sector, editor.map.sector);
                let battle = matches!(kind, NodeKind::Battle | NodeKind::Elite | NodeKind::Boss);
                assert_eq!(matches!(run.stage, Stage::Battle { .. }), battle);
            }
        }
    }

    /// Speichern schreibt eine neue Karte unter dem ersten freien Namen und nur, wenn sie stimmt.
    #[test]
    fn save_new_map() {
        let dir = std::env::temp_dir().join(format!("voidfleet-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sector1_a.json"), map_json(&blank())).unwrap();
        let mut editor = MapEditor::open(dir.clone());
        assert_eq!(editor.file.as_deref(), Some("sector1_a.json"));
        editor.apply(Edit::New);
        editor.apply(Edit::Select(0, 0));
        editor.apply(Edit::Place(None));
        editor.apply(Edit::Save);
        assert_eq!((editor.note.clone(), editor.files.len()), (None, 1));
        editor.apply(Edit::Place(Some(EnemyId::Fireship)));
        editor.apply(Edit::Save);
        assert_eq!(editor.note, Some(Note::Saved("sector1_b.json".into())));
        assert!(!editor.dirty());

        // Mit Ungespeichertem bleibt die Karte offen; sonst blättert der Editor.
        editor.apply(Edit::AddRow);
        editor.apply(Edit::File(1));
        assert_eq!(editor.file.as_deref(), Some("sector1_b.json"));
        editor.apply(Edit::Discard);
        editor.apply(Edit::File(1));
        assert_eq!(editor.file.as_deref(), Some("sector1_a.json"));
        assert_eq!(editor.map, blank());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
