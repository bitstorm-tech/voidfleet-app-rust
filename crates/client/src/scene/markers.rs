//! Leuchtende Marken am Boden unter einem Schiff (Port von `scene/markers.ts`), in der Farbe ihres Admirals: der Ring
//! unter jedem Flaggschiff, die Zielerfassung der Pendulum (Fadenkreuz mit Rauten), die Marken der
//! Admiralsfähigkeiten — Stacheln beim Gegenfeuer, Zielklammern beim Fokusfeuer, kreisende Drohnen beim
//! Schwarmangriff, eine Uhr beim Taktvorsprung — und die Ablenkung (pulsierender Leuchtring mit Winkeln nach innen).
//! Beim Planen der Feuerbefehle zeigt ein gestrichelter Ring mit drei Kerben die Bereitschaft, das Visier aus Ring und
//! vier Spitzen den gewählten Schützen.
//!
//! Welche Marken ein Schiff trägt, steht in [`ShipMarkers`] (der Ring der Flaggschiffe kommt von selbst dazu); eine Marke,
//! die schon liegt, bleibt liegen, damit ihre Drehung nicht springt. In der Wiedergabe fällt eine neue Marke mit
//! [`DropMarker`] auf das Schiff herab. Jede Marke besteht aus flachen Scheiben mit gezeichneter Textur, die sich drehen
//! oder pulsieren; ihr Halter ist auf die Grundfläche des Schiffs gestreckt und hängt an dessen Boden.

use std::f32::consts::{FRAC_PI_4, TAU};

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::admirals::PowerId;

use super::{
    animation::{Animation, AnimationSystems},
    fx::{FxAssets, FxMaterial, glowing},
    paint::{Canvas, around, dashed_ring, disc, polygon, rect, ring, unrotate},
    ship::Ship,
    status::{ShipBounds, ShipParts},
};
use crate::{data::admirals::flagship_admiral, palette::admiral_color};

/// Die Marken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    /// Ring unter einem Flaggschiff.
    Halo,
    /// Zielerfassung der Pendulum.
    Lock,
    /// Ablenkung: Hierher zielt das Feuer.
    Taunt,
    /// Feuerbefehle: Das Schiff kann noch ein Ziel bekommen.
    Ready,
    /// Feuerbefehle: der gewählte Schütze.
    Aim,
    Counterfire,
    FocusFire,
    SwarmStrike,
    HeadStart,
}

impl MarkerKind {
    /// Die Marke einer Admiralsfähigkeit; das Trugbild hat keine.
    pub fn of_power(power: PowerId) -> Option<Self> {
        match power {
            PowerId::Counterfire => Some(Self::Counterfire),
            PowerId::FocusFire => Some(Self::FocusFire),
            PowerId::SwarmStrike => Some(Self::SwarmStrike),
            PowerId::HeadStart => Some(Self::HeadStart),
            PowerId::Decoy => None,
        }
    }

    /// Zugabe zur Grundfläche, Deckkraft und Ebenen (Glyphe, Drehung in rad/s, Pulsieren in Hz und Ausschlag).
    fn look(self) -> (f32, f32, &'static [Layer]) {
        use Glyph::*;
        const fn layer(glyph: Glyph, spin: f32, pulse: (f32, f32)) -> Layer {
            Layer { glyph, spin, pulse }
        }
        const STILL: (f32, f32) = (0.0, 0.0);
        match self {
            Self::Halo => (
                0.8,
                0.8,
                const {
                    &[
                        layer(Halo, 0.0, (0.45, 0.02)),
                        layer(HaloDashes, 0.18, STILL),
                    ]
                },
            ),
            Self::Lock => (
                0.6,
                0.95,
                const { &[layer(Lock, 0.0, STILL), layer(LockRing, -0.5, STILL)] },
            ),
            Self::Counterfire => (0.6, 0.9, const { &[layer(Spikes, 0.35, STILL)] }),
            Self::FocusFire => (0.6, 0.95, const { &[layer(Brackets, 0.0, (1.6, 0.05))] }),
            Self::SwarmStrike => (0.6, 0.9, const { &[layer(Swarm, 1.3, STILL)] }),
            Self::HeadStart => (
                0.6,
                0.9,
                const { &[layer(Clock, 0.0, STILL), layer(ClockHand, 2.4, STILL)] },
            ),
            Self::Taunt => (
                0.6,
                0.95,
                const {
                    &[
                        layer(Beacon, 0.0, (1.2, 0.08)),
                        layer(BeaconChevrons, -0.4, STILL),
                    ]
                },
            ),
            Self::Ready => (0.6, 0.6, const { &[layer(Ready, 0.25, (0.9, 0.04))] }),
            Self::Aim => (
                0.6,
                0.95,
                const { &[layer(Aim, 0.0, (1.4, 0.03)), layer(AimPointers, 0.8, STILL)] },
            ),
        }
    }
}

