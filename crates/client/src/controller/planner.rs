//! Die Zugplanung per Klick (Port der Klick-Logik aus `matchController.ts`), ohne Bevy-Systeme und daher testbar.
//!
//! Gewählt ist immer höchstens eines ([`Selection`]): eine Schiffskarte (oder das Flaggschiff), ein Befehl samt
//! Ausgangsschiff eines Lane-Wechsels, ein eigenes Schiff auf dem Brett, die Admiralsfähigkeit oder die Zielerfassung.
//! Was ein Klick bedeutet, hängt davon ab; [`Planner`] ändert Plan und Auswahl und sagt, welcher Klang dazu gehört.
//! Was dabei wegfällt, merkt sich [`Notices`] bis zum nächsten Klick (die Statuszeile nennt es).
//!
//! Gültige Ziele sagt eine einzige Regel ([`is_valid_target`]) für Hervorhebung und Klick; [`hypothetical`] liefert
//! den Zug, wie er nach einem Klick auf das Ziel unter dem Zeiger aussähe (Geister, Pfeile, Zielvorschau).

use bevy::prelude::*;
use voidfleet_core::{
    admirals::PowerId,
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::{CommandId, ShipId},
    protocol::{CommandPlan, FireTarget, Maneuver, PowerPlan},
};

use crate::{
    audio::UiSound,
    game::{CommandChange, Match, ShipPick},
    scene::board::{BoardHover, Highlights, all_slots},
    ui::{HandCard, Selected},
};

/// Was gerade gewählt ist und auf einen Klick aufs Brett wartet.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Selection {
    #[default]
    Idle,
    /// Eine Schiffskarte bzw. das Flaggschiff wartet auf ihren Slot.
    Ship(ShipPick),
    /// Ein Befehl (Hand-Index) wartet auf sein Ziel; bei einem verlegenden Befehl ist `move_source` das gewählte
    /// Schiff, solange der Ziel-Slot noch fehlt.
    Command {
        index: usize,
        move_source: Option<SlotRef>,
    },
    /// Ein eigenes Schiff (Slot beim Planen, auch ein eingeplantes): wartet auf seinen Feuerbefehl und, steht es schon
    /// auf dem Brett, auf sein Grundmanöver.
    Unit(SlotRef),
    /// Die Admiralsfähigkeit wartet auf ihr Ziel.
    Power,
    /// Die Zielerfassung der Pendulum wartet auf ihr Ziel.
    Mark,
}

impl Selection {
    /// Was Hand und Fähigkeitskarte als gewählt zeigen.
    pub fn selected(self) -> Option<Selected> {
        match self {
            Self::Ship(pick) => Some(Selected::Ship(pick)),
            Self::Command { index, .. } => Some(Selected::Command(index)),
            Self::Power => Some(Selected::Power),
            Self::Mark => Some(Selected::Mark),
            Self::Idle | Self::Unit(_) => None,
        }
    }
}

/// Was der letzte Klick nebenbei bewirkt hat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Notice {
    /// Der alte Befehl ist weg und mit ihm, was er erlaubt hat: ein Schiff (Schnellverlegung) bzw. die Fähigkeit
    /// (Freie Hand).
    CommandDropped {
        command: CommandId,
        ship: Option<ShipId>,
        power: Option<PowerId>,
    },
    /// Das Schiff feuert nicht mehr (Manöver, Rammkurs, Kurzsprung); sein Feuerbefehl ist weg.
    FireCleared(ShipId),
    /// Das Grundmanöver hat die Fähigkeit verdrängt.
    ManeuverReplacesPower(PowerId),
    /// Die Fähigkeit hat das Grundmanöver dieses Schiffs verdrängt.
    PowerReplacesManeuver { power: PowerId, ship: ShipId },
    /// Die Fähigkeit hat gerade kein Ziel.
    NoPowerTarget(PowerId),
}

/// Die Hinweise des letzten Klicks; der nächste Klick löscht sie.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Notices(pub Vec<Notice>);

/// Was Escape bewirkt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    /// Etwas wurde zurückgenommen.
    TookBack,
    /// Nichts zurückzunehmen: Die Einstellungen öffnen.
    OpenSettings,
}

