//! Was ein Schiff außer seinem Modell trägt (Port aus `scene/ship.ts`): Statusbalken für Hülle und Schild über dem
//! Modell, als Billboard zur Kamera gedreht, Krone der Flaggschiffe rechts daneben, Totenkopf der Zielvorschau links
//! daneben (auch an Geistern, die keine Balken haben), und der Boden unter dem Schiff für die Marken
//! ([`crate::scene::markers`]). Die Werte stehen in [`ShipStatus`]; wer sie ändert, ändert die Balken.
//!
//! Zielvorschau: Die Füllung zeigt, was nach der Runde bleibt, der Rest bis zum jetzigen Stand blinkt in der
//! Schadensfarbe; fällt das Schiff, pulsiert der Totenkopf. Solange ein Schiff wegen seines Manövers nicht feuert, sind
//! seine Balken gedämpft. Dazu die Grenzen des Modells ([`ShipBounds`]: Höhe der Balken, Größe der Marken, Mitte und
//! Radius für Effekte) und das Flackern des Trugbilds ([`Hologram`]).

use bevy::{camera::primitives::Aabb, prelude::*};
use voidfleet_core::{cards::ShipId, protocol::ShipStats};

use super::{
    board::SHIP_ANCHOR_HEIGHT,
    fx::{Billboard, FxAssets, FxMaterial, linear, mix},
    paint::{Canvas, disc, polygon, polyline, rect},
    ship::{BOARD_SCALE, ModelReady, Ship, ShipModel, ToonState, Wreck},
};
use crate::palette;

/// Die Balken schweben mindestens so hoch, bei hohen Aufbauten knapp über dem Modell (Raum des Schiffs).
const BAR_MIN_HEIGHT: f32 = 1.15;
const BAR_MARGIN: f32 = 0.3;
const TRACK_WIDTH: f32 = 2.72;
const BAR_WIDTH: f32 = 2.56;
const TRACK_COLOR: Color = Color::srgb(0.13, 0.15, 0.22);
const TRACK_ALPHA: f32 = 0.92;
const FILL_ALPHA: f32 = 0.999;
/// Ein Schiff, das diese Runde manövriert und deshalb nicht feuert, zeigt seine Balken gedämpft.
const HOLD_FIRE_ALPHA: f32 = 0.3;
/// Zielvorschau: Takt (Hz) und schwächste Deckkraft des blinkenden Verlusts.
const PREVIEW_PULSE: f32 = 1.4;
const PREVIEW_MIN_VISIBILITY: f32 = 0.25;
/// Totenkopf und Krone, links bzw. rechts neben dem Hüllenbalken.
const SKULL_SIZE: f32 = 0.62;
const CROWN_SIZE: f32 = 0.62;
const BADGE_TEXTURE: u32 = 128;
/// Hologramm des Trugbilds: Deckkraft schwankt um diesen Wert, ab und zu setzt das Bild aus und springt seitlich.
const HOLOGRAM_ALPHA: f32 = 0.82;
const HOLOGRAM_FLICKER: f32 = 0.18;
const HOLOGRAM_GLITCH: f32 = 0.025;
const HOLOGRAM_JITTER: f32 = 0.09;

/// Werte über dem Schiff; die Maxima kommen aus der Karte.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ShipStatus {
    pub hp: i32,
    pub shield: i32,
    /// Zielvorschau: HP und Schild nach der Runde (beides 0, wenn das Schiff fällt); `None` ohne Vorschau.
    pub preview: Option<ShipStats>,
    /// Totenkopf: Laut Zielvorschau fällt das Schiff in dieser Runde.
    pub doomed: bool,
    /// Gedämpfte Balken: Das Schiff feuert wegen seines Manövers nicht.
    pub holding_fire: bool,
    /// Balken, Krone und Marken ausgeblendet (Einflug, Todeskampf).
    pub hidden: bool,
}

impl ShipStatus {
    pub fn new(ship: ShipId) -> Self {
        Self {
            hp: ship.hp(),
            shield: ship.card().shield,
            preview: None,
            doomed: false,
            holding_fire: false,
            hidden: false,
        }
    }
}

