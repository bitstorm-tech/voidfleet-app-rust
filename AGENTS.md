# Voidfleet

Das ist Voidfleet, ein asynchroner PvP-Deckbuilder, der im Science-Fiction-Weltraum-Universum spielt. Der Spieler befehligt eine Flotte aus Raumschiffen. Schiffe werden auf das Spielbrett (zwei Lanes pro Seite: Strike und Command) gelegt und mit Befehlen unterstützt, zum Beispiel Angriff, Reparieren, Schilde aufladen. Schiffe und Befehle werden jeweils als Karte mit unterschiedlichen Werten dargestellt und kommen aus zwei getrennten Decks.

Das Besondere an Voidfleet ist der asynchrone Spielmodus mit Simultaneous Blind Play: Beide Spieler wählen ihren Zug gleichzeitig und blind, ohne Timer — ein Zug darf beliebig lange dauern. Vor der ersten Runde wählt jeder verdeckt einen von fünf Admiralen (die Mauer, der Hammer, der Schwarm, das Phantom, der Uhrmacher). Jeder bringt sein Flaggschiff mit, ein Kampfschiff mit eigener Fähigkeit, das er in Runde 1 zusätzlich legt, und eine Admiralsfähigkeit. Ein Zug besteht aus bis zu 1 Schiff (in Runde 1 zwei plus Flaggschiff), 1 Befehl und entweder einem Grundmanöver oder der Admiralsfähigkeit: Beim Grundmanöver wechselt ein Schiff auf dem Brett in einen freien Slot, in der anderen oder derselben Lane, und feuert dafür in dieser Runde nicht. Dazu gibt der Spieler jedem seiner Kampfschiffe einen Feuerbefehl: entweder eine Spalte der Gegnerseite (Spaltenfeuer: trifft das vorderste Schiff, das dort steht, wer ausweicht, entgeht ihm) oder ein gegnerisches Schiff (Schiffsfeuer: folgt ihm, verpufft aber, wenn es gedeckt ist); ohne Feuerbefehl feuert es nicht, eine automatische Zielwahl gibt es nicht. Sobald beide submitted haben, wird aufgedeckt und in festen Phasen aufgelöst (Deploy → Manöver → Effects → Combat → Cleanup). Wessen Flaggschiff zerstört wird, hat verloren, und die Partie endet sofort, der Rest der Runde entfällt; fallen beide im selben Schritt (etwa gleichzeitig im Kampf), ist es unentschieden; ab Runde 8 bekommen alle Kampfschiffe Overdrive (+1 ATK, alle 4 Runden einen mehr), damit Partien nicht im Patt enden. Nach jeder Runde gibt es Nachschub: Aus 3 Schiffen und 3 Befehlen nimmt jeder je eine Karte auf die Hand, der Rest geht zurück ins Deck; hat ein eigener Scrap Raider Schrott geborgen, ist einer der Befehle die Schrottverwertung, eine Befehlskarte für die Hand (+1 ATK dauerhaft für einen Scrap Raider). Es gibt kein Energie-/Mana-System.

## Status

Wir sind momentan noch in der Entwicklungsphase. Das Spiel ist also nicht live. Aus diesem Grund können wir den Code jederzeit komplett ändern und es ist nicht notwendig, defensiven Code, Kompatibilitätslayer oder Sonstiges zu implementieren. Wir müssen also keine Rücksicht auf Legacy Code nehmen, sondern können den Code so einfach, schlank und elegant wie möglich halten.

## Aufbau

Rust-Port des TypeScript-Originals (Babylon.js-Client, Bun-Server, Electron-App): ein Cargo-Workspace (Edition 2024, Toolchain in `rust-toolchain.toml`), die Assets liegen in `assets/` im Wurzelverzeichnis.

- `crates/core` (`voidfleet-core`) — das Regelwerk ohne Ein- und Ausgabe: Karten, Brett, Admirale und Fähigkeiten, das Protokoll zwischen Client und Server und die Auflösung einer Runde. Siehe `crates/core/AGENTS.md`.
- `crates/server` (`voidfleet-server`) — der Spielserver (axum, WebSocket): Matches, Zugprüfung, Nachschub, der Computergegner und das Simulationswerkzeug `simulate`. Läuft eigenständig oder im Client eingebettet. Siehe `crates/server/AGENTS.md`.
- `crates/client` (`voidfleet-client`, Binary `voidfleet`) — der Desktop-Client in Bevy: Netz, Spielzustand, Controller, 3D-Szene, Rundenwiedergabe, Oberfläche, Ton, Texte (Deutsch/Englisch), Dev-Werkzeuge. Siehe `crates/client/AGENTS.md`.
- `tools/ships` — baut die 3D-Modelle der Schiffe und ihre Wracks per Python-Skript in Blender nach `assets/models/ships/` (`just ships [ids…]`, Vorschau-Renders mit `just ship-preview`; `BLENDER` ist der Pfad zu Blender, auf dem Linux-Server `~/.local/opt/blender-5.2.3-linux-x64/run.sh`). Aufbau, Material-Vertrag und Stilregeln in den Modulkommentaren von `build.py`, `vf.py`, `look.py`, `preview.py` und in `STYLE.md`.
- `tools/rasterize` — eigenständiges Werkzeug außerhalb des Workspaces: rastert Icons und Regel-Illustrationen einmalig zu PNG (`cargo run --release --manifest-path tools/rasterize/Cargo.toml`).

