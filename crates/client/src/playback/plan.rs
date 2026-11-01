//! Was ein Schritt auf dem Brett zeigt (Port von `playStep`, `playEvents` und den einzelnen Ereignissen aus
//! `roundPlayback.ts`): [`Planner::step`] übersetzt die Ereignisse eines Schritts in ein [`Script`] aus Takten.
//!
//! Angriffsbefehle unterscheiden sich im Bild: Orbitalschlag, Torpedo und Sperrfeuer kommen von oben, der Wärmesucher
//! als Rakete, das Minenfeld explodiert am Schiff, der Durchschlag geht als Strahl durch beide Schiffe einer
//! Slot-Nummer, das Bombardement schlägt mehrfach ein, das Kreuzfeuer kommt von beiden Seiten, der Gnadenstoß als heller
//! Präzisionsschuss. Die Schüsse im Kampf laufen als Salve kurz versetzt und überlappend ab, Einschläge auf dasselbe
//! Ziel mit etwas Abstand. Ein Schuss, der verpufft, verlischt ohne Einschlag; verpufft Schiffsfeuer an der Deckung,
//! zeigt eine Zielmarke, worauf er gezielt war.
//!
//! Positionen rechnet der Planer beim Start des Schritts aus der Aufteilung des Bretts; welche Schiffe wo stehen, sagt
//! ihm die Belegung zu diesem Zeitpunkt.

use std::mem::{Discriminant, discriminant};

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::{
    admirals::{AdmiralId, PowerId},
    board::{LaneType, Side, SlotRef},
    cards::{CommandId, ShipId},
    protocol::{AbilityEffect, CommandEffect, Miss, RoundEvent, ShipRef, ShipStats},
};

use super::{
    radio_cues::{announcement, reactions},
    script::{Cue, Effect, Script, ShipAction},
    steps::is_combat,
};
use crate::{
    audio::{PlaySound, SoundId},
    data::admirals::{flagship_admiral, power_admiral},
    i18n::Messages,
    palette::{self, admiral_color},
    scene::{
        board::anchor_position,
        camera::BoardLayout,
        effects::{
            Beam, BlastStyle, Bolt, Bubble, Detonation, Fizzle, HitVariant, Impact, Missile, Pulse,
            TargetRing, bolt_duration,
        },
        floating_text::FloatTone,
        fx::mix,
        markers::{MarkerKind, MarkerSpec},
        motion::HIT_FLASH_ALPHA,
        tween::AnimationScope,
    },
    ui::{
        ShowBanner,
        round_log::{describe_event, rule_subtitle, slot_label},
    },
};

/// Pause zwischen zwei Schritten, damit man sie auseinanderhalten kann.
const STEP_PAUSE: f32 = 0.2;
/// Abstand zwischen den Starts zweier Schüsse einer Salve.
const SALVO_STAGGER: f32 = 0.25;
/// Mindestabstand zweier Einschläge auf dasselbe Ziel, damit die Schadenszahlen nicht übereinanderliegen.
const SAME_TARGET_GAP: f32 = 0.5;
/// So lange steht das Banner, bevor die Wirkung einsetzt.
const BANNER_LEAD: f32 = 0.65;
/// Mitte eines Schiffs über seinem Anker.
pub(super) const SHIP_CENTER: Vec3 = Vec3::new(0.0, 0.15, 0.0);
/// Schwebetexte stehen so weit über der Mitte.
const ABOVE_SHIP: Vec3 = Vec3::new(0.0, 0.9, 0.0);
/// Orbitale Angriffe kommen von oben.
const ORBIT_OFFSET: Vec3 = Vec3::new(0.0, 9.0, 0.0);
/// Schüsse enden nicht im Mittelpunkt, sondern so weit davor auf der Hülle.
const HULL_RADIUS: f32 = 0.45;
/// Ein verpuffender Schuss verlischt so weit vor der Hülle seines schon zerstörten Ziels.
const FIZZLE_SHORT: f32 = 0.9;
/// Steht sein Ziel nicht mehr auf dem Brett, verlischt er über der Brettmitte, so weit über der Höhe des Schützen.
const VOID_RISE: f32 = 0.6;
/// Durchmesser der Schildblase des Wächterschilds.
const GUARDIAN_BUBBLE: f32 = 2.1;
/// Der Zielstrahl der Zielerfassung ist dünner als der Schuss der Railgun.
const LOCK_BEAM_WIDTH: f32 = 0.35;
/// Der Wärmesucher startet vor der eigenen Strike-Lane, knapp über dem Brett.
const MISSILE_LAUNCH: Vec3 = Vec3::new(0.0, 0.8, 0.0);
/// Minenfeld: erst glimmt die Mine unter dem Schiff auf, dann explodiert sie.
const MINE_FUSE: f32 = 0.26;
/// Bombardement: so viele Einschläge je Schiff, im Abstand und um so viel versetzt.
const BOMBARD_SHELLS: usize = 3;
const BOMBARD_STAGGER: f32 = 0.14;
const BOMBARD_SPREAD: f32 = 0.45;
/// Kreuzfeuer: Die beiden Schüsse kommen so weit seitlich entlang der Lane und so hoch.
const CROSSFIRE_REACH: f32 = 4.0;
const CROSSFIRE_HEIGHT: f32 = 1.4;
/// Gnadenstoß: ein dünner, fast weißer Strahl.
const COUP_BEAM_WIDTH: f32 = 0.55;
/// Durchschlag: Der Strahl tritt so weit hinter dem hinteren Slot aus.
const PIERCE_OVERSHOOT: f32 = 2.5;
/// Rammkurs: Der Rammer hält so weit vor der Mitte des Ziels, dort kracht es.
const RAM_STANDOFF: f32 = 1.1;
const RAM_BLAST: f32 = 0.6;
const RAM_SHAKE: f32 = 0.3;
/// Kurzsprung: Lichtpunkt am Ort des Absprungs und der Rückkehr.
const JUMP_PULSE: f32 = 1.5;
/// Traktorstrahl: Er greift von hoch über der eigenen Seite und hält kurz, bevor er zieht.
const TRACTOR_ORIGIN: Vec3 = Vec3::new(0.0, 3.5, 0.0);
const TRACTOR_GRIP: f32 = 0.35;

fn side_color(side: Side) -> Color {
    match side {
        Side::Player => palette::PLAYER,
        Side::Enemy => palette::ENEMY,
    }
}

