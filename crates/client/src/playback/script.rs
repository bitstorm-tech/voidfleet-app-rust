//! Der Ablauf als Daten: Ein [`Script`] ist eine Folge von Takten, ein Takt eine Liste von [`Cue`]s, die zusammen
//! beginnen. Das Skript lebt als Entity; [`run_scripts`] spielt je Bild höchstens einen Takt, und zwar erst, wenn alle
//! Aufträge des vorigen fertig sind. Gewartet wird über die Beziehung [`AwaitedBy`]: Jeder Auftrag, auf den das Skript
//! wartet (Animation, Effekt, Pause, Unterskript), trägt sie; er verschwindet, sobald er fertig ist
//! ([`crate::scene::animation`]), und mit dem letzten verschwindet [`Awaiting`] vom Skript. Gleichzeitiges ist ein
//! [`Cue::Fork`] in Unterskripte, die das Skript ebenso abwartet.
//!
//! Die Cues nennen Einheiten ([`UnitRef`]) und Orte ([`Spot`]) statt Entities und Weltpunkte: Welches Schiff gemeint ist
//! und wo es steht, entscheidet sich erst, wenn der Takt beginnt (nach einem Flug steht es schon im neuen Slot).

use std::collections::VecDeque;

use bevy::{ecs::system::SystemParam, prelude::*};
use voidfleet_core::{
    enemies::{EnemyClass, Unit},
    events::{Intent, UnitRef, UnitStats},
};

use super::Playback;
use crate::{
    audio::{PlaySound, RadioCue, Say, SoundId},
    data::ship_models::{ShipKind, ShipModels, muzzles},
    game::Battle,
    palette,
    scene::{
        animation::{Animation, AnimationClock, AnimationScope},
        aura::ShipAura,
        board::{BoardEntities, anchor_position, place},
        camera::{BoardLayout, CameraCommand},
        coords::{BOARD_CENTER, SlotRef},
        destruction::{DestroyHull, DestroyShip},
        effects::{
            Beam, Bolt, ChargedBeam, HitVariant, Impact, Laser, Missile, Pulse, ReleaseTether,
            ShieldImpact, TargetRing, Tether, Warp, bolt_duration,
        },
        floating_text::{FloatTone, FloatingText},
        formation::{Hull, formation, visible_hulls},
        labels::ShipCaption,
        motion::{Arrive, Charge, HitFlash, MoveShip, Withdraw},
        ship::{AimedHull, Ship, ShipModel, Turret, Wreck, spawn_ship},
        status::{ShieldStrike, ShipBounds, ShipStatus},
    },
    ui::{AppendLog, ShowBanner, ShowRoundBanner},
};

/// Mitte eines Schiffs über seinem Anker.
const SHIP_CENTER: Vec3 = Vec3::new(0.0, 0.15, 0.0);
/// Schwebetexte stehen so weit über der Mitte.
const ABOVE_SHIP: Vec3 = Vec3::new(0.0, 0.9, 0.0);
/// Angriffe von oben kommen aus dieser Höhe.
const ORBIT: Vec3 = Vec3::new(0.0, 9.0, 0.0);
/// Schüsse enden nicht in der Mitte, sondern so weit davor auf der Hülle.
const HULL_RADIUS: f32 = 0.45;
/// Ein Schuss, der verfehlt, fliegt so weit über sein Ziel hinaus, bis weit aus dem Bild.
const MISS_RANGE: f32 = 60.0;
/// Eine Mündung so nah an der Hochachse eines Turms (im Raum des Modells) dreht mit ihm: weiter als die längsten Rohre.
const TURRET_REACH: f32 = 2.0;
/// Rammkurs: Der Rammer hält so weit vor der Mitte des Ziels.
const RAM_STANDOFF: f32 = 1.1;
/// Der Traktorstrahl greift von so hoch über dem Flaggschiff.
const TRACTOR_ORIGIN: Vec3 = Vec3::new(0.0, 2.5, 0.0);

/// Ein Auftrag, auf den ein Skript wartet.
#[derive(Component, Clone, Copy, Debug)]
#[relationship(relationship_target = Awaiting)]
pub(super) struct AwaitedBy(pub Entity);

/// Die Aufträge, auf die ein Skript noch wartet; verschwindet mit dem letzten.
#[derive(Component, Debug)]
#[relationship_target(relationship = AwaitedBy)]
pub(super) struct Awaiting(Vec<Entity>);

