//! Flugbahn eines Slot-Wechsels (Port von `scene/maneuver.ts`). Schiffe fliegen nur vorwärts und stehen vorher wie
//! nachher mit der Nase zum Gegner: Nach vorn (Command → Strike) schwenken sie in einer S-Kurve in den neuen Slot, nach
//! hinten (Strike → Command) fliegen sie eine Schleife — erst vom Ziel weg nach außen herum, dann von hinten in den
//! Slot. Innerhalb der Lane versetzen sie seitlich, mit der Nase nach vorn und Schräglage in Flugrichtung.
//!
//! Gerechnet wird im Raum des Ziel-Ankers wie im Original (x seitlich, `ahead` nach vorn, Ziel im Ursprung); nach außen
//! gibt [`Flight::pose`] die Lage im Raum des Ankers in Bevy-Koordinaten (Nase nach −Z) zurück. Wie wendig ein Schiff
//! ist, legt seine Klasse fest; vom Traktorstrahl gezogen gleitet es langsamer, flacher und fast ohne Schräglage.

use bevy::prelude::*;
use voidfleet_core::cards::ShipClass;

#[derive(Clone, Copy, Debug)]
struct Handling {
    /// Reisegeschwindigkeit in Welteinheiten pro Sekunde.
    speed: f32,
    /// Wenderadius der Schleife.
    turn_radius: f32,
    /// Anteil der Flugzeit, der jeweils auf Beschleunigen und Abbremsen entfällt.
    acceleration: f32,
    /// Größte Schräglage in Kurven.
    bank: f32,
}

const fn handling(class: ShipClass) -> Handling {
    let (speed, turn_radius, acceleration, bank) = match class {
        ShipClass::Fighter => (9.0, 0.9, 0.2, 60.0),
        ShipClass::Support => (6.5, 1.2, 0.28, 32.0),
        ShipClass::Cruiser => (5.5, 1.5, 0.32, 22.0),
        ShipClass::Battleship => (4.0, 1.7, 0.42, 10.0),
    };
    Handling {
        speed,
        turn_radius,
        acceleration,
        bank: bank * std::f32::consts::PI / 180.0,
    }
}

/// Gezogen: Anteile von Tempo, Schräglage und Flughöhe, dazu ein gleichmäßigeres Anfahren und Abbremsen.
const TOWED_SPEED: f32 = 0.5;
const TOWED_BANK: f32 = 0.2;
const TOWED_HEIGHT: f32 = 0.5;
const TOWED_ACCELERATION: f32 = 0.45;
/// Die Schleife führt über andere Schiffe hinweg, die S-Kurve steigt je nach Seitenversatz leicht.
const LOOP_HEIGHT: f32 = 1.6;
const SWERVE_HEIGHT_PER_UNIT: f32 = 0.25;
const SWERVE_MAX_HEIGHT: f32 = 1.0;
/// Die Wende in den Slot ist enger als die erste, damit die Schleife hinter der Command-Lane im Bild bleibt.
const FINAL_TURN: f32 = 0.7;
/// Schräglage beim seitlichen Versetzen, relativ zur größten Schräglage der Klasse.
const STRAFE_BANK: f32 = 0.6;
/// Bis zu diesem Längsabstand zum Ziel gilt ein Wechsel als innerhalb der Lane.
const SAME_LANE_TOLERANCE: f32 = 0.01;
/// Bézier-Hebel für einen Viertelkreis, relativ zum Radius.
const QUARTER_CIRCLE: f32 = 0.5523;
const SAMPLES_PER_SEGMENT: usize = 48;
/// Die Schräglage folgt der über diese Strecke gemittelten Krümmung.
const BANK_SMOOTHING: f32 = 0.8;

/// Punkt in der Ebene: x seitlich, y nach vorn (zum Gegner).
type Point = Vec2;
type Segment = [Point; 4];

/// Lage des Schiffs im Raum des Ziel-Ankers: Lage und Kurs der Wurzel, Neigung und Schräglage des Modells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub translation: Vec3,
    /// Drehung der Wurzel um die Hochachse.
    pub heading: Quat,
    /// Neigung und Schräglage des Modells.
    pub attitude: Quat,
}

/// Eine abgetastete Flugbahn.
#[derive(Clone, Debug)]
pub struct Flight {
    /// Flugdauer in Sekunden.
    pub duration: f32,
    handling: Handling,
    height: f32,
    /// Seitliches Versetzen innerhalb der Lane: −1 nach links, 1 nach rechts, sonst 0.
    strafe: f32,
    points: Vec<Point>,
    distances: Vec<f32>,
    yaws: Vec<f32>,
    curvatures: Vec<f32>,
}