/// Eine Marke: Art, Farbe und ob sie erst eingeplant ist (dann blinkt sie).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerSpec {
    pub kind: MarkerKind,
    pub color: Color,
    pub planned: bool,
}

impl MarkerSpec {
    pub fn new(kind: MarkerKind, color: Color) -> Self {
        Self {
            kind,
            color,
            planned: false,
        }
    }
}

/// Die Marken eines Schiffs (ohne den Ring der Flaggschiffe, der immer liegt).
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct ShipMarkers(pub Vec<MarkerSpec>);

impl ShipMarkers {
    /// Legt eine Marke dazu oder ersetzt die gleicher Art.
    pub fn put(&mut self, spec: MarkerSpec) {
        self.0.retain(|other| other.kind != spec.kind);
        self.0.push(spec);
    }

    pub fn remove(&mut self, kind: MarkerKind) {
        self.0.retain(|other| other.kind != kind);
    }
}

/// Auftrag: Legt eine Marke auf ein Schiff und lässt sie aus großem Abstand auf das Schiff herabfallen.
#[derive(Component, Clone, Copy, Debug)]
pub struct DropMarker {
    pub ship: Entity,
    pub spec: MarkerSpec,
}

/// Eine Ebene einer Marke.
#[derive(Clone, Copy, Debug)]
struct Layer {
    glyph: Glyph,
    spin: f32,
    pulse: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Glyph {
    Halo,
    HaloDashes,
    Lock,
    LockRing,
    Spikes,
    Brackets,
    Swarm,
    Clock,
    ClockHand,
    Beacon,
    BeaconChevrons,
    Ready,
    Aim,
    AimPointers,
}

impl Glyph {
    const ALL: [Self; 14] = [
        Self::Halo,
        Self::HaloDashes,
        Self::Lock,
        Self::LockRing,
        Self::Spikes,
        Self::Brackets,
        Self::Swarm,
        Self::Clock,
        Self::ClockHand,
        Self::Beacon,
        Self::BeaconChevrons,
        Self::Ready,
        Self::Aim,
        Self::AimPointers,
    ];
}

/// Quer zur Nase bleibt jede Marke in ihrem Slot, aber nie kleiner als dieser Kreis.
const MAX_WIDTH: f32 = 2.15;
const MIN_SIZE: f32 = 1.5;
/// Knapp über der Platte des Pads, unter dem Schiff.
const FLOOR_LIFT: f32 = 0.07;
/// Eine eingeplante Zielerfassung blinkt (Hz) zwischen dieser und voller Deckkraft.
const PLANNED_BLINK: f32 = 1.4;
const PLANNED_MIN: f32 = 0.3;
/// Auftritt: Die Marke fällt aus dieser Größe auf ihre eigene zusammen.
const INTRO_SCALE: f32 = 2.2;
const INTRO_DURATION: f32 = 0.52;
const TEXTURE_SIZE: u32 = 256;

/// Eine liegende Marke (der Halter).
#[derive(Component, Clone, Copy, Debug)]
struct FloorMarker {
    /// Das Schiff, auf dessen Grundfläche die Marke gestreckt wird.
    ship: Entity,
    spec: MarkerSpec,
    /// Größe beim Auftritt, 1 sobald sie liegt.
    intro: f32,
}

/// Eine Scheibe einer Marke.
#[derive(Component, Clone, Copy, Debug)]
struct MarkerDisc {
    layer: Layer,
    alpha: f32,
}

#[derive(Resource)]
struct GlyphTextures(HashMap<Glyph, Handle<Image>>);

pub(super) struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlyphTextures>()
            .add_observer(drop_marker)
            .add_systems(
                Update,
                (
                    reconcile,
                    (land_markers, animate)
                        .chain()
                        .in_set(AnimationSystems::Apply),
                ),
            );
    }
}

