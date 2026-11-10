//! Was ein Schiff außer seinem Modell trägt (Port aus `scene/ship.ts`): Statusbalken für Hülle und Schild über dem
//! Modell, als Billboard zur Kamera gedreht, eine durchsichtige Schildhülle um das Modell, solange es Schild hat, drei
//! Sterne des gegnerischen Flaggschiffs (Siegel) rechts daneben, Totenkopf der Vorschau links daneben, und der Boden
//! unter dem Schiff für die Marken ([`crate::scene::markers`]). Die Werte stehen
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
    animation::{Animation, AnimationClock, AnimationSystems},
    board::SHIP_ANCHOR_HEIGHT,
    camera::MainCamera,
    effects::ShieldFlare,
    formation::Hull,
    fx::{Billboard, FxAssets, FxMaterial, glowing, linear, mix},
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
const HP_TRACK_HEIGHT: f32 = 0.42;
const HP_FILL_HEIGHT: f32 = 0.34;
const SHIELD_TRACK_HEIGHT: f32 = 0.34;
const SHIELD_FILL_HEIGHT: f32 = 0.27;
const SHIELD_ROW_OFFSET: f32 = (HP_TRACK_HEIGHT + SHIELD_TRACK_HEIGHT) / 2.0 + 0.03;
/// Schriftgröße der Zahl als Anteil der Spur auf dem Schirm (Ziffern sind etwa zwei Drittel davon hoch).
const VALUE_SIZE: f32 = 1.05;
/// Zielvorschau: Takt (Hz) und schwächste Deckkraft des blinkenden Verlusts.
const PREVIEW_PULSE: f32 = 1.4;
const PREVIEW_MIN_VISIBILITY: f32 = 0.25;
/// Totenkopf und Sterne, links bzw. rechts neben dem Hüllenbalken.
const SKULL_SIZE: f32 = 0.62;
// Breiter als der Totenkopf: Die drei Sterne füllen nur ein Drittel der Höhe.
const STARS_SIZE: f32 = 1.3;
const BADGE_TEXTURE: u32 = 128;
/// Schildhülle: Deckkraft, Kantenleuchten (Exponent), Abstand zum Modell (Faktor der halben Ausdehnung) und kleinste
/// Halbachse als Anteil der größten, damit flache Schiffe kein Diskus umgibt.
const SHIELD_HULL_ALPHA: f32 = 0.35;
const SHIELD_HULL_RIM: f32 = 2.5;
const SHIELD_HULL_MARGIN: f32 = 1.3;
const SHIELD_HULL_MIN_AXIS: f32 = 0.45;
/// Aufbau der Schildhülle in Sekunden; bei normaler Animationsgeschwindigkeit (×1,25) genau eine.
const SHIELD_RISE: f32 = 0.8;
/// Dauer des Aufglühens nach einem Schildtreffer, in Sekunden.
const SHIELD_STRIKE: f32 = 0.5;

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
    /// Balken, Sterne und Marken ausgeblendet (Einflug, Todeskampf).
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

/// Schildhülle: Größe nach dem Modell (Raum des Schiffs) und ob sie gerade steht.
#[derive(Component, Default)]
struct ShieldHull {
    scale: Vec3,
    up: bool,
}

/// Aufbau einer Schildhülle: Sie wächst mit Überschwingen, glüht ganz auf, flackert und beruhigt sich zum Rand.
#[derive(Component)]
#[require(Animation::new(SHIELD_RISE))]
struct ShieldRise(Entity);

