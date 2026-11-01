//! Look-Parameter von Clean Toon, dieselben Werte wie `tools/ships/look.py` und `scene/toon.ts`. Die Konstanten des
//! Fragment-Shaders (Stufen, Kantenlicht, Glanzpunkt, Blitzlicht) stehen in `assets/shaders/toon.wesl`.

use bevy::prelude::*;

/// Hauptlicht `KEY["board"]` (−0,25, −0,75, 0,62) aus Blender-Schiffskoordinaten in die Welt eines eigenen Schiffs
/// (Nase +x): von vorn oben, leicht von der Kameraseite der Seitenansicht (+z). Für den Gegner ist x gespiegelt.
pub const KEY: Vec3 = Vec3::new(0.75 / KEY_LENGTH, 0.62 / KEY_LENGTH, 0.25 / KEY_LENGTH);
/// Länge von (0,75, 0,62, 0,25).
const KEY_LENGTH: f32 = 1.004_689;

/// Schwellen des Kantenlichts auf N·R und facing und die Grenze auf N·oben, für eckige und runde Flächen. Eckige
/// Flächen, die nach oben zeigen (weniger als 37° gegen die Waagrechte), bekommen keins.
pub const RIM_FLAT: [f32; 3] = [0.28, 0.4, 0.8];
pub const RIM_ROUND: [f32; 3] = [0.0, 0.62, 2.0];

/// Konturfarbe (`OUTLINE` in `palette.py`) und Breite der Außenkontur in logischen Pixeln (`LINES["board"]`).
pub const LINE_COLOR: Color = Color::srgb(11.0 / 255.0, 14.0 / 255.0, 21.0 / 255.0);
pub const CONTOUR_WIDTH: f32 = 1.8;

/// Lichter mit `glow` leuchten so viel heller als 1 und glühen dadurch im Bloom der Kamera.
pub const GLOW_BOOST: f32 = 1.8;

/// Geister eingeplanter Schiffe: bläuliches Hologramm in drei Tönen, ohne Konturen.
pub const GHOST_TONES: [Color; 3] = [
    Color::srgb(0x8f as f32 / 255.0, 0xdc as f32 / 255.0, 1.0),
    Color::srgb(
        0x4d as f32 / 255.0,
        0x9f as f32 / 255.0,
        0xd6 as f32 / 255.0,
    ),
    Color::srgb(
        0x2a as f32 / 255.0,
        0x5f as f32 / 255.0,
        0x99 as f32 / 255.0,
    ),
];
pub const GHOST_LIGHT: Color = Color::srgb(0xd6 as f32 / 255.0, 0xf6 as f32 / 255.0, 1.0);
pub const GHOST_ALPHA: f32 = 0.45;
pub const GHOST_HOLOGRAM_ALPHA: f32 = 0.6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_normalized() {
        assert!((KEY.length() - 1.0).abs() < 1e-5);
    }
}
