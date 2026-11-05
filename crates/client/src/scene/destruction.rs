//! Die Zerstörung eines Schiffs in drei Akten (Port von `scene/destruction.ts`): Todeskampf (Zittern, Krängen,
//! Sekundärexplosionen aus der Hülle), Detonation (Kern, Druckwellen, Blitzlicht, Kamerawackeln) und das Wrack, dessen
//! glühende Bruchstücke auseinanderfliegen, brennen, abkühlen und verblassen. Jedes Schiff hat sein eigenes Profil;
//! Flaggschiffe gehen in der Farbe ihres Admirals unter, mit den größten Detonationen.
//!
//! [`DestroyShip`] ist ein Auftrag: Er meldet sein Ende, sobald die Detonation ihre Wucht entfaltet hat (wie das
//! Original, auf das die Rundenwiedergabe wartet). Die Trümmer fliegen und brennen für sich weiter und nehmen das
//! Schiff am Ende mit. Das Schiff verliert dabei seinen Slot ([`OnSlot`]).

use bevy::{
    ecs::entity::EntityHashSet,
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility},
    prelude::*,
};
use voidfleet_core::{
    admirals::{AdmiralId, FlagshipId},
    enemies::EnemyId,
};

use super::{
    animation::{Animation, AnimationSystems, random_between, random_in_cone},
    board::OnSlot,
    camera::CameraCommand,
    effects::{BlastStyle, Burn, Detonation, SecondaryExplosion},
    formation::Hull,
    motion::HitFlash,
    ship::{BreakApart, Ship, ShipModel, ToonState, WreckReady},
    status::{ShipBounds, ShipStatus},
    toon::ToonOutline,
};
use crate::{
    audio::{LoopingSound, SoundId},
    data::ship_models::ShipKind,
    palette::{self, admiral_color},
};

/// Wie ein Schiff untergeht.
#[derive(Clone, Copy, Debug)]
pub struct DestructionProfile {
    /// Farbe der freigesetzten Energie: Kern, Druckwellen, Entladungen.
    pub energy: Color,
    pub style: BlastStyle,
    /// Größe der Detonation; 1 passt zu einem Jäger.
    pub size: f32,
    /// Todeskampf: Dauer in Sekunden und Zahl der Risse, aus denen es dabei herausbricht.
    pub agony: f32,
    pub pops: usize,
    /// Wucht: Geschwindigkeit der Trümmer (Welteinheiten pro Sekunde) und Kamerawackeln (Welteinheiten).
    pub force: f32,
    pub shake: f32,
    /// Wie lange die Trümmer sichtbar bleiben, bevor sie verblassen (Sekunden).
    pub linger: f32,
}

const FIGHTER_ENERGY: Color = Color::srgb(1.0, 0.45, 0.15);
const CRUISER_ENERGY: Color = Color::srgb(0.3, 0.6, 1.0);
const PLASMA: Color = Color::srgb(0.6, 0.3, 1.0);
const CYAN: Color = Color::srgb(0.2, 0.9, 1.0);

