//! PLATZHALTER für den Computergegner, bis `server/src/bot.ts` portiert ist: Der Bot-Port ersetzt den Inhalt dieses
//! Moduls und behält die drei Signaturen, über die [`crate::game::Match`] den Bot fragt:
//!
//! - [`choose_bot_admiral`]: der Admiral, sobald der Bot sich setzt ([`crate::game::Match::add_bot`]);
//! - [`plan_bot_turn`]: sein Zug, sobald der Mensch abgegeben hat (er sieht den fremden Plan nicht);
//! - [`plan_bot_draft`]: seine Wahl aus dem Nachschub, sofort nach der Runde.
//!
//! Was er weiß, steht in [`BotView`]. Seine Züge prüft der Match nicht; sie müssen gültig sein.
//!
//! Bis dahin spielt er den kleinsten gültigen Zug: in Runde 1 nur das Flaggschiff mittig in die Command-Lane, danach
//! nichts, und aus dem Nachschub je die erste Karte.

use rand::RngCore;
use rand::seq::IndexedRandom;
use voidfleet_core::admirals::{ADMIRAL_IDS, AdmiralId};
use voidfleet_core::board::{LaneType, SlotTarget};
use voidfleet_core::protocol::{DraftOffer, DraftPick, TurnPlan};

pub use crate::game::BotView;

/// Der Admiral des Computergegners: zufällig.
pub fn choose_bot_admiral(rng: &mut dyn RngCore) -> AdmiralId {
    *ADMIRAL_IDS.choose(rng).expect("es gibt Admirale")
}

/// PLATZHALTER: Runde 1 nur das Flaggschiff in Command-Slot 2, danach passen.
pub fn plan_bot_turn(view: &BotView, _rng: &mut dyn RngCore) -> TurnPlan {
    TurnPlan {
        flagship: (view.round == 1).then_some(SlotTarget::new(LaneType::Command, 2)),
        ..TurnPlan::default()
    }
}

/// PLATZHALTER: je die erste Karte, soweit es welche gibt.
pub fn plan_bot_draft(_view: &BotView, offer: &DraftOffer) -> DraftPick {
    DraftPick {
        ship: (!offer.ships.is_empty()).then_some(0),
        command: (!offer.commands.is_empty()).then_some(0),
    }
}
