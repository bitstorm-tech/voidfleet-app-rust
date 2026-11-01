//! Kartenillustrationen (`images/cards/<id>.jpg`, erzeugt mit `tools/generateCardArt.ts`), auch Flaggschiffe, Drohne
//! und Trugbild.

use bevy::{platform::collections::HashMap, prelude::*};

use crate::assets::Preload;

/// Alle Karten mit eigenem Bild.
///
/// TODO(core): aus den Kartendefinitionen von `voidfleet_core` ableiten.
pub const CARDS: [&str; 47] = [
    "horizon_cruiser",
    "iron_dreadnought",
    "plasma_frigate",
    "repair_drone",
    "scrap_raider",
    "signal_booster",
    "titan_carrier",
    "void_interceptor",
    "drone",
    "decoy",
    "bastion",
    "vanguard",
    "hive",
    "mirage",
    "pendulum",
    "emergency_shields",
    "emp_blast",
    "flanking_maneuver",
    "intercept",
    "orbital_strike",
    "overcharge",
    "power_surge",
    "recall",
    "scavenge",
    "targeting_jammer",
    "volley_fire",
    "torpedo",
    "barrage",
    "evasive_action",
    "micro_jump",
    "rapid_deployment",
    "draw_fire",
    "shield_grid",
    "ablative_armor",
    "covering_fire",
    "drone_squadron",
    "collision_course",
    "countermeasures",
    "carte_blanche",
    "radio_silence",
    "minefield",
    "tractor_beam",
    "heat_seeker",
    "coup_de_grace",
    "piercing_shot",
    "bombardment",
    "crossfire",
];

/// Welches Bild eine Karte zeigt: ihr eigenes, nur die Schrottverwertung zeigt den Scrap Raider, für den sie ist.
///
/// TODO(core): Schlüssel gegen die Karten-Id aus `voidfleet_core` tauschen.
pub fn art_key(card: &str) -> &str {
    match card {
        "scrap_salvage" => "scrap_raider",
        card => card,
    }
}

/// Asset-Pfad der Illustration.
pub fn card_art_path(card: &str) -> String {
    format!("images/cards/{}.jpg", art_key(card))
}

/// Die Illustrationen aller Karten, beim Start geladen.
#[derive(Resource)]
pub struct CardArt(HashMap<&'static str, Handle<Image>>);

impl CardArt {
    /// Das Bild einer Karte (TODO(core): Karten-Id statt `&str`).
    pub fn get(&self, card: &str) -> Handle<Image> {
        self.0[art_key(card)].clone()
    }
}

pub(super) fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    let art = CARDS
        .into_iter()
        .map(|card| (card, preload.track(server.load(card_art_path(card)))))
        .collect();
    commands.insert_resource(CardArt(art));
}
