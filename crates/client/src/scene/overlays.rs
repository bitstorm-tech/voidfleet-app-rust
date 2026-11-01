//! Was beim Planen über dem Brett steht (Port von `moveArrow.ts`, `targetLines.ts` und `targetReticle.ts`):
//!
//! - **Bewegungspfeile**: leuchtende Bögen vom Schiff zum Ziel-Slot einer geplanten Bewegung; sie steigen über dem
//!   Schiff auf und fallen senkrecht auf den Slot (steht dort ein Schiff, über dessen schwebenden Geist).
//! - **Zielbögen** der Zielvorschau: je Schuss ein dünner Bogen in der Farbe des Schützen, der von oben auf das Ziel
//!   fällt; eigene höher und seitlich versetzt. Der Schuss des gewählten Schützen dicker und heller, ein verpuffender
//!   blass und gestrichelt mit einem Kreuz statt der Spitze, Schiffsfeuer mit einem Ring um Spitze bzw. Kreuz; Kreuz und
//!   Ring drehen sich zur Kamera.
//! - **Fadenkreuze** auf Slots der Gegnerseite, auf die ein Befehl zielt, langsam kreisend.
//!
//! Was zu sehen ist, steht in [`PlanOverlays`] (setzt [`crate::scene::sync`]); gezeichnet wird neu, wenn sich das ändert
//! oder das Brett beim Wechsel der Ansicht umzieht.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use voidfleet_core::board::{Side, SlotRef};

use super::{
    board::{BoardEntities, GHOST_LIFT},
    camera::BoardLayout,
    fx::{Billboard, FxAssets, FxMaterial, GLOW, mix},
};
use crate::palette;

/// Ein Bewegungspfeil; `lifted`: Im Ziel steht schon ein Schiff, die Spitze zielt auf den Geist darüber.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MoveArrow {
    pub from: SlotRef,
    pub to: SlotRef,
    pub lifted: bool,
}

/// Ein Bogen der Zielvorschau.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetLine {
    pub from: SlotRef,
    pub to: SlotRef,
    /// Die Seite des Schützen bestimmt Farbe, Höhe und Versatz.
    pub side: Side,
    /// Der Schuss verpufft.
    pub fizzle: bool,
    /// Schiffsfeuer: Der Schuss verfolgt sein Ziel.
    pub tracking: bool,
    /// Der Schuss des gewählten Schützen.
    pub selected: bool,
}

/// Was beim Planen über dem Brett steht.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanOverlays {
    pub arrows: Vec<MoveArrow>,
    pub lines: Vec<TargetLine>,
    /// Fadenkreuze auf diesen Slots.
    pub reticles: Vec<SlotRef>,
}

/// Bewegungspfeil: Start über dem Schiff, Spitze über dem Geist im Ziel-Slot, Scheitelhöhe, Rohr und Spitze.
const ARROW_START: f32 = 1.1;
const ARROW_TIP: f32 = 1.45;
const ARROW_ARC: f32 = 4.0;
const ARROW_RADIUS: f32 = 0.045;
const ARROW_HEAD_LENGTH: f32 = 0.6;
const ARROW_HEAD_WIDTH: f32 = 0.46;
const ARROW_SEGMENTS: usize = 48;
/// Zielbogen: Start über dem Schützen, Spitze knapp über dem Ziel, unter seinen Statusbalken.
const LINE_START: f32 = 0.9;
const LINE_TIP: f32 = 0.95;
const LINE_RADIUS: f32 = 0.022;
const LINE_HEAD_LENGTH: f32 = 0.34;
const LINE_HEAD_WIDTH: f32 = 0.24;
const LINE_SEGMENTS: usize = 32;
const LINE_ALPHA: f32 = 0.75;
const SELECTED_SCALE: f32 = 1.9;
const SELECTED_ALPHA: f32 = 0.95;
const SELECTED_LIGHTEN: f32 = 0.3;
const FIZZLE_ALPHA: f32 = 0.4;
const FIZZLE_SELECTED_ALPHA: f32 = 0.7;
const FIZZLE_GREY: Color = Color::srgb(0.62, 0.64, 0.68);
const FIZZLE_FADE: f32 = 0.55;
/// Strichelung: so viele Abschnitte der Bahn je Strich, dann so viele frei.
const DASH: usize = 3;
const GAP: usize = 2;
const CROSS_SIZE: f32 = 0.38;
const CROSS_WIDTH: f32 = 0.07;
const RING_DIAMETER: f32 = 0.56;
const RING_THICKNESS: f32 = 0.05;
/// Fadenkreuz: Durchmesser, Strichstärke, Striche über den Ring hinaus, Höhe und Kreisen (rad/s).
const RETICLE_DIAMETER: f32 = 1.3;
const RETICLE_THICKNESS: f32 = 0.05;
const RETICLE_TICK: f32 = 0.42;
const RETICLE_HEIGHT: f32 = 0.08;
const RETICLE_SPIN: f32 = 0.6;
/// Leuchten der Bögen und Pfeile (im Original im GlowLayer), etwas schwächer als die Effekte.
const OVERLAY_GLOW: f32 = GLOW * 0.7;

