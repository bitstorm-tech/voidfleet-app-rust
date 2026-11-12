# Voidfleet

Das ist Voidfleet, ein Einzelspieler-Deckbuilder im Stil von Slay the Spire, der im Science-Fiction-Weltraum-Universum spielt. Der Spieler führt genau ein Schiff, sein Flaggschiff, gegen Verbände, Schlachtschiffe und Bosse, die das Spiel nach festen Mustern steuert. Die Regeln stehen in `docs/`, je Thema eine Seite: `vision.md`, `gefecht.md`, `admirale.md`, `karten.md`, `gegner.md`, `run.md`, `inszenierung.md`, `beispielrunde.md`, `offene-punkte.md`; dazu `grafikstil.md` mit dem Look des ganzen Spiels (Farbpalette, Licht, Effekte, Oberfläche, Bilder; wer den Stil ändert oder etwas Sichtbares neu baut, beginnt dort) und `stilregeln.md` mit den Bauregeln der Schiffsmodelle. Das ist die Quelle für alle Regeln und Zahlen; jede Regel steht auf genau einer Seite, die anderen verweisen dorthin. `just docs` zeigt die Seiten als Wiki im Browser (mdBook, `book.toml`); eine neue Seite muss in `docs/SUMMARY.md` stehen, sonst fehlt sie dort.

Das Gefecht: Die Gegnerseite hat zwei Reihen (Front und Back) mit je drei Spalten, eine Einheit in der Back-Reihe ist gedeckt, wenn vorn in ihrer Spalte eine steht; der Spieler steht mit seinem Flaggschiff in einer von drei Spalten. Eine Runde ist Spielerzug, Gegnerzug, Auflösung. Im Spielerzug wirken beliebig viele Befehlskarten sofort, solange die 3 Befehlspunkte reichen (die Hand wird jede Runde neu gezogen, fünf Karten); das Flaggschiff löst je Runde eine Station aus (mit Alarmstufe Rot zwei verschiedene), die wie eine Karte sofort wirkt und wie eine Karte ihr Ziel beim Auslösen bekommt — Waffen (feuert auf ein gewähltes Schiff), Schilde, Schadenskontrolle, Manöver (Spalte wechseln) oder die Admiralsstation. Die Gegner sagen vorher an, was sie tun werden (Zielfeuer, Spaltenfeuer, Rammkurs, Schilde, Ausweichen, Aufladen, Zielerfassung, Verstärkung rufen); wer im Spielerzug fällt, handelt nie. Jeder Schuss trifft mit festen Zahlen; abgezogen werden in dieser Reihenfolge Ausweichen, Deckung, Schilde, Panzerung. Zustände sind Brand, Notfall und Zerstört; ab Runde 8 tickt die Gefechtsuhr. Der Run: Vier Admirale (Osei die Mauer, Vance der Hammer, Kerr der Uhrmacher, Reyes die Glut) mit Flaggschiff, Passiv, Admiralsstation und Startdeck; drei Sektoren mit Sternenkarte (Gefecht, Elite, Ereignis, Werft, Schwarzmarkt, Boss), Beute aus Befehlskarten, Modulen und Schrott; die Hülle bleibt zwischen den Gefechten, wie sie ist. Fällt das Flaggschiff, endet der Run. Koop und ein Duell folgen später auf denselben Gefechtsregeln.

## Status

Wir sind momentan noch in der Entwicklungsphase. Das Spiel ist also nicht live. Aus diesem Grund können wir den Code jederzeit komplett ändern und es ist nicht notwendig, defensiven Code, Kompatibilitätslayer oder Sonstiges zu implementieren. Wir müssen also keine Rücksicht auf Legacy Code nehmen, sondern können den Code so einfach, schlank und elegant wie möglich halten.

Der Umbau vom früheren asynchronen PvP (Rust-Port des TypeScript-Originals) zum PvE-Run ist umgesetzt; offene Punkte stehen in `TODO.md`.

## Aufbau

Ein Cargo-Workspace (Edition 2024, Toolchain in `rust-toolchain.toml`), die Assets liegen in `assets/` im Wurzelverzeichnis.

