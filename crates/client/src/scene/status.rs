//! Was ein Schiff außer seinem Modell trägt (Port aus `scene/ship.ts`): Statusbalken für Hülle und Schild über dem
//! Modell, als Billboard zur Kamera gedreht, Krone des gegnerischen Flaggschiffs (Siegel) rechts daneben, Totenkopf der
//! Vorschau links daneben, und der Boden unter dem Schiff für die Marken ([`crate::scene::markers`]). Die Werte stehen
//! in [`ShipStatus`]; wer sie ändert, ändert die Balken.
//!
//! Vorschau: Die Füllung zeigt, was nach dem Schuss oder der Karte bleibt, der Rest bis zum jetzigen Stand blinkt in
//! der Schadensfarbe; fällt das Schiff, pulsiert der Totenkopf. Dazu die Grenzen des Modells ([`ShipBounds`]: Höhe der
//! Balken, Größe der Marken, Mitte und Radius für Effekte). In jedem Balken steht `<aktuell>/<total>`, als Knoten der
//! Oberfläche über seiner Spur, so groß, wie die Spur auf dem Schirm hoch ist.

use bevy::{
    camera::primitives::Aabb,
    picking::Pickable,
    prelude::*,
    transform::TransformSystems,
    ui::{UiSystems, UiTransform},
};
use voidfleet_core::{enemies::EnemyClass, events::UnitStats};

use super::{
    animation::AnimationSystems,
    board::SHIP_ANCHOR_HEIGHT,
    camera::MainCamera,
    formation::Hull,
    fx::{Billboard, FxAssets, FxMaterial, linear, mix},
    paint::{Canvas, disc, polygon, polyline, rect},
    ship::{BOARD_SCALE, ModelReady, Ship, ShipModel, Wreck},
    sync::SyncSystems,
};
use crate::{data::ship_models::ShipKind, palette, ui::Fonts};

/// Die Balken schweben mindestens so hoch, bei hohen Aufbauten knapp über dem Modell (Raum des Schiffs).
const BAR_MIN_HEIGHT: f32 = 1.15;
const BAR_MARGIN: f32 = 0.3;
const TRACK_WIDTH: f32 = 2.72;
const BAR_WIDTH: f32 = 2.56;
const TRACK_COLOR: Color = Color::srgb(0.13, 0.15, 0.22);
const TRACK_ALPHA: f32 = 0.92;
const FILL_ALPHA: f32 = 0.999;
/// Höhe der Spuren und Füllungen von Hülle und Schild, Abstand der Mitten (Raum des Schiffs).
const HP_TRACK_HEIGHT: f32 = 0.62;
const HP_FILL_HEIGHT: f32 = 0.5;
const SHIELD_TRACK_HEIGHT: f32 = 0.5;
const SHIELD_FILL_HEIGHT: f32 = 0.4;
const SHIELD_ROW_OFFSET: f32 = (HP_TRACK_HEIGHT + SHIELD_TRACK_HEIGHT) / 2.0 + 0.03;
/// Schriftgröße der Zahl als Anteil der Spur auf dem Schirm (Ziffern sind etwa zwei Drittel davon hoch).
const VALUE_SIZE: f32 = 1.05;
/// Zielvorschau: Takt (Hz) und schwächste Deckkraft des blinkenden Verlusts.
const PREVIEW_PULSE: f32 = 1.4;
const PREVIEW_MIN_VISIBILITY: f32 = 0.25;
/// Totenkopf und Krone, links bzw. rechts neben dem Hüllenbalken.
const SKULL_SIZE: f32 = 0.62;
const CROWN_SIZE: f32 = 0.62;
const BADGE_TEXTURE: u32 = 128;

/// Werte über dem Schiff.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipStatus {
    pub hull: i32,
    pub max_hull: i32,
    pub shield: i32,
    /// Höchster Schild seit dem letzten Aufbau: Maßstab des Schildbalkens.
    pub max_shield: i32,
    /// Vorschau: Hülle und Schild nach dem Schuss oder der Karte; `None` ohne Vorschau.
    pub preview: Option<UnitStats>,
    /// Totenkopf: Laut Vorschau fällt das Schiff.
    pub doomed: bool,
    /// Balken, Krone und Marken ausgeblendet (Einflug, Todeskampf).
    pub hidden: bool,
}

impl ShipStatus {
    pub fn new(hull: i32, max_hull: i32, shield: i32) -> Self {
        Self {
            hull,
            max_hull,
            shield,
            max_shield: shield,
            ..default()
        }
    }