/// Die Planung im Griff eines Klicks: Spielzustand, Auswahl, Hinweise und ob gerade geplant werden darf (verbunden,
/// keine Wiedergabe, Admirale gewählt, Zug offen, Nachschub gewählt, Match nicht entschieden).
pub struct Planner<'a> {
    pub game: &'a mut Match,
    pub selection: &'a mut Selection,
    pub notices: &'a mut Notices,
    pub can_plan: bool,
}

impl Planner<'_> {
    /// Eine Karte der Hand: eine eingeplante Schiffskarte nimmt ihr Schiff zurück, die gewählte Karte die Wahl.
    /// Befehle ohne Zielwahl sind mit einem Klick eingeplant, sonst folgt der Klick aufs Brett.
    pub fn press_hand_card(&mut self, card: HandCard) -> UiSound {
        self.notices.0.clear();
        if !self.can_plan {
            return UiSound::Denied;
        }
        match card {
            HandCard::Ship(pick) => {
                if self
                    .game
                    .arrivals()
                    .iter()
                    .any(|arrival| arrival.pick == pick)
                {
                    self.unplan_ship(pick)
                } else if *self.selection == Selection::Ship(pick) {
                    self.deselect()
                } else {
                    self.select(Selection::Ship(pick))
                }
            }
            HandCard::Command(index) => {
                let card = self.game.command_hand[index];
                if self
                    .game
                    .plan()
                    .command
                    .is_some_and(|command| command.hand_index == index)
                {
                    self.set_command(None);
                    self.deselect()
                } else if matches!(*self.selection, Selection::Command { index: chosen, .. } if chosen == index)
                {
                    self.deselect()
                } else if card.needs_target() {
                    self.select(Selection::Command {
                        index,
                        move_source: None,
                    })
                } else {
                    self.set_command(Some(CommandPlan {
                        hand_index: index,
                        target: None,
                        destination: None,
                    }));
                    self.place()
                }
            }
        }
    }

    /// Die Fähigkeitskarte: startet die Zielwahl; ein weiterer Klick bricht sie ab bzw. nimmt die eingeplante
    /// Fähigkeit zurück. Ohne Ziel sagt ein Hinweis, warum nicht.
    pub fn press_power(&mut self) -> UiSound {
        self.notices.0.clear();
        if !self.can_plan {
            return UiSound::Denied;
        }
        if self.game.plan().power.is_some() {
            self.game.set_power(None);
            return self.deselect();
        }
        if *self.selection == Selection::Power {
            return self.deselect();
        }
        if self.game.power_targets().is_empty() {
            let power = self.game.power().expect("Fähigkeit nach der Admiralswahl");
            self.notices.0.push(Notice::NoPowerTarget(power));
            return UiSound::Denied;
        }
        self.select(Selection::Power)
    }

    /// Der Knopf der Zielerfassung: wie die Fähigkeitskarte, für das Ziel der Railgun in der nächsten Runde.
    pub fn press_mark(&mut self) -> UiSound {
        self.notices.0.clear();
        if !self.can_plan || !self.game.can_mark() {
            return UiSound::Denied;
        }
        if self.game.plan().mark.is_some() {
            self.game.set_mark(None);
            return self.deselect();
        }
        if *self.selection == Selection::Mark {
            return self.deselect();
        }
        if self.game.mark_targets().is_empty() {
            return UiSound::Denied;
        }
        self.select(Selection::Mark)
    }

    /// Ein Klick auf einen Slot (Pad, Schiff oder Geist darauf). Ohne Auswahl wählt er ein eigenes Schiff, auf das
    /// Trugbild nimmt er die Fähigkeit zurück; mit Auswahl plant er auf ein gültiges Ziel, sonst verweigert er.
    pub fn press_slot(&mut self, slot: SlotRef) -> Option<UiSound> {
        self.notices.0.clear();
        let game = &*self.game;
        match *self.selection {
            Selection::Idle => {
                if !self.can_plan || slot.side == Side::Enemy {
                    return None;
                }
                if game.planned_decoy() == Some(slot.target()) {
                    self.game.set_power(None);
                    return Some(self.deselect());
                }
                unit_at(game, slot).map(|unit| self.select_unit(unit))
            }
            Selection::Unit(unit) => Some(self.press_with_unit(unit, slot)),
            _ if !is_valid_target(game, *self.selection, slot) => Some(UiSound::Denied),
            Selection::Ship(pick) => {
                self.game.plan_ship(pick, slot.target());
                Some(self.place())
            }
            Selection::Command { index, move_source } => {
                Some(self.stage_command_target(index, move_source, slot))
            }
            Selection::Power => {
                let power = game.power().expect("Fähigkeit nach der Admiralswahl");
                let replaced = self.game.set_power(Some(PowerPlan {
                    target: slot.target(),
                }));
                if let Some(ship) = replaced {
                    self.notices
                        .0
                        .push(Notice::PowerReplacesManeuver { power, ship });
                }
                Some(self.place())
            }
            Selection::Mark => {
                self.game.set_mark(Some(slot.target()));
                Some(self.place())
            }
        }
    }

    /// Ein Klick auf den Pfeil vor einer gegnerischen Spalte: Spaltenfeuer des gewählten Schützen (die Pfeile sind nur
    /// zu sehen, solange einer zielt).
    pub fn press_column(&mut self, column: usize) -> Option<UiSound> {
        self.notices.0.clear();
        let shooter = shooter(self.game, *self.selection)?;
        Some(self.stage_fire(shooter, FireTarget::Column(column)))
    }

    /// Ein Klick neben das Brett bricht die Auswahl ab.
    pub fn miss(&mut self) -> Option<UiSound> {
        self.notices.0.clear();
        (*self.selection != Selection::Idle).then(|| self.deselect())
    }

    /// Ein Rechtsklick bricht eine angefangene Auswahl ab.
    pub fn cancel(&mut self) -> Option<UiSound> {
        (self.can_plan && *self.selection != Selection::Idle).then(|| {
            self.notices.0.clear();
            self.deselect()
        })
    }

    /// Escape bricht eine angefangene Auswahl ab, sonst nimmt es das Grundmanöver zurück, dann die Fähigkeit, dann die
    /// Zielerfassung; gibt es nichts zurückzunehmen, öffnen sich die Einstellungen.
    pub fn escape(&mut self) -> Escape {
        let plan = self.game.plan();
        let nothing = *self.selection == Selection::Idle
            && plan.maneuver.is_none()
            && plan.power.is_none()
            && plan.mark.is_none();
        if !self.can_plan || nothing {
            return Escape::OpenSettings;
        }
        self.notices.0.clear();
        if *self.selection == Selection::Idle {
            if plan.maneuver.is_some() {
                self.game.set_maneuver(None);
            } else if plan.power.is_some() {
                self.game.set_power(None);
            } else {
                self.game.set_mark(None);
            }
        }
        *self.selection = Selection::Idle;
        Escape::TookBack
    }

    fn select(&mut self, selection: Selection) -> UiSound {
        *self.selection = selection;
        UiSound::Select
    }

    fn deselect(&mut self) -> UiSound {
        *self.selection = Selection::Idle;
        UiSound::Cancel
    }

    /// Eingeplant: Die Auswahl endet.
    fn place(&mut self) -> UiSound {
        *self.selection = Selection::Idle;
        UiSound::Place
    }

    /// Wählt das eigene Schiff; eines, mit dem sich nichts planen oder zurücknehmen lässt, verweigert den Klick.
    fn select_unit(&mut self, unit: SlotRef) -> UiSound {
        let game = &*self.game;
        let selectable = game.can_fire(unit)
            || game.planned_pick_at(unit).is_some()
            || is_maneuvering(game, unit)
            || !game.maneuver_destinations(unit).is_empty();
        if selectable {
            self.select(Selection::Unit(unit))
        } else {
            UiSound::Denied
        }
    }

    /// Klick bei gewähltem Schiff: auf das Schiff selbst (auch seinen Geist) nimmt zurück, auf ein gegnerisches Schiff
    /// ändert den Feuerbefehl, auf einen Ziel-Slot plant das Grundmanöver, auf ein anderes eigenes Schiff wählt dieses.
    fn press_with_unit(&mut self, unit: SlotRef, slot: SlotRef) -> UiSound {
        let game = &*self.game;
        if shows_unit(game, slot, unit) {
            return self.take_back(unit);
        }
        if is_valid_target(game, *self.selection, slot) {
            return match slot.side {
                Side::Enemy => self.stage_fire(unit, FireTarget::Ship(slot.target())),
                Side::Player => self.stage_maneuver(unit, slot),
            };
        }
        match unit_at(game, slot) {
            Some(other) => self.select_unit(other),
            None => UiSound::Denied,
        }
    }

    /// Der zweite Klick auf das gewählte Schiff nimmt zurück, was es in dieser Runde bringt oder bewegt: das
    /// eingeplante Schiff, sonst sein Grundmanöver (auch den Phasentausch, von beiden Schiffen aus); sonst endet nur
    /// die Auswahl.
    fn take_back(&mut self, unit: SlotRef) -> UiSound {
        if let Some(pick) = self.game.planned_pick_at(unit) {
            return self.unplan_ship(pick);
        }
        if is_maneuvering(self.game, unit) {
            self.game.set_maneuver(None);
        }
        self.deselect()
    }

    /// Nimmt ein eingeplantes Schiff zurück, samt Befehl, Fähigkeit und Feuerbefehl darauf; ein angefangener
    /// Lane-Wechsel des Schiffs bricht ab, ebenso seine Auswahl auf dem Brett und die Wahl einer Schiffskarte.
    fn unplan_ship(&mut self, pick: ShipPick) -> UiSound {
        let moving = matches!(*self.selection, Selection::Command { move_source: Some(source), .. }
            if self.game.planned_pick_at(source) == Some(pick));
        self.game.unplan_ship(pick);
        let gone =
            matches!(*self.selection, Selection::Unit(unit) if self.game.ship_at(unit).is_none());
        if moving || gone || matches!(*self.selection, Selection::Ship(_)) {
            *self.selection = Selection::Idle;
        }
        UiSound::Cancel
    }

    /// Der Feuerbefehl nach einem Tipp auf ein Ziel ([`Match::next_fire`]); beim zweiten Tipp auf dasselbe Ziel keiner
    /// mehr. Das Schiff bleibt gewählt.
    fn stage_fire(&mut self, shooter: SlotRef, aim: FireTarget) -> UiSound {
        let order = self.game.next_fire(shooter.target(), aim);
        self.game.set_fire(shooter.target(), order);
        if order.is_some() {
            UiSound::Place
        } else {
            UiSound::Cancel
        }
    }

    /// Das Grundmanöver des gewählten Schiffs; was wegfällt (Fähigkeit, Feuerbefehl), sagt der Hinweis.
    fn stage_maneuver(&mut self, unit: SlotRef, slot: SlotRef) -> UiSound {
        let change = self.game.set_maneuver(Some(Maneuver {
            ship: unit.target(),
            destination: slot.target(),
        }));
        if change.power {
            let power = self.game.power().expect("Fähigkeit nach der Admiralswahl");
            self.notices.0.push(Notice::ManeuverReplacesPower(power));
        }
        self.notices.0.extend(change.fire.map(Notice::FireCleared));
        self.place()
    }

    /// Verlegende Befehle merken sich erst das Schiff und werden mit dem Ziel-Slot eingeplant.
    fn stage_command_target(
        &mut self,
        index: usize,
        move_source: Option<SlotRef>,
        slot: SlotRef,
    ) -> UiSound {
        let card = self.game.command_hand[index];
        match move_source {
            None if card.card().moves.is_some() => self.select(Selection::Command {
                index,
                move_source: Some(slot),
            }),
            None => {
                self.set_command(Some(command_plan(card, index, slot, None)));
                self.place()
            }
            Some(source) => {
                self.set_command(Some(command_plan(card, index, source, Some(slot))));
                self.place()
            }
        }
    }

    /// Plant einen Befehl (`None`: keinen); was mit dem alten Befehl wegfällt oder wegen des neuen nicht mehr feuert,
    /// sagt der Hinweis.
    fn set_command(&mut self, command: Option<CommandPlan>) {
        let before = self.game.planned_command();
        let CommandChange { ship, power, fire } = self.game.set_command(command);
        if let Some(command) = before
            && (ship.is_some() || power)
        {
            self.notices.0.push(Notice::CommandDropped {
                command,
                ship,
                power: self.game.power().filter(|_| power),
            });
        }
        self.notices.0.extend(fire.map(Notice::FireCleared));
    }
}

