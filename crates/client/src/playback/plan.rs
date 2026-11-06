//! Was ein Schritt auf dem Brett zeigt: [`Planner::step`] übersetzt seine Ereignisse in ein [`Script`] aus Takten.
//!
//! - **Spielerzug:** Station und Karte zeigen ihre Waffe (Railgun bei Kerr, Plasma bei Reyes, sonst Bolzen; Torpedo,
//!   Wärmesucher und Sperrfeuer als Rakete, Orbitalschlag und Bombardement von oben, Gnadenstoß als heller Strahl,
//!   Brandsatz mit Feuer), Salven auf mehrere Ziele kurz versetzt. Was keinen Schuss hat, zeigt sein Bild am Ziel oder
//!   am Flaggschiff (Schildblase, Entladung, Störung, Zeit-Effekt), Doktrinen und Admiralsstationen ein Banner.
//! - **Gegnerzug:** Je Einheit erst die Ansage (Kamera neigt sich zu ihr, Banner „Wachkreuzer: Zielfeuer“), dann die
//!   Wirkung. Gebündelte Schritte (gemeinsam ausweichen, eintreffen, aussetzen) laufen gleichzeitig unter einem Banner.
//! - **Treffer:** Einschlag, Blitz der Hülle, Balken, die Zahl (Schaden an der Hülle) und darunter ihre Rechnung; der
//!   Treffer am Vordermann ist ein eigener Einschlag ohne eigenes Geschoss. Am Flaggschiff wackelt die Kamera.
//! - **Auflösung:** Brände flackern gemeinsam, Rundenbanner, Gefechtsuhr, neue Absichten zusammen.
//!
//! Der Planer rechnet keine Positionen; die Takte nennen Einheiten und Orte ([`Spot`]), aufgelöst beim Spielen.

use voidfleet_core::{
    admirals::{AdmiralId, AdmiralStation, Station},
    battle::{BattleResult, Target},
    cards::{CardId, DeckCard},
    enemies::{Action, EnemyId},
    events::{BattleEvent, Breakdown, MissReason, UnitRef},
};

use super::{
    radio::radio_cue,
    script::{Cue, Fx, Job, Script, Spot},
    steps::chains,
};
use crate::{
    audio::SoundId,
    data::{ship_models::ShipKind, sounds::shot_sound},
    i18n::{Condition, Messages},
    palette::{self, admiral_color},
    scene::{
        animation::AnimationScope, effects::FIRE_FLASH, effects::HitVariant,
        floating_text::FloatTone, markers::MarkerKind,
    },
    ui::{ShowBanner, round_log::describe_event},
};

/// Pause nach jedem Schritt, damit man sie auseinanderhalten kann.
const STEP_PAUSE: f32 = 0.25;
/// So lange steht die Ansage, bevor die Wirkung einsetzt.
const BANNER_LEAD: f32 = 0.6;
/// Abstand zwischen den Starts zweier Schüsse einer Salve.
const SALVO_STAGGER: f32 = 0.18;
/// Das Rundenbanner steht so lange, bevor Gefechtsuhr und Absichten folgen.
const ROUND_LEAD: f32 = 0.8;
/// Traktorstrahl: Er hält kurz, bevor er zieht.
const TRACTOR_GRIP: f32 = 0.35;
/// Kamerawackeln bei Treffern am Flaggschiff: Grundwert und je Punkt Hüllenschaden, höchstens.
const SHAKE_BASE: f32 = 0.05;
const SHAKE_PER_POINT: f32 = 0.012;
const SHAKE_MAX: f32 = 0.3;

/// Wie ein Schuss aussieht.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Weapon {
    Bolt(Option<HitVariant>),
    Rail,
    Plasma,
    Missile,
    /// Von oben, als Strahl aus dem Orbit.
    Orbital,
    /// Heller Präzisionsstrahl (Gnadenstoß).
    Lance,
    /// Strahlen entlang der Spalten (Spaltenfeuer).
    Column([bool; 3]),
    /// Kein Geschoss, nur der Einschlag (Rammkurs, Nachsetzen).
    Contact(HitVariant),
}

impl Weapon {
    fn variant(self) -> Option<HitVariant> {
        match self {
            Self::Bolt(variant) => variant,
            Self::Rail | Self::Lance => Some(HitVariant::Pierce),
            Self::Plasma => Some(HitVariant::Fire),
            Self::Missile | Self::Orbital => Some(HitVariant::Fireball),
            Self::Column(_) => Some(HitVariant::Discharge),
            Self::Contact(variant) => Some(variant),
        }
    }
}

/// Die Farbe der Schüsse einer Seite.
fn shot_color(source: UnitRef, admiral: AdmiralId) -> bevy::prelude::Color {
    match source {
        UnitRef::Flagship => admiral_color(admiral),
        UnitRef::Enemy { .. } => palette::ENEMY,
    }
}

/// Was man von einer Karte hört, sobald sie wirkt; Schüsse hört man an Waffe und Einschlag.
fn card_sound(card: CardId) -> Option<SoundId> {
    use CardId::*;
    Some(match card {
        EmergencyShields | HoldTheLine | AblativeArmor | BattleStations => SoundId::ShieldUp,
        EmpBlast | Countermeasures => SoundId::Emp,
        Overcharge | PowerSurge | MicroJump | TractorBeam | RedAlert | FireControlDoctrine => {
            SoundId::PowerUp
        }
        OrbitalStrike | Torpedo | CoupDeGrace | Bombardment => SoundId::OrbitalStrike,
        RepairCrew => SoundId::Repair,
        Scavenge => SoundId::Scavenge,
        TargetingJammer | RadioSilence => SoundId::Jammer,
        VolleyFire | Barrage | HeatSeeker => SoundId::MissileVolley,
        Firebomb | CollisionCourse | EvasiveAction => return None,
    })
}

/// Übersetzt Schritte in Skripte.
pub(super) struct Planner<'a> {
    pub locale: &'a dyn Messages,
    pub admiral: AdmiralId,
}

