//! Die Kamera des Match-Bretts (Port aus `scene/matchScene.ts`): zwei Ansichten mit eigener Aufteilung des Bretts, von
//! der Seite mit der eigenen Seite links und von hinten oben mit der eigenen Seite unten, engeren Lanes und weiteren
//! Slots, damit das Brett über die Hand passt. Dazwischen ist jeder Stand möglich ([`CameraRig::blend`]):
//! [`CameraCommand::ToggleView`] fährt in einer kurzen Kamerafahrt zur anderen Ansicht, [`CameraCommand::Orbit`] dreht
//! stufenlos, solange die Richtung nicht 0 ist. Gemischt werden Ausrichtung, Aufteilung ([`BoardLayout`]) und das
//! Bildband, das die Pads senkrecht füllen; die Kamera stellt `frame_board` daraus in geschlossener Form.
//!
//! Dazu Maus-Parallaxe, Kamerawackeln ([`CameraCommand::Shake`]) und Glühen über Bloom: Was heller als 1 leuchtet
//! (Lichter der Schiffe, Effekte), glüht; alles andere bleibt scharf. Ohne Tone-Mapping, wie im Original.
//! Stand und Zoom merkt sich die Kamera in den Einstellungen, sobald sie ruht.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    camera::Hdr,
    post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
    prelude::*,
    render::view::Tonemapping,
    window::PrimaryWindow,
};

use super::{
    coords::{BOARD_CENTER, LaneType, PAD_DEPTH, PAD_WIDTH, Side, slot_z},
    tween::AnimationScope,
};
use crate::settings::Settings;

/// Eine Kameraansicht samt Aufteilung des Bretts. `yaw` dreht die Kamera um die Hochachse, `pitch` neigt sie nach
/// unten; wo sie steht, folgt aus `frame`.
#[derive(Clone, Copy, Debug)]
pub struct ViewSpec {
    pub yaw: f32,
    pub pitch: f32,
    /// Senkrechter Bildwinkel.
    pub fov: f32,
    /// Bildband, das die Pads senkrecht füllen: Ober- und Unterkante in Bildhöhen von oben.
    pub frame: (f32, f32),
    /// Welt-x der Lanes: eigene Command, eigene Strike, gegnerische Strike, gegnerische Command.
    pub lane_x: [f32; 4],
    /// Abstand der Slots entlang der Lane; zwischen den Pads bleibt eine Fuge.
    pub slot_step: f32,
    /// Lane-z der Beschriftung relativ zur Brettmitte, jenseits von Slot 0.
    pub label_z: f32,
}

/// Von der Seite: eigene Seite links, Slot 0 hinten, die Beschriftung dahinter.
pub const SIDE_VIEW: ViewSpec = ViewSpec {
    yaw: 0.0,
    pitch: 42.0 * PI / 180.0,
    fov: 33.5 * PI / 180.0,
    frame: (0.16, 0.725),
    lane_x: [-6.15, -2.75, 2.75, 6.15],
    slot_step: 2.2,
    label_z: -6.4,
};

/// Von hinten oben: eigene Seite unten, Slot 0 links, die Beschriftung links daneben.
pub const TOP_VIEW: ViewSpec = ViewSpec {
    yaw: FRAC_PI_2,
    pitch: 50.0 * PI / 180.0,
    fov: 30.0 * PI / 180.0,
    frame: (0.153, 0.679),
    lane_x: [-5.4, -2.0, 2.0, 5.4],
    slot_step: 2.6,
    label_z: -9.1,
};

/// Dauer der Kamerafahrt zwischen den Ansichten in Sekunden, gestreckt wie die übrigen Animationen.
const VIEW_SWITCH_DURATION: f32 = 1.1;
/// Mit gehaltener Taste dreht die Kamera in dieser Zeit von einer Ansicht zur anderen; An- und Auslauf in Sekunden.
const ORBIT_DURATION: f32 = 1.6;
const ORBIT_RAMP: f32 = 0.15;
/// Kamerawackeln: Abklingrate pro Sekunde und die Frequenzen (rad/s) der überlagerten Schwingungen.
const SHAKE_DECAY: f32 = 4.5;
const SHAKE_FREQUENCIES: (f32, f32, f32) = (83.0, 109.0, 23.0);
/// Kamera-Parallaxe: Reichweite in Welteinheiten und Nachlauf.
const MAX_SCROLL_OFFSET: Vec2 = Vec2::new(0.75, 0.4);
const FOLLOW_SPEED: f32 = 4.0;
/// Zoom: Nachlauf des geglätteten Versatzes und die tiefste Kamerahöhe.
const ZOOM_FOLLOW_SPEED: f32 = 8.0;
const ZOOM_MIN_HEIGHT: f32 = 2.6;
/// Sichtweite der Kamera; der Hintergrund liegt knapp davor.
pub const CAMERA_NEAR: f32 = 0.1;
pub const CAMERA_FAR: f32 = 80.0;

