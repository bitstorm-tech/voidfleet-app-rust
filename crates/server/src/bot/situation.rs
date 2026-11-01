//! Die Lage nach dem Deploy ([`Situation`]) und der Probekampf, mit dem der Bot Befehl, Manöver, Fähigkeit und
//! Feuerbefehle bewertet.

use std::cell::OnceCell;
use std::collections::HashMap;

use voidfleet_core::admirals::{ABILITY_VALUES, PowerId};
use voidfleet_core::board::{
    Aim, Board, LaneType, PlannedMove, Side, SlotRef, SlotTarget, nearest,
};
use voidfleet_core::cards::{CommandId, ShipId};
use voidfleet_core::powers::power_targets;
use voidfleet_core::protocol::{FireOrder, FireTarget, Maneuver, PowerPlan, RoundEvent};
use voidfleet_core::resolution::{
    Drawn, PlayedCommand, PowerUse, bind_fire, firing_railguns, resolve_cleanup, resolve_combat,
    resolve_effects, resolve_guardian_shields, resolve_maneuver, resolve_power, resolve_swap,
};

use super::fire::{Target, aim_fire};
use super::threat::{Lead, Risk, SAFE, guard_of, risks};
use super::{
    BotView, DECISIVE, FLAGSHIP_WEIGHT, KERR, KILL_BONUS, POWER, RAPID_TEMPO, attacks,
    damage_value, durability, highest, hp, is_flagship, is_token, lowest, unplayed,
};

/// Ein Befehl mit dem eigenen Zielschiff (beim Torpedo dem Slot der Gegnerseite) und, bei Befehlen, die ihr Ziel
/// verlegen, dem Slot in der anderen Lane; dazu sein geschätzter Nutzen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Choice {
    pub(super) score: f64,
    pub(super) target: Option<SlotRef>,
    pub(super) destination: Option<SlotRef>,
}

impl Choice {
    pub(super) fn untargeted(score: f64) -> Self {
        Choice {
            score,
            target: None,
            destination: None,
        }
    }

    pub(super) fn on(target: SlotRef, score: f64) -> Self {
        Choice {
            score,
            target: Some(target),
            destination: None,
        }
    }
}

/// Der Befehl im Probekampf: Karte, Ziel und Ziel-Slot (Slots der Kopie mit den gelegten Schiffen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TrialCommand {
    pub(super) card: CommandId,
    pub(super) target: Option<SlotRef>,
    pub(super) destination: Option<SlotRef>,
}

/// Was der Probekampf vom eigenen Zug durchspielt ([`Situation::battle`]): Admiralsfähigkeit, Grundmanöver und Befehl;
/// für die Funkstille stattdessen die Fähigkeit des Gegners.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Trial {
    pub(super) power: Option<SlotTarget>,
    pub(super) maneuver: Option<Maneuver>,
    pub(super) command: Option<TrialCommand>,
    pub(super) enemy_power: Option<PowerUse>,
}

/// Ein Schuss im Probekampf ohne eigenen Zug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shot {
    attacker: SlotRef,
    target: SlotRef,
    amount: i32,
}

