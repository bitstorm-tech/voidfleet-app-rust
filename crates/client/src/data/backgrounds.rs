//! Die zehn Weltraum-Hintergründe (`images/backgrounds/`, Prompts in `prompts.json`); jedes Match zeigt einen davon.

use bevy::prelude::*;

use crate::assets::Preload;

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

pub(super) fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let images = BACKGROUNDS
        .into_iter()
        .map(|name| preload.track(server.load(background_path(name))))
        .collect();
    commands.insert_resource(Backgrounds(images));
}
