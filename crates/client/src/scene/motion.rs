//! Animationen eines Schiffs (Port aus `scene/ship.ts`), jede ein Auftrag mit Fertig-Rückmeldung
//! ([`crate::scene::animation`]):
//!
//! - [`Arrive`] — Deploy: Das Schiff fällt direkt im Slot aus dem Hyperraum, lang gezogen und hell glühend, schnappt
//!   auf seine Form zurück und gleitet bremsend nach vorn; Portal, Blitz und Lichtstreifen kommen aus
//!   [`crate::scene::effects::Warp`]. Balken und Marken erscheinen erst danach. Wartet, bis das Modell geladen ist.
//! - [`MoveShip`] — Slot-Wechsel (Manöver, Ausweichmanöver, Traktorstrahl): hängt das Schiff an den neuen Slot und
//!   fliegt es auf der Bahn aus [`crate::scene::maneuver`] hinein, mit Kurs, Steigung und Schräglage.
//! - [`Charge`], [`Withdraw`] — Rammkurs: Anflug auf einen Punkt und, wer den Aufprall übersteht, Rückflug.
//! - [`HitFlash`] — Treffer: Die Hülle blitzt hell auf; der Einschlag selbst ist [`crate::scene::effects::Impact`].
//!
//! Flüge laufen im Takt der Bewegungen ([`AnimationScope::Movement`]); das Triebwerk der Schiffsklasse klingt solange
//! am Schiff und verklingt nach der Landung.

use std::f32::consts::PI;

use bevy::prelude::*;
use voidfleet_core::enemies::EnemyClass;

use super::{
    animation::{Animation, AnimationScope, AnimationSystems},
    board::{BoardEntities, attach},
    coords::SlotRef,
    effects::Warp,
    maneuver::Flight,
    ship::{Ship, ShipModel, ToonState},
    status::{ShipBounds, ShipParts, ShipStatus},
};
use crate::audio::{LoopingSound, SoundId};

/// Deploy: so weit hinter dem Pad (Welteinheiten) fällt das Schiff aus dem Hyperraum; anfangs ist es um diesen Faktor
/// in die Länge gezogen und dafür schmaler, auf diesem Anteil der Flugzeit schnappt es zurück. Die Hülle glüht dabei hell.
const WARP_DURATION: f32 = 1.2;
const WARP_GLIDE: f32 = 1.4;
const WARP_STRETCH: f32 = 3.5;
const WARP_SQUEEZE: f32 = 0.65;
const WARP_SNAP: f32 = 0.3;
const WARP_GLOW_COLOR: Color = Color::srgb(0.75, 0.9, 1.0);
const WARP_GLOW_ALPHA: f32 = 0.9;
const WARP_GLOW_FADE: f32 = 0.55;
/// Treffer: Die Hülle leuchtet kurz auf und klingt wieder ab.
const HIT_FLASH_COLOR: Color = Color::srgb(1.0, 0.93, 0.8);
pub const HIT_FLASH_ALPHA: f32 = 0.75;
const HIT_FLASH_DURATION: f32 = 0.22;
/// Rammkurs: Anflug mit Beschleunigung und leichtem Bogen, Rückflug in den Slot.
const RAM_DURATION: f32 = 0.62;
const RAM_ARC: f32 = 0.5;
const RAM_PITCH: f32 = 8.0 * PI / 180.0;
const RAM_RETURN_DURATION: f32 = 0.9;

/// Deploy-Einflug eines Schiffs, das schon auf seinem Slot hängt.
#[derive(Component, Clone, Copy, Debug)]
pub struct Arrive {
    pub ship: Entity,
}

/// Slot-Wechsel nach `to`; `towed`: vom Traktorstrahl gezogen (ruhiger, ohne eigenes Triebwerk).
#[derive(Component, Clone, Copy, Debug)]
pub struct MoveShip {
    pub ship: Entity,
    pub to: SlotRef,
    pub towed: bool,
}

/// Rammkurs, Anflug: Das Schiff dreht die Nase auf `point` (Welt, auf Höhe des Ankers) und beschleunigt bis dorthin, in
/// einem flachen Bogen und mit gesenkter Nase. Es bleibt dort, bis [`Withdraw`] es zurückholt oder es fällt.
#[derive(Component, Clone, Copy, Debug)]
pub struct Charge {
    pub ship: Entity,
    pub point: Vec3,
}

