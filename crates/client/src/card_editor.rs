//! Der Befehlskarten-Editor ohne Oberfläche (Einstellungen → Dev, [`Screen::Cards`](crate::run::Screen)): Art,
//! Ziel und Zahlen jeder Karte aus `assets/data/cards.json`, Name und Effekttext je Sprache aus
//! `assets/data/text/<sprache>.json` laden, Schritt für Schritt bearbeiten ([`CardEdit`]), prüfen
//! ([`CardEditor::check`]) und im Format der Dateien schreiben ([`cards_json`], [`text_json`]). Die Dateien sind
//! eingebettet: Was gespeichert ist, gilt im Spiel nach dem Neubauen; die Vorschau zeigt den Stand des Editors
//! ([`CardEditor::face`]). Die Ansicht steht in `ui/dialogs/card_editor.rs`; sie legt die Ressource beim ersten Öffnen
//! an.

use std::{collections::HashMap, path::PathBuf};

use bevy::prelude::*;
use serde_json::Value;
use voidfleet_core::{
    cards::{CardId, CardTarget, CardType, DeckCard, EXTRA_CARDS, LOOT_POOL},
    data::{CardData, data},
};

use crate::{
    editor::Note,
    i18n::{LANGUAGES, TEXT_FILES, effect_text, snake, with_data},
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
    saved_cards: HashMap<CardId, CardData>,
    /// Die Textdateien in der Reihenfolge von [`LANGUAGES`], ganz; der Editor ändert nur die Karten darin.
    texts: [Value; 2],
    saved_texts: [Value; 2],
    pub selected: CardId,
    pub note: Option<Note>,
}

fn index(language: Language) -> usize {
    LANGUAGES
        .iter()
        .position(|&listed| listed == language)
        .expect("jede Sprache steht in LANGUAGES")
}

fn text_pointer(card: CardId, field: TextField) -> String {
    let field = match field {
        TextField::Name => "name",
        TextField::Effect => "effect",
    };
    format!("/card/{}/{field}", snake(card))
}

impl CardEditor {
    /// Liest `cards.json` und die Textdateien aus `dir`; ist eine unlesbar, gilt der eingebettete Stand und die
    /// Meldung nennt den Fehler.
    pub fn open(dir: PathBuf) -> Self {
        let read = |name: &str| {
            std::fs::read_to_string(dir.join(name)).map_err(|error| format!("{name}: {error}"))
        };
        let cards = read("cards.json").and_then(|json| {
            serde_json::from_str(&json).map_err(|error| format!("cards.json: {error}"))
        });
        let texts = TEXT_FILES.map(|(name, embedded)| {
            read(&format!("text/{name}"))
                .and_then(|json| serde_json::from_str(&json).map_err(|e| format!("{name}: {e}")))
                .map_err(|error| (error, serde_json::from_str(embedded).expect("eingebettet")))
        });
        let mut errors = Vec::new();
        let cards = cards.unwrap_or_else(|error| {
            errors.push(error);
            data().cards.clone()
        });
        let texts = texts.map(|text| {
            text.unwrap_or_else(|(error, embedded)| {
                errors.push(error);
                embedded
            })
        });
        Self {
            dir,
            saved_cards: cards.clone(),
            cards,
            saved_texts: texts.clone(),
            texts,
            selected: LOOT_POOL[0],
            note: (!errors.is_empty()).then(|| Note::Failed(errors.join("; "))),
        }
    }

    pub fn card(&self) -> &CardData {
        &self.cards[&self.selected]
    }