/// Ein Teil der Overlays; alle verschwinden beim Neuzeichnen.
#[derive(Component)]
struct OverlayPart;

/// Ein kreisendes Fadenkreuz.
#[derive(Component)]
struct Reticle;

pub(super) struct OverlaysPlugin;

impl Plugin for OverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlanOverlays>().add_systems(
            Update,
            (
                redraw.run_if(
                    resource_changed::<PlanOverlays>.or_eager(resource_changed::<BoardLayout>),
                ),
                spin_reticles,
            ),
        );
    }
}

/// Selbstleuchtend nach Alpha gemischt, leicht im Bloom.
fn overlay_material(color: Color, alpha: f32) -> FxMaterial {
    FxMaterial::solid((color.to_linear() * OVERLAY_GLOW).with_alpha(alpha)).double_sided()
}

/// Ein Rohr entlang der Punkte.
fn tube(points: &[Vec3], radius: f32, sides: usize) -> Mesh {
    let mut positions = Vec::with_capacity(points.len() * sides);
    let mut normals = Vec::with_capacity(points.len() * sides);
    let mut indices = Vec::new();
    let mut normal = {
        let tangent = (points[1] - points[0]).normalize();
        tangent.any_orthonormal_vector()
    };
    for (i, point) in points.iter().enumerate() {
        let tangent =
            (points[(i + 1).min(points.len() - 1)] - points[i.saturating_sub(1)]).normalize();
        // Parallel weitergereicht, damit sich das Rohr nicht verdreht.
        normal = (normal - tangent * normal.dot(tangent)).normalize();
        let binormal = tangent.cross(normal);
        for side in 0..sides {
            let angle = side as f32 / sides as f32 * std::f32::consts::TAU;
            let direction = normal * angle.cos() + binormal * angle.sin();
            positions.push((*point + direction * radius).to_array());
            normals.push(direction.to_array());
        }
    }
    for ring in 0..points.len() as u32 - 1 {
        for side in 0..sides as u32 {
            let next = (side + 1) % sides as u32;
            let (a, b) = (ring * sides as u32 + side, ring * sides as u32 + next);
            let (c, d) = (a + sides as u32, b + sides as u32);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

fn cubic_bezier(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, segments: usize) -> Vec<Vec3> {
    (0..=segments)
        .map(|step| {
            let t = step as f32 / segments as f32;
            let u = 1.0 - t;
            p0 * u.powi(3) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * t.powi(3)
        })
        .collect()
}

/// Steil nach oben und leicht zum Ziel, am Ende senkrecht auf das Ziel herab.
fn arc(start: Vec3, head_base: Vec3, top: f32, segments: usize) -> Vec<Vec3> {
    cubic_bezier(
        start,
        start + Vec3::Y * (top - start.y) + (head_base - start) * 0.25,
        head_base + Vec3::Y * (top - head_base.y),
        head_base,
        segments,
    )
}

/// Spitze nach unten: oben breit, unten null.
fn head(width: f32, length: f32) -> Mesh {
    Cone::new(width / 2.0, length)
        .mesh()
        .resolution(16)
        .build()
        .rotated_by(Quat::from_rotation_x(std::f32::consts::PI))
}

#[allow(clippy::too_many_arguments)]
fn redraw(
    mut commands: Commands,
    overlays: Res<PlanOverlays>,
    layout: Res<BoardLayout>,
    board: Res<BoardEntities>,
    parts: Query<Entity, With<OverlayPart>>,
    fx: Res<FxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for part in &parts {
        commands.entity(part).despawn();
    }
    let pad = |slot: SlotRef| layout.slot_position(slot.side, slot.lane, slot.slot_index);

    let arrow_material = materials.add(overlay_material(palette::SELECT, 0.9));
    for arrow in &overlays.arrows {
        let start = pad(arrow.from) + Vec3::Y * ARROW_START;
        let lift = if arrow.lifted { GHOST_LIFT } else { 0.0 };
        let tip = pad(arrow.to) + Vec3::Y * (ARROW_TIP + lift);
        let head_base = tip + Vec3::Y * ARROW_HEAD_LENGTH;
        let points = arc(start, head_base, ARROW_ARC, ARROW_SEGMENTS);
        commands.spawn((
            OverlayPart,
            Mesh3d(meshes.add(tube(&points, ARROW_RADIUS, 12))),
            MeshMaterial3d(arrow_material.clone()),
            Transform::default(),
        ));
        commands.spawn((
            OverlayPart,
            Mesh3d(meshes.add(head(ARROW_HEAD_WIDTH, ARROW_HEAD_LENGTH))),
            MeshMaterial3d(arrow_material.clone()),
            Transform::from_translation(tip + Vec3::Y * ARROW_HEAD_LENGTH / 2.0),
        ));
    }

    for line in &overlays.lines {
        let (arc_height, offset, color) = match line.side {
            // Im Original +z für die eigenen Bögen, hier gespiegelt.
            Side::Player => (2.7, -0.16, palette::PLAYER),
            Side::Enemy => (2.2, 0.16, palette::ENEMY),
        };
        let offset = Vec3::Z * offset;
        let scale = if line.selected { SELECTED_SCALE } else { 1.0 };
        let (color, alpha) = match (line.fizzle, line.selected) {
            (true, selected) => (
                mix(color, FIZZLE_GREY, FIZZLE_FADE),
                if selected {
                    FIZZLE_SELECTED_ALPHA
                } else {
                    FIZZLE_ALPHA
                },
            ),
            (false, true) => (mix(color, Color::WHITE, SELECTED_LIGHTEN), SELECTED_ALPHA),
            (false, false) => (color, LINE_ALPHA),
        };
        let material = materials.add(overlay_material(color, alpha));
        let start = pad(line.from) + offset + Vec3::Y * LINE_START;
        let tip = pad(line.to) + offset + Vec3::Y * LINE_TIP;
        let head_base = tip + Vec3::Y * LINE_HEAD_LENGTH;
        let points = arc(start, head_base, arc_height, LINE_SEGMENTS);
        let radius = LINE_RADIUS * scale;
        let tubes: Vec<Mesh> = if line.fizzle {
            (0..points.len() - 1)
                .step_by(DASH + GAP)
                .map(|first| {
                    tube(
                        &points[first..(first + DASH + 1).min(points.len())],
                        radius,
                        8,
                    )
                })
                .collect()
        } else {
            vec![tube(&points, radius, 8)]
        };
        for mesh in tubes {
            commands.spawn((
                OverlayPart,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                Transform::default(),
            ));
        }
        let head_at = tip + Vec3::Y * LINE_HEAD_LENGTH / 2.0;
        let size = scale.sqrt();
        if line.fizzle {
            // Zwei gekreuzte Balken, immer zur Kamera gedreht.
            commands
                .spawn((
                    OverlayPart,
                    Billboard,
                    Transform::from_translation(head_at),
                    Visibility::default(),
                ))
                .with_children(|cross| {
                    for angle in [FRAC_PI_4, -FRAC_PI_4] {
                        cross.spawn((
                            Mesh3d(fx.cube.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_rotation(Quat::from_rotation_z(angle))
                                .with_scale(Vec3::new(CROSS_SIZE * size, CROSS_WIDTH, 0.01)),
                        ));
                    }
                });
        } else {
            commands.spawn((
                OverlayPart,
                Mesh3d(meshes.add(head(LINE_HEAD_WIDTH * size, LINE_HEAD_LENGTH))),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(head_at),
            ));
        }
        if line.tracking {
            // Ein Ring, der wie das Kreuz zur Kamera zeigt.
            let ring = Torus::new(
                (RING_DIAMETER - RING_THICKNESS) / 2.0 * size,
                (RING_DIAMETER + RING_THICKNESS) / 2.0 * size,
            )
            .mesh()
            .minor_resolution(8)
            .major_resolution(40)
            .build()
            .rotated_by(Quat::from_rotation_x(FRAC_PI_2));
            commands.spawn((
                OverlayPart,
                Billboard,
                Mesh3d(meshes.add(ring)),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(head_at),
            ));
        }
    }

    if overlays.reticles.is_empty() {
        return;
    }
    let reticle_material = materials.add(overlay_material(palette::DAMAGE, 0.9));
    let ring = meshes.add(
        Torus::new(
            (RETICLE_DIAMETER - RETICLE_THICKNESS) / 2.0,
            (RETICLE_DIAMETER + RETICLE_THICKNESS) / 2.0,
        )
        .mesh()
        .minor_resolution(8)
        .major_resolution(48),
    );
    for slot in &overlays.reticles {
        commands
            .spawn((
                OverlayPart,
                Reticle,
                Transform::from_xyz(0.0, RETICLE_HEIGHT, 0.0),
                Visibility::default(),
                ChildOf(board.pads[slot]),
            ))
            .with_children(|reticle| {
                reticle.spawn((
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(reticle_material.clone()),
                ));
                // Die vier Striche ragen über den Ring hinaus nach außen.
                for quarter in 0..4 {
                    let direction = Vec2::from_angle(quarter as f32 * FRAC_PI_2);
                    let at = direction * RETICLE_DIAMETER / 2.0;
                    reticle.spawn((
                        Mesh3d(fx.cube.clone()),
                        MeshMaterial3d(reticle_material.clone()),
                        Transform::from_xyz(at.x, 0.0, at.y)
                            .with_rotation(Quat::from_rotation_y(-direction.to_angle()))
                            .with_scale(Vec3::new(
                                RETICLE_TICK,
                                RETICLE_THICKNESS,
                                RETICLE_THICKNESS,
                            )),
                    ));
                }
            });
    }
}

fn spin_reticles(time: Res<Time>, mut reticles: Query<&mut Transform, With<Reticle>>) {
    for mut transform in &mut reticles {
        transform.rotate_y(-RETICLE_SPIN * time.delta_secs());
    }
}
