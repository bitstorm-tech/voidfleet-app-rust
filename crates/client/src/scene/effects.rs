//! Kurzlebige Effekte auf dem Brett (Port von `scene/effects.ts`): Bolzen (auch verpuffende), die Zielmarke eines
//! umgelenkten Schusses, der Strahl der Railgun, der stehende Laserstrahl, die Rakete des Wärmesuchers mit Leuchtspur,
//! der Traktorstrahl, Impulse, Einschläge in fünf Varianten, Sekundärexplosionen, Detonationen, brennende Trümmer und
//! der Austritt eines Schiffs aus dem Hyperraum, dazu das eine Blitzlicht der Szene ([`ToonFlash`], immer der stärkste laufende
//! Blitz). Alle räumen sich nach dem Abspielen selbst weg.
//!
//! Jeder Effekt ist ein Auftrag ([`crate::scene::animation`]): Man spawnt seine Komponente und wartet, wenn nötig, auf
//! [`crate::scene::animation::AnimationFinished`]. Wie im Original endet ein Einschlag mit Blitz und Druckwelle und eine
//! Detonation mit ihrem Kern; Partikel, Rauch und Nachglühen laufen für sich weiter. Die Sounds spielen dort, wo das
//! Original sie auslöst: Einschläge je Variante, Sekundärexplosionen und Detonationen.
//!
//! Glühen: Was im Original im GlowLayer lag, leuchtet hier heller als 1 und glüht im Bloom ([`GLOW`]).

use std::f32::consts::PI;

use bevy::{ecs::system::SystemParam, prelude::*, world_serialization::WorldAssetRoot};

use super::{
    animation::{Animation, AnimationScope, AnimationSystems},
    fx::{Billboard, FxAssets, FxMaterial, GLOW, mix},
    particles::{Burst, Emitter, Particles, SPHERE, ember, shot, smoke, soot},
    toon::ToonFlash,
};
use crate::audio::{PlaySound, SoundId};

/// Bolzen: Welteinheiten pro Sekunde, Länge, Durchmesser des fast weißen Kerns und des weichen Halos in der Schussfarbe
/// (der zum Rand hin mit diesem Exponenten verläuft und vorn und hinten etwas übersteht).
const BOLT_SPEED: f32 = 15.0;
const BOLT_LENGTH: f32 = 2.2;
const BOLT_CORE: f32 = 0.05;
const BOLT_HALO: f32 = 0.34;
const BOLT_HALO_FALLOFF: f32 = 1.5;
const BOLT_HALO_OVERHANG: f32 = 1.2;
/// Ein verpuffender Bolzen verblasst auf diesem letzten Anteil seiner Flugzeit und schrumpft dabei auf ein Drittel.
const FIZZLE_FADE: f32 = 0.45;
/// Zielmarke: Der Ring zieht sich von der ersten auf die zweite Größe zusammen.
const TARGET_RING_SIZE: (f32, f32) = (3.4, 1.9);
const TARGET_RING_DURATION: f32 = 0.42;
const PULSE_ALPHA: f32 = 0.55;
/// Strahl der Railgun: weißer Kern, Mantel in der Schussfarbe, der sich beim Verblassen weitet.
const BEAM_CORE: f32 = 0.07;
const BEAM_SHEATH: f32 = 0.26;
const BEAM_DURATION: f32 = 0.52;
/// Laser: So lange steht der Strahl; er ist dünner als die Railgun, flimmert (Hz), sprüht am Ziel Funken (je Sekunde)
/// und verblasst auf dem letzten Anteil seiner Dauer.
const LASER_DURATION: f32 = 0.75;
const LASER_WIDTH: f32 = 0.25;
const LASER_SHIMMER: f32 = 16.0;
const LASER_SPARKS: f32 = 70.0;
const LASER_FADE: f32 = 0.15;
/// Rakete: Welteinheiten pro Sekunde entlang der Bahn, Höhe und seitlicher Schwung der Kurve.
const MISSILE_SPEED: f32 = 8.0;
/// Maßstab des Torpedo-Modells (5,75 lang, also 0,4 auf dem Brett).
const MISSILE_SCALE: f32 = 0.07;
const MISSILE_LENGTH: f32 = 5.75 * MISSILE_SCALE;
const TRAIL_RATE: f32 = 220.0;
/// Traktorstrahl: dünner als die Railgun, flimmert (Hz), blendet ein und aus.
const TETHER_WIDTH: f32 = 0.6;
const TETHER_SHIMMER: f32 = 9.0;
const TETHER_FADE_IN: f32 = 0.22;
const TETHER_FADE_OUT: f32 = 0.3;
/// Einschlag: Blitz und Druckwelle.
const FLASH_DURATION: f32 = 0.14;
const RING_SIZE: f32 = 2.6;
const RING_DURATION: f32 = 0.3;
/// Ab dieser Größe (Kreuzer) klingt die Detonation nach großer Explosion.
const LARGE_BLAST: f32 = 1.4;
/// Hyperraum: Größen in Vielfachen des Schiffsradius.
const WARP_FLASH: f32 = 2.6;
const WARP_FLASH_DURATION: f32 = 0.26;
const WARP_LIGHT: f32 = 5.0;
const WARP_PORTAL: f32 = 2.6;
const WARP_PORTAL_DURATION: f32 = 0.75;

pub const FIRE_FLASH: Color = Color::srgb(1.0, 0.8, 0.5);
const WARP_COLOR: Color = Color::srgb(0.45, 0.75, 1.0);

/// Die Einschlag-Varianten; je Treffer wird zufällig eine gespielt, nie zweimal dieselbe hintereinander.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitVariant {
    /// Funkenfächer: glühende Splitter spritzen zum Angreifer zurück, Glut glimmt nach.
    Sparks,
    /// Feuerball: eine Wolke großer, langsamer Flammen, dazu brennende Trümmer.
    Fireball,
    /// Entladung: die Energie des Schusses zuckt in dünnen Fäden nach allen Seiten.
    Discharge,
    /// Durchschlag: ein Strahl aus Splittern tritt auf der Gegenseite aus, am Eintritt spritzt es zurück.
    Pierce,
    /// Brand: Flammen schlagen aus dem Riss, Rauch steigt auf und verweht.
    Fire,
}

impl HitVariant {
    pub const ALL: [Self; 5] = [
        Self::Sparks,
        Self::Fireball,
        Self::Discharge,
        Self::Pierce,
        Self::Fire,
    ];

    /// Wie der Einschlag klingt: Metall, Feuer oder Entladung.
    fn sound(self) -> SoundId {
        match self {
            Self::Sparks | Self::Pierce => SoundId::ImpactMetal,
            Self::Fireball | Self::Fire => SoundId::ImpactFire,
            Self::Discharge => SoundId::ImpactEnergy,
        }
    }

