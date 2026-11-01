//! Der Ablauf als Daten: Ein [`Script`] ist eine Folge von Takten, ein Takt eine Liste von [`Cue`]s, die zusammen
//! beginnen. Das Skript lebt als Entity; [`run_scripts`] spielt je Bild höchstens einen Takt, und zwar erst, wenn alle
//! Aufträge des vorigen fertig sind. Gewartet wird über die Beziehung [`AwaitedBy`]: Jeder Auftrag, auf den das Skript
//! wartet (Animation, Effekt, Pause, Unterskript), trägt sie; er verschwindet, sobald er fertig ist
//! ([`crate::scene::animation`]), und mit dem letzten verschwindet [`Awaiting`] vom Skript. Gleichzeitiges ist ein
//! [`Cue::Fork`] in Unterskripte, die das Skript ebenso abwartet. Ist das letzte Takt gespielt, verschwindet das Skript.
//!
//! Die Cues tragen Slots statt Schiffs-Entities: Welches Schiff gemeint ist, entscheidet sich erst, wenn der Takt
//! beginnt — nach einem Flug steht es schon auf seinem neuen Slot ([`OnSlot`]).

use std::collections::VecDeque;

use bevy::{ecs::system::SystemParam, prelude::*};
use voidfleet_core::{board::SlotRef, cards::ShipId, protocol::ShipStats};

use super::{Playback, plan::SHIP_CENTER};
use crate::{
    audio::{PlaySound, PlayUiSound, RadioCue, Say, SoundId, UiSound},
    data::ship_models::ShipModels,
    scene::{
        animation::Animation,
        board::{BoardEntities, OnSlot, place},
        camera::CameraCommand,
        destruction::DestroyShip,
        effects::{
            Beam, Bolt, Bubble, Detonation, Fizzle, Impact, Missile, Pulse, ReleaseTether,
            TargetRing, Tether,
        },
        floating_text::{FloatTone, FloatingText},
        markers::{DropMarker, MarkerKind, MarkerSpec, ShipMarkers},
        motion::{
            Arrive, Charge, Dematerialize, HitFlash, Materialize, MoveShip, PhaseIn, PhaseOut,
            Withdraw,
        },
        ship::{BOARD_SCALE, Ship, spawn_ship},
        status::ShipStatus,
        tween::{AnimationClock, AnimationScope},
    },
    ui::{AppendLog, ShowBanner},
};

/// Ein Auftrag, auf den ein Skript wartet.
#[derive(Component, Clone, Copy, Debug)]
#[relationship(relationship_target = Awaiting)]
pub(super) struct AwaitedBy(pub Entity);

/// Die Aufträge, auf die ein Skript noch wartet; verschwindet mit dem letzten.
#[derive(Component, Debug)]
#[relationship_target(relationship = AwaitedBy)]
pub(super) struct Awaiting(Vec<Entity>);

/// Ein laufendes Skript: die noch offenen Takte und ein Traktorstrahl, den ein späterer Takt freigibt.
#[derive(Component, Default)]
pub(super) struct Script {
    beats: VecDeque<Vec<Cue>>,
    tether: Option<Entity>,
}

impl Script {
    /// Hängt einen Takt an.
    pub fn then(mut self, beat: impl IntoIterator<Item = Cue>) -> Self {
        self.beats.push_back(beat.into_iter().collect());
        self
    }

    /// Hängt die Takte eines anderen Skripts an; es läuft danach.
    pub fn and(mut self, other: Script) -> Self {
        self.beats.extend(other.beats);
        self
    }

    /// Fügt `cues` in den ersten Takt ein (ein leeres Skript bekommt einen).
    pub fn with(mut self, cues: impl IntoIterator<Item = Cue>) -> Self {
        match self.beats.front_mut() {
            Some(first) => first.extend(cues),
            None => self.beats.push_back(cues.into_iter().collect()),
        }
        self
    }
}

