//! Gruppierung der Ereignisse einer Runde in Schritte (`stepKey`/`groupSteps` aus `roundPlayback.ts`): Ereignisse mit
//! gleichem Schlüssel stehen hintereinander und laufen gemeinsam ab. Das Trugbild erscheint im Schritt seiner
//! Fähigkeit, der Phasentausch fliegt mit den Manövern, die Schüsse mit Taktvorsprung bilden eine eigene Salve.

use std::mem::{Discriminant, discriminant};

use voidfleet_core::{
    board::{Side, SlotRef},
    cards::CommandId,
    protocol::RoundEvent,
};

/// Woran man erkennt, dass zwei aufeinanderfolgende Ereignisse in denselben Schritt gehören.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepKey {
    /// Deploys und Passen beider Seiten.
    Deploy,
    /// Admiralsfähigkeit samt Trugbild.
    Power(Side),
    Mark(Side),
    /// Grundmanöver und Phasentausch beider Seiten.
    Maneuver,
    /// Was ein Befehl bewirkt hat, je Seite und Karte.
    Command(Side, CommandId),
    /// Fähigkeit eines Schiffs, je Quelle.
    Ability(SlotRef),
    Spawn(Side),
    /// Schüsse im Kampf, mit Taktvorsprung als eigene Salve.
    Attack {
        head_start: bool,
    },
    /// Alles Übrige nach seiner Art.
    Other(Discriminant<RoundEvent>),
}

fn step_key(event: &RoundEvent) -> StepKey {
    match *event {
        RoundEvent::Deploy { .. } | RoundEvent::Pass { .. } => StepKey::Deploy,
        RoundEvent::Power { side, .. } => StepKey::Power(side),
        RoundEvent::Decoy { ship } => StepKey::Power(ship.side),
        RoundEvent::Mark { side, .. } => StepKey::Mark(side),
        RoundEvent::Maneuver { .. } | RoundEvent::Swap { .. } => StepKey::Maneuver,
        RoundEvent::Effect { side, card_id, .. }
        | RoundEvent::Draw { side, card_id, .. }
        | RoundEvent::NoEffect { side, card_id }
        | RoundEvent::Miss { side, card_id, .. }
        | RoundEvent::Countered { side, card_id, .. }
        | RoundEvent::Rule { side, card_id } => StepKey::Command(side, card_id),
        RoundEvent::Ability { source, .. } => StepKey::Ability(source.slot()),
        RoundEvent::Spawn { source, .. } => StepKey::Spawn(source.side),
        RoundEvent::Attack { head_start, .. } | RoundEvent::Fizzle { head_start, .. } => {
            StepKey::Attack { head_start }
        }
        _ => StepKey::Other(discriminant(event)),
    }
}

/// Teilt die Ereignisse in Schritte: aufeinanderfolgende mit gleichem Schlüssel landen im selben.
pub fn group_steps(events: &[RoundEvent]) -> Vec<Vec<RoundEvent>> {
    let mut steps: Vec<Vec<RoundEvent>> = Vec::new();
    for event in events {
        match steps.last_mut() {
            Some(step) if step_key(&step[0]) == step_key(event) => step.push(*event),
            _ => steps.push(vec![*event]),
        }
    }
    steps
}