/// Was eine Kette von Folgen über ihre Ereignisse hinweg weiß.
#[derive(Clone, Copy, Debug)]
struct Context {
    weapon: Weapon,
    /// Der Treffer am Vordermann folgt: Er bekommt kein eigenes Geschoss.
    covering: Option<u32>,
    /// Das Flaggschiff rammt und kehrt nach dem Aufprall zurück.
    ramming: bool,
}

impl Planner<'_> {
    /// Der Name einer Einheit ohne Slot: „Wachkreuzer“, „Bastion“.
    fn name(&self, unit: UnitRef) -> String {
        match unit {
            UnitRef::Flagship => self.locale.flagship(self.admiral.flagship()).to_owned(),
            UnitRef::Enemy { id, .. } => self.locale.enemy(id).to_owned(),
        }
    }

    fn banner(&self, title: String, subtitle: Option<String>, portrait: bool) -> Cue {
        Cue::Banner(ShowBanner {
            title,
            subtitle,
            admiral: portrait.then_some(self.admiral),
        })
    }

    fn kind(&self, unit: UnitRef) -> ShipKind {
        match unit {
            UnitRef::Flagship => ShipKind::Flagship(self.admiral.flagship()),
            UnitRef::Enemy { id, .. } => ShipKind::Enemy(id),
        }
    }

    /// Die Waffe der Station Waffen und des Fokusfeuers.
    fn flagship_weapon(&self) -> Weapon {
        match self.admiral {
            AdmiralId::Kerr => Weapon::Rail,
            AdmiralId::Reyes => Weapon::Plasma,
            AdmiralId::Osei | AdmiralId::Vance => Weapon::Bolt(None),
        }
    }

    /// Ein Schritt: Log, dann je Kette die Bilder; mehrere Ketten (gebündelt) laufen gleichzeitig.
    pub fn step(&self, step: &[BattleEvent]) -> Script {
        let combat = step.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Hit { .. }
                    | BattleEvent::Missed { .. }
                    | BattleEvent::Counterfire { .. }
                    | BattleEvent::HoldTheLine { .. }
            )
        });
        let scope = if combat {
            AnimationScope::Attack
        } else {
            AnimationScope::General
        };
        let log = step
            .iter()
            .map(|event| describe_event(self.locale, event, self.admiral))
            .collect();
        let chains = chains(step);
        let body = if chains.len() == 1 {
            self.chain(step, true)
        } else {
            let mut script = Script::default();
            if let Some(banner) = self.bundle_banner(&chains) {
                script = script.then([banner, Cue::Pause(BANNER_LEAD)]);
            }
            script.then([Cue::Fork(
                chains
                    .iter()
                    .enumerate()
                    .map(|(i, chain)| {
                        Script::default()
                            .then([Cue::Pause(i as f32 * SALVO_STAGGER)])
                            .and(self.chain(chain, false))
                    })
                    .collect(),
            )])
        };
        let say = radio_cue(step, self.admiral).map(Cue::Say);
        Script::default()
            .then([Cue::Scope(scope), Cue::Log(log)].into_iter().chain(say))
            .and(body)
            .then([Cue::Pause(STEP_PAUSE)])
    }

    /// Ein Banner über gebündelten Aktionen: „Plünderer, Torpedo-Staffel: Ausweichen“.
    fn bundle_banner(&self, chains: &[&[BattleEvent]]) -> Option<Cue> {
        let BattleEvent::EnemyAction { action, .. } = chains[0][0] else {
            return None;
        };
        let names: Vec<String> = chains
            .iter()
            .filter_map(|chain| match chain[0] {
                BattleEvent::EnemyAction { unit, .. } => Some(self.name(unit)),
                _ => None,
            })
            .collect();
        Some(self.banner(
            format!("{}: {}", names.join(", "), self.locale.action(action)),
            None,
            false,
        ))
    }

    /// Eine Kette: ihr Kopf, dann die Folgen. `announce`: Gegner sagen ihre Aktion selbst an.
    fn chain(&self, chain: &[BattleEvent], announce: bool) -> Script {
        let (head, rest) = chain.split_first().expect("Kette nicht leer");
        match *head {
            BattleEvent::CardPlayed { card, target } => self.card(card, target, rest),
            BattleEvent::StationTriggered { station } => self.station(station, rest),
            BattleEvent::EnemyAction { unit, action } => {
                self.enemy_action(unit, action, rest, announce)
            }
            BattleEvent::RoundStarted { round } => Script::default()
                .then([Cue::RoundBanner(round), Cue::Pause(ROUND_LEAD)])
                .and(self.intents_together(rest)),
            _ => self.sequence(chain, self.plain()),
        }
    }

    /// Nach dem Rundenbanner: Gefechtsuhr, dann alle neuen Absichten zugleich.
    fn intents_together(&self, rest: &[BattleEvent]) -> Script {
        let (intents, others): (Vec<BattleEvent>, Vec<BattleEvent>) = rest
            .iter()
            .cloned()
            .partition(|event| matches!(event, BattleEvent::Intents { .. }));
        self.sequence(&others, self.plain()).then(
            intents
                .iter()
                .flat_map(|event| self.consequence(event, &mut self.plain())),
        )
    }

    fn plain(&self) -> Context {
        Context {
            weapon: Weapon::Bolt(None),
            covering: None,
            ramming: false,
        }
    }

    fn card(&self, card: DeckCard, target: Target, rest: &[BattleEvent]) -> Script {
        let flagship = Spot::Unit(UnitRef::Flagship);
        let name = self.locale.card(card).name;
        let sound: Vec<Cue> = card_sound(card.id)
            .map(|sound| Cue::Sound(sound, flagship))
            .into_iter()
            .collect();
        let shots = |weapon| {
            Script::default()
                .then(sound.iter().map(clone_sound))
                .and(self.sequence(
                    rest,
                    Context {
                        weapon,
                        ..self.plain()
                    },
                ))
        };
        let salvo = |weapon| {
            Script::default()
                .then(sound.iter().map(clone_sound))
                .and(self.salvo(rest, weapon))
        };
        let buff = || {
            Script::default()
                .then(sound.iter().map(clone_sound).chain([
                    Cue::Fx(Fx::Pulse {
                        at: flagship,
                        color: palette::BUFF,
                        size: 2.2,
                        duration: 0.6,
                    }),
                    Cue::float(flagship, name.clone(), FloatTone::Buff),
                ]))
                .and(self.sequence(rest, self.plain()))
        };
        let doctrine = || {
            Script::default()
                .then(
                    [
                        self.banner(name.clone(), None, true),
                        Cue::Pause(BANNER_LEAD),
                    ]
                    .into_iter()
                    .chain(sound.iter().map(clone_sound)),
                )
                .then([Cue::Fx(Fx::Pulse {
                    at: flagship,
                    color: palette::BUFF,
                    size: 2.4,
                    duration: 0.6,
                })])
                .and(self.sequence(rest, self.plain()))
        };
        let at_target = |spot: Spot, color, variant: Option<HitVariant>| {
            let mut beat: Vec<Cue> = sound.iter().map(clone_sound).collect();
            beat.push(Cue::Fx(Fx::Ring { at: spot, color }));
            beat.push(Cue::Fx(Fx::Pulse {
                at: spot,
                color,
                size: 2.0,
                duration: 0.7,
            }));
            if let Some(variant) = variant {
                beat.push(Cue::Fx(Fx::Impact {
                    at: spot,
                    from: Spot::Above(UnitRef::Flagship),
                    color,
                    variant: Some(variant),
                }));
            }
            beat.push(Cue::float(spot, name.clone(), FloatTone::Debuff));
            Script::default().then(beat)
        };
        let chosen = match target {
            Target::Enemy(uid) => Some(Spot::Unit(self.enemy_ref(uid, rest))),
            _ => None,
        };
        match card.id {
            CardId::Torpedo | CardId::HeatSeeker => shots(Weapon::Missile),
            CardId::OrbitalStrike => shots(Weapon::Orbital),
            CardId::CoupDeGrace => shots(Weapon::Lance),
            CardId::Firebomb => shots(Weapon::Bolt(Some(HitVariant::Fire))),
            CardId::VolleyFire => salvo(Weapon::Bolt(None)),
            CardId::Barrage => salvo(Weapon::Missile),
            CardId::Bombardment => salvo(Weapon::Orbital),
            CardId::CollisionCourse => self.sequence(
                rest,
                Context {
                    weapon: Weapon::Contact(HitVariant::Fireball),
                    ..self.plain()
                },
            ),
            CardId::EmpBlast => match chosen {
                Some(spot) => {
                    at_target(spot, palette::DEBUFF, Some(HitVariant::Discharge)).then(match spot {
                        Spot::Unit(unit) => Some(Cue::Unit(unit, Job::Charged(false))),
                        _ => None,
                    })
                }
                None => buff(),
            },
            CardId::RadioSilence => match chosen {
                Some(spot) => at_target(spot, palette::DEBUFF, None).then(match spot {
                    Spot::Unit(unit) => Some(Cue::Unit(unit, Job::Charged(false))),
                    _ => None,
                }),
                None => buff(),
            },
            CardId::TargetingJammer => {
                Script::default().then(sound.iter().map(clone_sound).chain([
                    Cue::Fx(Fx::Pulse {
                        at: flagship,
                        color: palette::DEBUFF,
                        size: 4.0,
                        duration: 0.9,
                    }),
                    Cue::float(flagship, name.clone(), FloatTone::Debuff),
                ]))
            }
            CardId::RedAlert
            | CardId::FireControlDoctrine
            | CardId::BattleStations
            | CardId::HoldTheLine => doctrine(),
            CardId::Countermeasures
            | CardId::Overcharge
            | CardId::PowerSurge
            | CardId::AblativeArmor
            | CardId::Scavenge => buff(),
            CardId::EmergencyShields
            | CardId::RepairCrew
            | CardId::TractorBeam
            | CardId::MicroJump
            | CardId::EvasiveAction => shots(Weapon::Bolt(None)),
        }
    }

    /// Die Einheit hinter einer Kennung, wie die Ereignisse sie nennen. Nennt keines sie (EMP, Funkstille), genügt die
    /// Kennung: Die Bühne findet das Schiff über sie; Art und Slot sind dann nur Platzhalter.
    fn enemy_ref(&self, uid: u32, events: &[BattleEvent]) -> UnitRef {
        events
            .iter()
            .find_map(|event| {
                units(event)
                    .into_iter()
                    .find(|unit| matches!(unit, UnitRef::Enemy { uid: u, .. } if *u == uid))
            })
            .unwrap_or(UnitRef::Enemy {
                uid,
                id: EnemyId::Raiders,
                slot: voidfleet_core::board::EnemySlot::new(voidfleet_core::board::Row::Front, 1),
            })
    }

    fn station(&self, station: Station, rest: &[BattleEvent]) -> Script {
        let weapon = self.flagship_weapon();
        let fire = Context {
            weapon,
            ..self.plain()
        };
        if station != Station::Admiral {
            return self.sequence(rest, fire);
        }
        let banner = Script::default().then([
            self.banner(
                self.locale.station(station, self.admiral).to_owned(),
                None,
                true,
            ),
            Cue::Pause(BANNER_LEAD),
        ]);
        let body = match self.admiral.station() {
            AdmiralStation::FocusFire => self.sequence(rest, fire),
            AdmiralStation::Counterfire => self.sequence(rest, self.plain()),
            AdmiralStation::Stasis => Script::default().then([
                Cue::Sound(SoundId::Jammer, Spot::StationTarget),
                Cue::Fx(Fx::Ring {
                    at: Spot::StationTarget,
                    color: palette::DEBUFF,
                }),
                Cue::Fx(Fx::Pulse {
                    at: Spot::StationTarget,
                    color: palette::DEBUFF,
                    size: 2.6,
                    duration: 1.2,
                }),
                Cue::float(
                    Spot::StationTarget,
                    self.locale.condition(Condition::Stasis),
                    FloatTone::Debuff,
                ),
            ]),
            // Nachsetzen: Alle Brände flackern zugleich auf.
            AdmiralStation::FollowUp => self.salvo(rest, Weapon::Contact(HitVariant::Fire)),
        };
        banner.and(body)
    }

    /// Mehrere Schüsse kurz versetzt, je Schuss mit seinen Folgen.
    fn salvo(&self, events: &[BattleEvent], weapon: Weapon) -> Script {
        let mut shots: Vec<&[BattleEvent]> = Vec::new();
        let mut start = 0;
        let mut covering = None;
        for (i, event) in events.iter().enumerate() {
            if let BattleEvent::Hit {
                target, breakdown, ..
            } = event
            {
                let is_cover = covering.is_some_and(|uid| unit_uid(*target) == Some(uid));
                covering = breakdown.covered.map(|(uid, _)| uid);
                if !is_cover && i > start {
                    shots.push(&events[start..i]);
                    start = i;
                }
            }
        }
        shots.push(&events[start..]);
        let context = Context {
            weapon,
            ..self.plain()
        };
        Script::default().then([Cue::Fork(
            shots
                .into_iter()
                .filter(|shot| !shot.is_empty())
                .enumerate()
                .map(|(i, shot)| {
                    Script::default()
                        .then([Cue::Pause(i as f32 * SALVO_STAGGER)])
                        .and(self.sequence(shot, context))
                })
                .collect(),
        )])
    }

    fn enemy_action(
        &self,
        unit: UnitRef,
        action: Action,
        rest: &[BattleEvent],
        announce: bool,
    ) -> Script {
        let actor = Spot::Unit(unit);
        let mut script = Script::default();
        if announce {
            script = script.then([
                Cue::Lean(Some(actor)),
                self.banner(
                    format!("{}: {}", self.name(unit), self.locale.action(action)),
                    None,
                    false,
                ),
                Cue::Pause(BANNER_LEAD),
            ]);
        }
        let plasma = matches!(
            unit,
            UnitRef::Enemy {
                id: EnemyId::PlasmaCruiser,
                ..
            }
        );
        let fire = |weapon| {
            self.sequence(
                rest,
                Context {
                    weapon,
                    ..self.plain()
                },
            )
        };
        let beat = match action {
            Action::Fire if plasma => return script.and(fire(Weapon::Plasma)),
            Action::Fire => return script.and(fire(Weapon::Bolt(None))),
            Action::ColumnFire { columns } => return script.and(fire(Weapon::Column(columns))),
            Action::Ram => {
                return script
                    .then([Cue::Unit(unit, Job::Charge(Spot::Unit(UnitRef::Flagship)))])
                    .and(fire(Weapon::Contact(HitVariant::Fireball)));
            }
            Action::Evade => vec![
                Cue::Fx(Fx::Pulse {
                    at: actor,
                    color: palette::PLAYER,
                    size: 1.6,
                    duration: 0.35,
                }),
                Cue::Unit(unit, Job::Mark(MarkerKind::Evading, palette::ENEMY)),
            ],
            Action::Charge => vec![
                Cue::Fx(Fx::Pulse {
                    at: actor,
                    color: palette::BUFF,
                    size: 2.4,
                    duration: 1.0,
                }),
                Cue::Unit(unit, Job::Charged(true)),
            ],
            Action::TargetLock => vec![Cue::Fx(Fx::Beam {
                from: actor,
                to: Spot::Unit(UnitRef::Flagship),
                color: palette::ENEMY,
                width: 0.35,
            })],
            Action::Call(_) => vec![Cue::Fx(Fx::Pulse {
                at: actor,
                color: palette::DEBUFF,
                size: 1.8,
                duration: 0.8,
            })],
            Action::Shields(_) | Action::Bolt => Vec::new(),
        };
        let script = script.then(beat);
        if !matches!(action, Action::Shields(_) | Action::Bolt) {
            return script.and(self.sequence(rest, self.plain()));
        }
        // Schilde und Riegel: die Blasen aller zugleich.
        script.then([Cue::Fork(
            rest.iter()
                .map(|event| self.sequence(std::slice::from_ref(event), self.plain()))
                .collect(),
        )])
    }

    /// Folgen nacheinander, Treffer mit Geschoss und Einschlag.
    fn sequence(&self, events: &[BattleEvent], mut context: Context) -> Script {
        let mut script = Script::default();
        for (i, event) in events.iter().enumerate() {
            match *event {
                BattleEvent::Hit {
                    source,
                    target,
                    breakdown,
                    after,
                } => {
                    let cover = breakdown
                        .covered
                        .map(|(uid, _)| self.enemy_ref(uid, &events[i..]));
                    let covering = context
                        .covering
                        .is_some_and(|uid| unit_uid(target) == Some(uid));
                    context.covering = breakdown.covered.map(|(uid, _)| uid);
                    if !covering {
                        script = script.then(self.projectile(source, target, context.weapon));
                    }
                    script = script.then(self.impact(
                        source,
                        target,
                        &breakdown,
                        after,
                        cover,
                        context.weapon,
                    ));
                    if context.ramming && target == UnitRef::Flagship {
                        context.ramming = false;
                        script = script.then([Cue::Unit(UnitRef::Flagship, Job::Withdraw)]);
                    }
                }
                BattleEvent::Missed { source, reason } => {
                    script = script.and(self.missed(source, reason, context.weapon));
                }
                BattleEvent::Rammed { target } => {
                    context.ramming = true;
                    script = script.then([Cue::Unit(
                        UnitRef::Flagship,
                        Job::Charge(Spot::Unit(target)),
                    )]);
                }
                BattleEvent::Tractored { unit, .. } => {
                    let UnitRef::Enemy { slot, .. } = unit else {
                        continue;
                    };
                    script = script
                        .then([Cue::Unit(unit, Job::Tether(palette::PLAYER))])
                        .then([Cue::Pause(TRACTOR_GRIP)])
                        .then([Cue::Unit(
                            unit,
                            Job::Move {
                                to: crate::scene::coords::SlotRef::enemy(slot),
                                towed: true,
                            },
                        )])
                        .then([Cue::Release]);
                }
                _ => script = script.then(self.consequence(event, &mut context)),
            }
        }
        if context.ramming {
            script = script.then([Cue::Unit(UnitRef::Flagship, Job::Withdraw)]);
        }
        script
    }

    /// Das Geschoss eines Schusses samt Mündungsklang.
    fn projectile(&self, source: UnitRef, target: UnitRef, weapon: Weapon) -> Vec<Cue> {
        let from = Spot::Unit(source);
        let to = Spot::Unit(target);
        let color = shot_color(source, self.admiral);
        let sound = Cue::Sound(shot_sound(self.kind(source)), from);
        let fx = match weapon {
            Weapon::Bolt(_) => Fx::Bolt { from, to, color },
            Weapon::Plasma => Fx::Bolt {
                from,
                to,
                color: FIRE_FLASH,
            },
            Weapon::Rail => Fx::Beam {
                from,
                to,
                color,
                width: 1.0,
            },
            Weapon::Lance => Fx::Beam {
                from,
                to,
                color: palette::TEXT,
                width: 0.55,
            },
            Weapon::Missile => Fx::Missile { from, to, color },
            Weapon::Orbital => Fx::Beam {
                from: Spot::Above(target),
                to,
                color,
                width: 1.3,
            },
            Weapon::Column(columns) => {
                return std::iter::once(sound)
                    .chain(column_beams(from, columns, color))
                    .collect();
            }
            Weapon::Contact(_) => return Vec::new(),
        };
        vec![sound, Cue::Fx(fx)]
    }

    /// Einschlag, Blitz, Balken, Zahl samt Rechnung; am Flaggschiff wackelt die Kamera.
    fn impact(
        &self,
        source: UnitRef,
        target: UnitRef,
        breakdown: &Breakdown,
        after: voidfleet_core::events::UnitStats,
        cover: Option<UnitRef>,
        weapon: Weapon,
    ) -> Vec<Cue> {
        let at = Spot::Unit(target);
        let mut beat = vec![
            Cue::Fx(Fx::Impact {
                at,
                from: Spot::Unit(source),
                color: shot_color(source, self.admiral),
                variant: weapon.variant(),
            }),
            Cue::Unit(target, Job::Flash),
            Cue::Unit(target, Job::Stats(after)),
            Cue::Float {
                at,
                text: breakdown.hull.to_string(),
                detail: self.marks(breakdown, cover),
                tone: FloatTone::Damage,
            },
        ];
        if target == UnitRef::Flagship {
            beat.push(Cue::Shake(
                (SHAKE_BASE + SHAKE_PER_POINT * breakdown.hull as f32).min(SHAKE_MAX),
            ));
        }
        beat
    }

    /// Die Marken unter der Zahl: „weicht aus: 6 · Panzerung −2 · gedeckt: 4 → Wachkreuzer · Schild schluckt 8“.
    fn marks(&self, breakdown: &Breakdown, cover: Option<UnitRef>) -> String {
        let locale = self.locale;
        let mut marks = Vec::new();
        if breakdown.evaded > 0 {
            marks.push(locale.mark_evaded(breakdown.evaded));
        }
        if breakdown.countermeasures > 0 {
            marks.push(locale.mark_countermeasures(breakdown.countermeasures));
        }
        if breakdown.armor > 0 {
            marks.push(locale.mark_armor(breakdown.armor));
        }
        if let (Some((_, amount)), Some(cover)) = (breakdown.covered, cover) {
            marks.push(locale.mark_covered(amount, &self.name(cover)));
        }
        if breakdown.shielded > 0 {
            marks.push(locale.mark_shielded(breakdown.shielded));
        }
        if breakdown.lethal {
            marks.push(locale.breakdown_words()[5].to_owned());
        }
        marks.join(" · ")
    }

    /// Ein Schuss ins Leere: Spaltenfeuer trifft die leere Spalte, sonst verpufft er am Platz des Flaggschiffs.
    fn missed(&self, source: UnitRef, reason: MissReason, weapon: Weapon) -> Script {
        let from = Spot::Unit(source);
        let color = shot_color(source, self.admiral);
        let sound = Cue::Sound(shot_sound(self.kind(source)), from);
        let float = Cue::float(
            Spot::Unit(UnitRef::Flagship),
            self.locale.report_missed(),
            FloatTone::Buff,
        );
        match (weapon, reason) {
            (Weapon::Column(columns), _) => {
                let pulses = (0..3).filter(|&c| columns[c]).map(|column| {
                    Cue::Fx(Fx::Pulse {
                        at: Spot::Column(column),
                        color,
                        size: 1.6,
                        duration: 0.5,
                    })
                });
                Script::default()
                    .then(std::iter::once(sound).chain(column_beams(from, columns, color)))
                    .then(pulses.chain([float]))
            }
            _ => Script::default().then([
                sound,
                Cue::Fx(Fx::Fizzle {
                    from,
                    to: Spot::Unit(UnitRef::Flagship),
                    color,
                }),
                float,
            ]),
        }
    }

    /// Eine Folge ohne Geschoss in einem Takt.
    fn consequence(&self, event: &BattleEvent, context: &mut Context) -> Vec<Cue> {
        let locale = self.locale;
        let mut beat = Vec::new();
        let flagship = Spot::Unit(UnitRef::Flagship);
        let pulse = |at, color, size| {
            Cue::Fx(Fx::Pulse {
                at,
                color,
                size,
                duration: 0.6,
            })
        };
        let gain = |unit: UnitRef, amount: i32, after, color, tone| {
            vec![
                pulse(Spot::Unit(unit), color, 1.8),
                Cue::Unit(unit, Job::Stats(after)),
                Cue::float(Spot::Unit(unit), format!("+{amount}"), tone),
            ]
        };
        let loss = |unit: UnitRef, amount: i32, after, color| {
            vec![
                pulse(Spot::Unit(unit), color, 1.4),
                Cue::Unit(unit, Job::Flash),
                Cue::Unit(unit, Job::Stats(after)),
                Cue::float(Spot::Unit(unit), amount.to_string(), FloatTone::Damage),
            ]
        };
        match *event {
            BattleEvent::Shielded {
                target,
                amount,
                after,
            } => beat.extend([
                Cue::Fx(Fx::Bubble {
                    at: Spot::Unit(target),
                    color: palette::SHIELD,
                }),
                Cue::Unit(target, Job::Stats(after)),
                Cue::float(Spot::Unit(target), format!("+{amount}"), FloatTone::Shield),
            ]),
            BattleEvent::Healed {
                target,
                amount,
                after,
            } => beat.extend(gain(target, amount, after, palette::HEAL, FloatTone::Heal)),
            BattleEvent::Cannibalized {
                unit,
                amount,
                after,
            } => beat.extend(gain(unit, amount, after, palette::HEAL, FloatTone::Heal)),
            BattleEvent::Ignited { target } => {
                beat.extend([
                    Cue::Unit(target, Job::Burning(true)),
                    Cue::float(
                        Spot::Unit(target),
                        locale.condition(Condition::Burning),
                        FloatTone::Debuff,
                    ),
                ]);
                if target == UnitRef::Flagship {
                    beat.push(self.banner(
                        locale.condition(Condition::Burning),
                        Some(self.name(target)),
                        true,
                    ));
                }
            }
            BattleEvent::Extinguished { target } => beat.extend([
                Cue::Unit(target, Job::Burning(false)),
                pulse(Spot::Unit(target), palette::HEAL, 1.6),
            ]),
            BattleEvent::Emergency { target, active } => {
                let tone = if active {
                    FloatTone::Debuff
                } else {
                    FloatTone::Heal
                };
                beat.push(Cue::float(
                    Spot::Unit(target),
                    locale.condition(Condition::Emergency),
                    tone,
                ));
                if target == UnitRef::Flagship && active {
                    beat.extend([
                        self.banner(
                            locale.condition(Condition::Emergency),
                            Some(self.name(target)),
                            true,
                        ),
                        Cue::Shake(SHAKE_MAX),
                    ]);
                }
            }
            BattleEvent::Destroyed { target, cause } => {
                beat.push(Cue::Unit(target, Job::Destroy));
                if cause == voidfleet_core::events::DestroyCause::BurnedOut {
                    beat.push(self.banner(
                        locale.report_burned_out(&self.name(target)),
                        None,
                        false,
                    ));
                }
            }
            BattleEvent::FireshipSpent { unit } => beat.push(Cue::Unit(unit, Job::Destroy)),
            BattleEvent::Jumped => beat.extend([
                Cue::Fx(Fx::Warp { at: flagship }),
                Cue::Sound(SoundId::PowerUp, flagship),
                Cue::Unit(UnitRef::Flagship, Job::Hide),
            ]),
            BattleEvent::Returned => beat.push(Cue::Arrive(UnitRef::Flagship)),
            BattleEvent::Arrived { unit } => beat.push(Cue::Arrive(unit)),
            BattleEvent::Maneuvered { to, .. } => beat.push(Cue::Unit(
                UnitRef::Flagship,
                Job::Move {
                    to: crate::scene::coords::SlotRef::player(to),
                    towed: false,
                },
            )),
            BattleEvent::Idle { unit, reason } => {
                let condition = match reason {
                    voidfleet_core::events::IdleReason::Stasis => Condition::Stasis,
                    voidfleet_core::events::IdleReason::Silenced => Condition::Silenced,
                };
                beat.extend([
                    Cue::Fx(Fx::Ring {
                        at: Spot::Unit(unit),
                        color: palette::DEBUFF,
                    }),
                    Cue::float(
                        Spot::Unit(unit),
                        locale.condition(condition),
                        FloatTone::Debuff,
                    ),
                ]);
            }
            BattleEvent::Counterfire {
                target,
                amount,
                after,
            }
            | BattleEvent::HoldTheLine {
                target,
                amount,
                after,
            } => {
                let breakdown = Breakdown {
                    base: amount,
                    hull: amount,
                    lethal: after.hull == 0,
                    ..Breakdown::default()
                };
                let weapon = Weapon::Bolt(None);
                let mut shot = self.projectile(UnitRef::Flagship, target, weapon);
                shot.extend(self.impact(
                    UnitRef::Flagship,
                    target,
                    &breakdown,
                    after,
                    None,
                    weapon,
                ));
                // Rückschuss: erst das Geschoss, Einschlag und Zahl gleich danach.
                beat.push(Cue::Fork(vec![
                    Script::default().then(shot.drain(..2)).then(shot),
                ]));
            }
            BattleEvent::SelfDamage { amount, after } => {
                beat.extend(loss(UnitRef::Flagship, amount, after, palette::BUFF))
            }
            BattleEvent::BurnTick {
                target,
                amount,
                after,
            } => beat.extend(loss(target, amount, after, FIRE_FLASH)),
            BattleEvent::EmergencyGenerator { after } => beat.extend([
                self.banner(locale.log_emergency_generator().to_owned(), None, true),
                pulse(flagship, palette::HEAL, 2.4),
                Cue::Unit(UnitRef::Flagship, Job::Stats(after)),
            ]),
            BattleEvent::Intents { unit, intents } => {
                let [first, second] = intents.map(|intent| locale.intent(intent));
                beat.push(Cue::Unit(
                    unit,
                    Job::Caption {
                        intent: first,
                        next: if self.admiral == AdmiralId::Kerr {
                            second
                        } else {
                            String::new()
                        },
                    },
                ));
            }
            BattleEvent::Clock { bonus } => beat.push(self.banner(
                locale.hud_clock(bonus),
                Some(locale.hud_clock_hint(bonus)),
                false,
            )),
            BattleEvent::RoundStarted { round } => beat.push(Cue::RoundBanner(round)),
            BattleEvent::BattleEnded { result } => {
                beat.push(Cue::Lean(None));
                if result == BattleResult::Lost {
                    beat.push(Cue::Unit(UnitRef::Flagship, Job::Destroy));
                }
            }
            BattleEvent::Rammed { target } => {
                context.ramming = true;
                beat.push(Cue::Unit(
                    UnitRef::Flagship,
                    Job::Charge(Spot::Unit(target)),
                ));
            }
            BattleEvent::StationTriggered { .. } | BattleEvent::CardPlayed { .. } => {}
            BattleEvent::EnemyAction { unit, .. } => beat.push(Cue::Lean(Some(Spot::Unit(unit)))),
            // Die Hand zeigt Mischen und Nachziehen selbst.
            BattleEvent::Shuffled | BattleEvent::Drawn { .. } => {}
            BattleEvent::Hit { .. }
            | BattleEvent::Missed { .. }
            | BattleEvent::Tractored { .. } => {
                unreachable!("in sequence")
            }
        }
        beat
    }
}