/// Die Lage nach dem Deploy: `board` ist eine Kopie, auf der die geplanten Schiffe schon stehen. Befehle dürfen sie
/// treffen, wo es sich lohnt, das Grundmanöver nicht (es bewegt nur Schiffe, die schon standen).
pub(super) struct Situation<'a> {
    pub(super) board: Board,
    pub(super) view: &'a BotView,
    pub(super) side: Side,
    pub(super) own: Vec<SlotRef>,
    pub(super) enemies: Vec<SlotRef>,
    /// Die in dieser Runde gelegten Schiffe, in Runde 1 auch das Flaggschiff.
    pub(super) fresh: Vec<SlotRef>,
    pub(super) enemy_attack: i32,
    pub(super) enemy_max_attack: i32,
    pub(super) flagship: Option<SlotRef>,
    /// Der stärkste Angriffsbefehl, den der Gegner noch auf das eigene Flaggschiff spielen kann
    /// ([`Situation::command_threat`]).
    pub(super) command_danger: i32,
    /// Wie die gegnerischen Schiffe seinen Schüssen wohl entgehen ([`risks`]): danach wählt er Ziel und Modus.
    risks: HashMap<SlotRef, Risk>,
    /// Der Kampf dieser Runde samt Railgun, falls sich niemand bewegt und keiner einen Befehl spielt.
    preview: Board,
    shots: Vec<Shot>,
    /// Schnellverlegung: was das zusätzliche Schiff bringt ([`Situation::arrival_gain`]); `None`, wenn keins mehr
    /// passt.
    pub(super) extra_ship: Option<f64>,
    /// Die Stellung nach dem Kampf dieser Runde ohne Befehl und Manöver ([`Situation::standing`]).
    baseline: OnceCell<f64>,
    /// Das Grundmanöver neben einem Befehl ohne eigenes Zielschiff (`plan_maneuver`), einmal geplant.
    pub(super) free_maneuver: OnceCell<Option<Maneuver>>,
    /// Die vermuteten Feuerbefehle des Gegners ([`super::threat::predict_fire`]), Slots beim Planen aus seiner Sicht.
    enemy_fire: &'a [FireOrder],
    /// Wohin er vorhält; `None`, wenn das gegnerische Flaggschiff nicht fällt.
    lead: Option<&'a Lead>,
}

impl<'a> Situation<'a> {
    /// `arrivals`: die Slots der Schiffe, die in dieser Runde landen.
    pub(super) fn new(
        board: Board,
        view: &'a BotView,
        arrivals: &[SlotTarget],
        enemy_fire: &'a [FireOrder],
        lead: Option<&'a Lead>,
    ) -> Self {
        let side = view.side;
        let enemy = side.opposite();
        let own: Vec<SlotRef> = board.ships(side).collect();
        let enemies: Vec<SlotRef> = board.ships(enemy).collect();
        let attacks = || enemies.iter().map(|&at| board.attack(at));
        let opponent = &view.opponent;
        let mut situation = Situation {
            side,
            fresh: arrivals.iter().map(|target| target.on(side)).collect(),
            enemy_attack: attacks().sum(),
            enemy_max_attack: attacks().max().unwrap_or(0).max(0),
            flagship: board.flagship(side),
            command_danger: 0,
            risks: risks(
                &board,
                enemy,
                guard_of(opponent.ship_hand_size, opponent.admiral),
            ),
            preview: Board::new(),
            shots: Vec::new(),
            extra_ship: None,
            baseline: OnceCell::new(),
            free_maneuver: OnceCell::new(),
            own,
            enemies,
            board,
            view,
            enemy_fire,
            lead,
        };
        situation.command_danger = situation
            .flagship
            .map_or(0, |flagship| situation.command_threat(flagship));
        let (mut preview, _) = situation.battle(&Trial::default(), None);
        let mut events = Vec::new();
        let railguns = firing_railguns(&preview);
        resolve_combat(&mut preview, &mut events, &railguns);
        situation.shots = events
            .into_iter()
            .filter_map(|event| match event {
                RoundEvent::Attack {
                    attacker,
                    target,
                    amount,
                    ..
                } => Some(Shot {
                    attacker: attacker.slot(),
                    target: target.slot(),
                    amount,
                }),
                _ => None,
            })
            .collect();
        situation.preview = preview;
        situation
    }

    /// Der Slot der eigenen Seite.
    pub(super) fn own_slot(&self, lane: LaneType, index: usize) -> SlotRef {
        SlotRef::new(self.side, lane, index)
    }

    /// Ohne die frisch gelegten Schiffe.
    pub(super) fn settled(&self, slots: impl IntoIterator<Item = SlotRef>) -> Vec<SlotRef> {
        slots
            .into_iter()
            .filter(|at| !self.fresh.contains(at))
            .collect()
    }

    pub(super) fn durability(&self, at: SlotRef) -> i32 {
        durability(&self.board, at)
    }

    pub(super) fn damage_value(&self, at: SlotRef, amount: i32) -> f64 {
        damage_value(&self.board, at, amount)
    }

