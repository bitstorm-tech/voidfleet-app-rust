//! Koordinaten der Szene.
//!
//! Bevy ist rechtshändig mit Y oben und blickt mit Kameras und `Transform::forward` nach −Z; Babylon ist linkshändig
//! und blickt nach +Z, Godot ist rechtshändig wie Bevy. Ein Punkt aus dem TypeScript-Client (Babylon) wird hier zu
//! **(x, y, −z)**: x (quer über das Brett) und y (oben) bleiben, z wird negiert ([`from_babylon`]). Damit stimmen die
//! Werte wieder mit der Godot-Szene (`match_screen.tscn`) überein.
//!
//! - Die eigene Seite liegt bei negativem x, der Gegner bei positivem x; die Strike-Lanes liegen innen.
//! - Die Brettmitte liegt bei z = [`BOARD_Z`] (−2,5). Slot 0 liegt am Ende der Lane mit dem **kleinsten** z: von der
//!   Seite gesehen hinten (weit von der Kamera), von hinten oben links ([`slot_z`]).
//! - Die Kamera der Seitenansicht steht bei positivem z und blickt nach −z; die Ansicht von hinten oben steht bei
//!   negativem x und blickt nach +x ([`crate::scene::camera`]).
//! - Schiffe zeigen mit der Nase nach ihrem `Transform::forward` (lokal −Z); ein eigenes Schiff blickt also nach +x,
//!   ein gegnerisches nach −x. Die glTF-Modelle (Nase glTF +Z) dreht der glTF-Loader dafür um 180° um Y
//!   (`GltfConvertCoordinates::rotate_scene_entity`).
//! - Das Hauptlicht der Schiffe (`KEY` in [`crate::scene::toon`]) steht fest am Brett: für ein eigenes Schiff
//!   `(0.75, 0.62, 0.25)`, also von vorn oben und leicht von der Kameraseite der Seitenansicht; für den Gegner ist x
//!   gespiegelt.

use bevy::prelude::*;
use voidfleet_core::board::Side;

/// Weltlage der Brettmitte (z).
pub const BOARD_Z: f32 = -2.5;
pub const BOARD_CENTER: Vec3 = Vec3::new(0.0, 0.0, BOARD_Z);

/// Landeplatz: Breite (x) und Tiefe (z) eines Pads.
pub const PAD_WIDTH: f32 = 2.5;
pub const PAD_DEPTH: f32 = 1.7;
/// Slots je Lane.
pub const SLOTS: usize = 6;

/// Wandelt einen Punkt oder eine Richtung aus dem Babylon-Client in Bevy-Koordinaten.
pub const fn from_babylon(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, -v.z)
}

/// Lane-z eines Slots relativ zur Brettmitte: Slot 0 am Ende mit dem kleinsten z, die sechs Slots symmetrisch um die
/// Lane-Mitte, `step` auseinander.
pub fn slot_z(index: usize, step: f32) -> f32 {
    (index as f32 - (SLOTS as f32 - 1.0) / 2.0) * step
}

/// +1 für die eigene Seite (Nase nach +x), −1 für den Gegner.
pub const fn facing(side: Side) -> f32 {
    match side {
        Side::Player => 1.0,
        Side::Enemy => -1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_zero_is_far_end() {
        assert_eq!(slot_z(0, 2.0), -5.0);
        assert_eq!(slot_z(5, 2.0), 5.0);
        assert_eq!(
            from_babylon(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, -3.0)
        );
    }
}