/// Ein laufendes Skript: die noch offenen Takte und ein Traktorstrahl, den ein späterer Takt freigibt.
#[derive(Component, Default, Debug)]
pub(super) struct Script {
    beats: VecDeque<Vec<Cue>>,
    tether: Option<Entity>,
}

impl Script {
    /// Hängt einen Takt an (ein leerer fällt weg).
    pub fn then(mut self, beat: impl IntoIterator<Item = Cue>) -> Self {
        let beat: Vec<Cue> = beat.into_iter().collect();
        if !beat.is_empty() {
            self.beats.push_back(beat);
        }
        self
    }

    /// Hängt die Takte eines anderen Skripts an; es läuft danach.
    pub fn and(mut self, other: Script) -> Self {
        self.beats.extend(other.beats);
        self
    }

    /// Alle Cues, auch die der Unterskripte.
    #[cfg(test)]
    pub fn cues(&self) -> Vec<&Cue> {
        self.beats
            .iter()
            .flatten()
            .flat_map(|cue| match cue {
                Cue::Fork(scripts) => scripts.iter().flat_map(Script::cues).collect(),
                cue => vec![cue],
            })
            .collect()
    }
}

/// Ein Ort auf dem Brett, aufgelöst beim Start des Takts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Spot {
    /// Mitte einer Einheit; steht sie nicht (mehr) auf dem Brett, ihr Slot.
    Unit(UnitRef),
    /// Hoch über einer Einheit (Angriffe von oben).
    Above(UnitRef),
    /// Eine Spalte der Spielerseite (Spaltenfeuer).
    Column(usize),
    /// Das Ziel des Stillstands (das Ereignis nennt es nicht): der Gegner, der im Stillstand steht.
    StationTarget,
}

/// Ein Effekt der Szene; das Skript wartet auf ihn.
#[derive(Clone, Copy, Debug)]
pub(super) enum Fx {
    /// Verfehlt ein Schuss (`miss`), fliegt er durch `to` hindurch ins Leere.
    Bolt {
        from: Spot,
        to: Spot,
        color: Color,
        miss: bool,
    },
    Beam {
        from: Spot,
        to: Spot,
        color: Color,
        width: f32,
        miss: bool,
    },
    /// Ein Laserstrahl aus einer Mündung der Einheit `from`; `charged`: der aufgeladene Schuss (Spaltenfeuer).
    Laser {
        from: UnitRef,
        to: Spot,
        color: Color,
        miss: bool,
        charged: bool,
    },
    Missile {
        from: Spot,
        to: Spot,
        color: Color,
    },
    Pulse {
        at: Spot,
        color: Color,
        size: f32,
        duration: f32,
    },
    Impact {
        at: Spot,
        from: Spot,
        color: Color,
        variant: Option<HitVariant>,
    },
    /// Ein Schuss verpufft am Schild: Funken und Welle am Schild, die Hülle glüht auf.
    ShieldImpact {
        at: UnitRef,
        from: Spot,
    },
    Ring {
        at: Spot,
        color: Color,
    },
    /// Hyperraum: Portal und Blitz am Ort.
    Warp {
        at: Spot,
    },
}

/// Was mit dem Schiff einer Einheit geschieht. Animationen wartet das Skript ab.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Job {
    Stats(UnitStats),
    Flash,
    /// Flug in einen Slot; `towed`: vom Traktorstrahl gezogen.
    Move {
        to: SlotRef,
        towed: bool,
    },
    /// Rammkurs: Anflug auf einen Ort; [`Job::Withdraw`] holt es zurück.
    Charge(Spot),
    Withdraw,
    Destroy,
    /// Kurzsprung: Das Schiff verschwindet (der Abgleich räumt es danach ab).
    Hide,
    Burning(bool),
    Charged(bool),
    /// Die Absichten unter dem Schiff.
    Caption {
        intent: Option<Intent>,
        next: Option<Intent>,
    },
    /// Ein Traktorstrahl vom Flaggschiff hält das Schiff, bis [`Cue::Release`] ihn freigibt.
    Tether(Color),
}

