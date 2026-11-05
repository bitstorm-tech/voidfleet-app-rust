//! Eine Einheit als Gruppe von Modellen (Entwurf Abschnitt 3, Spalte „Darstellung“): die Jäger-Staffel als fünf
//! kleine Modelle in 2-1-2, das Kreuzer-Paar als zwei mittlere nebeneinander, Schlachtschiff und Flaggschiffe als eins.
//! Die Einheit bleibt ein [`Ship`] mit Balken und Beschriftung über der Gruppe; ihre Modelle sind [`Hull`]s unter der
//! Gruppe ([`ShipModel`]).
//!
//! Der Verfall folgt der Hülle in [`ShipStatus`] ([`wear_hulls`]): Je 20 % Hülle verschwindet ein Jäger (der zuletzt
//! genannte zuerst; die Wiedergabe lässt ihn dabei klein explodieren), unter der Hälfte brennt der zweite Kreuzer, im
//! Notfall ist er ausgebrannt ([`HullWear::scorched`]: dunkel, Lichter aus, er brennt weiter), Schlachtschiff und
//! Flaggschiff bekommen im Notfall zwei Rumpfbrüche (sprühende Funken). Der Brand einer Einheit ([`ShipAura`]) brennt
//! beim Paar am zweiten Kreuzer statt zwischen beiden.

use bevy::prelude::*;
use voidfleet_core::enemies::EnemyClass;

use super::{
    animation::AnimationSystems,
    aura::ShipAura,
    ship::{Ship, ShipModel, Wreck},
    status::{ShipBounds, ShipStatus},
    sync::SyncSystems,
    toon::{ToonMaterial, look::SCORCHED},
};

/// Staffel in 2-1-2 im Raum des Schiffs (Nase −Z, quer x): einer in der Mitte, je ein Paar davor und dahinter. In
/// dieser Reihenfolge bleiben sie; wer zuletzt steht, verschwindet zuerst.
const FIGHTERS: [Vec3; 5] = [
    Vec3::new(0.0, 0.15, 0.0),
    Vec3::new(-0.9, 0.0, -1.1),
    Vec3::new(0.9, 0.0, -1.1),
    Vec3::new(-0.9, 0.0, 1.1),
    Vec3::new(0.9, 0.0, 1.1),
];
const FIGHTER_SCALE: f32 = 0.42;
/// Paar nebeneinander, leicht gestaffelt; der zweite trägt die Schäden.
const CRUISERS: [Vec3; 2] = [Vec3::new(-0.78, 0.0, -0.45), Vec3::new(0.78, 0.0, 0.45)];
const CRUISER_SCALE: f32 = 0.72;
const SINGLE: [Vec3; 1] = [Vec3::ZERO];

/// Rumpfbrüche: Lage der zwei Funkenquellen als Anteil der Ausdehnung des Modells, von seiner Mitte aus.
const BREACHES: [Vec3; 2] = [Vec3::new(0.22, 0.3, -0.25), Vec3::new(-0.25, 0.25, 0.3)];

/// Maßstab je Modell und Versätze der Modelle einer Klasse im Raum des Schiffs.
pub fn formation(class: EnemyClass) -> (f32, &'static [Vec3]) {
    match class {
        EnemyClass::Fighter => (FIGHTER_SCALE, &FIGHTERS),
        EnemyClass::Cruiser => (CRUISER_SCALE, &CRUISERS),
        EnemyClass::Battleship | EnemyClass::Flagship => (1.0, &SINGLE),
    }
}

/// Wie viele Modelle stehen: bei Jägern aufgerundet Hülle / Maximum × 5, mindestens einer, sonst alle.
pub fn visible_hulls(class: EnemyClass, hull: i32, max_hull: i32) -> usize {
    let count = formation(class).1.len();
    if class != EnemyClass::Fighter || max_hull <= 0 {
        return count;
    }
    let count = count as i32;
    ((hull * count + max_hull - 1) / max_hull).clamp(1, count) as usize
}

/// Was einem Modell der Gruppe anzusehen ist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Damage {
    pub burning: bool,
    pub scorched: bool,
    pub breached: bool,
}

/// Der Verfall des Modells `index` bei dieser Hülle.
pub fn damage(class: EnemyClass, index: usize, hull: i32, max_hull: i32) -> Damage {
    let half = hull * 2 <= max_hull;
    let emergency = hull * 4 <= max_hull;
    match class {
        EnemyClass::Cruiser if index == 1 => Damage {
            burning: half,
            scorched: emergency,
            breached: false,
        },
        EnemyClass::Battleship | EnemyClass::Flagship => Damage {
            breached: emergency && hull > 0,
            ..default()
        },
        _ => Damage::default(),
    }
}

/// Ein Modell einer Gruppe, Träger seiner glTF-Szene (Modell oder Wrack).
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Visibility, HullWear, ShipAura)]
pub struct Hull {
    /// Platz in der Formation.
    pub index: usize,
}

/// Was die Materialien eines Modells über den Zustand des Schiffs hinaus zeigen.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HullWear {
    /// Ausgebrannt: dunkel, Lichter aus.
    pub scorched: bool,
    /// Treffer-Blitz nur an diesem Modell (rgb linear, a Anteil).
    pub flash: Vec4,
}