- `crates/core` (`voidfleet-core`) — das Regelwerk ohne Ein- und Ausgabe: Karten, Admirale, Gegner und ihre Muster, Module, das Gefecht (`Battle`) mit seinem Ereignisstrom und der Run (`Run`: Sternenkarte, Verbände, Beute, Werft, Schwarzmarkt, Ereignisse). Siehe `crates/core/AGENTS.md`.
- `crates/client` (`voidfleet-client`, Binary `voidfleet`) — das Spiel in Bevy: Hauptmenü, Run, Gefecht, 3D-Szene, Wiedergabe, Oberfläche, Ton, Texte (Deutsch/Englisch), Speicherstand, Dev-Werkzeuge. Läuft offline, ohne Server. Siehe `crates/client/AGENTS.md`.
- `tools/ships` — baut die 3D-Modelle der Schiffe und ihre Wracks per Python-Skript in Blender nach `assets/models/ships/` (`just ships [ids…]`, Vorschau-Renders mit `just ship-preview`; `BLENDER` ist der Pfad zu Blender, auf dem Mac `/Applications/Blender.app/Contents/MacOS/Blender`, auf dem Linux-Server `~/.local/opt/blender-5.2.3-linux-x64/run.sh`). Blender nur headless (`-b`) verwenden. Aufbau, Material-Vertrag, Verwitterung und Stilregeln in den Modulkommentaren von `build.py`, `vf.py`, `look.py`, `weather.py`, `preview.py` und in `docs/stilregeln.md`.
- `tools/rasterize` — eigenständiges Werkzeug außerhalb des Workspaces: rastert die Icons einmalig zu PNG (`cargo run --release --manifest-path tools/rasterize/Cargo.toml`).
- Neue Bilder (Porträts, Kartenbilder) entstehen über die fal.ai-API (Schlüssel `FAL_AI_API_KEY` in der Shell-Umgebung), mit einem vorhandenen Bild oder Modell-Render als Stilvorlage, in 736 × 1024 als JPEG.

## Befehle

Die gängigen Befehle stehen als Rezepte in der `justfile` (`just` listet sie).

- `cargo run -p voidfleet-client` (`just play`) — startet das Spiel: Hauptmenü mit den gespeicherten Runs zum Blättern, „Neuer Run“, „Weiter“, „Spielregeln“, „Einstellungen“ und „Beenden“.
- `just check` — alles, was vor einem Commit durchgehen muss: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Abhängigkeiten und Features mit `cargo add` hinzufügen, nicht von Hand in `Cargo.toml`. Die API einer Abhängigkeit (vor allem Bevy) im Quellcode der verwendeten Version unter `~/.cargo/registry/src/` nachsehen, nicht aus dem Gedächtnis.
- Das gebaute Binary sucht seine Assets in `$BEVY_ASSET_ROOT/assets`, sonst in `assets/` neben dem Binary bzw. in `Contents/Resources/assets` eines macOS-Bundles, sonst im Repository.

## Dev-Szenen und Umgebungsvariablen (Client)

Alle Einzelheiten in `crates/client/AGENTS.md` und den Modulkommentaren von `crates/client/src/dev/`.

- `VOIDFLEET_DEV_SCENE=board|hit|destroy|arrive|move|effects` (`just scene <name>`) — Vorschau-Szenen (erstes Gefecht eines Runs, Treffer, Zerstörung, Ankunft, Manöver, Effekte); `VOIDFLEET_DEV_HIT=<variante>` wählt bei `hit` eine Treffervariante.
- Rauchtest: `VOIDFLEET_SMOKE_FRAMES=<n>` beendet die App nach `n` Bildern (mit `VOIDFLEET_SMOKE_SCREENSHOT=<pfad>.png` als Screenshot in Bild `n`, nach dem sie endet; `VOIDFLEET_SMOKE_SHOTS=<bild>,…` als Serie, `VOIDFLEET_SMOKE_TOGGLE=1` in der anderen Ansicht). `VOIDFLEET_SMOKE_PLAY=<runden>` (`just smoke`) spielt dazu einen Run über den Controller (erster wählbarer Knoten, erste Beute, im Gefecht Waffen auf den ersten Gegner), bis zum Run-Ende oder nach `runden` Runden, mit einem Screenshot je Station (`<stamm>-<nr>-<station>.png`).
- Fenster und Ton brauchen eine echte Sitzung: Der Client startet nicht in der Bash-Sandbox (dann ohne Sandbox laufen lassen); bei gesperrtem Bildschirm sind die Screenshots schwarz.

## Linting und Formatierung

`cargo fmt --all` formatiert (`rustfmt.toml`: Zeilenbreite 100; Kommentare von Hand bis 120 Zeichen umbrechen). Nach Änderungen müssen `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` und `cargo test --workspace` ohne Befund durchgehen. Die Lints stehen im Workspace (`Cargo.toml`): `unsafe_code` verboten, `clippy::all` als Warnung. Einzige Ausnahme ist das Crate-Attribut in `crates/client/src/lib.rs`, das `too_many_arguments` und `type_complexity` für Bevy-Systeme erlaubt; sonst keine `#[allow]`s.