    /// Neuer Schildwert; fällt der Schild auf null, beginnt der Maßstab beim nächsten Aufbau neu.
    pub fn with_shield(self, shield: i32) -> Self {
        let max_shield = if shield == 0 {
            0
        } else {
            self.max_shield.max(shield)
        };
        Self {
            shield,
            max_shield,
            ..self
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
    /// Die Zahlen in Hüllen- und Schildbalken (Knoten der Oberfläche).
    values: [Entity; 2],
    crown: Option<Entity>,
    skull: Entity,
    /// Boden unter dem Schiff in Welteinheiten (Nase −Z): Träger der Marken.
    pub floor: Entity,
}

/// Material und volle Deckkraft eines Balkens.
#[derive(Component, Clone, Copy, Debug)]
struct Bar {
    alpha: f32,
}

/// Zahl in einem Balken: Schiff und Spur (0 Hülle, 1 Schild).
#[derive(Component, Clone, Copy, Debug)]
struct BarValue {
    ship: Entity,
    track: usize,
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
        app.init_resource::<Badges>()
            .add_observer(dress_ship)
            .add_systems(
                Update,
                (
                    measure,
                    // Nach Wiedergabe und Abgleich, die die Werte schreiben: Die Balken zeigen sie im selben Bild.
                    (update_bars, pulse_preview)
                        .after(AnimationSystems::Apply)
                        .after(SyncSystems),
                ),
            )
            .add_systems(
                PostUpdate,
                // Nach den Lagen dieses Bilds und vor dem UI-Layout: Die Zahl steht im selben Bild im Balken.
                follow_bars
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
            );
    }
}

impl FromWorld for Badges {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        Self {
            crown: images.add(crown()),
            skull: images.add(skull()),
        }
    }
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

/// Gibt jedem neuen Schiff Balken, Krone, Totenkopf, Boden und Werte.
fn dress_ship(
    add: On<Add<Ship>>,
    ships: Query<&Ship>,
    mut commands: Commands,
    fx: Res<FxAssets>,
    badges: Res<Badges>,
    fonts: Res<Fonts>,
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
            Visibility::default(),
            ChildOf(overlay),
        ))
        .id();
    let hp_track = bar(
        &mut commands,
        bars,
        TRACK_WIDTH,
        HP_TRACK_HEIGHT,
        TRACK_COLOR,
        TRACK_ALPHA,
        0.0,
    );
    let hp_fill = bar(
        &mut commands,
        bars,
        BAR_WIDTH,
        HP_FILL_HEIGHT,
        palette::HP_FULL,
        FILL_ALPHA,
        0.01,
    );
    let hp_loss = bar(
        &mut commands,
        bars,
        BAR_WIDTH,
        HP_FILL_HEIGHT,
        palette::DAMAGE,
        FILL_ALPHA,
        0.01,
    );
    let shield_row = commands
        .spawn((
            Transform::from_xyz(0.0, -SHIELD_ROW_OFFSET, 0.0),
            Visibility::default(),
            ChildOf(bars),
        ))
        .id();
    let shield_track = bar(
        &mut commands,
        shield_row,
        TRACK_WIDTH,
        SHIELD_TRACK_HEIGHT,
        TRACK_COLOR,
        TRACK_ALPHA,
        0.0,
    );
    let shield_fill = bar(
        &mut commands,
        shield_row,
        BAR_WIDTH,
        SHIELD_FILL_HEIGHT,
        palette::SHIELD,
        FILL_ALPHA,
        0.01,
    );
    let shield_loss = bar(
        &mut commands,
        shield_row,
        BAR_WIDTH,
        SHIELD_FILL_HEIGHT,
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
    // Die Krone trägt nur ein gegnerisches Flaggschiff; sie wird mit den Balken gedämpft.
    let crown =
        matches!(ship.kind, ShipKind::Enemy(enemy) if enemy.class() == EnemyClass::Flagship).then(
            || {
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
            },
        );
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
    let values = [0, 1].map(|track| {
        commands
            .spawn((
                Name::new("Balkenwert"),
                BarValue {
                    ship: add.entity,
                    track,
                },
                Text::default(),
                fonts.display_bold(12.0),
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(0.0, 1.0),
                    color: Color::BLACK.with_alpha(0.9),
                },
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                UiTransform {
                    translation: Val2::percent(-50.0, -50.0),
                    ..default()
                },
                GlobalZIndex(-6),
                Pickable::IGNORE,
                Visibility::Hidden,
            ))
            .id()
    });
    let mut entity = commands.entity(add.entity);
    // Wer das Schiff spawnt, kann seine Werte gleich mitgeben.
    entity.insert_if_new(ShipStatus::default());
    entity.insert((ShipParts {
        overlay,
        bars,
        hp_fill,
        hp_loss,
        shield_row,
        shield_fill,
        shield_loss,
        tracks: [hp_track, shield_track],
        values,
        crown,
        skull,
        floor,
    },));
}

