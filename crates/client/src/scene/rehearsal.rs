//! Generalprobe hinter dem Ladebildschirm (Port von `scene/rehearsal.ts`): Einmal stehen alle Gegner und Flaggschiffe
//! als Modellgruppen (Gegner in ihrer Lackierung, ein Kreuzermodell brennend und ausgebrannt, Rumpfbrüche) und ihre
//! Wracks mit glühenden Bruchflächen und ein Geist auf dem Brett, damit Bevy alle Pipelines übersetzt, bevor das
//! Gefecht beginnt. Dazu Balken mit Verlustvorschau, Totenkopf und Sternen, alle Marken am Boden und die stummen Effekte (Bolzen,
//! Zielmarke, Strahl, Laser, Rakete mit Leuchtspur, Schildaufbau, Impuls, Hyperraum-Austritt mit
//! Blitz und Partikeln, Brand mit Rauch, brennende und glühende Schiffe). Neue Darstellungsvarianten gehören deshalb auch hierher. Am Ende des Ladens
//! verschwindet alles wieder; die Effekte räumen sich selbst weg.

use bevy::prelude::*;
use voidfleet_core::{enemies::EnemyClass, events::UnitStats};

use super::{
    aura::ShipAura,
    coords::{BOARD_CENTER, Side, facing},
    effects::{
        Beam, Bolt, Burn, ChargedBeam, Laser, Missile, Pulse, ShieldFlare, TargetRing, WARP_COLOR,
        Warp,
    },
    markers::{MarkerKind, MarkerSpec, ShipMarkers},
    ship::{BreakApart, ToonState, spawn_ghost, spawn_ship},
    status::ShipStatus,
};
use crate::{assets::AppState, data::ship_models::ShipModels, palette};

/// Alle Marken, reihum auf die Schiffe verteilt.
const MARKERS: [MarkerKind; 6] = [
    MarkerKind::Lock,
    MarkerKind::Taunt,
    MarkerKind::Ready,
    MarkerKind::Counterfire,
    MarkerKind::FocusFire,
    MarkerKind::HeadStart,
];

pub(super) fn stage(mut commands: Commands, models: Res<ShipModels>) {
    let ships: Vec<_> = models.iter().map(|(kind, _)| kind).collect();
    let mut wrecks = Vec::new();
    for (index, ship) in ships.into_iter().enumerate() {
        for (row, wreck) in [(0, false), (1, true)] {
            let position =
                BOARD_CENTER + Vec3::new(row as f32 * 2.0 - 1.0, 0.0, index as f32 - 7.0);
            let side = if index % 2 == 0 {
                Side::Player
            } else {
                Side::Enemy
            };
            let entity = spawn_ship(&mut commands, &models, ship);
            commands.entity(entity).insert((
                Transform::from_translation(position)
                    .looking_to(Vec3::X * facing(side), Vec3::Y)
                    .with_scale(Vec3::splat(super::ship::BOARD_SCALE)),
                ToonState {
                    heat: wreck.then_some(LinearRgba::rgb(1.0, 0.5, 0.2)),
                    ..ToonState::facing(side)
                },
                // Jäger voll (alle fünf Modelle), die übrigen im Notfall: ein Kreuzer brennt, einer ist ausgebrannt,
                // Schlachtschiffe und Flaggschiffe mit Rumpfbrüchen.
                ShipStatus {
                    preview: Some(UnitStats {
                        hull: i32::from(index % 3 > 0),
                        shield: 0,
                    }),
                    doomed: true,
                    ..ShipStatus::new(
                        if ship.class() == EnemyClass::Fighter {
                            10
                        } else {
                            2
                        },
                        10,
                        5,
                    )
                },
                ShipMarkers(vec![MarkerSpec {
                    planned: index % 2 == 0,
                    ..MarkerSpec::new(MARKERS[index % MARKERS.len()], palette::SELECT)
                }]),
                // Brand und Aufladung als Dauerzustand.
                ShipAura {
                    burning: index == 0,
                    charged: index == 1,
                    breached: index == 2,
                },
                DespawnOnExit(AppState::Loading),
            ));
            if wreck {
                commands.trigger(BreakApart { entity });
                wrecks.push(entity);
            }
        }
    }

    // Ein Geist (durchsichtig, eigene Pipeline).
    if let Some((kind, _)) = models.iter().find(|(kind, _)| kind.is_flagship()) {
        let ghost = spawn_ghost(&mut commands, &models, kind);
        commands.entity(ghost).insert((
            Transform::from_translation(BOARD_CENTER + Vec3::new(3.0, 0.0, 0.0))
                .with_scale(Vec3::splat(super::ship::BOARD_SCALE)),
            DespawnOnExit(AppState::Loading),
        ));
    }

    // Die stummen Effekte; was Klang macht (Einschläge, Explosionen), bleibt der Probe fern.
    let at = |x: f32| BOARD_CENTER + Vec3::new(x, 0.6, 3.0);
    commands.spawn(Bolt {
        from: at(-3.0),
        to: at(3.0),
        color: palette::PLAYER,
    });
    commands.spawn(TargetRing {
        position: at(0.0),
        color: palette::ENEMY,
    });
    commands.spawn(Beam {
        from: at(-4.0),
        to: at(4.0),
        color: palette::PLAYER,
        width: 1.0,
    });
    commands.spawn(Laser {
        from: at(-4.0),
        to: at(4.0),
        color: palette::ENEMY,
    });
    commands.spawn(ChargedBeam {
        from: at(-4.0),
        to: at(4.0),
        color: palette::ENEMY,
    });
    commands.spawn(Missile {
        from: at(-4.0),
        to: at(4.0),
        color: palette::ENEMY,
    });
    commands.spawn(ShieldFlare {
        position: at(-1.0),
        color: palette::SHIELD,
        size: 1.0,
    });
    commands.spawn(Pulse {
        position: at(1.0),
        color: palette::HEAL,
        size: 1.5,
        duration: 0.6,
    });
    commands.spawn(Warp {
        position: at(2.0),
        forward: Vec3::X,
        size: 1.0,
        color: WARP_COLOR,
    });
    if let Some(&piece) = wrecks.first() {
        commands.spawn(Burn {
            piece,
            size: 0.5,
            spread: 0.3,
            duration: 0.5,
        });
    }
}
