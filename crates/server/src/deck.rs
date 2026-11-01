//! Gemischte Nachziehstapel, getrennt für Schiffe und Befehle.

use rand::Rng;
use rand::seq::SliceRandom;
use voidfleet_core::cards::{COMMANDS, CommandId, SHIPS, ShipId};

/// Eine Karte, die in einem Deck liegen kann; wie oft, sagt [`Card::deck_copies`] (der Regel-Dialog nennt es auch).
pub trait Card: Copy + 'static {
    /// Alle Karten dieser Art, auch die, die in keinem Deck liegen.
    const ALL: &'static [Self];

    fn deck_copies(self) -> usize;
}

impl Card for ShipId {
    const ALL: &'static [Self] = &SHIPS;

    fn deck_copies(self) -> usize {
        ShipId::deck_copies(self)
    }
}

impl Card for CommandId {
    const ALL: &'static [Self] = &COMMANDS;

    fn deck_copies(self) -> usize {
        CommandId::deck_copies(self)
    }
}

/// Ein Nachziehstapel; gezogen wird von oben (vom Ende).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck<T> {
    cards: Vec<T>,
}

impl<T: Card> Deck<T> {
    /// Jede Karte so oft, wie [`Card::deck_copies`] vorgibt, gemischt.
    pub fn full(rng: &mut (impl Rng + ?Sized)) -> Self {
        let cards = T::ALL
            .iter()
            .flat_map(|&card| std::iter::repeat_n(card, card.deck_copies()))
            .collect();
        let mut deck = Deck { cards };
        deck.cards.shuffle(rng);
        deck
    }

    pub fn draw(&mut self) -> Option<T> {
        self.cards.pop()
    }

    /// Zieht die oberste passende Karte; `None`, wenn keine passt.
    pub fn draw_where(&mut self, predicate: impl Fn(T) -> bool) -> Option<T> {
        let index = self.cards.iter().rposition(|&card| predicate(card))?;
        Some(self.cards.remove(index))
    }

    /// Zieht bis zu `count` Karten, weniger, wenn das Deck leer wird.
    pub fn draw_many(&mut self, count: usize) -> Vec<T> {
        let rest = self.cards.len().saturating_sub(count);
        self.cards.drain(rest..).rev().collect()
    }

    /// Legt Karten zurück und mischt neu (der Rest des Nachschubs); was in keinem Deck liegt (die Schrottverwertung),
    /// verfällt.
    pub fn put_back(&mut self, cards: impl IntoIterator<Item = T>, rng: &mut (impl Rng + ?Sized)) {
        self.cards
            .extend(cards.into_iter().filter(|card| card.deck_copies() > 0));
        self.cards.shuffle(rng);
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Leert das Deck (Tests: kein Nachschub mehr).
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.cards.clear();
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    #[test]
    fn das_deck_hat_jede_karte_so_oft_wie_vorgegeben() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(Deck::<ShipId>::full(&mut rng).len(), SHIPS.len() * 2);
        // 32 Befehle, Torpedo und Sperrfeuer doppelt, die Schrottverwertung nie.
        let mut commands = Deck::<CommandId>::full(&mut rng);
        assert_eq!(commands.len(), 34);
        assert_eq!(
            commands.draw_where(|card| card == CommandId::ScrapSalvage),
            None
        );
    }

    #[test]
    fn gezogen_wird_von_oben_und_zurueckgelegt_wird_ohne_schrottverwertung() {
        let mut rng = StdRng::seed_from_u64(2);
        let mut deck = Deck::<CommandId>::full(&mut rng);
        let top = deck.cards[deck.len() - 3..].to_vec();
        let drawn = deck.draw_many(3);
        assert_eq!(drawn, top.into_iter().rev().collect::<Vec<_>>());
        deck.put_back([drawn[0], CommandId::ScrapSalvage], &mut rng);
        assert_eq!(deck.len(), 32);
        assert_eq!(deck.draw_many(40).len(), 32);
        assert!(deck.is_empty());
    }
}
