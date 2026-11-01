//! Das Grundmanöver neben dem geplanten Befehl ([`Situation::plan_maneuver`]): Es lässt dessen Zielschiff in Ruhe und
//! meidet den Slot, in den der Befehl verlegt. Erst einem wertvollen Schiff aus dem vermuteten Feuer helfen (zuerst dem
//! Flaggschiff), dann das ungedeckte Flaggschiff in Deckung bringen, ein markiertes Schiff aus dem Schuss der Railgun
//! nehmen, der Phasentausch der Mirage, Rückzug, Deckung, Vorrücken, ein Ausweichen des Flaggschiffs vor einem
//! Torpedo, sonst selten ein Ausweichen.

use rand::{Rng, RngCore};
use voidfleet_core::admirals::ABILITY_VALUES;
use voidfleet_core::board::{LaneType, PlannedMove, SlotRef, nearest};
use voidfleet_core::cards::{CommandId, ShipId};
use voidfleet_core::protocol::Maneuver;

use super::situation::{Choice, Situation, Trial};
use super::{
    DECISIVE, EVADE, FLAGSHIP_RETREAT_HP, SURPRISE_CHANCE, SWAP_CHANCE, attacks, card, damaged,
    highest, hp, is_flagship, is_fragile, is_token, unplayed,
};

/// Eine Bewegung dieser Runde, per Grundmanöver oder Befehl.
type Move = PlannedMove;