/// Grenzen des Modells im Raum des Schiffs (vor dessen Maßstab), sobald es geladen ist.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ShipBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl ShipBounds {
    /// Mitte des Modells in der Welt.
    pub fn world_center(&self, ship: &GlobalTransform) -> Vec3 {
        ship.transform_point((self.min + self.max) / 2.0)
    }

    /// Halbe größte Ausdehnung in Welteinheiten.
    pub fn world_radius(&self) -> f32 {
        (self.max - self.min).length() / 2.0 * BOARD_SCALE
    }

    /// Grundfläche in Welteinheiten: Breite quer zur Nase, Länge entlang der Nase.
    pub fn footprint(&self) -> Vec2 {
        let size = (self.max - self.min) * BOARD_SCALE;
        Vec2::new(size.x.abs(), size.z.abs())
    }
}

/// Die Teile über und unter einem Schiff.
#[derive(Component, Clone, Copy, Debug)]
pub struct ShipParts {
    /// Billboard über dem Modell: Balken, Krone, Totenkopf.
    pub overlay: Entity,
    pub bars: Entity,
    hp_fill: Entity,
    hp_loss: Entity,
    shield_row: Entity,
    shield_fill: Entity,
    shield_loss: Entity,
    tracks: [Entity; 2],
    crown: Option<Entity>,
    skull: Entity,
    /// Boden unter dem Schiff in Welteinheiten (Nase −Z): Träger der Marken.
    pub floor: Entity,
}

/// Flackern des Trugbilds; `fade` ist die Deckkraft beim Auf- und Abbau darunter.
#[derive(Component, Clone, Copy, Debug)]
pub struct Hologram {
    pub fade: f32,
}

/// Material und volle Deckkraft eines Balkens.
#[derive(Component, Clone, Copy, Debug)]
struct Bar {
    alpha: f32,
}

/// Texturen von Krone und Totenkopf.
#[derive(Resource)]
struct Badges {
    crown: Handle<Image>,
    skull: Handle<Image>,
}

pub(super) struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_badges)
            .add_observer(dress_ship)
            .add_systems(Update, (measure, update_bars, pulse_preview, flicker));
    }
}

fn create_badges(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(Badges {
        crown: images.add(crown()),
        skull: images.add(skull()),
    });
}

/// Punkt im Canvas des Originals (0..128 Pixel).
fn pixels(point: Vec2) -> Vec2 {
    (point + 1.0) * BADGE_TEXTURE as f32 / 2.0
}

/// Goldene Krone mit dunklem Rand: fünf Zacken, drei Kugeln, darunter das Band.
fn crown() -> Image {
    let mut canvas = Canvas::new(BADGE_TEXTURE);
    let trim = palette::FLAGSHIP_TRIM.to_srgba();
    let edge = Srgba::new(40.0 / 255.0, 28.0 / 255.0, 8.0 / 255.0, 0.9);
    let outline = [
        (14.0, 96.0),
        (10.0, 38.0),
        (38.0, 62.0),
        (64.0, 24.0),
        (90.0, 62.0),
        (118.0, 38.0),
        (114.0, 96.0),
    ]
    .map(|(x, y)| Vec2::new(x, y));
    let mut closed = outline.to_vec();
    closed.push(outline[0]);
    let stroke = polyline(closed, 6.0);
    canvas.fill(edge, move |point| stroke(pixels(point)));
    let body = polygon(outline.to_vec());
    canvas.fill(trim, move |point| body(pixels(point)));
    let band = rect(14.0, 100.0, 100.0, 14.0);
    canvas.fill(trim, move |point| band(pixels(point)));
    let band_edge = polyline(
        vec![
            Vec2::new(14.0, 100.0),
            Vec2::new(114.0, 100.0),
            Vec2::new(114.0, 114.0),
            Vec2::new(14.0, 114.0),
            Vec2::new(14.0, 100.0),
        ],
        6.0,
    );
    canvas.fill(edge, move |point| band_edge(pixels(point)));
    for (x, y) in [(10.0, 36.0), (64.0, 22.0), (118.0, 36.0)] {
        let ball = disc(Vec2::new(x, y), 8.0);
        canvas.fill(trim, move |point| ball(pixels(point)));
    }
    canvas.into_image()
}