/// Ein Bestandteil eines Takts.
#[derive(Debug)]
pub(super) enum Cue {
    Log(Vec<String>),
    Banner(ShowBanner),
    RoundBanner(u32),
    Say(RadioCue),
    Sound(SoundId, Spot),
    Shake(f32),
    /// Der Regler der Animationsgeschwindigkeit für alles, was ab hier beginnt.
    Scope(AnimationScope),
    /// Die Kamera neigt sich zu einem Ort oder kehrt zurück.
    Lean(Option<Spot>),
    Float {
        at: Spot,
        text: String,
        detail: String,
        tone: FloatTone,
    },
    /// Eine Pause, die das Skript abwartet.
    Pause(f32),
    Fx(Fx),
    /// Unterskripte laufen gleichzeitig; das Skript wartet auf alle.
    Fork(Vec<Script>),
    Unit(UnitRef, Job),
    /// Die Einheit fällt aus dem Hyperraum in ihren Slot (Verstärkung, Nachrücker, Rückkehr aus dem Kurzsprung).
    Arrive(UnitRef),
    /// Gibt den Traktorstrahl dieses Skripts frei und wartet, bis er verblasst ist.
    Release,
}

impl Cue {
    pub fn float(at: Spot, text: impl Into<String>, tone: FloatTone) -> Self {
        Self::Float {
            at,
            text: text.into(),
            detail: String::new(),
            tone,
        }
    }

    /// Nur Bild und Ton, ohne Folgen für den Zustand der Szene; beim Überspringen fällt es weg.
    fn is_presentation(&self) -> bool {
        matches!(
            self,
            Self::Banner(_)
                | Self::RoundBanner(_)
                | Self::Say(_)
                | Self::Sound(..)
                | Self::Shake(_)
                | Self::Lean(_)
                | Self::Float { .. }
                | Self::Fx(_)
        )
    }
}

