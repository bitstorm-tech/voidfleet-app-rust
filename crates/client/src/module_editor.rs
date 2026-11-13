//! Der Modul-Editor ohne Oberfläche (Einstellungen → Dev, [`Screen::ModuleEditor`](crate::run::Screen)): die Zahl
//! jedes Moduls, das eine hat ([`number`]), in `assets/data/rules.json` Schritt für Schritt bearbeiten
//! ([`ModuleEdit`]) und im Format der Datei schreiben ([`rules_json`]). Der Editor hält die Datei als JSON, so ist
//! jede Zahl ein Pfad in ihr. Speichern setzt die Regeln sofort im Spiel ein ([`install`]); die Texte der Module
//! (`{rules.modules.salvage_crane_scrap}`) folgen erst beim nächsten Start, die Vorschau zeigt schon die neuen. Die
//! Ansicht steht in `ui/dialogs/module_editor.rs`; sie legt die Ressource beim ersten Öffnen an.

use std::path::PathBuf;

use bevy::prelude::*;
use serde_json::Value;
use voidfleet_core::{
    data::{Data, Rules, data, install, read, rules},
    modules::ModuleId,
    run::MODULES,
};

use crate::editor::Note;

const FILE: &str = "rules.json";

/// Der Pfad der Zahl eines Moduls in `rules.json`; Löschanlage und Notaggregat haben keine.
pub fn number(module: ModuleId) -> Option<&'static str> {
    use ModuleId::*;
    Some(match module {
        FireControlComputer => "/modules/fire_control_computer_attack",
        ArmorPlates => "/modules/armor_plates_armor",
        ReserveReactor => "/modules/reserve_reactor_points",
        SupplyPod => "/modules/supply_pod_hand",
        SalvageCrane => "/modules/salvage_crane_scrap",
        ReinforcedFrames => "/modules/reinforced_frames_hull",
        CombatSimulator => "/run/combat_simulator_offer",
        FireSuppression | EmergencyGenerator => return None,
    })
}

/// Ein Schritt im Editor; `Number` meint das gewählte Modul.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleEdit {
    Select(ModuleId),
    /// Ändert die Zahl um so viel; sie fällt nicht unter 0.
    Number(i64),
    Save,
    Discard,
}

#[derive(Resource, Debug)]
pub struct ModuleEditor {
    /// `assets/data`.
    dir: PathBuf,
    /// `rules.json` als JSON, in der Reihenfolge der Datei.
    pub rules: Value,
    /// Der Stand der Datei, zum Verwerfen und um Ungespeichertes zu erkennen.
    saved: Value,
    pub selected: ModuleId,
    pub note: Option<Note>,
}

impl ModuleEditor {
    /// Liest `rules.json` aus `dir`; ist sie unlesbar, gilt der Stand des Spiels und die Meldung nennt den Fehler.
    pub fn open(dir: PathBuf) -> Self {
        let (rules, note) = match read(&dir, FILE) {
            Ok(rules) => (rules, None),
            Err(error) => (
                serde_json::to_value(rules()).expect("Regeln als JSON"),
                Some(Note::Failed(error)),
            ),
        };
        Self {
            dir,
            saved: rules.clone(),
            rules,
            selected: MODULES[0],
            note,
        }
    }

    /// Die Zahl des gewählten Moduls, wenn es eine hat.
    pub fn value(&self) -> Option<i64> {
        self.rules.pointer(number(self.selected)?)?.as_i64()
    }

    pub fn dirty(&self) -> bool {
        self.rules != self.saved
    }

    pub fn apply(&mut self, edit: ModuleEdit) {
        match edit {
            ModuleEdit::Select(module) => self.selected = module,
            ModuleEdit::Number(delta) => {
                if let Some(value) = self.value() {
                    let path = number(self.selected).expect("eine Zahl");
                    *self.rules.pointer_mut(path).expect("die Zahl") =
                        (value + delta).max(0).into();
                }
            }
            ModuleEdit::Save => self.save(),
            ModuleEdit::Discard => {
                self.rules = self.saved.clone();
                self.note = None;
            }
        }
    }