/// Ein Schuss verpufft am Schild des Schiffs: Die Hülle glüht auf und beruhigt sich wieder.
#[derive(Component)]
#[require(Animation::new(SHIELD_STRIKE))]
pub struct ShieldStrike(pub Entity);

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

    /// Halbachsen der Schildhülle im Raum des Schiffs; sie steht um die Mitte des Modells.
    fn shield_half(&self) -> Vec3 {
        let half = (self.max - self.min) / 2.0 * SHIELD_HULL_MARGIN;
        half.max(Vec3::splat(half.max_element() * SHIELD_HULL_MIN_AXIS))
    }

    /// Wo die Linie von der Mitte des Modells nach `from` die Schildhülle schneidet, in der Welt.
    pub fn shield_point(&self, ship: &GlobalTransform, from: Vec3) -> Vec3 {
        let (center, half) = ((self.min + self.max) / 2.0, self.shield_half());
        let toward = (ship.affine().inverse().transform_point3(from) - center) / half;
        ship.transform_point(center + toward.normalize_or(Vec3::Z) * half)
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
    /// Billboard über dem Modell: Balken, Sterne, Totenkopf.
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
    stars: Option<Entity>,
    skull: Entity,
    /// Schildhülle um das Modell (Raum des Schiffs), sichtbar, solange es Schild hat.
    shield_hull: Entity,
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

/// Texturen von Sternen und Totenkopf.
#[derive(Resource)]
struct Badges {
    stars: Handle<Image>,
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
                    (rise_shields, strike_shields).in_set(AnimationSystems::Apply),
                    (update_bars, pulse_preview, raise_shields)
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
            stars: images.add(stars()),
            skull: images.add(skull()),
        }
    }
}

/// Punkt im Canvas des Originals (0..128 Pixel).
fn pixels(point: Vec2) -> Vec2 {
    (point + 1.0) * BADGE_TEXTURE as f32 / 2.0
}

