//! Soundeffekte und Musik, erzeugt mit `tools/generateAudio.ts` (ElevenLabs Sound Effects und Music).
//!
//! Ein Effekt hat eine oder mehrere Varianten (`audio/sfx/<id>_<n>.mp3`), von denen jedes Abspielen eine zufällig
//! wählt. Alle Varianten werden beim Laden auf gleiche Lautheit gebracht ([`crate::audio`]); [`SoundId::volume`] legt
//! nur noch fest, wie laut ein Effekt im Verhältnis zu den anderen ist.

use voidfleet_core::{admirals::FlagshipId, enemies::EnemyId};

use super::ship_models::ShipKind;

/// Waffe je Schiff.
pub fn shot_sound(kind: ShipKind) -> SoundId {
    match kind {
        ShipKind::Enemy(EnemyId::GuardCruiser | EnemyId::Carrier | EnemyId::Seal)
        | ShipKind::Flagship(FlagshipId::Vanguard) => SoundId::LaserMedium,
        ShipKind::Enemy(EnemyId::PlasmaCruiser) | ShipKind::Flagship(FlagshipId::Pyre) => {
            SoundId::PlasmaShot
        }
        ShipKind::Enemy(EnemyId::Besieger | EnemyId::ScrapKing | EnemyId::Warden)
        | ShipKind::Flagship(FlagshipId::Bastion | FlagshipId::Pendulum) => SoundId::CannonHeavy,
        ShipKind::Enemy(
            EnemyId::Raiders | EnemyId::Fireship | EnemyId::TorpedoWing | EnemyId::Spotter,
        ) => SoundId::LaserLight,
    }
}

/// Ein Soundeffekt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundId {
    // Waffen, je nach Schiff
    LaserLight,
    LaserMedium,
    CannonHeavy,
    PlasmaShot,
    // Einschläge
    ImpactMetal,
    ImpactFire,
    ImpactEnergy,
    // Zerstörung
    HullStress,
    ExplosionSmall,
    ExplosionLarge,
    ReactorBreach,
    // Triebwerke (Schleifen)
    EngineLight,
    EngineMedium,
    EngineHeavy,
    // Befehle
    OrbitalStrike,
    MissileVolley,
    PowerUp,
    Jammer,
    ShieldUp,
    Emp,
    Repair,
    Scavenge,
    // Spielausgang
    Victory,
    Defeat,
}

impl SoundId {
    pub const ALL: [Self; 24] = [
        Self::LaserLight,
        Self::LaserMedium,
        Self::CannonHeavy,
        Self::PlasmaShot,
        Self::ImpactMetal,
        Self::ImpactFire,
        Self::ImpactEnergy,
        Self::HullStress,
        Self::ExplosionSmall,
        Self::ExplosionLarge,
        Self::ReactorBreach,
        Self::EngineLight,
        Self::EngineMedium,
        Self::EngineHeavy,
        Self::OrbitalStrike,
        Self::MissileVolley,
        Self::PowerUp,
        Self::Jammer,
        Self::ShieldUp,
        Self::Emp,
        Self::Repair,
        Self::Scavenge,
        Self::Victory,
        Self::Defeat,
    ];

    /// Dateiname ohne Variante.
    pub const fn key(self) -> &'static str {
        match self {
            Self::LaserLight => "laser_light",
            Self::LaserMedium => "laser_medium",
            Self::CannonHeavy => "cannon_heavy",
            Self::PlasmaShot => "plasma_shot",
            Self::ImpactMetal => "impact_metal",
            Self::ImpactFire => "impact_fire",
            Self::ImpactEnergy => "impact_energy",
            Self::HullStress => "hull_stress",
            Self::ExplosionSmall => "explosion_small",
            Self::ExplosionLarge => "explosion_large",
            Self::ReactorBreach => "reactor_breach",
            Self::EngineLight => "engine_light",
            Self::EngineMedium => "engine_medium",
            Self::EngineHeavy => "engine_heavy",
            Self::OrbitalStrike => "orbital_strike",
            Self::MissileVolley => "missile_volley",
            Self::PowerUp => "power_up",
            Self::Jammer => "jammer",
            Self::ShieldUp => "shield_up",
            Self::Emp => "emp",
            Self::Repair => "repair",
            Self::Scavenge => "scavenge",
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }

    /// Anzahl der Varianten (`<id>_1.mp3` … `<id>_<n>.mp3`).
    pub const fn variants(self) -> usize {
        match self {
            Self::LaserLight
            | Self::LaserMedium
            | Self::ImpactMetal
            | Self::ImpactFire
            | Self::ExplosionSmall => 3,
            Self::CannonHeavy
            | Self::PlasmaShot
            | Self::ImpactEnergy
            | Self::HullStress
            | Self::ExplosionLarge
            | Self::Victory
            | Self::Defeat => 2,
            _ => 1,
        }
    }

    /// Lautstärke im Verhältnis zu den anderen Effekten.
    pub const fn volume(self) -> f32 {
        match self {
            Self::LaserLight => 0.4,
            Self::LaserMedium => 0.45,
            Self::CannonHeavy => 0.6,
            Self::PlasmaShot => 0.5,
            Self::ImpactMetal => 0.5,
            Self::ImpactFire => 0.55,
            Self::ImpactEnergy => 0.5,
            Self::HullStress => 0.6,
            Self::ExplosionSmall => 0.65,
            Self::ExplosionLarge => 1.0,
            Self::ReactorBreach => 0.7,
            Self::EngineLight | Self::EngineMedium => 0.3,
            Self::EngineHeavy => 0.35,
            Self::OrbitalStrike => 0.8,
            Self::MissileVolley => 0.6,
            Self::PowerUp => 0.45,
            Self::Jammer => 0.4,
            Self::ShieldUp => 0.45,
            Self::Emp => 0.6,
            Self::Repair => 0.45,
            Self::Scavenge => 0.45,
            Self::Victory | Self::Defeat => 0.8,
        }
    }

    /// Asset-Pfade aller Varianten.
    pub fn paths(self) -> impl Iterator<Item = String> {
        (1..=self.variants()).map(move |n| format!("audio/sfx/{}_{n}.mp3", self.key()))
    }
}

/// Stimmung der Hintergrundmusik: ruhig während der Planung, angespannt während eine Runde abgespielt wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MusicMood {
    Calm,
    Battle,
}

impl MusicMood {
    pub const ALL: [Self; 2] = [Self::Calm, Self::Battle];

    pub fn path(self) -> &'static str {
        match self {
            Self::Calm => "audio/music/calm.mp3",
            Self::Battle => "audio/music/battle.mp3",
        }
    }

    /// Kernteil ohne Ein- und Ausklang in Sekunden, der endlos läuft (gemessen mit `dev/sounds.html`).
    pub fn core(self) -> (f64, f64) {
        match self {
            Self::Calm => (10.0, 161.0),
            Self::Battle => (4.0, 114.0),
        }
    }
}