impl FromWorld for GlyphTextures {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let textures = Glyph::ALL
            .into_iter()
            .map(|glyph| (glyph, images.add(draw(glyph))))
            .collect();
        Self(textures)
    }
}

/// Radialer Verlauf der Deckkraft zwischen den Radien `from` und `to`, Stufen als (Anteil, Deckkraft).
fn radial(from: f32, to: f32, stops: &'static [(f32, f32)]) -> impl Fn(Vec2) -> Option<Srgba> {
    move |point| {
        let t = ((point.length() - from) / (to - from)).clamp(0.0, 1.0);
        let next = stops
            .iter()
            .position(|(at, _)| *at >= t)
            .unwrap_or(stops.len() - 1);
        let alpha = if next == 0 {
            stops[0].1
        } else {
            let (a, alpha_a) = stops[next - 1];
            let (b, alpha_b) = stops[next];
            alpha_a + (alpha_b - alpha_a) * (t - a) / (b - a)
        };
        Some(Srgba::WHITE.with_alpha(alpha))
    }
}

/// Zeichnet im Einheitskreis (Mitte 0, Rand 1); das Schiff verdeckt die Mitte, alles Wichtige liegt außen.
fn draw(glyph: Glyph) -> Image {
    let white = Srgba::WHITE;
    let mut canvas = Canvas::new(TEXTURE_SIZE);
    let p = Vec2::new;
    let spin = |index: usize, count: usize| index as f32 / count as f32 * TAU;
    match glyph {
        Glyph::Halo => {
            canvas.fill_with(radial(
                0.5,
                1.0,
                &[(0.0, 0.0), (0.62, 0.55), (0.7, 0.25), (1.0, 0.0)],
            ));
            canvas.fill(white, ring(0.86, 0.025));
        }
        Glyph::HaloDashes => canvas.fill(white, |point| {
            (0..24).any(|index| {
                let length = if index % 3 == 0 { 0.08 } else { 0.04 };
                rect(-0.012, -0.97, 0.024, length)(unrotate(point, spin(index, 24)))
            })
        }),
        Glyph::Lock => {
            canvas.fill(white, ring(0.8, 0.035));
            let bar = rect(-0.022, -0.99, 0.044, 0.34);
            let diamond = polygon(vec![
                p(0.0, -0.97),
                p(0.05, -0.91),
                p(0.0, -0.85),
                p(-0.05, -0.91),
            ]);
            canvas.fill(
                white,
                around(4, move |point| {
                    bar(point) || diamond(unrotate(point, FRAC_PI_4))
                }),
            );
        }
        Glyph::LockRing => canvas.fill(white, dashed_ring(0.9, 0.03, 0.16, 0.1)),
        Glyph::Spikes => {
            canvas.fill(white, ring(0.76, 0.035));
            canvas.fill(
                white,
                around(
                    16,
                    polygon(vec![p(-0.06, -0.77), p(0.0, -0.99), p(0.06, -0.77)]),
                ),
            );
        }
        Glyph::Brackets => {
            let (reach, arm, width) = (0.7, 0.3, 0.06);
            let a = rect(reach - arm, reach - width, arm, width);
            let b = rect(reach - width, reach - arm, width, arm);
            let tick = rect(-0.03, -1.0, 0.06, 0.24);
            canvas.fill(
                white,
                around(4, move |point| a(point) || b(point) || tick(point)),
            );
            canvas.fill(white, ring(0.9, 0.02));
        }
        Glyph::Swarm => {
            canvas.fill(white, |point| {
                (0..40).any(|index| {
                    let angle = spin(index, 40);
                    disc(p(angle.cos() * 0.86, angle.sin() * 0.86), 0.014)(point)
                })
            });
            let drone = polygon(
                [p(-0.13, -0.08), p(0.12, 0.0), p(-0.13, 0.08), p(-0.07, 0.0)]
                    .map(|corner| corner + p(0.0, -0.86))
                    .to_vec(),
            );
            canvas.fill(white, around(6, drone));
        }
        Glyph::Clock => {
            canvas.fill(white, ring(0.9, 0.03));
            canvas.fill(white, |point| {
                (0..12).any(|index| {
                    let length = if index % 3 == 0 { 0.16 } else { 0.08 };
                    rect(-0.02, -0.88, 0.04, length)(unrotate(point, spin(index, 12)))
                })
            });
        }
        Glyph::ClockHand => canvas.fill(
            white,
            polygon(vec![p(-0.035, 0.0), p(0.0, -0.97), p(0.035, 0.0)]),
        ),
        Glyph::Beacon => {
            canvas.fill_with(radial(0.55, 0.95, &[(0.0, 0.0), (0.55, 0.5), (1.0, 0.0)]));
            canvas.fill(white, ring(0.74, 0.04));
            canvas.fill(white, ring(0.84, 0.02));
        }
        Glyph::BeaconChevrons => canvas.fill(
            white,
            around(
                8,
                polygon(vec![
                    p(-0.1, -0.99),
                    p(0.0, -0.89),
                    p(0.1, -0.99),
                    p(0.1, -0.94),
                    p(0.0, -0.84),
                    p(-0.1, -0.94),
                ]),
            ),
        ),
        Glyph::Ready => {
            canvas.fill(white, dashed_ring(0.82, 0.035, 0.12, 0.09));
            canvas.fill(white, around(3, rect(-0.025, -0.99, 0.05, 0.12)));
        }
        Glyph::Aim => {
            canvas.fill(white, ring(0.8, 0.05));
            canvas.fill(white, ring(0.7, 0.015));
        }
        Glyph::AimPointers => canvas.fill(
            white,
            around(
                4,
                polygon(vec![p(-0.09, -0.84), p(0.0, -0.99), p(0.09, -0.84)]),
            ),
        ),
    }
    canvas.into_image()
}

