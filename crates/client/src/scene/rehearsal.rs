//! Generalprobe hinter dem Ladebildschirm (Port von `scene/rehearsal.ts`): Einmal stehen alle Schiffe samt Drohne,
//! Trugbild und Flaggschiffen, ihre Geister und ihre Wracks mit glühenden Bruchflächen auf dem Brett, damit Bevy alle
//! Pipelines übersetzt, bevor das Match beginnt. Dazu Balken mit Verlustvorschau, Totenkopf und Krone, alle Marken am
//! Boden, Bögen, Pfeile und Fadenkreuze der Planung in jeder Darstellung und die stummen Effekte (Bolzen, verpuffender
//! Bolzen, Zielmarke, Strahl, Rakete mit Leuchtspur, Schildblase, Impuls, Hyperraum-Austritt mit Blitz und Partikeln,
//! Brand mit Rauch). Neue Darstellungsvarianten gehören deshalb auch hierher. Am Ende des Ladens verschwindet alles
//! wieder; die Effekte räumen sich selbst weg.

use bevy::prelude::*;

use voidfleet_core::{
    board::{LaneType, Side, SlotRef},
    protocol::ShipStats,
};

use super::{
    coords::{BOARD_CENTER, facing},
    effects::{Beam, Bolt, Bubble, Burn, Fizzle, Missile, Pulse, TargetRing, Warp},
    markers::{MarkerKind, MarkerSpec, ShipMarkers},
    overlays::{MoveArrow, PlanOverlays, TargetLine},
    ship::{BreakApart, ToonState, spawn_ship},
    status::ShipStatus,
};
use crate::{assets::AppState, data::ship_models::ShipModels, palette};

/// Alle Marken, reihum auf die Schiffe verteilt.
const MARKERS: [MarkerKind; 8] = [
    MarkerKind::Lock,
    MarkerKind::Taunt,
    MarkerKind::Ready,
    MarkerKind::Aim,
    MarkerKind::Counterfire,
    MarkerKind::FocusFire,
    MarkerKind::SwarmStrike,
    MarkerKind::HeadStart,
];

pub(super) fn stage(
    mut commands: Commands,
    models: Res<ShipModels>,
    mut overlays: ResMut<PlanOverlays>,
) {
    let ships: Vec<_> = models.iter().map(|(id, _)| id).collect();
    let mut wrecks = Vec::new();
    for (index, ship) in ships.into_iter().enumerate() {
        for (row, ghost, wreck) in [(0, false, false), (1, true, false), (2, false, true)] {
            let position =
                BOARD_CENTER + Vec3::new(row as f32 * 2.0 - 2.0, 0.0, index as f32 - 7.0);
            let side = if index % 2 == 0 {
                Side::Player
            } else {
                Side::Enemy
            };
            let entity = spawn_ship(&mut commands, &models, ship, ghost);
            commands.entity(entity).insert((
                Transform::from_translation(position)
                    .looking_to(Vec3::X * facing(side), Vec3::Y)
                    .with_scale(Vec3::splat(super::ship::BOARD_SCALE)),
                ToonState {
                    heat: wreck.then_some(LinearRgba::rgb(1.0, 0.5, 0.2)),
                    ..ToonState::facing(side)
                },
                ShipStatus {
                    preview: Some(ShipStats {
                        hp: i32::from(index % 3 > 0),
                        shield: 0,
                    }),
                    doomed: true,
                    holding_fire: index % 2 == 0,
                    ..ShipStatus::new(ship)
                },
                ShipMarkers(vec![MarkerSpec {
                    planned: index % 2 == 0,
                    ..MarkerSpec::new(MARKERS[index % MARKERS.len()], palette::SELECT)
                }]),
                DespawnOnExit(AppState::Loading),
            ));
            if wreck {
                commands.trigger(BreakApart { entity });
                wrecks.push(entity);
            }
        }
    }

    // Bögen in jeder Darstellung, Pfeile mit und ohne Anhebung, Fadenkreuze.
    let own = |lane, index| SlotRef::new(Side::Player, lane, index);
    let enemy = |lane, index| SlotRef::new(Side::Enemy, lane, index);
    overlays.arrows = vec![
        MoveArrow {
            from: own(LaneType::Command, 0),
            to: own(LaneType::Strike, 0),
            lifted: false,
        },
        MoveArrow {
            from: own(LaneType::Command, 1),
            to: own(LaneType::Strike, 1),
            lifted: true,
        },
    ];
    overlays.lines = [
        (false, false, false),
        (true, true, false),
        (true, false, true),
        (false, true, true),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (fizzle, tracking, selected))| TargetLine {
        from: own(LaneType::Strike, index),
        to: enemy(LaneType::Strike, index),
        side: if index % 2 == 0 {
            Side::Player
        } else {
            Side::Enemy
        },
        fizzle,
        tracking,
        selected,
    })
    .collect();
    overlays.reticles = vec![enemy(LaneType::Command, 2)];

    // Die stummen Effekte; was Klang macht (Einschläge, Explosionen), bleibt der Probe fern.
    let at = |x: f32| BOARD_CENTER + Vec3::new(x, 0.6, 3.0);
    commands.spawn(Bolt {
        from: at(-3.0),
        to: at(3.0),
        color: palette::PLAYER,
    });
    commands.spawn(Fizzle {
        from: at(3.0),
        to: at(-3.0),
        color: palette::ENEMY,
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
    commands.spawn(Missile {
        from: at(-4.0),
        to: at(4.0),
        color: palette::ENEMY,
    });
    commands.spawn(Bubble {
        position: at(-1.0),
        color: palette::SHIELD,
        size: 2.0,
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

/// Nach der Probe gehen die Bögen und Pfeile wieder.
pub(super) fn clear(mut overlays: ResMut<PlanOverlays>) {
    *overlays = PlanOverlays::default();
}