impl DestructionProfile {
    pub fn of(kind: ShipKind) -> Self {
        use BlastStyle::{Energy, Fire};
        let profile = |energy, style, size, agony: f32, pops, force, shake, linger: f32| Self {
            energy,
            style,
            size,
            agony: agony / 1000.0,
            pops,
            force,
            shake,
            linger: linger / 1000.0,
        };
        let flagship = |admiral, style| {
            profile(
                admiral_color(admiral),
                style,
                2.5,
                1900.0,
                9,
                3.0,
                0.55,
                3100.0,
            )
        };
        match kind {
            ShipKind::Enemy(enemy) => match enemy {
                // Jäger: zu klein für einen langen Todeskampf, ein scharfer Knall reißt ihn auseinander.
                EnemyId::Raiders => profile(FIGHTER_ENERGY, Fire, 0.9, 350.0, 1, 5.0, 0.12, 1400.0),
                // Brander: Schrott, fällt schon vor der Detonation an mehreren Stellen auseinander.
                EnemyId::Fireship => {
                    profile(FIGHTER_ENERGY, Fire, 1.0, 650.0, 3, 4.0, 0.14, 1600.0)
                }
                // Torpedo-Staffel: kleiner Knall, die Lanzen fliegen davon.
                EnemyId::TorpedoWing => {
                    profile(FIGHTER_ENERGY, Fire, 0.9, 450.0, 2, 4.0, 0.1, 1400.0)
                }
                // Kreuzer: eine Kette von Explosionen läuft den Rumpf entlang.
                EnemyId::GuardCruiser => {
                    profile(CRUISER_ENERGY, Fire, 1.4, 1000.0, 4, 3.5, 0.22, 2000.0)
                }
                // Plasmakreuzer: Die Kondensatoren entladen sich, der Reaktor geht violett hoch.
                EnemyId::PlasmaCruiser => profile(PLASMA, Energy, 1.5, 800.0, 3, 4.5, 0.25, 1800.0),
                // Spotter: Die Sender entladen sich in Blitzen.
                EnemyId::Spotter => profile(CYAN, Energy, 1.0, 500.0, 2, 4.0, 0.12, 1500.0),
                // Belagerer: langer Todeskampf, dann die größte Detonation.
                EnemyId::Besieger => {
                    profile(CRUISER_ENERGY, Fire, 2.0, 1600.0, 7, 3.0, 0.4, 2600.0)
                }
                // Träger: Hangars brennen, dann reißt der Kern im Bug alles mit.
                EnemyId::Carrier => profile(PLASMA, Energy, 2.0, 1500.0, 6, 3.0, 0.38, 2600.0),
                // Schrottkönig: geflickt, bricht an allen Nähten.
                EnemyId::ScrapKing => {
                    profile(FIGHTER_ENERGY, Fire, 2.5, 1900.0, 9, 3.0, 0.55, 3100.0)
                }
                // Wächter: Die Station entlädt ihr Hauptgeschütz.
                EnemyId::Warden => profile(CYAN, Energy, 2.6, 2100.0, 9, 3.0, 0.6, 3200.0),
                // Siegel: das gegnerische Flaggschiff.
                EnemyId::Seal => profile(palette::ENEMY, Energy, 2.3, 1700.0, 8, 3.5, 0.5, 2900.0),
            },
            ShipKind::Flagship(flagship_id) => match flagship_id {
                FlagshipId::Bastion => flagship(AdmiralId::Osei, Fire),
                FlagshipId::Vanguard => flagship(AdmiralId::Vance, Fire),
                FlagshipId::Pendulum => flagship(AdmiralId::Kerr, Energy),
                FlagshipId::Pyre => flagship(AdmiralId::Reyes, Fire),
            },
        }
    }
}

/// Sekundär- und Nachexplosionen sind kleiner als die Detonation; die Hülle blitzt dabei nur leicht.
const POP_SIZE: f32 = 0.9;
const POP_FLASH: f32 = 0.2;
/// Bis der Kern der Detonation verglüht ist (Ende des Auftrags nach dem Todeskampf).
const DETONATION_CORE: f32 = 0.38;
/// Ausblenden der Trümmer am Ende.
const FADE: f32 = 0.7;
/// Trümmer: Sinken und Abbremsen (Anteil pro Sekunde).
const GRAVITY: f32 = 1.1;
const DRAG: f32 = 0.45;
/// Sekunden, bis die Bruchkanten dunkel und Düsen, Fenster und Kennzeichen erloschen sind.
const COOL_TIME: f32 = 1.6;
const LIGHTS_OUT: f32 = 1.1;
/// Glut der Bruchkanten (linear, mal Glutmuster): weißglühend, dann orange, zuletzt dunkelrot.
const WHITE_HOT: Vec3 = Vec3::new(1.0, 0.72, 0.4);
const COLD: Vec3 = Vec3::new(0.7, 0.06, 0.0);
const EMBERS: f32 = 0.4;
const HEAT_BOOST: f32 = 10.0;
/// Die ganze Hülle wird vom Feuerball angestrahlt und verliert das Licht in dieser Zeit.
const GLARE: Vec3 = Vec3::new(1.0, 0.6, 0.3);
const GLARE_TIME: f32 = 0.5;
/// Zum Schluss verdunkeln die Stücke und schrumpfen.
const SHRINK: f32 = 0.6;
/// Todeskampf: Zittern (Raum des Schiffs), Absacken und Krängen bei voller Stärke.
const DISTRESS_JITTER: f32 = 0.07;
const DISTRESS_SINK: f32 = 0.3;
const DISTRESS_PITCH: f32 = 7.0 * std::f32::consts::PI / 180.0;
const DISTRESS_ROLL: f32 = 22.0 * std::f32::consts::PI / 180.0;
/// Hüllenpunkte: Strahlen aus diesem Kegel um „oben“, Abstand zur Hülle.
const SURFACE_CONE: f32 = 75.0 * std::f32::consts::PI / 180.0;
const SURFACE_CLEARANCE: f32 = 0.05;