/// Weißer Schädel auf einer Plakette in der Schadensfarbe.
fn skull() -> Image {
    let mut canvas = Canvas::new(BADGE_TEXTURE);
    let badge = palette::DAMAGE.to_srgba();
    let bone = Srgba::new(1.0, 250.0 / 255.0, 245.0 / 255.0, 1.0);
    let plate = disc(Vec2::splat(64.0), 62.0);
    canvas.fill(badge, move |point| plate(pixels(point)));
    let head = disc(Vec2::new(64.0, 56.0), 35.0);
    let jaw = rect(44.0, 70.0, 40.0, 34.0);
    canvas.fill(bone, move |point| {
        let point = pixels(point);
        head(point) || jaw(point)
    });
    let eyes = [
        disc(Vec2::new(50.0, 60.0), 11.0),
        disc(Vec2::new(78.0, 60.0), 11.0),
    ];
    let nose = polygon(vec![
        Vec2::new(64.0, 70.0),
        Vec2::new(58.0, 82.0),
        Vec2::new(70.0, 82.0),
    ]);
    let teeth = [56.0, 64.0, 72.0].map(|x| rect(x - 1.5, 90.0, 3.0, 14.0));
    canvas.fill(badge, move |point| {
        let point = pixels(point);
        eyes.iter().any(|eye| eye(point)) || nose(point) || teeth.iter().any(|tooth| tooth(point))
    });
    canvas.into_image()
}

/// Gibt jedem neuen Schiff Balken, Krone, Totenkopf, Boden und Werte; dem Trugbild sein Flackern.
fn dress_ship(
    add: On<Add<Ship>>,
    ships: Query<&Ship>,
    mut commands: Commands,
    fx: Res<FxAssets>,
    badges: Res<Badges>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    let ship = ships.get(add.entity).expect("Schiff");
    let mut bar = |commands: &mut Commands,
                   parent: Entity,
                   width: f32,
                   height: f32,
                   color: Color,
                   alpha: f32,
                   z: f32| {
        commands
            .spawn((
                Bar { alpha },
                Mesh3d(fx.quad.clone()),
                MeshMaterial3d(materials.add(FxMaterial::solid(linear(color, alpha)).on_top())),
                Transform::from_xyz(0.0, 0.0, z).with_scale(Vec3::new(width, height, 1.0)),
                ChildOf(parent),
            ))
            .id()
    };
    let overlay = commands
        .spawn((
            Name::new("Status"),
            Billboard,
            Transform::from_xyz(0.0, BAR_MIN_HEIGHT, 0.0),
            Visibility::default(),
            ChildOf(add.entity),
        ))
        .id();
    let bars = commands
        .spawn((
            Name::new("Balken"),
            Transform::default(),
            if ship.ghost {
                Visibility::Hidden
            } else {
                Visibility::default()
            },
            ChildOf(overlay),
        ))
        .id();
    let hp_track = bar(
        &mut commands,
        bars,
        TRACK_WIDTH,
        0.3,
        TRACK_COLOR,
        TRACK_ALPHA,
        0.0,
    );
    let hp_fill = bar(
        &mut commands,
        bars,
        BAR_WIDTH,
        0.19,
        palette::HP_FULL,
        FILL_ALPHA,
        0.01,
    );
    let hp_loss = bar(
        &mut commands,
        bars,
        BAR_WIDTH,
        0.19,
        palette::DAMAGE,
        FILL_ALPHA,
        0.01,
    );
    let shield_row = commands
        .spawn((
            Transform::from_xyz(0.0, -0.29, 0.0),
            Visibility::default(),
            ChildOf(bars),
        ))
        .id();
    let shield_track = bar(
        &mut commands,
        shield_row,
        TRACK_WIDTH,
        0.21,
        TRACK_COLOR,
        TRACK_ALPHA,
        0.0,
    );
    let shield_fill = bar(
        &mut commands,
        shield_row,
        BAR_WIDTH,
        0.12,
        palette::SHIELD,
        FILL_ALPHA,
        0.01,
    );
    let shield_loss = bar(
        &mut commands,
        shield_row,
        BAR_WIDTH,
        0.12,
        palette::DAMAGE,
        FILL_ALPHA,
        0.01,
    );
    let badge = |commands: &mut Commands,
                 materials: &mut Assets<FxMaterial>,
                 parent: Entity,
                 texture: Handle<Image>,
                 size: f32,
                 x: f32| {
        commands
            .spawn((
                Mesh3d(fx.quad.clone()),
                MeshMaterial3d(
                    materials.add(
                        FxMaterial::solid(LinearRgba::WHITE)
                            .with_texture(texture)
                            .on_top(),
                    ),
                ),
                Transform::from_xyz(x, -0.05, 0.02).with_scale(Vec3::new(size, size, 1.0)),
                ChildOf(parent),
            ))
            .id()
    };
    // Die Krone wird mit den Balken gedämpft.
    let crown = (ship.id.is_flagship() && !ship.ghost).then(|| {
        let crown = badge(
            &mut commands,
            &mut materials,
            bars,
            badges.crown.clone(),
            CROWN_SIZE,
            (TRACK_WIDTH + CROWN_SIZE) / 2.0 + 0.08,
        );
        commands.entity(crown).insert(Bar { alpha: 1.0 });
        crown
    });
    let skull = badge(
        &mut commands,
        &mut materials,
        overlay,
        badges.skull.clone(),
        SKULL_SIZE,
        -(TRACK_WIDTH + SKULL_SIZE) / 2.0 - 0.08,
    );
    commands.entity(skull).insert(Visibility::Hidden);
    let floor = commands
        .spawn((
            Name::new("Boden"),
            Transform::from_xyz(0.0, -SHIP_ANCHOR_HEIGHT / BOARD_SCALE, 0.0)
                .with_scale(Vec3::splat(1.0 / BOARD_SCALE)),
            Visibility::default(),
            ChildOf(add.entity),
        ))
        .id();
    let mut entity = commands.entity(add.entity);
    // Wer das Schiff spawnt, kann seine Werte gleich mitgeben.
    entity.insert_if_new(ShipStatus::new(ship.id));
    entity.insert((ShipParts {
        overlay,
        bars,
        hp_fill,
        hp_loss,
        shield_row,
        shield_fill,
        shield_loss,
        tracks: [hp_track, shield_track],
        crown,
        skull,
        floor,
    },));
    if ship.id == ShipId::Decoy {
        entity.insert(Hologram { fade: 1.0 });
    }
}