    /// Blitz (Größe, aufgehellte Schussfarbe oder Feuer), Druckwelle und Schwärme.
    fn look(self) -> (f32, Option<Color>, bool, Vec<Burst>) {
        let deg = |degrees: f32| degrees.to_radians();
        match self {
            Self::Sparks => (
                1.6,
                None,
                true,
                vec![
                    Burst {
                        count: 90,
                        cone: deg(60.0),
                        speed: (3.0, 9.0),
                        life: (0.3, 0.75),
                        size: (0.09, 0.18),
                        stretch: 4.0,
                        gravity: -6.0,
                        ..Burst::DEFAULT
                    },
                    Burst {
                        count: 16,
                        cone: deg(80.0),
                        speed: (0.6, 2.5),
                        life: (0.6, 1.2),
                        size: (0.06, 0.13),
                        gravity: -2.5,
                        colors: ember,
                        ..Burst::DEFAULT
                    },
                ],
            ),
            Self::Fireball => (
                2.6,
                Some(FIRE_FLASH),
                false,
                vec![
                    Burst {
                        count: 50,
                        cone: deg(100.0),
                        speed: (2.5, 6.0),
                        life: (0.6, 1.0),
                        size: (0.5, 0.8),
                        grow: 0.4,
                        gravity: 0.5,
                        ..Burst::DEFAULT
                    },
                    Burst {
                        count: 30,
                        cone: deg(70.0),
                        speed: (3.0, 7.0),
                        life: (0.6, 1.1),
                        size: (0.07, 0.14),
                        stretch: 2.5,
                        gravity: -5.0,
                        colors: ember,
                        ..Burst::DEFAULT
                    },
                ],
            ),
            Self::Discharge => (
                1.4,
                None,
                true,
                vec![
                    Burst {
                        count: 60,
                        cone: deg(150.0),
                        speed: (6.0, 14.0),
                        life: (0.12, 0.3),
                        size: (0.05, 0.09),
                        stretch: 10.0,
                        colors: shot,
                        ..Burst::DEFAULT
                    },
                    Burst {
                        count: 24,
                        cone: deg(120.0),
                        speed: (0.5, 2.0),
                        life: (0.3, 0.6),
                        size: (0.08, 0.14),
                        colors: shot,
                        ..Burst::DEFAULT
                    },
                ],
            ),
            Self::Pierce => (
                1.2,
                None,
                true,
                vec![
                    Burst {
                        count: 50,
                        cone: deg(14.0),
                        through: true,
                        speed: (8.0, 16.0),
                        life: (0.3, 0.6),
                        size: (0.08, 0.16),
                        stretch: 6.0,
                        gravity: -3.0,
                        ..Burst::DEFAULT
                    },
                    Burst {
                        count: 30,
                        cone: deg(50.0),
                        speed: (2.0, 5.0),
                        life: (0.3, 0.6),
                        size: (0.07, 0.13),
                        stretch: 3.0,
                        gravity: -6.0,
                        ..Burst::DEFAULT
                    },
                ],
            ),
            Self::Fire => (
                1.4,
                Some(FIRE_FLASH),
                false,
                vec![
                    Burst {
                        count: 50,
                        cone: deg(60.0),
                        speed: (2.0, 4.0),
                        life: (0.5, 1.0),
                        size: (0.25, 0.4),
                        grow: 0.25,
                        stretch: 1.5,
                        gravity: 4.0,
                        ..Burst::DEFAULT
                    },
                    Burst {
                        count: 18,
                        cone: deg(70.0),
                        speed: (0.5, 1.5),
                        life: (0.9, 1.5),
                        size: (0.35, 0.55),
                        grow: 2.2,
                        gravity: 1.0,
                        opaque: true,
                        colors: smoke,
                        ..Burst::DEFAULT
                    },
                ],
            ),
        }
    }
}

/// Wie ein Schiff in die Luft fliegt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlastStyle {
    /// Chemisch: ein wogender Feuerball, der in einer Rußwolke erstickt.
    Fire,
    /// Reaktorbruch: die Energie zuckt in Fäden heraus, das Feuer bleibt kleiner, kaum Ruß.
    Energy,
}

impl BlastStyle {
    fn bursts(self) -> Vec<Burst> {
        let fireball = |count| Burst {
            count,
            speed: (1.5, 4.5),
            life: (0.6, 1.2),
            size: (0.5, 0.9),
            grow: 0.6,
            stretch: 1.2,
            gravity: 0.4,
            ..Burst::DEFAULT
        };
        let sparks = |count| Burst {
            count,
            speed: (5.0, 13.0),
            life: (0.5, 1.0),
            size: (0.08, 0.16),
            stretch: 5.0,
            gravity: -4.0,
            ..Burst::DEFAULT
        };
        let embers = |count| Burst {
            count,
            speed: (1.5, 4.5),
            life: (1.4, 2.6),
            size: (0.05, 0.12),
            stretch: 1.5,
            gravity: -1.2,
            colors: ember,
            ..Burst::DEFAULT
        };
        let soot = |count, life| Burst {
            count,
            speed: (0.8, 2.0),
            life,
            size: (0.6, 1.0),
            grow: 2.0,
            gravity: 0.35,
            opaque: true,
            colors: soot,
            ..Burst::DEFAULT
        };
        match self {
            Self::Fire => vec![fireball(70), sparks(180), embers(70), soot(36, (2.2, 3.4))],
            Self::Energy => vec![
                Burst {
                    count: 110,
                    speed: (9.0, 20.0),
                    life: (0.15, 0.4),
                    size: (0.06, 0.1),
                    stretch: 12.0,
                    colors: shot,
                    ..Burst::DEFAULT
                },
                Burst {
                    count: 36,
                    speed: (1.0, 3.0),
                    life: (0.5, 0.9),
                    size: (0.8, 1.3),
                    grow: 0.5,
                    colors: shot,
                    ..Burst::DEFAULT
                },
                Burst {
                    life: (0.6, 1.1),
                    size: (0.45, 0.8),
                    speed: (1.5, 4.0),
                    ..fireball(40)
                },
                sparks(140),
                embers(50),
                soot(16, (2.0, 3.0)),
            ],
        }
    }
}

/// Sekundärexplosion: ein Riss in der Hülle, aus dem Feuer, Funken und ein Rauchpilz brechen.
fn pop_bursts() -> [Burst; 3] {
    let deg = |degrees: f32| degrees.to_radians();
    [
        Burst {
            count: 24,
            cone: deg(75.0),
            speed: (1.5, 4.0),
            life: (0.4, 0.8),
            size: (0.22, 0.4),
            grow: 0.5,
            stretch: 1.2,
            gravity: 0.6,
            ..Burst::DEFAULT
        },
        Burst {
            count: 36,
            cone: deg(65.0),
            speed: (3.0, 8.0),
            life: (0.3, 0.7),
            size: (0.06, 0.12),
            stretch: 4.0,
            gravity: -5.0,
            ..Burst::DEFAULT
        },
        Burst {
            count: 8,
            cone: deg(60.0),
            speed: (0.5, 1.4),
            life: (1.0, 1.7),
            size: (0.3, 0.45),
            grow: 2.2,
            gravity: 0.8,
            opaque: true,
            colors: smoke,
            ..Burst::DEFAULT
        },
    ]
}