impl Situation<'_> {
    /// Das Grundmanöver neben dem Befehl `command`; ohne eigenes Zielschiff steht es schon fest, wenn Freie Hand es
    /// bewertet hat.
    pub(super) fn plan_maneuver(
        &self,
        command: Option<&Choice>,
        rng: &mut dyn RngCore,
    ) -> Option<Maneuver> {
        match command.filter(|command| command.target.is_some()) {
            Some(command) => self.maneuver_beside(Some(command), rng),
            None => *self
                .free_maneuver
                .get_or_init(|| self.maneuver_beside(None, rng)),
        }
    }

    fn maneuver_beside(&self, command: Option<&Choice>, rng: &mut dyn RngCore) -> Option<Maneuver> {
        let target = command.and_then(|command| command.target);
        let other = command.and_then(|command| {
            Some(Move {
                from: command.target?,
                to: command.destination?,
            })
        });
        let movable: Vec<SlotRef> = self
            .settled(self.own.iter().copied())
            .into_iter()
            .filter(|&at| Some(at) != target)
            .collect();
        // Die Pendulum bleibt sonst stehen: Sie feuert oder markiert jede Runde, und bewegt sie sich, fällt der Schuss
        // aus.
        let free: Vec<SlotRef> = movable
            .iter()
            .copied()
            .filter(|&at| card(&self.board, at) != ShipId::Pendulum)
            .collect();
        let chosen = self
            .evade(&movable, other, rng)
            .or_else(|| self.protect_flagship(&movable, other))
            .or_else(|| self.evade_mark(&free, other))
            .or_else(|| self.phase_swap(&free, other, rng))
            .or_else(|| self.retreat(&free, other))
            .or_else(|| self.take_cover(&free, other))
            .or_else(|| self.advance(&free, other))
            .or_else(|| self.flagship_dodge(&free, other, rng))
            .or_else(|| {
                (rng.random::<f64>() < SURPRISE_CHANCE)
                    .then(|| self.dodge(&free, other))
                    .flatten()
            });
        chosen.map(|chosen| Maneuver {
            ship: chosen.from.target(),
            destination: chosen.to.target(),
        })
    }

    /// Ausweichen vor dem vermuteten Feuer: Der Gegner zielt auf die Spalten beim Planen. Droht einem wertvollen
    /// eigenen Schiff dort der Fall oder schwerer Schaden ([`EVADE`]), zuerst dem Flaggschiff, wechselt es in einen
    /// freien Slot einer anderen Spalte ([`Situation::evasions`]). Jeder Slot wird im Probekampf durchgespielt, samt dem
    /// Schuss, den das Schiff dafür verliert, und dem, was die Schüsse auf seine alte Spalte jetzt treffen; ein
    /// gedeckter Slot zählt etwas mehr. Unter den Slots, die fast so viel bringen wie der beste, entscheidet der Zufall,
    /// und geht es nicht um das Match, bleibt das Schiff manchmal stehen: So kann der Gegner nicht einfach vorhalten.
    /// Ein Kampfschiff weicht nur aus, wenn ein anderes weiter feuert, ein Wächter vor dem Flaggschiff nie, die
    /// Pendulum nur, wenn sie sonst fällt.
    fn evade(
        &self,
        movable: &[SlotRef],
        other: Option<Move>,
        rng: &mut dyn RngCore,
    ) -> Option<Move> {
        let board = &self.board;
        let flagship = self.flagship;
        let mut threatened: Vec<SlotRef> = movable
            .iter()
            .copied()
            .filter(|&at| {
                let pendulum = card(board, at) == ShipId::Pendulum;
                !is_token(board, at)
                    && (self.dies(at) || self.hit(at) >= EVADE.hit)
                    && (!pendulum || self.dies(at))
                    && (Some(at) == flagship
                        || ((!attacks(board, at) || self.attacker_remains(at)) && !self.guards(at)))
            })
            .collect();
        threatened.sort_by(|a, b| {
            (Some(*b) == flagship)
                .cmp(&(Some(*a) == flagship))
                .then(self.loss(*b).total_cmp(&self.loss(*a)))
        });
        for ship in threatened {
            let options: Vec<(SlotRef, f64, f64)> = self
                .evasions(ship, movable, other)
                .into_iter()
                .map(|destination| {
                    let trial = Trial {
                        maneuver: Some(Maneuver {
                            ship: ship.target(),
                            destination: destination.target(),
                        }),
                        ..Trial::default()
                    };
                    let gain = self.outcome(&trial) - self.standing();
                    let cover = if self.covered(destination, ship, other) {
                        EVADE.cover
                    } else {
                        0.0
                    };
                    (destination, gain, gain + cover)
                })
                .filter(|&(_, gain, _)| gain >= EVADE.gain)
                .collect();
            let Some(best) = options.iter().map(|&(_, _, score)| score).reduce(f64::max) else {
                continue;
            };
            let good: Vec<(SlotRef, f64, f64)> = options
                .into_iter()
                .filter(|&(_, _, score)| score >= best - EVADE.margin)
                .collect();
            let pick = (rng.random::<f64>() * good.len() as f64) as usize;
            let (destination, gain, _) = good[pick.min(good.len() - 1)];
            // Rettet es das Flaggschiff, weicht es immer aus.
            let decisive = Some(ship) == flagship && self.dies(ship) && gain >= DECISIVE / 2.0;
            if !decisive && rng.random::<f64>() < EVADE.stay {
                return None;
            }
            return Some(Move {
                from: ship,
                to: destination,
            });
        }
        None
    }

    /// Wohin ein Schiff ausweichen kann: in einen freien Slot einer anderen Spalte, das Flaggschiff nur in die
    /// Command-Lane; die Mirage tauscht auch.
    fn evasions(&self, ship: SlotRef, movable: &[SlotRef], other: Option<Move>) -> Vec<SlotRef> {
        let mut destinations: Vec<SlotRef> = self
            .destinations(ship, other)
            .into_iter()
            .filter(|to| {
                to.slot_index != ship.slot_index
                    && (Some(ship) != self.flagship || to.lane == LaneType::Command)
            })
            .collect();
        if card(&self.board, ship) == ShipId::Mirage {
            destinations.extend(
                movable
                    .iter()
                    .copied()
                    .filter(|&to| to.slot_index != ship.slot_index && !is_token(&self.board, to)),
            );
        }
        destinations
    }

    /// Ein Strike-Schiff vor dem eigenen Flaggschiff.
    fn guards(&self, at: SlotRef) -> bool {
        at.lane == LaneType::Strike && Some(super::partner(at)) == self.flagship
    }

    /// Das Flaggschiff in Sicherheit: ungedeckt, sobald ein Gegner darauf feuern kann (auch die Railgun,
    /// [`Situation::fire_threat`]), oder vorne angeschlagen, geht es hinter ein Strike-Schiff. Die Mirage tauscht
    /// lieber ([`Situation::phase_swap`]); die Pendulum bewegt sich nur, wenn ihr der Fall droht.
    fn protect_flagship(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        let flagship = self
            .flagship
            .filter(|flagship| movable.contains(flagship))?;
        let ship = card(&self.board, flagship);
        if ship == ShipId::Mirage || !self.exposed(flagship) {
            return None;
        }
        let threat = self.fire_threat(flagship);
        let durability = self.durability(flagship);
        let falls = threat >= durability;
        let in_danger = if flagship.lane == LaneType::Strike {
            falls || durability <= FLAGSHIP_RETREAT_HP
        } else {
            threat > 0
        };
        if !in_danger || (ship == ShipId::Pendulum && !falls) {
            return None;
        }
        let to = self.shelter(flagship, other)?;
        Some(Move { from: flagship, to })
    }

    /// Ein markiertes Schiff (die gegnerische Railgun feuert in dieser Runde) geht in Deckung, wenn ihr Schuss es
    /// zerstören kann.
    fn evade_mark(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        let marked = movable
            .iter()
            .copied()
            .find(|&at| self.board.ship(at).marked && !is_flagship(&self.board, at))?;
        if !self.exposed(marked) || self.durability(marked) > ABILITY_VALUES.railgun + 1 {
            return None;
        }
        let to = self.shelter(marked, other)?;
        Some(Move { from: marked, to })
    }

    /// Phasentausch der Mirage: Ungedeckt unter Beschuss tauscht sie mit einem robusten Schiff in Deckung; droht ein
    /// Torpedo, tauscht sie manchmal einfach den Platz, damit er das andere Schiff trifft.
    fn phase_swap(
        &self,
        movable: &[SlotRef],
        other: Option<Move>,
        rng: &mut dyn RngCore,
    ) -> Option<Move> {
        let mirage = self
            .flagship
            .filter(|&at| card(&self.board, at) == ShipId::Mirage && movable.contains(&at))?;
        let partners: Vec<SlotRef> = movable
            .iter()
            .copied()
            .filter(|&at| at != mirage && other.is_none_or(|other| other.from != at))
            .collect();
        let covered: Vec<SlotRef> = partners
            .iter()
            .copied()
            .filter(|&at| !self.exposed(at))
            .collect();
        let durability = |at: &SlotRef| self.durability(*at);
        if self.exposed(mirage) && self.fire_threat(mirage) > 0 {
            let robust = covered.iter().copied().filter(|&at| {
                self.durability(at) > self.hit(mirage) + 1 && !is_token(&self.board, at)
            });
            if let Some(partner) = highest(robust, durability) {
                return Some(Move {
                    from: mirage,
                    to: partner,
                });
            }
        }
        let torpedo =
            self.view.opponent.command_hand_size > 0 && unplayed(self.view, CommandId::Torpedo) > 0;
        if !torpedo || rng.random::<f64>() >= SWAP_CHANCE {
            return None;
        }
        let candidates = if self.exposed(mirage) {
            partners
        } else {
            covered
        };
        let partner = highest(candidates, durability)?;
        Some(Move {
            from: mirage,
            to: partner,
        })
    }

    /// Kampfschiffe vorne, nachdem `other` die Lane gewechselt hat; `except` zählt nicht mit.
    fn attackers_in_front(&self, except: Option<SlotRef>, other: Option<Move>) -> usize {
        self.own
            .iter()
            .copied()
            .filter(|&at| {
                let lane = match other {
                    Some(other) if other.from == at => other.to.lane,
                    _ => at.lane,
                };
                Some(at) != except && attacks(&self.board, at) && lane == LaneType::Strike
            })
            .count()
    }

    /// Ob außer `except` noch ein Kampfschiff in dieser Runde feuert.
    pub(super) fn attacker_remains(&self, except: SlotRef) -> bool {
        self.own
            .iter()
            .any(|&at| at != except && attacks(&self.board, at))
    }

    /// Rückzug: Ein angeschlagenes Strike-Schiff, das sonst im Kampf fällt, geht in einen gedeckten Command-Slot, wenn
    /// der Verlust schwerer wiegt als sein verschenkter Schuss, vorne ein Angreifer bleibt und dahinter niemand
    /// ungedeckt zurückbleibt.
    fn retreat(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        let candidates = movable.iter().copied().filter(|&at| {
            at.lane == LaneType::Strike
                && damaged(&self.board, at)
                && self.dies(at)
                && self.loss(at) > self.shot_value(at)
                && !self.uncovers(at)
                && self.attackers_in_front(Some(at), other) > 0
                && self
                    .lane_switch(at, other)
                    .is_some_and(|(_, paired)| paired)
        });
        let ship = highest(candidates, |&at| self.loss(at) - self.shot_value(at))?;
        let (to, _) = self.lane_switch(ship, other).expect("es bildet ein Paar");
        Some(Move { from: ship, to })
    }

    /// Deckung: Ein ungedecktes Command-Schiff, das im Kampf mehr verliert, als sein Schuss bringt, rückt innerhalb der
    /// Lane hinter ein Strike-Schiff, das stehen bleibt (oder hinter das Ziel von `other`).
    fn take_cover(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        let covered = |destination: SlotRef| {
            let front = destination.in_lane(LaneType::Strike);
            other.is_some_and(|other| other.to == front)
                || (self.board.is_occupied(front) && other.is_none_or(|other| other.from != front))
        };
        let shelter = |at: SlotRef| {
            let destinations = self
                .destinations(at, other)
                .into_iter()
                .filter(|&destination| destination.lane == at.lane && covered(destination));
            nearest(destinations, at.slot_index)
        };
        let candidates = movable.iter().copied().filter(|&at| {
            at.lane == LaneType::Command
                && self.exposed(at)
                && self.loss(at) > self.shot_value(at)
                && shelter(at).is_some()
        });
        let ship = highest(candidates, |&at| self.loss(at) - self.shot_value(at))?;
        let to = shelter(ship).expect("es hat Deckung");
        Some(Move { from: ship, to })
    }

    /// Vorrücken: Greift vorne niemand an, deckt ein robustes Command-Schiff (kein Flaggschiff) ein anderes.
    fn advance(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        if self.enemy_attack == 0 || self.attackers_in_front(None, other) > 0 {
            return None;
        }
        let board = &self.board;
        let candidates = movable.iter().copied().filter(|&at| {
            at.lane == LaneType::Command
                && !is_flagship(board, at)
                && attacks(board, at)
                && !is_fragile(card(board, at))
                && self.durability(at) > self.enemy_max_attack
                && self.attacker_remains(at)
                && self
                    .lane_switch(at, other)
                    .is_some_and(|(_, paired)| paired)
        });
        let ship = highest(candidates, |&at| hp(board, at))?;
        let (to, _) = self.lane_switch(ship, other).expect("es bildet ein Paar");
        Some(Move { from: ship, to })
    }

    /// Das Flaggschiff weicht einem vermuteten Torpedo in einen anderen gedeckten Command-Slot aus, angeschlagen öfter.
    fn flagship_dodge(
        &self,
        movable: &[SlotRef],
        other: Option<Move>,
        rng: &mut dyn RngCore,
    ) -> Option<Move> {
        let flagship = self
            .flagship
            .filter(|at| movable.contains(at) && at.lane == LaneType::Command)?;
        if self.view.opponent.command_hand_size == 0 || unplayed(self.view, CommandId::Torpedo) == 0
        {
            return None;
        }
        let chance = if self.durability(flagship) <= 10 {
            0.3
        } else {
            0.1
        };
        if rng.random::<f64>() >= chance {
            return None;
        }
        let to = self.shelter(flagship, other)?;
        Some(Move { from: flagship, to })
    }

    /// Ausweichen: das Schiff (kein Flaggschiff), auf das ein gegnerischer Torpedo am ehesten zielt, wechselt die Lane.
    fn dodge(&self, movable: &[SlotRef], other: Option<Move>) -> Option<Move> {
        let settled = self.settled(self.own.iter().copied());
        let ship = highest(
            settled
                .into_iter()
                .filter(|&at| !is_flagship(&self.board, at)),
            |&at| self.torpedo_value(at),
        )?;
        if !movable.contains(&ship) || self.uncovers(ship) || !self.attacker_remains(ship) {
            return None;
        }
        let (to, _) = self.lane_switch(ship, other)?;
        Some(Move { from: ship, to })
    }
}