/// Auftrag: Zerstört ein Schiff.
#[derive(Component, Clone, Copy, Debug)]
pub struct DestroyShip {
    pub ship: Entity,
}

/// Der laufende Todeskampf.
#[derive(Component, Clone, Debug)]
struct Agony {
    profile: DestructionProfile,
    /// Risse in der Hülle (Welt) und wann sie aufbrechen (Anteil des Todeskampfs).
    pops: Vec<(Vec3, f32)>,
    next: usize,
    /// Zu welcher Seite das Schiff krängt.
    list_side: f32,
    detonated: bool,
    /// Lage des Modells zuletzt im Todeskampf; das Wrack übernimmt sie.
    pose: Transform,
}

/// Das Schiff wartet auf sein Wrack.
#[derive(Component, Clone, Copy, Debug)]
struct AwaitingWreck {
    profile: DestructionProfile,
    center: Vec3,
    pose: Transform,
    /// Wie viele Modelle der Gruppe schon zerfallen.
    scattered: usize,
}

/// Ein fliegendes Bruchstück: Geschwindigkeit im Raum des Wracks, Taumelachse und Drehrate (rad/s).
#[derive(Clone, Copy, Debug)]
struct Body {
    piece: Entity,
    velocity: Vec3,
    axis: Vec3,
    spin: f32,
    scale: Vec3,
}

/// Die Trümmer eines Schiffs fliegen; am Ende verschwindet das Schiff.
#[derive(Component, Clone, Debug)]
struct Scatter {
    ship: Entity,
    profile: DestructionProfile,
    bodies: Vec<Body>,
    /// Nachexplosionen: Stück und Zeitpunkt (Sekunden).
    aftershocks: Vec<(Entity, f32)>,
}

pub(super) struct DestructionPlugin;

impl Plugin for DestructionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(begin)
            .add_observer(scatter_wreck)
            .add_systems(
                Update,
                (agonize, fly_pieces).in_set(AnimationSystems::Apply),
            );
    }
}

/// Diagonale eines Meshes in seinem Raum; die Wrack-Meshes leben nur in der Render-Welt, dann gilt die beim Übergeben
/// gemerkte Box.
fn mesh_extent(mesh: &Mesh) -> Option<f32> {
    match mesh.try_attribute_option(Mesh::ATTRIBUTE_POSITION) {
        Ok(Some(positions)) => {
            let positions = positions.as_float3()?;
            let (min, max) = positions.iter().fold(
                (Vec3::INFINITY, Vec3::NEG_INFINITY),
                |(min, max), position| {
                    let position = Vec3::from_array(*position);
                    (min.min(position), max.max(position))
                },
            );
            (!positions.is_empty()).then(|| (max - min).length())
        }
        _ => mesh.final_aabb.map(|aabb| (aabb.max - aabb.min).length()),
    }
}

