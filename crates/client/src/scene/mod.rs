//! Die 3D-Szene des Match-Bretts (Port von `client/src/scene/` und `ui/floatingText.ts`). Koordinaten: [`coords`].
//!
//! - Grundlagen: Kamera mit zwei Ansichten und Eingabe ([`camera`]), Hintergrund ([`background`]), Toon-Rendering
//!   ([`toon`]), Schiffe und Wracks ([`ship`]), selbstleuchtendes Material und Billboards ([`fx`]), gezeichnete
//!   Texturen ([`paint`]).
//! - Brett: Pads, Anker, Beschriftungen, Spaltenpfeile, Hervorhebung ([`board`]); Statusbalken, Krone, Totenkopf und
//!   Grenzen der Schiffe ([`status`]); Marken am Boden ([`markers`]); Pfeile, Bögen und Fadenkreuze der Planung
//!   ([`overlays`]).
//! - Zustand: [`sync`] stellt die Szene aus dem Spielzustand, solange keine Runde abgespielt wird ([`sync::BoardMode`]);
//!   [`picking`] meldet Klicks und Zeiger auf Slots, Schiffe und Spaltenpfeile.
//! - Wiedergabe: Animationen als Aufträge mit Fertig-Rückmeldung, gestreckt um die Animationsgeschwindigkeit
//!   ([`animation`]): Schiffe ([`motion`], Bahnen in [`maneuver`]), Effekte ([`effects`], Partikel in [`particles`]),
//!   Zerstörung ([`destruction`]), Schwebetexte ([`floating_text`]).
//!
//! Lichter hat die Szene keine: Die Schiffe rechnen ihr Licht selbst, alles andere leuchtet aus sich.

pub mod animation;
pub mod background;
pub mod board;
pub mod camera;
pub mod coords;
pub mod destruction;
pub mod effects;
pub mod floating_text;
pub mod fx;
pub mod maneuver;
pub mod markers;
pub mod motion;
pub mod overlays;
pub mod paint;
pub mod particles;
pub mod picking;
mod rehearsal;
pub mod ship;
pub mod status;
pub mod sync;
pub mod toon;

use bevy::prelude::*;

use crate::assets::LoadingPhase;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            background::BackgroundPlugin,
            toon::ToonPlugin,
            ship::ShipPlugin,
            fx::FxPlugin,
            animation::AnimationPlugin,
            particles::ParticlesPlugin,
            effects::EffectsPlugin,
        ))
        .add_plugins((
            board::BoardPlugin,
            status::StatusPlugin,
            markers::MarkersPlugin,
            overlays::OverlaysPlugin,
            picking::PickingPlugin,
            motion::MotionPlugin,
            destruction::DestructionPlugin,
            floating_text::FloatingTextPlugin,
            sync::SyncPlugin,
        ))
        .add_systems(OnEnter(LoadingPhase::Rehearsal), rehearsal::stage)
        .add_systems(OnExit(LoadingPhase::Rehearsal), rehearsal::clear);
    }
}