/// Der Befehl auf `target`, bei verlegenden Befehlen in den Ziel-Slot `destination`. Der Durchschlag schickt den
/// Strike-Slot der gewählten Slot-Nummer.
fn command_plan(
    card: CommandId,
    index: usize,
    target: SlotRef,
    destination: Option<SlotRef>,
) -> CommandPlan {
    let target = if card.targets_enemy_column() {
        SlotTarget::new(LaneType::Strike, target.slot_index)
    } else {
        target.target()
    };
    CommandPlan {
        hand_index: index,
        target: Some(target),
        destination: destination.map(|slot| slot.slot_index),
    }
}

/// Das eigene Schiff, für das ein Slot steht, als Slot beim Planen: das echte oder eingeplante Schiff dort; der Geist
/// im Ziel-Slot einer Bewegung steht für das bewegte Schiff. Drohnen eingeplanter Träger und das Trugbild zählen nicht.
pub fn unit_at(game: &Match, slot: SlotRef) -> Option<SlotRef> {
    if slot.side == Side::Enemy {
        return None;
    }
    if game.ship_at(slot).is_some() {
        return Some(slot);
    }
    [game.planned_maneuver(), game.planned_command_move()]
        .into_iter()
        .flatten()
        .find(|planned| planned.to == slot)
        .map(|planned| planned.from)
}