    pub(super) fn exposed(&self, at: SlotRef) -> bool {
        self.board.is_exposed(at, false)
    }

    /// Steht unter Beschuss und ein einzelner Treffer kann es zerstören.
    pub(super) fn endangered(&self, at: SlotRef) -> bool {
        self.exposed(at) && self.durability(at) <= self.enemy_max_attack
    }

    /// Fällt im Kampf dieser Runde, wenn es stehen bleibt.
    pub(super) fn dies(&self, at: SlotRef) -> bool {
        self.preview.ship(at).hp <= 0
    }

    /// Schaden im Kampf dieser Runde (samt Railgun), wenn es stehen bleibt; den Wächterschild verbraucht er zuerst.
    pub(super) fn hit(&self, at: SlotRef) -> i32 {
        let after = self.preview.ship(at);
        (self.durability(at) - after.hp - after.shield).max(0)
    }

    /// Was die Gegenseite im Kampf an dem Schiff gewinnt, wenn es stehen bleibt.
    pub(super) fn loss(&self, at: SlotRef) -> f64 {
        self.damage_value(at, self.hit(at))
    }

    /// Was das Schiff mit seinem Schuss in diesem Kampf anrichtet.
    pub(super) fn shot_value(&self, at: SlotRef) -> f64 {
        self.shots
            .iter()
            .find(|shot| shot.attacker == at)
            .map_or(0.0, |shot| self.damage_value(shot.target, shot.amount))
    }

    /// Was ein eigenes Schiff im Kampf dieser Runde schlimmstenfalls trifft: Jedes gegnerische Kampfschiff kann auf
    /// seine Spalte zielen (beim Fokusfeuer je Treffer 1 mehr). Steht ein Schiff davor, nimmt es die Schüsse in
    /// Kampfreihenfolge, bis es fällt, der Rest geht durch. Dazu die Railgun. Fürs Flaggschiff, dessen Fall
    /// entscheidet; für die übrigen Schiffe zählt der vermutete Kampf ([`Situation::hit`]).
    pub(super) fn fire_threat(&self, at: SlotRef) -> i32 {
        let focused = self
            .view
            .opponent
            .admiral
            .is_some_and(|admiral| admiral.power() == PowerId::FocusFire);
        let focus = if focused {
            ABILITY_VALUES.focus_fire
        } else {
            0
        };
        let front = self.own_slot(LaneType::Strike, at.slot_index);
        let mut cover = if at.lane == LaneType::Command && self.board.is_occupied(front) {
            self.durability(front)
        } else {
            0
        };
        let mut fire = 0;
        for &enemy in self.enemies.iter().filter(|&&at| attacks(&self.board, at)) {
            if cover > 0 {
                cover -= self.board.attack(enemy);
            } else {
                fire += self.board.attack(enemy) + focus;
            }
        }
        let railgun = self.board.ship(at).marked && self.board.is_exposed(at, true);
        fire + if railgun { ABILITY_VALUES.railgun } else { 0 }
    }

    /// Was dem eigenen Flaggschiff in dieser Runde schlimmstenfalls droht: der Kampf und ein starker Befehl, abzüglich
    /// Wächterschild.
    pub(super) fn flagship_threat(&self) -> i32 {
        let Some(flagship) = self.flagship else {
            return 0;
        };
        let guard = if self.board.ship(flagship).card == ShipId::Bastion {
            ABILITY_VALUES.guardian_shield
        } else {
            0
        };
        (self.fire_threat(flagship) + self.command_danger - guard).max(0)
    }