/// Misst das Modell, sobald seine Teile Lage und Grenzen haben, und hebt die Balken knapp darüber.
fn measure(
    mut commands: Commands,
    models: Query<(Entity, &ChildOf), (With<ShipModel>, With<ModelReady>, Without<Wreck>)>,
    ships: Query<(&GlobalTransform, &ShipParts), (With<Ship>, Without<ShipBounds>)>,
    children: Query<&Children>,
    parts: Query<(&GlobalTransform, Option<&Aabb>), With<Mesh3d>>,
    mut transforms: Query<&mut Transform>,
) {
    for (model, child_of) in &models {
        let Ok((ship_transform, ship_parts)) = ships.get(child_of.parent()) else {
            continue;
        };
        let meshes: Vec<_> = children
            .iter_descendants(model)
            .filter_map(|part| parts.get(part).ok())
            .collect();
        if meshes.is_empty() || meshes.iter().any(|(_, aabb)| aabb.is_none()) {
            continue;
        }
        let to_ship = ship_transform.affine().inverse();
        let (mut min, mut max) = (Vec3::INFINITY, Vec3::NEG_INFINITY);
        for (transform, aabb) in meshes {
            let aabb = aabb.expect("geprüft");
            let (center, half) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
            for corner in 0..8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let point =
                    to_ship.transform_point3(transform.transform_point(center + half * sign));
                min = min.min(point);
                max = max.max(point);
            }
        }
        commands
            .entity(child_of.parent())
            .insert(ShipBounds { min, max });
        if let Ok(mut overlay) = transforms.get_mut(ship_parts.overlay) {
            overlay.translation.y = BAR_MIN_HEIGHT.max(max.y + BAR_MARGIN);
        }
    }
}