/// Todeskampf beginnt: Balken weg, die Hülle ächzt (umso lauter, je länger er dauert), Risse werden ausgewählt.
fn begin(
    add: On<Add<DestroyShip>>,
    jobs: Query<&DestroyShip>,
    mut ships: Query<(
        &Ship,
        &GlobalTransform,
        Option<&ShipBounds>,
        &mut ShipStatus,
        &Children,
    )>,
    models: Query<(), With<ShipModel>>,
    hulls: Query<(&Visibility, &GlobalTransform), With<Hull>>,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<ToonOutline>)>,
    mut ray_cast: MeshRayCast,
    mut commands: Commands,
) {
    let job = *jobs.get(add.entity).expect("Zerstörung");
    let Ok((ship, global, bounds, mut status, ship_children)) = ships.get_mut(job.ship) else {
        return;
    };
    let profile = DestructionProfile::of(ship.kind);
    status.hidden = true;
    commands.entity(job.ship).remove::<OnSlot>();
    let center = bounds.map_or(global.translation(), |bounds| bounds.world_center(global));
    let radius = bounds.map_or(1.0, ShipBounds::world_radius);

    // Hüllenpunkte per Strahl von außen, weil die Modelle aus überlappenden Teilen bestehen; genommen wird der erste
    // Treffer auf einem stehenden Modell. In einer Gruppe zielen die Strahlen reihum auf jedes Modell, so bricht jedes
    // für sich auf.
    let standing: Vec<(Entity, Vec3)> = ship_children
        .iter()
        .filter(|child| models.contains(*child))
        .flat_map(|group| children.iter_descendants(group))
        .filter_map(|member| {
            let (visibility, global) = hulls.get(member).ok()?;
            (*visibility != Visibility::Hidden).then_some((member, global.translation()))
        })
        .collect();
    let hull: EntityHashSet = standing
        .iter()
        .flat_map(|(member, _)| children.iter_descendants(*member))
        .filter(|part| meshes.contains(*part))
        .collect();
    let aims: Vec<Vec3> = if standing.len() > 1 {
        standing.iter().map(|(_, at)| *at).collect()
    } else {
        vec![center]
    };
    let pop_count = profile.pops.max(aims.len());
    let filter = |entity: Entity| hull.contains(&entity);
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)
        .with_filter(&filter);
    let reach = radius + 1.0;
    let mut points = Vec::new();
    for attempt in 0..pop_count * 8 {
        if points.len() >= pop_count {
            break;
        }
        let aim = aims[(points.len() + attempt) % aims.len()];
        let direction = random_in_cone(Vec3::Y, SURFACE_CONE);
        let ray = Ray3d::new(
            aim + direction * reach,
            Dir3::new(-direction).expect("Richtung"),
        );
        if let Some((_, hit)) = ray_cast.cast_ray(ray, &settings).first() {
            points.push(hit.point + direction * SURFACE_CLEARANCE);
        }
    }
    let mut schedule: Vec<f32> = points
        .iter()
        .map(|_| 0.12 + 0.8 * rand::random::<f32>().powf(0.7))
        .collect();
    schedule.sort_by(f32::total_cmp);

    commands.entity(add.entity).insert((
        Name::new("Zerstörung"),
        Agony {
            profile,
            pops: points.into_iter().zip(schedule).collect(),
            next: 0,
            list_side: if rand::random::<bool>() { 1.0 } else { -1.0 },
            detonated: false,
            pose: Transform::default(),
        },
        Transform::from_translation(center),
        LoopingSound {
            volume: (profile.agony / 1.6).min(1.0),
            ..LoopingSound::new(SoundId::HullStress)
        },
        Animation::new(profile.agony + DETONATION_CORE),
    ));
}

/// Das Schiff zittert immer stärker, sackt ab und krängt, Risse brechen auf; dann die Detonation.
fn agonize(
    mut commands: Commands,
    mut jobs: Query<(Entity, &DestroyShip, &mut Agony, &Animation)>,
    ships: Query<(&GlobalTransform, Option<&ShipBounds>, &Children), With<Ship>>,
    mut models: Query<&mut Transform, With<ShipModel>>,
    mut camera: MessageWriter<CameraCommand>,
) {
    for (job, destroy, mut agony, animation) in &mut jobs {
        let Ok((global, bounds, children)) = ships.get(destroy.ship) else {
            continue;
        };
        let profile = agony.profile;
        let center = bounds.map_or(global.translation(), |bounds| bounds.world_center(global));
        if !agony.detonated && animation.seconds() < profile.agony {
            let amount = animation.seconds() / profile.agony.max(1e-3);
            let jitter = DISTRESS_JITTER * (0.2 + 0.8 * amount * amount);
            let shake = || (rand::random::<f32>() - 0.5) * jitter;
            let pose =
                Transform::from_xyz(shake(), -DISTRESS_SINK * amount * amount + shake(), shake())
                    .with_rotation(Quat::from_euler(
                        EulerRot::YXZ,
                        0.0,
                        -DISTRESS_PITCH * amount * amount,
                        agony.list_side * DISTRESS_ROLL * amount.powi(3),
                    ));
            for child in children.iter() {
                if let Ok(mut model) = models.get_mut(child) {
                    model.translation = pose.translation;
                    model.rotation = pose.rotation;
                }
            }
            agony.pose = pose;
            while agony.next < agony.pops.len() && agony.pops[agony.next].1 <= amount {
                let point = agony.pops[agony.next].0;
                agony.next += 1;
                commands.spawn(SecondaryExplosion {
                    position: point,
                    outward: (point - center).normalize_or(Vec3::Y),
                    size: POP_SIZE * profile.size.powf(0.75),
                });
                camera.write(CameraCommand::Shake(profile.shake * 0.25));
                commands.spawn(HitFlash {
                    strength: POP_FLASH,
                    ..HitFlash::new(destroy.ship)
                });
            }
        } else if !agony.detonated {
            agony.detonated = true;
            // Die Detonation schneidet das Ächzen ab.
            commands.entity(job).remove::<LoopingSound>();
            commands.entity(destroy.ship).insert(AwaitingWreck {
                profile,
                center,
                pose: agony.pose,
                scattered: 0,
            });
            commands.trigger(BreakApart {
                entity: destroy.ship,
            });
            camera.write(CameraCommand::Shake(profile.shake));
            commands.spawn(Detonation {
                position: center,
                energy: profile.energy,
                size: profile.size,
                style: profile.style,
            });
        }
    }
}

