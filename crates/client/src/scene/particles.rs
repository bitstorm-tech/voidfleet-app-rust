//! Partikel der Effekte (Ersatz für Babylons `ParticleSystem`): ein Schwarm je Entity, gerechnet auf der CPU und jedes
//! Bild als ein Mesh aus zur Kamera gedrehten, in Flugrichtung gestreckten Quadraten neu geschrieben. Funken und Feuer
//! leuchten additiv, Rauch mischt sich nach Alpha. Ein Schwarm geht auf einmal los ([`Burst`]) oder strömt aus einem
//! Emitter, der einem Punkt oder einer Entity folgt ([`Emitter`]); er verschwindet, sobald das letzte Partikel
//! verglüht ist und nichts mehr nachströmt. Partikel altern im Takt der Animationsgeschwindigkeit.

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::{
    animation::{random_between, random_in_cone},
    camera::MainCamera,
    fx::{FxAssets, FxBlend, FxMaterial, GLOW},
    tween::{AnimationClock, AnimationScope},
};
use crate::settings::Settings;

/// Ganze Kugel als Kegel.
pub const SPHERE: f32 = PI;

/// Farbverlauf über die Lebensdauer (0..1): sRGB-Farbe und Deckkraft, wahlweise aus der Schussfarbe abgeleitet.
pub type Gradient = fn(Color) -> Vec<(f32, Srgba)>;

/// Ein Schwarm Partikel.
#[derive(Clone, Copy, Debug)]
pub struct Burst {
    pub count: u32,
    /// Halber Öffnungswinkel des Kegels um die Achse.
    pub cone: f32,
    /// Durch das Schiff hindurch: Die Achse zeigt vom Angreifer weg.
    pub through: bool,
    /// Halbe Kantenlänge des Kastens, aus dem die Partikel kommen (Welteinheiten).
    pub spread: f32,
    /// Welteinheiten pro Sekunde und Lebensdauer in Sekunden, jeweils min und max.
    pub speed: (f32, f32),
    pub life: (f32, f32),
    /// Startgröße (min, max) und Faktor am Lebensende.
    pub size: (f32, f32),
    pub grow: f32,
    /// Streckung entlang der Flugrichtung; 1 bleibt rund.
    pub stretch: f32,
    pub gravity: f32,
    /// Alpha-Mischung statt additivem Leuchten, für Rauch.
    pub opaque: bool,
    pub colors: Gradient,
}

impl Burst {
    /// Grundwerte, die die Varianten überschreiben.
    pub const DEFAULT: Self = Self {
        count: 0,
        cone: SPHERE,
        through: false,
        spread: 0.06,
        speed: (1.0, 1.0),
        life: (1.0, 1.0),
        size: (0.1, 0.1),
        grow: 0.3,
        stretch: 1.0,
        gravity: 0.0,
        opaque: false,
        colors: fire,
    };
}

/// Weißglühend über Orange nach Rot.
pub fn fire(_: Color) -> Vec<(f32, Srgba)> {
    vec![
        (0.0, Srgba::new(1.0, 0.95, 0.7, 1.0)),
        (0.3, Srgba::new(1.0, 0.55, 0.15, 1.0)),
        (0.7, Srgba::new(0.9, 0.2, 0.05, 0.7)),
        (1.0, Srgba::new(0.4, 0.03, 0.0, 0.0)),
    ]
}

/// Glut: warm und lange sichtbar.
pub fn ember(_: Color) -> Vec<(f32, Srgba)> {
    vec![
        (0.0, Srgba::new(1.0, 0.85, 0.5, 1.0)),
        (0.5, Srgba::new(1.0, 0.45, 0.12, 0.9)),
        (1.0, Srgba::new(0.6, 0.1, 0.0, 0.0)),
    ]
}

/// Rauch: erst vom Feuer angeleuchtet, dann grau und verwehend.
pub fn smoke(_: Color) -> Vec<(f32, Srgba)> {
    vec![
        (0.0, Srgba::new(0.95, 0.55, 0.3, 0.85)),
        (0.3, Srgba::new(0.55, 0.52, 0.58, 0.7)),
        (1.0, Srgba::new(0.4, 0.4, 0.45, 0.0)),
    ]
}

/// Ruß einer Detonation: dunkler und träger als der Rauch eines Brandes.
pub fn soot(_: Color) -> Vec<(f32, Srgba)> {
    vec![
        (0.0, Srgba::new(0.95, 0.5, 0.25, 0.9)),
        (0.25, Srgba::new(0.36, 0.34, 0.37, 0.8)),
        (1.0, Srgba::new(0.18, 0.18, 0.2, 0.0)),
    ]
}

