//! Der Karteneditor ohne Oberfläche (Einstellungen → Dev, [`Screen::Editor`](crate::run::Screen)): die Sternenkarte
//! eines Sektors aus `assets/data/maps/` laden, Schritt für Schritt bearbeiten ([`Edit`]; jede Einheit mit ihren Werten
//! und ihrem Muster, auch die gerufenen), prüfen ([`SectorMap::check`]), im Format
//! der Dateien schreiben ([`map_json`]) und einen Knoten probespielen ([`MapEditor::test_run`]). Die Ansicht steht
//! in `ui/dialogs/map_editor.rs`; sie legt die Ressource [`MapEditor`] beim ersten Öffnen an.

use std::{io, path::PathBuf};

use bevy::prelude::*;
use serde::Serialize;
use voidfleet_core::{
    admirals::AdmiralId,
    battle::{Encounter, Reinforcement},
    board::{COLUMNS, Row},
    data::Step,
    enemies::{EnemyId, Unit},
    run::{MapNode, NodeKind, Run, SECTORS, SectorMap},
};

/// Wohin die nächste Einheit aus der Palette geht. Ein Feld und ein Eintrag einer Liste tragen eine Einheit, deren
/// Werte und Muster der Editor dann zeigt ([`MapEditor::unit`]); eine Liste selbst nimmt neue Einträge an.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Slot(Row, usize),
    Reinforcement,
    Replacement,
    ReinforcementEntry(usize),
    ReplacementEntry(usize),
}

/// Ein Wert einer Einheit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Hull,
    Attack,
    Armor,
}

/// Ein Schritt im Editor. Knoten sind (Reihe, Spalte); was einen Knoten ändert, meint den gewählten, was eine Einheit
/// ändert, die gezeigte ([`MapEditor::unit`]).
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Lädt die Karte dieses Sektors.
    Sector(u32),
    Target(Target),
    /// Die Einheit der Palette (`None`: leer) geht an das Ziel ([`Target`]), mit den Werten ihrer Vorlage.
    Place(Option<EnemyId>),
    /// Verschiebt die Runde der Verstärkung mit diesem Index.
    Round(usize, i32),
    DropReinforcement(usize),
    DropReplacement(usize),
    /// Ändert einen Wert der Einheit um so viel.
    Stat(Stat, i32),
    /// Hängt einen Schritt an ihr Muster; die folgenden meinen ihren Schritt mit Index.
    AddStep(Step),
    DropStep(usize),
    /// Verstellt den Wert eines Schritts ([`adjust`]).
    StepValue(usize, i32),
    /// Zeigt die Einheit, die dieser Schritt ruft.
    OpenCall(usize),
    /// Zurück zur Einheit, die die gezeigte ruft.
    CloseCall,
    /// Die Einheit bekommt wieder die Werte ihrer Vorlage aus `enemies_defaults.json`.
    ResetUnit,
    Admiral(AdmiralId),
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
    /// Der Sektor der offenen Karte, 1 bis [`SECTORS`].
    pub sector: u32,
    pub map: SectorMap,
    /// Der Stand der Datei, zum Verwerfen und um Ungespeichertes zu erkennen.
    saved: SectorMap,
    pub selected: Option<(usize, usize)>,
    pub target: Target,
    /// Der Weg vom Ziel zur gezeigten Einheit: je gerufener Einheit der Index ihres Schritts im Muster davor.
    pub calls: Vec<usize>,
    /// Mit diesem Admiral und seinem Startdeck läuft das Probespiel.
    pub admiral: AdmiralId,
    pub note: Option<Note>,
}

fn encounter(unit: EnemyId) -> Encounter {
    Encounter {
        front: [None, Some(unit.into()), None],
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
        rows: vec![vec![battle_node(vec![0])], vec![boss]],
    }
}

