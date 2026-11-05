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
//!
//! Maus wie im Original ([`CameraInput`]): Mausrad zoomt auf den Brettpunkt unter dem Zeiger, die mittlere Maustaste
//! dreht (das Brett folgt dem Zeiger wie gegriffen), herangezoomt verschiebt Ziehen mit der linken Taste das Brett.
//! Über der Oberfläche reagiert die Kamera nicht auf die Maus. Die Tasten (V, ← →, ↑ ↓) übersetzt
//! [`crate::ui::view_toggle`] in [`CameraCommand`]s, damit offene Dialoge sie behalten.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    camera::Hdr,
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    picking::{hover::HoverMap, mesh_picking::MeshPickingCamera, pointer::PointerId},
    post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
    prelude::*,
    render::view::{NoIndirectDrawing, Tonemapping},
    window::PrimaryWindow,
};

use super::{
    animation::AnimationScope,
    coords::{BOARD_CENTER, PAD_DEPTH, PAD_WIDTH, slot_z},
};
use voidfleet_core::board::{LaneType, Side};

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
/// Kamera-Parallaxe: Reichweite in Welteinheiten und Nachlauf (Abklingrate pro Sekunde).
const MAX_SCROLL_OFFSET: Vec2 = Vec2::new(0.75, 0.4);
const FOLLOW_SPEED: f32 = 4.0;
/// Zoom: Nachlauf des geglätteten Versatzes (Abklingrate pro Sekunde) und die tiefste Kamerahöhe.
const ZOOM_FOLLOW_SPEED: f32 = 8.0;
const ZOOM_MIN_HEIGHT: f32 = 2.6;
/// Höhenfaktor pro Pixel Mausrad (exponentiell, damit Trackpads und Rasterräder gleich wirken), Pixel je Zeile, und
/// pro Sekunde mit ↑ ↓.
const ZOOM_SPEED: f32 = 0.0018;
const LINE_PIXELS: f32 = 16.0;
const ZOOM_KEY_SPEED: f32 = 1.4;
/// Brettbereich, auf den gezoomt und die Kamera ausgerichtet werden kann (Welt-x und Abstand in z von der Brettmitte).
const ZOOM_FOCUS_X: f32 = 7.5;
const ZOOM_FOCUS_Z: f32 = 7.5;
/// Ab dieser Zeigerbewegung (logische Pixel) wird aus einem Klick ein Verschieben der Kamera.
const PAN_THRESHOLD: f32 = 5.0;
/// Mit gedrückter mittlerer Maustaste dreht so viel Fensterbreite die Kamera von einer Ansicht zur anderen.
const ORBIT_DRAG_WIDTH: f32 = 0.5;
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
    /// Zoomt stetig auf die Mitte des Brettbands zu bzw. zurück, solange die Richtung nicht 0 ist: 1 hinein, −1 heraus.
    Zoom(f32),
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
    /// Richtung einer stufenlosen Drehung und des stetigen Zooms.
    orbit: f32,
    zoom_direction: f32,
    /// Ruhelage und Ausrichtung beim jetzigen Stand.
    rest: Vec3,
    yaw: f32,
    pitch: f32,
    fov: f32,
    /// Bildhöhe (von oben) der Mitte des Bands, das die Pads in der Ruhelage füllen; dorthin zoomen ↑ ↓.
    band_middle: f32,
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

    /// Stand und Zoom aus den Einstellungen samt der Aufteilung des Bretts dazu.
    fn from_settings(settings: &Settings) -> (Self, BoardLayout) {
        let zoom = Vec3::from_array(settings.camera.zoom);
        let mut rig = Self {
            blend: settings.camera.view.clamp(0.0, 1.0),
            blend_speed: 0.0,
            goal: None,
            orbit: 0.0,
            zoom_direction: 0.0,
            rest: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov: SIDE_VIEW.fov,
            band_middle: 0.5,
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
        self.band_middle = (band.0 + band.1) / 2.0;
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
        if self.zoom_target.y < 0.0 {
            self.clamp_zoom_target();
        }
    }

    /// Ändert die Kamerahöhe um `factor`: kleiner zoomt auf den Brettpunkt `focus` zu (der unter dem Zeiger stehen
    /// bleibt), größer zurück in Richtung Ruhelage.
    fn zoom(&mut self, factor: f32, focus: Option<Vec3>) {
        let rest = self.rest;
        let height = rest.y + self.zoom_target.y;
        if factor < 1.0 {
            let Some(focus) = focus.map(clamp_focus) else {
                return;
            };
            let next_height = (height * factor).max(ZOOM_MIN_HEIGHT);
            // Auf der Geraden Kamera → Fokus bleibt der Punkt unter dem Zeiger stehen.
            let camera = rest + self.zoom_target;
            let next = focus.lerp(camera, (next_height - focus.y) / (camera.y - focus.y));
            self.zoom_target = next - rest;
            self.clamp_zoom_target();
        } else if height < rest.y {
            // Herauszoomen schrumpft den Versatz, bis die Kamera wieder in der Ruhelage steht.
            let next_height = (height * factor).min(rest.y);
            self.zoom_target *= (rest.y - next_height) / (rest.y - height);
        }
    }

    /// Verschiebt herangezoomt das Brett: Der Brettpunkt `from` wandert nach `to`, ohne Nachlauf.
    fn pan(&mut self, from: Vec3, to: Vec3) {
        let before = self.zoom_target;
        self.zoom_target.x += from.x - to.x;
        self.zoom_target.z += from.z - to.z;
        self.clamp_zoom_target();
        self.zoom_offset += self.zoom_target - before;
    }

    /// Hält den Blickpunkt der Kamera (Bildmitte auf dem Brett) im Brettbereich.
    fn clamp_zoom_target(&mut self) {
        let camera = self.rest + self.zoom_target;
        let look_ahead = camera.y / self.pitch.tan();
        let focus = Vec3::new(
            camera.x + self.yaw.sin() * look_ahead,
            0.0,
            camera.z - self.yaw.cos() * look_ahead,
        );
        let clamped = clamp_focus(focus);
        self.zoom_target.x += clamped.x - focus.x;
        self.zoom_target.z += clamped.z - focus.z;
    }

    /// Dreht mit der Maus: Das Brett folgt dem Zeiger, nach rechts also Richtung hinten oben; der Winkel folgt der Maus
    /// gleichmäßig, ohne das Bremsen von `SineInOut` an den Ansichten.
    fn drag_orbit(&mut self, dx: f32, width: f32) -> Option<BoardLayout> {
        let eased = EaseFunction::SineInOut.sample_clamped(self.blend);
        let progress = (eased + dx / (width * ORBIT_DRAG_WIDTH)).clamp(0.0, 1.0);
        self.set_blend((1.0 - 2.0 * progress).acos() / PI)
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
        self.scroll.smooth_nudge(&target, FOLLOW_SPEED, seconds);
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

/// Die Kamera beginnt beim gespeicherten Stand und Zoom.
impl FromWorld for CameraRig {
    fn from_world(world: &mut World) -> Self {
        Self::from_settings(world.resource::<Settings>()).0
    }
}

/// Die Aufteilung beim gespeicherten Stand der Kamera.
impl FromWorld for BoardLayout {
    fn from_world(world: &mut World) -> Self {
        CameraRig::from_settings(world.resource::<Settings>()).1
    }
}

/// Begrenzt einen Brettpunkt auf den Bereich, auf den gezoomt werden kann.
fn clamp_focus(point: Vec3) -> Vec3 {
    Vec3::new(
        point.x.clamp(-ZOOM_FOCUS_X, ZOOM_FOCUS_X),
        point.y,
        point
            .z
            .clamp(BOARD_CENTER.z - ZOOM_FOCUS_Z, BOARD_CENTER.z + ZOOM_FOCUS_Z),
    )
}

/// Schnittpunkt des Strahls durch einen Bildpunkt (logische Pixel) mit der Brettebene.
pub fn board_point(camera: &Camera, transform: &GlobalTransform, cursor: Vec2) -> Option<Vec3> {
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    let direction = ray.direction.as_vec3();
    (direction.y < 0.0).then(|| ray.origin + direction * (-ray.origin.y / direction.y))
}

/// Gedrückte Maustasten über dem Brett: links klickt oder verschiebt, die mittlere dreht.
#[derive(Resource, Debug, Default)]
pub struct CameraInput {
    /// Linke Taste: Startpunkt, letzter Punkt und ob schon verschoben wird.
    drag: Option<(Vec2, Vec2, bool)>,
    /// Ob die letzte Geste mit gedrückter linker Taste die Kamera verschoben hat statt zu klicken.
    panned: bool,
    /// Mittlere Taste: letzte Zeiger-x, solange damit gedreht wird.
    orbit_x: Option<f32>,
}

impl CameraInput {
    /// Die letzte Geste hat verschoben: Ein Klick zählt dann nicht.
    pub fn panned(&self) -> bool {
        self.panned
    }

    fn is_dragging(&self) -> bool {
        self.drag.is_some_and(|(_, _, panning)| panning) || self.orbit_x.is_some()
    }
}

/// Weltpunkt → waagerechte Lage im Bild von −1 (linker Rand) bis 1 (rechter Rand), etwa für das Stereopanorama.
pub fn screen_side(camera: &Camera, transform: &GlobalTransform, world: Vec3) -> Option<f32> {
    camera.world_to_ndc(transform, world).map(|ndc| ndc.x)
}

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraRig>()
            .init_resource::<BoardLayout>()
            .init_resource::<CameraInput>()
            .add_message::<CameraCommand>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (read_mouse, handle_commands, update_rig, remember_camera)
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
        // Fast jedes Schiffsteil hat ein eigenes Toon-Material, Multidraw bündelt also nichts; mit GPU-Culling schriebe
        // Bevy trotzdem jedes Bild je Material Puffer und Bind-Groups neu (gemessen: ~10 ms Render-Thread je Bild).
        NoIndirectDrawing,
        MeshPickingCamera,
        rig.transform(),
    ));
}