/// Waffe je Schiff; das Trugbild feuert nie.
fn shot_sound(ship: ShipId) -> SoundId {
    match ship {
        ShipId::HorizonCruiser | ShipId::TitanCarrier | ShipId::Vanguard | ShipId::Hive => {
            SoundId::LaserMedium
        }
        ShipId::PlasmaFrigate => SoundId::PlasmaShot,
        ShipId::IronDreadnought | ShipId::Bastion | ShipId::Pendulum => SoundId::CannonHeavy,
        ShipId::VoidInterceptor
        | ShipId::ScrapRaider
        | ShipId::RepairDrone
        | ShipId::SignalBooster
        | ShipId::Drone
        | ShipId::Decoy
        | ShipId::Mirage => SoundId::LaserLight,
    }
}

/// Was man von einem Befehl hört, sobald er wirkt; Lane-Wechsel hört man an den Triebwerken, Treffer an den
/// Einschlägen.
fn command_sound(command: CommandId) -> Option<SoundId> {
    use CommandId::*;
    Some(match command {
        EmergencyShields | ShieldGrid | AblativeArmor => SoundId::ShieldUp,
        EmpBlast | Countermeasures => SoundId::Emp,
        Intercept | Overcharge | PowerSurge | MicroJump | CoveringFire | TractorBeam => {
            SoundId::PowerUp
        }
        OrbitalStrike | Torpedo | CoupDeGrace | Bombardment => SoundId::OrbitalStrike,
        Recall => SoundId::Repair,
        Scavenge | ScrapSalvage => SoundId::Scavenge,
        TargetingJammer | DrawFire | RadioSilence => SoundId::Jammer,
        VolleyFire | Barrage | HeatSeeker => SoundId::MissileVolley,
        PiercingShot => SoundId::CannonHeavy,
        Crossfire => SoundId::LaserMedium,
        FlankingManeuver | EvasiveAction | RapidDeployment | DroneSquadron | CollisionCourse
        | CarteBlanche | Minefield => return None,
    })
}

/// Ein Pulse-Effekt.
fn pulse(position: Vec3, color: Color, size: f32, duration: f32) -> Effect {
    Effect::Pulse(Pulse {
        position,
        color,
        size,
        duration,
    })
}

/// Schlüssel einer Kette von Wirkungen eines Befehls: am selben Schiff nacheinander, sonst je Art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChainKey {
    Ship(SlotRef),
    Kind(Discriminant<RoundEvent>),
}

/// Übersetzt Schritte in Skripte.
pub(super) struct Planner<'a> {
    pub layout: &'a BoardLayout,
    pub locale: &'a dyn Messages,
    /// Welches Schiff zu Beginn des Schritts auf welchem Slot steht.
    pub ships: &'a HashMap<SlotRef, ShipId>,
}