/// Legt genau die Marken, die das Schiff tragen soll; liegende gleiche bleiben.
fn reconcile(
    mut commands: Commands,
    ships: Query<
        (Entity, &Ship, &ShipMarkers, &ShipParts),
        Or<(Changed<ShipMarkers>, Added<ShipParts>)>,
    >,
    children: Query<&Children>,
    markers: Query<(Entity, &FloorMarker)>,
    fx: Res<FxAssets>,
    textures: Res<GlyphTextures>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (ship_entity, ship, specs, parts) in &ships {
        let mut wanted = specs.0.clone();
        if let Some(admiral) = flagship_admiral(ship.id) {
            wanted.push(MarkerSpec::new(MarkerKind::Halo, admiral_color(admiral)));
        }
        let lying: Vec<(Entity, MarkerSpec)> = markers
            .iter_many(children.relationship_sources::<Children>(parts.floor))
            .matched()
            .map(|(entity, marker)| (entity, marker.spec))
            .collect();
        for (entity, spec) in &lying {
            if !wanted.contains(spec) {
                commands.entity(*entity).despawn();
            }
        }
        for spec in wanted {
            if lying.iter().any(|(_, lying)| *lying == spec) {
                continue;
            }
            let (_, alpha, layers) = spec.kind.look();
            commands
                .spawn((
                    Name::new(format!("Marke {:?}", spec.kind)),
                    FloorMarker {
                        ship: ship_entity,
                        spec,
                        intro: 1.0,
                    },
                    Transform::default(),
                    Visibility::default(),
                    ChildOf(parts.floor),
                ))
                .with_children(|holder| {
                    for (index, layer) in layers.iter().enumerate() {
                        holder.spawn((
                            MarkerDisc {
                                layer: *layer,
                                alpha,
                            },
                            Mesh3d(fx.floor_quad.clone()),
                            MeshMaterial3d(
                                materials.add(
                                    FxMaterial::light(glowing(spec.color, alpha))
                                        .with_texture(textures.0[&layer.glyph].clone()),
                                ),
                            ),
                            // Jede Ebene ein Hauch höher, damit sich die Scheiben nicht flimmernd schneiden.
                            Transform::from_xyz(0.0, FLOOR_LIFT + index as f32 * 0.004, 0.0),
                        ));
                    }
                });
        }
    }
}