fn ratio(current: i32, maximum: i32) -> f32 {
    if maximum <= 0 {
        0.0
    } else {
        (current as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn hp_color(ratio: f32) -> Color {
    if ratio >= 0.5 {
        mix(palette::HP_HALF, palette::HP_FULL, (ratio - 0.5) * 2.0)
    } else {
        mix(palette::HP_EMPTY, palette::HP_HALF, ratio * 2.0)
    }
}

/// Ein Abschnitt des Balkens zwischen zwei Anteilen, von links gemessen.
fn set_section(transform: &mut Transform, visibility: &mut Visibility, from: f32, to: f32) {
    *visibility = if to > from {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    transform.scale.x = BAR_WIDTH * (to - from).max(1e-4);
    transform.translation.x = BAR_WIDTH * ((from + to) / 2.0 - 0.5);
}

/// Füllung bis zu dem, was nach der Vorschau bleibt (ohne Vorschau: alles), dahinter der Verlust bis zum jetzigen Stand.
fn update_bars(
    ships: Query<(&Ship, &ShipStatus, &ShipParts), Changed<ShipStatus>>,
    mut parts: Query<(&mut Transform, &mut Visibility)>,
    bars: Query<(&Bar, &MeshMaterial3d<FxMaterial>)>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (ship, status, ship_parts) in &ships {
        let card = ship.id.card();
        let after = status.preview.unwrap_or(ShipStats {
            hp: status.hp,
            shield: status.shield,
        });
        let mut section = |entity: Entity, from: f32, to: f32| {
            if let Ok((mut transform, mut visibility)) = parts.get_mut(entity) {
                set_section(&mut transform, &mut visibility, from, to);
            }
        };
        let hp = after.hp.min(status.hp);
        section(ship_parts.hp_fill, 0.0, ratio(hp, card.hp));
        section(
            ship_parts.hp_loss,
            ratio(hp, card.hp),
            ratio(status.hp, card.hp),
        );
        let shield_max = card.shield.max(status.shield);
        let shield = after.shield.min(status.shield);
        section(ship_parts.shield_fill, 0.0, ratio(shield, shield_max));
        section(
            ship_parts.shield_loss,
            ratio(shield, shield_max),
            ratio(status.shield, shield_max),
        );
        let visible = |shown: bool| {
            if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        };
        if let Ok((_, mut visibility)) = parts.get_mut(ship_parts.shield_row) {
            *visibility = visible(shield_max > 0);
        }
        for (entity, shown) in [
            (ship_parts.bars, !ship.ghost && !status.hidden),
            (ship_parts.skull, status.doomed && !status.hidden),
            (ship_parts.floor, !status.hidden),
        ] {
            if let Ok((_, mut visibility)) = parts.get_mut(entity) {
                visibility.set_if_neq(visible(shown));
            }
        }

        let dim = if status.holding_fire {
            HOLD_FIRE_ALPHA
        } else {
            1.0
        };
        let fill_color = hp_color(ratio(hp, card.hp));
        let mut entities = vec![
            ship_parts.hp_fill,
            ship_parts.shield_fill,
            ship_parts.tracks[0],
            ship_parts.tracks[1],
        ];
        entities.extend(ship_parts.crown);
        for entity in entities {
            let Ok((bar, material)) = bars.get(entity) else {
                continue;
            };
            if let Some(mut material) = materials.get_mut(&material.0) {
                material.set_alpha(bar.alpha * dim);
                if entity == ship_parts.hp_fill {
                    material.color = linear(fill_color, bar.alpha * dim);
                }
            }
        }
    }
}

/// Verlust und Totenkopf blinken im selben Takt, über alle Schiffe gleich.
fn pulse_preview(
    time: Res<Time>,
    ships: Query<(&ShipStatus, &ShipParts)>,
    materials_of: Query<&MeshMaterial3d<FxMaterial>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    let strength = 0.5 + 0.5 * (time.elapsed_secs() * PREVIEW_PULSE * std::f32::consts::TAU).sin();
    let loss = PREVIEW_MIN_VISIBILITY + (1.0 - PREVIEW_MIN_VISIBILITY) * strength;
    for (status, parts) in &ships {
        if status.preview.is_none() && !status.doomed {
            continue;
        }
        let dim = if status.holding_fire {
            HOLD_FIRE_ALPHA
        } else {
            1.0
        };
        for (entity, alpha) in [
            (parts.hp_loss, loss * FILL_ALPHA * dim),
            (parts.shield_loss, loss * FILL_ALPHA * dim),
            (parts.skull, 0.55 + 0.45 * strength),
        ] {
            if let Ok(material) = materials_of.get(entity)
                && let Some(mut material) = materials.get_mut(&material.0)
            {
                material.set_alpha(alpha);
            }
        }
    }
}

/// Hologramm: Die Deckkraft schwankt, ab und zu setzt das Bild aus und springt seitlich.
fn flicker(
    mut holograms: Query<(&Hologram, &mut ToonState, &Children)>,
    mut models: Query<&mut Transform, With<ShipModel>>,
) {
    for (hologram, mut state, children) in &mut holograms {
        let glitch = rand::random::<f32>() < HOLOGRAM_GLITCH;
        let alpha = if glitch {
            0.25
        } else {
            HOLOGRAM_ALPHA + HOLOGRAM_FLICKER * (rand::random::<f32>() - 0.5)
        };
        state.opacity = alpha * hologram.fade;
        for child in children.iter() {
            if let Ok(mut model) = models.get_mut(child) {
                model.translation.x = if glitch {
                    (rand::random::<f32>() - 0.5) * HOLOGRAM_JITTER
                } else {
                    0.0
                };
            }
        }
    }
}
