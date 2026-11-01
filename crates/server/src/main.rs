//! Der eigenständige Server: `PORT` (Standard 3000) und `HOST` (Standard `0.0.0.0`) aus der Umgebung.

use std::env;
use std::error::Error;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port: u16 = env::var("PORT").map_or(Ok(3000), |port| port.parse())?;
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let listener = TcpListener::bind((host.as_str(), port)).await?;
    println!(
        "Voidfleet-Server läuft auf ws://{}/ws",
        listener.local_addr()?
    );
    voidfleet_server::serve(listener).await?;
    Ok(())
}
