//! Der Admirals-Editor ohne Oberfläche (Einstellungen → Dev, [`Screen::AdmiralEditor`](crate::run::Screen)): die
//! Admirale aus `assets/data/admirals.json` laden (Hülle und ATK des Flaggschiffs, Schild-Station, Kennkarte,
//! Startdeck), Schritt für Schritt bearbeiten ([`AdmiralEdit`]), prüfen ([`AdmiralData::check`]) und im Format der
//! Datei schreiben ([`admirals_json`]). Speichern setzt die Admirale sofort im Spiel ein ([`install`]); Texte mit
//! Zahlen aus den Daten (`{admirals.osei.shield_station}`) folgen erst beim nächsten Start. Die Ansicht steht in
//! `ui/dialogs/admiral_editor.rs`; sie legt die Ressource beim ersten Öffnen an.

use std::{collections::HashMap, path::PathBuf};

use bevy::prelude::*;
use serde_json::Value;
use voidfleet_core::{
    admirals::{ADMIRAL_IDS, AdmiralId},
    cards::CardId,
    data::{AdmiralData, Data, data, install, read},
};

use crate::{data::FileKey, editor::Note};

const FILE: &str = "admirals.json";

/// Eine Zahl des Admirals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Hull,
    Attack,
    ShieldStation,
}

/// Ein Schritt im Editor; alles außer `Select`, `Save` und `Discard` meint den gewählten Admiral.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmiralEdit {
    Select(AdmiralId),
    /// Ändert eine Zahl um so viel; keine fällt unter 0.
    Number(Field, i32),
    Signature(CardId),
    /// Eine Karte mehr ins Startdeck, hinter die letzte gleiche.
    AddCard(CardId),
    /// Die letzte dieser Karte aus dem Startdeck.
    RemoveCard(CardId),
    Save,
    Discard,
}

#[derive(Resource, Debug)]
pub struct AdmiralEditor {
    /// `assets/data`.
    dir: PathBuf,
    pub admirals: HashMap<AdmiralId, AdmiralData>,
    /// Der Stand der Datei, zum Verwerfen und um Ungespeichertes zu erkennen.
    saved: HashMap<AdmiralId, AdmiralData>,
    pub selected: AdmiralId,
    pub note: Option<Note>,
}

impl AdmiralEditor {
    /// Liest `admirals.json` aus `dir`; ist sie unlesbar, gilt der Stand des Spiels und die Meldung nennt den Fehler.
    pub fn open(dir: PathBuf) -> Self {
        let (admirals, note) = match read(&dir, FILE) {
            Ok(admirals) => (admirals, None),
            Err(error) => (data().admirals.clone(), Some(Note::Failed(error))),
        };
        Self {
            dir,
            saved: admirals.clone(),
            admirals,
            selected: ADMIRAL_IDS[0],
            note,
        }
    }

    pub fn admiral(&self) -> &AdmiralData {
        &self.admirals[&self.selected]
    }

    pub fn dirty(&self) -> bool {
        self.admirals != self.saved
    }

    /// Der erste Admiral (in der Reihenfolge von [`ADMIRAL_IDS`]), der die Prüfung nicht besteht.
    pub fn check(&self) -> Result<(), AdmiralId> {
        match ADMIRAL_IDS
            .into_iter()
            .find(|id| !self.admirals[id].check())
        {
            Some(id) => Err(id),
            None => Ok(()),
        }
    }

    pub fn apply(&mut self, edit: AdmiralEdit) {
        let admiral = self
            .admirals
            .get_mut(&self.selected)
            .expect("jeder Admiral");
        match edit {
            AdmiralEdit::Select(id) => self.selected = id,
            AdmiralEdit::Number(field, delta) => {
                let number = match field {
                    Field::Hull => &mut admiral.hull,
                    Field::Attack => &mut admiral.attack,
                    Field::ShieldStation => &mut admiral.shield_station,
                };
                *number = (*number + delta).max(0);
            }
            AdmiralEdit::Signature(card) => admiral.signature_card = card,
            AdmiralEdit::AddCard(card) => {
                let at = admiral.deck.iter().rposition(|&c| c == card);
                admiral
                    .deck
                    .insert(at.map_or(admiral.deck.len(), |at| at + 1), card);
            }
            AdmiralEdit::RemoveCard(card) => {
                if let Some(at) = admiral.deck.iter().rposition(|&c| c == card) {
                    admiral.deck.remove(at);
                }
            }
            AdmiralEdit::Save => self.save(),
            AdmiralEdit::Discard => {
                self.admirals = self.saved.clone();
                self.note = None;
            }
        }
    }

    /// Schreibt die Datei, wenn alles stimmt, und setzt die Admirale im Spiel ein.
    fn save(&mut self) {
        if self.check().is_err() {
            return;
        }
        if let Err(error) = std::fs::write(self.dir.join(FILE), admirals_json(&self.admirals)) {
            self.note = Some(Note::Failed(format!("{FILE}: {error}")));
            return;
        }
        self.saved = self.admirals.clone();
        let data = Data {
            admirals: self.admirals.clone(),
            ..data().clone()
        };
        self.note = Some(match install(data) {
            Ok(()) => Note::Saved(FILE.to_owned()),
            Err(error) => Note::Failed(error),
        });
    }
}