/// Ein Effekt der Szene, als Auftrag gespawnt.
#[derive(Clone, Copy, Debug)]
pub(super) enum Effect {
    Bolt(Bolt),
    Fizzle(Fizzle),
    TargetRing(TargetRing),
    Beam(Beam),
    Missile(Missile),
    Bubble(Bubble),
    Pulse(Pulse),
    Impact(Impact),
    Detonation(Detonation),
}

impl Effect {
    fn spawn(self, commands: &mut Commands) -> Entity {
        match self {
            Self::Bolt(job) => commands.spawn(job),
            Self::Fizzle(job) => commands.spawn(job),
            Self::TargetRing(job) => commands.spawn(job),
            Self::Beam(job) => commands.spawn(job),
            Self::Missile(job) => commands.spawn(job),
            Self::Bubble(job) => commands.spawn(job),
            Self::Pulse(job) => commands.spawn(job),
            Self::Impact(job) => commands.spawn(job),
            Self::Detonation(job) => commands.spawn(job),
        }
        .id()
    }
}

/// Was mit dem Schiff auf einem Slot geschieht. Animationen wartet das Skript ab.
#[derive(Clone, Copy, Debug)]
pub(super) enum ShipAction {
    /// HP und Schild auf die Werte nach dem Ereignis.
    Stats(ShipStats),
    /// Die Hülle blitzt in dieser Stärke auf.
    Flash(f32),
    /// Gedämpfte Balken: Das Schiff feuert diese Runde nicht.
    HoldFire,
    /// Eine Marke fällt auf das Schiff.
    Mark(MarkerSpec),
    Unmark(MarkerKind),
    /// Flug in einen anderen Slot (auch denselben: eine gestartete Drohne fliegt von ihrem Träger hinein).
    Move {
        to: SlotRef,
        towed: bool,
    },
    PhaseOut,
    /// Rammkurs: Anflug auf einen Weltpunkt; `Withdraw` holt es zurück.
    Charge(Vec3),
    Withdraw,
    /// Kurzsprung: Das (schon geschrumpfte) Schiff verlässt das Brett, bis [`Cue::Rejoin`] es zurückholt.
    Jump,
    Destroy,
    /// Das Trugbild löst sich auf und verschwindet.
    Vanish,
    /// Ein Traktorstrahl von diesem Weltpunkt hält das Schiff, bis [`Cue::Release`] ihn freigibt.
    Tether {
        from: Vec3,
        color: Color,
    },
}

/// Ein Bestandteil eines Takts.
pub(super) enum Cue {
    Log(Vec<String>),
    Banner(ShowBanner),
    Say(RadioCue),
    Sound(PlaySound),
    /// Der Klang „geht nicht“ der Oberfläche.
    Denied,
    Shake(f32),
    /// Der Regler der Animationsgeschwindigkeit für alles, was ab hier beginnt.
    Scope(AnimationScope),
    /// Schwebetext über dem Weltpunkt `at`.
    Float {
        at: Vec3,
        text: String,
        tone: FloatTone,
    },
    /// Eine Pause, die das Skript abwartet.
    Pause(f32),
    Effect {
        effect: Effect,
        awaited: bool,
    },
    /// Unterskripte laufen gleichzeitig; das Skript wartet auf alle.
    Fork(Vec<Script>),
    Ship {
        slot: SlotRef,
        action: ShipAction,
    },
    /// Deploy (auch eine Drohne der Drohnenstaffel): Das Schiff fällt im Slot aus dem Hyperraum.
    Arrive {
        slot: SlotRef,
        ship: ShipId,
    },
    /// Das Trugbild baut sich im Slot auf.
    Materialize {
        slot: SlotRef,
    },
    /// Eine Drohne startet am Träger auf `source`; ein [`ShipAction::Move`] auf `slot` fliegt sie hinein.
    Launch {
        source: SlotRef,
        slot: SlotRef,
    },
    /// Phasentausch, zweite Hälfte: Die geschrumpften Schiffe tauschen die Slots und wachsen wieder.
    Swap {
        a: SlotRef,
        b: SlotRef,
    },
    /// Rückkehr aus dem Kurzsprung mit den Werten danach.
    Rejoin {
        slot: SlotRef,
        stats: ShipStats,
    },
    /// Gibt den Traktorstrahl dieses Skripts frei und wartet, bis er verblasst ist.
    Release,
}