/// Rammkurs, Rückkehr: Abgeprallt treibt das Schiff rückwärts in seinen Slot und richtet sich wieder aus.
#[derive(Component, Clone, Copy, Debug)]
pub struct Withdraw {
    pub ship: Entity,
}

/// Treffer: Die Hülle blitzt in `strength` (üblich [`HIT_FLASH_ALPHA`]) auf und klingt ab.
#[derive(Component, Clone, Copy, Debug)]
#[require(Animation::new(HIT_FLASH_DURATION))]
pub struct HitFlash {
    pub ship: Entity,
    pub strength: f32,
}

impl HitFlash {
    pub fn new(ship: Entity) -> Self {
        Self {
            ship,
            strength: HIT_FLASH_ALPHA,
        }
    }
}

/// Grundmaßstab des Modells beim Start eines Auftrags, der es verformt.
#[derive(Component, Clone, Copy, Debug)]
struct ModelBase {
    scale: Vec3,
    half_length: f32,
}

/// Die Bahn eines Slot-Wechsels.
#[derive(Component, Clone, Debug)]
struct FlightPath(Flight);

/// Rammkurs: Ziel im Raum des Ankers und Kurs dorthin.
#[derive(Component, Clone, Copy, Debug)]
struct RamTarget {
    target: Vec3,
    yaw: f32,
}

/// Rückflug: Lage und Kurs beim Start.
#[derive(Component, Clone, Copy, Debug)]
struct ReturnFrom {
    translation: Vec3,
    rotation: Quat,
}

/// Triebwerk je Schiffsklasse während eines Flugs.
fn engine(class: EnemyClass) -> SoundId {
    match class {
        EnemyClass::Fighter => SoundId::EngineLight,
        EnemyClass::Cruiser => SoundId::EngineMedium,
        EnemyClass::Battleship | EnemyClass::Flagship => SoundId::EngineHeavy,
    }
}

/// Das Modell unter einem Schiff.
fn model_of(
    children: &Children,
    models: &Query<&mut Transform, With<ShipModel>>,
) -> Option<Entity> {
    children.iter().find(|child| models.contains(*child))
}

pub(super) struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(start_move)
            .add_observer(start_charge)
            .add_observer(start_withdraw)
            .add_systems(
                Update,
                (
                    (begin_arrivals, begin_flights).before(AnimationSystems::Advance),
                    ((warp_in, fly, charge, withdraw, flash_hull), land)
                        .chain()
                        .in_set(AnimationSystems::Apply),
                ),
            );
    }
}

/// Der Deploy beginnt, sobald das Modell gemessen ist: Balken und Marken verschwinden, das Schiff rückt hinter das Pad,
/// das Portal öffnet sich.
fn begin_arrivals(
    mut commands: Commands,
    jobs: Query<(Entity, &Arrive), Without<Animation>>,
    mut ships: Query<
        (
            &Ship,
            &ShipBounds,
            &GlobalTransform,
            &Children,
            &mut ShipStatus,
            &mut Transform,
        ),
        Without<ShipModel>,
    >,
    models: Query<&mut Transform, With<ShipModel>>,
) {
    for (job, arrive) in &jobs {
        let Ok((ship, bounds, global, children, mut status, mut transform)) =
            ships.get_mut(arrive.ship)
        else {
            continue;
        };
        let Some(model) = model_of(children, &models).and_then(|model| models.get(model).ok())
        else {
            continue;
        };
        status.hidden = true;
        transform.translation.z = WARP_GLIDE;
        commands.spawn(Warp {
            position: bounds.world_center(global),
            forward: global.forward().as_vec3(),
            size: bounds.world_radius(),
        });
        commands.entity(job).insert((
            ModelBase {
                scale: model.scale,
                half_length: (bounds.max.z - bounds.min.z) / 2.0,
            },
            Animation::new(WARP_DURATION).in_scope(AnimationScope::Movement),
        ));
        commands
            .entity(arrive.ship)
            .insert(LoopingSound::new(engine(ship.kind.class())));
    }
}