impl Planner<'_> {
    /// Ein Schritt samt Bordfunk: die Ankündigung zusammen mit dem Banner, die Reaktion, sobald er abgespielt ist,
    /// danach eine kurze Pause.
    pub fn step(&self, step: &[RoundEvent]) -> Script {
        let scope = if is_combat(step) {
            AnimationScope::Attack
        } else {
            AnimationScope::General
        };
        let reactions: Vec<Cue> = reactions(step).into_iter().map(Cue::Say).collect();
        let mut script = Script::default()
            .then(
                [Cue::Scope(scope)]
                    .into_iter()
                    .chain(announcement(step).map(Cue::Say)),
            )
            .and(self.events(step));
        if !reactions.is_empty() {
            script = script.then(reactions);
        }
        script.then([Cue::Pause(STEP_PAUSE)])
    }

    fn events(&self, step: &[RoundEvent]) -> Script {
        let first = step[0];
        match first {
            RoundEvent::Attack { .. }
            | RoundEvent::Fizzle { .. }
            | RoundEvent::Counterfire { .. } => {
                let lead = match first {
                    RoundEvent::Attack {
                        attacker,
                        head_start: true,
                        ..
                    }
                    | RoundEvent::Fizzle {
                        attacker,
                        head_start: true,
                        ..
                    } => Some(self.banner(
                        attacker.side,
                        self.locale.power(PowerId::HeadStart).name,
                        None,
                        Some(AdmiralId::Kerr),
                    )),
                    RoundEvent::Counterfire { source, .. } => Some(self.banner(
                        source.side,
                        self.locale.power(PowerId::Counterfire).name,
                        None,
                        Some(AdmiralId::Osei),
                    )),
                    _ => None,
                };
                let salvo = Script::default().then([Cue::Fork(self.salvo(step))]);
                match lead {
                    Some(banner) => Script::default()
                        .then([banner, Cue::Pause(BANNER_LEAD)])
                        .and(salvo),
                    None => salvo,
                }
            }
            _ => {
                let log = Cue::Log(step.iter().map(|event| self.describe(event)).collect());
                if let Some((side, card)) = command_outcome(&first) {
                    let mut effects = self.command_sound(&first);
                    if card == CommandId::PiercingShot {
                        effects.extend(self.pierce_beam(side, step));
                    }
                    effects.push(Cue::Fork(self.in_sequence(step)));
                    return Script::default()
                        .then([log, self.command_banner(&first), Cue::Pause(BANNER_LEAD)])
                        .then(effects);
                }
                let all = Cue::Fork(step.iter().map(|event| self.event(event)).collect());
                match self.header(&first) {
                    Some(banner) => Script::default()
                        .then([log, banner, Cue::Pause(BANNER_LEAD)])
                        .then([all]),
                    None => Script::default().then([log, all]),
                }
            }
        }
    }

    fn describe(&self, event: &RoundEvent) -> String {
        describe_event(self.locale, event)
    }

    /// Banner mit dem Namen eines Befehls, einer Fähigkeit oder eines Schiffs; mit `admiral` sein Porträt.
    fn banner(
        &self,
        side: Side,
        name: &str,
        subtitle: Option<String>,
        admiral: Option<AdmiralId>,
    ) -> Cue {
        Cue::Banner(ShowBanner {
            title: self.locale.banner_command(self.locale.log_who(side), name),
            subtitle,
            side,
            admiral,
        })
    }

    /// Das Banner eines Schritts ohne Befehl: Fähigkeiten von Schiffen (Flaggschiffe mit ihrem Admiral), die
    /// Admiralsfähigkeit und die Zielerfassung.
    fn header(&self, first: &RoundEvent) -> Option<Cue> {
        let locale = self.locale;
        match *first {
            RoundEvent::Ability { source, .. }
            | RoundEvent::Spawn { source, .. }
            | RoundEvent::Railgun { source, .. } => {
                let admiral = flagship_admiral(source.card_id);
                let subtitle = match first {
                    RoundEvent::Railgun { blocked: true, .. } => {
                        Some(locale.banner_blocked().to_owned())
                    }
                    _ => admiral.map(|admiral| locale.admiral(admiral).ability.to_owned()),
                };
                Some(self.banner(
                    source.side,
                    locale.ship(source.card_id).name,
                    subtitle,
                    admiral,
                ))
            }
            RoundEvent::Power { side, power, .. } => {
                let admiral = power_admiral(power);
                Some(self.banner(
                    side,
                    locale.power(power).name,
                    Some(locale.admiral_title(admiral)),
                    Some(admiral),
                ))
            }
            RoundEvent::Mark { side, ship } => Some(self.banner(
                side,
                locale.hud_mark_button(),
                Some(locale.banner_mark_target(locale.ship(ship.card_id).name)),
                Some(AdmiralId::Kerr),
            )),
            _ => None,
        }
    }

    fn command_banner(&self, event: &RoundEvent) -> Cue {
        let locale = self.locale;
        let (side, card) = command_outcome(event).expect("Befehl");
        let subtitle = match *event {
            RoundEvent::NoEffect { .. } => Some(locale.banner_no_effect().to_owned()),
            RoundEvent::Draw {
                ships, commands, ..
            } => Some(locale.banner_draw(ships, commands)),
            RoundEvent::Miss { target, .. } => {
                Some(locale.banner_miss(&slot_label(locale, target)))
            }
            RoundEvent::Countered { countered, .. } => {
                Some(locale.banner_countered(locale.command(countered).name))
            }
            RoundEvent::Rule { card_id, .. } => Some(rule_subtitle(locale, card_id).to_owned()),
            _ => None,
        };
        self.banner(side, locale.command(card).name, subtitle, None)
    }

    /// Einmal je Befehl, auch wenn er mehrere Schiffe trifft; ohne Wirkung verpufft er hörbar.
    fn command_sound(&self, event: &RoundEvent) -> Vec<Cue> {
        let (side, card) = command_outcome(event).expect("Befehl");
        if matches!(event, RoundEvent::NoEffect { .. }) {
            return vec![Cue::Denied];
        }
        let at = match *event {
            RoundEvent::Effect { ship, .. } => self.center(ship.slot()),
            RoundEvent::Miss { target, .. } => self.center(target),
            _ => self.layout.side_center(side),
        };
        command_sound(card)
            .map(|sound| Cue::sound(sound, at))
            .into_iter()
            .collect()
    }

    /// Die Wirkungen eines Befehls: an verschiedenen Schiffen gleichzeitig, am selben Schiff nacheinander, damit
    /// Impulse und Schwebetexte nicht übereinanderliegen (Feuerschutz: ATK, dann Schild). Ein verlegtes Schiff nimmt
    /// seine Kette in den neuen Slot mit (Ausweichmanöver: erst der Flug, dann der Schild).
    fn in_sequence(&self, step: &[RoundEvent]) -> Vec<Script> {
        let mut chains: Vec<(Vec<ChainKey>, Script)> = Vec::new();
        for event in step {
            let key = match event {
                RoundEvent::Effect { ship, .. } => ChainKey::Ship(ship.slot()),
                _ => ChainKey::Kind(discriminant(event)),
            };
            let index = match chains.iter().position(|(keys, _)| keys.contains(&key)) {
                Some(index) => index,
                None => {
                    chains.push((vec![key], Script::default()));
                    chains.len() - 1
                }
            };
            let (keys, chain) = &mut chains[index];
            *chain = std::mem::take(chain).and(self.event(event));
            if let RoundEvent::Effect {
                ship,
                effect: CommandEffect::Move { to },
                ..
            } = event
            {
                keys.push(ChainKey::Ship(to.on(ship.side)));
            }
        }
        chains.into_iter().map(|(_, chain)| chain).collect()
    }

    /// Die Schüsse starten im Abstand [`SALVO_STAGGER`] und kommen dabei je Schuss ins Log; ein Schuss wartet
    /// zusätzlich, bis sein Einschlag [`SAME_TARGET_GAP`] nach dem vorigen auf dasselbe Ziel liegt. So treffen die
    /// Schüsse jedes Ziel in der gemeldeten Reihenfolge, und die Statusbalken springen nicht zurück. Ein Schuss, der
    /// verpufft, zählt dort, wo er verlischt ([`Self::landing`]), wie ein Einschlag.
    fn salvo(&self, shots: &[RoundEvent]) -> Vec<Script> {
        let mut impacts: HashMap<SlotRef, f32> = HashMap::default();
        let mut start = -SALVO_STAGGER;
        shots
            .iter()
            .map(|shot| {
                let origin = self.center(shooter(shot).slot());
                let landing = self.landing(shot);
                let flight = bolt_duration(origin, self.shot_end(shot, origin));
                let after_previous = landing
                    .and_then(|slot| impacts.get(&slot))
                    .map_or(f32::NEG_INFINITY, |impact| {
                        impact + SAME_TARGET_GAP - flight
                    });
                start = (start + SALVO_STAGGER).max(after_previous);
                if let Some(slot) = landing {
                    impacts.insert(slot, start + flight);
                }
                let script = self.event(shot).with([Cue::Log(vec![self.describe(shot)])]);
                if start > 0.0 {
                    Script::default().then([Cue::Pause(start)]).and(script)
                } else {
                    script
                }
            })
            .collect()
    }

    /// Das Skript eines einzelnen Ereignisses.
    fn event(&self, event: &RoundEvent) -> Script {
        let one = |cues: Vec<Cue>| Script::default().then(cues);
        match *event {
            RoundEvent::Deploy {
                side,
                card_id,
                target,
            } => one(vec![Cue::Arrive {
                slot: target.on(side),
                ship: card_id,
            }]),
            RoundEvent::Power { power, target, .. } => match MarkerKind::of_power(power) {
                // Das Trugbild erscheint für sich (`Decoy`).
                None => Script::default(),
                Some(kind) => one(vec![
                    Cue::sound(SoundId::PowerUp, self.center(target)),
                    Cue::ship(
                        target,
                        ShipAction::Mark(MarkerSpec::new(
                            kind,
                            admiral_color(power_admiral(power)),
                        )),
                    ),
                ]),
            },
            RoundEvent::Decoy { ship } => self.decoy(ship.slot()),
            RoundEvent::Mark { side, ship } => self.lock(side, ship.slot()),
            RoundEvent::Maneuver { side, ship, to } => {
                let to = to.on(side);
                Script::default()
                    .then([Cue::ship(
                        ship.slot(),
                        ShipAction::Move { to, towed: false },
                    )])
                    .then(self.hold_fire(to))
            }
            RoundEvent::Swap { a, b, .. } => self.swap(a.slot(), b.slot()),
            RoundEvent::Effect {
                side,
                card_id,
                ship,
                effect,
                after,
            } => self.effect(side, card_id, ship, effect, after),
            RoundEvent::Miss {
                side,
                card_id,
                target,
            } => self.miss(side, card_id, target),
            RoundEvent::Ability {
                source,
                ship,
                effect,
                after,
            } => self.ability(source, ship.slot(), effect, after),
            RoundEvent::Spawn { source, ship } => Script::default()
                .then([Cue::Launch {
                    source: source.slot(),
                    slot: ship.slot(),
                }])
                .then([Cue::ship(
                    ship.slot(),
                    ShipAction::Move {
                        to: ship.slot(),
                        towed: false,
                    },
                )]),
            RoundEvent::Railgun {
                source,
                target,
                amount,
                after,
                blocked,
            } => one(self.railgun(source.slot(), target.slot(), amount, after, blocked)),
            RoundEvent::Attack {
                attacker,
                target,
                amount,
                after,
                ..
            } => {
                let color = tracer(event);
                let origin = self.center(attacker.slot());
                Script::default()
                    .then(self.shoot(origin, target.slot(), color, shot_sound(attacker.card_id)))
                    .then(self.damage(target.slot(), amount, after, origin, color, None))
            }
            RoundEvent::Fizzle { .. } => self.fizzle(event),
            RoundEvent::Silenced { ship } => {
                let mut cues = vec![Cue::ship(ship.slot(), ShipAction::HoldFire), Cue::Denied];
                cues.extend(self.mark(
                    ship.slot(),
                    self.locale.float_silenced().to_owned(),
                    FloatTone::Debuff,
                    1.2,
                ));
                one(cues)
            }
            RoundEvent::Counterfire {
                source,
                ship,
                amount,
                after,
            } => {
                let color = admiral_color(AdmiralId::Osei);
                let origin = self.center(source.slot());
                Script::default()
                    .then(self.shoot(origin, ship.slot(), color, SoundId::LaserLight))
                    .then(self.damage(ship.slot(), amount, after, origin, color, None))
            }
            RoundEvent::SelfDamage {
                ship,
                amount,
                after,
            } => one(self.damage(
                ship.slot(),
                amount,
                after,
                self.center(ship.slot()) + ORBIT_OFFSET,
                FloatTone::Damage.color(),
                None,
            )),
            RoundEvent::Destroyed { ship } => {
                one(vec![Cue::ship(ship.slot(), ShipAction::Destroy)])
            }
            RoundEvent::Vanish { ship } => one(self.vanish(ship.slot())),
            RoundEvent::Return { ship, after } => {
                let center = self.center(ship.slot());
                one(vec![
                    Cue::Rejoin {
                        slot: ship.slot(),
                        stats: after,
                    },
                    Cue::sound(SoundId::PowerUp, center),
                    Cue::detached(pulse(center, FloatTone::Shield.color(), JUMP_PULSE, 0.45)),
                ])
            }
            RoundEvent::Pass { .. }
            | RoundEvent::Command { .. }
            | RoundEvent::Draw { .. }
            | RoundEvent::NoEffect { .. }
            | RoundEvent::Countered { .. }
            | RoundEvent::Rule { .. } => Script::default(),
        }
    }

    // --- Einzelne Ereignisse ---

    /// Trugbild: Das Hologramm baut sich im gewählten Strike-Slot auf.
    fn decoy(&self, slot: SlotRef) -> Script {
        let center = self.center(slot);
        Script::default().then([
            Cue::Materialize { slot },
            Cue::sound(SoundId::ShieldUp, center),
            Cue::detached(pulse(center, admiral_color(AdmiralId::Moreau), 1.6, 0.7)),
            self.float(slot, self.locale.float_decoy(), FloatTone::Buff),
        ])
    }

    /// Zielerfassung: ein dünner Zielstrahl von der Pendulum, dann zieht sich das Fadenkreuz um das Ziel zusammen.
    fn lock(&self, side: Side, slot: SlotRef) -> Script {
        let color = admiral_color(AdmiralId::Kerr);
        let center = self.center(slot);
        let pendulum = self
            .ships
            .iter()
            .find(|(at, ship)| at.side == side && **ship == ShipId::Pendulum)
            .map(|(at, _)| {
                Cue::detached(Effect::Beam(Beam {
                    from: self.center(*at),
                    to: center,
                    color,
                    width: LOCK_BEAM_WIDTH,
                }))
            });
        Script::default().then(
            [Cue::sound(SoundId::Jammer, center)]
                .into_iter()
                .chain(pendulum)
                .chain([
                    self.float(slot, self.locale.float_locked(), FloatTone::Debuff),
                    Cue::ship(
                        slot,
                        ShipAction::Mark(MarkerSpec::new(MarkerKind::Lock, color)),
                    ),
                ]),
        )
    }

    /// Nach einem Flug: Die Balken sind gedämpft, bis die Runde vorbei ist.
    fn hold_fire(&self, slot: SlotRef) -> Vec<Cue> {
        let mut cues = vec![Cue::ship(slot, ShipAction::HoldFire)];
        cues.extend(self.mark(
            slot,
            self.locale.float_hold_fire().to_owned(),
            FloatTone::Debuff,
            1.6,
        ));
        cues
    }

    /// Phasentausch: Beide Schiffe schrumpfen zu Lichtpunkten und erscheinen im Slot des anderen; beide feuern weiter.
    fn swap(&self, a: SlotRef, b: SlotRef) -> Script {
        let color = admiral_color(AdmiralId::Moreau);
        let pulses =
            || [a, b].map(|slot| Cue::detached(pulse(self.center(slot), color, 1.5, 0.45)));
        Script::default()
            .then(
                [Cue::sound(SoundId::PowerUp, self.center(a))]
                    .into_iter()
                    .chain(pulses())
                    .chain([
                        Cue::ship(a, ShipAction::PhaseOut),
                        Cue::ship(b, ShipAction::PhaseOut),
                    ]),
            )
            .then([Cue::Swap { a, b }].into_iter().chain(pulses()))
    }

    fn effect(
        &self,
        side: Side,
        card: CommandId,
        ship: ShipRef,
        effect: CommandEffect,
        after: ShipStats,
    ) -> Script {
        let locale = self.locale;
        let slot = ship.slot();
        let center = self.center(slot);
        let one = |cues: Vec<Cue>| Script::default().then(cues);
        match effect {
            CommandEffect::Damage { amount } => self.strike(side, card, slot, amount, after),
            CommandEffect::Attack { delta } => one(self.mark(
                slot,
                locale.float_attack(delta),
                if delta > 0 {
                    FloatTone::Buff
                } else {
                    FloatTone::Debuff
                },
                1.6,
            )),
            CommandEffect::Shield { amount, permanent } => {
                let text = if permanent {
                    locale.float_shield_permanent(amount)
                } else {
                    locale.float_shield(amount)
                };
                one([Cue::ship(slot, ShipAction::Stats(after))]
                    .into_iter()
                    .chain(self.mark(slot, text, FloatTone::Shield, 2.2))
                    .collect())
            }
            CommandEffect::Disable => one(self.mark(
                slot,
                locale.float_disabled().to_owned(),
                FloatTone::Debuff,
                1.6,
            )),
            CommandEffect::Heal { amount } => one([Cue::ship(slot, ShipAction::Stats(after))]
                .into_iter()
                .chain(self.mark(slot, locale.float_heal(amount), FloatTone::Heal, 1.6))
                .collect()),
            CommandEffect::Move { to } => {
                let to = to.on(ship.side);
                if ship.side == side {
                    one(vec![Cue::ship(slot, ShipAction::Move { to, towed: false })])
                } else {
                    self.tow(side, slot, to)
                }
            }
            CommandEffect::Jump => Script::default()
                .then([
                    Cue::detached(pulse(center, FloatTone::Shield.color(), JUMP_PULSE, 0.45)),
                    self.float(slot, locale.float_jump(), FloatTone::Shield),
                    Cue::ship(slot, ShipAction::PhaseOut),
                ])
                .then([Cue::ship(slot, ShipAction::Jump)]),
            CommandEffect::Taunt => one(vec![
                self.float(slot, locale.float_taunt(), FloatTone::Debuff),
                Cue::detached(pulse(center, side_color(side), 1.8, 0.65)),
                Cue::ship(
                    slot,
                    ShipAction::Mark(MarkerSpec::new(MarkerKind::Taunt, side_color(side))),
                ),
            ]),
            CommandEffect::Spawn => one(vec![Cue::Arrive {
                slot,
                ship: ship.card_id,
            }]),
            CommandEffect::Ram {
                target,
                dealt,
                taken,
                target_after,
            } => self.ram(slot, target.slot(), dealt, taken, after, target_after),
            CommandEffect::Jam { power } => {
                let jammed = self.float(slot, locale.float_jammed(), FloatTone::Debuff);
                let rest = match MarkerKind::of_power(power) {
                    Some(kind) => vec![
                        Cue::ship(slot, ShipAction::Unmark(kind)),
                        Cue::effect(pulse(center, FloatTone::Debuff.color(), 1.6, 0.65)),
                    ],
                    // Das Trugbild löst sich sofort auf wie am Rundenende.
                    None => self.vanish(slot),
                };
                one([jammed].into_iter().chain(rest).collect())
            }
        }
    }

    /// Schaden durch einen Befehl, je Karte anders ins Bild gesetzt (siehe Modulkopf). Der Durchschlag zieht seinen
    /// Strahl einmal für beide Schiffe ([`Self::pierce_beam`]); hier trifft er nur noch ein.
    fn strike(
        &self,
        side: Side,
        card: CommandId,
        slot: SlotRef,
        amount: i32,
        after: ShipStats,
    ) -> Script {
        let center = self.center(slot);
        let color = side_color(side);
        let hit = |origin: Vec3, variant: Option<HitVariant>| {
            self.damage(slot, amount, after, origin, color, variant)
        };
        match card {
            CommandId::HeatSeeker => {
                let origin = self.own_strike_front(side, center);
                let above = origin + ORBIT_OFFSET;
                Script::default()
                    .then([Cue::effect(Effect::Missile(Missile {
                        from: origin,
                        to: self.impact_from(slot, above, HULL_RADIUS),
                        color,
                    }))])
                    .then(hit(above, Some(HitVariant::Fireball)))
            }
            CommandId::Minefield => {
                let floor = self
                    .layout
                    .slot_position(slot.side, slot.lane, slot.slot_index);
                Script::default()
                    .then([Cue::effect(pulse(
                        floor,
                        FloatTone::Damage.color(),
                        0.9,
                        MINE_FUSE,
                    ))])
                    .then(
                        [Cue::Shake(0.08)]
                            .into_iter()
                            .chain(hit(center + Vec3::Y, Some(HitVariant::Fireball))),
                    )
            }
            CommandId::PiercingShot => Script::default().then(hit(
                self.pierce_origin(side, center),
                Some(HitVariant::Pierce),
            )),
            CommandId::Bombardment => {
                let origin = center + ORBIT_OFFSET;
                let shells = (0..BOMBARD_SHELLS)
                    .map(|index| {
                        let offset = Vec3::new(
                            rand::random::<f32>() - 0.5,
                            0.0,
                            rand::random::<f32>() - 0.5,
                        ) * 2.0
                            * BOMBARD_SPREAD;
                        let impact = self.impact_from(slot, origin, HULL_RADIUS) + offset;
                        let mut shell = Script::default();
                        if index > 0 {
                            shell = shell.then([Cue::Pause(index as f32 * BOMBARD_STAGGER)]);
                        }
                        shell = shell.then([Cue::effect(Effect::Bolt(Bolt {
                            from: origin + offset,
                            to: impact,
                            color,
                        }))]);
                        if index < BOMBARD_SHELLS - 1 {
                            shell = shell.then([
                                Cue::ship(slot, ShipAction::Flash(0.4)),
                                Cue::effect(Effect::Impact(Impact {
                                    position: impact,
                                    origin,
                                    color,
                                    variant: Some(HitVariant::Fire),
                                })),
                            ]);
                        }
                        shell
                    })
                    .collect();
                Script::default()
                    .then([Cue::Fork(shells)])
                    .then(hit(origin, Some(HitVariant::Fireball)))
            }
            CommandId::Crossfire => {
                let [right, left] = [1.0, -1.0].map(|direction| {
                    center + Vec3::new(0.0, CROSSFIRE_HEIGHT, direction * CROSSFIRE_REACH)
                });
                Script::default()
                    .then(
                        self.shoot(right, slot, color, SoundId::LaserMedium)
                            .into_iter()
                            .chain(self.shoot(left, slot, color, SoundId::LaserMedium)),
                    )
                    .then(
                        [Cue::detached(Effect::Impact(Impact {
                            position: self.impact_from(slot, left, HULL_RADIUS),
                            origin: left,
                            color,
                            variant: Some(HitVariant::Sparks),
                        }))]
                        .into_iter()
                        .chain(hit(right, Some(HitVariant::Sparks))),
                    )
            }
            CommandId::CoupDeGrace => {
                let origin = center + ORBIT_OFFSET;
                let bright = mix(color, Color::WHITE, 0.75);
                let impact = self.impact_from(slot, origin, HULL_RADIUS);
                Script::default().then(
                    [
                        Cue::detached(pulse(impact, bright, 1.4, 0.3)),
                        Cue::detached(Effect::Beam(Beam {
                            from: origin,
                            to: impact,
                            color: bright,
                            width: COUP_BEAM_WIDTH,
                        })),
                        Cue::Shake(0.1),
                    ]
                    .into_iter()
                    .chain(hit(origin, Some(HitVariant::Discharge))),
                )
            }
            _ => {
                let origin = center + ORBIT_OFFSET;
                Script::default()
                    .then([Cue::effect(Effect::Bolt(Bolt {
                        from: origin,
                        to: self.impact_from(slot, origin, HULL_RADIUS),
                        color,
                    }))])
                    .then(hit(origin, None))
            }
        }
    }

    /// Durchschlag: ein gerader Strahl über die ganze Slot-Nummer, durch beide Schiffe hindurch, einmal je Befehl.
    fn pierce_beam(&self, side: Side, step: &[RoundEvent]) -> Vec<Cue> {
        let Some(slot) = step.iter().find_map(|event| match *event {
            RoundEvent::Effect { ship, .. } => Some(ship.slot()),
            RoundEvent::Miss { target, .. } => Some(target),
            _ => None,
        }) else {
            return Vec::new();
        };
        let back = self.center(slot.in_lane(LaneType::Command));
        let origin = self.pierce_origin(side, back);
        let exit = back + Vec3::X * (back.x - origin.x).signum() * PIERCE_OVERSHOOT;
        let color = side_color(side);
        vec![
            Cue::Sound(PlaySound::new(SoundId::CannonHeavy).at(origin).rate(0.9)),
            Cue::detached(pulse(origin, color, 1.0, 0.26)),
            Cue::detached(Effect::Beam(Beam {
                from: origin,
                to: exit,
                color,
                width: 0.8,
            })),
            Cue::Shake(0.1),
        ]
    }

    /// Woher der Durchschlag kommt: aus der Mitte der Seite, die ihn spielt, auf Höhe von `target`.
    fn pierce_origin(&self, side: Side, target: Vec3) -> Vec3 {
        Vec3::new(self.layout.side_center(side).x, target.y, target.z)
    }

    /// Startpunkt des Wärmesuchers: vor der eigenen Strike-Lane, auf Höhe des Ziels.
    fn own_strike_front(&self, side: Side, target: Vec3) -> Vec3 {
        let strike = self.center(SlotRef::new(side, LaneType::Strike, 0));
        Vec3::new(strike.x, 0.0, target.z) + MISSILE_LAUNCH
    }

    /// Der Torpedo kommt von oben wie ein Orbitalschlag und schlägt im leeren Slot ein. Der Durchschlag hat seinen
    /// Strahl schon gezogen; am leeren Strike-Slot bleibt nur das „Daneben“.
    fn miss(&self, side: Side, card: CommandId, slot: SlotRef) -> Script {
        let missed = self.float(slot, self.locale.float_miss(), FloatTone::Debuff);
        if card == CommandId::PiercingShot {
            return Script::default().then([missed]);
        }
        let color = side_color(side);
        let point = self.center(slot);
        let origin = point + ORBIT_OFFSET;
        Script::default()
            .then([
                missed,
                Cue::effect(Effect::Bolt(Bolt {
                    from: origin,
                    to: point,
                    color,
                })),
            ])
            .then([Cue::effect(Effect::Impact(Impact {
                position: point,
                origin,
                color,
                variant: None,
            }))])
    }

    /// Rammkurs: Der Rammer fliegt auf sein Ziel zu, beim Aufprall kracht es mit Explosion und Kamerawackeln, beide
    /// zeigen ihren Schaden. Übersteht er es, kehrt er in seinen Slot zurück und feuert diese Runde nicht; sonst bleibt
    /// er bis zu seiner Zerstörung am Ziel liegen.
    fn ram(
        &self,
        slot: SlotRef,
        target: SlotRef,
        dealt: i32,
        taken: i32,
        after: ShipStats,
        target_after: ShipStats,
    ) -> Script {
        let locale = self.locale;
        let from = anchor_position(self.layout, slot);
        let goal = anchor_position(self.layout, target);
        let stop = goal - (goal - from).normalize() * RAM_STANDOFF;
        let impact = (stop + goal) / 2.0 + SHIP_CENTER;
        let fire = FloatTone::Damage.color();
        let script = Script::default()
            .then([Cue::ship(slot, ShipAction::Charge(stop))])
            .then([
                Cue::Shake(RAM_SHAKE),
                Cue::detached(Effect::Detonation(Detonation {
                    position: impact,
                    energy: fire,
                    size: RAM_BLAST,
                    style: BlastStyle::Fire,
                })),
                Cue::ship(slot, ShipAction::Stats(after)),
                Cue::ship(target, ShipAction::Stats(target_after)),
                self.float(target, locale.float_damage(dealt), FloatTone::Damage),
                self.float_at(
                    stop + SHIP_CENTER,
                    locale.float_damage(taken),
                    FloatTone::Damage,
                ),
                Cue::ship(slot, ShipAction::Flash(HIT_FLASH_ALPHA)),
                Cue::ship(target, ShipAction::Flash(HIT_FLASH_ALPHA)),
                Cue::effect(Effect::Impact(Impact {
                    position: impact,
                    origin: self.center(slot),
                    color: fire,
                    variant: Some(HitVariant::Fireball),
                })),
            ]);
        if after.hp <= 0 {
            return script;
        }
        script
            .then([Cue::ship(slot, ShipAction::Withdraw)])
            .then(self.hold_fire(slot))
    }

    /// Traktorstrahl: Ein Strahl greift von hoch über der Seite, die ihn spielt, nach dem gegnerischen Schiff, hält es
    /// kurz und zieht es dann ruhig in den neuen Slot.
    fn tow(&self, side: Side, from: SlotRef, to: SlotRef) -> Script {
        let color = side_color(side);
        let center = self.center(from);
        let origin = Vec3::new(self.layout.side_center(side).x, 0.0, center.z) + TRACTOR_ORIGIN;
        Script::default()
            .then([
                Cue::ship(
                    from,
                    ShipAction::Tether {
                        from: origin,
                        color,
                    },
                ),
                Cue::detached(pulse(center, color, 1.3, 0.4)),
                Cue::Pause(TRACTOR_GRIP),
            ])
            .then([Cue::ship(from, ShipAction::Move { to, towed: true })])
            .then([Cue::Release])
    }

    /// Plasma Frigate: ein Schuss auf das Ziel. Repair Drone: erst ein Impuls an der Quelle, dann die Reparatur am
    /// Ziel. Scrap Raider: Der geborgene Schrott leuchtet am Raider selbst auf. Bastion: Die Schildblase des
    /// Wächterschilds legt sich um sie und das Schiff vor ihr.
    fn ability(
        &self,
        source: ShipRef,
        slot: SlotRef,
        effect: AbilityEffect,
        after: ShipStats,
    ) -> Script {
        let locale = self.locale;
        let origin = self.center(source.slot());
        let color = side_color(source.side);
        match effect {
            AbilityEffect::Damage { amount } => Script::default()
                .then(self.shoot(origin, slot, color, shot_sound(source.card_id)))
                .then(self.damage(slot, amount, after, origin, color, None)),
            AbilityEffect::Heal { amount } => Script::default()
                .then([
                    Cue::sound(SoundId::Repair, origin),
                    Cue::effect(pulse(origin, color, 1.2, 0.35)),
                ])
                .then(
                    [Cue::ship(slot, ShipAction::Stats(after))]
                        .into_iter()
                        .chain(self.mark(slot, locale.float_heal(amount), FloatTone::Heal, 1.6)),
                ),
            AbilityEffect::Salvage => {
                Script::default().then([Cue::sound(SoundId::PowerUp, origin)].into_iter().chain(
                    self.mark(
                        slot,
                        locale.float_salvage().to_owned(),
                        FloatTone::Buff,
                        1.6,
                    ),
                ))
            }
            AbilityEffect::Shield { amount } => Script::default().then([
                Cue::sound(SoundId::ShieldUp, origin),
                Cue::ship(slot, ShipAction::Stats(after)),
                self.float(slot, locale.float_shield(amount), FloatTone::Shield),
                Cue::effect(Effect::Bubble(Bubble {
                    position: self.center(slot),
                    color: admiral_color(AdmiralId::Osei),
                    size: GUARDIAN_BUBBLE,
                })),
            ]),
        }
    }

    /// Railgun: Ein Strahl schießt von der Pendulum auf das erfasste Schiff, das Fadenkreuz ist damit verbraucht. Ist
    /// das Ziel gedeckt, bricht der Strahl am deckenden Schiff davor ab.
    fn railgun(
        &self,
        source: SlotRef,
        target: SlotRef,
        amount: i32,
        after: ShipStats,
        blocked: bool,
    ) -> Vec<Cue> {
        let color = admiral_color(AdmiralId::Kerr);
        let origin = self.center(source);
        let mut cues = vec![
            Cue::Sound(PlaySound::new(SoundId::CannonHeavy).at(origin).rate(0.8)),
            Cue::detached(pulse(origin, color, 1.1, 0.26)),
            Cue::Shake(0.12),
            Cue::ship(target, ShipAction::Unmark(MarkerKind::Lock)),
        ];
        let beam = |to| {
            Cue::detached(Effect::Beam(Beam {
                from: origin,
                to,
                color,
                width: 1.0,
            }))
        };
        if blocked {
            let cover = target.in_lane(LaneType::Strike);
            let impact = self.impact_from(cover, origin, HULL_RADIUS);
            cues.extend([
                beam(impact),
                self.float(cover, self.locale.float_blocked(), FloatTone::Debuff),
                Cue::ship(cover, ShipAction::Flash(HIT_FLASH_ALPHA)),
                Cue::effect(Effect::Impact(Impact {
                    position: impact,
                    origin,
                    color,
                    variant: Some(HitVariant::Discharge),
                })),
            ]);
        } else {
            cues.push(beam(self.impact_from(target, origin, HULL_RADIUS)));
            cues.extend(self.damage(target, amount, after, origin, color, None));
        }
        cues
    }

    /// Ein Schuss, der verpufft: Mündungsfeuer wie sonst, dann verlischt der Bolzen ohne Einschlag und Schaden dort,
    /// wo er landet, mit „Verpufft“. Ist das verfolgte Ziel gedeckt, zieht sich beim Abfeuern eine Zielmarke um es
    /// zusammen, der Bolzen verlischt vor der Deckung, und am Ziel steht „Gedeckt“.
    fn fizzle(&self, shot: &RoundEvent) -> Script {
        let RoundEvent::Fizzle { attacker, miss, .. } = *shot else {
            unreachable!("verpuffender Schuss")
        };
        let locale = self.locale;
        let color = tracer(shot);
        let origin = self.center(attacker.slot());
        let covered = match miss {
            Miss::Target {
                target: Some(target),
                covered: true,
            } => Some(target.slot()),
            _ => None,
        };
        let end = self.shot_end(shot, origin);
        let ring = covered.map(|slot| {
            Cue::detached(Effect::TargetRing(TargetRing {
                position: self.center(slot),
                color,
            }))
        });
        let label = match (covered, self.landing(shot)) {
            (Some(slot), _) => self.float(slot, locale.float_covered(), FloatTone::Shield),
            (None, Some(slot)) => self.float(slot, locale.float_fizzle(), FloatTone::Debuff),
            (None, None) => self.float_at(end, locale.float_fizzle(), FloatTone::Debuff),
        };
        Script::default()
            .then(
                ring.into_iter()
                    .chain(self.muzzle(origin, color, shot_sound(attacker.card_id)))
                    .chain([Cue::effect(Effect::Fizzle(Fizzle {
                        from: origin,
                        to: end,
                        color,
                    }))]),
            )
            .then([label])
    }

    /// Das Trugbild löst sich auf: am Rundenende oder durch Funkstille.
    fn vanish(&self, slot: SlotRef) -> Vec<Cue> {
        vec![
            Cue::Sound(
                PlaySound::new(SoundId::Jammer)
                    .at(self.center(slot))
                    .volume(0.6),
            ),
            Cue::ship(slot, ShipAction::Vanish),
        ]
    }

    // --- Bausteine ---

    /// Treffer eines Schusses aus Richtung `origin`: Einschlag auf der Hülle, Hüllenblitz und Schadenszahl.
    fn damage(
        &self,
        slot: SlotRef,
        amount: i32,
        after: ShipStats,
        origin: Vec3,
        color: Color,
        variant: Option<HitVariant>,
    ) -> Vec<Cue> {
        vec![
            Cue::ship(slot, ShipAction::Stats(after)),
            self.float(slot, self.locale.float_damage(amount), FloatTone::Damage),
            Cue::ship(slot, ShipAction::Flash(HIT_FLASH_ALPHA)),
            Cue::effect(Effect::Impact(Impact {
                position: self.impact_from(slot, origin, HULL_RADIUS),
                origin,
                color,
                variant,
            })),
        ]
    }

    /// Mündungsblitz und Schuss bei `origin`, dann fliegt der Bolzen bis auf die Hülle des Ziels.
    fn shoot(&self, origin: Vec3, to: SlotRef, color: Color, sound: SoundId) -> Vec<Cue> {
        let mut cues = self.muzzle(origin, color, sound);
        cues.push(Cue::effect(Effect::Bolt(Bolt {
            from: origin,
            to: self.impact_from(to, origin, HULL_RADIUS),
            color,
        })));
        cues
    }

    /// Mündungsblitz und Schussgeräusch bei `origin`.
    fn muzzle(&self, origin: Vec3, color: Color, sound: SoundId) -> Vec<Cue> {
        vec![
            Cue::sound(sound, origin),
            Cue::detached(pulse(origin, color, 0.7, 0.18)),
        ]
    }

    /// Buffs und Debuffs: farbiger Impuls plus Text über dem Schiff.
    fn mark(&self, slot: SlotRef, text: String, tone: FloatTone, size: f32) -> Vec<Cue> {
        vec![
            self.float(slot, text, tone),
            Cue::effect(pulse(self.center(slot), tone.color(), size, 0.65)),
        ]
    }

    /// Wo ein Schuss landet: am getroffenen Schiff. Verpufft er, am schon zerstörten Ziel, vor der Deckung eines
    /// gedeckten, bei Spaltenfeuer am vordersten Schiff, das in der Spalte gefallen ist (es steht bis zu seiner
    /// Zerstörung noch auf dem Slot), sonst im leeren Strike-Slot der Spalte; `None`, wenn sein Ziel nicht mehr auf dem
    /// Brett steht.
    fn landing(&self, shot: &RoundEvent) -> Option<SlotRef> {
        match *shot {
            RoundEvent::Counterfire { ship, .. } => Some(ship.slot()),
            RoundEvent::Attack { target, .. } => Some(target.slot()),
            RoundEvent::Fizzle { attacker, miss, .. } => {
                let side = attacker.side.opposite();
                match miss {
                    Miss::Column { column, .. } => {
                        let front = SlotRef::new(side, LaneType::Strike, column);
                        let back = front.in_lane(LaneType::Command);
                        let fallen_back =
                            !self.ships.contains_key(&front) && self.ships.contains_key(&back);
                        Some(if fallen_back { back } else { front })
                    }
                    Miss::Target { target, covered } => target.map(|target| {
                        if covered {
                            SlotRef::new(side, LaneType::Strike, target.slot_index)
                        } else {
                            target.slot()
                        }
                    }),
                }
            }
            _ => unreachable!("kein Schuss"),
        }
    }

    /// Wo der Bolzen eines Schusses aus `origin` endet: auf der Hülle des Ziels. Verpufft er, verlischt er schon vor
    /// der Stelle, an der er landet, bzw. über der Brettmitte, wenn sein Ziel nicht mehr auf dem Brett steht.
    fn shot_end(&self, shot: &RoundEvent, origin: Vec3) -> Vec3 {
        match self.landing(shot) {
            None => Vec3::new(0.0, origin.y + VOID_RISE, origin.z),
            Some(slot) => {
                let short = matches!(shot, RoundEvent::Fizzle { .. });
                let distance = if short {
                    HULL_RADIUS + FIZZLE_SHORT
                } else {
                    HULL_RADIUS
                };
                self.impact_from(slot, origin, distance)
            }
        }
    }

    fn center(&self, slot: SlotRef) -> Vec3 {
        anchor_position(self.layout, slot) + SHIP_CENTER
    }

    /// Wo ein Schuss aus Richtung `origin` die Hülle des Schiffs auf `slot` trifft; `distance` vor dessen Mitte.
    fn impact_from(&self, slot: SlotRef, origin: Vec3, distance: f32) -> Vec3 {
        let center = self.center(slot);
        center - (center - origin).normalize() * distance
    }

    fn float(&self, slot: SlotRef, text: impl Into<String>, tone: FloatTone) -> Cue {
        self.float_at(self.center(slot), text, tone)
    }

    /// Schwebetext über einem Punkt, an dem gerade ein Schiff steht, auch wenn es nicht auf seinem Slot ist
    /// (Rammkurs).
    fn float_at(&self, point: Vec3, text: impl Into<String>, tone: FloatTone) -> Cue {
        Cue::Float {
            at: point + ABOVE_SHIP,
            text: text.into(),
            tone,
        }
    }
}