/// Verpuffen: ein paar matte Funken in der Schussfarbe ziehen langsam weiter und verlöschen, ohne Blitz.
fn fizzle_colors(color: Color) -> Vec<(f32, Srgba)> {
    let srgba = color.to_srgba();
    vec![
        (0.0, srgba.with_alpha(0.7)),
        (
            1.0,
            Srgba::new(srgba.red * 0.4, srgba.green * 0.4, srgba.blue * 0.4, 0.0),
        ),
    ]
}

/// Leuchtspur der Rakete: kleine, langsam verwehende Funken in der Schussfarbe.
fn trail_colors(color: Color) -> Vec<(f32, Srgba)> {
    let srgba = color.to_srgba();
    vec![
        (0.0, Srgba::new(1.0, 1.0, 1.0, 0.9)),
        (0.25, srgba.with_alpha(0.8)),
        (1.0, srgba.with_alpha(0.0)),
    ]
}

/// Flugzeit eines Bolzens von `from` nach `to` in Sekunden (vor der Streckung).
pub fn bolt_duration(from: Vec3, to: Vec3) -> f32 {
    from.distance(to) / BOLT_SPEED
}

/// Ein langer, dünner Leuchtbolzen mit Halo fliegt von `from` nach `to`.
#[derive(Component, Clone, Copy, Debug)]
pub struct Bolt {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Color,
}

/// Ein Bolzen, der verpufft: Er fliegt wie [`Bolt`], verblasst und schrumpft auf dem letzten Stück und zerstiebt dort in
/// ein paar matten Funken, ohne Einschlag, Blitz und Klang.
#[derive(Component, Clone, Copy, Debug)]
pub struct Fizzle {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Color,
}

/// Zielmarke: Ein Ring in der Schussfarbe zieht sich um `position` zusammen und verblasst; dorthin zielte ein Schuss.
#[derive(Component, Clone, Copy, Debug)]
pub struct TargetRing {
    pub position: Vec3,
    pub color: Color,
}

/// Ein gerader Strahl steht zwischen `from` und `to` und verblasst, sein Mantel weitet sich dabei; `width` 1 für die
/// Railgun, dünner für den Zielstrahl der Zielerfassung.
#[derive(Component, Clone, Copy, Debug)]
pub struct Beam {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Color,
    pub width: f32,
}

/// Ein Laserstrahl steht eine Weile zwischen `from` (der Mündung) und `to`, flimmert und sprüht am Ziel Funken. Wer auf
/// ihn wartet, lässt den Schaden folgen, sobald er erlischt.
#[derive(Component, Clone, Copy, Debug)]
pub struct Laser {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Color,
}

/// Torpedo: Das Modell des Torpedos fliegt geradlinig von `from` auf `to`, mit Leuchtspur am Heck.
#[derive(Component, Clone, Copy, Debug)]
pub struct Missile {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Color,
}

/// Traktorstrahl: Ein flimmernder Strahl steht von `from` zu `target` (plus `offset`) und folgt ihm jedes Bild. Er
/// blendet ein und bleibt, bis [`ReleaseTether`] ihn freigibt; dann verblasst er und meldet sein Ende.
#[derive(Component, Clone, Copy, Debug)]
pub struct Tether {
    pub from: Vec3,
    pub target: Entity,
    pub offset: Vec3,
    pub color: Color,
}

/// Gibt einen [`Tether`] frei.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ReleaseTether {
    pub entity: Entity,
}

/// Ein Schild baut sich um ein Schiff vom Radius `size` auf: Blitz, zwei Druckwellen und Funken, die nach außen sprühen.
#[derive(Component, Clone, Copy, Debug)]
pub struct ShieldFlare {
    pub position: Vec3,
    pub color: Color,
    pub size: f32,
}

/// Ein Schuss verpufft am Schild bei `position`, aus Richtung `origin`: Blitz und Welle in der Schildfarbe, Funken
/// gleiten über die Hülle, ohne das Schiff zu treffen.
#[derive(Component, Clone, Copy, Debug)]
pub struct ShieldImpact {
    pub position: Vec3,
    pub origin: Vec3,
    pub color: Color,
}

/// Eine Leuchtkugel wächst auf `size` und verblasst dabei in `duration` Sekunden.
#[derive(Component, Clone, Copy, Debug)]
pub struct Pulse {
    pub position: Vec3,
    pub color: Color,
    pub size: f32,
    pub duration: f32,
}

/// Einschlag eines Schusses aus Richtung `origin` in der Schussfarbe, ohne `variant` zufällig. Endet mit Blitz und
/// Druckwelle; die Partikel glimmen für sich weiter.
#[derive(Component, Clone, Copy, Debug)]
pub struct Impact {
    pub position: Vec3,
    pub origin: Vec3,
    pub color: Color,
    pub variant: Option<HitVariant>,
}

/// Sekundärexplosion auf der Hülle: bricht in Richtung `outward` auf; `size` 1 passt zu einem Jäger.
#[derive(Component, Clone, Copy, Debug)]
pub struct SecondaryExplosion {
    pub position: Vec3,
    pub outward: Vec3,
    pub size: f32,
}

/// Ein Schiff mit dem Radius `size` fällt bei `position` aus dem Hyperraum und fliegt in Richtung `forward` weiter; der
/// Portalring steht quer dazu.
#[derive(Component, Clone, Copy, Debug)]
pub struct Warp {
    pub position: Vec3,
    pub forward: Vec3,
    pub size: f32,
}

/// Detonation eines Schiffs: Kern und Blitz, eine flache und eine aufrechte Druckwelle, die Schwärme des Stils und das
/// Blitzlicht. Endet, sobald der Kern verglüht ist; der Rest läuft weiter.
#[derive(Component, Clone, Copy, Debug)]
pub struct Detonation {
    pub position: Vec3,
    pub energy: Color,
    pub size: f32,
    pub style: BlastStyle,
}

/// Feuer und Rauch schlagen `duration` Sekunden aus einem Trümmerstück und erlöschen dann; `spread` ist die halbe
/// Ausdehnung des Stücks, damit die Flammen aus seiner ganzen Oberfläche kommen.
#[derive(Component, Clone, Copy, Debug)]
pub struct Burn {
    pub piece: Entity,
    pub size: f32,
    pub spread: f32,
    pub duration: f32,
}