    /// Der stärkste Angriffsbefehl auf das Flaggschiff, von dem der Gegner noch eine Kopie hat.
    fn command_threat(&self, flagship: SlotRef) -> i32 {
        if self.view.opponent.command_hand_size == 0 {
            return 0;
        }
        let board = &self.board;
        let ships = self.own.iter().copied().filter(|&at| !is_token(board, at));
        let strongest = highest(self.own.iter().copied(), |&at| hp(board, at)) == Some(flagship);
        let weakest = lowest(ships, |&at| hp(board, at)) == Some(flagship);
        let threats = [
            (CommandId::Torpedo, 5),
            (CommandId::OrbitalStrike, if strongest { 4 } else { 0 }),
            (CommandId::CoupDeGrace, if weakest { 4 } else { 0 }),
            (CommandId::HeatSeeker, 3),
            (
                CommandId::PiercingShot,
                if self.exposed(flagship) { 3 } else { 2 },
            ),
            (
                CommandId::Crossfire,
                if board.is_occupied(super::partner(flagship)) {
                    0
                } else {
                    3
                },
            ),
            (
                CommandId::Bombardment,
                if flagship.lane == LaneType::Command {
                    2
                } else {
                    0
                },
            ),
            (CommandId::VolleyFire, 1),
        ];
        threats
            .into_iter()
            .filter(|&(id, _)| unplayed(self.view, id) > 0)
            .map(|(_, amount)| amount)
            .max()
            .unwrap_or(0)
            .max(0)
    }

    /// Ob ein Schild dieser Stärke das Flaggschiff rettet, dem sonst in dieser Runde der Fall droht.
    pub(super) fn rescues(&self, shield: i32) -> bool {
        let threat = self.flagship_threat();
        self.flagship.is_some_and(|flagship| {
            let durability = self.durability(flagship);
            threat >= durability && threat - shield < durability
        })
    }

    /// Ein Strike-Schiff, hinter dem ein Schiff steht, das ohne es ungedeckt wäre.
    pub(super) fn uncovers(&self, at: SlotRef) -> bool {
        at.lane == LaneType::Strike && self.board.is_occupied(at.in_lane(LaneType::Command))
    }

    /// Freie Slots, in die das Schiff wechseln kann, neben dem Lane-Wechsel `other` ([`Board::can_move`]): Sein Ziel
    /// ist belegt, und die Lane-Grenze gilt für beide Bewegungen zusammen, so wie der Server sie prüft.
    pub(super) fn destinations(&self, at: SlotRef, other: Option<PlannedMove>) -> Vec<SlotRef> {
        self.board.maneuver_destinations(at, &[], other)
    }

    /// Wohin das Schiff beim Wechsel in die andere Lane geht: am liebsten in einen Slot, der mit einem anderen Schiff
    /// der bisherigen Lane ein Paar bildet (vorne deckt es, hinten ist es gedeckt), sonst möglichst nah. `other` ist
    /// der zweite Lane-Wechsel dieser Runde: Sein Ziel ist belegt, sein Schiff bildet nur noch dort ein Paar. Gibt den
    /// Slot zurück und ob er ein Paar bildet; `None`, wenn die andere Lane keinen Platz hat.
    pub(super) fn lane_switch(
        &self,
        at: SlotRef,
        other: Option<PlannedMove>,
    ) -> Option<(SlotRef, bool)> {
        let free: Vec<SlotRef> = self
            .destinations(at, other)
            .into_iter()
            .filter(|destination| destination.lane != at.lane)
            .collect();
        let partner = |index: usize| {
            let neighbour = SlotRef::new(at.side, at.lane, index);
            other.is_some_and(|other| other.to == neighbour)
                || (self.board.is_occupied(neighbour)
                    && neighbour != at
                    && other.is_none_or(|other| other.from != neighbour))
        };
        let paired: Vec<SlotRef> = free
            .iter()
            .copied()
            .filter(|destination| partner(destination.slot_index))
            .collect();
        let is_paired = !paired.is_empty();
        let candidates = if is_paired { paired } else { free };
        nearest(candidates, at.slot_index).map(|destination| (destination, is_paired))
    }

    /// Vorrücken lohnt als Deckung nur, wenn das Schiff vor einem anderen Command-Schiff landet; Zurückziehen bringt
    /// nur Sicherheit, wenn es hinter einem anderen Strike-Schiff landet.
    pub(super) fn pairs_after_move(&self, at: SlotRef) -> bool {
        self.lane_switch(at, None).is_some_and(|(_, paired)| paired)
    }