/// Seite und Karte, wenn das Ereignis der Ausgang eines Befehls ist (Wirkung, Nachziehen, ohne Wirkung, Fehlschuss,
/// abgefangen, Regel).
fn command_outcome(event: &RoundEvent) -> Option<(Side, CommandId)> {
    match *event {
        RoundEvent::Effect { side, card_id, .. }
        | RoundEvent::Draw { side, card_id, .. }
        | RoundEvent::NoEffect { side, card_id }
        | RoundEvent::Miss { side, card_id, .. }
        | RoundEvent::Countered { side, card_id, .. }
        | RoundEvent::Rule { side, card_id } => Some((side, card_id)),
        _ => None,
    }
}

/// Wer einen Schuss abgibt: im Kampf der Angreifer, beim Gegenfeuer das getroffene Schiff.
fn shooter(shot: &RoundEvent) -> ShipRef {
    match *shot {
        RoundEvent::Counterfire { source, .. } => source,
        RoundEvent::Attack { attacker, .. } | RoundEvent::Fizzle { attacker, .. } => attacker,
        _ => unreachable!("kein Schuss"),
    }
}

/// Die Leuchtspur eines Schusses im Kampf: in der Farbe des Admirals, dessen Fähigkeit ihn verstärkt, lenkt oder
/// beschleunigt.
fn tracer(shot: &RoundEvent) -> Color {
    match *shot {
        RoundEvent::Attack { focus: true, .. } => admiral_color(AdmiralId::Vance),
        RoundEvent::Attack { swarm: true, .. } => admiral_color(AdmiralId::Halvorsen),
        RoundEvent::Attack {
            head_start: true, ..
        }
        | RoundEvent::Fizzle {
            head_start: true, ..
        } => admiral_color(AdmiralId::Kerr),
        _ => side_color(shooter(shot).side),
    }
}
