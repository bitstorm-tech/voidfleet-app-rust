//! Der dreckige Look der Schiffe und Wracks: Material, Zielkontur und Glühen, gebaut nach dem Material-Vertrag aus
//! `tools/ships/vf.py`. Modul, Material und Vertrag heißen noch nach dem früheren Clean-Toon-Look.
//!
//! - **Material** ([`ToonMaterial`], `assets/shaders/toon.wesl`): ein Material für alle Arten des Vertrags (`toon`,
//!   `light`, `hologram`, `wreck`). Licht ist gerechnet, nicht beleuchtet — die Szene hat keine Lichter: drei Töne als
//!   matter Verlauf über N·L des Hauptlichts, Kantenlicht am Umriss, Glanz nur an Materialien mit Glanz und an blankem
//!   Metall; Lichter unbeleuchtet in genau ihrer Farbe; Hologramme durchscheinend und beidseitig; Bruchflächen mit
//!   Glutmuster × Glut. Das Hauptlicht hängt fest am Brett, je Seite gespiegelt. Das Blitzlicht der Explosionen
//!   ([`ToonFlash`]) hellt die zugewandten Flächen in zwei harten Stufen auf.
//! - **Verwitterung**: Die toon-Flächen eines Modells und seines Wracks teilen eine gebackene Maske
//!   (`tools/ships/weather.py`, `<id>_weather.ktx2`): Helligkeit aus Plattenton und Verdeckung, Fugen, Schmutz, Ruß,
//!   Kantenabrieb. Die Plattenkanten fallen als Fase zur Fuge ab (die Normale neigt sich), Schmutz und Ruß setzen als
//!   Verlauf ein. Harte Rille, Fase und Abrieb verschwinden, wenn sie zu fein für den Bildschirm werden, der Rest über
//!   die Mip-Kette.
//! - **Feinstruktur**: Über allen Rümpfen liegt `detail.ktx2` (`tools/ships/detail.py`), aus den drei Achsen des
//!   Schiffs projiziert: Kratzer, Poren und Beulen als Relief, Flecken im Ton, Korn an den Rändern von Schmutz und
//!   Abrieb. Sie trägt die Nahaufnahme; auf dem Brett mittelt die Mip-Kette sie weg.
//! - **Zielkontur** ([`outline`]): Inverted Hull in Rot, nur am Ziel unter dem Fadenkreuz sichtbar. Andere Konturen
//!   gibt es nicht; so dürfen die Modelle Kleinteile tragen, ohne dass Konturringe sie zu Flecken verschmelzen.
//! - **Glühen**: Lichter mit `glow`, glühende Hologramme und heiße Bruchflächen leuchten heller als 1 und glühen im
//!   Bloom der Kamera ([`crate::scene::camera`]).
//!
//! Die Materialien entstehen beim Laden der Modelle ([`gltf`]); jedes Schiff bekommt beim Spawnen eigene Kopien
//! ([`crate::scene::ship`]).

mod gltf;
pub mod look;
pub mod material;
pub mod outline;

use bevy::{prelude::*, render::storage::ShaderBuffer};

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

/// Schreibt den Blitz in den Puffer, den alle Toon-Materialien lesen (läuft auch im ersten Bild und legt ihn an).
fn apply_flash(flash: Res<ToonFlash>, mut buffers: ResMut<Assets<ShaderBuffer>>) {
    let value = flash.position.extend(flash.intensity).to_array();
    buffers
        .insert(&material::FLASH, ShaderBuffer::from(vec![value]))
        .expect("UUID-Handle");
}