/// Mausrad, Verschieben und Drehen mit der Maus, solange der Zeiger nicht über der Oberfläche liegt.
fn read_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    hover_map: Res<HoverMap>,
    nodes: Query<(), With<Node>>,
    mut input: ResMut<CameraInput>,
    mut rig: ResMut<CameraRig>,
    mut layout: ResMut<BoardLayout>,
) {
    let (camera, camera_transform) = *camera;
    let cursor = window.cursor_position();
    let over_ui = hover_map
        .get(&PointerId::Mouse)
        .is_some_and(|hits| hits.keys().any(|entity| nodes.contains(*entity)));

    let pixels = match scroll.unit {
        MouseScrollUnit::Line => -scroll.delta.y * LINE_PIXELS,
        MouseScrollUnit::Pixel => -scroll.delta.y,
    };
    if pixels != 0.0 && !over_ui {
        let focus = cursor.and_then(|cursor| board_point(camera, camera_transform, cursor));
        rig.zoom((pixels * ZOOM_SPEED).exp(), focus);
    }

    if let Some(cursor) = cursor
        && !over_ui
    {
        if buttons.just_pressed(MouseButton::Left) {
            input.drag = Some((cursor, cursor, false));
            input.panned = false;
        }
        if buttons.just_pressed(MouseButton::Middle) {
            input.orbit_x = Some(cursor.x);
            rig.goal = None;
            rig.blend_speed = 0.0;
        }
    }
    if buttons.just_released(MouseButton::Left) {
        input.drag = None;
    }
    if buttons.just_released(MouseButton::Middle) {
        input.orbit_x = None;
    }
    let Some(cursor) = cursor else {
        return;
    };
    if let Some(last) = input.orbit_x {
        if let Some(next) = rig.drag_orbit(cursor.x - last, window.width()) {
            *layout = next;
        }
        input.orbit_x = Some(cursor.x);
        return;
    }
    // Verschieben gibt es nur herangezoomt; in der Ruhelage ist das ganze Brett zu sehen.
    let Some((start, last, panning)) = input.drag else {
        return;
    };
    if rig.zoom_target.y >= 0.0 {
        return;
    }
    if !panning && cursor.distance(start) < PAN_THRESHOLD {
        return;
    }
    input.drag = Some((start, cursor, true));
    input.panned = true;
    // Der Brettpunkt unter dem Zeiger wandert mit, als würde man das Brett greifen.
    if let (Some(from), Some(to)) = (
        board_point(camera, camera_transform, last),
        board_point(camera, camera_transform, cursor),
    ) {
        rig.pan(from, to);
    }
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
            CameraCommand::Zoom(direction) => rig.zoom_direction = direction,
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
    camera: Single<(&Camera, &GlobalTransform, &mut Transform, &mut Projection), With<MainCamera>>,
) {
    let seconds = time.delta_secs();
    let time_scale = settings.time_scale(AnimationScope::General);
    if let Some(next) = rig.update_view(seconds, time_scale) {
        *layout = next;
    }
    let (camera, camera_transform, mut transform, mut projection) = camera.into_inner();
    if rig.zoom_direction != 0.0 {
        let factor = (-rig.zoom_direction * ZOOM_KEY_SPEED * seconds).exp();
        let middle = Vec2::new(window.width() / 2.0, window.height() * rig.band_middle);
        let focus = board_point(camera, camera_transform, middle);
        rig.zoom(factor, focus);
    }
    let pointer = window.cursor_position().map_or(Vec2::ZERO, |cursor| {
        (cursor / window.size()).clamp(Vec2::ZERO, Vec2::ONE) - 0.5
    });
    rig.update_parallax(seconds, pointer);
    rig.update_shake(seconds / time_scale);
    let zoom_target = rig.zoom_target;
    rig.zoom_offset
        .smooth_nudge(&zoom_target, ZOOM_FOLLOW_SPEED, seconds);

    *transform = rig.transform();
    if let Projection::Perspective(perspective) = projection.as_mut() {
        perspective.fov = rig.fov;
    }
}

