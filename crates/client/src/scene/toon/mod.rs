//! Clean Toon: Material, Konturen und Glühen der Schiffe und Wracks (Port von `scene/toon.ts` und `scene/glow.ts`),
//! gebaut nach dem Material-Vertrag aus `tools/ships/vf.py` und den Werten aus `tools/ships/look.py`.
//!
//! - **Material** ([`ToonMaterial`], `assets/shaders/toon.wesl`): ein Material für alle Arten des Vertrags (`toon`,
//!   `light`, `hologram`, `wreck`). Licht ist gerechnet, nicht beleuchtet — die Szene hat keine Lichter: drei Töne
//!   hart über N·L des Hauptlichts gestuft, Kantenlicht von hinten oben, Saum, Glanzpunkt; Lichter unbeleuchtet in
//!   genau ihrer Farbe; Hologramme durchscheinend und beidseitig; Bruchflächen mit Glutmuster × Glut. Das Hauptlicht
//!   hängt fest am Brett, je Seite gespiegelt; Drehen und Zoomen ändern keinen Ton. Das Blitzlicht der Explosionen
//!   ([`ToonFlash`]) hellt die zugewandten Flächen in zwei harten Stufen auf.
//! - **Verwitterung**: Die toon-Flächen eines Modells teilen eine gebackene Maske (`tools/ships/weather.py`), aus der
//!   der Shader nach den drei Tönen weitere harte Tonflächen setzt: Plattenton, Fugen, Schmutz, Ruß, Kantenabrieb.
//!   Fugen und Abrieb verschwinden, wenn ein Texel kleiner als etwa ein Pixel wird, der Rest über die Mip-Kette.
//! - **Konturen** ([`outline`]): Inverted Hull, Breite in Welt-Einheiten auf ein Pixelband geklemmt, Farbe ein
//!   abgedunkelter Ton der Fläche.
//! - **Glühen**: Lichter mit `glow`, glühende Hologramme und heiße Bruchflächen leuchten heller als 1 und glühen im
//!   Bloom der Kamera ([`crate::scene::camera`]).
//!
//! Die Materialien entstehen beim Laden der Modelle ([`gltf`]); jedes Schiff bekommt beim Spawnen eigene Kopien
//! ([`crate::scene::ship`]).

mod gltf;
pub mod look;
pub mod material;
pub mod outline;

use bevy::prelude::*;

pub use material::{ToonMaterial, ToonSpec};
pub use outline::{OutlineMaterial, ToonOutline};

/// Das eine Blitzlicht der Szene: Effekte schreiben jedes Bild den gerade stärksten Blitz hinein (Weltlage und Stärke,
/// Bestrahlung = Stärke · cos / (1 + Abstand²)); 0 heißt kein Blitz.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ToonFlash {
    pub position: Vec3,
    pub intensity: f32,
}

pub(super) struct ToonPlugin;

impl Plugin for ToonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<ToonMaterial>::default(),
            MaterialPlugin::<OutlineMaterial>::default(),
        ))
        .register_type::<ToonOutline>()
        .init_resource::<ToonFlash>()
        .add_systems(
            Update,
            (
                outline::sync_width,
                apply_flash.run_if(resource_changed::<ToonFlash>),
            ),
        );
        gltf::register(app);
    }
}

/// Gibt den Blitz an alle Toon-Materialien weiter.
fn apply_flash(flash: Res<ToonFlash>, mut materials: ResMut<Assets<ToonMaterial>>) {
    let value = flash.position.extend(flash.intensity);
    for (_, material) in materials.iter_mut() {
        material.flash = value;
    }
}