    /// Ob ein eigener Command-Slot für `at` gedeckt ist: davor ein Strike-Schiff, das stehen bleibt, oder das Ziel von
    /// `other`.
    pub(super) fn covered(
        &self,
        destination: SlotRef,
        at: SlotRef,
        other: Option<PlannedMove>,
    ) -> bool {
        let front = self.own_slot(LaneType::Strike, destination.slot_index);
        destination.lane == LaneType::Command
            && (other.is_some_and(|other| other.to == front)
                || (self.board.is_occupied(front)
                    && other.is_none_or(|other| other.from != front)
                    && front != at))
    }

    /// Ein freier, gedeckter Command-Slot für ein eigenes Schiff ([`Situation::covered`]).
    pub(super) fn shelter(&self, at: SlotRef, other: Option<PlannedMove>) -> Option<SlotRef> {
        let covered = self
            .destinations(at, other)
            .into_iter()
            .filter(|&destination| self.covered(destination, at, other));
        nearest(covered, at.slot_index)
    }

    /// Schnellverlegung, auf der Kopie mit dem zusätzlichen Schiff in `target`: was es im Kampf dieser Runde gegenüber
    /// `without` bringt.
    pub(super) fn arrival_gain(&self, without: &Situation, target: SlotTarget) -> f64 {
        let ship = target.on(self.side);
        let worth = self.durability(ship) + KILL_BONUS + self.board.attack(ship) * POWER.future;
        RAPID_TEMPO + self.standing() - without.standing() - f64::from(worth)
    }

    // --- Probekampf ---

    /// Die Stellung nach dem Kampf dieser Runde, wenn niemand etwas tut.
    pub(super) fn standing(&self) -> f64 {
        *self
            .baseline
            .get_or_init(|| self.outcome(&Trial::default()))
    }

    /// Der Kampf dieser Runde auf einer Kopie mit `trial`: die Stellung danach aus eigener Sicht, samt dem Feuer der
    /// Folgerunden. Wie in [`Situation::flagship_threat`] zählt dazu der stärkste Befehl des Gegners: Bleibt dem
    /// eigenen Flaggschiff nach dem Kampf nicht mehr, fällt es (der Befehl wirkt vor dem Kampf); so rettet ein Schild
    /// nur, wenn er auch dafür reicht.
    pub(super) fn outcome(&self, trial: &Trial) -> f64 {
        let (mut board, _) = self.battle(trial, None);
        let railguns = firing_railguns(&board);
        resolve_combat(&mut board, &mut Vec::new(), &railguns);
        let doomed = board.flagship(self.side).is_some_and(|flagship| {
            board.ship(flagship).hp > 0 && durability(&board, flagship) <= self.command_danger
        });
        resolve_cleanup(&mut board, &mut Vec::new());
        let firepower = |side: Side| {
            f64::from(board.ships(side).map(|at| board.attack(at)).sum::<i32>() * POWER.future)
        };
        position_score(&board, self.side) - if doomed { DECISIVE } else { 0.0 }
            + firepower(self.side)
            - firepower(self.side.opposite())
    }

