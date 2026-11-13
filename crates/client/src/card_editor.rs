//! Der Befehlskarten-Editor ohne Oberfläche (Einstellungen → Dev, [`Screen::Cards`](crate::run::Screen)): jede Karte
//! aus ihrer Datei `assets/data/cards/<id>.json` laden (Art, Ziel, Zahlen, Name und Effekttext je Sprache), Schritt
//! für Schritt bearbeiten ([`CardEdit`]), prüfen ([`CardEditor::check`]) und im Format der Dateien schreiben
//! ([`card_json`]). Speichern setzt die Karten sofort im Spiel ein ([`install`]); die Vorschau zeigt schon vorher den
//! Stand des Editors ([`CardEditor::face`]). Die Ansicht steht in `ui/dialogs/card_editor.rs`; sie legt die Ressource
//! beim ersten Öffnen an.

use std::{collections::HashMap, path::PathBuf};

use bevy::prelude::*;
use serde_json::Value;
use voidfleet_core::{
    cards::{CardId, CardTarget, CardType, EXTRA_CARDS, LOOT_POOL},
    data::{CardData, CardText, Data, data, install, read},
};

use crate::{
    editor::Note,
    i18n::{check_card_texts, code, effect_text},
    settings::Language,
    ui::card::Face,
};

/// Eine Zahl der Karte. Die aufgewerteten gelten als ganze Zahl; gleicht eine der Grundfassung, entfällt sie in der
/// Datei.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Cost,
    Value,
    Extra,
    UpgradedCost,
    UpgradedValue,
}

/// Ein Text der Karte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextField {
    Name,
    Effect,
}

/// Ein Schritt im Editor; alles außer `Select`, `Save` und `Discard` meint die gewählte Karte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardEdit {
    Select(CardId),
    Type(CardType),
    Target(CardTarget),
    /// Ändert eine Zahl um so viel; keine fällt unter 0.
    Number(Field, i32),
    Text(Language, TextField, String),
    Save,
    Discard,
}

#[derive(Resource, Debug)]
pub struct CardEditor {
    /// `assets/data`.
    dir: PathBuf,
    pub cards: HashMap<CardId, CardData>,
    /// Der Stand der Dateien, zum Verwerfen und um Ungespeichertes zu erkennen.
    saved: HashMap<CardId, CardData>,
    pub selected: CardId,
    pub note: Option<Note>,
}

impl CardEditor {
    /// Liest die Kartendateien aus `dir`; ist eine unlesbar, gilt für sie der Stand des Spiels und die Meldung nennt
    /// den Fehler.
    pub fn open(dir: PathBuf) -> Self {
        let mut errors = Vec::new();
        let cards: HashMap<_, _> = LOOT_POOL
            .iter()
            .map(|&id| {
                let card = read(&dir, &id.file()).unwrap_or_else(|error| {
                    errors.push(error);
                    data().cards[&id].clone()
                });
                (id, card)
            })
            .collect();
        Self {
            dir,
            saved: cards.clone(),
            cards,
            selected: LOOT_POOL[0],
            note: (!errors.is_empty()).then(|| Note::Failed(errors.join("; "))),
        }
    }

    pub fn card(&self) -> &CardData {
        &self.cards[&self.selected]
    }

    pub fn text(&self, language: Language, card: CardId, field: TextField) -> &str {
        let text = self.cards[&card].text.get(code(language));
        text.map_or("", |text| match field {
            TextField::Name => &text.name,
            TextField::Effect => &text.effect,
        })
    }

    pub fn dirty(&self) -> bool {
        self.cards != self.saved
    }

    pub fn apply(&mut self, edit: CardEdit) {
        let card = self.cards.get_mut(&self.selected).expect("jede Karte");
        match edit {
            CardEdit::Select(id) => self.selected = id,
            CardEdit::Type(card_type) => card.card_type = card_type,
            CardEdit::Target(target) => card.target = target,
            CardEdit::Number(field, delta) => {
                let at_least_0 = |value: i32| (value + delta).max(0);
                match field {
                    Field::Cost => card.cost = at_least_0(card.cost as i32) as u32,
                    Field::Value => card.value = at_least_0(card.value),
                    Field::Extra => card.extra = card.extra.map(at_least_0),
                    Field::UpgradedCost => {
                        let cost = card.upgraded.cost.unwrap_or(card.cost) as i32;
                        card.upgraded.cost = Some(at_least_0(cost) as u32);
                    }
                    Field::UpgradedValue => {
                        let value = card.upgraded.value.unwrap_or(card.value);
                        card.upgraded.value = Some(at_least_0(value));
                    }
                }
                // Was die aufgewertete Fassung nicht ändert, steht nicht in der Datei.
                card.upgraded.cost = card.upgraded.cost.filter(|&cost| cost != card.cost);
                card.upgraded.value = card.upgraded.value.filter(|&value| value != card.value);
            }
            CardEdit::Text(language, field, text) => {
                let slot = card
                    .text
                    .entry(code(language).to_owned())
                    .or_insert(CardText {
                        name: String::new(),
                        effect: String::new(),
                    });
                match field {
                    TextField::Name => slot.name = text,
                    TextField::Effect => slot.effect = text,
                }
            }
            CardEdit::Save => self.save(),
            CardEdit::Discard => {
                self.cards = self.saved.clone();
                self.note = None;
            }
        }
    }

