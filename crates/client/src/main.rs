//! Startpunkt des Desktop-Clients: Das ganze Spiel steckt in [`ClientPlugin`], wohin er sich verbindet, sagt die
//! Kommandozeile ([`Launch`]).

use bevy::prelude::*;
use voidfleet_client::{
    ClientPlugin,
    net::{Launch, USAGE},
};

fn main() -> AppExit {
    let launch = match Launch::from_args(std::env::args().skip(1)) {
        Ok(launch) => launch,
        Err(error) => {
            eprintln!("{error}\n\n{USAGE}");
            return AppExit::from_code(2);
        }
    };
    App::new()
        .insert_resource(launch)
        .add_plugins(ClientPlugin)
        .run()
}