/// Ein Lichtblitz, der die Schiffe aufhellt: springt auf `intensity` und klingt quadratisch ab.
#[derive(Component, Clone, Copy, Debug)]
pub struct FlashLight {
    pub position: Vec3,
    pub intensity: f32,
}

/// Wächst und verblasst nach zwei Kurven über den Fortschritt (Babylons Tweens von Größe und Deckkraft).
#[derive(Component, Clone, Copy)]
struct Fade {
    size: f32,
    scale: fn(f32) -> f32,
    alpha: fn(f32) -> f32,
}

/// Ein Blitz-Sprite (`flash`) oder eine Druckwelle (`shockwave`).
#[derive(Component, Clone, Copy, Debug)]
struct FxSprite {
    position: Vec3,
    color: LinearRgba,
    size: f32,
    duration: f32,
    kind: SpriteKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SpriteKind {
    Flash,
    Shockwave,
    FlatShockwave,
}

/// Die beiden Zylinder eines Strahls.
#[derive(Component, Clone, Copy)]
struct BeamParts {
    core: Entity,
    sheath: Entity,
}

/// Ein- und Ausblenden eines Traktorstrahls.
#[derive(Component, Clone, Copy)]
struct TetherFade {
    fade: f32,
    released: bool,
}

/// Die zuletzt gespielte Einschlag-Variante.
#[derive(Resource, Default)]
struct LastHit(Option<HitVariant>);

/// Was die Effekte zum Spawnen brauchen.
#[derive(SystemParam)]
pub struct Fx<'w, 's> {
    pub commands: Commands<'w, 's>,
    assets: Res<'w, FxAssets>,
    materials: ResMut<'w, Assets<FxMaterial>>,
    sounds: MessageWriter<'w, PlaySound>,
}

impl Fx<'_, '_> {
    fn material(&mut self, material: FxMaterial) -> MeshMaterial3d<FxMaterial> {
        MeshMaterial3d(self.materials.add(material))
    }

    fn sound(&mut self, sound: PlaySound) {
        self.sounds.write(sound);
    }

    /// Partikel aus `position` in Richtung `axis`.
    fn burst(&mut self, position: Vec3, axis: Vec3, burst: Burst, color: Color, scale: f32) {
        self.commands.spawn((
            Name::new("Partikel"),
            Transform::from_translation(position),
            Particles::burst(position, axis, burst, color, scale),
        ));
    }

    fn sprite(
        &mut self,
        position: Vec3,
        color: LinearRgba,
        size: f32,
        duration: f32,
        kind: SpriteKind,
    ) {
        self.commands.spawn(FxSprite {
            position,
            color,
            size,
            duration,
            kind,
        });
    }

    fn flash_light(&mut self, position: Vec3, intensity: f32, duration: f32) {
        self.commands.spawn((
            FlashLight {
                position,
                intensity,
            },
            Animation::new(duration),
        ));
    }

    fn pulse(&mut self, position: Vec3, color: Color, size: f32, duration: f32) {
        self.commands.spawn(Pulse {
            position,
            color,
            size,
            duration,
        });
    }
}

/// Selbstleuchtende, additive Farbe für das Bloom.
fn light(color: Color) -> LinearRgba {
    color.to_linear() * GLOW
}

/// Aufgehellt zum Weiß hin.
fn lighten(color: Color, amount: f32) -> Color {
    mix(color, Color::WHITE, amount)
}

pub(super) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastHit>()
            .add_observer(start_bolt)
            .add_observer(start_fizzle)
            .add_observer(start_target_ring)
            .add_observer(start_beam)
            .add_observer(start_laser)
            .add_observer(start_missile)
            .add_observer(start_tether)
            .add_observer(release_tether)
            .add_observer(start_pulse)
            .add_observer(start_shield_flare)
            .add_observer(start_shield_impact)
            .add_observer(start_sprite)
            .add_observer(start_impact)
            .add_observer(start_secondary)
            .add_observer(start_warp)
            .add_observer(start_detonation)
            .add_observer(start_burn)
            .add_systems(
                Update,
                (
                    fly_bolts,
                    fly_fizzles,
                    fade,
                    fade_beams,
                    hold_lasers,
                    fly_missiles,
                    hold_tethers,
                    open_portals,
                    follow_pieces,
                    update_flash_light,
                )
                    .in_set(AnimationSystems::Apply),
            );
    }
}

/// Hängt Kern und Halo eines Bolzens an `entity` und stellt ihn an den Anfang seiner Bahn.
fn spawn_bolt(fx: &mut Fx, entity: Entity, name: &'static str, from: Vec3, to: Vec3, color: Color) {
    let (beam, sphere) = (fx.assets.beam.clone(), fx.assets.sphere.clone());
    let core = fx.material(FxMaterial::light(light(lighten(color, 0.7))));
    let halo = fx.material(FxMaterial {
        rim: -BOLT_HALO_FALLOFF,
        ..FxMaterial::light(light(color))
    });
    fx.commands
        .entity(entity)
        .insert((
            Name::new(name),
            bolt_transform(from, to, 0.0, 1.0),
            Visibility::default(),
            Animation::new(bolt_duration(from, to)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Bolzen-Kern"),
                Mesh3d(beam),
                core,
                Transform::from_scale(Vec3::new(BOLT_CORE, BOLT_CORE, BOLT_LENGTH)),
            ));
            parent.spawn((
                Name::new("Bolzen-Halo"),
                Mesh3d(sphere),
                halo,
                Transform::from_scale(Vec3::new(
                    BOLT_HALO,
                    BOLT_HALO,
                    BOLT_LENGTH * BOLT_HALO_OVERHANG,
                )),
            ));
        });
}

/// Lage eines Bolzens bei `progress`: Die Spitze fliegt von `from` nach `to` (erst langsam, dann gleichmäßig:
/// `easeInCubic` zu 40 %), der Bolzen wächst dabei aus `from` heraus auf seine Länge; `size` skaliert ihn im Ganzen.
fn bolt_transform(from: Vec3, to: Vec3, progress: f32, size: f32) -> Transform {
    let travel = EaseFunction::CubicIn.sample_clamped(progress) * 0.4 + progress * 0.6;
    let head = from.distance(to) * travel;
    let length = head.clamp(0.01, BOLT_LENGTH * size);
    let direction = (to - from).normalize();
    Transform::from_translation(from + direction * (head - length / 2.0))
        .looking_to(direction, Vec3::Y)
        .with_scale(Vec3::new(size, size, length / BOLT_LENGTH))
}

fn start_bolt(add: On<Add<Bolt>>, bolts: Query<&Bolt>, mut fx: Fx) {
    let bolt = *bolts.get(add.entity).expect("Bolzen");
    spawn_bolt(
        &mut fx, add.entity, "Bolzen", bolt.from, bolt.to, bolt.color,
    );
}