impl Flight {
    /// Bahn von `from` (Lage im Raum des Ziel-Ankers, Bevy-Koordinaten) in den Ursprung.
    pub fn new(from: Vec3, class: ShipClass, towed: bool) -> Self {
        let base = handling(class);
        let handling = if towed {
            Handling {
                speed: base.speed * TOWED_SPEED,
                bank: base.bank * TOWED_BANK,
                acceleration: TOWED_ACCELERATION,
                ..base
            }
        } else {
            base
        };
        // Nach vorn ist −Z in Bevy.
        let start = Point::new(from.x, -from.z);
        let same_lane = start.y.abs() < SAME_LANE_TOLERANCE;
        let is_loop = !same_lane && start.y > 0.0;
        let strafe = if same_lane { -start.x.signum() } else { 0.0 };
        let segments = if same_lane {
            sidestep(start)
        } else if is_loop {
            looping(start, handling.turn_radius)
        } else {
            swerve(start)
        };
        let height = if is_loop {
            LOOP_HEIGHT
        } else {
            (SWERVE_HEIGHT_PER_UNIT * start.x.abs()).min(SWERVE_MAX_HEIGHT)
        } * if towed { TOWED_HEIGHT } else { 1.0 };
        let mut flight = Self {
            duration: 0.0,
            handling,
            height,
            strafe,
            points: Vec::new(),
            distances: Vec::new(),
            yaws: Vec::new(),
            curvatures: Vec::new(),
        };
        flight.sample(&segments);
        flight.duration = flight.length() / handling.speed / (1.0 - handling.acceleration);
        flight
    }

    fn length(&self) -> f32 {
        *self.distances.last().expect("abgetastete Bahn")
    }

    /// Lage nach dem Anteil `progress` der Flugzeit.
    pub fn pose(&self, progress: f32) -> Pose {
        let (travelled, speed) = self.travel(progress);
        let length = self.length().max(1e-6);
        let distance = travelled * length;
        let i = self.index_at(distance);
        let span = self.distances[i + 1] - self.distances[i];
        let f = if span > 0.0 {
            (distance - self.distances[i]) / span
        } else {
            0.0
        };
        let point = self.points[i].lerp(self.points[i + 1], f);
        let yaw = if self.strafe != 0.0 {
            0.0
        } else {
            lerp_angle(self.yaws[i], self.yaws[i + 1], f)
        };
        let curvature = self.curvatures[i] + (self.curvatures[i + 1] - self.curvatures[i]) * f;

        let phase = std::f32::consts::PI * distance / length;
        let y = self.height * phase.sin().powi(2);
        let climb = self.height * std::f32::consts::PI / length * (2.0 * phase).sin();
        // Schräglage braucht Fahrt: Sie wächst mit Krümmung und Geschwindigkeit; Rechtskurve = rechte Seite runter.
        let turn = if self.strafe != 0.0 {
            STRAFE_BANK * self.strafe
        } else {
            (curvature * self.handling.turn_radius).clamp(-1.0, 1.0)
        };
        let roll = -self.handling.bank * turn * speed;
        // Im Original (linkshändig, Nase +z) dreht ein positiver Kurs nach rechts und eine positive Neigung die Nase
        // nach unten; in Bevy ist beides umgekehrt, die Schräglage bleibt.
        Pose {
            translation: Vec3::new(point.x, y, -point.y),
            heading: Quat::from_rotation_y(-yaw),
            attitude: Quat::from_euler(EulerRot::YXZ, 0.0, climb.atan(), roll),
        }
    }

    /// Trapez-Profil: beschleunigen, mit Reisegeschwindigkeit fliegen, abbremsen. Liefert die zurückgelegte Strecke
    /// (0..1) und die Geschwindigkeit relativ zur Reisegeschwindigkeit.
    fn travel(&self, progress: f32) -> (f32, f32) {
        let a = self.handling.acceleration;
        let top = 1.0 / (1.0 - a);
        if progress < a {
            (top * progress * progress / (2.0 * a), progress / a)
        } else if progress > 1.0 - a {
            (
                1.0 - top * (1.0 - progress).powi(2) / (2.0 * a),
                (1.0 - progress) / a,
            )
        } else {
            (top * (progress - a / 2.0), 1.0)
        }
    }

    fn index_at(&self, distance: f32) -> usize {
        let last = self.distances.len() - 2;
        self.distances[..=last]
            .partition_point(|&travelled| travelled <= distance)
            .saturating_sub(1)
            .min(last)
    }