## Befehle

Die gängigen Befehle stehen auch als Rezepte in der `justfile` (`just` listet sie, z. B. `just play`, `just check`, `just scene hit`).

- `cargo run -p voidfleet-client` — startet das Spiel: ohne Argumente mit eingebettetem Server (eigener Thread, `127.0.0.1`, freier Port) in einem neuen Match gegen den Computer.
- `cargo run -p voidfleet-client -- --server ws://host:port [--match <id>] [--bot]` — verbindet mit einem eigenständigen Server; ohne `--match` entsteht ein neues Match (die Match-Id steht im Log und in der Statuszeile), mit `--match <id>` tritt der zweite Spieler bei (PvP), `--bot` legt das neue Match gleich gegen den Computer an.
- `cargo run -p voidfleet-server` — eigenständiger Server auf `ws://<HOST>:<PORT>/ws` (Umgebung `PORT`, Standard 3000, und `HOST`, Standard `0.0.0.0`).
- `cargo run -p voidfleet-server --release --bin simulate -- [partien je paarung]` — Bot gegen Bot über alle Paarungen der Admirale (Standard 2000 Partien je Paarung), misst Spieltempo und Balance.
- Das gebaute Binary sucht seine Assets in `$BEVY_ASSET_ROOT/assets`, sonst in `assets/` neben dem Binary bzw. in `Contents/Resources/assets` eines macOS-Bundles, sonst im Repository.

## Dev-Szenen und Umgebungsvariablen (Client)

Alle Einzelheiten in `crates/client/AGENTS.md` und den Modulkommentaren von `crates/client/src/dev/` und `crates/client/src/ui/preview.rs`.

- `VOIDFLEET_DEV_SCENE=board|hit|destroy|arrive|move|effects|commands` — Vorschau-Szenen ohne Server (Brett, Treffer, Zerstörung, Ankunft, Manöver, Effekte, Rundenwiedergabe der Befehle); `VOIDFLEET_DEV_HIT=<variante>` wählt bei `hit` eine Treffervariante, `VOIDFLEET_DEV_CARD`/`VOIDFLEET_DEV_SHOTS` steuern die `commands`-Szene.
- `VOIDFLEET_UI_PREVIEW=<runde>` — Oberfläche mit Beispiel-Hand ohne Server; `VOIDFLEET_UI_PREVIEW_SHOT=<pfad>` schreibt Screenshots eines kurzen Ablaufs, `VOIDFLEET_UI_PREVIEW_DIALOGS=<pfad>` je Dialog einen.
- Rauchtest: `VOIDFLEET_SMOKE_FRAMES=<n>` beendet die App nach `n` Bildern (mit `VOIDFLEET_SMOKE_SCREENSHOT=<pfad>.png` als Screenshot in Bild `n`, nach dem sie endet; `VOIDFLEET_SMOKE_SHOTS=<bild>,…` als Serie, `VOIDFLEET_SMOKE_TOGGLE=1` in der anderen Ansicht). `VOIDFLEET_SMOKE_PLAY=<runden>` spielt dazu eine Partie gegen den Computer über den Controller, bis zum Matchende oder nach `runden` Runden, mit einem Screenshot je Station (`<stamm>-<station>.png`), z. B. `VOIDFLEET_SMOKE_FRAMES=200000 VOIDFLEET_SMOKE_PLAY=30 VOIDFLEET_SMOKE_SCREENSHOT=$TMPDIR/play.png cargo run -p voidfleet-client`.
- Fenster und Ton brauchen eine echte Sitzung: Der Client startet nicht in der Bash-Sandbox (dann ohne Sandbox laufen lassen).

## Linting und Formatierung

`cargo fmt --all` formatiert (`rustfmt.toml`: Zeilenbreite 100; Kommentare von Hand bis 120 Zeichen umbrechen). Nach Änderungen müssen `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` und `cargo test --workspace` ohne Befund durchgehen. Die Lints stehen im Workspace (`Cargo.toml`): `unsafe_code` verboten, `clippy::all` als Warnung. Einzige Ausnahme ist das Crate-Attribut in `crates/client/src/lib.rs`, das `too_many_arguments` und `type_complexity` für Bevy-Systeme erlaubt; sonst keine `#[allow]`s.
