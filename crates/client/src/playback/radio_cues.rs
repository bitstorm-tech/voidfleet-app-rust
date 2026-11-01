//! Welche Funksprüche des Bordfunks ([`crate::audio::radio`]) zu den Schritten einer Rundenwiedergabe gehören (Port
//! von `radioCues.ts`): beim Aufdecken eines Befehls oder einer Fähigkeit die Ankündigung, nach dem Schritt die
//! Reaktion auf seinen Ausgang. Gesprochen wird aus Sicht der eigenen Flotte: eigene Befehle bestätigt der Funk,
//! gegnerische Angriffe meldet er. Drohnen sind keinen Funkspruch wert, wenn sie fallen.

use bevy::platform::collections::HashMap;
use voidfleet_core::{
    board::Side,
    cards::CommandId,
    protocol::{AbilityEffect, RoundEvent},
};

use crate::audio::RadioCue;

/// Ab diesem Anteil seiner HP gilt ein eigenes Schiff nach dem Kampf als schwer getroffen.
const CRITICAL_HULL: f32 = 0.3;

/// Ankündigung eines Befehls: eigener bzw. gegnerischer.
fn announcement_of(command: CommandId) -> (Option<RadioCue>, Option<RadioCue>) {
    use RadioCue::*;
    match command {
        CommandId::Torpedo => (Some(TorpedoAway), Some(TorpedoIncoming)),
        CommandId::OrbitalStrike | CommandId::Bombardment => {
            (Some(OrbitalStrike), Some(OrbitalIncoming))
        }
        CommandId::VolleyFire => (Some(Volley), Some(VolleyIncoming)),
        CommandId::TargetingJammer => (Some(Jammer), Some(Jammed)),
        CommandId::EmergencyShields | CommandId::ShieldGrid | CommandId::AblativeArmor => {
            (Some(ShieldsUp), None)
        }
        CommandId::Overcharge => (Some(Overcharge), None),
        CommandId::PowerSurge => (Some(PowerSurge), None),
        CommandId::Scavenge => (Some(Scavenge), None),
        CommandId::Intercept => (Some(Intercept), None),
        CommandId::Recall => (Some(Recall), None),
        CommandId::FlankingManeuver => (Some(Flanking), None),
        CommandId::DroneSquadron => (Some(DroneLaunch), None),
        CommandId::Crossfire => (Some(Volley), None),
        _ => (None, None),
    }
}

/// Eigene Fähigkeiten: Die Plasma Frigate feuert, die Repair Drone repariert, der Scrap Raider plündert, die Bastion
/// hebt den Wächterschild.
fn ability(effect: AbilityEffect) -> RadioCue {
    match effect {
        AbilityEffect::Damage { .. } => RadioCue::FrigateFire,
        AbilityEffect::Heal { .. } => RadioCue::Repair,
        AbilityEffect::Salvage => RadioCue::Scrap,
        AbilityEffect::Shield { .. } => RadioCue::ShieldsUp,
    }
}

/// Der Spruch, mit dem ein Schritt beginnt; ein Befehl ohne Wirkung wird nicht angekündigt.
pub fn announcement(step: &[RoundEvent]) -> Option<RadioCue> {
    match step[0] {
        RoundEvent::Effect { side, card_id, .. }
        | RoundEvent::Draw { side, card_id, .. }
        | RoundEvent::Miss { side, card_id, .. } => {
            let (own, enemy) = announcement_of(card_id);
            match side {
                Side::Player => own,
                Side::Enemy => enemy,
            }
        }
        // Titan Carrier beim Deploy, Hive am Rundenende.
        RoundEvent::Spawn { source, .. } => {
            (source.side == Side::Player).then_some(RadioCue::DroneLaunch)
        }
        RoundEvent::Ability { source, effect, .. } => {
            (source.side == Side::Player).then(|| ability(effect))
        }
        _ => None,
    }
}