/// Ob der Slot das Schiff zeigt, echt oder als Geist; beim Phasentausch stehen beide Schiffe auf beiden Slots.
fn shows_unit(game: &Match, slot: SlotRef, unit: SlotRef) -> bool {
    let swapped = game.planned_swap().is_some_and(|swap| {
        let pair = [swap.from, swap.to];
        pair.contains(&unit) && pair.contains(&slot)
    });
    swapped || unit_at(game, slot) == Some(unit)
}

/// Ob das Schiff im eingeplanten Grundmanöver den Slot wechselt, beim Phasentausch auch der Tauschpartner.
pub fn is_maneuvering(game: &Match, slot: SlotRef) -> bool {
    game.planned_maneuver().is_some_and(|planned| {
        planned.from == slot || (game.planned_swap().is_some() && planned.to == slot)
    })
}

/// Das gewählte Schiff, wenn es feuern kann.
pub fn shooter(game: &Match, selection: Selection) -> Option<SlotRef> {
    match selection {
        Selection::Unit(unit) if game.can_fire(unit) => Some(unit),
        _ => None,
    }
}

/// Die einzige Gültigkeitsregel für Hervorhebung und Klick: Schiffskarte → freie eigene Slots, die den eingeplanten
/// Bewegungen nicht im Weg stehen; eigenes Schiff → gegnerische Schiffe, wenn es feuern kann, dazu seine Ziel-Slots
/// des Grundmanövers (bei der Mirage auch eigene Schiffe); Befehl → eigene Schiffe in einer erlaubten Lane (verlegende
/// nur mit freiem Ziel-Slot, danach die freien Slots der Ziel-Lane), Torpedo und Durchschlag jeder Slot der
/// Gegnerseite, Wärmesucher und Traktorstrahl gegnerische Schiffe; Fähigkeit und Zielerfassung → ihre Ziele. Ohne
/// Auswahl leuchten das eigene Befehlsziel, die Ziel-Slots der Bewegungen und das manövrierende Schiff weiter.
pub fn is_valid_target(game: &Match, selection: Selection, slot: SlotRef) -> bool {
    let own = slot.side == Side::Player;
    match selection {
        Selection::Power => game.power_targets().contains(&slot),
        Selection::Mark => game.mark_targets().contains(&slot),
        Selection::Ship(pick) => own && game.can_deploy(slot, pick),
        Selection::Unit(unit) if own => game.maneuver_destinations(unit).contains(&slot),
        Selection::Unit(unit) => game.board.is_occupied(slot) && game.can_fire(unit),
        Selection::Command { index, move_source } => {
            let card = game.command_hand[index];
            if card.targets_enemy_slot() {
                return !own;
            }
            if card.targets_enemy_ship() {
                return !own
                    && game
                        .board
                        .card(slot)
                        .is_some_and(|ship| card.can_target_ship(ship));
            }
            if !own {
                return false;
            }
            if let Some(source) = move_source {
                return game.command_move_destinations(card, source).contains(&slot);
            }
            game.can_target_own(card, slot)
                && (card.card().moves.is_none()
                    || !game.command_move_destinations(card, slot).is_empty())
        }
        Selection::Idle => {
            own && (game.planned_own_target() == Some(slot.target())
                || game
                    .planned_command_move()
                    .is_some_and(|planned| planned.to == slot)
                || game
                    .planned_maneuver()
                    .is_some_and(|planned| planned.from == slot || planned.to == slot))
        }
    }
}