    /// Die Kopie zu Kampfbeginn, wie er sie erwartet: Die vermuteten Feuerbefehle des Gegners greifen wie beim
    /// Aufdecken vor allen Bewegungen; sie folgen ihren Schützen, Schiffsfeuer auch seinem Ziel, die Spalten bleiben.
    /// Dann der eigene Zug in der Reihenfolge der Auflösung (Fähigkeit, Manöver, Wächterschild, Befehl); zuletzt zielt
    /// er selbst ([`aim_fire`], mit `lead` auch vorhaltend), mit allem, was er über seinen Zug weiß. Feuerbefehle gibt
    /// es auf beiden Seiten nur für Schiffe, die beim Planen standen, Schiffsfeuer nur auf solche: Die Karte nennt den
    /// Slot jedes Schiffs beim Planen je Kennung ([`voidfleet_core::board::Ship::uid`]).
    pub(super) fn battle(
        &self,
        trial: &Trial,
        lead: Option<&Lead>,
    ) -> (Board, HashMap<u32, SlotTarget>) {
        let mut board = self.board.clone();
        let enemy = self.side.opposite();
        let planned: HashMap<u32, SlotTarget> = board
            .all_ships()
            .map(|at| (board.ship(at).uid, at.target()))
            .collect();
        let risk: HashMap<u32, Risk> = self
            .risks
            .iter()
            .map(|(&at, &risk)| (board.ship(at).uid, risk))
            .collect();
        for &order in self.enemy_fire {
            bind_fire(&mut board, enemy, order);
        }
        if let Some(target) = trial.power {
            let power = PowerUse {
                id: self.view.admiral.power(),
                target,
            };
            resolve_power(&mut board, self.side, power, &mut Vec::new());
        }
        if let Some(power) = trial.enemy_power {
            resolve_power(&mut board, enemy, power, &mut Vec::new());
        }
        if let Some(maneuver) = trial.maneuver {
            apply_move(&mut board, self.side, maneuver);
        }
        resolve_guardian_shields(&mut board, &mut Vec::new());
        if let Some(TrialCommand {
            card,
            target,
            destination,
        }) = trial.command
        {
            // Wärmesucher und Traktorstrahl wirken auf das erfasste Schiff.
            if card.targets_enemy_ship()
                && let Some(ship) = target.and_then(|at| board[at].as_mut())
            {
                ship.locks = vec![card];
            }
            let command = PlayedCommand {
                side: self.side,
                card,
                target,
                destination,
            };
            // Was er dabei ziehen würde, zählt der Probelauf nicht.
            let mut nothing = |_: Side, _: u32, _: u32| Drawn::default();
            resolve_effects(&mut board, &[command], &mut nothing, &mut Vec::new());
        }
        let uid = |board: &Board, at: SlotRef| board.ship(at).uid;
        let aims = aim_fire(
            &board,
            self.side,
            &|at| planned.contains_key(&uid(&board, at)),
            &|at| risk.get(&uid(&board, at)).copied().unwrap_or(SAFE),
            lead,
        );
        for (shooter, target) in aims {
            let aim = match target {
                Target::Column(column) => Aim::Column(column),
                Target::Ship(ship) => Aim::Ship(board.ship(ship).uid),
            };
            board.ship_mut(shooter).aim = Some(aim);
        }
        (board, planned)
    }

    /// Die Feuerbefehle zum fertigen Zug: seine Zielwahl im Probekampf mit dem ganzen Zug, in den Slots beim Planen.
    /// Nur hier hält er vor; die Probekämpfe, mit denen er Befehl, Manöver und Fähigkeit wählt, zielen auf die
    /// Stellung, wie sie ist.
    pub(super) fn plan_fire(&self, trial: &Trial) -> Vec<FireOrder> {
        let (board, planned) = self.battle(trial, self.lead);
        board
            .ships(self.side)
            .filter_map(|at| {
                let ship = board.ship(at);
                let target = match ship.aim? {
                    Aim::Column(column) => FireTarget::Column(column),
                    Aim::Ship(uid) => FireTarget::Ship(planned[&uid]),
                };
                Some(FireOrder {
                    ship: planned[&ship.uid],
                    target,
                })
            })
            .collect()
    }

    // --- Admiralsfähigkeit und Markierung ---

    /// Admiralsfähigkeit statt Grundmanöver: Jedes mögliche Ziel der Fähigkeit und das geplante Manöver werden im Kampf
    /// dieser Runde durchgespielt (gegen das vermutete Feuer des Gegners, sonst ohne seinen Zug); die Fähigkeit
    /// gewinnt, wenn sie mehr bringt als das Manöver samt Vorschuss. Mit Freier Hand (`both`) gibt es beide.
    pub(super) fn plan_power(
        &self,
        maneuver: Option<Maneuver>,
        command: Option<&Choice>,
        both: bool,
    ) -> (Option<Maneuver>, Option<PowerPlan>) {
        let Some((target, gain)) = self.best_power(command, maneuver.filter(|_| both)) else {
            return (maneuver, None);
        };
        if !both
            && let Some(maneuver) = maneuver
            && self.maneuver_gain(maneuver) >= gain
        {
            return (Some(maneuver), None);
        }
        (maneuver.filter(|_| both), Some(PowerPlan { target }))
    }