/// Ob ein Schritt zum Kampf gehört; er läuft dann mit dem Regler für Angriffe.
pub fn is_combat(step: &[RoundEvent]) -> bool {
    matches!(
        step[0],
        RoundEvent::Attack { .. }
            | RoundEvent::Fizzle { .. }
            | RoundEvent::Railgun { .. }
            | RoundEvent::Silenced { .. }
            | RoundEvent::Counterfire { .. }
    )
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        admirals::PowerId,
        board::{LaneType, SlotTarget},
        cards::ShipId,
        protocol::{CommandEffect, Miss, ShipRef, ShipStats},
    };

    use super::*;

    fn ship(side: Side, lane: LaneType, index: usize, card: ShipId) -> ShipRef {
        ShipRef::new(SlotRef::new(side, lane, index), card)
    }

    fn attack(attacker: ShipRef, target: ShipRef, head_start: bool) -> RoundEvent {
        RoundEvent::Attack {
            attacker,
            target,
            amount: 2,
            after: ShipStats { hp: 3, shield: 0 },
            focus: false,
            swarm: false,
            head_start,
            through: false,
            taunt: false,
        }
    }

    fn shield(side: Side, card_id: CommandId, ship: ShipRef) -> RoundEvent {
        RoundEvent::Effect {
            side,
            card_id,
            ship,
            effect: CommandEffect::Shield {
                amount: 1,
                permanent: false,
            },
            after: ShipStats { hp: 5, shield: 1 },
        }
    }

    fn sizes(events: &[RoundEvent]) -> Vec<usize> {
        group_steps(events).iter().map(Vec::len).collect()
    }

    const P: Side = Side::Player;
    const E: Side = Side::Enemy;

    /// Deploys und Passen beider Seiten laufen gemeinsam; das Trugbild gehört zur Fähigkeit seiner Seite, die
    /// Fähigkeit der anderen Seite ist ein eigener Schritt.
    #[test]
    fn deploys_and_powers() {
        let decoy = ship(P, LaneType::Strike, 0, ShipId::Decoy);
        let events = [
            RoundEvent::Deploy {
                side: P,
                card_id: ShipId::VoidInterceptor,
                target: SlotTarget::new(LaneType::Strike, 2),
            },
            RoundEvent::Pass { side: E },
            RoundEvent::Power {
                side: P,
                power: PowerId::Decoy,
                target: decoy.slot(),
            },
            RoundEvent::Decoy { ship: decoy },
            RoundEvent::Power {
                side: E,
                power: PowerId::FocusFire,
                target: SlotRef::new(E, LaneType::Strike, 3),
            },
        ];
        assert_eq!(sizes(&events), [2, 2, 1]);
    }

    /// Grundmanöver und Phasentausch fliegen zusammen.
    #[test]
    fn maneuvers_and_swaps_together() {
        let mirage = ship(E, LaneType::Strike, 1, ShipId::Mirage);
        let partner = ship(E, LaneType::Command, 1, ShipId::RepairDrone);
        let events = [
            RoundEvent::Maneuver {
                side: P,
                ship: ship(P, LaneType::Strike, 2, ShipId::HorizonCruiser),
                to: SlotTarget::new(LaneType::Command, 2),
            },
            RoundEvent::Swap {
                side: E,
                a: mirage,
                b: partner,
            },
        ];
        assert_eq!(sizes(&events), [2]);
    }

    /// Die Wirkungen eines Befehls bilden einen Schritt, samt Fehlschuss; derselbe Befehl der Gegenseite und ein
    /// anderer Befehl derselben Seite sind eigene Schritte.
    #[test]
    fn command_outcomes_by_side_and_card() {
        let a = ship(P, LaneType::Strike, 2, ShipId::VoidInterceptor);
        let b = ship(P, LaneType::Strike, 3, ShipId::HorizonCruiser);
        let enemy = ship(E, LaneType::Strike, 3, ShipId::IronDreadnought);
        let events = [
            RoundEvent::Command {
                side: P,
                card_id: CommandId::ShieldGrid,
                target: None,
            },
            shield(P, CommandId::ShieldGrid, a),
            shield(P, CommandId::ShieldGrid, b),
            shield(E, CommandId::ShieldGrid, enemy),
            shield(E, CommandId::EmergencyShields, enemy),
            RoundEvent::Miss {
                side: E,
                card_id: CommandId::EmergencyShields,
                target: a.slot(),
            },
        ];
        assert_eq!(sizes(&events), [1, 2, 1, 2]);
    }

    /// Die Fähigkeiten verschiedener Schiffe sind eigene Schritte, Drohnenstarts je Seite einer.
    #[test]
    fn abilities_by_source() {
        let frigate = ship(P, LaneType::Command, 1, ShipId::PlasmaFrigate);
        let other = ship(P, LaneType::Command, 2, ShipId::PlasmaFrigate);
        let target = ship(E, LaneType::Strike, 1, ShipId::ScrapRaider);
        let fire = |source| RoundEvent::Ability {
            source,
            ship: target,
            effect: voidfleet_core::protocol::AbilityEffect::Damage { amount: 1 },
            after: ShipStats { hp: 2, shield: 0 },
        };
        let carrier = ship(E, LaneType::Command, 3, ShipId::TitanCarrier);
        let spawn = |index| RoundEvent::Spawn {
            source: carrier,
            ship: ship(E, LaneType::Command, index, ShipId::Drone),
        };
        let events = [
            fire(frigate),
            fire(frigate),
            fire(other),
            spawn(2),
            spawn(4),
        ];
        assert_eq!(sizes(&events), [2, 1, 2]);
    }

    /// Die Salve mit Taktvorsprung ist ein eigener Schritt vor der übrigen; verpuffende Schüsse gehören in ihre Salve.
    #[test]
    fn head_start_salvo_apart() {
        let a = ship(P, LaneType::Strike, 2, ShipId::VoidInterceptor);
        let b = ship(E, LaneType::Strike, 2, ShipId::ScrapRaider);
        let events = [
            attack(a, b, true),
            RoundEvent::Fizzle {
                attacker: a,
                head_start: true,
                miss: Miss::Column {
                    column: 4,
                    cleared: false,
                },
            },
            RoundEvent::Silenced { ship: b },
            attack(b, a, false),
            RoundEvent::Fizzle {
                attacker: b,
                head_start: false,
                miss: Miss::Target {
                    target: None,
                    covered: false,
                },
            },
            RoundEvent::Destroyed { ship: b },
            RoundEvent::Destroyed { ship: a },
        ];
        assert_eq!(sizes(&events), [2, 1, 2, 2]);
        let steps = group_steps(&events);
        assert!(is_combat(&steps[0]) && is_combat(&steps[1]) && !is_combat(&steps[3]));
    }

    /// Gleiche Ereignisse, die nicht direkt aufeinander folgen, bleiben getrennt.
    #[test]
    fn only_neighbours_merge() {
        let a = ship(P, LaneType::Strike, 2, ShipId::VoidInterceptor);
        let events = [
            RoundEvent::Destroyed { ship: a },
            RoundEvent::Vanish {
                ship: ship(E, LaneType::Strike, 0, ShipId::Decoy),
            },
            RoundEvent::Destroyed { ship: a },
        ];
        assert_eq!(sizes(&events), [1, 1, 1]);
        assert!(group_steps(&[]).is_empty());
    }
}