    pub fn text(&self, language: Language, card: CardId, field: TextField) -> &str {
        self.texts[index(language)]
            .pointer(&text_pointer(card, field))
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    pub fn dirty(&self) -> bool {
        self.cards != self.saved_cards || self.texts != self.saved_texts
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
                let pointer = text_pointer(self.selected, field);
                if let Some(slot) = self.texts[index(language)].pointer_mut(&pointer) {
                    *slot = Value::String(text);
                }
            }
            CardEdit::Save => self.save(),
            CardEdit::Discard => {
                self.cards = self.saved_cards.clone();
                self.texts = self.saved_texts.clone();
                self.note = None;
            }
        }
    }

    /// Die Daten des Regelwerks als JSON, mit den Karten des Editors: Daraus setzen die Texte ihre Zahlen ein.
    fn data(&self) -> Value {
        let mut data = serde_json::to_value(data()).expect("Daten des Regelwerks als JSON");
        data["cards"] = serde_json::to_value(&self.cards).expect("Karten als JSON");
        data
    }

    /// Der Effekttext, wie die Karte ihn zeigt; `Err` mit dem Text, wie er dasteht, wenn ein Platzhalter nicht
    /// aufgeht.
    fn effect(&self, data: &Value, language: Language, card: DeckCard) -> Result<String, String> {
        let template = self.text(language, card.id, TextField::Effect);
        let extra = self.cards[&card.id].extra.unwrap_or(0);
        with_data(template, data)
            .map(|text| effect_text(&text, self.value(card), extra))
            .ok()
            .filter(|text| !text.contains(['{', '}']))
            .ok_or_else(|| template.to_owned())
    }

    fn value(&self, card: DeckCard) -> i32 {
        let data = &self.cards[&card.id];
        data.upgraded
            .value
            .filter(|_| card.upgraded)
            .unwrap_or(data.value)
    }

    /// Was auf der (aufgewerteten) gewählten Karte steht, im Stand des Editors.
    pub fn face(&self, language: Language, upgraded: bool) -> Face {
        let card = DeckCard {
            id: self.selected,
            upgraded,
        };
        let data = self.card();
        let name = self.text(language, card.id, TextField::Name);
        Face {
            name: if upgraded {
                format!("{name}+")
            } else {
                name.to_owned()
            },
            effect: self
                .effect(&self.data(), language, card)
                .unwrap_or_else(|text| text),
            cost: data.upgraded.cost.filter(|_| upgraded).unwrap_or(data.cost),
            card_type: data.card_type,
            target: data.target,
        }
    }

    /// Die erste Karte (in der Reihenfolge von [`LOOT_POOL`]) mit leerem Namen, einer Zahl, die nicht stimmt, oder
    /// einem Platzhalter im Effekttext, der nicht aufgeht; dazu die Sprache des Texts.
    pub fn check(&self) -> Result<(), (CardId, Option<Language>)> {
        let data = self.data();
        for id in LOOT_POOL {
            if !self.cards[&id].check(EXTRA_CARDS.contains(&id)) {
                return Err((id, None));
            }
            for language in LANGUAGES {
                let named = !self.text(language, id, TextField::Name).trim().is_empty();
                let card = DeckCard {
                    id,
                    upgraded: false,
                };
                if !named || self.effect(&data, language, card).is_err() {
                    return Err((id, Some(language)));
                }
            }
        }
        Ok(())
    }

    /// Schreibt die geänderten Dateien, wenn alles stimmt.
    fn save(&mut self) {
        if self.check().is_err() {
            return;
        }
        let mut files = Vec::new();
        if self.cards != self.saved_cards {
            files.push(("cards.json".to_owned(), cards_json(&self.cards)));
        }
        for (i, (name, _)) in TEXT_FILES.iter().enumerate() {
            if self.texts[i] != self.saved_texts[i] {
                files.push((format!("text/{name}"), text_json(&self.texts[i])));
            }
        }
        let mut written = Vec::new();
        for (name, json) in files {
            if let Err(error) = std::fs::write(self.dir.join(&name), json) {
                self.note = Some(Note::Failed(format!("{name}: {error}")));
                return;
            }
            written.push(name);
        }
        self.saved_cards = self.cards.clone();
        self.saved_texts = self.texts.clone();
        self.note = Some(Note::Saved(written.join(", ")));
    }
}

/// Ein JSON-Wert auf einer Zeile, mit Leerzeichen wie in `cards.json`: `{ "cost": 1 }`, `{}`.
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

