//! Kartenillustrationen (`images/cards/<id>.jpg`): die Befehlskarten und die Flaggschiffe für die Admiralswahl.

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::{admirals::FlagshipId, cards::CardId};

use super::{FileKey, ship_models::FLAGSHIPS};
use crate::assets::Preload;

/// Ein Bild: Befehlskarte oder Flaggschiff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Card {
    Command(CardId),
    Flagship(FlagshipId),
}

/// Alle Karten mit eigenem Bild.
pub fn cards() -> impl Iterator<Item = Card> {
    voidfleet_core::cards::LOOT_POOL
        .into_iter()
        .map(Card::Command)
        .chain(FLAGSHIPS.into_iter().map(Card::Flagship))
}

/// Asset-Pfad der Illustration.
pub fn card_art_path(card: Card) -> String {
    let key = match card {
        Card::Command(card) => card.file_key(),
        Card::Flagship(flagship) => flagship.file_key(),
    };
    format!("images/cards/{key}.jpg")
}

/// Die Illustrationen aller Karten, beim Start geladen.
#[derive(Resource)]
pub struct CardArt(HashMap<Card, Handle<Image>>);

impl CardArt {
    /// Das Bild einer Karte.
    pub fn get(&self, card: Card) -> Handle<Image> {
        self.0[&card].clone()
    }
}

impl FromWorld for CardArt {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>().clone();
        let mut preload = world.resource_mut::<Preload>();
        Self(
            cards()
                .map(|card| (card, preload.track(server.load(card_art_path(card)))))
                .collect(),
        )
    }
}
