//! Modellschau ([`Screen::Models`], Einstellungen → Dev): alle Schiffe in einem Raster, weit weg vom Brett, mit eigener
//! Kamera um einen Blickpunkt. Mausrad zoomt, Ziehen mit der linken Taste verschiebt, mit der rechten dreht; ein Klick
//! auf ein Schiff zerstört es ([`DestroyShip`]), und sobald seine Trümmer verglüht sind, steht es wieder da. Solange die
//! Schau offen ist, ruht die Brettkamera ([`CameraSystems`]).

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    picking::{
        events::PointerClick,
        hover::HoverMap,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    ui_widgets::{Activate, observe},
};

use crate::{
    assets::AppState,
    data::ship_models::{ShipModels, ships},
    i18n::Locale,
    palette,
    run::{RunView, Screen},
    scene::{
        camera::{CameraSystems, LINE_PIXELS, MainCamera, ZOOM_SPEED},
        destruction::DestroyShip,
        ship::{BOARD_SCALE, Ship, spawn_ship},
        status::ShipStatus,
    },
    ui::{
        Fonts,
        button::{self, Key},
        theme,
    },
};

/// Mitte des Rasters, außer Sichtweite des Bretts.
const ORIGIN: Vec3 = Vec3::new(0.0, 0.0, 300.0);
const COLUMNS: usize = 5;
const SPACING: f32 = 4.0;
/// Bis zu so vielen Pixeln Weg gilt ein Druck mit der linken Taste als Klick.
const CLICK_SLOP: f32 = 4.0;
/// Drehung in Radiant und Verschieben in Abständen je Pixel Mausweg.
const TURN_SPEED: f32 = 0.006;
const PAN_SPEED: f32 = 0.0012;

/// Ein Schiff der Schau mit seinem Platz im Raster.
#[derive(Component, Clone, Copy)]
struct ViewerShip(Transform);

/// Das Schiff geht gerade unter.
#[derive(Component)]
struct Dying;

/// Die Oberfläche der Schau.
#[derive(Component)]
struct ViewerUi;

/// Kamera um den Blickpunkt `focus`, dazu der Mausweg seit dem letzten Druck und welche Taste zieht.
#[derive(Resource)]
struct Orbit {
    focus: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    travel: f32,
    grab: Option<MouseButton>,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            focus: ORIGIN,
            yaw: 0.0,
            pitch: 0.6,
            distance: 24.0,
            travel: 0.0,
            grab: None,
        }
    }
}

impl Orbit {
    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, -self.pitch, 0.0)
    }

    fn transform(&self) -> Transform {
        let rotation = self.rotation();
        Transform::from_translation(self.focus + rotation * Vec3::Z * self.distance)
            .with_rotation(rotation)
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Orbit>()
        .configure_sets(
            Update,
            CameraSystems.run_if(not(resource_equals(Screen::Models))),
        )
        .add_observer(respawn)
        .add_systems(
            Update,
            (
                follow_screen.run_if(resource_changed::<Screen>),
                (steer, destroy_clicked).run_if(resource_equals(Screen::Models)),
            )
                .chain()
                .run_if(in_state(AppState::Match)),
        );
}

fn spawn_viewer_ship(commands: &mut Commands, models: &ShipModels, ship: &Ship, home: Transform) {
    let entity = spawn_ship(commands, models, ship.kind);
    commands.entity(entity).insert((
        ViewerShip(home),
        home,
        ShipStatus {
            hidden: true,
            ..ShipStatus::new(1, 1, 0)
        },
    ));
}