/// Was die Takte zum Spielen brauchen.
#[derive(SystemParam)]
pub(super) struct Stage<'w, 's> {
    commands: Commands<'w, 's>,
    ships: Query<'w, 's, (Entity, &'static Ship, &'static GlobalTransform)>,
    ship_children: Query<'w, 's, &'static Children, With<Ship>>,
    groups: Query<'w, 's, &'static Children, (With<ShipModel>, Without<Wreck>)>,
    hulls: Query<'w, 's, (&'static Hull, &'static Visibility, &'static GlobalTransform)>,
    aimed: Query<'w, 's, &'static mut AimedHull>,
    turrets: Query<'w, 's, (&'static Turret, &'static GlobalTransform, &'static ChildOf)>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    bounds: Query<'w, 's, (&'static ShipBounds, &'static GlobalTransform)>,
    statuses: Query<'w, 's, &'static mut ShipStatus>,
    captions: Query<'w, 's, &'static mut ShipCaption>,
    auras: Query<'w, 's, &'static mut ShipAura>,
    board: Res<'w, BoardEntities>,
    layout: Res<'w, BoardLayout>,
    models: Res<'w, ShipModels>,
    battle: Option<Res<'w, Battle>>,
    playback: Res<'w, Playback>,
    clock: ResMut<'w, AnimationClock>,
    logs: MessageWriter<'w, AppendLog>,
    banners: MessageWriter<'w, ShowBanner>,
    round_banners: MessageWriter<'w, ShowRoundBanner>,
    radio: MessageWriter<'w, Say>,
    sounds: MessageWriter<'w, PlaySound>,
    camera: MessageWriter<'w, CameraCommand>,
}

/// Spielt je wartendem Skript den nächsten Takt; ein Skript ohne Takte verschwindet.
pub(super) fn run_scripts(
    mut scripts: Query<(Entity, &mut Script), Without<Awaiting>>,
    mut stage: Stage,
) {
    for (entity, mut script) in &mut scripts {
        match script.beats.pop_front() {
            Some(beat) => {
                for cue in beat {
                    stage.play(entity, &mut script, cue);
                }
            }
            None => stage.commands.entity(entity).despawn(),
        }
    }
}

impl Stage<'_, '_> {
    /// Das Schiff einer Einheit und seine Mitte.
    fn ship(&self, unit: UnitRef) -> Option<(Entity, Vec3)> {
        self.ships
            .iter()
            .find(|(_, ship, _)| match (unit, ship.kind) {
                (UnitRef::Flagship, ShipKind::Flagship(_)) => true,
                (UnitRef::Enemy { uid, .. }, ShipKind::Enemy(_)) => ship.uid == uid,
                _ => false,
            })
            .map(|(entity, _, transform)| (entity, transform.translation() + SHIP_CENTER))
    }

    /// Wo eine Einheit steht oder stünde.
    fn slot(&self, unit: UnitRef) -> SlotRef {
        match unit {
            UnitRef::Enemy { slot, .. } => SlotRef::enemy(slot),
            UnitRef::Flagship => SlotRef::player(
                self.battle
                    .as_ref()
                    .map_or(1, |battle| battle.flagship.column),
            ),
        }
    }

    fn point(&self, spot: Spot) -> Vec3 {
        match spot {
            Spot::Unit(unit) => self.ship(unit).map_or_else(
                || anchor_position(&self.layout, self.slot(unit)) + SHIP_CENTER,
                |(_, at)| at,
            ),
            Spot::Above(unit) => self.point(Spot::Unit(unit)) + ORBIT,
            Spot::Column(column) => {
                anchor_position(&self.layout, SlotRef::player(column)) + SHIP_CENTER
            }
            Spot::StationTarget => self
                .battle
                .as_ref()
                .and_then(|battle| battle.enemies.iter().find(|enemy| enemy.stasis))
                .and_then(|enemy| {
                    self.ships
                        .iter()
                        .find(|(_, ship, _)| {
                            ship.uid == enemy.uid && matches!(ship.kind, ShipKind::Enemy(_))
                        })
                        .map(|(_, _, transform)| transform.translation() + SHIP_CENTER)
                })
                .unwrap_or(BOARD_CENTER),
        }
    }

    /// Die Modelle der Gruppe eines Schiffs mit Platz, Mitte in der Welt und ob sie stehen; dazu ihr Maßstab.
    fn hulls_of(&self, ship: Entity) -> (f32, Vec<(Entity, usize, Vec3, bool)>) {
        let scale = self
            .ships
            .get(ship)
            .map_or(1.0, |(_, ship, _)| formation(ship.kind.class()).0);
        let hulls = self
            .ship_children
            .get(ship)
            .into_iter()
            .flatten()
            .filter_map(|child| self.groups.get(*child).ok())
            .flatten()
            .filter_map(|member| {
                let (hull, visibility, global) = self.hulls.get(*member).ok()?;
                let center = global.translation() + SHIP_CENTER * scale;
                Some((
                    *member,
                    hull.index,
                    center,
                    *visibility != Visibility::Hidden,
                ))
            })
            .collect();
        (scale, hulls)
    }

    /// Die Mündung einer Einheit, die `toward` am nächsten liegt, an einem stehenden Modell ihrer Gruppe; ohne Mündung
    /// ([`muzzles`]) ihre Mitte.
    fn muzzle(&self, unit: UnitRef, toward: Vec3) -> Vec3 {
        let Some((ship, center)) = self.ship(unit) else {
            return self.point(Spot::Unit(unit));
        };
        let muzzles = self
            .ships
            .get(ship)
            .map_or(&[][..], |(_, ship, _)| muzzles(ship.kind));
        self.hulls_of(ship)
            .1
            .into_iter()
            .filter(|(.., standing)| *standing)
            .filter_map(|(hull, ..)| Some((hull, self.hulls.get(hull).ok()?.2)))
            .flat_map(|(hull, global)| muzzles.iter().map(move |&at| self.turned(hull, global, at)))
            .min_by(|a, b| a.distance(toward).total_cmp(&b.distance(toward)))
            .unwrap_or(center)
    }

    /// Ein Punkt `at` im Raum des Modells `hull` in der Welt; liegt er an einem drehbaren Turm ([`Turret`]), dreht er
    /// mit ihm.
    fn turned(&self, hull: Entity, model: &GlobalTransform, at: Vec3) -> Vec3 {
        let point = model.transform_point(at);
        let to_model = model.affine().inverse();
        self.turrets
            .iter()
            .filter(|(turret, ..)| turret.hull == hull)
            .filter_map(|(turret, global, parent)| {
                let rest = self
                    .globals
                    .get(parent.parent())
                    .ok()?
                    .mul_transform(turret.rest);
                let reach = to_model.transform_point3(rest.translation()).distance(at);
                (reach < TURRET_REACH)
                    .then(|| (global.affine() * rest.affine().inverse()).transform_point3(point))
            })
            .next()
            .unwrap_or(point)
    }

    /// Wo ein Schuss von `from` die Hülle bei `to` trifft: bei einer Einheit am vordersten stehenden Modell ihrer
    /// Gruppe, das sich das Schiff für den Treffer-Blitz merkt.
    fn hull_point(&mut self, from: Vec3, to: Spot) -> Vec3 {
        if let Spot::Unit(unit) = to
            && let Some((ship, _)) = self.ship(unit)
        {
            let (scale, hulls) = self.hulls_of(ship);
            if let Some((hull, _, at, _)) = hulls
                .into_iter()
                .filter(|(.., standing)| *standing)
                .min_by(|a, b| a.2.distance(from).total_cmp(&b.2.distance(from)))
            {
                if let Ok(mut aimed) = self.aimed.get_mut(ship) {
                    aimed.0 = Some(hull);
                }
                return at + (from - at).normalize_or_zero() * HULL_RADIUS * scale;
            }
        }
        let to = self.point(to);
        to + (from - to).normalize_or_zero() * HULL_RADIUS
    }

    /// Wo die Schildhülle einer Einheit zum Schützen bei `from` hin liegt.
    fn shield_point(&self, unit: UnitRef, from: Vec3) -> Option<Vec3> {
        let (ship, _) = self.ship(unit)?;
        let (bounds, transform) = self.bounds.get(ship).ok()?;
        Some(bounds.shield_point(transform, from))
    }

    /// Wo ein Geschoss endet: Steht am Ziel (bei Spaltenfeuer: am Flaggschiff in der Spalte) ein Schild, an dessen
    /// Hülle, sonst auf der Hülle des Schiffs. Der Schild gilt vor dem Treffer, die Werte folgen erst mit dem Einschlag.
    fn shot_point(&mut self, from: Vec3, to: Spot) -> Vec3 {
        let unit = match to {
            Spot::Unit(unit) => Some(unit),
            Spot::Column(column) if self.slot(UnitRef::Flagship) == SlotRef::player(column) => {
                Some(UnitRef::Flagship)
            }
            _ => None,
        };
        unit.filter(|&unit| {
            self.ship(unit)
                .and_then(|(ship, _)| self.statuses.get(ship).ok())
                .is_some_and(|status| status.shield > 0 && !status.hidden)
        })
        .and_then(|unit| self.shield_point(unit, from))
        .unwrap_or_else(|| self.hull_point(from, unit.map_or(to, Spot::Unit)))
    }

    fn await_job(&mut self, script: Entity, job: impl Bundle) {
        self.commands.spawn((job, AwaitedBy(script)));
    }

    fn play(&mut self, entity: Entity, script: &mut Script, cue: Cue) {
        if self.playback.skipped && cue.is_presentation() {
            return;
        }
        match cue {
            Cue::Log(lines) => {
                self.logs.write(AppendLog(lines));
            }
            Cue::Banner(banner) => {
                self.banners.write(banner);
            }
            Cue::RoundBanner(round) => {
                self.round_banners.write(ShowRoundBanner(round));
            }
            Cue::Say(cue) => {
                self.radio.write(Say::new(cue));
            }
            Cue::Sound(id, at) => {
                let at = self.point(at);
                self.sounds.write(PlaySound::new(id).at(at));
            }
            Cue::Shake(strength) => {
                self.camera.write(CameraCommand::Shake(strength));
            }
            Cue::Scope(scope) => self.clock.scope = scope,
            Cue::Lean(spot) => {
                let point = spot.map(|spot| self.point(spot));
                self.camera.write(CameraCommand::Lean(point));
            }
            Cue::Float {
                at,
                text,
                detail,
                tone,
            } => {
                let at = self.point(at) + ABOVE_SHIP;
                self.commands.spawn(FloatingText {
                    at,
                    text,
                    detail,
                    tone,
                    ..default()
                });
            }
            Cue::Pause(seconds) => self.await_job(entity, Animation::new(seconds)),
            Cue::Fx(fx) => self.effect(entity, fx),
            Cue::Fork(scripts) => {
                for child in scripts {
                    self.await_job(entity, child);
                }
            }
            Cue::Unit(unit, job) => {
                if let Some((ship, _)) = self.ship(unit) {
                    self.act(entity, script, ship, job);
                }
            }
            Cue::Arrive(unit) => self.arrive(entity, unit),
            Cue::Release => {
                if let Some(tether) = script.tether.take() {
                    self.commands.trigger(ReleaseTether { entity: tether });
                    self.commands.entity(tether).insert(AwaitedBy(entity));
                }
            }
        }
    }

    fn effect(&mut self, script: Entity, fx: Fx) {
        match fx {
            Fx::Bolt {
                from,
                to,
                color,
                miss: true,
            } => {
                // Das Skript wartet nur, bis der Bolzen am Ziel vorbei ist; den Rest seines Flugs fliegt er für sich.
                let from = self.point(from);
                let aim = self.point(to);
                let to = past(from, aim);
                self.commands.spawn(Bolt { from, to, color });
                self.await_job(script, Animation::new(bolt_duration(from, aim)));
            }
            Fx::Bolt {
                from, to, color, ..
            } => {
                let from = self.point(from);
                let to = self.shot_point(from, to);
                self.await_job(script, Bolt { from, to, color });
            }
            Fx::Beam {
                from,
                to,
                color,
                width,
                miss,
            } => {
                let from = self.point(from);
                let to = if miss {
                    past(from, self.point(to))
                } else {
                    self.shot_point(from, to)
                };
                self.await_job(
                    script,
                    Beam {
                        from,
                        to,
                        color,
                        width,
                    },
                );
            }
            Fx::Laser {
                from,
                to,
                color,
                miss,
                charged,
            } => {
                let aim = self.point(to);
                let from = self.muzzle(from, aim);
                let to = if miss {
                    past(from, aim)
                } else {
                    self.shot_point(from, to)
                };
                if charged {
                    self.await_job(script, ChargedBeam { from, to, color });
                } else {
                    self.await_job(script, Laser { from, to, color });
                }
            }
            Fx::Missile { from, to, color } => {
                let from = self.point(from);
                let to = self.shot_point(from, to);
                self.await_job(script, Missile { from, to, color });
            }
            Fx::Pulse {
                at,
                color,
                size,
                duration,
            } => {
                let position = self.point(at);
                self.await_job(
                    script,
                    Pulse {
                        position,
                        color,
                        size,
                        duration,
                    },
                );
            }
            Fx::Impact {
                at,
                from,
                color,
                variant,
            } => {
                let origin = self.point(from);
                let position = self.hull_point(origin, at);
                self.await_job(
                    script,
                    Impact {
                        position,
                        origin,
                        color,
                        variant,
                    },
                );
            }
            Fx::ShieldImpact { at, from } => {
                let origin = self.point(from);
                let position = self
                    .shield_point(at, origin)
                    .unwrap_or_else(|| self.point(Spot::Unit(at)));
                if let Some((ship, _)) = self.ship(at) {
                    self.commands.spawn(ShieldStrike(ship));
                }
                self.await_job(
                    script,
                    ShieldImpact {
                        position,
                        origin,
                        color: palette::SHIELD,
                    },
                );
            }
            Fx::Ring { at, color } => {
                let position = self.point(at);
                self.await_job(script, TargetRing { position, color });
            }
            Fx::Warp { at } => {
                let position = self.point(at);
                self.commands.spawn(Warp {
                    position,
                    forward: Vec3::X,
                    size: 1.0,
                });
            }
        }
    }

    /// Ein Schiff fällt in seinen Slot; ein altes derselben Einheit (das Flaggschiff vor dem Kurzsprung) geht.
    fn arrive(&mut self, script: Entity, unit: UnitRef) {
        if let Some((old, _)) = self.ship(unit) {
            self.commands.entity(old).despawn();
        }
        let battle = self.battle.as_deref();
        let (kind, uid, stats, max_hull) = match unit {
            UnitRef::Enemy { uid, id, .. } => {
                let enemy = battle.and_then(|battle| battle.enemy(uid));
                let max_hull = enemy.map_or_else(|| Unit::from(id).hull, |enemy| enemy.max_hull);
                let stats = UnitStats {
                    hull: max_hull,
                    shield: 0,
                };
                (ShipKind::Enemy(id), uid, stats, max_hull)
            }
            UnitRef::Flagship => {
                let Some(battle) = battle else { return };
                let flagship = &battle.flagship;
                let kind = ShipKind::Flagship(flagship.admiral.flagship());
                (kind, 0, flagship.stats(), flagship.max_hull)
            }
        };
        let slot = self.slot(unit);
        let ship = spawn_ship(&mut self.commands, &self.models, kind);
        place(&mut self.commands, &self.board, ship, slot);
        self.commands.entity(ship).insert((
            Ship { kind, uid },
            ShipStatus::new(stats.hull, max_hull, stats.shield),
            ShipCaption::default(),
            ShipAura::default(),
        ));
        self.await_job(script, Arrive { ship });
    }

    fn act(&mut self, script_entity: Entity, script: &mut Script, ship: Entity, job: Job) {
        match job {
            Job::Stats(stats) => {
                let class = self
                    .ships
                    .get(ship)
                    .map_or(EnemyClass::Flagship, |(_, ship, _)| ship.kind.class());
                let Ok(mut status) = self.statuses.get_mut(ship) else {
                    return;
                };
                let lost = visible_hulls(class, stats.hull, status.max_hull)
                    ..visible_hulls(class, status.hull, status.max_hull);
                status.hull = stats.hull;
                *status = status.with_shield(stats.shield);
                // Wer aus der Staffel fällt, geht unter wie ein ganzes Schiff; der Verfall blendet ihn in der Gruppe aus.
                if !self.playback.skipped {
                    for index in lost {
                        self.commands.spawn(DestroyHull { ship, index });
                    }
                }
            }
            Job::Flash => {
                let hull = self
                    .aimed
                    .get_mut(ship)
                    .ok()
                    .and_then(|mut aimed| aimed.0.take());
                self.await_job(
                    script_entity,
                    HitFlash {
                        hull,
                        ..HitFlash::new(ship)
                    },
                );
            }
            Job::Move { to, towed } => {
                self.await_job(script_entity, MoveShip { ship, to, towed });
            }
            Job::Charge(target) => {
                let from = self
                    .ships
                    .get(ship)
                    .map_or(Vec3::ZERO, |(.., t)| t.translation());
                let target = self.point(target).with_y(from.y);
                let point = target + (from - target).normalize_or_zero() * RAM_STANDOFF;
                self.await_job(script_entity, Charge { ship, point });
            }
            Job::Withdraw => self.await_job(script_entity, Withdraw { ship }),
            Job::Destroy => self.await_job(script_entity, DestroyShip { ship }),
            Job::Hide => {
                self.commands.entity(ship).insert(Visibility::Hidden);
            }
            Job::Burning(on) | Job::Charged(on) => {
                let mut aura = self.auras.get(ship).copied().unwrap_or_default();
                if matches!(job, Job::Burning(_)) {
                    aura.burning = on;
                } else {
                    aura.charged = on;
                }
                self.commands.entity(ship).insert(aura);
            }
            Job::Caption { intent, next } => {
                if let Ok(mut caption) = self.captions.get_mut(ship) {
                    caption.intent = intent;
                    caption.next = next;
                }
            }
            Job::Tether(color) => {
                let from = self.point(Spot::Unit(UnitRef::Flagship)) + TRACTOR_ORIGIN;
                let tether = self
                    .commands
                    .spawn(Tether {
                        from,
                        target: ship,
                        offset: SHIP_CENTER,
                        color,
                    })
                    .id();
                script.tether = Some(tether);
            }
        }
    }
}

/// Weit hinter `aim` auf der Linie von `from` durch `aim`: wohin ein Schuss fliegt, der verfehlt.
fn past(from: Vec3, aim: Vec3) -> Vec3 {
    from + (aim - from).normalize_or_zero() * MISS_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Beim Überspringen fällt nur Bild und Ton weg; was den Zustand der Szene ändert, läuft weiter.
    #[test]
    fn skipping_keeps_state() {
        let unit = UnitRef::Flagship;
        let at = Spot::Unit(unit);
        for cue in [
            Cue::Unit(unit, Job::Destroy),
            Cue::Arrive(unit),
            Cue::Release,
            Cue::Log(Vec::new()),
            Cue::Scope(AnimationScope::Attack),
            Cue::Pause(1.0),
            Cue::Fork(Vec::new()),
        ] {
            assert!(!cue.is_presentation(), "{cue:?}");
        }
        for cue in [
            Cue::Say(RadioCue::Victory),
            Cue::Sound(SoundId::Emp, at),
            Cue::Shake(0.2),
            Cue::Lean(None),
            Cue::float(at, "3", FloatTone::Damage),
            Cue::Fx(Fx::Warp { at }),
            Cue::RoundBanner(2),
        ] {
            assert!(cue.is_presentation(), "{cue:?}");
        }
    }
}