/// Drei goldene Sterne in einer Reihe mit dunklem Rand, wie das Icon `stars` der Oberfläche.
fn stars() -> Image {
    let mut canvas = Canvas::new(BADGE_TEXTURE);
    let trim = palette::FLAGSHIP_TRIM.to_srgba();
    let edge = Srgba::new(40.0 / 255.0, 28.0 / 255.0, 8.0 / 255.0, 0.9);
    for x in [22.0, 64.0, 106.0] {
        let outline: Vec<Vec2> = (0..10)
            .map(|index| {
                let angle = (index as f32 * 36.0 - 90.0).to_radians();
                let radius = if index % 2 == 0 { 20.0 } else { 8.5 };
                Vec2::new(x, 64.0) + radius * Vec2::from_angle(angle)
            })
            .collect();
        let mut closed = outline.clone();
        closed.push(outline[0]);
        let stroke = polyline(closed, 6.0);
        canvas.fill(edge, move |point| stroke(pixels(point)));
        let body = polygon(outline);
        canvas.fill(trim, move |point| body(pixels(point)));
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

/// Gibt jedem neuen Schiff Balken, Sterne, Totenkopf, Boden und Werte.
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
    // Die Sterne trägt nur ein gegnerisches Flaggschiff; sie werden mit den Balken gedämpft.
    let stars =
        matches!(ship.kind, ShipKind::Enemy(enemy) if enemy.class() == EnemyClass::Flagship).then(
            || {
                let stars = badge(
                    &mut commands,
                    &mut materials,
                    bars,
                    badges.stars.clone(),
                    STARS_SIZE,
                    (TRACK_WIDTH + STARS_SIZE) / 2.0 + 0.08,
                );
                commands.entity(stars).insert(Bar { alpha: 1.0 });
                stars
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
    // Ohne Grenzen keine Größe: [`measure`] legt die Hülle um das Modell.
    let shield_hull = commands
        .spawn((
            Name::new("Schildhülle"),
            ShieldHull::default(),
            Mesh3d(fx.sphere.clone()),
            MeshMaterial3d(materials.add(FxMaterial {
                rim: SHIELD_HULL_RIM,
                ..FxMaterial::light(glowing(palette::SHIELD, SHIELD_HULL_ALPHA))
            })),
            Transform::from_scale(Vec3::ZERO),
            Visibility::Hidden,
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
        stars,
        skull,
        shield_hull,
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
    mut shield_hulls: Query<&mut ShieldHull>,
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
        if let (Ok(mut transform), Ok(mut hull)) = (
            transforms.get_mut(ship_parts.shield_hull),
            shield_hulls.get_mut(ship_parts.shield_hull),
        ) {
            // Die Kugel hat den Durchmesser 1.
            hull.scale = ShipBounds { min, max }.shield_half() * 2.0;
            *transform = Transform::from_translation((min + max) / 2.0).with_scale(hull.scale);
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
            (ship_parts.shield_hull, status.shield > 0 && !status.hidden),
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
        entities.extend(ship_parts.stars);
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
        // Verborgene Zahlen nicht vermessen: Nah an der Kamera wüchse die Schrift sonst ins Riesige (Modellschau).
        let (true, Some((top, bottom))) = (shown, ends) else {
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

/// Kommt ein Schild hoch, baut sich die Hülle auf (bis dahin ohne Größe) und flammt der Aufbau am Schiff auf.
fn raise_shields(
    mut commands: Commands,
    clock: Res<AnimationClock>,
    ships: Query<
        (
            &ShipStatus,
            &ShipParts,
            Option<&ShipBounds>,
            &GlobalTransform,
        ),
        Changed<ShipStatus>,
    >,
    mut hulls: Query<(&mut ShieldHull, &mut Transform)>,
) {
    for (status, parts, bounds, ship) in &ships {
        let Ok((mut hull, mut transform)) = hulls.get_mut(parts.shield_hull) else {
            continue;
        };
        let up = status.shield > 0 && !status.hidden;
        if hull.up == up {
            continue;
        }
        hull.up = up;
        if !up || clock.skip {
            continue;
        }
        transform.scale = Vec3::ZERO;
        commands.spawn(ShieldRise(parts.shield_hull));
        if let Some(bounds) = bounds {
            commands.spawn(ShieldFlare {
                position: bounds.world_center(ship),
                color: palette::SHIELD,
                size: bounds.world_radius(),
            });
        }
    }
}

/// Endet bei Größe, Kante und Deckkraft der ruhenden Hülle.
fn rise_shields(
    rises: Query<(&ShieldRise, &Animation)>,
    mut hulls: Query<(&ShieldHull, &mut Transform, &MeshMaterial3d<FxMaterial>)>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (rise, animation) in &rises {
        let Ok((hull, mut transform, material)) = hulls.get_mut(rise.0) else {
            continue;
        };
        let t = animation.progress();
        transform.scale = hull.scale * EaseFunction::BackOut.sample_clamped(t * 2.0);
        let calm = EaseFunction::CubicOut.sample_clamped(t);
        let flicker = 1.0 + 0.5 * (1.0 - t) * (t * 60.0).sin();
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.rim = SHIELD_HULL_RIM * (0.15 + 0.85 * calm);
            material.set_alpha(SHIELD_HULL_ALPHA * (1.0 + 4.0 * (1.0 - calm)) * flicker);
        }
    }
}

/// Endet bei Kante und Deckkraft der ruhenden Hülle.
fn strike_shields(
    strikes: Query<(&ShieldStrike, &Animation)>,
    ships: Query<&ShipParts>,
    hulls: Query<&MeshMaterial3d<FxMaterial>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (strike, animation) in &strikes {
        let Some(mut material) = ships
            .get(strike.0)
            .and_then(|parts| hulls.get(parts.shield_hull))
            .ok()
            .and_then(|material| materials.get_mut(&material.0))
        else {
            continue;
        };
        let glow = (1.0 - animation.progress()).powi(2);
        material.rim = SHIELD_HULL_RIM * (1.0 - 0.6 * glow);
        material.set_alpha(SHIELD_HULL_ALPHA * (1.0 + 3.0 * glow));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shield_point_lies_on_the_hull_toward_the_shooter() {
        let bounds = ShipBounds {
            min: Vec3::new(-1.0, -0.5, -2.0),
            max: Vec3::new(1.0, 0.5, 2.0),
        };
        let ship =
            GlobalTransform::from(Transform::from_xyz(3.0, 0.0, 0.0).with_scale(Vec3::splat(2.0)));
        let from = Vec3::new(3.0, 0.0, 20.0);
        let point = bounds.shield_point(&ship, from);
        let local = ship.affine().inverse().transform_point3(point) / bounds.shield_half();
        assert!((local.length() - 1.0).abs() < 1e-4);
        assert!(point.z > 0.0 && (point.x - 3.0).abs() < 1e-4);
    }
}