/// Baut die Schau beim Öffnen auf und beim Verlassen wieder ab.
fn follow_screen(
    mut commands: Commands,
    screen: Res<Screen>,
    models: Res<ShipModels>,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    mut orbit: ResMut<Orbit>,
    shown: Query<Entity, Or<(With<ViewerShip>, With<ViewerUi>)>>,
) {
    let open = *screen == Screen::Models;
    if open == !shown.is_empty() {
        return;
    }
    if !open {
        for entity in &shown {
            commands.entity(entity).despawn();
        }
        return;
    }
    *orbit = Orbit::default();
    let kinds: Vec<_> = ships().collect();
    let rows = kinds.len().div_ceil(COLUMNS);
    for (index, kind) in kinds.into_iter().enumerate() {
        let cell = Vec2::new((index % COLUMNS) as f32, (index / COLUMNS) as f32)
            - Vec2::new(COLUMNS as f32 - 1.0, rows as f32 - 1.0) / 2.0;
        let home = Transform::from_translation(ORIGIN + Vec3::new(cell.x, 0.0, cell.y) * SPACING)
            .looking_to(Vec3::X, Vec3::Y)
            .with_scale(Vec3::splat(BOARD_SCALE));
        spawn_viewer_ship(&mut commands, &models, &Ship { kind, uid: 0 }, home);
    }
    commands.spawn((
        Name::new("Modellschau"),
        ViewerUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            bottom: px(24),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                theme::text(fonts.body(15.0), locale.models_hint(), palette::TEXT_DIM),
                Pickable::IGNORE,
            ),
            (
                button::text_button(
                    fonts.display_bold(18.0),
                    locale.dialog_close(),
                    Key::PRIMARY,
                    Node {
                        padding: UiRect::axes(px(22), px(9)),
                        ..default()
                    },
                ),
                observe(|_: On<Activate>, mut view: ResMut<RunView>| view.models = false),
            ),
        ],
    ));
}

/// Mausrad zoomt, links ziehen verschiebt, rechts ziehen dreht; über der Oberfläche beginnt kein Ziehen.
fn steer(
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    motion: Res<AccumulatedMouseMotion>,
    hover_map: Res<HoverMap>,
    nodes: Query<(), With<Node>>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
) {
    let over_ui = hover_map
        .get(&PointerId::Mouse)
        .is_some_and(|hits| hits.keys().any(|entity| nodes.contains(*entity)));
    if !over_ui {
        let pixels = match scroll.unit {
            MouseScrollUnit::Line => -scroll.delta.y * LINE_PIXELS,
            MouseScrollUnit::Pixel => -scroll.delta.y,
        };
        orbit.distance = (orbit.distance * (pixels * ZOOM_SPEED).exp()).clamp(2.0, 60.0);
        if let Some(&button) = buttons
            .get_just_pressed()
            .find(|button| matches!(button, MouseButton::Left | MouseButton::Right))
        {
            orbit.grab = Some(button);
            orbit.travel = 0.0;
        }
    }
    if orbit.grab.is_some_and(|button| !buttons.pressed(button)) {
        orbit.grab = None;
    }
    let delta = motion.delta;
    match orbit.grab {
        Some(MouseButton::Left) => {
            orbit.travel += delta.length();
            let rotation = orbit.rotation();
            let step = orbit.distance * PAN_SPEED;
            orbit.focus += (rotation * Vec3::Y * delta.y - rotation * Vec3::X * delta.x) * step;
        }
        Some(MouseButton::Right) => {
            orbit.yaw -= delta.x * TURN_SPEED;
            orbit.pitch = (orbit.pitch + delta.y * TURN_SPEED).clamp(-1.5, 1.5);
        }
        _ => {}
    }
    **camera = orbit.transform();
}

/// Ein Klick (ohne Verschieben) auf ein stehendes Schiff zerstört es.
fn destroy_clicked(
    mut commands: Commands,
    mut clicks: MessageReader<PointerClick>,
    orbit: Res<Orbit>,
    parents: Query<&ChildOf>,
    standing: Query<(), (With<ViewerShip>, Without<Dying>)>,
) {
    for click in clicks.read() {
        if click.button != PointerButton::Primary || orbit.travel > CLICK_SLOP {
            continue;
        }
        let ship = std::iter::once(click.entity)
            .chain(parents.iter_ancestors(click.entity))
            .find(|entity| standing.contains(*entity));
        if let Some(ship) = ship {
            commands.entity(ship).insert(Dying);
            commands.spawn(DestroyShip { ship });
        }
    }
}

/// Die Zerstörung nimmt das Schiff mit, wenn die Trümmer verglüht sind; dann steht es neu an seinem Platz.
fn respawn(
    despawn: On<Despawn<ViewerShip>>,
    mut commands: Commands,
    screen: Res<Screen>,
    models: Res<ShipModels>,
    ships: Query<(&Ship, &ViewerShip)>,
) {
    if *screen != Screen::Models {
        return;
    }
    if let Ok((ship, &ViewerShip(home))) = ships.get(despawn.entity) {
        spawn_viewer_ship(&mut commands, &models, ship, home);
    }
}