/// Energie in der Schussfarbe: weiß aufblitzend, dann in der Farbe verblassend.
pub fn shot(color: Color) -> Vec<(f32, Srgba)> {
    let color = color.to_srgba();
    vec![
        (0.0, Srgba::WHITE),
        (0.3, color),
        (1.0, color.with_alpha(0.0)),
    ]
}

/// Wohin und wie schnell neue Partikel fliegen.
#[derive(Clone, Copy, Debug)]
struct Launch {
    axis: Vec3,
    cone: f32,
    /// Faktor auf die Geschwindigkeit.
    throwing: f32,
}

/// Ein Dauerstrom: so viele Partikel pro Sekunde, solange er läuft.
#[derive(Clone, Copy, Debug)]
pub struct Emitter {
    pub rate: f32,
    /// Folgt dieser Entity; sonst steht er am Ursprung des Schwarms.
    pub follow: Option<Entity>,
    pub running: bool,
    owed: f32,
}

impl Emitter {
    pub fn new(rate: f32, follow: Option<Entity>) -> Self {
        Self {
            rate,
            follow,
            running: true,
            owed: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    position: Vec3,
    direction: Vec3,
    age: f32,
    life: f32,
    size: f32,
}

/// Ein Schwarm. Die Lage der Entity ist sein Ursprung; die Partikel rechnen in Weltkoordinaten.
#[derive(Component, Debug)]
#[require(NoFrustumCulling)]
pub struct Particles {
    burst: Burst,
    scale: f32,
    launch: Launch,
    gradient: Vec<(f32, LinearRgba)>,
    particles: Vec<Particle>,
    /// Der Strom, aus dem neue Partikel kommen; `None` bei einem Schwarm, der auf einmal losging.
    pub emitter: Option<Emitter>,
    scope: Option<AnimationScope>,
}

impl Particles {
    /// Ein Schwarm, der auf einmal aus `position` in Richtung `axis` spritzt; `scale` vergrößert ihn (Größen,
    /// Schwerkraft), große Explosionen werfen dabei weiter, aber nicht proportional.
    pub fn burst(position: Vec3, axis: Vec3, burst: Burst, color: Color, scale: f32) -> Self {
        let mut particles = Self::new(
            burst,
            color,
            scale,
            Launch {
                axis: if burst.through { -axis } else { axis },
                cone: burst.cone,
                throwing: scale.sqrt(),
            },
        );
        for _ in 0..burst.count {
            particles.emit(position);
        }
        particles
    }

    /// Ein Strom aus dem Kasten (halbe Kantenlänge `burst.spread`) um den Emitter, nach oben und zu allen Seiten.
    pub fn stream(burst: Burst, color: Color, scale: f32, cone: f32, emitter: Emitter) -> Self {
        Self {
            emitter: Some(emitter),
            ..Self::new(
                burst,
                color,
                scale,
                Launch {
                    axis: Vec3::Y,
                    cone,
                    throwing: scale,
                },
            )
        }
    }

    fn new(burst: Burst, color: Color, scale: f32, launch: Launch) -> Self {
        let boost = if burst.opaque { 1.0 } else { GLOW };
        let gradient = (burst.colors)(color)
            .into_iter()
            .map(|(at, color)| {
                let linear = LinearRgba::from(color);
                (at, (linear * boost).with_alpha(color.alpha))
            })
            .collect();
        Self {
            burst,
            scale,
            launch,
            gradient,
            particles: Vec::with_capacity(burst.count as usize),
            emitter: None,
            scope: None,
        }
    }

    fn emit(&mut self, origin: Vec3) {
        let spread = self.burst.spread;
        let offset = Vec3::new(
            random_between((-spread, spread)),
            random_between((-spread, spread)),
            random_between((-spread, spread)),
        );
        let random = rand::random::<f32>();
        self.particles.push(Particle {
            position: origin + offset,
            direction: random_in_cone(self.launch.axis, self.launch.cone)
                * random_between(self.burst.speed)
                * self.launch.throwing,
            age: 0.0,
            life: random_between(self.burst.life),
            size: (self.burst.size.0 + (self.burst.size.1 - self.burst.size.0) * random)
                * self.scale,
        });
    }

    fn color(&self, t: f32) -> LinearRgba {
        let next = self
            .gradient
            .iter()
            .position(|(at, _)| *at >= t)
            .unwrap_or(self.gradient.len() - 1);
        if next == 0 {
            return self.gradient[0].1;
        }
        let (from_at, from) = self.gradient[next - 1];
        let (to_at, to) = self.gradient[next];
        from.mix(
            &to,
            ((t - from_at) / (to_at - from_at).max(1e-6)).clamp(0.0, 1.0),
        )
    }

    fn is_spent(&self) -> bool {
        self.particles.is_empty() && self.emitter.is_none_or(|emitter| !emitter.running)
    }
}

pub(super) struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(prepare).add_systems(
            PostUpdate,
            update.before(bevy::transform::TransformSystems::Propagate),
        );
    }
}