/// Die Strahlen eines Spaltenfeuers, je Spalte einer.
fn column_beams(
    from: Spot,
    columns: [bool; 3],
    color: bevy::prelude::Color,
) -> impl Iterator<Item = Cue> {
    (0..3).filter(move |&c| columns[c]).map(move |column| {
        Cue::Fx(Fx::Beam {
            from,
            to: Spot::Column(column),
            color,
            width: 1.6,
        })
    })
}

fn clone_sound(cue: &Cue) -> Cue {
    match cue {
        Cue::Sound(id, at) => Cue::Sound(*id, *at),
        _ => unreachable!("nur Klänge"),
    }
}

fn unit_uid(unit: UnitRef) -> Option<u32> {
    match unit {
        UnitRef::Enemy { uid, .. } => Some(uid),
        UnitRef::Flagship => None,
    }
}

/// Die Einheiten, die ein Ereignis nennt.
fn units(event: &BattleEvent) -> Vec<UnitRef> {
    match *event {
        BattleEvent::Hit { source, target, .. } => vec![source, target],
        BattleEvent::Missed { source, .. } => vec![source],
        BattleEvent::Shielded { target, .. }
        | BattleEvent::Healed { target, .. }
        | BattleEvent::Ignited { target }
        | BattleEvent::Extinguished { target }
        | BattleEvent::Emergency { target, .. }
        | BattleEvent::Destroyed { target, .. }
        | BattleEvent::Rammed { target }
        | BattleEvent::Counterfire { target, .. }
        | BattleEvent::HoldTheLine { target, .. }
        | BattleEvent::BurnTick { target, .. } => vec![target],
        BattleEvent::FireshipSpent { unit }
        | BattleEvent::Tractored { unit, .. }
        | BattleEvent::Arrived { unit }
        | BattleEvent::EnemyAction { unit, .. }
        | BattleEvent::Idle { unit, .. }
        | BattleEvent::Cannibalized { unit, .. }
        | BattleEvent::Intents { unit, .. } => vec![unit],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::{
        board::{EnemySlot, Row},
        events::{DestroyCause, IdleReason, Intent, UnitStats},
    };

    use super::*;
    use crate::{i18n::Locale, settings::Language};

    const GUARD: UnitRef = UnitRef::Enemy {
        uid: 1,
        id: EnemyId::GuardCruiser,
        slot: EnemySlot::new(Row::Front, 1),
    };
    const STATS: UnitStats = UnitStats {
        hull: 10,
        shield: 2,
    };

    /// Jede Variante von [`BattleEvent`] einmal.
    pub(crate) fn all_events() -> Vec<BattleEvent> {
        let intent = Intent {
            action: Action::Fire,
            damage: 4,
        };
        let hit = |source, target| BattleEvent::Hit {
            source,
            target,
            breakdown: Breakdown {
                base: 8,
                armor: 2,
                hull: 6,
                ..Breakdown::default()
            },
            after: STATS,
        };
        vec![
            BattleEvent::RoundStarted { round: 2 },
            BattleEvent::CardPlayed {
                card: DeckCard::new(CardId::RedAlert),
                target: Target::None,
            },
            BattleEvent::StationTriggered {
                station: Station::Admiral,
            },
            hit(UnitRef::Flagship, GUARD),
            BattleEvent::Missed {
                source: GUARD,
                reason: MissReason::Jumped,
            },
            BattleEvent::Shielded {
                target: GUARD,
                amount: 2,
                after: STATS,
            },
            BattleEvent::Healed {
                target: UnitRef::Flagship,
                amount: 5,
                after: STATS,
            },
            BattleEvent::Ignited { target: GUARD },
            BattleEvent::Extinguished {
                target: UnitRef::Flagship,
            },
            BattleEvent::Emergency {
                target: UnitRef::Flagship,
                active: true,
            },
            BattleEvent::Destroyed {
                target: GUARD,
                cause: DestroyCause::BurnedOut,
            },
            BattleEvent::FireshipSpent { unit: GUARD },
            BattleEvent::Rammed { target: GUARD },
            BattleEvent::Jumped,
            BattleEvent::Returned,
            BattleEvent::Maneuvered { from: 1, to: 0 },
            BattleEvent::Tractored {
                unit: GUARD,
                from: EnemySlot::new(Row::Front, 0),
            },
            BattleEvent::Arrived { unit: GUARD },
            BattleEvent::EnemyAction {
                unit: GUARD,
                action: Action::Charge,
            },
            BattleEvent::Idle {
                unit: GUARD,
                reason: IdleReason::Stasis,
            },
            BattleEvent::Counterfire {
                target: GUARD,
                amount: 3,
                after: STATS,
            },
            BattleEvent::HoldTheLine {
                target: GUARD,
                amount: 2,
                after: STATS,
            },
            BattleEvent::SelfDamage {
                amount: 3,
                after: STATS,
            },
            BattleEvent::BurnTick {
                target: GUARD,
                amount: 2,
                after: STATS,
            },
            BattleEvent::Cannibalized {
                unit: GUARD,
                amount: 8,
                after: STATS,
            },
            BattleEvent::EmergencyGenerator { after: STATS },
            BattleEvent::Shuffled,
            BattleEvent::Drawn { cards: Vec::new() },
            BattleEvent::Intents {
                unit: GUARD,
                intents: [intent, intent],
            },
            BattleEvent::Clock { bonus: 1 },
            BattleEvent::BattleEnded {
                result: BattleResult::Won,
            },
        ]
    }

    /// Ob ein Cue etwas auf Brett, Oberfläche oder im Ton zeigt (Log, Regler und Pausen zählen nicht).
    fn shows(cue: &Cue) -> bool {
        !matches!(cue, Cue::Log(_) | Cue::Scope(_) | Cue::Pause(_))
    }

    /// Jede Ereignisart zeigt etwas, für sich allein und bei jedem Admiral; nur Mischen und Nachziehen zeigt die Hand.
    #[test]
    fn every_event_shows_something() {
        let locale = Locale::new(Language::De);
        for admiral in [
            AdmiralId::Osei,
            AdmiralId::Vance,
            AdmiralId::Kerr,
            AdmiralId::Reyes,
        ] {
            let planner = Planner {
                locale: &*locale,
                admiral,
            };
            for event in all_events() {
                let script = planner.step(std::slice::from_ref(&event));
                let silent = matches!(event, BattleEvent::Shuffled | BattleEvent::Drawn { .. });
                assert_eq!(
                    script.cues().into_iter().any(shows),
                    !silent,
                    "{admiral:?}: {event:?}"
                );
                assert!(matches!(script.cues()[1], Cue::Log(lines) if lines.len() == 1));
            }
        }
    }

    /// Jede Karte zeigt etwas; was nicht angreift und nichts auslöst, auch ohne Folgeereignisse.
    #[test]
    fn every_card_shows_something() {
        let locale = Locale::new(Language::En);
        let planner = Planner {
            locale: &*locale,
            admiral: AdmiralId::Osei,
        };
        for id in voidfleet_core::cards::LOOT_POOL {
            let target = match id.target() {
                voidfleet_core::cards::CardTarget::None => Target::None,
                voidfleet_core::cards::CardTarget::Enemy => Target::Enemy(1),
                voidfleet_core::cards::CardTarget::Column => Target::Column(0),
            };
            let event = BattleEvent::CardPlayed {
                card: DeckCard::new(id),
                target,
            };
            let consequence = match id {
                CardId::EmergencyShields | CardId::HoldTheLine => Some(BattleEvent::Shielded {
                    target: UnitRef::Flagship,
                    amount: 4,
                    after: STATS,
                }),
                CardId::RepairCrew => Some(BattleEvent::Healed {
                    target: UnitRef::Flagship,
                    amount: 4,
                    after: STATS,
                }),
                CardId::MicroJump => Some(BattleEvent::Jumped),
                CardId::CollisionCourse => Some(BattleEvent::Rammed { target: GUARD }),
                CardId::EvasiveAction => Some(BattleEvent::Maneuvered { from: 1, to: 0 }),
                CardId::Torpedo
                | CardId::OrbitalStrike
                | CardId::HeatSeeker
                | CardId::CoupDeGrace
                | CardId::Firebomb
                | CardId::VolleyFire
                | CardId::Barrage
                | CardId::Bombardment => Some(BattleEvent::Hit {
                    source: UnitRef::Flagship,
                    target: GUARD,
                    breakdown: Breakdown::default(),
                    after: STATS,
                }),
                _ => None,
            };
            let step: Vec<BattleEvent> = std::iter::once(event).chain(consequence).collect();
            let script = planner.step(&step);
            assert!(script.cues().into_iter().any(shows), "{id:?}");
        }
    }

    /// Ein Treffer mit Deckung: Der Schuss fliegt einmal, beide Ziele bekommen Einschlag und Zahl; die Rechnung nennt
    /// den Vordermann.
    #[test]
    fn covered_hit_has_one_projectile_and_two_numbers() {
        let locale = Locale::new(Language::De);
        let planner = Planner {
            locale: &*locale,
            admiral: AdmiralId::Osei,
        };
        let back = UnitRef::Enemy {
            uid: 2,
            id: EnemyId::Spotter,
            slot: EnemySlot::new(Row::Back, 1),
        };
        let step = [
            BattleEvent::StationTriggered {
                station: Station::Weapons,
            },
            BattleEvent::Hit {
                source: UnitRef::Flagship,
                target: back,
                breakdown: Breakdown {
                    base: 8,
                    covered: Some((1, 4)),
                    hull: 4,
                    ..Breakdown::default()
                },
                after: STATS,
            },
            BattleEvent::Hit {
                source: UnitRef::Flagship,
                target: GUARD,
                breakdown: Breakdown {
                    base: 4,
                    hull: 4,
                    ..Breakdown::default()
                },
                after: STATS,
            },
        ];
        let script = planner.step(&step);
        let cues = script.cues();
        let bolts = cues
            .iter()
            .filter(|cue| matches!(cue, Cue::Fx(Fx::Bolt { .. })))
            .count();
        let numbers: Vec<&str> = cues
            .iter()
            .filter_map(|cue| match cue {
                Cue::Float { detail, .. } => Some(detail.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(bolts, 1);
        assert_eq!(numbers, ["gedeckt: 4 → Wachkreuzer", ""]);
    }
}