fn fly_bolts(mut bolts: Query<(&Bolt, &Animation, &mut Transform)>) {
    for (bolt, animation, mut transform) in &mut bolts {
        *transform = bolt_transform(bolt.from, bolt.to, animation.progress(), 1.0);
    }
}

fn start_fizzle(add: On<Add<Fizzle>>, fizzles: Query<&Fizzle>, mut fx: Fx) {
    let fizzle = *fizzles.get(add.entity).expect("verpuffender Bolzen");
    let name = "Verpuffender Bolzen";
    spawn_bolt(
        &mut fx,
        add.entity,
        name,
        fizzle.from,
        fizzle.to,
        fizzle.color,
    );
}

fn fly_fizzles(
    mut fizzles: Query<(&Fizzle, &Animation, &mut Transform, &Children)>,
    parts: Query<&MeshMaterial3d<FxMaterial>>,
    mut fx: Fx,
) {
    for (fizzle, animation, mut transform, children) in &mut fizzles {
        let progress = animation.progress();
        let left = ((1.0 - progress) / FIZZLE_FADE).min(1.0);
        let size = (1.0 + 2.0 * left) / 3.0;
        *transform = bolt_transform(fizzle.from, fizzle.to, progress, size);
        for material in children.iter().filter_map(|part| parts.get(part).ok()) {
            if let Some(mut material) = fx.materials.get_mut(&material.0) {
                material.set_alpha(left);
            }
        }
        if animation.is_done() {
            let burst = Burst {
                count: 16,
                cone: 70f32.to_radians(),
                speed: (0.4, 1.6),
                life: (0.25, 0.55),
                size: (0.05, 0.11),
                stretch: 1.5,
                colors: fizzle_colors,
                ..Burst::DEFAULT
            };
            let axis = (fizzle.to - fizzle.from).normalize();
            fx.burst(fizzle.to, axis, burst, fizzle.color, 1.0);
        }
    }
}

fn start_target_ring(add: On<Add<TargetRing>>, rings: Query<&TargetRing>, mut fx: Fx) {
    let ring = rings.get(add.entity).expect("Zielmarke");
    let material = fx.material(
        FxMaterial::light(light(lighten(ring.color, 0.35)))
            .with_texture(fx.assets.ring.clone())
            .on_top(),
    );
    let mesh = Mesh3d(fx.assets.quad.clone());
    fx.commands.entity(add.entity).insert((
        Name::new("Zielmarke"),
        mesh,
        material,
        Billboard,
        Transform::from_translation(ring.position),
        Fade {
            size: 1.0,
            scale: |t| {
                let (from, to) = TARGET_RING_SIZE;
                from + (to - from) * EaseFunction::CubicOut.sample_clamped(t)
            },
            alpha: |t| (t * 5.0).min(1.0) * (1.0 - t).powf(1.5),
        },
        Animation::new(TARGET_RING_DURATION),
    ));
}

/// Größe und Deckkraft aller [`Fade`]-Aufträge.
fn fade(
    mut fades: Query<(
        &Fade,
        &Animation,
        &mut Transform,
        &MeshMaterial3d<FxMaterial>,
    )>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (fade, animation, mut transform, material) in &mut fades {
        let progress = animation.progress();
        transform.scale = Vec3::splat((fade.size * (fade.scale)(progress)).max(0.01));
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.set_alpha((fade.alpha)(progress));
        }
    }
}

/// Ein Zylinder (Achse z, Länge 1) von `from` nach `to`.
fn beam_transform(from: Vec3, to: Vec3, diameter: f32) -> Transform {
    Transform::from_translation((from + to) / 2.0)
        .looking_at(to, Vec3::Y)
        .with_scale(Vec3::new(diameter, diameter, from.distance(to)))
}

fn spawn_beam(fx: &mut Fx, from: Vec3, to: Vec3, color: Color, width: f32) -> BeamParts {
    let mesh = fx.assets.beam.clone();
    let core_material = fx.material(FxMaterial::light(light(lighten(color, 0.7))));
    let sheath_material = fx.material(FxMaterial::light(light(color)));
    BeamParts {
        core: fx
            .commands
            .spawn((
                Name::new("Strahl-Kern"),
                Mesh3d(mesh.clone()),
                core_material,
                beam_transform(from, to, BEAM_CORE * width),
            ))
            .id(),
        sheath: fx
            .commands
            .spawn((
                Name::new("Strahl-Mantel"),
                Mesh3d(mesh),
                sheath_material,
                beam_transform(from, to, BEAM_SHEATH * width),
            ))
            .id(),
    }
}

fn start_beam(add: On<Add<Beam>>, beams: Query<&Beam>, mut fx: Fx) {
    let beam = *beams.get(add.entity).expect("Strahl");
    let parts = spawn_beam(&mut fx, beam.from, beam.to, beam.color, beam.width);
    fx.commands
        .entity(add.entity)
        .insert((
            Name::new("Strahl"),
            Transform::default(),
            Visibility::default(),
            Animation::new(BEAM_DURATION),
        ))
        .add_children(&[parts.core, parts.sheath]);
    fx.commands.entity(add.entity).insert(parts);
}

fn fade_beams(
    beams: Query<(&Beam, &BeamParts, &Animation)>,
    mut parts: Query<(&mut Transform, &MeshMaterial3d<FxMaterial>)>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (beam, beam_parts, animation) in &beams {
        let progress = animation.progress();
        if let Ok((_, material)) = parts.get(beam_parts.core)
            && let Some(mut material) = materials.get_mut(&material.0)
        {
            material.set_alpha((1.0 - progress).powi(2));
        }
        if let Ok((mut transform, material)) = parts.get_mut(beam_parts.sheath) {
            let eased = EaseFunction::CubicOut.sample_clamped(progress);
            let widen = BEAM_SHEATH * beam.width * (1.0 + 1.4 * eased);
            transform.scale.x = widen;
            transform.scale.y = widen;
            if let Some(mut material) = materials.get_mut(&material.0) {
                material.set_alpha(0.6 * (1.0 - progress));
            }
        }
    }
}

