//! Wohin der Client sich verbindet, aus der Kommandozeile (Ersatz für die Electron-App und den Match-Link):
//!
//! - ohne Argumente: Der Client startet den Server eingebettet (nur auf `127.0.0.1`), legt ein neues Match an und
//!   spielt gegen den Computergegner.
//! - `--server ws://host:port [--match <id>] [--bot]`: Verbindung zu einem entfernten Server. Mit `--match` tritt er
//!   diesem Match bei (PvP über die geteilte Match-Id), ohne legt er ein neues an; `--bot` setzt beim Anlegen den
//!   Computergegner auf den zweiten Platz.

use std::fmt::Write;

use bevy::prelude::*;

pub const USAGE: &str = "\
Aufruf: voidfleet [--server ws://host:port [--match <id>] [--bot]]

Ohne Argumente startet der Server eingebettet, und du spielst ein neues Match gegen den Computer.
  --server <url>   Server, mit dem sich der Client verbindet (z. B. ws://192.168.1.20:3000)
  --match <id>     Match-Id, der beigetreten wird; ohne entsteht ein neues Match
  --bot            neues Match gleich gegen den Computer anlegen";

/// Der gewählte Start.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub enum Launch {
    /// Eingebetteter Server, neues Match gegen den Computergegner.
    #[default]
    Local,
    Remote {
        /// Adresse des Servers ohne Pfad, etwa `ws://host:3000`.
        server: String,
        match_id: Option<String>,
        bot: bool,
    },
}

impl Launch {
    /// Liest die Argumente (ohne den Programmnamen); ein Fehler nennt das Problem, [`USAGE`] den Aufruf.
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut server = None;
        let mut match_id = None;
        let mut bot = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--server" => server = Some(args.next().ok_or("--server braucht eine Adresse")?),
                "--match" => match_id = Some(args.next().ok_or("--match braucht eine Id")?),
                "--bot" => bot = true,
                other => return Err(format!("unbekanntes Argument: {other}")),
            }
        }
        match server {
            Some(server) if server.starts_with("ws://") => Ok(Launch::Remote {
                server: server.trim_end_matches('/').to_owned(),
                match_id,
                bot,
            }),
            Some(server) => Err(format!("--server erwartet ws://host:port, nicht {server}")),
            None if match_id.is_some() || bot => {
                Err("--match und --bot gehen nur mit --server".into())
            }
            None => Ok(Launch::Local),
        }
    }
}

/// Alles, was es für den Beitritt braucht.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub server: String,
    pub match_id: String,
    /// Identifiziert den Spieler beim Server; je Start neu.
    pub token: String,
    /// Das Match entsteht gleich gegen den Computergegner.
    pub bot: bool,
}

impl Endpoint {
    /// Ein neues Match bzw. der Beitritt zu `match_id`, mit frischem Token.
    pub fn new(server: String, match_id: Option<String>, bot: bool) -> Self {
        Endpoint {
            server,
            match_id: match_id.unwrap_or_else(|| random_id(4)),
            token: random_id(16),
            bot,
        }
    }

    /// Die URL des WebSockets.
    pub fn url(&self) -> String {
        let mut url = format!(
            "{}/ws?match={}&token={}",
            self.server, self.match_id, self.token
        );
        if self.bot {
            url.push_str("&bot");
        }
        url
    }
}

/// Zufällige Hex-Id aus `bytes` Bytes.
fn random_id(bytes: usize) -> String {
    (0..bytes).fold(String::with_capacity(bytes * 2), |mut id, _| {
        write!(id, "{:02x}", rand::random::<u8>()).expect("String nimmt alles");
        id
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Launch, String> {
        Launch::from_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn arguments() {
        assert_eq!(parse(&[]), Ok(Launch::Local));
        assert_eq!(
            parse(&["--server", "ws://host:3000/", "--match", "abcd"]),
            Ok(Launch::Remote {
                server: "ws://host:3000".into(),
                match_id: Some("abcd".into()),
                bot: false,
            })
        );
        assert!(parse(&["--match", "abcd"]).is_err());
        assert!(parse(&["--server", "http://host"]).is_err());
        assert!(parse(&["--server"]).is_err());
        assert!(parse(&["--fast"]).is_err());
    }

    #[test]
    fn url_and_ids() {
        let endpoint = Endpoint::new("ws://127.0.0.1:3000".into(), None, true);
        assert_eq!(endpoint.match_id.len(), 8);
        assert_eq!(endpoint.token.len(), 32);
        assert!(endpoint.token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            endpoint.url(),
            format!(
                "ws://127.0.0.1:3000/ws?match={}&token={}&bot",
                endpoint.match_id, endpoint.token
            )
        );
        assert_ne!(
            Endpoint::new(endpoint.server.clone(), None, false).token,
            endpoint.token
        );
    }
}