/// Die Kamera des Bretts. Effekte, Picking und das Stereopanorama der Sounds rechnen mit ihr.
#[derive(Component)]
pub struct MainCamera;

/// Befehle an die Kamera.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum CameraCommand {
    /// Fährt zur anderen Ansicht, von einem Stand dazwischen zur ferneren, und kehrt eine laufende Fahrt um.
    ToggleView,
    /// Dreht stufenlos zwischen den Ansichten, solange die Richtung nicht 0 ist: 1 nach hinten oben, −1 zur Seite.
    Orbit(f32),
    /// Stößt die Kamera an: Sie zittert mit so vielen Welteinheiten und beruhigt sich wieder.
    Shake(f32),
}

/// Aufteilung des Bretts beim jetzigen Stand zwischen den Ansichten; Lanes, Slots und Beschriftungen folgen ihr.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BoardLayout {
    /// Lane-x wie in [`ViewSpec::lane_x`].
    pub lane_x: [f32; 4],
    pub slot_step: f32,
    pub label_z: f32,
}

impl BoardLayout {
    fn mix(progress: f32) -> Self {
        let mix = |from: f32, to: f32| from + (to - from) * progress;
        Self {
            lane_x: std::array::from_fn(|i| mix(SIDE_VIEW.lane_x[i], TOP_VIEW.lane_x[i])),
            slot_step: mix(SIDE_VIEW.slot_step, TOP_VIEW.slot_step),
            label_z: mix(SIDE_VIEW.label_z, TOP_VIEW.label_z),
        }
    }

    /// Welt-x einer Lane.
    pub fn lane_x(&self, side: Side, lane: LaneType) -> f32 {
        self.lane_x[match (side, lane) {
            (Side::Player, LaneType::Command) => 0,
            (Side::Player, LaneType::Strike) => 1,
            (Side::Enemy, LaneType::Strike) => 2,
            (Side::Enemy, LaneType::Command) => 3,
        }]
    }

    /// Weltlage der Mitte eines Pads.
    pub fn slot_position(&self, side: Side, lane: LaneType, index: usize) -> Vec3 {
        BOARD_CENTER + Vec3::new(self.lane_x(side, lane), 0.0, slot_z(index, self.slot_step))
    }

    /// Mitte einer Brettseite zwischen ihren Lanes, für Effekte und Sounds, die zu keinem Schiff gehören.
    pub fn side_center(&self, side: Side) -> Vec3 {
        let x = (self.lane_x(side, LaneType::Strike) + self.lane_x(side, LaneType::Command)) / 2.0;
        BOARD_CENTER + Vec3::X * x
    }

    fn half_extent(&self) -> Vec2 {
        let half_width =
            self.lane_x.iter().fold(0.0_f32, |max, x| max.max(x.abs())) + PAD_WIDTH / 2.0;
        Vec2::new(
            half_width,
            slot_z(0, self.slot_step).abs() + PAD_DEPTH / 2.0,
        )
    }
}

/// Zustand der Kamera: Stand zwischen den Ansichten, Fahrt, Zoom, Parallaxe und Wackeln.
#[derive(Resource, Debug)]
pub struct CameraRig {
    /// Stand zwischen den Ansichten (0 von der Seite, 1 von hinten oben) und sein Tempo pro Sekunde.
    blend: f32,
    blend_speed: f32,
    /// Ziel einer Kamerafahrt (Knopf, Taste V).
    goal: Option<f32>,
    /// Richtung einer stufenlosen Drehung.
    orbit: f32,
    /// Ruhelage und Ausrichtung beim jetzigen Stand.
    rest: Vec3,
    yaw: f32,
    pitch: f32,
    fov: f32,
    /// Versatz aus der Ruhelage durch Zoom und Verschieben: Ziel und geglätteter Ist-Wert.
    zoom_target: Vec3,
    zoom_offset: Vec3,
    /// Parallaxe (Welteinheiten, waagerecht im Bild und in der Höhe).
    scroll: Vec2,
    shake_strength: f32,
    shake_time: f32,
    shake_offset: Vec2,
}

