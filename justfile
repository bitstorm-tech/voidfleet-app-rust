# Voidfleet — gängige Befehle. `just` listet alle Rezepte.

# Rezepte auflisten
default:
    @just --list --unsorted

# --- Spielen ---

# Spiel mit eingebettetem Server gegen den Computer starten (weitere Argumente gehen an den Client)
play *args:
    cargo run -p voidfleet-client -- {{ args }}

# Spiel im Release-Build starten
play-release *args:
    cargo run -p voidfleet-client --release -- {{ args }}

# Eigenständigen Server starten (ws://<host>:<port>/ws)
server port="3000" host="0.0.0.0":
    PORT={{ port }} HOST={{ host }} cargo run -p voidfleet-server

# Mit einem eigenständigen Server verbinden und ein neues Match anlegen
host server="ws://127.0.0.1:3000":
    cargo run -p voidfleet-client -- --server {{ server }}

# Einem Match auf einem eigenständigen Server beitreten (PvP)
join match server="ws://127.0.0.1:3000":
    cargo run -p voidfleet-client -- --server {{ server }} --match {{ match }}

# Auf einem eigenständigen Server ein Match gegen den Computer anlegen
bot server="ws://127.0.0.1:3000":
    cargo run -p voidfleet-client -- --server {{ server }} --bot

# --- Prüfen ---

# Alles prüfen, was vor einem Commit durchgehen muss: Format, Clippy, Tests
check: fmt-check clippy test

# Code formatieren
fmt:
    cargo fmt --all

# Formatierung prüfen
fmt-check:
    cargo fmt --all --check

# Clippy über den ganzen Workspace, Warnungen als Fehler
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Tests des ganzen Workspaces (weitere Argumente gehen an `cargo test`)
test *args:
    cargo test --workspace {{ args }}

# --- Werkzeuge ---

# Bot gegen Bot über alle Paarungen der Admirale (Partien je Paarung)
simulate games="2000":
    cargo run -p voidfleet-server --release --bin simulate -- {{ games }}

# Dev-Szene ohne Server: board, hit, destroy, arrive, move, effects, commands
scene name="board":
    VOIDFLEET_DEV_SCENE={{ name }} cargo run -p voidfleet-client

# Oberfläche mit Beispiel-Hand ohne Server in der angegebenen Runde
ui-preview round="1":
    VOIDFLEET_UI_PREVIEW={{ round }} cargo run -p voidfleet-client

# Rauchtest: Partie gegen den Computer über den Controller, Screenshots je Station unter <dir>
smoke rounds="30" dir="${TMPDIR:-/tmp}/voidfleet-smoke":
    mkdir -p "{{ dir }}"
    VOIDFLEET_SMOKE_FRAMES=200000 VOIDFLEET_SMOKE_PLAY={{ rounds }} VOIDFLEET_SMOKE_SCREENSHOT="{{ dir }}/play.png" cargo run -p voidfleet-client

# Mauszeiger, Icons und Regel-Illustrationen zu PNG rastern
rasterize:
    cargo run --release --manifest-path tools/rasterize/Cargo.toml

# Schiffsmodelle in Blender bauen (alle oder die angegebenen Ids) nach assets/models/ships/; BLENDER = Pfad zu Blender
ships *ids:
    "${BLENDER:-blender}" -b --factory-startup -P tools/ships/build.py -- {{ ids }}

# Vorschau-Renders der Schiffe nach tools/ships/renders/ (z. B. `just ship-preview vanguard --views board,hero`)
ship-preview *args:
    "${BLENDER:-blender}" -b --factory-startup -P tools/ships/preview.py -- {{ args }}
