//! Zielvorschau beim Planen (Port von `game/preview.ts`): die Runde, wie sie mit dem eigenen Plan abliefe, wenn der
//! Gegner nichts tut — kein Schiff, kein Befehl, kein Manöver, keine Fähigkeit, kein Feuerbefehl. Gerechnet wird mit
//! derselben Auflösung wie auf dem Server ([`resolve_turns`]) auf einer Kopie des Bretts, samt Flaggschiff,
//! Admiralsfähigkeit, Markierung der Pendulum und Feuerbefehlen; nur der Zufallsschuss der Plasma Frigate bleibt
//! außen vor. Die eigenen Schüsse und was sie anrichten, stimmen also genau; was der Gegner vorhat, zeigt sie nicht.

use bevy::platform::collections::{HashMap, HashSet};
use voidfleet_core::{
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::ShipId,
    powers::PlannedArrival,
    protocol::{CommandEffect, FireTarget, Miss, RoundEvent, ShipRef, ShipStats, TurnPlan},
    resolution::{Drawn, PowerUse, Turn, TurnCommand, resolve_turns},
};

use super::state::Match;

/// Wie ein Schuss endet: Er trifft ein Schiff (`through`: Spaltenfeuer, das durchging, weil das vordere Schiff der
/// Spalte vorher gefallen ist) oder verpufft, mit dem Grund.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShotOutcome {
    Hit { ship: ShipRef, through: bool },
    Miss(Miss),
}

/// Ein Schuss im Kampf, mit den Slots nach allen Bewegungen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreviewShot {
    pub attacker: SlotRef,
    /// Der Slot des Schützen beim Planen, an dem sein Feuerbefehl hängt; bei der Railgun `None`.
    pub shooter: Option<SlotRef>,
    /// Wohin der Bogen fällt: auf das getroffene Schiff; verpufft der Schuss, auf das verfolgte Schiff bzw. bei
    /// Spaltenfeuer auf das vorderste Schiff, das in der Spalte gefallen ist, sonst auf ihren Strike-Slot.
    pub target: SlotRef,
    /// Schiffsfeuer: Der Schuss verfolgt sein Ziel.
    pub tracking: bool,
    pub outcome: ShotOutcome,
}

/// Die Runde mit dem eigenen Plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoundPreview {
    /// Wer im Kampf worauf feuert: die eigenen Schüsse samt verpuffenden und die Railguns, wenn sie treffen.
    pub shots: Vec<PreviewShot>,
    /// Wo Schiffe fallen, mit dem Slot zu diesem Zeitpunkt; eingeplante Schiffe eingeschlossen.
    pub losses: Vec<SlotRef>,
    /// Wohin Titan Carrier ihre Drohnen beim Deploy schicken und die Drohnenstaffel ihre startet; die Drohne der Hive
    /// startet erst nach dem Kampf.
    pub drones: Vec<SlotRef>,
    /// Je Schiff, das schon auf dem Brett steht (nach seinem jetzigen Slot): HP und Schild nach der Runde, `None`, wenn
    /// es fällt. Ein Schiff im Kurzsprung ist am Rundenende wieder da.
    pub outcomes: HashMap<SlotRef, Option<ShipStats>>,
}