impl CameraRig {
    /// Stand zwischen den Ansichten: 0 von der Seite, 1 von hinten oben.
    pub fn blend(&self) -> f32 {
        self.blend
    }

    /// Ob die Kamera gerade zwischen den Ansichten fährt oder dreht.
    pub fn is_turning(&self) -> bool {
        self.goal.is_some() || self.blend_speed != 0.0
    }

    /// Parallaxe als Versatz in Welteinheiten (waagerecht im Bild, Höhe); der Hintergrund verschiebt sich dagegen.
    pub fn scroll(&self) -> Vec2 {
        self.scroll
    }

    /// Waagerecht nach rechts im Bild.
    pub fn screen_right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    fn from_settings(settings: &Settings) -> (Self, BoardLayout) {
        let zoom = Vec3::from_array(settings.camera.zoom);
        let mut rig = Self {
            blend: settings.camera.view.clamp(0.0, 1.0),
            blend_speed: 0.0,
            goal: None,
            orbit: 0.0,
            rest: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov: SIDE_VIEW.fov,
            zoom_target: zoom,
            zoom_offset: zoom,
            scroll: Vec2::ZERO,
            shake_strength: 0.0,
            shake_time: 0.0,
            shake_offset: Vec2::ZERO,
        };
        let layout = rig.apply_view();
        (rig, layout)
    }

    /// Stellt Ausrichtung und Ruhelage auf den Stand zwischen den Ansichten und liefert die Aufteilung des Bretts.
    fn apply_view(&mut self) -> BoardLayout {
        let progress = EaseFunction::SineInOut.sample_clamped(self.blend);
        let mix = |from: f32, to: f32| from + (to - from) * progress;
        let layout = BoardLayout::mix(progress);
        self.yaw = mix(SIDE_VIEW.yaw, TOP_VIEW.yaw);
        self.pitch = mix(SIDE_VIEW.pitch, TOP_VIEW.pitch);
        self.fov = mix(SIDE_VIEW.fov, TOP_VIEW.fov);
        let band = (
            mix(SIDE_VIEW.frame.0, TOP_VIEW.frame.0),
            mix(SIDE_VIEW.frame.1, TOP_VIEW.frame.1),
        );
        self.rest = self.frame_board(layout.half_extent(), band);
        layout
    }

    /// Blickrichtung und Oben der Kamera bei ihrer Ausrichtung.
    fn axes(&self) -> (Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let forward = Vec3::new(sin_yaw * cos_pitch, -sin_pitch, -cos_yaw * cos_pitch);
        let up = Vec3::new(sin_yaw * sin_pitch, cos_pitch, -cos_yaw * sin_pitch);
        (forward, up)
    }

    /// Ruhelage, in der die Brettmitte waagerecht in der Bildmitte liegt und die Pads senkrecht genau das Band von
    /// `top` bis `bottom` (Bildhöhen von oben) füllen. Alle Punkte auf einer Bildhöhe y (unten −1, oben 1) liegen in
    /// einer Ebene durch die Kamera mit der Normalen oben − y·tan(fov/2)·vorn; am höchsten bzw. tiefsten liegt eine
    /// Brettecke, `ahead` weit vor der parallelen Ebene durch die Brettmitte. Damit sind Abstand und Höhe der Kamera
    /// linear bestimmt. Schräg reicht das Brett in Blickrichtung weiter (`reach`), die Kamera rückt also zurück.
    fn frame_board(&self, half: Vec2, (top, bottom): (f32, f32)) -> Vec3 {
        let tan = (self.fov / 2.0).tan();
        let reach = half.x * self.yaw.sin() + half.y * self.yaw.cos();
        let screen_y = |from_top: f32| 1.0 - 2.0 * from_top;
        let ahead = |from_top: f32| {
            reach * (self.pitch.sin() - screen_y(from_top) * tan * self.pitch.cos())
        };
        let distance = (ahead(top) + ahead(bottom)) / (tan * (screen_y(top) - screen_y(bottom)));
        let lift = ahead(top) - distance * screen_y(top) * tan;
        let (forward, up) = self.axes();
        BOARD_CENTER + up * lift - forward * distance
    }

    /// Setzt den Stand zwischen den Ansichten; herangezoomt bleibt der Zoom (`keep_zoom`).
    fn set_blend(&mut self, blend: f32) -> Option<BoardLayout> {
        if blend == self.blend {
            return None;
        }
        let previous_rest = self.rest;
        self.blend = blend;
        let layout = self.apply_view();
        self.keep_zoom(previous_rest);
        Some(layout)
    }