/// Die Sprüche nach einem Schritt: wie Torpedo, Sperrfeuer und EMP ausgingen, Verluste und schwer getroffene eigene
/// Schiffe.
pub fn reactions(step: &[RoundEvent]) -> Vec<RadioCue> {
    use RadioCue::*;
    match step[0] {
        RoundEvent::Effect { side, card_id, .. }
        | RoundEvent::NoEffect { side, card_id }
        | RoundEvent::Miss { side, card_id, .. } => {
            let own = side == Side::Player;
            let hit = matches!(step[0], RoundEvent::Effect { .. });
            let missed = matches!(step[0], RoundEvent::Miss { .. });
            let cue = match card_id {
                CommandId::Torpedo if missed => {
                    Some(if own { TorpedoMissed } else { TorpedoDodged })
                }
                CommandId::Torpedo => own.then_some(TorpedoHit),
                CommandId::Barrage if own => Some(if hit { BarrageHit } else { BarrageMissed }),
                CommandId::Barrage => hit.then_some(BarrageCaught),
                CommandId::EmpBlast if hit => Some(if own { EmpHit } else { EmpIncoming }),
                _ => None,
            };
            cue.into_iter().collect()
        }
        RoundEvent::Destroyed { .. } => {
            let lost = |side: Side| {
                step.iter().any(|event| {
                    matches!(event, RoundEvent::Destroyed { ship }
                        if ship.side == side && !ship.card_id.is_token())
                })
            };
            [(Side::Player, ShipLost), (Side::Enemy, EnemyDestroyed)]
                .into_iter()
                .filter(|(side, _)| lost(*side))
                .map(|(_, cue)| cue)
                .collect()
        }
        // Eine Salve kann mit einem Schuss beginnen, der verpufft.
        RoundEvent::Attack { .. } | RoundEvent::Fizzle { .. } => {
            // Je getroffenem eigenem Schiff zählt der letzte Treffer: Es übersteht den Kampf knapp.
            let mut hulls = HashMap::new();
            for event in step {
                if let RoundEvent::Attack { target, after, .. } = event
                    && target.side == Side::Player
                {
                    hulls.insert(target.slot(), after.hp as f32 / target.card_id.hp() as f32);
                }
            }
            let critical = hulls
                .values()
                .any(|hull| *hull > 0.0 && *hull <= CRITICAL_HULL);
            critical.then_some(HullCritical).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        board::{LaneType, SlotRef},
        cards::ShipId,
        protocol::{ShipRef, ShipStats},
    };

    use super::*;

    fn ship(side: Side, card: ShipId) -> ShipRef {
        ShipRef::new(SlotRef::new(side, LaneType::Strike, 2), card)
    }

    #[test]
    fn torpedo_outcomes() {
        let miss = |side| RoundEvent::Miss {
            side,
            card_id: CommandId::Torpedo,
            target: SlotRef::new(side.opposite(), LaneType::Strike, 1),
        };
        assert_eq!(
            announcement(&[miss(Side::Player)]),
            Some(RadioCue::TorpedoAway)
        );
        assert_eq!(reactions(&[miss(Side::Player)]), [RadioCue::TorpedoMissed]);
        assert_eq!(reactions(&[miss(Side::Enemy)]), [RadioCue::TorpedoDodged]);
    }

    /// Verluste ohne Drohnen; ein eigenes Schiff knapp über null meldet der Funk als schwer getroffen.
    #[test]
    fn losses_and_critical_hull() {
        let destroyed = |side, card| RoundEvent::Destroyed {
            ship: ship(side, card),
        };
        assert!(reactions(&[destroyed(Side::Player, ShipId::Drone)]).is_empty());
        assert_eq!(
            reactions(&[
                destroyed(Side::Enemy, ShipId::ScrapRaider),
                destroyed(Side::Player, ShipId::VoidInterceptor),
            ]),
            [RadioCue::ShipLost, RadioCue::EnemyDestroyed]
        );
        let cruiser = ship(Side::Player, ShipId::HorizonCruiser);
        let hit = RoundEvent::Attack {
            attacker: ship(Side::Enemy, ShipId::IronDreadnought),
            target: cruiser,
            amount: 4,
            after: ShipStats { hp: 1, shield: 0 },
            focus: false,
            swarm: false,
            head_start: false,
            through: false,
            taunt: false,
        };
        assert_eq!(reactions(&[hit]), [RadioCue::HullCritical]);
    }
}
