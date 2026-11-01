//! Kartenillustrationen (`images/cards/<id>.jpg`, erzeugt mit `tools/generateCardArt.ts`), auch Flaggschiffe, Drohne
//! und Trugbild.

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::cards::{COMMANDS, CommandId, FLAGSHIPS, SHIPS, ShipId, TOKENS};

use super::FileKey;
use crate::assets::Preload;

/// Eine Karte: Schiff (auch Token und Flaggschiff) oder Befehl.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Card {
    Ship(ShipId),
    Command(CommandId),
}

/// Alle Karten mit eigenem Bild: Schiffe samt Tokens und Flaggschiffen, Befehle ohne die Schrottverwertung.
pub fn cards() -> impl Iterator<Item = Card> {
    SHIPS
        .into_iter()
        .chain(TOKENS)
        .chain(FLAGSHIPS)
        .map(Card::Ship)
        .chain(
            COMMANDS
                .into_iter()
                .filter(|&command| command != CommandId::ScrapSalvage)
                .map(Card::Command),
        )
}

/// Welches Bild eine Karte zeigt: ihr eigenes, nur die Schrottverwertung zeigt den Scrap Raider, für den sie ist.
pub fn art_card(card: Card) -> Card {
    match card {
        Card::Command(CommandId::ScrapSalvage) => Card::Ship(ShipId::ScrapRaider),
        card => card,
    }
}

/// Asset-Pfad der Illustration.
pub fn card_art_path(card: Card) -> String {
    let key = match art_card(card) {
        Card::Ship(ship) => ship.file_key(),
        Card::Command(command) => command.file_key(),
    };
    format!("images/cards/{key}.jpg")
}

/// Die Illustrationen aller Karten, beim Start geladen.
#[derive(Resource)]
pub struct CardArt(HashMap<Card, Handle<Image>>);

impl CardArt {
    /// Das Bild einer Karte.
    pub fn get(&self, card: Card) -> Handle<Image> {
        self.0[&art_card(card)].clone()
    }
}

pub(super) fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let art = cards()
        .map(|card| (card, preload.track(server.load(card_art_path(card)))))
        .collect();
    commands.insert_resource(CardArt(art));
}