/// Hervorhebung beim Planen: die gültigen Ziele und das Gitter, solange ein freier Slot gesucht wird (Legen,
/// Lane-Wechsel, Manöver, Trugbild).
pub fn highlights(game: &Match, selection: Selection) -> Highlights {
    let grid = match selection {
        Selection::Ship(_) => true,
        Selection::Unit(unit) => !game.maneuver_destinations(unit).is_empty(),
        Selection::Command { move_source, .. } => move_source.is_some(),
        Selection::Power => game.power() == Some(PowerId::Decoy),
        Selection::Idle | Selection::Mark => false,
    };
    Highlights {
        slots: all_slots()
            .filter(|&slot| is_valid_target(game, selection, slot))
            .collect(),
        grid,
    }
}

/// Worauf ein Klick des zielenden Schiffs gerade gälte: der Pfeil vor einer Spalte oder das gegnerische Schiff unter
/// dem Zeiger.
pub fn hovered_aim(game: &Match, selection: Selection, hover: BoardHover) -> Option<FireTarget> {
    shooter(game, selection)?;
    if let Some(column) = hover.column {
        return Some(FireTarget::Column(column));
    }
    hover
        .slot
        .filter(|&slot| slot.side == Side::Enemy && is_valid_target(game, selection, slot))
        .map(|slot| FireTarget::Ship(slot.target()))
}