    fn sample(&mut self, segments: &[Segment]) {
        let path = CubicBezier::new(segments.iter().copied())
            .to_curve()
            .expect("Bahn aus mindestens einem Stück");
        self.points = path
            .iter_positions(segments.len() * SAMPLES_PER_SEGMENT)
            .collect();
        let mut travelled = 0.0;
        for i in 0..self.points.len() {
            if i > 0 {
                travelled += self.points[i - 1].distance(self.points[i]);
            }
            self.distances.push(travelled);
        }
        // Kurs aus der Richtung zum Nachbarpunkt; Start und Ziel liegen exakt in Flugrichtung.
        let count = self.points.len();
        for i in 0..count {
            let a = self.points[i.saturating_sub(1)];
            let b = self.points[(i + 1).min(count - 1)];
            self.yaws.push((b.x - a.x).atan2(b.y - a.y));
        }
        self.yaws[0] = 0.0;
        self.yaws[count - 1] = 0.0;
        let raw: Vec<f32> = (0..count)
            .map(|i| {
                if i == 0 {
                    return 0.0;
                }
                let step = self.distances[i] - self.distances[i - 1];
                if step > 0.0 {
                    wrap_angle(self.yaws[i] - self.yaws[i - 1]) / step
                } else {
                    0.0
                }
            })
            .collect();
        // Gleitender Mittelwert der Krümmung über die Strecke.
        for i in 0..count {
            let window = (0..count)
                .filter(|&j| (self.distances[i] - self.distances[j]).abs() <= BANK_SMOOTHING / 2.0);
            let (sum, n) = window.fold((0.0, 0), |(sum, n), j| (sum + raw[j], n + 1));
            self.curvatures.push(sum / n as f32);
        }
    }
}

/// Nach vorn: eine S-Kurve, die geradeaus los- und geradeaus ankommt.
fn swerve(start: Point) -> Vec<Segment> {
    let lever = -start.y / 2.0;
    vec![[
        start,
        Point::new(start.x, start.y + lever),
        Point::new(0.0, -lever),
        Point::ZERO,
    ]]
}

/// Innerhalb der Lane: geradewegs seitlich in den Slot.
fn sidestep(start: Point) -> Vec<Segment> {
    vec![[
        start,
        Point::new(start.x * 2.0 / 3.0, 0.0),
        Point::new(start.x / 3.0, 0.0),
        Point::ZERO,
    ]]
}

/// Nach hinten: Halbkreis vom Ziel weg nach außen, zurück hinter die Ziel-Lane und mit einem zweiten Halbkreis von
/// hinten in den Slot. Bei gleicher Slot-Höhe wendet das Schiff zur +x-Seite.
fn looping(start: Point, radius: f32) -> Vec<Segment> {
    let side = if start.x == 0.0 {
        1.0
    } else {
        start.x.signum()
    };
    let k = QUARTER_CIRCLE * radius;
    let r = FINAL_TURN * radius;
    let kr = QUARTER_CIRCLE * r;
    let apex = Point::new(start.x + side * radius, start.y + radius);
    let turned = Point::new(start.x + 2.0 * side * radius, start.y);
    let behind = Point::new(2.0 * side * r, 0.0);
    let bottom = Point::new(side * r, -r);
    let lever = turned.distance(behind) * 0.4;
    vec![
        [
            start,
            Point::new(start.x, start.y + k),
            Point::new(apex.x - side * k, apex.y),
            apex,
        ],
        [
            apex,
            Point::new(apex.x + side * k, apex.y),
            Point::new(turned.x, turned.y + k),
            turned,
        ],
        [
            turned,
            Point::new(turned.x, turned.y - lever),
            Point::new(behind.x, behind.y + lever),
            behind,
        ],
        [
            behind,
            Point::new(behind.x, -kr),
            Point::new(bottom.x + side * kr, bottom.y),
            bottom,
        ],
        [
            bottom,
            Point::new(bottom.x - side * kr, bottom.y),
            Point::new(0.0, -kr),
            Point::ZERO,
        ],
    ]
}

fn wrap_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

fn lerp_angle(a: f32, b: f32, f: f32) -> f32 {
    a + wrap_angle(b - a) * f
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jede Bahn beginnt am Start, endet im Slot und steht dort mit der Nase nach vorn.
    #[test]
    fn flights_end_in_slot() {
        for from in [
            Vec3::new(3.4, 0.0, 0.0),
            Vec3::new(-2.2, 0.0, 3.4),
            Vec3::new(2.2, 0.0, -3.4),
        ] {
            let flight = Flight::new(from, ShipClass::Cruiser, false);
            assert!(flight.duration > 0.0);
            assert!(flight.pose(0.0).translation.distance(from) < 1e-3);
            let end = flight.pose(1.0);
            assert!(
                end.translation.length() < 1e-3,
                "{from} → {}",
                end.translation
            );
            assert!(end.heading.angle_between(Quat::IDENTITY) < 1e-3);
        }
    }

    /// Nach hinten (der Start liegt vor dem Ziel, also bei −Z) fliegt die Schleife höher als eine S-Kurve nach vorn.
    #[test]
    fn loop_flies_higher() {
        let back = Flight::new(Vec3::new(0.0, 0.0, -3.4), ShipClass::Fighter, false);
        let forward = Flight::new(Vec3::new(0.0, 0.0, 3.4), ShipClass::Fighter, false);
        assert!(back.height > forward.height);
        let towed = Flight::new(Vec3::new(0.0, 0.0, 3.4), ShipClass::Fighter, true);
        assert!(towed.duration > forward.duration);
    }
}
