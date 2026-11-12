//! Die zehn Weltraum-Hintergründe (`images/backgrounds/`, Prompts in `prompts.json`); jedes Match zeigt einen davon.

use bevy::prelude::*;

use crate::assets::{Preload, gpu_image};

pub const BACKGROUNDS: [&str; 10] = [
    "01-nebelschlucht",
    "02-ringplanet",
    "03-asteroidenguertel",
    "04-zerbrochener-mond",
    "05-orbitalruinen",
    "06-eisnebel",
    "07-sternenwerft",
    "08-ereignishorizont",
    "09-doppelsonnen",
    "10-ionensturm",
];

pub fn background_path(name: &str) -> String {
    format!("images/backgrounds/{name}.png")
}

/// Alle Hintergründe, beim Start geladen (zusammen etwa 14 MB).
#[derive(Resource)]
pub struct Backgrounds(pub Vec<Handle<Image>>);

impl FromWorld for Backgrounds {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>().clone();
        let mut preload = world.resource_mut::<Preload>();
        Self(
            BACKGROUNDS
                .into_iter()
                .map(|name| preload.track(gpu_image(&server, background_path(name))))
                .collect(),
        )
    }
}