impl Cue {
    pub fn effect(effect: Effect) -> Self {
        Self::Effect {
            effect,
            awaited: true,
        }
    }

    /// Ein Effekt, der für sich weiterläuft.
    pub fn detached(effect: Effect) -> Self {
        Self::Effect {
            effect,
            awaited: false,
        }
    }

    pub fn ship(slot: SlotRef, action: ShipAction) -> Self {
        Self::Ship { slot, action }
    }

    pub fn sound(id: SoundId, at: Vec3) -> Self {
        Self::Sound(PlaySound::new(id).at(at))
    }

    /// Nur Bild und Ton, ohne Folgen für den Ablauf; beim Überspringen fällt es weg.
    fn is_presentation(&self) -> bool {
        matches!(self, Self::Banner(_) | Self::Float { .. } | Self::Say(_))
    }
}

/// Was die Takte zum Spielen brauchen.
#[derive(SystemParam)]
pub(super) struct Stage<'w, 's> {
    commands: Commands<'w, 's>,
    ships: Query<'w, 's, (Entity, &'static Ship, &'static OnSlot)>,
    statuses: Query<'w, 's, (&'static mut ShipStatus, Option<&'static mut ShipMarkers>)>,
    board: Res<'w, BoardEntities>,
    models: Res<'w, ShipModels>,
    playback: ResMut<'w, Playback>,
    clock: ResMut<'w, AnimationClock>,
    logs: MessageWriter<'w, AppendLog>,
    banners: MessageWriter<'w, ShowBanner>,
    radio: MessageWriter<'w, Say>,
    sounds: MessageWriter<'w, PlaySound>,
    ui_sounds: MessageWriter<'w, PlayUiSound>,
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
    /// Das Schiff auf einem Slot (kein Geist).
    fn ship_at(&self, slot: SlotRef) -> Option<Entity> {
        self.ships
            .iter()
            .find(|(_, ship, on)| !ship.ghost && on.0 == slot)
            .map(|(entity, ..)| entity)
    }

    /// Spawnt einen Auftrag, auf den das Skript wartet.
    fn await_job(&mut self, script: Entity, job: impl Bundle) {
        self.commands.spawn((job, AwaitedBy(script)));
    }