    /// Das beste Ziel der Fähigkeit und was sie bringt; `None`, wenn sie nichts bringt. Sie meidet die Ziel-Slots von
    /// Befehl und `maneuver`.
    pub(super) fn best_power(
        &self,
        command: Option<&Choice>,
        maneuver: Option<Maneuver>,
    ) -> Option<(SlotTarget, f64)> {
        let blocked: Vec<SlotTarget> = command
            .and_then(|command| command.destination)
            .map(SlotRef::target)
            .into_iter()
            .chain(maneuver.map(|maneuver| maneuver.destination))
            .collect();
        let power = self.view.admiral.power();
        let mut best: Option<(SlotTarget, f64)> = None;
        for target in power_targets(&self.board, self.side, power, &[], &blocked) {
            let trial = Trial {
                power: Some(target.target()),
                ..Trial::default()
            };
            let gain = self.outcome(&trial) - self.standing();
            if gain > best.map_or(0.0, |(_, best)| best) {
                best = Some((target.target(), gain));
            }
        }
        best
    }

    /// Was das Grundmanöver im Kampf dieser Runde bringt, samt Vorschuss.
    pub(super) fn maneuver_gain(&self, maneuver: Maneuver) -> f64 {
        let trial = Trial {
            maneuver: Some(maneuver),
            ..Trial::default()
        };
        self.outcome(&trial) - self.standing() + POWER.maneuver_head_start
    }

    /// Pendulum: markiert das Ziel, das ihre Railgun in der nächsten Runde am ehesten ungedeckt trifft; nicht, wenn sie
    /// sich bewegt.
    pub(super) fn plan_mark(&self, maneuver: Option<Maneuver>) -> Option<SlotTarget> {
        let flagship = self.flagship.expect("die Pendulum steht auf dem Brett");
        if maneuver.is_some_and(|maneuver| maneuver.ship == flagship.target()) {
            return None;
        }
        let exposure = |at: SlotRef| {
            if at.lane == LaneType::Strike {
                return 0.8;
            }
            let front = at.in_lane(LaneType::Strike);
            if !self.board.is_occupied(front) {
                0.6
            } else if self.durability(front) <= 5 {
                KERR.weak_cover
            } else {
                KERR.covered
            }
        };
        let targets = self
            .enemies
            .iter()
            .copied()
            .filter(|&at| !is_token(&self.board, at));
        highest(targets, |&at| {
            self.damage_value(at, ABILITY_VALUES.railgun) * exposure(at)
        })
        .map(SlotRef::target)
    }
}

/// Die Stellung aus Sicht von `side`: HP und Schild aller Schiffe (des Flaggschiffs doppelt), je Schiff außer Tokens
/// ein Bonus; der Fall eines Flaggschiffs entscheidet.
fn position_score(board: &Board, side: Side) -> f64 {
    let value = |of: Side| -> i32 {
        board
            .ships(of)
            .map(|at| {
                let weight = if is_flagship(board, at) {
                    FLAGSHIP_WEIGHT
                } else {
                    1
                };
                durability(board, at) * weight + if is_token(board, at) { 0 } else { KILL_BONUS }
            })
            .sum()
    };
    let fallen = |of: Side| {
        if board.flagship(of).is_some() {
            0.0
        } else {
            DECISIVE
        }
    };
    f64::from(value(side) - value(side.opposite())) - fallen(side) + fallen(side.opposite())
}

/// Das Grundmanöver auf einer Brettkopie: Wechsel in einen freien Slot oder Phasentausch.
fn apply_move(board: &mut Board, side: Side, maneuver: Maneuver) {
    let from = maneuver.ship.on(side);
    let to = maneuver.destination.on(side);
    if board.is_occupied(to) {
        resolve_swap(board, from, to, &mut Vec::new());
    } else {
        resolve_maneuver(board, from, to, &mut Vec::new());
    }
}