impl MapEditor {
    /// Öffnet die Karte von Sektor 1; ist sie unlesbar, eine leere.
    pub fn open(dir: PathBuf) -> Self {
        let mut editor = Self {
            dir,
            sector: 1,
            map: blank(),
            saved: blank(),
            selected: None,
            target: Target::Slot(Row::Front, 1),
            calls: Vec::new(),
            admiral: AdmiralId::Osei,
            note: None,
        };
        editor.load(1);
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

    /// Die Einheiten vom Ziel bis zur gezeigten: die im Ziel, dann je Eintrag von [`Self::calls`] die gerufene.
    pub fn chain(&self) -> Vec<&Unit> {
        let encounter = self.node().and_then(|node| node.encounter.as_ref());
        let Some(mut unit) = encounter.and_then(|encounter| root(encounter, self.target)) else {
            return Vec::new();
        };
        let mut chain = vec![unit];
        for &step in &self.calls {
            let Some(Step::Call(called)) = unit.pattern.get(step) else {
                break;
            };
            unit = called;
            chain.push(unit);
        }
        chain
    }

    /// Die gezeigte Einheit, deren Werte und Muster der Editor bearbeitet.
    pub fn unit(&self) -> Option<&Unit> {
        let chain = self.chain();
        (chain.len() == self.calls.len() + 1).then(|| chain[self.calls.len()])
    }

    fn unit_mut(&mut self) -> Option<&mut Unit> {
        let (target, calls) = (self.target, self.calls.clone());
        let mut unit = root_mut(self.node_mut()?.encounter.as_mut()?, target)?;
        for step in calls {
            let Some(Step::Call(called)) = unit.pattern.get_mut(step) else {
                return None;
            };
            unit = called;
        }
        Some(unit)
    }

    fn edit_unit(&mut self, change: impl FnOnce(&mut Unit)) {
        if let Some(unit) = self.unit_mut() {
            change(unit);
        }
    }

    pub fn apply(&mut self, edit: Edit) {
        self.note = None;
        // Was nicht an der gezeigten Einheit arbeitet, kann den Weg zu ihr abschneiden: zurück zum Ziel.
        let keep = matches!(
            edit,
            Edit::Stat(..)
                | Edit::AddStep(_)
                | Edit::DropStep(_)
                | Edit::StepValue(..)
                | Edit::OpenCall(_)
                | Edit::CloseCall
                | Edit::ResetUnit
                | Edit::Round(..)
                | Edit::Admiral(_)
                | Edit::Save
        );
        if !keep {
            self.calls.clear();
        }
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
            Edit::Sector(sector) => {
                if !self.dirty() {
                    self.load(sector);
                }
            }
            Edit::Target(target) => self.target = target,
            Edit::Place(unit) => {
                let target = self.target;
                let Some(encounter) = self.node_mut().and_then(|node| node.encounter.as_mut())
                else {
                    return;
                };
                let unit = unit.map(Unit::from);
                match (target, unit) {
                    (Target::Slot(Row::Front, column), unit) => encounter.front[column] = unit,
                    (Target::Slot(Row::Back, column), unit) => encounter.back[column] = unit,
                    (Target::Reinforcement, Some(unit)) => {
                        encounter
                            .reinforcements
                            .push(Reinforcement { round: 3, unit });
                    }
                    (Target::Replacement, Some(unit)) => encounter.replacements.push(unit),
                    (Target::ReinforcementEntry(_) | Target::ReplacementEntry(_), Some(unit)) => {
                        if let Some(entry) = root_mut(encounter, target) {
                            *entry = unit;
                        }
                    }
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
                    if matches!(self.target, Target::ReinforcementEntry(_)) {
                        self.target = Target::Reinforcement;
                    }
                }
            }
            Edit::DropReplacement(index) => {
                if let Some(encounter) = self.node_mut().and_then(|node| node.encounter.as_mut())
                    && index < encounter.replacements.len()
                {
                    encounter.replacements.remove(index);
                    if matches!(self.target, Target::ReplacementEntry(_)) {
                        self.target = Target::Replacement;
                    }
                }
            }
            Edit::Stat(stat, by) => self.edit_unit(|unit| {
                let (value, min) = match stat {
                    Stat::Hull => (&mut unit.hull, 1),
                    Stat::Attack => (&mut unit.attack, 0),
                    Stat::Armor => (&mut unit.armor, 0),
                };
                *value = (*value + by).max(min);
            }),
            Edit::AddStep(step) => self.edit_unit(|unit| unit.pattern.push(step)),
            // Der letzte Schritt bleibt: Ein Muster ist nie leer.
            Edit::DropStep(index) => self.edit_unit(|unit| {
                if unit.pattern.len() > 1 && index < unit.pattern.len() {
                    unit.pattern.remove(index);
                }
            }),
            Edit::StepValue(index, by) => self.edit_unit(|unit| {
                if let Some(step) = unit.pattern.get_mut(index) {
                    *step = adjust(step, by);
                }
            }),
            Edit::OpenCall(index) => {
                if let Some(Step::Call(_)) = self.unit().and_then(|unit| unit.pattern.get(index)) {
                    self.calls.push(index);
                }
            }
            Edit::CloseCall => {
                self.calls.pop();
            }
            Edit::ResetUnit => self.edit_unit(|unit| *unit = Unit::from(unit.id)),
            Edit::Admiral(admiral) => self.admiral = admiral,
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

    /// Lädt die Karte eines Sektors, auch eine, die die Prüfung nicht besteht; unlesbar bleibt die bisherige stehen.
    fn load(&mut self, sector: u32) {
        let name = SectorMap::file(sector);
        let read = std::fs::read_to_string(self.dir.join(&name))
            .map_err(|error| error.to_string())
            .and_then(|json| {
                serde_json::from_str::<SectorMap>(&json).map_err(|error| error.to_string())
            });
        match read {
            Ok(map) => {
                (self.saved, self.map) = (map.clone(), map);
                (self.sector, self.selected, self.note) = (sector, None, None);
            }
            Err(error) => self.note = Some(Note::Failed(format!("{name}: {error}"))),
        }
    }

    /// Schreibt die Karte in die Datei ihres Sektors, wenn sie stimmt.
    fn save(&mut self) {
        if self.map.check().is_err() {
            return;
        }
        let name = SectorMap::file(self.sector);
        self.note = Some(
            match std::fs::write(self.dir.join(&name), map_json(&self.map)) {
                Ok(()) => {
                    self.saved = self.map.clone();
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
        let mut run = Run::new(
            self.admiral,
            seed,
            std::array::from_fn(|_| self.map.clone()),
        );
        run.sector = self.sector;
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

/// Die Einheit in einem Feld oder Eintrag einer Liste.
fn root(encounter: &Encounter, target: Target) -> Option<&Unit> {
    match target {
        Target::Slot(Row::Front, column) => encounter.front[column].as_ref(),
        Target::Slot(Row::Back, column) => encounter.back[column].as_ref(),
        Target::ReinforcementEntry(index) => Some(&encounter.reinforcements.get(index)?.unit),
        Target::ReplacementEntry(index) => encounter.replacements.get(index),
        Target::Reinforcement | Target::Replacement => None,
    }
}

fn root_mut(encounter: &mut Encounter, target: Target) -> Option<&mut Unit> {
    match target {
        Target::Slot(Row::Front, column) => encounter.front[column].as_mut(),
        Target::Slot(Row::Back, column) => encounter.back[column].as_mut(),
        Target::ReinforcementEntry(index) => {
            Some(&mut encounter.reinforcements.get_mut(index)?.unit)
        }
        Target::ReplacementEntry(index) => encounter.replacements.get_mut(index),
        Target::Reinforcement | Target::Replacement => None,
    }
}

/// Ein Schritt mit verstelltem Wert: Spaltenfeuer 1 bis 3 Spalten breit, Schilde ab 1, Verstärkung die Vorlage der
/// Einheit `by` Plätze weiter in [`EnemyId::ALL`] (im Kreis); Schritte ohne Wert bleiben.
pub fn adjust(step: &Step, by: i32) -> Step {
    match step {
        Step::ColumnFire(width) => {
            Step::ColumnFire(width.saturating_add_signed(by as isize).clamp(1, COLUMNS))
        }
        Step::Shields(amount) => Step::Shields((amount + by).max(1)),
        Step::Call(unit) => {
            let all = EnemyId::ALL;
            let at = all
                .iter()
                .position(|&id| id == unit.id)
                .expect("jede Id steht in ALL");
            Step::Call(all[(at as i32 + by).rem_euclid(all.len() as i32) as usize].into())
        }
        step => step.clone(),
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
    format!("{{\n  \"rows\": [\n{}\n  ]\n}}\n", rows.join(",\n"))
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
        for sector in 1..=SECTORS {
            let name = SectorMap::file(sector);
            let json = std::fs::read_to_string(maps_dir().join(&name)).unwrap();
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
        assert_eq!(encounter.back[1], Some(EnemyId::Spotter.into()));
        assert_eq!(encounter.reinforcements[0].round, 1);
        assert_eq!(encounter.replacements, [EnemyId::Fireship.into()]);

        // Werte und Muster der Einheit im Feld; neue Einheiten kommen mit ihrer Vorlage.
        editor.apply(Edit::Target(Target::Slot(Row::Back, 1)));
        editor.apply(Edit::Stat(Stat::Hull, -100));
        editor.apply(Edit::Stat(Stat::Armor, 2));
        editor.apply(Edit::DropStep(0));
        editor.apply(Edit::AddStep(Step::Evade));
        editor.apply(Edit::AddStep(Step::Shields(6)));
        editor.apply(Edit::StepValue(2, -10));
        editor.apply(Edit::AddStep(Step::Call(EnemyId::Seal.into())));
        editor.apply(Edit::StepValue(3, 1));
        editor.apply(Edit::DropStep(0));
        let unit = editor.unit().unwrap();
        assert_eq!((unit.id, unit.hull, unit.armor), (EnemyId::Spotter, 1, 2));
        let raiders = Unit::from(EnemyId::Raiders);
        let pattern = [Step::Evade, Step::Shields(1), Step::Call(raiders.clone())];
        assert_eq!(unit.pattern, pattern);

        // Die gerufene Einheit hat eigene Werte; „Zurück“ zeigt wieder die rufende.
        editor.apply(Edit::OpenCall(0));
        assert!(editor.calls.is_empty());
        editor.apply(Edit::OpenCall(2));
        assert_eq!(editor.unit(), Some(&raiders));
        editor.apply(Edit::Stat(Stat::Attack, 3));
        assert_eq!(editor.chain().len(), 2);
        editor.apply(Edit::CloseCall);
        let Step::Call(called) = &editor.unit().unwrap().pattern[2] else {
            panic!("Ruf");
        };
        assert_eq!(called.attack, raiders.attack + 3);
        assert_eq!(editor.map.check(), Ok(()));
        let json = map_json(&editor.map);
        let line = format!(
            r#"{{ "id": "spotter", "hull": 1, "attack": 2, "armor": 2, "pattern": ["evade", {{ "shields": 1 }}, {{ "call": {{ "id": "raiders", "hull": {}, "attack": {}, "armor": 0, "pattern": ["fire"] }} }}] }}"#,
            raiders.hull,
            raiders.attack + 3
        );
        assert!(json.contains(&line), "{json}");
        assert_eq!(
            serde_json::from_str::<SectorMap>(&json).unwrap(),
            editor.map
        );
        editor.apply(Edit::DropStep(0));
        editor.apply(Edit::DropStep(0));
        editor.apply(Edit::DropStep(0));
        assert_eq!(editor.unit().unwrap().pattern.len(), 1);
        editor.apply(Edit::ResetUnit);
        assert_eq!(editor.unit(), Some(&EnemyId::Spotter.into()));

        // Ein Eintrag der Liste: Die Palette ersetzt ihn, seine Werte stehen zur Wahl; fällt er, bleibt die Liste Ziel.
        editor.apply(Edit::Target(Target::ReinforcementEntry(0)));
        editor.apply(Edit::Place(Some(EnemyId::Besieger)));
        editor.apply(Edit::Stat(Stat::Hull, 10));
        let encounter = editor.node().unwrap().encounter.clone().unwrap();
        assert_eq!(encounter.reinforcements.len(), 1);
        assert_eq!(
            encounter.reinforcements[0].unit.hull,
            Unit::from(EnemyId::Besieger).hull + 10
        );
        editor.apply(Edit::Target(Target::ReplacementEntry(0)));
        editor.apply(Edit::Stat(Stat::Hull, 1));
        assert_eq!(
            editor.unit().unwrap().hull,
            Unit::from(EnemyId::Fireship).hull + 1
        );
        editor.apply(Edit::DropReplacement(0));
        assert_eq!(editor.target, Target::Replacement);

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
                assert_eq!(run.sector, editor.sector);
                let battle = matches!(kind, NodeKind::Battle | NodeKind::Elite | NodeKind::Boss);
                assert_eq!(matches!(run.stage, Stage::Battle { .. }), battle);
            }
        }
    }

    /// Speichern schreibt die Datei des Sektors und nur, wenn die Karte stimmt; mit Ungespeichertem bleibt der
    /// Sektor offen.
    #[test]
    fn save_and_switch() {
        let dir = std::env::temp_dir().join(format!("voidfleet-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for sector in 1..=2 {
            std::fs::write(dir.join(SectorMap::file(sector)), map_json(&blank())).unwrap();
        }
        let mut editor = MapEditor::open(dir.clone());
        editor.apply(Edit::Sector(2));
        assert_eq!((editor.sector, editor.note.clone()), (2, None));
        editor.apply(Edit::Select(0, 0));
        editor.apply(Edit::Place(None));
        editor.apply(Edit::Save);
        assert_eq!(editor.note, None);
        editor.apply(Edit::Sector(1));
        assert_eq!(editor.sector, 2);
        editor.apply(Edit::Place(Some(EnemyId::Fireship)));
        editor.apply(Edit::Save);
        assert_eq!(editor.note, Some(Note::Saved("sector2.json".into())));
        assert!(!editor.dirty());
        let json = std::fs::read_to_string(dir.join("sector2.json")).unwrap();
        assert_eq!(json, map_json(&editor.map));

        // Eine fehlende Datei lässt die offene Karte stehen.
        editor.apply(Edit::Sector(3));
        assert_eq!(editor.sector, 2);
        assert!(matches!(editor.note, Some(Note::Failed(_))));
        editor.apply(Edit::Sector(1));
        assert_eq!((editor.sector, &editor.map), (1, &blank()));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