    /// Was auf der (aufgewerteten) gewählten Karte steht, im Stand des Editors; geht ein Platzhalter im Effekt nicht
    /// auf, steht er da, wie er geschrieben ist.
    pub fn face(&self, language: Language, upgraded: bool) -> Face {
        let card = self.card();
        let name = self.text(language, self.selected, TextField::Name);
        let template = self.text(language, self.selected, TextField::Effect);
        let value = card
            .upgraded
            .value
            .filter(|_| upgraded)
            .unwrap_or(card.value);
        let effect = effect_text(template, value, card.extra.unwrap_or(0));
        Face {
            name: if upgraded {
                format!("{name}+")
            } else {
                name.to_owned()
            },
            effect: if effect.contains(['{', '}']) {
                template.to_owned()
            } else {
                effect
            },
            cost: card.upgraded.cost.filter(|_| upgraded).unwrap_or(card.cost),
            card_type: card.card_type,
            target: card.target,
        }
    }

    /// Die erste Karte (in der Reihenfolge von [`LOOT_POOL`]) mit einer Zahl, die nicht stimmt (`None`), oder einem
    /// Text, der in dieser Sprache fehlt, keinen Namen hat oder einen Platzhalter, der nicht aufgeht.
    pub fn check(&self) -> Result<(), (CardId, Option<Language>)> {
        if let Some(&id) = LOOT_POOL
            .iter()
            .find(|id| !self.cards[id].check(EXTRA_CARDS.contains(id)))
        {
            return Err((id, None));
        }
        check_card_texts(&self.cards).map_err(|(id, language)| (id, Some(language)))
    }

    /// Schreibt die geänderten Karten, wenn alles stimmt, und setzt sie im Spiel ein.
    fn save(&mut self) {
        if self.check().is_err() {
            return;
        }
        let mut written = Vec::new();
        for id in LOOT_POOL {
            if self.cards[&id] == self.saved[&id] {
                continue;
            }
            let file = id.file();
            if let Err(error) = std::fs::write(self.dir.join(&file), card_json(&self.cards[&id])) {
                self.note = Some(Note::Failed(format!("{file}: {error}")));
                return;
            }
            self.saved.insert(id, self.cards[&id].clone());
            written.push(file);
        }
        let data = Data {
            cards: self.cards.clone(),
            ..data().clone()
        };
        self.note = Some(match install(data) {
            Ok(()) => Note::Saved(written.join(", ")),
            Err(error) => Note::Failed(error),
        });
    }
}

/// Ein JSON-Wert auf einer Zeile, mit Leerzeichen wie in den Dateien: `{ "cost": 1 }`, `{}`.
fn inline(value: &Value) -> String {
    match value {
        Value::Object(fields) if fields.is_empty() => "{}".to_owned(),
        Value::Object(fields) => {
            let fields: Vec<_> = fields
                .iter()
                .map(|(name, value)| format!("{}: {}", Value::from(name.as_str()), inline(value)))
                .collect();
            format!("{{ {} }}", fields.join(", "))
        }
        value => value.to_string(),
    }
}

