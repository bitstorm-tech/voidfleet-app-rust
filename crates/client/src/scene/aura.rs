//! Dauerhafte Zustände eines Schiffs als Partikel: Ein brennendes Schiff brennt sichtbar (Flammen und Rauch), ein
//! aufgeladenes glüht (aufsteigende Funken). Was gilt, steht in [`ShipAura`]; [`crate::scene::sync`] setzt es aus dem
//! Gefecht, die Wiedergabe beim Ereignis. Die Ströme folgen dem Schiff und verschwinden mit ihm; fehlt einer (etwa
//! nach dem Überspringen, das alle Partikel räumt), entsteht er im nächsten Bild neu.

use bevy::prelude::*;

use super::{
    effects::FIRE_FLASH,
    particles::{Burst, Emitter, Particles, shot, smoke},
    ship::Ship,
};
use crate::palette;

/// Was an einem Schiff dauerhaft zu sehen ist.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipAura {
    pub burning: bool,
    pub charged: bool,
}

/// Die laufenden Ströme eines Schiffs.
#[derive(Component, Debug, Default)]
struct AuraStreams(Vec<(AuraKind, Entity)>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuraKind {
    Flames,
    Smoke,
    Glow,
}

impl AuraKind {
    fn particles(self, ship: Entity) -> Particles {
        let (burst, color, rate) = match self {
            Self::Flames => (
                Burst {
                    spread: 0.3,
                    speed: (0.3, 0.9),
                    life: (0.35, 0.7),
                    size: (0.16, 0.3),
                    grow: 0.3,
                    stretch: 1.2,
                    gravity: 1.2,
                    ..Burst::DEFAULT
                },
                FIRE_FLASH,
                26.0,
            ),
            Self::Smoke => (
                Burst {
                    spread: 0.25,
                    speed: (0.2, 0.6),
                    life: (1.0, 1.8),
                    size: (0.2, 0.35),
                    grow: 2.5,
                    gravity: 0.6,
                    opaque: true,
                    colors: smoke,
                    ..Burst::DEFAULT
                },
                FIRE_FLASH,
                6.0,
            ),
            Self::Glow => (
                Burst {
                    spread: 0.45,
                    speed: (0.2, 0.6),
                    life: (0.5, 0.9),
                    size: (0.06, 0.12),
                    stretch: 2.0,
                    gravity: 1.0,
                    colors: shot,
                    ..Burst::DEFAULT
                },
                palette::BUFF,
                30.0,
            ),
        };
        Particles::stream(
            burst,
            color,
            1.0,
            100f32.to_radians(),
            Emitter::new(rate, Some(ship)),
        )
    }
}

pub(super) struct AuraPlugin;

impl Plugin for AuraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keep_streams);
    }
}

/// Startet fehlende Ströme und lässt nicht mehr gewollte versiegen.
fn keep_streams(
    mut commands: Commands,
    ships: Query<(Entity, &ShipAura, Option<&AuraStreams>), With<Ship>>,
    mut streams: Query<&mut Particles>,
) {
    for (ship, aura, current) in &ships {
        let wanted: Vec<AuraKind> = [
            (aura.burning, AuraKind::Flames),
            (aura.burning, AuraKind::Smoke),
            (aura.charged, AuraKind::Glow),
        ]
        .into_iter()
        .filter_map(|(on, kind)| on.then_some(kind))
        .collect();
        let mut kept = Vec::new();
        for (kind, stream) in current.map_or(&[][..], |streams| &streams.0) {
            let Ok(mut particles) = streams.get_mut(*stream) else {
                continue;
            };
            if wanted.contains(kind) {
                kept.push((*kind, *stream));
            } else if let Some(emitter) = particles.emitter.as_mut() {
                emitter.running = false;
            }
        }
        for &kind in &wanted {
            if !kept.iter().any(|(have, _)| *have == kind) {
                let stream = commands
                    .spawn((Name::new("Zustand"), kind.particles(ship)))
                    .id();
                kept.push((kind, stream));
            }
        }
        let unchanged = current.is_some_and(|streams| streams.0 == kept);
        if !unchanged {
            commands.entity(ship).insert(AuraStreams(kept));
        }
    }
}