fn start_laser(add: On<Add<Laser>>, lasers: Query<&Laser>, mut fx: Fx) {
    let laser = *lasers.get(add.entity).expect("Laser");
    // Der Auftrag steht am Ziel, damit ihm die Funken folgen und mit ihm versiegen; der Strahl hängt relativ daran.
    let origin = laser.from - laser.to;
    let parts = spawn_beam(&mut fx, origin, Vec3::ZERO, laser.color, LASER_WIDTH);
    fx.commands
        .entity(add.entity)
        .insert((
            Name::new("Laser"),
            Transform::from_translation(laser.to),
            Visibility::default(),
            Animation::new(LASER_DURATION),
            parts,
        ))
        .add_children(&[parts.core, parts.sheath]);
    let sparks = Burst {
        speed: (0.6, 2.2),
        life: (0.15, 0.4),
        size: (0.04, 0.09),
        stretch: 2.0,
        colors: shot,
        ..Burst::DEFAULT
    };
    fx.commands.spawn((
        Name::new("Laser-Funken"),
        Transform::from_translation(laser.to),
        Particles::stream(
            sparks,
            laser.color,
            1.0,
            SPHERE,
            Emitter::new(LASER_SPARKS, Some(add.entity)),
        ),
    ));
}

fn hold_lasers(
    lasers: Query<(&BeamParts, &Animation), With<Laser>>,
    parts: Query<&MeshMaterial3d<FxMaterial>>,
    mut materials: ResMut<Assets<FxMaterial>>,
    time: Res<Time>,
) {
    let shimmer = 0.8 + 0.2 * (time.elapsed_secs() * LASER_SHIMMER * std::f32::consts::TAU).sin();
    for (beam_parts, animation) in &lasers {
        let alpha = ((1.0 - animation.progress()) / LASER_FADE).min(1.0) * shimmer;
        for (part, strength) in [(beam_parts.core, 1.0), (beam_parts.sheath, 0.5)] {
            if let Ok(material) = parts.get(part)
                && let Some(mut material) = materials.get_mut(&material.0)
            {
                material.set_alpha(strength * alpha);
            }
        }
    }
}

fn start_missile(add: On<Add<Missile>>, missiles: Query<&Missile>, mut fx: Fx) {
    let missile = *missiles.get(add.entity).expect("Rakete");
    // Das Modell liegt vor dem Ursprung, damit die Leuchtspur am Heck austritt und es nicht verdeckt.
    let model = fx
        .commands
        .spawn((
            Name::new("Torpedo-Modell"),
            WorldAssetRoot(fx.assets.torpedo.clone()),
            Transform::from_xyz(0.0, 0.0, -MISSILE_LENGTH / 2.0)
                .with_scale(Vec3::splat(MISSILE_SCALE)),
        ))
        .id();
    fx.commands
        .entity(add.entity)
        .insert((
            Name::new("Rakete"),
            Transform::from_translation(missile.from).looking_at(missile.to, Vec3::Y),
            Visibility::default(),
            Animation::new(missile.from.distance(missile.to) / MISSILE_SPEED),
        ))
        .add_children(&[model]);
    let trail = Burst {
        spread: 0.03,
        speed: (0.1, 0.5),
        life: (0.35, 0.7),
        size: (0.05, 0.08),
        grow: 0.2,
        gravity: 0.3,
        colors: trail_colors,
        ..Burst::DEFAULT
    };
    fx.commands.spawn((
        Name::new("Leuchtspur"),
        Transform::from_translation(missile.from),
        Particles::stream(
            trail,
            missile.color,
            1.0,
            SPHERE,
            Emitter::new(TRAIL_RATE, Some(add.entity)),
        ),
    ));
}

fn fly_missiles(mut missiles: Query<(&Missile, &Animation, &mut Transform)>) {
    for (missile, animation, mut transform) in &mut missiles {
        let progress = animation.progress();
        let travelled = EaseFunction::CubicIn.sample_clamped(progress) * 0.35 + progress * 0.65;
        transform.translation = missile.from.lerp(missile.to, travelled);
    }
}

fn start_tether(
    add: On<Add<Tether>>,
    tethers: Query<&Tether>,
    targets: Query<&GlobalTransform>,
    mut fx: Fx,
) {
    let tether = *tethers.get(add.entity).expect("Traktorstrahl");
    let to = targets
        .get(tether.target)
        .map_or(tether.from, |target| target.translation())
        + tether.offset;
    let parts = spawn_beam(&mut fx, tether.from, to, tether.color, TETHER_WIDTH);
    fx.commands
        .entity(add.entity)
        .insert((
            Name::new("Traktorstrahl"),
            Transform::default(),
            Visibility::default(),
            parts,
            TetherFade {
                fade: 0.0,
                released: false,
            },
        ))
        .add_children(&[parts.core, parts.sheath]);
}

fn release_tether(
    release: On<ReleaseTether>,
    mut tethers: Query<&mut TetherFade>,
    mut commands: Commands,
) {
    if let Ok(mut tether) = tethers.get_mut(release.entity)
        && !tether.released
    {
        tether.released = true;
        commands
            .entity(release.entity)
            .insert(Animation::new(TETHER_FADE_OUT));
    }
}

fn hold_tethers(
    mut tethers: Query<(&Tether, &BeamParts, &mut TetherFade, Option<&Animation>)>,
    targets: Query<&GlobalTransform, Without<BeamParts>>,
    mut parts: Query<(&mut Transform, &MeshMaterial3d<FxMaterial>)>,
    mut materials: ResMut<Assets<FxMaterial>>,
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
) {
    let shimmer =
        0.75 + 0.25 * (time.elapsed_secs() * TETHER_SHIMMER * std::f32::consts::TAU).sin();
    for (tether, beam_parts, mut fade, animation) in &mut tethers {
        fade.fade = match animation {
            Some(animation) => 1.0 - animation.progress(),
            None => (fade.fade
                + time.delta_secs()
                    / (TETHER_FADE_IN * settings.time_scale(AnimationScope::General)))
            .min(1.0),
        };
        let Ok(target) = targets.get(tether.target) else {
            continue;
        };
        let to = target.translation() + tether.offset;
        for (part, diameter, alpha) in [
            (beam_parts.core, BEAM_CORE * TETHER_WIDTH, 1.0),
            (beam_parts.sheath, BEAM_SHEATH * TETHER_WIDTH, 0.45),
        ] {
            if let Ok((mut transform, material)) = parts.get_mut(part) {
                *transform = beam_transform(tether.from, to, diameter);
                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.set_alpha(alpha * fade.fade * shimmer);
                }
            }
        }
    }
}

fn start_pulse(add: On<Add<Pulse>>, pulses: Query<&Pulse>, mut fx: Fx) {
    let pulse = *pulses.get(add.entity).expect("Impuls");
    let material = fx.material(FxMaterial::light(
        light(pulse.color).with_alpha(PULSE_ALPHA),
    ));
    let mesh = Mesh3d(fx.assets.sphere.clone());
    fx.commands.entity(add.entity).insert((
        Name::new("Impuls"),
        mesh,
        material,
        Transform::from_translation(pulse.position),
        Fade {
            size: pulse.size,
            scale: |t| EaseFunction::CubicOut.sample_clamped(t),
            alpha: |t| PULSE_ALPHA * (1.0 - t).powi(2),
        },
        Animation::new(pulse.duration),
    ));
}

