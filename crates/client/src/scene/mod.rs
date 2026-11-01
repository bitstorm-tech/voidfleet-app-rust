//! Die 3D-Szene des Match-Bretts: Kamera mit zwei Ansichten ([`camera`]), Hintergrund ([`background`]), Tweens
//! ([`tween`]), Toon-Rendering ([`toon`]) und Schiffe samt Wracks ([`ship`]). Koordinaten: [`coords`].
//!
//! Lichter hat die Szene keine: Die Schiffe rechnen ihr Licht selbst, alles andere leuchtet aus sich.

pub mod background;
pub mod camera;
pub mod coords;
mod rehearsal;
pub mod ship;
pub mod toon;
pub mod tween;

use bevy::prelude::*;

use crate::assets::LoadingPhase;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            tween::TweenPlugin,
            camera::CameraPlugin,
            background::BackgroundPlugin,
            toon::ToonPlugin,
            ship::ShipPlugin,
        ))
        .add_systems(OnEnter(LoadingPhase::Rehearsal), rehearsal::stage);
    }
}