fn drop_marker(
    add: On<Add<DropMarker>>,
    drops: Query<&DropMarker>,
    mut ships: Query<&mut ShipMarkers>,
    mut commands: Commands,
) {
    let drop = *drops.get(add.entity).expect("Marke");
    // Ist das Schiff schon fort, fällt die Marke ins Leere; der Auftrag läuft trotzdem ab.
    if let Ok(mut markers) = ships.get_mut(drop.ship) {
        markers.put(drop.spec);
    }
    commands
        .entity(add.entity)
        .insert((Name::new("Marke fällt"), Animation::new(INTRO_DURATION)));
}

/// Erscheinen: Die Marke zieht sich aus großem Abstand auf das Schiff zusammen.
fn land_markers(
    drops: Query<(&DropMarker, &Animation)>,
    ships: Query<&ShipParts>,
    children: Query<&Children>,
    mut markers: Query<&mut FloorMarker>,
) {
    for (drop, animation) in &drops {
        let Ok(parts) = ships.get(drop.ship) else {
            continue;
        };
        let landed = EaseFunction::CubicOut.sample_clamped(animation.progress());
        let intro = INTRO_SCALE - (INTRO_SCALE - 1.0) * landed;
        for entity in children.relationship_sources::<Children>(parts.floor) {
            if let Ok(mut marker) = markers.get_mut(entity)
                && marker.spec.kind == drop.spec.kind
            {
                marker.intro = intro;
            }
        }
    }
}

/// Halter auf die Grundfläche gestreckt; die Scheiben drehen sich, pulsieren und blinken, wenn sie erst eingeplant sind.
fn animate(
    time: Res<Time>,
    mut holders: Query<(&FloorMarker, &mut Transform, &Children)>,
    ships: Query<&ShipBounds>,
    mut discs: Query<
        (&MarkerDisc, &mut Transform, &MeshMaterial3d<FxMaterial>),
        Without<FloorMarker>,
    >,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    let seconds = time.elapsed_secs();
    for (marker, mut transform, children) in &mut holders {
        let (margin, _, _) = marker.spec.kind.look();
        let footprint = ships
            .get(marker.ship)
            .map_or(Vec2::ZERO, ShipBounds::footprint);
        transform.scale = Vec3::new(
            (footprint.x + margin).clamp(MIN_SIZE, MAX_WIDTH),
            1.0,
            (footprint.y + margin).max(MIN_SIZE),
        );
        // Nur eingeplante Marken blinken; die übrigen behalten die Deckkraft, mit der sie gespawnt wurden.
        let blink = marker.spec.planned.then(|| {
            PLANNED_MIN + (1.0 - PLANNED_MIN) * (0.5 + 0.5 * (seconds * PLANNED_BLINK * TAU).sin())
        });
        for child in children.iter() {
            let Ok((disc, mut transform, material)) = discs.get_mut(child) else {
                continue;
            };
            let (rate, amount) = disc.layer.pulse;
            let pulse = 1.0 + amount * (seconds * rate * TAU).sin();
            // Babylon dreht linkshändig: Positive Winkel laufen von oben gesehen im Uhrzeigersinn.
            transform.rotation = Quat::from_rotation_y(-seconds * disc.layer.spin);
            transform.scale = Vec3::splat(pulse * marker.intro);
            if let Some(blink) = blink
                && let Some(mut material) = materials.get_mut(&material.0)
            {
                material.set_alpha(disc.alpha * blink);
            }
        }
    }
}