fn start_shield_flare(add: On<Add<ShieldFlare>>, flares: Query<&ShieldFlare>, mut fx: Fx) {
    let ShieldFlare {
        position,
        color,
        size,
    } = *flares.get(add.entity).expect("Schildaufbau");
    fx.sprite(
        position,
        light(lighten(color, 0.7)),
        2.2 * size,
        0.25,
        SpriteKind::Flash,
    );
    fx.sprite(
        position,
        light(color),
        3.4 * size,
        0.6,
        SpriteKind::FlatShockwave,
    );
    fx.sprite(
        position,
        light(lighten(color, 0.3)),
        2.6 * size,
        0.45,
        SpriteKind::Shockwave,
    );
    let sparks = Burst {
        count: 48,
        spread: 0.3 * size,
        speed: (1.5 * size, 3.5 * size),
        life: (0.25, 0.55),
        size: (0.05, 0.1),
        stretch: 3.0,
        colors: shot,
        ..Burst::DEFAULT
    };
    fx.burst(position, Vec3::Y, sparks, color, 1.0);
    fx.commands
        .entity(add.entity)
        .insert((Name::new("Schildaufbau"), Animation::new(0.6)));
}

fn start_shield_impact(add: On<Add<ShieldImpact>>, impacts: Query<&ShieldImpact>, mut fx: Fx) {
    let ShieldImpact {
        position,
        origin,
        color,
    } = *impacts.get(add.entity).expect("Schildtreffer");
    fx.sound(PlaySound::new(SoundId::ImpactEnergy).at(position).rate(1.3));
    let normal = (origin - position).normalize();
    let sparks = Burst {
        count: 40,
        cone: 80f32.to_radians(),
        speed: (2.0, 6.0),
        life: (0.15, 0.4),
        size: (0.05, 0.09),
        stretch: 6.0,
        colors: shot,
        ..Burst::DEFAULT
    };
    fx.burst(position, normal, sparks, color, 1.0);
    fx.sprite(
        position,
        light(lighten(color, 0.6)),
        1.3,
        FLASH_DURATION * 1.5,
        SpriteKind::Flash,
    );
    fx.sprite(
        position,
        light(color),
        1.8,
        RING_DURATION,
        SpriteKind::Shockwave,
    );
    fx.commands
        .entity(add.entity)
        .insert((Name::new("Schildtreffer"), Animation::new(RING_DURATION)));
}

fn start_sprite(add: On<Add<FxSprite>>, sprites: Query<&FxSprite>, mut fx: Fx) {
    let sprite = *sprites.get(add.entity).expect("Blitz");
    let texture = match sprite.kind {
        SpriteKind::Flash => fx.assets.dot.clone(),
        SpriteKind::Shockwave | SpriteKind::FlatShockwave => fx.assets.ring.clone(),
    };
    let material = fx.material(
        FxMaterial::light(sprite.color)
            .with_texture(texture)
            .on_top(),
    );
    let mesh = Mesh3d(fx.assets.quad.clone());
    let fade = match sprite.kind {
        SpriteKind::Flash => Fade {
            size: sprite.size,
            scale: |t| 0.6 + 0.4 * EaseFunction::CubicOut.sample_clamped(t),
            alpha: |t| (1.0 - t).powi(2),
        },
        SpriteKind::Shockwave | SpriteKind::FlatShockwave => Fade {
            size: sprite.size,
            scale: |t| EaseFunction::CubicOut.sample_clamped(t),
            alpha: |t| (1.0 - t).powf(1.5),
        },
    };
    let mut entity = fx.commands.entity(add.entity);
    entity.insert((
        Name::new("Blitz"),
        mesh,
        material,
        fade,
        Animation::new(sprite.duration),
    ));
    if sprite.kind == SpriteKind::FlatShockwave {
        entity.insert(
            Transform::from_translation(sprite.position)
                .with_rotation(Quat::from_rotation_x(-PI / 2.0)),
        );
    } else {
        entity.insert((Transform::from_translation(sprite.position), Billboard));
    }
}

fn start_impact(
    add: On<Add<Impact>>,
    impacts: Query<&Impact>,
    mut last: ResMut<LastHit>,
    mut fx: Fx,
) {
    let impact = *impacts.get(add.entity).expect("Einschlag");
    let variant = impact.variant.unwrap_or_else(|| {
        let choices: Vec<_> = HitVariant::ALL
            .into_iter()
            .filter(|variant| Some(*variant) != last.0)
            .collect();
        choices[rand::random_range(0..choices.len())]
    });
    last.0 = Some(variant);
    let (flash_size, flash_color, ring, bursts) = variant.look();
    fx.sound(PlaySound::new(variant.sound()).at(impact.position));
    let normal = (impact.origin - impact.position).normalize();
    for burst in bursts {
        fx.burst(impact.position, normal, burst, impact.color, 1.0);
    }
    let flash = flash_color.unwrap_or_else(|| lighten(impact.color, 0.5));
    fx.sprite(
        impact.position,
        light(flash),
        flash_size,
        FLASH_DURATION,
        SpriteKind::Flash,
    );
    if ring {
        fx.sprite(
            impact.position,
            light(impact.color),
            RING_SIZE,
            RING_DURATION,
            SpriteKind::Shockwave,
        );
    }
    fx.commands.entity(add.entity).insert((
        Name::new("Einschlag"),
        Animation::new(if ring { RING_DURATION } else { FLASH_DURATION }),
    ));
}

fn start_secondary(add: On<Add<SecondaryExplosion>>, pops: Query<&SecondaryExplosion>, mut fx: Fx) {
    let pop = *pops.get(add.entity).expect("Sekundärexplosion");
    fx.sound(
        PlaySound::new(SoundId::ExplosionSmall)
            .at(pop.position)
            .volume(0.5)
            .rate(1.05 - 0.1 * pop.size),
    );
    for burst in pop_bursts() {
        fx.burst(pop.position, pop.outward, burst, FIRE_FLASH, pop.size);
    }
    fx.sprite(
        pop.position,
        light(FIRE_FLASH),
        1.1 * pop.size,
        0.16,
        SpriteKind::Flash,
    );
    fx.flash_light(pop.position, 6.0 * pop.size, 0.22);
    fx.commands
        .entity(add.entity)
        .insert((Name::new("Sekundärexplosion"), Animation::new(0.22)));
}