/// Die Bruchstücke stehen: Flugbahnen radial vom Explosionsherd, kleine Stücke schneller, jedes mit eigenem Taumeln;
/// die größten brennen.
fn scatter_wreck(
    ready: On<WreckReady>,
    mut commands: Commands,
    mut ships: Query<(&mut AwaitingWreck, &GlobalTransform, &Children)>,
    mut models: Query<&mut Transform, With<ShipModel>>,
    nodes: Query<(&Transform, &ChildOf), Without<ShipModel>>,
    children: Query<&Children>,
    parts: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
) {
    let Ok((mut awaiting, global, ship_children)) = ships.get_mut(ready.entity) else {
        return;
    };
    awaiting.scattered += 1;
    if awaiting.scattered >= ready.hulls {
        commands.entity(ready.entity).remove::<AwaitingWreck>();
    }
    let Some(&first) = ready.pieces.first() else {
        return;
    };
    let profile = awaiting.profile;
    // Das Wrack übernimmt die Lage der Gruppe im Todeskampf; die Stücke liegen unter dem Knoten der glTF-Szene, die
    // unter ihrem Modell der Gruppe.
    let mut model_transform = Transform::default();
    for child in ship_children.iter() {
        if let Ok(mut model) = models.get_mut(child) {
            model.translation = awaiting.pose.translation;
            model.rotation = awaiting.pose.rotation;
            model_transform = *model;
        }
    }
    let scene_transform = nodes
        .get(first)
        .ok()
        .and_then(|(_, child_of)| nodes.get(child_of.parent()).ok())
        .map_or(Transform::default(), |(transform, _)| *transform);
    let hull_transform = nodes
        .get(ready.hull)
        .map_or(Transform::default(), |(transform, _)| *transform);
    let to_world = global.affine()
        * model_transform.compute_affine()
        * hull_transform.compute_affine()
        * scene_transform.compute_affine();
    let world_per_local = to_world.transform_vector3(Vec3::X).length();
    // Jedes Modell einer Gruppe fällt um seine eigene Mitte auseinander, etwas daneben, damit es nicht symmetrisch
    // zerfällt.
    let center = if ready.hulls > 1 {
        scene_transform
            .compute_affine()
            .inverse()
            .transform_point3(Vec3::ZERO)
    } else {
        to_world.inverse().transform_point3(awaiting.center)
    };
    let origin = center + random_in_cone(Vec3::Y, std::f32::consts::PI) * 0.6;

    let mut bodies: Vec<(Body, f32)> = ready
        .pieces
        .iter()
        .filter_map(|&piece| {
            let (transform, _) = nodes.get(piece).ok()?;
            let extent = children
                .iter_descendants(piece)
                .chain([piece])
                .filter_map(|part| parts.get(part).ok())
                .filter_map(|mesh| meshes.get(&mesh.0).and_then(mesh_extent))
                .fold(0.0, f32::max)
                * world_per_local
                * transform.scale.max_element();
            let mass = (extent / 0.8).clamp(0.5, 2.5);
            let mut direction = transform.translation - origin;
            if direction.length() < 0.01 {
                direction = random_in_cone(Vec3::Y, std::f32::consts::PI);
            }
            let direction = (direction.normalize()
                + random_in_cone(Vec3::Y, std::f32::consts::PI) * 0.35)
                .normalize();
            let speed = profile.force * random_between((0.7, 1.3)) / mass.sqrt();
            Some((
                Body {
                    piece,
                    velocity: direction * speed / world_per_local,
                    axis: random_in_cone(Vec3::Y, std::f32::consts::PI),
                    spin: random_between((1.5, 4.5)) / mass.sqrt(),
                    scale: transform.scale,
                },
                extent,
            ))
        })
        .collect();
    bodies.sort_by(|a, b| b.1.total_cmp(&a.1));
    let share = ready.hulls.max(1) as f32;
    let burning = (profile.size * 1.5 / share).ceil() as usize;
    let aftershocks = (profile.size / share).round() as usize;
    for (body, extent) in bodies.iter().take(burning) {
        commands.spawn(Burn {
            piece: body.piece,
            size: 0.3 + 0.3 * extent,
            spread: extent * 0.35,
            duration: profile.linger,
        });
    }
    let aftershocks = bodies
        .iter()
        .take(burning.min(aftershocks))
        .map(|(body, _)| (body.piece, random_between((0.2, 0.7)) * profile.linger))
        .collect();
    commands.spawn((
        Name::new("Trümmer"),
        Scatter {
            ship: ready.entity,
            profile,
            bodies: bodies.into_iter().map(|(body, _)| body).collect(),
            aftershocks,
        },
        Animation::new(profile.linger + FADE),
    ));
}

