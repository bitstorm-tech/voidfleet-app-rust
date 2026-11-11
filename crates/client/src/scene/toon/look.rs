//! Look-Parameter der Schiffe im dreckigen Look. Die Konstanten des Fragment-Shaders (Verlauf, Verwitterung,
//! Kantenlicht, Glanz, Blitzlicht) stehen in `assets/shaders/toon.wesl`. Die Vorschau in Blender
//! (`tools/ships/look.py`, `preview.py`) rendert noch den früheren Clean-Toon-Look; gemeinsam sind Hauptlicht, Töne
//! und die Maske der Verwitterung.

use bevy::prelude::*;

/// Hauptlicht `KEY["board"]` (−0,25, −0,75, 0,62) aus Blender-Schiffskoordinaten in die Welt eines eigenen Schiffs
/// (Nase +x): von vorn oben, leicht von der Kameraseite der Seitenansicht (+z). Für den Gegner ist x gespiegelt.
pub const KEY: Vec3 = Vec3::new(0.75 / KEY_LENGTH, 0.62 / KEY_LENGTH, 0.25 / KEY_LENGTH);
/// Länge von (0,75, 0,62, 0,25).
const KEY_LENGTH: f32 = 1.004_689;

/// Breite der roten Kontur am Ziel unter dem Fadenkreuz in Welt-Einheiten (eine Blender-Einheit sind auf dem Brett
/// 0,3), geklemmt auf [`CONTOUR_MIN`, `CONTOUR_MAX`] logische Pixel: auf dem Brett (≈ 22 px je Blender-Einheit)
/// etwa 1 px, herangezoomt bis 1,8 px. Andere Konturen gibt es nicht.
pub const CONTOUR_WORLD: f32 = 0.0135;
pub const CONTOUR_MIN: f32 = 1.0;
pub const CONTOUR_MAX: f32 = 1.8;

/// Texel der Verwitterungsmaske je Blender-Einheit (`WEATHER_DENSITY` in `look.py`); darauf beziehen sich die
/// Schwellen `FINE_LOD` im Shader. Ein Schiff mit anderer Dichte nennt sie im Vertrag (`ToonSpec::Toon::density`).
pub const WEATHER_DENSITY: f32 = 32.0;

/// Lichter mit `glow` leuchten so viel heller als 1 und glühen dadurch im Bloom der Kamera.
pub const GLOW_BOOST: f32 = 1.8;

/// Geister eingeplanter Schiffe: bläuliches Hologramm in drei Tönen.
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

/// Ausgebranntes Modell eines Kreuzer-Paars: fast schwarz überlagert (rgb linear, a Anteil).
pub const SCORCHED: Vec4 = Vec4::new(0.02, 0.014, 0.01, 0.72);

/// Spreizung der drei Töne (Toon und Wrack): Halbschatten und Schatten rücken in Oklch-Lightness um diesen Faktor vom
/// Licht-Ton weg, damit Deck, Flanke und Plattenlagen auf dem kleinen Brett auseinandergehen. Experiment im Client;
/// bewährt es sich, wandert es nach `tones()` in `tools/ships/palette.py` und fällt hier weg.
pub const TONE_SPREAD: f32 = 1.4;

/// Lackierung der Gegner mit den alten Rümpfen (nur im Client, ohne Gegenstück in `look.py`): Lichter nehmen den
/// Farbton der Gegnerfarbe an, ihr Chroma rückt um diesen Anteil zu deren Chroma; die Rumpftöne bleiben.
pub const LIVERY_ACCENT: f32 = 0.7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_normalized() {
        assert!((KEY.length() - 1.0).abs() < 1e-5);
    }
}
