//! Generalprobe hinter dem Ladebildschirm (Port von `scene/rehearsal.ts`): Einmal stehen alle Schiffe samt Drohne,
//! Trugbild und Flaggschiffen, ihre Geister und ihre Wracks mit glühenden Bruchflächen auf dem Brett, damit Bevy alle
//! Pipelines übersetzt, bevor das Match beginnt. Neue Darstellungsvarianten gehören deshalb auch hierher. Am Ende des
//! Ladens verschwindet alles wieder.

use bevy::prelude::*;

use voidfleet_core::board::Side;

use super::{
    coords::{BOARD_CENTER, facing},
    ship::{BreakApart, ToonState, spawn_ship},
};
use crate::{assets::AppState, data::ship_models::ShipModels};

pub(super) fn stage(mut commands: Commands, models: Res<ShipModels>) {
    let ships: Vec<_> = models.iter().map(|(id, _)| id).collect();
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
                DespawnOnExit(AppState::Loading),
            ));
            if wreck {
                commands.trigger(BreakApart { entity });
            }
        }
    }
}