/// Trümmer fliegen und taumeln, Bruchkanten kühlen ab, Lichter erlöschen flackernd, zuletzt verblasst alles.
fn fly_pieces(
    mut commands: Commands,
    mut scatters: Query<(&mut Scatter, &Animation)>,
    mut pieces: Query<(&mut Transform, &GlobalTransform), Without<Ship>>,
    mut ships: Query<&mut ToonState>,
    mut camera: MessageWriter<CameraCommand>,
) {
    for (mut scatter, animation) in &mut scatters {
        let t = animation.seconds();
        let dt = animation.delta();
        let profile = scatter.profile;
        for body in &mut scatter.bodies {
            body.velocity.y -= GRAVITY * dt;
            body.velocity *= (-DRAG * dt).exp();
            if let Ok((mut transform, _)) = pieces.get_mut(body.piece) {
                transform.translation += body.velocity * dt;
                transform
                    .rotate_local_axis(Dir3::new(body.axis).unwrap_or(Dir3::Y), body.spin * dt);
            }
        }
        let due: Vec<Entity> = scatter
            .aftershocks
            .iter()
            .filter(|(_, at)| *at <= t)
            .map(|(piece, _)| *piece)
            .collect();
        scatter.aftershocks.retain(|(_, at)| *at > t);
        for piece in due {
            if let Ok((_, global)) = pieces.get(piece) {
                commands.spawn(SecondaryExplosion {
                    position: global.translation(),
                    outward: random_in_cone(Vec3::Y, std::f32::consts::PI),
                    size: POP_SIZE * 0.8 * profile.size.powf(0.75),
                });
                camera.write(CameraCommand::Shake(profile.shake * 0.3));
            }
        }
        let heat = (1.0 - t / COOL_TIME).max(0.0);
        let glow = COLD.lerp(WHITE_HOT, heat * heat) * (EMBERS + HEAT_BOOST * heat.powi(3));
        let glare = (1.0 - t / GLARE_TIME).max(0.0).powi(2);
        let fade = ((t - profile.linger) / FADE).clamp(0.0, 1.0);
        if let Ok(mut state) = ships.get_mut(scatter.ship) {
            state.heat = Some(LinearRgba::rgb(glow.x, glow.y, glow.z));
            state.lights = (1.0 - t / LIGHTS_OUT).max(0.0) * (0.5 + 0.5 * rand::random::<f32>());
            state.overlay = if fade > 0.0 {
                Vec3::ZERO.extend(fade)
            } else {
                GLARE.extend(0.75 * glare)
            };
        }
        if fade > 0.0 {
            for body in &scatter.bodies {
                if let Ok((mut transform, _)) = pieces.get_mut(body.piece) {
                    transform.scale = body.scale * (1.0 - SHRINK * fade);
                }
            }
        }
        // Eine Gruppe hat je Modell Trümmer; die ersten nehmen das Schiff mit.
        if animation.is_done() {
            commands.entity(scatter.ship).try_despawn();
        }
    }
}