/// Eine Kartendatei wie die Dateien: je Feld eine Zeile, die Texte je Sprache auf einer.
pub fn card_json(card: &CardData) -> String {
    let Value::Object(fields) = serde_json::to_value(card).expect("Karte als JSON") else {
        unreachable!("eine Karte ist ein Objekt")
    };
    let lines: Vec<_> = fields
        .iter()
        .map(|(name, value)| {
            let value = match (name.as_str(), value) {
                ("text", Value::Object(languages)) => {
                    let lines: Vec<_> = languages
                        .iter()
                        .map(|(code, text)| format!("    \"{code}\": {}", inline(text)))
                        .collect();
                    format!("{{\n{}\n  }}", lines.join(",\n"))
                }
                _ => inline(value),
            };
            format!("  \"{name}\": {value}")
        })
        .collect();
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

#[cfg(test)]
mod tests {
    use voidfleet_core::data::REPO_DATA;

    use super::*;

    fn open() -> CardEditor {
        CardEditor::open(REPO_DATA.into())
    }

    #[test]
    fn files_come_out_as_they_are() {
        let editor = open();
        assert_eq!(editor.note, None);
        for id in LOOT_POOL {
            let file = std::fs::read_to_string(PathBuf::from(REPO_DATA).join(id.file())).unwrap();
            assert_eq!(card_json(&editor.cards[&id]), file, "{}", id.file());
        }
        assert_eq!(editor.check(), Ok(()));
    }

    #[test]
    fn unreadable_files_fall_back_to_the_game() {
        let editor = CardEditor::open(PathBuf::from("nirgends"));
        assert!(matches!(editor.note, Some(Note::Failed(_))));
        assert_eq!(editor.cards, data().cards);
    }

    #[test]
    fn upgraded_numbers_drop_out_when_they_match() {
        let mut editor = open();
        editor.apply(CardEdit::Select(CardId::Torpedo));
        assert_eq!(editor.card().upgraded.value, Some(8));
        editor.apply(CardEdit::Number(Field::UpgradedValue, -2));
        assert_eq!(editor.card().upgraded.value, None);
        editor.apply(CardEdit::Number(Field::UpgradedCost, -1));
        assert_eq!(editor.card().upgraded.cost, Some(1));
        editor.apply(CardEdit::Number(Field::Cost, -1));
        assert_eq!((editor.card().cost, editor.card().upgraded.cost), (1, None));
        editor.apply(CardEdit::Number(Field::Cost, -5));
        assert_eq!(editor.card().cost, 0);
        // Ohne Zweitwert bleibt er weg.
        editor.apply(CardEdit::Number(Field::Extra, 1));
        assert_eq!(editor.card().extra, None);
        assert!(editor.dirty());
        editor.apply(CardEdit::Discard);
        assert!(!editor.dirty());
    }

    #[test]
    fn the_preview_follows_numbers_and_text() {
        let mut editor = open();
        editor.apply(CardEdit::Select(CardId::HoldTheLine));
        editor.apply(CardEdit::Number(Field::Extra, 4));
        let text = "Shields {v}, retaliation {extra}.".to_owned();
        editor.apply(CardEdit::Text(Language::En, TextField::Effect, text));
        let face = editor.face(Language::En, true);
        assert_eq!(face.effect, "Shields 7, retaliation 6.");
        assert_eq!(face.name, "Hold the Line+");
        assert_eq!(editor.check(), Ok(()));
        let broken = "Shields {w}.".to_owned();
        editor.apply(CardEdit::Text(Language::En, TextField::Effect, broken));
        assert_eq!(editor.face(Language::En, false).effect, "Shields {w}.");
        assert_eq!(
            editor.check(),
            Err((CardId::HoldTheLine, Some(Language::En)))
        );
    }

    #[test]
    fn saving_writes_the_changed_card_and_installs_it() {
        let dir = std::env::temp_dir().join(format!("voidfleet-cards-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("cards")).unwrap();
        for id in LOOT_POOL {
            std::fs::copy(
                PathBuf::from(REPO_DATA).join(id.file()),
                dir.join(id.file()),
            )
            .unwrap();
        }
        let mut editor = CardEditor::open(dir.clone());
        // Ein leerer Name wird nicht gespeichert.
        editor.apply(CardEdit::Text(Language::De, TextField::Name, " ".into()));
        editor.apply(CardEdit::Save);
        assert!(editor.dirty());
        editor.apply(CardEdit::Text(
            Language::De,
            TextField::Name,
            "Torpedo".into(),
        ));
        // Ein Text, den kein anderer Test liest: Die Tests laufen nebeneinander und teilen die Daten.
        let effect = |editor: &CardEditor| {
            editor
                .text(Language::De, CardId::RedAlert, TextField::Effect)
                .to_owned()
        };
        let before = effect(&editor);
        editor.apply(CardEdit::Select(CardId::RedAlert));
        editor.apply(CardEdit::Text(
            Language::De,
            TextField::Effect,
            format!("{before} Los."),
        ));
        editor.apply(CardEdit::Save);
        assert_eq!(
            editor.note,
            Some(Note::Saved("cards/red_alert.json".into()))
        );
        assert!(!editor.dirty());
        let reopened = CardEditor::open(dir.clone());
        assert_eq!(effect(&reopened), format!("{before} Los."));
        assert_eq!(
            data().cards[&CardId::RedAlert].text["de"].effect,
            format!("{before} Los.")
        );
        // Zurück auf den Stand des Repositorys.
        editor.apply(CardEdit::Text(
            Language::De,
            TextField::Effect,
            before.clone(),
        ));
        editor.apply(CardEdit::Save);
        assert_eq!(data().cards[&CardId::RedAlert].text["de"].effect, before);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