/// Misst die Modelle einer Gruppe, sobald alle Teile Lage und Grenzen haben, und hebt die Balken knapp darüber.
fn measure(
    mut commands: Commands,
    groups: Query<(Entity, &ChildOf, &Children), (With<ShipModel>, Without<Wreck>)>,
    ready: Query<(), With<ModelReady>>,
    hulls: Query<(), With<Hull>>,
    ships: Query<(&GlobalTransform, &ShipParts), (With<Ship>, Without<ShipBounds>)>,
    children: Query<&Children>,
    parts: Query<(&GlobalTransform, Option<&Aabb>), With<Mesh3d>>,
    mut transforms: Query<&mut Transform>,
) {
    for (group, child_of, members) in &groups {
        let Ok((ship_transform, ship_parts)) = ships.get(child_of.parent()) else {
            continue;
        };
        if !members
            .iter()
            .filter(|member| hulls.contains(*member))
            .all(|member| ready.contains(member))
        {
            continue;
        }
        let meshes: Vec<_> = children
            .iter_descendants(group)
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
    ships: Query<(&ShipStatus, &ShipParts), Changed<ShipStatus>>,
    mut parts: Query<(&mut Transform, &mut Visibility)>,
    bars: Query<(&Bar, &MeshMaterial3d<FxMaterial>)>,
    mut texts: Query<&mut Text>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (status, ship_parts) in &ships {
        for (entity, current, total) in [
            (ship_parts.values[0], status.hull, status.max_hull),
            (ship_parts.values[1], status.shield, status.max_shield),
        ] {
            if let Ok(mut text) = texts.get_mut(entity) {
                text.0 = format!("{current}/{total}");
            }
        }
        let after = status.preview.unwrap_or(UnitStats {
            hull: status.hull,
            shield: status.shield,
        });
        let mut section = |entity: Entity, from: f32, to: f32| {
            if let Ok((mut transform, mut visibility)) = parts.get_mut(entity) {
                set_section(&mut transform, &mut visibility, from, to);
            }
        };
        let hull = after.hull.min(status.hull);
        section(ship_parts.hp_fill, 0.0, ratio(hull, status.max_hull));
        section(
            ship_parts.hp_loss,
            ratio(hull, status.max_hull),
            ratio(status.hull, status.max_hull),
        );
        // Schilde halten nur einen Gegnerzug: Der Balken zeigt, was davon übrig ist.
        let shield_max = status.max_shield;
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
            *visibility = visible(status.shield > 0);
        }
        for (entity, shown) in [
            (ship_parts.bars, !status.hidden),
            (ship_parts.skull, status.doomed && !status.hidden),
            (ship_parts.floor, !status.hidden),
        ] {
            if let Ok((_, mut visibility)) = parts.get_mut(entity) {
                visibility.set_if_neq(visible(shown));
            }
        }

        let fill_color = hp_color(ratio(hull, status.max_hull));
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
                material.set_alpha(bar.alpha);
                if entity == ship_parts.hp_fill {
                    material.color = linear(fill_color, bar.alpha);
                }
            }
        }
    }
}

/// Stellt jede Zahl in die Mitte ihrer Spur, so hoch wie die Spur auf dem Schirm; ist das Schiff fort, geht sie mit.
fn follow_bars(
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ships: Query<(&ShipParts, &ShipStatus), With<Ship>>,
    tracks: Query<&GlobalTransform>,
    mut values: Query<(Entity, &BarValue, &mut Node, &mut TextFont, &mut Visibility)>,
) {
    let (camera, camera_transform) = *camera;
    for (entity, value, mut node, mut font, mut visibility) in &mut values {
        let Ok((parts, status)) = ships.get(value.ship) else {
            commands.entity(entity).despawn();
            continue;
        };
        let screen = |point: Vec3| camera.world_to_viewport(camera_transform, point).ok();
        // Die Spur ist ein Einheitsquadrat, auf ihre Größe skaliert.
        let ends = tracks
            .get(parts.tracks[value.track])
            .ok()
            .and_then(|track| {
                Some((
                    screen(track.transform_point(Vec3::Y * 0.5))?,
                    screen(track.transform_point(Vec3::Y * -0.5))?,
                ))
            });
        let shown = !status.hidden && (value.track == 0 || status.shield > 0);
        visibility.set_if_neq(if shown && ends.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let Some((top, bottom)) = ends else {
            continue;
        };
        let center = (top + bottom) / 2.0;
        node.left = Val::Px(center.x);
        node.top = Val::Px(center.y);
        let size = FontSize::Px(((bottom.y - top.y).abs() * VALUE_SIZE).round().max(1.0));
        if font.font_size != size {
            font.font_size = size;
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
        for (entity, alpha) in [
            (parts.hp_loss, loss * FILL_ALPHA),
            (parts.shield_loss, loss * FILL_ALPHA),
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