fn warp_in(
    jobs: Query<(&Arrive, &ModelBase, &Animation)>,
    mut ships: Query<(&Children, &mut Transform, &mut ToonState), Without<ShipModel>>,
    mut models: Query<&mut Transform, With<ShipModel>>,
) {
    for (arrive, base, animation) in &jobs {
        let Ok((children, mut transform, mut state)) = ships.get_mut(arrive.ship) else {
            continue;
        };
        let progress = animation.progress();
        transform.translation.z =
            WARP_GLIDE * (1.0 - EaseFunction::CubicOut.sample_clamped(progress));
        let snap = EaseFunction::CubicOut.sample_clamped(progress / WARP_SNAP);
        let length = WARP_STRETCH + (1.0 - WARP_STRETCH) * snap;
        let width = WARP_SQUEEZE + (1.0 - WARP_SQUEEZE) * snap;
        if let Some(model) = model_of(children, &models)
            && let Ok(mut model) = models.get_mut(model)
        {
            model.scale = base.scale * Vec3::new(width, width, length);
            // Die Nase bleibt vorn, das Heck zieht sich nach.
            model.translation.z = base.half_length * (length - 1.0);
        }
        let glow = WARP_GLOW_ALPHA * (1.0 - (progress / WARP_GLOW_FADE).min(1.0)).powi(2);
        state.overlay = WARP_GLOW_COLOR.to_linear().to_vec3().extend(glow);
    }
}

/// Hängt das Schiff an den neuen Slot (es bleibt, wo es ist); die Bahn rechnet [`begin_flights`] ab dem nächsten Bild.
fn start_move(
    add: On<Add<MoveShip>>,
    jobs: Query<&MoveShip>,
    ships: Query<&ShipParts>,
    board: Res<BoardEntities>,
    mut commands: Commands,
) {
    let job = *jobs.get(add.entity).expect("Slot-Wechsel");
    attach(&mut commands, &board, job.ship, job.to);
    // Die Marken am Boden landen mit dem Schiff, statt mitzufliegen.
    if let Ok(parts) = ships.get(job.ship) {
        commands.entity(parts.floor).insert(Visibility::Hidden);
    }
}

fn begin_flights(
    mut commands: Commands,
    jobs: Query<(Entity, &MoveShip), Without<Animation>>,
    ships: Query<(&Ship, &Transform, &ChildOf)>,
    board: Res<BoardEntities>,
) {
    for (job, flight) in &jobs {
        let Ok((ship, transform, child_of)) = ships.get(flight.ship) else {
            continue;
        };
        // Erst wenn das Schiff am neuen Anker hängt, liegt seine Lage in dessen Raum.
        if child_of.parent() != board.anchor(flight.to) {
            continue;
        }
        let path = Flight::new(transform.translation, ship.kind.class(), flight.towed);
        commands.entity(job).insert((
            Animation::new(path.duration).in_scope(AnimationScope::Movement),
            FlightPath(path),
        ));
        if !flight.towed {
            commands
                .entity(flight.ship)
                .insert(LoopingSound::new(engine(ship.kind.class())));
        }
    }
}

fn fly(
    jobs: Query<(&MoveShip, &FlightPath, &Animation)>,
    mut ships: Query<(&Children, &mut Transform), Without<ShipModel>>,
    mut models: Query<&mut Transform, With<ShipModel>>,
) {
    for (job, path, animation) in &jobs {
        let Ok((children, mut transform)) = ships.get_mut(job.ship) else {
            continue;
        };
        let pose = path.0.pose(animation.progress());
        transform.translation = pose.translation;
        transform.rotation = pose.heading;
        if let Some(model) = model_of(children, &models)
            && let Ok(mut model) = models.get_mut(model)
        {
            model.rotation = pose.attitude;
        }
    }
}

fn start_charge(
    add: On<Add<Charge>>,
    jobs: Query<&Charge>,
    ships: Query<(&Ship, &ChildOf, &ShipParts)>,
    anchors: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    let job = *jobs.get(add.entity).expect("Rammkurs");
    let Ok((ship, child_of, parts)) = ships.get(job.ship) else {
        return;
    };
    let Ok(anchor) = anchors.get(child_of.parent()) else {
        return;
    };
    let target = anchor.affine().inverse().transform_point3(job.point);
    commands.entity(parts.floor).insert(Visibility::Hidden);
    commands.entity(add.entity).insert((
        RamTarget {
            target,
            yaw: (-target.x).atan2(-target.z),
        },
        Animation::new(RAM_DURATION).in_scope(AnimationScope::Movement),
    ));
    commands
        .entity(job.ship)
        .insert(LoopingSound::new(engine(ship.kind.class())));
}

