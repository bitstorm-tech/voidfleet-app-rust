//! Weltraum-Hintergrund hinter der Szene (Port von `scene/background.ts`): ein unbeleuchtetes Bild am fernen Ende des
//! Sichtfelds, das wie CSS „cover“ füllt und sich leicht gegen die Parallaxe der Kamera verschiebt. Jedes Match zeigt
//! zufällig einen der zehn Hintergründe, nach [`ChangeBackground`] nie zweimal hintereinander denselben.

use bevy::{picking::Pickable, prelude::*, window::PrimaryWindow};
use rand::Rng;

use super::camera::{CAMERA_FAR, CameraRig, CameraSystems, MainCamera};
use crate::data::backgrounds::Backgrounds;

/// Etwas größer als das Bild, damit die Parallaxe keine Ränder zeigt.
const OVERSCAN: f32 = 1.06;
/// Abstand vor der Kamera, knapp vor ihrer Sichtweite.
const DISTANCE: f32 = CAMERA_FAR * 0.95;
/// Versatz als Anteil des Bilds pro Welteinheit Parallaxe.
const SCROLL_STRENGTH: Vec2 = Vec2::new(0.0125, 0.011);

/// Wechselt zum nächsten Match zu einem anderen Hintergrund.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ChangeBackground;

/// Das Bild des Hintergrunds, Kind der Kamera.
#[derive(Component)]
pub struct Background {
    index: usize,
}

pub(super) struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ChangeBackground>()
            .add_systems(PostStartup, spawn_background)
            .add_systems(
                Update,
                (
                    change_background.run_if(on_message::<ChangeBackground>),
                    fit_background.after(CameraSystems),
                ),
            );
    }
}

fn spawn_background(
    mut commands: Commands,
    camera: Single<Entity, With<MainCamera>>,
    backgrounds: Res<Backgrounds>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let index = rand::rng().random_range(0..backgrounds.0.len());
    commands.entity(*camera).with_child((
        Name::new("Hintergrund"),
        Background { index },
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(backgrounds.0[index].clone()),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -DISTANCE),
        Pickable::IGNORE,
    ));
}

fn change_background(
    backgrounds: Res<Backgrounds>,
    background: Single<(&mut Background, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (mut background, material) = background.into_inner();
    let count = backgrounds.0.len();
    background.index = (background.index + 1 + rand::rng().random_range(0..count - 1)) % count;
    if let Some(mut material) = materials.get_mut(&material.0) {
        material.base_color_texture = Some(backgrounds.0[background.index].clone());
    }
}

/// Füllt das Bild wie „cover“ und verschiebt es gegen die Parallaxe der Kamera.
fn fit_background(
    window: Single<&Window, With<PrimaryWindow>>,
    rig: Res<CameraRig>,
    camera: Single<&Projection, With<MainCamera>>,
    backgrounds: Res<Backgrounds>,
    images: Res<Assets<Image>>,
    background: Single<(&Background, &mut Transform)>,
) {
    let Projection::Perspective(perspective) = *camera else {
        return;
    };
    let (background, mut transform) = background.into_inner();
    let Some(image) = images.get(&backgrounds.0[background.index]) else {
        return;
    };
    let height = 2.0 * DISTANCE * (perspective.fov / 2.0).tan();
    let view = Vec2::new(height * window.width() / window.height().max(1.0), height);
    let image_aspect = image.aspect_ratio().ratio();
    let cover = if image_aspect > view.x / view.y {
        Vec2::new(view.y * image_aspect, view.y)
    } else {
        Vec2::new(view.x, view.x / image_aspect)
    };
    // Gegen die Parallaxe: Die Kamera rückt nach rechts, das Bild bleibt zurück, aber viel weniger als das Brett.
    let shift = -rig.scroll() * SCROLL_STRENGTH * view;
    transform.translation = Vec3::new(shift.x, shift.y, -DISTANCE);
    transform.scale = (cover * OVERSCAN).extend(1.0);
}