impl HullWear {
    pub fn apply(&self, material: &mut ToonMaterial) {
        if self.scorched {
            material.lights = 0.0;
            if material.overlay.w < SCORCHED.w {
                material.overlay = SCORCHED;
            }
        }
        if self.flash.w > material.overlay.w {
            material.overlay = self.flash;
        }
    }
}

/// Funkenquelle eines Rumpfbruchs, ein Kind der Gruppe.
#[derive(Component, Debug)]
struct Breach;

pub(super) struct FormationPlugin;

impl Plugin for FormationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            wear_hulls.after(AnimationSystems::Apply).after(SyncSystems),
        );
    }
}

/// Stellt die Modelle einer Gruppe nach der Hülle: sichtbar oder nicht, Brand, ausgebrannt, Rumpfbrüche.
fn wear_hulls(
    mut commands: Commands,
    ships: Query<
        (
            &Ship,
            &ShipStatus,
            Option<&ShipAura>,
            &Children,
            Option<&ShipBounds>,
        ),
        Or<(Changed<ShipStatus>, Changed<ShipAura>, Added<ShipBounds>)>,
    >,
    groups: Query<&Children, (With<ShipModel>, Without<Wreck>)>,
    mut hulls: Query<(&Hull, &mut Visibility, &mut HullWear, &mut ShipAura), Without<Ship>>,
    breaches: Query<(), With<Breach>>,
) {
    for (ship, status, ship_aura, children, bounds) in &ships {
        let class = ship.kind.class();
        let Some((group, members)) = children
            .iter()
            .find_map(|child| groups.get(child).ok().map(|members| (child, members)))
        else {
            continue;
        };
        let standing = visible_hulls(class, status.hull, status.max_hull);
        let mut breached = false;
        for member in members.iter() {
            let Ok((hull, mut visibility, mut wear, mut aura)) = hulls.get_mut(member) else {
                continue;
            };
            let damage = damage(class, hull.index, status.hull, status.max_hull);
            breached |= damage.breached;
            visibility.set_if_neq(if hull.index < standing {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            if wear.scorched != damage.scorched {
                wear.scorched = damage.scorched;
            }
            aura.set_if_neq(ShipAura {
                burning: damage.burning
                    || (class == EnemyClass::Cruiser
                        && hull.index == 1
                        && ship_aura.is_some_and(|aura| aura.burning)),
                ..default()
            });
        }
        let present: Vec<Entity> = members
            .iter()
            .filter(|member| breaches.contains(*member))
            .collect();
        match (breached, bounds) {
            (true, Some(bounds)) if present.is_empty() => {
                let center = (bounds.min + bounds.max) / 2.0;
                let size = bounds.max - bounds.min;
                for at in BREACHES {
                    commands.spawn((
                        Name::new("Rumpfbruch"),
                        Breach,
                        ShipAura {
                            breached: true,
                            ..default()
                        },
                        Transform::from_translation(center + at * size),
                        ChildOf(group),
                    ));
                }
            }
            (false, _) => {
                for breach in present {
                    commands.entity(breach).despawn();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{
        coords::{PAD_DEPTH, PAD_WIDTH},
        ship::BOARD_SCALE,
    };

    /// 5, 2 und 1 Modelle, alle Mitten auf dem Pad (Nase entlang der Padbreite, quer seine Tiefe).
    #[test]
    fn formations_fit_the_pad() {
        for (class, count) in [
            (EnemyClass::Fighter, 5),
            (EnemyClass::Cruiser, 2),
            (EnemyClass::Battleship, 1),
            (EnemyClass::Flagship, 1),
        ] {
            let (scale, offsets) = formation(class);
            assert_eq!(offsets.len(), count);
            assert!(scale > 0.0 && scale <= 1.0);
            for offset in offsets {
                let world = *offset * BOARD_SCALE;
                assert!(world.x.abs() < PAD_DEPTH / 2.0, "{class:?} {offset}");
                assert!(world.z.abs() < PAD_WIDTH / 2.0, "{class:?} {offset}");
            }
        }
    }

    #[test]
    fn fighters_vanish_by_fifths() {
        let fighter = |hull| visible_hulls(EnemyClass::Fighter, hull, 10);
        assert_eq!(fighter(10), 5);
        assert_eq!(fighter(9), 5);
        assert_eq!(fighter(8), 4);
        assert_eq!(fighter(3), 2);
        assert_eq!(fighter(2), 1);
        assert_eq!(fighter(1), 1);
        assert_eq!(fighter(0), 1);
        assert_eq!(visible_hulls(EnemyClass::Cruiser, 1, 16), 2);
        assert_eq!(visible_hulls(EnemyClass::Battleship, 1, 30), 1);
    }

    #[test]
    fn cruiser_burns_then_wrecks_one() {
        let cruiser = |index, hull| damage(EnemyClass::Cruiser, index, hull, 16);
        assert_eq!(cruiser(1, 9), Damage::default());
        assert!(cruiser(1, 8).burning && !cruiser(1, 8).scorched);
        assert!(cruiser(1, 4).burning && cruiser(1, 4).scorched);
        assert_eq!(cruiser(0, 1), Damage::default());
        assert!(!damage(EnemyClass::Battleship, 0, 8, 30).breached);
        assert!(damage(EnemyClass::Battleship, 0, 7, 30).breached);
        assert!(!damage(EnemyClass::Battleship, 0, 0, 30).breached);
    }
}