    /// Schreibt die Datei und setzt die Regeln im Spiel ein.
    fn save(&mut self) {
        let rules = match serde_json::from_value::<Rules>(self.rules.clone()) {
            Ok(rules) => rules,
            Err(error) => {
                self.note = Some(Note::Failed(format!("{FILE}: {error}")));
                return;
            }
        };
        if let Err(error) = std::fs::write(self.dir.join(FILE), rules_json(&self.rules)) {
            self.note = Some(Note::Failed(format!("{FILE}: {error}")));
            return;
        }
        self.saved = self.rules.clone();
        let data = Data {
            rules,
            ..data().clone()
        };
        self.note = Some(match install(data) {
            Ok(()) => Note::Saved(FILE.to_owned()),
            Err(error) => Note::Failed(error),
        });
    }
}

/// `rules.json` wie die Datei: je Abschnitt ein Block, je Zahl eine Zeile, Spannen als `[von, bis]`.
pub fn rules_json(rules: &Value) -> String {
    let line = |items: Vec<String>, separator| items.join(separator);
    let object = |value: &Value| value.as_object().expect("ein Abschnitt").clone();
    let sections = object(rules)
        .iter()
        .map(|(name, fields)| {
            let fields = object(fields)
                .iter()
                .map(|(field, value)| {
                    let value = match value {
                        Value::Array(items) => {
                            format!(
                                "[{}]",
                                line(items.iter().map(Value::to_string).collect(), ", ")
                            )
                        }
                        value => value.to_string(),
                    };
                    format!("    \"{field}\": {value}")
                })
                .collect();
            format!("  \"{name}\": {{\n{}\n  }}", line(fields, ",\n"))
        })
        .collect();
    format!("{{\n{}\n}}\n", line(sections, ",\n"))
}

#[cfg(test)]
mod tests {
    use voidfleet_core::data::REPO_DATA;

    use super::*;

    fn open() -> ModuleEditor {
        ModuleEditor::open(REPO_DATA.into())
    }

    #[test]
    fn the_file_comes_out_as_it_is() {
        let editor = open();
        assert_eq!(editor.note, None);
        let file = std::fs::read_to_string(PathBuf::from(REPO_DATA).join(FILE)).unwrap();
        assert_eq!(rules_json(&editor.rules), file);
    }

    #[test]
    fn every_number_is_in_the_file() {
        let editor = open();
        for module in MODULES {
            if let Some(path) = number(module) {
                assert!(
                    editor.rules.pointer(path).is_some_and(Value::is_u64),
                    "{path}"
                );
            }
        }
    }

    #[test]
    fn numbers_stay_at_or_above_zero() {
        let mut editor = open();
        editor.apply(ModuleEdit::Select(ModuleId::FireSuppression));
        editor.apply(ModuleEdit::Number(1));
        assert_eq!(editor.value(), None);
        assert!(!editor.dirty());
        editor.apply(ModuleEdit::Select(ModuleId::SupplyPod));
        editor.apply(ModuleEdit::Number(-500));
        assert_eq!(editor.value(), Some(0));
        assert!(editor.dirty());
        editor.apply(ModuleEdit::Discard);
        assert!(!editor.dirty());
    }

    #[test]
    fn saving_writes_the_file_and_installs_it() {
        let dir = std::env::temp_dir().join(format!("voidfleet-modules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(PathBuf::from(REPO_DATA).join(FILE), dir.join(FILE)).unwrap();
        let mut editor = ModuleEditor::open(dir.clone());
        // Ein Wert, den kein anderer Test liest: Die Tests laufen nebeneinander und teilen die Daten.
        editor.apply(ModuleEdit::Select(ModuleId::SalvageCrane));
        let before = rules().modules.salvage_crane_scrap;
        editor.apply(ModuleEdit::Number(1));
        editor.apply(ModuleEdit::Save);
        assert_eq!(editor.note, Some(Note::Saved(FILE.into())));
        assert!(!editor.dirty());
        let reopened = ModuleEditor::open(dir.clone());
        let path = number(ModuleId::SalvageCrane).unwrap();
        assert_eq!(reopened.rules.pointer(path), Some(&(before + 1).into()));
        assert_eq!(rules().modules.salvage_crane_scrap, before + 1);
        // Zurück auf den Stand des Repositorys.
        editor.apply(ModuleEdit::Number(-1));
        editor.apply(ModuleEdit::Save);
        assert_eq!(rules().modules.salvage_crane_scrap, before);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