    /// Ein neues Schiff mit vollen Werten; `place` stellt es danach auf seinen Slot.
    fn new_ship(&mut self, ship: ShipId) -> Entity {
        let entity = spawn_ship(&mut self.commands, &self.models, ship, false);
        self.commands.entity(entity).insert(ShipStatus::new(ship));
        entity
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
            Cue::Say(cue) => {
                self.radio.write(Say::new(cue));
            }
            Cue::Sound(sound) => {
                self.sounds.write(sound);
            }
            Cue::Denied => {
                self.ui_sounds.write(PlayUiSound(UiSound::Denied));
            }
            Cue::Shake(strength) => {
                self.camera.write(CameraCommand::Shake(strength));
            }
            Cue::Scope(scope) => self.clock.scope = scope,
            Cue::Float { at, text, tone } => {
                self.commands.spawn(FloatingText { at, text, tone });
            }
            Cue::Pause(seconds) => self.await_job(entity, Animation::new(seconds)),
            Cue::Effect { effect, awaited } => {
                let job = effect.spawn(&mut self.commands);
                if awaited {
                    self.commands.entity(job).insert(AwaitedBy(entity));
                }
            }
            Cue::Fork(scripts) => {
                for child in scripts {
                    self.await_job(entity, child);
                }
            }
            Cue::Ship { slot, action } => {
                if let Some(ship) = self.ship_at(slot) {
                    self.act(entity, script, ship, action);
                }
            }
            Cue::Arrive { slot, ship } => {
                let ship = self.new_ship(ship);
                place(&mut self.commands, &self.board, ship, slot);
                self.await_job(entity, Arrive { ship });
            }
            Cue::Materialize { slot } => {
                let ship = self.new_ship(ShipId::Decoy);
                place(&mut self.commands, &self.board, ship, slot);
                self.await_job(entity, Materialize { ship });
            }
            Cue::Launch { source, slot } => {
                // Am Anker des Trägers, schon mit seinem Slot; der Flug hängt sie im nächsten Bild um.
                let drone = self.new_ship(ShipId::Drone);
                self.commands.entity(drone).insert((
                    OnSlot(slot),
                    ChildOf(self.board.anchor(source)),
                    Transform::from_scale(Vec3::splat(BOARD_SCALE)),
                ));
            }
            Cue::Swap { a, b } => {
                let (Some(first), Some(second)) = (self.ship_at(a), self.ship_at(b)) else {
                    return;
                };
                for (ship, to) in [(first, b), (second, a)] {
                    self.commands.entity(ship).insert((
                        OnSlot(to),
                        ChildOf(self.board.anchor(to)),
                        Transform::from_scale(Vec3::splat(BOARD_SCALE * 0.01)),
                    ));
                    self.await_job(entity, PhaseIn { ship });
                }
            }
            Cue::Rejoin { slot, stats } => {
                let Some(ship) = self.playback.jumped.remove(&slot) else {
                    return;
                };
                self.commands
                    .entity(ship)
                    .insert((OnSlot(slot), Visibility::Inherited));
                self.set_stats(ship, stats);
                self.await_job(entity, PhaseIn { ship });
            }
            Cue::Release => {
                if let Some(tether) = script.tether.take() {
                    self.commands.trigger(ReleaseTether { entity: tether });
                    self.commands.entity(tether).insert(AwaitedBy(entity));
                }
            }
        }
    }

    fn set_stats(&mut self, ship: Entity, stats: ShipStats) {
        if let Ok((mut status, _)) = self.statuses.get_mut(ship) {
            status.hp = stats.hp;
            status.shield = stats.shield;
        }
    }

    fn act(&mut self, entity: Entity, script: &mut Script, ship: Entity, action: ShipAction) {
        match action {
            ShipAction::Stats(stats) => self.set_stats(ship, stats),
            ShipAction::Flash(strength) => self.await_job(entity, HitFlash { ship, strength }),
            ShipAction::HoldFire => {
                if let Ok((mut status, _)) = self.statuses.get_mut(ship) {
                    status.holding_fire = true;
                }
            }
            ShipAction::Mark(spec) => self.await_job(entity, DropMarker { ship, spec }),
            ShipAction::Unmark(kind) => {
                if let Ok((_, Some(mut markers))) = self.statuses.get_mut(ship) {
                    markers.remove(kind);
                }
            }
            ShipAction::Move { to, towed } => self.await_job(entity, MoveShip { ship, to, towed }),
            ShipAction::PhaseOut => self.await_job(entity, PhaseOut { ship }),
            ShipAction::Charge(point) => self.await_job(entity, Charge { ship, point }),
            ShipAction::Withdraw => self.await_job(entity, Withdraw { ship }),
            ShipAction::Jump => {
                let slot = self.ships.get(ship).map(|(.., on)| on.0).expect("Slot");
                self.commands
                    .entity(ship)
                    .remove::<OnSlot>()
                    .insert(Visibility::Hidden);
                self.playback.jumped.insert(slot, ship);
            }
            ShipAction::Destroy => self.await_job(entity, DestroyShip { ship }),
            ShipAction::Vanish => {
                self.commands.entity(ship).remove::<OnSlot>();
                self.await_job(entity, Dematerialize { ship });
            }
            ShipAction::Tether { from, color } => {
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
