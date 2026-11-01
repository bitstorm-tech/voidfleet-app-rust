//! Formuliert die aufgedeckten Ereignisse einer Runde als Log-Zeilen (Port von `ui/roundLog.ts`); bei Schüssen auch,
//! ob Spaltenfeuer durchging, ob die Ablenkung einen gezogen hat und warum einer verpufft. Die Zeilen hängt, wer die
//! Runde abspielt, mit [`super::AppendLog`] ans Log.

use voidfleet_core::{
    admirals::PowerId,
    board::SlotRef,
    cards::{CommandId, ShipId},
    protocol::{AbilityEffect, CommandEffect, Miss, RoundEvent, ShipRef},
};

use crate::i18n::Messages;

/// Ein Slot im Log: „dein Strike-Slot 3“.
pub fn slot_label(locale: &dyn Messages, slot: SlotRef) -> String {
    locale.log_slot(slot.side, locale.lane(slot.lane), slot.slot_index + 1)
}

fn ship_label(locale: &dyn Messages, ship: ShipRef) -> String {
    locale.log_ship(
        locale.ship(ship.card_id).name,
        &slot_label(locale, ship.slot()),
    )
}

/// Was Schnellverlegung bzw. Freie Hand schon beim Planen erlaubt haben; für Banner und Log.
pub fn rule_subtitle(locale: &dyn Messages, card: CommandId) -> &'static str {
    if card == CommandId::RapidDeployment {
        locale.banner_extra_ship()
    } else {
        locale.banner_maneuver_and_power()
    }
}