/// Speichert Stand und Zoom fürs nächste Match, sobald sich die Kamera geändert hat und wieder ruht: nicht während einer
/// Fahrt oder Drehung, solange ↑ oder ↓ gedrückt ist und solange das Brett verschoben oder mit der Maus gedreht wird.
fn remember_camera(rig: Res<CameraRig>, input: Res<CameraInput>, mut settings: ResMut<Settings>) {
    if rig.is_turning() || rig.orbit != 0.0 || rig.zoom_direction != 0.0 || input.is_dragging() {
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

    /// Hineinzoomen senkt die Kamera auf den Brettpunkt zu, Herauszoomen bringt sie zurück in die Ruhelage; verschoben
    /// wird nur innerhalb des Brettbereichs.
    #[test]
    fn zoom_and_pan_stay_on_board() {
        let (mut rig, _) = CameraRig::from_settings(&Settings::default());
        rig.zoom_target = Vec3::ZERO;
        rig.zoom(0.5, Some(BOARD_CENTER + Vec3::X * 2.0));
        assert!(rig.zoom_target.y < 0.0);
        assert!(rig.rest.y + rig.zoom_target.y >= ZOOM_MIN_HEIGHT - 1e-4);
        rig.pan(Vec3::ZERO, Vec3::new(-100.0, 0.0, 0.0));
        let camera = rig.rest + rig.zoom_target;
        let look_ahead = camera.y / rig.pitch.tan();
        assert!(camera.x + rig.yaw.sin() * look_ahead <= ZOOM_FOCUS_X + 1e-3);
        rig.zoom(100.0, None);
        assert!(rig.zoom_target.length() < 1e-4);
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