    /// Hält den Zoom, während sich die Ansicht dreht: Herangezoomt steht die Kamera den Anteil `dive` ihrer Höhe weit
    /// auf dem Weg von der Ruhelage zum Brettpunkt, auf den sie zoomt. Punkt und Anteil bleiben, nur die Ruhelage
    /// wandert; tiefer als `ZOOM_MIN_HEIGHT` geht es dabei nicht.
    fn keep_zoom(&mut self, previous_rest: Vec3) {
        let deepest = 1.0 - ZOOM_MIN_HEIGHT / self.rest.y;
        let rest = self.rest;
        for offset in [&mut self.zoom_target, &mut self.zoom_offset] {
            let dive = -offset.y / previous_rest.y;
            if dive <= 0.0 {
                continue;
            }
            let focus = previous_rest + *offset / dive;
            *offset = (focus - rest) * dive.min(deepest);
        }
    }

    /// Dreht die Ansicht, solange eine Richtung gilt, sonst bis zum Ziel der Kamerafahrt; das Tempo läuft kurz an und
    /// aus, an den Ansichten selbst bremst `SineInOut` sanft.
    fn update_view(&mut self, seconds: f32, time_scale: f32) -> Option<BoardLayout> {
        let goal = if self.orbit == 0.0 { self.goal } else { None };
        let direction = goal.map_or(self.orbit, |goal| (goal - self.blend).signum());
        let top_speed = 1.0 / goal.map_or(ORBIT_DURATION, |_| VIEW_SWITCH_DURATION * time_scale);
        let change = top_speed * seconds / ORBIT_RAMP;
        self.blend_speed =
            (direction * top_speed).clamp(self.blend_speed - change, self.blend_speed + change);
        let blend = (self.blend + self.blend_speed * seconds).clamp(0.0, 1.0);
        if blend == 0.0 || blend == 1.0 {
            self.blend_speed = 0.0;
        }
        if Some(blend) == self.goal {
            self.goal = None;
        }
        self.set_blend(blend)
    }

    /// Zwei schnelle Schwingungen und ein langsames Schwanken, die mit der Stärke abklingen.
    fn update_shake(&mut self, scaled_seconds: f32) {
        if self.shake_strength < 0.001 {
            self.shake_strength = 0.0;
            self.shake_offset = Vec2::ZERO;
            return;
        }
        self.shake_time += scaled_seconds;
        let (x, y, sway) = SHAKE_FREQUENCIES;
        let t = self.shake_time;
        self.shake_offset = self.shake_strength
            * Vec2::new(
                0.7 * (t * x).sin() + 0.3 * (t * sway).sin(),
                0.5 * (t * y + 1.3).sin() + 0.3 * (t * sway * 1.4 + 0.7).sin(),
            );
        self.shake_strength *= (-SHAKE_DECAY * scaled_seconds).exp();
    }

    /// Parallaxe: Die Kamera folgt dem Zeiger (Anteil der Fensterbreite und -höhe, Mitte 0) ein Stück; nah am Brett
    /// weniger, weil sie sonst zu unruhig wäre.
    fn update_parallax(&mut self, seconds: f32, pointer: Vec2) {
        let closeness = (self.rest.y + self.zoom_offset.y) / self.rest.y;
        let target = Vec2::new(-pointer.x, pointer.y) * MAX_SCROLL_OFFSET * 2.0 * closeness;
        self.scroll += (target - self.scroll) * (seconds * FOLLOW_SPEED).min(1.0);
    }

    /// Lage der Kamera samt Zoom, Parallaxe und Wackeln.
    fn transform(&self) -> Transform {
        let sideways = self.screen_right() * (self.scroll.x + self.shake_offset.x);
        let lift = Vec3::Y * (self.scroll.y + self.shake_offset.y);
        let (forward, up) = self.axes();
        Transform::from_translation(self.rest + self.zoom_offset + sideways + lift)
            .looking_to(forward, up)
    }
}

/// Weltpunkt → waagerechte Lage im Bild von −1 (linker Rand) bis 1 (rechter Rand), etwa für das Stereopanorama.
pub fn screen_side(camera: &Camera, transform: &GlobalTransform, world: Vec3) -> Option<f32> {
    camera.world_to_ndc(transform, world).map(|ndc| ndc.x)
}

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        let (rig, layout) = CameraRig::from_settings(app.world().resource::<Settings>());
        app.insert_resource(rig)
            .insert_resource(layout)
            .add_message::<CameraCommand>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (handle_commands, update_rig, remember_camera)
                    .chain()
                    .in_set(CameraSystems),
            );
    }
}