/// `admirals.json` wie die Datei: je Admiral eine Zeile mit den Zahlen und der Kennkarte, eine mit dem Startdeck.
pub fn admirals_json(admirals: &HashMap<AdmiralId, AdmiralData>) -> String {
    let entries: Vec<_> = ADMIRAL_IDS
        .iter()
        .map(|id| {
            let Value::Object(fields) =
                serde_json::to_value(&admirals[id]).expect("Admiral als JSON")
            else {
                unreachable!("ein Admiral ist ein Objekt")
            };
            let line = |fields: Vec<String>| fields.join(", ");
            let values = fields
                .iter()
                .filter(|(name, _)| *name != "deck")
                .map(|(name, value)| format!("\"{name}\": {value}"));
            let deck = admirals[id]
                .deck
                .iter()
                .map(|card| format!("\"{}\"", card.file_key()));
            format!(
                "  \"{}\": {{\n    {},\n    \"deck\": [{}]\n  }}",
                id.file_key(),
                line(values.collect()),
                line(deck.collect())
            )
        })
        .collect();
    format!("{{\n{}\n}}\n", entries.join(",\n"))
}

#[cfg(test)]
mod tests {
    use voidfleet_core::data::REPO_DATA;

    use super::*;

    fn open() -> AdmiralEditor {
        AdmiralEditor::open(REPO_DATA.into())
    }

    #[test]
    fn the_file_comes_out_as_it_is() {
        let editor = open();
        assert_eq!(editor.note, None);
        let file = std::fs::read_to_string(PathBuf::from(REPO_DATA).join(FILE)).unwrap();
        assert_eq!(admirals_json(&editor.admirals), file);
        assert_eq!(editor.check(), Ok(()));
    }

    #[test]
    fn unreadable_files_fall_back_to_the_game() {
        let editor = AdmiralEditor::open(PathBuf::from("nirgends"));
        assert!(matches!(editor.note, Some(Note::Failed(_))));
        assert_eq!(editor.admirals, data().admirals);
    }

    #[test]
    fn the_deck_stays_grouped_and_the_signature_card_must_be_in_it() {
        let mut editor = open();
        editor.apply(AdmiralEdit::Select(AdmiralId::Vance));
        editor.apply(AdmiralEdit::AddCard(CardId::Torpedo));
        assert_eq!(editor.admiral().deck[..4], [CardId::Torpedo; 4]);
        editor.apply(AdmiralEdit::AddCard(CardId::Barrage));
        assert_eq!(editor.admiral().deck.last(), Some(&CardId::Barrage));
        editor.apply(AdmiralEdit::Signature(CardId::Barrage));
        editor.apply(AdmiralEdit::RemoveCard(CardId::Barrage));
        assert_eq!(editor.check(), Err(AdmiralId::Vance));
        editor.apply(AdmiralEdit::Signature(CardId::Overcharge));
        assert_eq!(editor.check(), Ok(()));
        editor.apply(AdmiralEdit::Number(Field::Hull, -500));
        assert_eq!(editor.admiral().hull, 0);
        assert_eq!(editor.check(), Err(AdmiralId::Vance));
        assert!(editor.dirty());
        editor.apply(AdmiralEdit::Discard);
        assert!(!editor.dirty());
    }

    #[test]
    fn saving_writes_the_file_and_installs_it() {
        let dir = std::env::temp_dir().join(format!("voidfleet-admirals-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(PathBuf::from(REPO_DATA).join(FILE), dir.join(FILE)).unwrap();
        let mut editor = AdmiralEditor::open(dir.clone());
        editor.apply(AdmiralEdit::Select(AdmiralId::Kerr));
        // Eine Kennkarte außerhalb des Decks wird nicht gespeichert.
        editor.apply(AdmiralEdit::Signature(CardId::Barrage));
        editor.apply(AdmiralEdit::Save);
        assert!(editor.dirty());
        editor.apply(AdmiralEdit::Signature(CardId::MicroJump));
        // Ein Wert, den kein anderer Test liest: Die Tests laufen nebeneinander und teilen die Daten.
        let before = editor.admiral().shield_station;
        editor.apply(AdmiralEdit::Number(Field::ShieldStation, 1));
        editor.apply(AdmiralEdit::Save);
        assert_eq!(editor.note, Some(Note::Saved(FILE.into())));
        assert!(!editor.dirty());
        let reopened = AdmiralEditor::open(dir.clone());
        assert_eq!(
            reopened.admirals[&AdmiralId::Kerr].shield_station,
            before + 1
        );
        assert_eq!(AdmiralId::Kerr.shield_station(), before + 1);
        // Zurück auf den Stand des Repositorys.
        editor.apply(AdmiralEdit::Number(Field::ShieldStation, -1));
        editor.apply(AdmiralEdit::Save);
        assert_eq!(AdmiralId::Kerr.shield_station(), before);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