/// Gibt einem neuen Schwarm Mesh und Material.
fn prepare(
    add: On<Add<Particles>>,
    mut commands: Commands,
    particles: Query<&Particles>,
    assets: Res<FxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    let opaque = particles
        .get(add.entity)
        .is_ok_and(|swarm| swarm.burst.opaque);
    let mut material = FxMaterial::light(LinearRgba::WHITE).with_texture(assets.dot.clone());
    if opaque {
        material.blend = FxBlend::Alpha;
    }
    commands.entity(add.entity).insert((
        Mesh3d(meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ))),
        MeshMaterial3d(materials.add(material)),
    ));
}

/// Lässt die Partikel altern, fliegen und neue ausströmen, und schreibt das Mesh neu.
#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    mut swarms: Query<(Entity, &mut Particles, &Transform, &Mesh3d)>,
    targets: Query<&GlobalTransform>,
    camera: Single<&Transform, With<MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    clock: Res<AnimationClock>,
    settings: Res<Settings>,
) {
    let view = camera.forward().as_vec3();
    let camera_up = camera.up().as_vec3();
    for (entity, mut swarm, transform, mesh) in &mut swarms {
        let scope = *swarm.scope.get_or_insert(clock.scope);
        let seconds = time.delta_secs() / settings.time_scale(scope);
        let origin = transform.translation;

        // Ein Strom, dessen Ziel verschwunden ist (Rakete eingeschlagen, Brand erloschen), versiegt.
        if let Some(emitter) = swarm.emitter.as_mut()
            && emitter
                .follow
                .is_some_and(|target| !targets.contains(target))
        {
            emitter.running = false;
        }
        let emission = swarm
            .emitter
            .as_mut()
            .filter(|emitter| emitter.running)
            .map(|emitter| {
                emitter.owed += emitter.rate * seconds;
                let count = emitter.owed.floor();
                emitter.owed -= count;
                (count as u32, emitter.follow)
            });
        if let Some((count, follow)) = emission {
            let at = match follow {
                Some(target) => targets.get(target).ok().map(GlobalTransform::translation),
                None => Some(origin),
            };
            if let Some(at) = at {
                for _ in 0..count {
                    swarm.emit(at);
                }
            }
        }

        let gravity = Vec3::Y * swarm.burst.gravity * swarm.scale;
        for particle in &mut swarm.particles {
            particle.age += if clock.skip { particle.life } else { seconds };
            let t = (particle.age / particle.life).min(1.0);
            particle.direction += gravity * seconds;
            // Babylons Geschwindigkeitsverlauf: von voller Fahrt auf 15 %.
            particle.position += particle.direction * (1.0 - 0.85 * t) * seconds;
        }
        swarm
            .particles
            .retain(|particle| particle.age < particle.life);
        if swarm.is_spent() || clock.skip {
            commands.entity(entity).despawn();
            continue;
        }

        let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
            continue;
        };
        let count = swarm.particles.len();
        let mut positions = Vec::with_capacity(count * 4);
        let mut colors = Vec::with_capacity(count * 4);
        let mut uvs = Vec::with_capacity(count * 4);
        let mut indices = Vec::with_capacity(count * 6);
        for particle in &swarm.particles {
            let t = particle.age / particle.life;
            let size = particle.size * (1.0 + (swarm.burst.grow - 1.0) * t);
            let along = particle.direction - view * particle.direction.dot(view);
            let axis = along.try_normalize().unwrap_or(camera_up);
            let side = axis.cross(view).normalize() * size / 2.0;
            let length = axis * size * swarm.burst.stretch / 2.0;
            let center = particle.position - origin;
            let base = positions.len() as u32;
            for corner in [-side - length, side - length, side + length, -side + length] {
                positions.push((center + corner).to_array());
            }
            let color = swarm.color(t).to_f32_array();
            colors.extend([color; 4]);
            uvs.extend([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let normals = vec![(-view).to_array(); positions.len()];
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(Indices::U32(indices));
    }
}
