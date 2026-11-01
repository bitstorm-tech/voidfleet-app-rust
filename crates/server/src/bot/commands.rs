//! Der Befehl der Runde: der mit dem größten geschätzten Nutzen ([`Situation::plan_command`]).

use rand::{Rng, RngCore};
use voidfleet_core::admirals::{ABILITY_VALUES, AdmiralId};
use voidfleet_core::board::{LaneType, MAX_SHIPS_PER_SIDE, PlannedMove, SlotRef, max_deploys};
use voidfleet_core::cards::{COMMANDS, CommandId, CommandType, ShipId};
use voidfleet_core::powers::power_targets;
use voidfleet_core::resolution::PowerUse;

use super::situation::{Choice, Situation, Trial, TrialCommand};
use super::{
    COMBOS, COUNTER, DECISIVE, FLAGSHIP_WEIGHT, FLEE_CHANCE, GUESS, SURPRISE_CHANCE, UNSURE,
    attacks, card, damaged, highest, hp, is_flagship, is_token, lowest, partner, stays, unplayed,
};

impl Situation<'_> {
    /// Der Befehl mit dem größten geschätzten Nutzen samt Hand-Index; `None`, wenn keiner etwas bringt.
    pub(super) fn plan_command(&self, rng: &mut dyn RngCore) -> Option<(usize, Choice)> {
        let mut best: Option<(usize, Choice)> = None;
        for (hand_index, &card) in self.view.command_hand.iter().enumerate() {
            let Some(choice) = self.evaluate(card, rng) else {
                continue;
            };
            if choice.score > best.map_or(0.0, |(_, best)| best.score) {
                best = Some((hand_index, choice));
            }
        }
        best
    }

    /// Bewertet den Befehl ([`Situation::estimate`]); bei den [`COMBOS`] zählt der Probekampf, wenn er mehr bringt.
    fn evaluate(&self, card: CommandId, rng: &mut dyn RngCore) -> Option<Choice> {
        let choice = self.estimate(card, rng)?;
        if !COMBOS.contains(&card) {
            return Some(choice);
        }
        let simulated = self.simulated(card, choice.target, choice.destination);
        Some(Choice {
            score: choice.score.max(simulated),
            ..choice
        })
    }

    /// Schätzt den Nutzen des Befehls; `None` oder Score 0, wenn er nichts bringt. Flaggschiffe bewegt er per Befehl
    /// nicht nach vorne.
    fn estimate(&self, card: CommandId, rng: &mut dyn RngCore) -> Option<Choice> {
        let board = &self.board;
        let enemy = self.side.opposite();
        let enemies = || self.enemies.iter().copied();
        let own = || self.own.iter().copied();
        let strike_count = board.occupied(self.side, LaneType::Strike).count();
        let half = |sure: bool| if sure { 1.0 } else { UNSURE };
        match card {
            CommandId::OrbitalStrike => {
                let target = highest(enemies(), |&at| hp(board, at))?;
                Some(Choice::untargeted(self.damage_value(target, 4)))
            }
            CommandId::VolleyFire => Some(Choice::untargeted(
                enemies().map(|at| self.damage_value(at, 1)).sum(),
            )),
            CommandId::Torpedo => {
                // Feuert die eigene Railgun auf ein Schiff hinter schwacher Deckung, räumt der Torpedo die Deckung weg.
                if let Some((front, marked)) = self.mark_cover() {
                    let score = self.damage_value(front, 5)
                        + self.damage_value(marked, ABILITY_VALUES.railgun);
                    return Some(Choice::on(front, score));
                }
                // Er trifft, wer nach allen Bewegungen im Slot steht: meist ein Schiff, das wohl stehen bleibt (im
                // Probekampf samt dem, was er dem eigenen Feuer ermöglicht), ab und zu der Slot, in den sich ein
                // angeschlagenes Strike-Schiff am ehesten zurückzieht.
                let surprise = rng.random::<f64>() < SURPRISE_CHANCE;
                if let Some(retreat) = surprise.then(|| self.likely_retreat()).flatten() {
                    let score = self.damage_value(retreat.from, 5) * UNSURE;
                    return Some(Choice::on(retreat.to, score));
                }
                let value = |at: SlotRef| {
                    let simulated = self.simulated(card, Some(at), None);
                    self.torpedo_value(at)
                        .max(simulated * half(stays(board, at)))
                };
                let (target, score) =
                    highest(enemies().map(|at| (at, value(at))), |&(_, score)| score)?;
                Some(Choice::on(target, score))
            }
            CommandId::Barrage => {
                // Trifft nur, wer die Lane wechselt: am ehesten angeschlagene Gegner, die sonst im Kampf fallen. Mehr
                // als zwei Bewegungen (Manöver und Befehl) hat keine Seite.
                let mut values: Vec<f64> = enemies()
                    .filter(|&at| damaged(board, at) && self.dies(at))
                    .map(|at| FLEE_CHANCE * self.damage_value(at, 3))
                    .collect();
                values.sort_by(|a, b| b.total_cmp(a));
                let score = if self.enemies.is_empty() {
                    0.0
                } else {
                    0.5 + values.iter().take(2).sum::<f64>()
                };
                Some(Choice::untargeted(score))
            }
            CommandId::EmpBlast => {
                let target = highest(enemies(), |&at| board.attack(at))?;
                Some(Choice::untargeted(f64::from(board.attack(target))))
            }
            CommandId::TargetingJammer => {
                let armed = board
                    .occupied(enemy, LaneType::Strike)
                    .filter(|&at| board.attack(at) > 0)
                    .count();
                Some(Choice::untargeted(armed as f64))
            }
            CommandId::PowerSurge => {
                // Ohne gegnerisches Schiff beim Planen (Runde 1) bekommt niemand einen Feuerbefehl.
                let armed = board
                    .occupied(self.side, LaneType::Strike)
                    .filter(|&at| attacks(board, at))
                    .count();
                (!self.enemies.is_empty()).then(|| Choice::untargeted(armed as f64))
            }
            CommandId::EmergencyShields => {
                // Droht dem Flaggschiff der Fall und rettet es der Schild, bekommt es ihn.
                if let Some(flagship) = self.flagship {
                    let threat = self.flagship_threat();
                    let durability = self.durability(flagship);
                    if threat >= durability && threat - 3 < durability {
                        return Some(Choice::on(flagship, DECISIVE / 2.0));
                    }
                }
                // Sonst dorthin, wo das vermutete Feuer im Probekampf am meisten anrichtet, auch auf ein frisch
                // gelegtes Schiff vorne, das die Schüsse auf seine Spalte abfängt, oder ein gedecktes, zu dem Overkill
                // durchgeht.
                self.best_target(card, own())
            }
            CommandId::Overcharge => {
                // +3 ATK gegen 2 Eigenschaden: nur für ein Kampfschiff, das das und einen Treffer übersteht, auch für
                // eines, das gerade vorne landet; nie für das Flaggschiff, nie ohne gegnerisches Ziel (Runde 1).
                if self.enemies.is_empty() {
                    return None;
                }
                let sturdy = own().filter(|&at| {
                    let threat = if self.exposed(at) {
                        self.enemy_max_attack
                    } else {
                        0
                    };
                    attacks(board, at)
                        && !is_flagship(board, at)
                        && (!self.fresh.contains(&at) || at.lane == LaneType::Strike)
                        && hp(board, at) > 2 + threat
                });
                let target = highest(sturdy, |&at| hp(board, at))?;
                Some(Choice::on(target, 2.0))
            }
            CommandId::ScrapSalvage => {
                // +1 ATK für immer: für den Scrap Raider, der am längsten durchhält.
                let raiders = own().filter(|&at| card.can_target_ship(self::card(board, at)));
                let target = highest(raiders, |&at| hp(board, at))?;
                Some(Choice::on(target, 1.5))
            }
            CommandId::FlankingManeuver => {
                // Das robusteste Schiff nach vorne, wenn es damit ein anderes deckt; sonst ein angeschlagenes Schiff
                // vorne in einen gedeckten Slot bringen, solange vorne jemand bleibt.
                let command = self.settled(board.occupied(self.side, LaneType::Command));
                let advance = highest(
                    command
                        .into_iter()
                        .filter(|&at| !is_flagship(board, at) && self.pairs_after_move(at)),
                    |&at| hp(board, at),
                );
                if let Some(advance) = advance {
                    return self.moving(advance, 2.0);
                }
                let strike = self.settled(board.occupied(self.side, LaneType::Strike));
                let retreat = lowest(strike, |&at| hp(board, at))?;
                (self.endangered(retreat) && self.pairs_after_move(retreat) && strike_count >= 2)
                    .then(|| self.moving(retreat, 1.5))
                    .flatten()
            }
            CommandId::Intercept => {
                // Holt ein Schiff nach vorne: +2 ATK für ein Kampfschiff, dazu Deckung für die anderen. Ein frisch in
                // die Command-Lane gelegtes Kampfschiff darf sofort mit.
                let candidates = board.occupied(self.side, LaneType::Command).filter(|&at| {
                    (!self.fresh.contains(&at) || attacks(board, at)) && !is_flagship(board, at)
                });
                let target = highest(candidates, |&at| {
                    (if attacks(board, at) { 100 } else { 0 }) + hp(board, at)
                })?;
                let score = (if attacks(board, target) { 1.5 } else { 0.0 })
                    + if self.pairs_after_move(target) {
                        2.0
                    } else {
                        0.0
                    };
                self.moving(target, score)
            }
            CommandId::Recall => {
                // Zieht ein angeschlagenes Schiff nach hinten und heilt es; vorne muss aber jemand bleiben, gelegte
                // Schiffe zählen mit.
                let strike = self.settled(board.occupied(self.side, LaneType::Strike));
                let target = lowest(strike, |&at| hp(board, at))?;
                if strike_count < 2 {
                    return None;
                }
                let healing = 2.min(self::card(board, target).hp() - hp(board, target));
                let rescue = self.endangered(target) && self.pairs_after_move(target);
                self.moving(target, f64::from(healing) + if rescue { 1.5 } else { 0.0 })
            }
            CommandId::Scavenge => {
                // Nachziehen lohnt sich, solange die Schiffshand dünn ist; eine volle Hand verschluckt die Karte.
                let thin = self.view.ship_hand.len() <= 3;
                Some(Choice::untargeted(if thin { 1.5 } else { 0.5 }))
            }
            CommandId::EvasiveAction => {
                // Ausweichen innerhalb der Lane samt Schild 2: für ein Schiff unter Beschuss, in den Slot, der im
                // Probekampf am meisten bringt.
                let rescue = |at: SlotRef| Some(at) == self.flagship && self.rescues(2);
                let mut best: Option<Choice> = None;
                let ships = own().filter(|&at| {
                    self::card(board, at) != ShipId::Pendulum && (self.hit(at) > 0 || rescue(at))
                });
                for ship in ships {
                    let destinations = self
                        .destinations(ship, None)
                        .into_iter()
                        .filter(|destination| destination.lane == ship.lane);
                    for destination in destinations {
                        let simulated = self.simulated(card, Some(ship), Some(destination));
                        let score = simulated.max(if rescue(ship) { DECISIVE / 2.0 } else { 0.0 });
                        if score > best.map_or(0.0, |best| best.score) {
                            best = Some(Choice {
                                score,
                                target: Some(ship),
                                destination: Some(destination),
                            });
                        }
                    }
                }
                best
            }
            CommandId::MicroJump => {
                // Droht dem Flaggschiff der Fall, springt es (reicht ein Notschild, ist der lieber); sonst ein Schiff,
                // das im Kampf mehr verliert, als sein Schuss bringt. Die Pendulum springt nur zur Rettung, sonst fiele
                // ihre Railgun aus.
                if let Some(flagship) = self.flagship
                    && self.flagship_threat() >= self.durability(flagship)
                {
                    let shield = if self.rescues(3) { 1.0 } else { 0.0 };
                    return Some(Choice::on(flagship, DECISIVE / 2.0 - shield));
                }
                self.best_target(
                    card,
                    own().filter(|&at| {
                        self::card(board, at) != ShipId::Pendulum && self.hit(at) > 0
                    }),
                )
            }
            CommandId::RapidDeployment => self.extra_ship.map(Choice::untargeted),
            CommandId::DrawFire => self.best_target(card, own().filter(|&at| self.exposed(at))),
            CommandId::ShieldGrid => {
                let rescue = if self.rescues(1) { DECISIVE / 2.0 } else { 0.0 };
                Some(Choice::untargeted(
                    self.simulated(card, None, None).max(rescue),
                ))
            }
            CommandId::AblativeArmor | CommandId::CoveringFire => {
                // Rettet der Schild das Flaggschiff (Feuerschutz auch über seinen Partner), geht er dorthin; sonst
                // wohin er im Probekampf am meisten bringt.
                let shield = if card == CommandId::AblativeArmor {
                    2
                } else {
                    1
                };
                if let Some(flagship) = self.flagship
                    && self.rescues(shield)
                {
                    return Some(Choice::on(flagship, DECISIVE / 2.0));
                }
                self.best_target(card, own())
            }
            CommandId::DroneSquadron => Some(Choice::untargeted(self.simulated(card, None, None))),
            CommandId::CollisionCourse => self.best_target(
                card,
                board
                    .occupied(self.side, LaneType::Strike)
                    .filter(|&at| card.can_target_ship(self::card(board, at))),
            ),
            CommandId::Countermeasures => {
                // Lässt einen Angriff oder eine Schwächung des Gegners verpuffen, sonst zieht er einen Befehl. Wackelt
                // das eigene Flaggschiff erst durch einen Befehl, kann sie das Match retten.
                let chance = self.hostile_chance();
                let decisive = self.flagship.is_some_and(|flagship| {
                    let durability = self.durability(flagship);
                    self.fire_threat(flagship) < durability && self.flagship_threat() >= durability
                });
                let countered = if decisive {
                    DECISIVE / 2.0
                } else {
                    COUNTER.typical
                };
                Some(Choice::untargeted(
                    chance * countered + (1.0 - chance) * COUNTER.draw,
                ))
            }
            CommandId::CarteBlanche => {
                // Grundmanöver und Fähigkeit zusammen: Sie bringt, was das schwächere der beiden zusätzlich bringt.
                let maneuver = self.plan_maneuver(None, rng)?;
                let (_, gain) = self.best_power(None, Some(maneuver))?;
                Some(Choice::untargeted(self.maneuver_gain(maneuver).min(gain)))
            }
            CommandId::RadioSilence => {
                // Die gegnerische Fähigkeit dieser Runde verpufft: so oft, wie er sie einsetzt, mal ihr Wert an seinem
                // besten Ziel.
                let id = self.view.opponent.admiral?.power();
                let loss = power_targets(board, enemy, id, &[], &[])
                    .into_iter()
                    .map(|at| {
                        let trial = Trial {
                            enemy_power: Some(PowerUse {
                                id,
                                target: at.target(),
                            }),
                            ..Trial::default()
                        };
                        self.standing() - self.outcome(&trial)
                    })
                    .fold(0.0, f64::max);
                Some(Choice::untargeted(GUESS.power * loss))
            }
            CommandId::Minefield => {
                // Trifft, was beim Gegner in dieser Runde ankommt: meist ein Schiff, in Runde 1 zwei und das
                // Flaggschiff, bei Moreau manchmal ein Trugbild.
                let flagship = if self.view.round == 1 {
                    f64::from(3 * FLAGSHIP_WEIGHT)
                } else {
                    0.0
                };
                let decoy = if self.view.opponent.admiral == Some(AdmiralId::Moreau) {
                    GUESS.decoy
                } else {
                    0.0
                };
                let deploys = self.enemy_deploys() as f64;
                Some(Choice::untargeted(
                    deploys * GUESS.deploy * 3.0 + flagship + decoy,
                ))
            }
            CommandId::TractorBeam => {
                // Zieht ein Schiff in die andere Lane, am liebsten den Wächter vor dem Flaggschiff.
                self.best_target(
                    card,
                    enemies().filter(|&at| card.can_target_ship(self::card(board, at))),
                )
            }
            CommandId::HeatSeeker => {
                // Folgt seinem Ziel: 3 Schaden, 5 an einem Schiff, das wohl ausweicht; im Probekampf samt dem, was er
                // dem eigenen Feuer ermöglicht.
                let value = |at: SlotRef| {
                    let amount = if self.flees(at) { 5 } else { 3 };
                    self.damage_value(at, amount)
                        .max(self.simulated(card, Some(at), None))
                };
                let (target, score) =
                    highest(enemies().map(|at| (at, value(at))), |&(_, score)| score)?;
                Some(Choice::on(target, score))
            }
            CommandId::CoupDeGrace => {
                let ships = enemies().filter(|&at| !is_token(board, at));
                let target = lowest(ships, |&at| hp(board, at))?;
                Some(Choice::untargeted(self.damage_value(target, 4)))
            }
            CommandId::PiercingShot => {
                // Wie der Torpedo nach allen Bewegungen: 3 am vordersten Schiff der Slot-Nummer, 2 am Command-Schiff
                // dahinter.
                let sure = |at: SlotRef, amount: i32| {
                    self.damage_value(at, amount) * half(stays(board, at))
                };
                let column = |front: SlotRef| {
                    let back = partner(front);
                    let behind = |amount: i32| {
                        if board.is_occupied(back) {
                            sure(back, amount)
                        } else {
                            0.0
                        }
                    };
                    if board.is_occupied(front) {
                        sure(front, 3) + behind(2)
                    } else {
                        behind(3)
                    }
                };
                // Im Probekampf samt dem, was er dem eigenen Feuer ermöglicht, halb gewichtet, wenn ein Schiff der
                // Slot-Nummer wohl ausweicht.
                let stand = |front: SlotRef| {
                    [front, partner(front)]
                        .into_iter()
                        .all(|at| !board.is_occupied(at) || stays(board, at))
                };
                let value = |front: SlotRef| {
                    let damage = column(front);
                    if damage > 0.0 {
                        damage.max(self.simulated(card, Some(front), None) * half(stand(front)))
                    } else {
                        0.0
                    }
                };
                let columns = voidfleet_core::board::lane_slots(enemy, LaneType::Strike);
                let (target, score) =
                    highest(columns.map(|at| (at, value(at))), |&(_, score)| score)?;
                Some(Choice::on(target, score))
            }
            CommandId::Bombardment => Some(Choice::untargeted(
                board
                    .occupied(enemy, LaneType::Command)
                    .map(|at| self.damage_value(at, 2))
                    .sum(),
            )),
            CommandId::Crossfire => {
                // Ein Schiff, das der Gegner in dieser Runde legt, gibt einem ungepaarten oft einen Partner.
                let unpaired: f64 = enemies()
                    .filter(|&at| !board.is_occupied(partner(at)))
                    .map(|at| self.damage_value(at, 3))
                    .sum();
                Some(Choice::untargeted(
                    unpaired * half(self.enemy_deploys() == 0),
                ))
            }
        }
    }

    /// Wie viele Schiffe der Gegner in dieser Runde höchstens legt: so viele, wie Hand, Flottengrenze und Runde
    /// erlauben.
    fn enemy_deploys(&self) -> usize {
        let room = MAX_SHIPS_PER_SIDE - self.board.ship_count(self.side.opposite());
        self.view
            .opponent
            .ship_hand_size
            .min(room)
            .min(max_deploys(self.view.round, None))
    }

    /// Wie wahrscheinlich der Gegner einen Angriff oder eine Schwächung spielt: ob er einen hält (aus seinen
    /// ungespielten Kopien) und spielt.
    fn hostile_chance(&self) -> f64 {
        let mut total = 0;
        let mut hostile = 0;
        for card in COMMANDS {
            let copies = unplayed(self.view, card);
            total += copies;
            if matches!(
                card.command_type(),
                CommandType::Attack | CommandType::Debuff
            ) {
                hostile += copies;
            }
        }
        if total == 0 {
            return 0.0;
        }
        let peaceful = 1.0 - f64::from(hostile) / f64::from(total);
        let hand = i32::try_from(self.view.opponent.command_hand_size).expect("kleine Hand");
        GUESS.hostile * (1.0 - peaceful.powi(hand))
    }

    /// Was der Befehl im Kampf dieser Runde bringt: auf einer Kopie angewendet, verglichen mit der Lage ohne ihn
    /// ([`Situation::standing`]).
    pub(super) fn simulated(
        &self,
        card: CommandId,
        target: Option<SlotRef>,
        destination: Option<SlotRef>,
    ) -> f64 {
        let trial = Trial {
            command: Some(TrialCommand {
                card,
                target,
                destination,
            }),
            ..Trial::default()
        };
        self.outcome(&trial) - self.standing()
    }

    /// Das gültige Ziel, auf dem der Befehl im Probekampf am meisten bringt; `None`, wenn er nirgends etwas bringt.
    fn best_target(
        &self,
        card: CommandId,
        targets: impl IntoIterator<Item = SlotRef>,
    ) -> Option<Choice> {
        let mut best: Option<Choice> = None;
        let valid = targets.into_iter().filter(|&at| {
            card.can_target_lane(at.lane) && card.can_target_ship(self::card(&self.board, at))
        });
        for target in valid {
            let score = self.simulated(card, Some(target), None);
            if score > best.map_or(0.0, |best| best.score) {
                best = Some(Choice::on(target, score));
            }
        }
        best
    }

    /// Feuert die eigene Railgun in dieser Runde auf ein Command-Schiff hinter einer Deckung, die ein Torpedo zerstört:
    /// Deckung und Ziel.
    fn mark_cover(&self) -> Option<(SlotRef, SlotRef)> {
        let marked = self
            .enemies
            .iter()
            .copied()
            .find(|&at| self.board.ship(at).marked)
            .filter(|at| at.lane == LaneType::Command)?;
        let front = marked.in_lane(LaneType::Strike);
        (self.board.is_occupied(front) && self.durability(front) <= 5).then_some((front, marked))
    }

    /// Wie gern ein Torpedo auf dieses Schiff zielt: der Schaden, halb für eines, das ausweichen könnte ([`stays`]).
    pub(super) fn torpedo_value(&self, at: SlotRef) -> f64 {
        let sure = if stays(&self.board, at) { 1.0 } else { UNSURE };
        self.damage_value(at, 5) * sure
    }

    /// Ob ein angeschlagenes gegnerisches Schiff, das sonst fällt, wohl die Lane wechselt (Wärmesucher).
    fn flees(&self, at: SlotRef) -> bool {
        damaged(&self.board, at) && self.dies(at) && self.pairs_after_move(at)
    }

    /// Wohin sich ein angeschlagenes gegnerisches Strike-Schiff, das sonst fällt, am ehesten zurückzieht.
    fn likely_retreat(&self) -> Option<PlannedMove> {
        let fleeing = self
            .board
            .occupied(self.side.opposite(), LaneType::Strike)
            .filter(|&at| self.flees(at));
        let ship = highest(fleeing, |&at| self.damage_value(at, 5))?;
        let (to, _) = self.lane_switch(ship, None).expect("es bildet ein Paar");
        Some(PlannedMove { from: ship, to })
    }

    /// Befehl auf ein Schiff, das dabei die Lane wechselt; `None`, wenn die andere Lane keinen Platz hat.
    fn moving(&self, at: SlotRef, score: f64) -> Option<Choice> {
        let (destination, _) = self.lane_switch(at, None)?;
        Some(Choice {
            score,
            target: Some(at),
            destination: Some(destination),
        })
    }
}