fn charge(
    jobs: Query<(&Charge, &RamTarget, &Animation)>,
    mut ships: Query<(&Children, &mut Transform), Without<ShipModel>>,
    mut models: Query<&mut Transform, With<ShipModel>>,
) {
    for (job, ram, animation) in &jobs {
        let Ok((children, mut transform)) = ships.get_mut(job.ship) else {
            continue;
        };
        let progress = animation.progress();
        let travelled = EaseFunction::CubicIn.sample_clamped(progress);
        let arc = (PI * travelled).sin();
        transform.translation = Vec3::new(
            ram.target.x * travelled,
            RAM_ARC * arc,
            ram.target.z * travelled,
        );
        transform.rotation =
            Quat::from_rotation_y(ram.yaw * EaseFunction::CubicOut.sample_clamped(progress * 2.5));
        if let Some(model) = model_of(children, &models)
            && let Ok(mut model) = models.get_mut(model)
        {
            // Nase gesenkt.
            model.rotation = Quat::from_rotation_x(-RAM_PITCH * arc);
        }
    }
}

fn start_withdraw(
    add: On<Add<Withdraw>>,
    jobs: Query<&Withdraw>,
    ships: Query<(&Ship, &Transform)>,
    mut commands: Commands,
) {
    let job = *jobs.get(add.entity).expect("Rückflug");
    let Ok((ship, transform)) = ships.get(job.ship) else {
        return;
    };
    commands.entity(add.entity).insert((
        ReturnFrom {
            translation: transform.translation,
            rotation: transform.rotation,
        },
        Animation::new(RAM_RETURN_DURATION).in_scope(AnimationScope::Movement),
    ));
    commands
        .entity(job.ship)
        .insert(LoopingSound::new(engine(ship.kind.class())));
}

fn withdraw(
    jobs: Query<(&Withdraw, &ReturnFrom, &Animation)>,
    mut ships: Query<&mut Transform, With<Ship>>,
) {
    for (job, from, animation) in &jobs {
        if let Ok(mut transform) = ships.get_mut(job.ship) {
            let back = EaseFunction::SineInOut.sample_clamped(animation.progress());
            transform.translation = from.translation.lerp(Vec3::ZERO, back);
            transform.rotation = from.rotation.slerp(Quat::IDENTITY, back);
        }
    }
}

fn flash_hull(jobs: Query<(&HitFlash, &Animation)>, mut ships: Query<&mut ToonState>) {
    for (job, animation) in &jobs {
        if let Ok(mut state) = ships.get_mut(job.ship) {
            let alpha = job.strength * (1.0 - animation.progress()).powi(2);
            state.overlay = HIT_FLASH_COLOR.to_linear().to_vec3().extend(alpha);
        }
    }
}

/// Am Ende eines Flugs steht das Schiff genau im Slot, Modell und Boden wie vorher; das Triebwerk verklingt.
fn land(
    mut commands: Commands,
    arrivals: Query<(&Arrive, &ModelBase, &Animation)>,
    flights: Query<(&MoveShip, &Animation)>,
    withdrawals: Query<(&Withdraw, &Animation)>,
    charges: Query<(&Charge, &Animation)>,
    mut ships: Query<
        (
            &Children,
            &ShipParts,
            &mut Transform,
            &mut ShipStatus,
            &mut ToonState,
        ),
        Without<ShipModel>,
    >,
    mut models: Query<&mut Transform, With<ShipModel>>,
) {
    let landed = arrivals
        .iter()
        .filter(|(_, _, animation)| animation.is_done())
        .map(|(job, base, _)| (job.ship, Some(*base), true))
        .chain(
            flights
                .iter()
                .filter(|(_, animation)| animation.is_done())
                .map(|(job, _)| (job.ship, None, true)),
        )
        .chain(
            withdrawals
                .iter()
                .filter(|(_, animation)| animation.is_done())
                .map(|(job, _)| (job.ship, None, true)),
        )
        .chain(
            charges
                .iter()
                .filter(|(_, animation)| animation.is_done())
                .map(|(job, _)| (job.ship, None, false)),
        );
    for (ship, base, home) in landed {
        commands.entity(ship).remove::<LoopingSound>();
        let Ok((children, parts, mut transform, mut status, mut state)) = ships.get_mut(ship)
        else {
            continue;
        };
        if !home {
            continue;
        }
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        status.hidden = false;
        state.overlay = Vec4::ZERO;
        commands.entity(parts.floor).insert(Visibility::Inherited);
        if let Some(model) = model_of(children, &models)
            && let Ok(mut model) = models.get_mut(model)
        {
            model.rotation = Quat::IDENTITY;
            model.translation = Vec3::ZERO;
            if let Some(base) = base {
                model.scale = base.scale;
            }
        }
    }
}
