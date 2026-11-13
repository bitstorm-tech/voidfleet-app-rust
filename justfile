# Voidfleet — gängige Befehle. `just` listet alle Rezepte.

# Rezepte auflisten
default:
    @just --list --unsorted

# --- Spielen ---

# Spiel starten (weitere Argumente gehen an den Client)
play *args:
    git pull
    cargo run -p voidfleet-client --features dynamic_linking -- {{ args }}

# Spiel im Release-Build starten
play-release *args:
    cargo run -p voidfleet-client --release -- {{ args }}

play-no-pull *args:
    cargo run -p voidfleet-client --features dynamic_linking -- {{ args }}

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

# Vorschau-Szene: board, hit, destroy, arrive, move, effects, show (ein Modell, VOIDFLEET_DEV_SHIP=<dateiname>)
scene name="board":
    VOIDFLEET_DEV_SCENE={{ name }} cargo run -p voidfleet-client --features dynamic_linking

# Rauchtest: ein Run über den Controller, Screenshots je Station unter <dir>
smoke rounds="30" dir="${TMPDIR:-/tmp}/voidfleet-smoke":
    mkdir -p "{{ dir }}"
    VOIDFLEET_SMOKE_FRAMES=200000 VOIDFLEET_SMOKE_PLAY={{ rounds }} VOIDFLEET_SMOKE_SCREENSHOT="{{ dir }}/play.png" cargo run -p voidfleet-client --features dynamic_linking

# Wiki aus docs/ im Browser, lädt bei Änderungen neu (braucht mdBook: `cargo install mdbook`)
docs *args:
    mdbook serve --open {{ args }}

# Icons zu PNG rastern
rasterize:
    cargo run --release --manifest-path tools/rasterize/Cargo.toml

# Schiffsmodelle in Blender bauen (alle oder die angegebenen Ids) nach assets/models/ships/; BLENDER = Pfad zu Blender,
# CTT = Pfad zum Textur-Encoder (`cargo install ctt-cli --locked --version 0.6.1`)
ships *ids:
    "${BLENDER:-blender}" -b --factory-startup -P tools/ships/build.py -- {{ ids }}

# Vorschau-Renders der Schiffe nach tools/ships/renders/ (z. B. `just ship-preview vanguard --views board,hero`)
ship-preview *args:
    "${BLENDER:-blender}" -b --factory-startup -P tools/ships/preview.py -- {{ args }}
