//! Startpunkt des Desktop-Clients: Das ganze Spiel steckt in [`ClientPlugin`].

use bevy::prelude::*;
use voidfleet_client::ClientPlugin;

fn main() -> AppExit {
    App::new().add_plugins(ClientPlugin).run()
}