/// Eine Zeile je Ereignis.
pub fn describe_event(locale: &dyn Messages, event: &RoundEvent) -> String {
    let ship = |at: ShipRef| ship_label(locale, at);
    let slot = |at: SlotRef| slot_label(locale, at);
    let who = |side| locale.log_who(side);
    let command = |card: CommandId| locale.command(card).name;
    match *event {
        RoundEvent::Deploy {
            side,
            card_id,
            target,
        } => {
            let name = locale.ship(card_id).name;
            let at = slot(target.on(side));
            if card_id.is_flagship() {
                locale.log_deploy_flagship(who(side), name, &at)
            } else {
                locale.log_deploy(who(side), name, &at)
            }
        }
        RoundEvent::Pass { side } => locale.log_pass(who(side)),
        RoundEvent::Power {
            side,
            power,
            target,
        } => locale.log_power(who(side), locale.power(power).name, &slot(target)),
        RoundEvent::Decoy { ship: decoy } => locale.log_decoy(&ship(decoy)),
        RoundEvent::Mark { side, ship: marked } => locale.log_mark(who(side), &ship(marked)),
        RoundEvent::Maneuver {
            side,
            ship: moved,
            to,
        } => locale.log_maneuver(who(side), &ship(moved), &slot(to.on(side))),
        RoundEvent::Swap { side, a, b } => locale.log_swap(who(side), &ship(a), &ship(b)),
        RoundEvent::Command {
            side,
            card_id,
            target,
        } => match target {
            Some(target) => locale.log_command_on(who(side), command(card_id), &slot(target)),
            None => locale.log_command(who(side), command(card_id)),
        },
        RoundEvent::Effect {
            side,
            card_id,
            ship: affected,
            effect,
            ..
        } => {
            let name = command(card_id);
            let target = ship(affected);
            match effect {
                CommandEffect::Damage { amount } => locale.log_damage(name, &target, amount),
                CommandEffect::Attack { delta } => locale.log_attack_bonus(name, &target, delta),
                CommandEffect::Shield {
                    amount,
                    permanent: true,
                } => locale.log_shield_permanent(name, &target, amount),
                CommandEffect::Shield { amount, .. } => locale.log_shield(name, &target, amount),
                CommandEffect::Disable => locale.log_disable(name, &target),
                CommandEffect::Heal { amount } => locale.log_heal(name, &target, amount),
                CommandEffect::Move { to } => {
                    let destination = slot(to.on(affected.side));
                    if affected.side == side {
                        locale.log_move(name, &target, &destination)
                    } else {
                        locale.log_tow(name, &target, &destination)
                    }
                }
                CommandEffect::Jump => locale.log_jump(name, &target),
                CommandEffect::Taunt => locale.log_taunt(name, &target),
                CommandEffect::Spawn => locale.log_launch(name, &target),
                CommandEffect::Ram {
                    target: rammed,
                    dealt,
                    taken,
                    ..
                } => locale.log_ram(name, &target, &ship(rammed), dealt, taken),
                CommandEffect::Jam {
                    power: PowerId::Decoy,
                } => locale.log_jam_decoy(name, &target),
                CommandEffect::Jam { power } => {
                    locale.log_jam(name, &target, locale.power(power).name)
                }
            }
        }
        RoundEvent::Ability {
            source,
            ship: affected,
            effect,
            ..
        } => {
            let source = ship(source);
            match effect {
                AbilityEffect::Damage { amount } => {
                    locale.log_ability_damage(&source, &ship(affected), amount)
                }
                AbilityEffect::Heal { amount } => {
                    locale.log_repair(&source, &ship(affected), amount)
                }
                AbilityEffect::Salvage => locale.log_salvage(&source),
                AbilityEffect::Shield { amount } => {
                    locale.log_guardian_shield(&source, &ship(affected), amount)
                }
            }
        }
        RoundEvent::Spawn {
            source,
            ship: drone,
        } => {
            if source.card_id == ShipId::Hive {
                locale.log_brood(&ship(source), &ship(drone))
            } else {
                locale.log_spawn(&ship(source), &ship(drone))
            }
        }
        RoundEvent::Draw {
            side,
            card_id,
            ships,
            commands,
        } => locale.log_draw(command(card_id), who(side), ships, commands),
        RoundEvent::Countered {
            side,
            card_id,
            countered,
        } => locale.log_countered(command(card_id), who(side), command(countered)),
        RoundEvent::Rule { side, card_id } => {
            locale.log_rule(command(card_id), who(side), rule_subtitle(locale, card_id))
        }
        RoundEvent::NoEffect { side, card_id } => locale.log_no_effect(command(card_id), who(side)),
        RoundEvent::Miss {
            card_id, target, ..
        } => locale.log_miss(command(card_id), &slot(target)),
        RoundEvent::Railgun {
            source,
            target,
            amount,
            blocked,
            ..
        } => {
            if blocked {
                locale.log_railgun_blocked(&ship(source), &ship(target))
            } else {
                locale.log_railgun(&ship(source), &ship(target), amount)
            }
        }
        RoundEvent::Attack {
            attacker,
            target,
            amount,
            focus,
            swarm,
            head_start,
            through,
            taunt,
            ..
        } => {
            // Was den Schuss verstärkt, gelenkt oder beschleunigt hat.
            let notes: Vec<&str> = [
                (focus, locale.power(PowerId::FocusFire).name),
                (swarm, locale.power(PowerId::SwarmStrike).name),
                (head_start, locale.power(PowerId::HeadStart).name),
                (taunt, command(CommandId::DrawFire)),
            ]
            .into_iter()
            .filter_map(|(applies, note)| applies.then_some(note))
            .collect();
            if through {
                locale.log_through(&ship(attacker), &ship(target), amount, &notes)
            } else {
                locale.log_attack(&ship(attacker), &ship(target), amount, &notes)
            }
        }
        RoundEvent::Fizzle { attacker, miss, .. } => {
            let shooter = ship(attacker);
            match miss {
                Miss::Column { column, cleared } => {
                    let column = locale.log_column(attacker.side.opposite(), column + 1);
                    if cleared {
                        locale.log_fizzle_cleared(&shooter, &column)
                    } else {
                        locale.log_fizzle_empty(&shooter, &column)
                    }
                }
                Miss::Target { target: None, .. } => locale.log_fizzle_gone(&shooter),
                Miss::Target {
                    target: Some(target),
                    covered: true,
                } => locale.log_fizzle_covered(&shooter, &ship(target)),
                Miss::Target {
                    target: Some(target),
                    ..
                } => locale.log_fizzle(&shooter, &ship(target)),
            }
        }
        RoundEvent::Silenced { ship: silenced } => locale.log_silenced(&ship(silenced)),
        RoundEvent::Counterfire {
            source,
            ship: hit,
            amount,
            ..
        } => locale.log_counterfire(&ship(hit), &ship(source), amount),
        RoundEvent::SelfDamage {
            ship: hit, amount, ..
        } => locale.log_self_damage(&ship(hit), amount),
        RoundEvent::Destroyed { ship: lost } => {
            if lost.card_id.is_flagship() {
                locale.log_flagship_destroyed(&ship(lost))
            } else {
                locale.log_destroyed(&ship(lost))
            }
        }
        RoundEvent::Vanish { ship: gone } => locale.log_vanish(&ship(gone)),
        RoundEvent::Return { ship: back, .. } => locale.log_rejoin(&ship(back)),
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        board::{LaneType, Side},
        protocol::ShipStats,
    };

    use super::*;
    use crate::{i18n::Locale, settings::Language};

    fn at(side: Side, slot: usize, card: ShipId) -> ShipRef {
        ShipRef::new(SlotRef::new(side, LaneType::Strike, slot), card)
    }

    #[test]
    fn attack_names_notes_and_fizzle_reason() {
        let en = Locale::new(Language::En);
        let attack = RoundEvent::Attack {
            attacker: at(Side::Player, 0, ShipId::VoidInterceptor),
            target: at(Side::Enemy, 1, ShipId::Hive),
            amount: 3,
            after: ShipStats::default(),
            focus: true,
            swarm: false,
            head_start: false,
            through: false,
            taunt: false,
        };
        let line = describe_event(&*en, &attack);
        assert!(line.starts_with("Combat: "), "{line}");
        assert!(line.contains("(3, Focus Fire)"), "{line}");
        let fizzle = RoundEvent::Fizzle {
            attacker: at(Side::Player, 0, ShipId::VoidInterceptor),
            head_start: false,
            miss: Miss::Target {
                target: None,
                covered: false,
            },
        };
        assert_eq!(
            describe_event(&*en, &fizzle),
            en.log_fizzle_gone(&ship_label(
                &*en,
                at(Side::Player, 0, ShipId::VoidInterceptor)
            ))
        );
    }

    #[test]
    fn every_language_describes_deploys() {
        for language in crate::i18n::LANGUAGES {
            let locale = Locale::new(language);
            let deploy = RoundEvent::Deploy {
                side: Side::Enemy,
                card_id: ShipId::Bastion,
                target: SlotRef::new(Side::Enemy, LaneType::Command, 2).target(),
            };
            let line = describe_event(&*locale, &deploy);
            assert!(line.contains(locale.ship(ShipId::Bastion).name), "{line}");
        }
    }
}