/// Die Runde mit `plan` (dem eigenen Zug oder einer Variante davon, siehe [`Match::try_plan`]).
pub fn preview_round(game: &Match, plan: &TurnPlan) -> RoundPreview {
    let mut board = game.board.clone();
    // Nachgezogen wird in der Vorschau nicht (Plündern, Gegenmaßnahmen).
    let mut commander = |_: Side, _: u32, _: u32| Drawn::default();
    let turns = [own_turn(game, plan), Turn::new(Side::Enemy)];
    let events = resolve_turns(&mut board, &turns, &mut commander, None);

    // Wohin Schiffe gewechselt sind, je Schiff höchstens einmal.
    let moves: Vec<(SlotRef, SlotRef)> = events
        .iter()
        .flat_map(|event| match *event {
            RoundEvent::Maneuver { side, ship, to } => vec![(ship.slot(), to.on(side))],
            RoundEvent::Swap { a, b, .. } => vec![(a.slot(), b.slot()), (b.slot(), a.slot())],
            RoundEvent::Effect {
                ship,
                effect: CommandEffect::Move { to },
                ..
            } => vec![(ship.slot(), to.on(ship.side))],
            _ => Vec::new(),
        })
        .collect();
    let after = |at: SlotRef| {
        moves
            .iter()
            .find(|(from, _)| *from == at)
            .map_or(at, |&(_, to)| to)
    };
    let before = |at: SlotRef| {
        moves
            .iter()
            .find(|(_, to)| *to == at)
            .map_or(at, |&(from, _)| from)
    };
    // Das Ziel des Schiffsfeuers eines eigenen Schützen (Slot beim Planen); `None` bei Spaltenfeuer oder ohne Befehl.
    let tracked = |shooter: SlotRef| -> Option<SlotTarget> {
        plan.fire
            .iter()
            .find(|order| shooter.side == Side::Player && order.ship == shooter.target())
            .and_then(|order| match order.target {
                FireTarget::Ship(target) => Some(target),
                FireTarget::Column(_) => None,
            })
    };

    let losses: Vec<SlotRef> = events
        .iter()
        .filter_map(|event| match event {
            RoundEvent::Destroyed { ship } => Some(ship.slot()),
            _ => None,
        })
        .collect();
    let fell: HashSet<SlotRef> = losses.iter().copied().collect();
    // Wo verpuffendes Spaltenfeuer verlischt: am vordersten Schiff, das in der Spalte gefallen ist, sonst im Strike-Slot.
    let column_end = |side: Side, column: usize| {
        let front = SlotRef::new(side, LaneType::Strike, column);
        let back = front.in_lane(LaneType::Command);
        if !fell.contains(&front) && fell.contains(&back) {
            back
        } else {
            front
        }
    };

    let mut preview = RoundPreview::default();
    for event in &events {
        match *event {
            RoundEvent::Attack {
                attacker,
                target,
                swarm,
                taunt,
                through,
                ..
            } => {
                let shooter = before(attacker.slot());
                preview.shots.push(PreviewShot {
                    attacker: attacker.slot(),
                    shooter: Some(shooter),
                    target: target.slot(),
                    tracking: tracked(shooter).is_some() && !swarm && !taunt,
                    outcome: ShotOutcome::Hit {
                        ship: target,
                        through,
                    },
                });
            }
            RoundEvent::Fizzle { attacker, miss, .. } => {
                let shooter = before(attacker.slot());
                let (target, tracking) = match miss {
                    Miss::Column { column, .. } => {
                        (column_end(attacker.side.opposite(), column), false)
                    }
                    // Schiffsfeuer auf ein Schiff, das schon vor dem Kampf gefallen ist, geht dorthin, wo es zuletzt
                    // stand. (Drohnen im Schwarmangriff verpuffen nur an ihrem Ziel, das dann noch steht.)
                    Miss::Target { target, .. } => (
                        target.map_or_else(
                            || {
                                let aim =
                                    tracked(shooter).expect("verpufftes Schiffsfeuer hat ein Ziel");
                                after(aim.on(Side::Enemy))
                            },
                            ShipRef::slot,
                        ),
                        tracked(shooter).is_some(),
                    ),
                };
                preview.shots.push(PreviewShot {
                    attacker: attacker.slot(),
                    shooter: Some(shooter),
                    target,
                    tracking,
                    outcome: ShotOutcome::Miss(miss),
                });
            }
            RoundEvent::Railgun {
                source,
                target,
                blocked: false,
                ..
            } => preview.shots.push(PreviewShot {
                attacker: source.slot(),
                shooter: None,
                target: target.slot(),
                tracking: false,
                outcome: ShotOutcome::Hit {
                    ship: target,
                    through: false,
                },
            }),
            RoundEvent::Spawn { source, ship } if source.card_id != ShipId::Hive => {
                preview.drones.push(ship.slot());
            }
            RoundEvent::Effect {
                ship,
                effect: CommandEffect::Spawn,
                ..
            } => preview.drones.push(ship.slot()),
            _ => {}
        }
    }

    // Wer fällt, steht mit seinem Slot nach allen Bewegungen in `losses`; dort kann die Hive am Rundenende schon die
    // nächste Drohne starten.
    preview.outcomes = game
        .board
        .all_ships()
        .map(|at| {
            let to = after(at);
            let stats = (!fell.contains(&to))
                .then(|| board[to].as_ref())
                .flatten()
                .map(|ship| ShipStats {
                    hp: ship.hp,
                    shield: ship.shield,
                });
            (at, stats)
        })
        .collect();
    preview.losses = losses;
    preview
}

/// Der eigene Zug aus dem Plan, mit den Karten statt der Hand-Indizes.
fn own_turn(game: &Match, plan: &TurnPlan) -> Turn {
    Turn {
        ships: plan
            .ships
            .iter()
            .map(|deploy| PlannedArrival {
                card: game.ship_hand[deploy.hand_index],
                target: deploy.target,
            })
            .collect(),
        flagship: plan
            .flagship
            .zip(game.flagship())
            .map(|(target, card)| PlannedArrival { card, target }),
        command: plan.command.map(|command| TurnCommand {
            card: game.command_hand[command.hand_index],
            target: command.target,
            destination: command.destination,
        }),
        maneuver: plan.maneuver,
        power: plan.power.zip(game.power()).map(|(power, id)| PowerUse {
            id,
            target: power.target,
        }),
        mark: plan.mark,
        fire: plan.fire.clone(),
        ..Turn::new(Side::Player)
    }
}

#[cfg(test)]
mod tests;