/// Der Zug, wie er nach einem Klick auf das Ziel unter dem Zeiger aussähe; `None`, wenn der Klick dort nichts planen
/// würde (dann zeigt die Szene den eingeplanten Zug).
pub fn hypothetical(game: &Match, selection: Selection, hover: BoardHover) -> Option<Match> {
    let valid = hover
        .slot
        .filter(|&slot| is_valid_target(game, selection, slot));
    let mut shown = game.clone();
    match selection {
        Selection::Unit(unit) => match hovered_aim(game, selection, hover) {
            Some(aim) => shown.set_fire(unit.target(), game.next_fire(unit.target(), aim)),
            None => {
                let slot = valid.filter(|slot| slot.side == Side::Player)?;
                shown.set_maneuver(Some(Maneuver {
                    ship: unit.target(),
                    destination: slot.target(),
                }));
            }
        },
        Selection::Ship(pick) => shown.plan_ship(pick, valid?.target()),
        Selection::Command { index, move_source } => {
            let card = game.command_hand[index];
            let slot = valid?;
            let plan = match move_source {
                Some(source) => command_plan(card, index, source, Some(slot)),
                None if card.targets_enemy_side() => command_plan(card, index, slot, None),
                None => return None,
            };
            shown.set_command(Some(plan));
        }
        Selection::Power => {
            shown.set_power(Some(PowerPlan {
                target: valid?.target(),
            }));
        }
        Selection::Mark => shown.set_mark(Some(valid?.target())),
        Selection::Idle => return None,
    }
    Some(shown)
}

#[cfg(test)]
mod tests;