/// Stellt die Kamera; wer ihre Lage im selben Bild braucht, ordnet sich danach.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraSystems;

fn spawn_camera(mut commands: Commands, rig: Res<CameraRig>) {
    commands.spawn((
        Name::new("Kamera"),
        MainCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: rig.fov,
            near: CAMERA_NEAR,
            far: CAMERA_FAR,
            ..default()
        }),
        Hdr,
        // Keine Tonkurve: Die Toon-Farben erscheinen genau so, wie sie gerechnet sind.
        Tonemapping::Linear,
        // Nur was heller als 1 ist, glüht: Lichter der Schiffe, glühende Bruchkanten, Effekte.
        Bloom {
            intensity: 0.35,
            low_frequency_boost: 0.4,
            high_pass_frequency: 0.6,
            prefilter: BloomPrefilter {
                threshold: 1.0,
                threshold_softness: 0.3,
            },
            composite_mode: BloomCompositeMode::Additive,
            ..Bloom::NATURAL
        },
        Msaa::Sample4,
        rig.transform(),
    ));
}

fn handle_commands(mut commands: MessageReader<CameraCommand>, mut rig: ResMut<CameraRig>) {
    for command in commands.read() {
        match *command {
            CameraCommand::ToggleView => {
                rig.goal = Some(if rig.goal.unwrap_or(rig.blend) < 0.5 {
                    1.0
                } else {
                    0.0
                })
            }
            CameraCommand::Orbit(direction) => {
                rig.orbit = direction;
                if direction != 0.0 {
                    rig.goal = None;
                }
            }
            CameraCommand::Shake(strength) => rig.shake_strength = rig.shake_strength.max(strength),
        }
    }
}

fn update_rig(
    time: Res<Time>,
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut rig: ResMut<CameraRig>,
    mut layout: ResMut<BoardLayout>,
    camera: Single<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let seconds = time.delta_secs();
    let time_scale = settings.time_scale(AnimationScope::General);
    if let Some(next) = rig.update_view(seconds, time_scale) {
        *layout = next;
    }
    let pointer = window.cursor_position().map_or(Vec2::ZERO, |cursor| {
        (cursor / window.size()).clamp(Vec2::ZERO, Vec2::ONE) - 0.5
    });
    rig.update_parallax(seconds, pointer);
    rig.update_shake(seconds / time_scale);
    let follow = (seconds * ZOOM_FOLLOW_SPEED).min(1.0);
    rig.zoom_offset = rig.zoom_offset.lerp(rig.zoom_target, follow);

    let (mut transform, mut projection) = camera.into_inner();
    *transform = rig.transform();
    if let Projection::Perspective(perspective) = projection.as_mut() {
        perspective.fov = rig.fov;
    }
}

/// Speichert Stand und Zoom fürs nächste Match, sobald sich die Kamera geändert hat und wieder ruht.
fn remember_camera(rig: Res<CameraRig>, mut settings: ResMut<Settings>) {
    if rig.is_turning() || rig.orbit != 0.0 {
        return;
    }
    let camera = crate::settings::CameraSettings {
        view: rig.blend,
        zoom: rig.zoom_target.to_array(),
    };
    if settings.camera != camera {
        settings.camera = camera;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In der Seitenansicht steht die Kamera vor dem Brett (positives z) und über ihm; von hinten oben hinter der
    /// eigenen Seite (negatives x). Die Brettmitte liegt in beiden Fällen waagerecht in der Bildmitte.
    #[test]
    fn views_frame_the_board() {
        let (side, _) = CameraRig::from_settings(&Settings {
            camera: crate::settings::CameraSettings {
                view: 0.0,
                zoom: [0.0; 3],
            },
            ..default()
        });
        assert!(side.rest.z > BOARD_CENTER.z && side.rest.y > 0.0 && side.rest.x.abs() < 1e-4);
        let (top, _) = CameraRig::from_settings(&Settings::default());
        assert!(top.rest.x < 0.0 && top.rest.y > 0.0 && (top.rest.z - BOARD_CENTER.z).abs() < 1e-4);
        let to_center = (BOARD_CENTER - top.rest).normalize();
        assert!(top.transform().forward().dot(to_center) > 0.95);
    }

    #[test]
    fn toggle_drives_to_other_view() {
        let (mut rig, _) = CameraRig::from_settings(&Settings::default());
        rig.goal = Some(0.0);
        for _ in 0..200 {
            rig.update_view(1.0 / 60.0, 1.0);
        }
        assert_eq!(rig.blend, 0.0);
        assert!(!rig.is_turning());
    }
}