fn start_warp(add: On<Add<Warp>>, warps: Query<&Warp>, mut fx: Fx) {
    let warp = *warps.get(add.entity).expect("Hyperraum-Austritt");
    let streaks = Burst {
        count: 36,
        cone: 6f32.to_radians(),
        spread: 0.5,
        speed: (14.0, 24.0),
        life: (0.12, 0.3),
        size: (0.05, 0.1),
        stretch: 9.0,
        colors: |_| {
            vec![
                (0.0, Srgba::WHITE),
                (0.4, Srgba::new(0.55, 0.8, 1.0, 0.9)),
                (1.0, Srgba::new(0.3, 0.5, 1.0, 0.0)),
            ]
        },
        ..Burst::DEFAULT
    };
    let sparks = Burst {
        count: 24,
        spread: 0.4,
        speed: (0.4, 1.4),
        life: (0.4, 0.8),
        size: (0.06, 0.12),
        stretch: 1.5,
        colors: |_| {
            vec![
                (0.0, Srgba::new(0.85, 0.95, 1.0, 1.0)),
                (1.0, Srgba::new(0.4, 0.6, 1.0, 0.0)),
            ]
        },
        ..Burst::DEFAULT
    };
    fx.burst(
        warp.position - warp.forward * warp.size,
        warp.forward,
        streaks,
        WARP_COLOR,
        warp.size,
    );
    fx.burst(warp.position, warp.forward, sparks, WARP_COLOR, warp.size);
    fx.sprite(
        warp.position,
        light(lighten(WARP_COLOR, 0.6)),
        WARP_FLASH * warp.size,
        WARP_FLASH_DURATION,
        SpriteKind::Flash,
    );
    fx.flash_light(
        warp.position,
        WARP_LIGHT * warp.size,
        WARP_FLASH_DURATION * 1.5,
    );
    // Der Portalring steht im Raum (vom Rumpf verdeckt), quer zur Flugrichtung.
    let material =
        fx.material(FxMaterial::light(light(WARP_COLOR)).with_texture(fx.assets.ring.clone()));
    let mesh = Mesh3d(fx.assets.quad.clone());
    fx.commands.entity(add.entity).insert((
        Name::new("Portal"),
        mesh,
        material,
        Transform::from_translation(warp.position).looking_to(warp.forward, Vec3::Y),
        Animation::new(WARP_PORTAL_DURATION),
    ));
}

/// Der Portalring reißt schnell auf und zieht sich langsamer wieder zusammen.
fn open_portals(
    mut portals: Query<(
        &Warp,
        &Animation,
        &mut Transform,
        &MeshMaterial3d<FxMaterial>,
    )>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    for (warp, animation, mut transform, material) in &mut portals {
        let progress = animation.progress();
        let open = if progress < 0.25 {
            EaseFunction::CubicOut.sample_clamped(progress / 0.25)
        } else {
            1.0 - EaseFunction::CubicIn.sample_clamped((progress - 0.25) / 0.75)
        };
        transform.scale = Vec3::splat((WARP_PORTAL * warp.size * open).max(0.01));
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.set_alpha((open * 1.5).min(1.0));
        }
    }
}

fn start_detonation(add: On<Add<Detonation>>, detonations: Query<&Detonation>, mut fx: Fx) {
    let blast = *detonations.get(add.entity).expect("Detonation");
    let (position, energy, size) = (blast.position, blast.energy, blast.size);
    fx.sound(
        PlaySound::new(if size >= LARGE_BLAST {
            SoundId::ExplosionLarge
        } else {
            SoundId::ExplosionSmall
        })
        .at(position)
        .rate(1.0 - 0.1 * size),
    );
    if blast.style == BlastStyle::Energy {
        fx.sound(PlaySound::new(SoundId::ReactorBreach).at(position));
    }
    for burst in blast.style.bursts() {
        fx.burst(position, Vec3::Y, burst, energy, size);
    }
    fx.sprite(
        position,
        light(lighten(energy, 0.6)),
        2.2 * size,
        0.18,
        SpriteKind::Flash,
    );
    fx.sprite(
        position,
        light(energy),
        5.0 * size,
        0.7,
        SpriteKind::FlatShockwave,
    );
    fx.sprite(
        position,
        light(lighten(energy, 0.4)),
        3.0 * size,
        0.45,
        SpriteKind::Shockwave,
    );
    fx.flash_light(position, 55.0 * size, 0.65);
    fx.pulse(position, energy, 2.0 * size, 0.65);
    // Der Kern: Auf ihn wartet, wer die Detonation abwartet.
    let core = lighten(energy, 0.7);
    fx.commands.entity(add.entity).insert((
        Name::new("Detonation"),
        Pulse {
            position,
            color: core,
            size: 1.2 * size,
            duration: 0.38,
        },
    ));
}

fn start_burn(
    add: On<Add<Burn>>,
    burns: Query<&Burn>,
    pieces: Query<&GlobalTransform>,
    mut fx: Fx,
) {
    let burn = *burns.get(add.entity).expect("Brand");
    let at = pieces
        .get(burn.piece)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    fx.commands.entity(add.entity).insert((
        Name::new("Brand"),
        Transform::from_translation(at),
        Animation::new(burn.duration),
    ));
    let flames = Burst {
        spread: burn.spread,
        speed: (0.4, 1.2),
        life: (0.4, 0.8),
        size: (0.25, 0.45),
        grow: 0.3,
        stretch: 1.2,
        gravity: 1.5,
        ..Burst::DEFAULT
    };
    let smoke = Burst {
        spread: burn.spread,
        speed: (0.3, 0.8),
        life: (1.0, 1.8),
        size: (0.3, 0.5),
        grow: 2.5,
        gravity: 0.8,
        opaque: true,
        colors: smoke,
        ..Burst::DEFAULT
    };
    for (burst, rate) in [(flames, 50.0), (smoke, 12.0)] {
        fx.commands.spawn((
            Name::new("Brand-Partikel"),
            Transform::from_translation(at),
            Particles::stream(
                burst,
                FIRE_FLASH,
                burn.size,
                120f32.to_radians(),
                Emitter::new(rate, Some(add.entity)),
            ),
        ));
    }
}

/// Brände folgen ihrem Trümmerstück; die Ströme folgen dem Brand und versiegen mit ihm.
fn follow_pieces(
    mut burns: Query<(&Burn, &mut Transform)>,
    pieces: Query<&GlobalTransform, Without<Burn>>,
) {
    for (burn, mut transform) in &mut burns {
        if let Ok(piece) = pieces.get(burn.piece) {
            transform.translation = piece.translation();
        }
    }
}

/// Das eine Blitzlicht der Szene zeigt jedes Bild den stärksten laufenden Blitz.
fn update_flash_light(lights: Query<(&FlashLight, &Animation)>, mut flash: ResMut<ToonFlash>) {
    let strongest = lights
        .iter()
        .map(|(light, animation)| {
            (
                light.position,
                light.intensity * (1.0 - animation.progress()).powi(2),
            )
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let next = strongest.map_or(ToonFlash::default(), |(position, intensity)| ToonFlash {
        position,
        intensity,
    });
    flash.set_if_neq(next);
}