/// `cards.json` wie die Datei: je Karte eine Zeile, in der Reihenfolge von [`LOOT_POOL`].
pub fn cards_json(cards: &HashMap<CardId, CardData>) -> String {
    let lines: Vec<_> = LOOT_POOL
        .iter()
        .map(|id| {
            let card = serde_json::to_value(&cards[id]).expect("Karte als JSON");
            format!("  \"{}\": {}", snake(id), inline(&card))
        })
        .collect();
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

/// Eine Textdatei wie die Dateien: eingerückt mit zwei Leerzeichen, Felder in ihrer Reihenfolge.
pub fn text_json(text: &Value) -> String {
    serde_json::to_string_pretty(text).expect("Text als JSON") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_data() -> CardEditor {
        CardEditor::open(PathBuf::from(crate::REPO_ASSETS).join("data"))
    }

    #[test]
    fn files_come_out_as_they_are() {
        let dir = PathBuf::from(crate::REPO_ASSETS).join("data");
        let editor = open_data();
        assert_eq!(editor.note, None);
        let cards = std::fs::read_to_string(dir.join("cards.json")).unwrap();
        assert_eq!(cards_json(&editor.cards), cards);
        for (i, (name, _)) in TEXT_FILES.iter().enumerate() {
            let text = std::fs::read_to_string(dir.join("text").join(name)).unwrap();
            assert_eq!(text_json(&editor.texts[i]), text, "{name}");
        }
        assert_eq!(editor.check(), Ok(()));
    }

    #[test]
    fn unreadable_files_fall_back_to_the_embedded_ones() {
        let editor = CardEditor::open(PathBuf::from("nirgends"));
        assert!(matches!(editor.note, Some(Note::Failed(_))));
        assert_eq!(editor.cards, data().cards);
        assert_eq!(editor.check(), Ok(()));
    }

    #[test]
    fn upgraded_numbers_drop_out_when_they_match() {
        let mut editor = open_data();
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
        let mut editor = open_data();
        editor.apply(CardEdit::Select(CardId::HoldTheLine));
        editor.apply(CardEdit::Number(Field::Extra, 4));
        let text = "Shields {v}, retaliation {extra}, {cards.torpedo.value}.".to_owned();
        editor.apply(CardEdit::Text(Language::En, TextField::Effect, text));
        let face = editor.face(Language::En, true);
        assert_eq!(face.effect, "Shields 7, retaliation 6, 6.");
        assert_eq!(face.name, "Hold the Line+");
        assert_eq!(editor.check(), Ok(()));
        let broken = "Shields {w}.".to_owned();
        editor.apply(CardEdit::Text(Language::En, TextField::Effect, broken));
        assert_eq!(editor.face(Language::En, false).effect, "Shields {w}.");
        assert_eq!(
            editor.check(),
            Err((CardId::HoldTheLine, Some(Language::En)))
        );
        editor.apply(CardEdit::Text(
            Language::En,
            TextField::Effect,
            "{cards.x.y}".into(),
        ));
        assert!(editor.check().is_err());
    }

    #[test]
    fn saving_writes_only_what_changed() {
        let dir = std::env::temp_dir().join(format!("voidfleet-cards-{}", std::process::id()));
        let source = PathBuf::from(crate::REPO_ASSETS).join("data");
        std::fs::create_dir_all(dir.join("text")).unwrap();
        std::fs::copy(source.join("cards.json"), dir.join("cards.json")).unwrap();
        for (name, _) in TEXT_FILES {
            std::fs::copy(source.join("text").join(name), dir.join("text").join(name)).unwrap();
        }
        let mut editor = CardEditor::open(dir.clone());
        editor.apply(CardEdit::Text(Language::De, TextField::Name, "Aal".into()));
        editor.apply(CardEdit::Save);
        assert_eq!(editor.note, Some(Note::Saved("text/de.json".into())));
        assert!(!editor.dirty());
        let reopened = CardEditor::open(dir.clone());
        assert_eq!(
            reopened.text(Language::De, CardId::Torpedo, TextField::Name),
            "Aal"
        );
        // Ein leerer Name wird nicht gespeichert.
        editor.apply(CardEdit::Text(Language::De, TextField::Name, " ".into()));
        editor.apply(CardEdit::Save);
        assert!(editor.dirty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
