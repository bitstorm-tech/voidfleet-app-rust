"""Feinstruktur aller Rümpfe: eine kleine, kachelbare Textur `detail.ktx2`, die das Spiel in der Nahaufnahme über jede
toon-Fläche mit Verwitterung legt (`toon.wesl`), aus den drei Achsen des Schiffs projiziert und damit unabhängig von der
Abwicklung und der Texeldichte der Maske. Sie liefert, was feiner ist als die Maske: Kratzer, Poren und flache Beulen
als Relief, Flecken im Ton und das Korn, das die Ränder von Schmutz und Abrieb aufbricht. Auf dem Brett mittelt die
Mip-Kette sie zur neutralen Fläche.

Kanäle (8 Bit, linear gelesen, 0,5 ist neutral):
  R, G  Neigung des Reliefs entlang u und v (Steigung der Höhe, auf ±1 gebracht)
  B     Ton: Kratzer heller (blankes Metall), Poren und Flecken dunkler
  A     Korn: feines Rauschen ohne Richtung

Eine Kachel ist SIZE Texel groß und im Spiel eine Blender-Einheit lang (`DETAIL` in toon.wesl legt sie dazu gröber ein
zweites Mal darüber). Geschrieben wird unkomprimiert mit Mip-Kette (1,4 MB): Blockkompression machte aus dem Rauschen
Klötzchen. Feste Saat, dieselben Bytes bei jedem Lauf.
"""

import os
import subprocess

import numpy as np

import weather

SIZE = 512
SEED = 11
CACHE = os.path.join(weather.CACHE, "detail.png")


def _noise(cells, rng):
    """Zufallsgitter mit `cells` Zellen je Achse, glatt interpoliert und kachelbar; 0 bis 1."""
    grid = rng.random((cells, cells))
    t = np.linspace(0, cells, SIZE, endpoint=False)
    i0 = np.floor(t).astype(int) % cells
    i1 = (i0 + 1) % cells
    f = t - np.floor(t)
    f = f * f * (3 - 2 * f)
    fv, fu = f[:, None], f[None, :]
    return (grid[np.ix_(i0, i0)] * (1 - fv) * (1 - fu) + grid[np.ix_(i1, i0)] * fv * (1 - fu)
            + grid[np.ix_(i0, i1)] * (1 - fv) * fu + grid[np.ix_(i1, i1)] * fv * fu)


def _fbm(cells, octaves, rng):
    total, amp, norm = np.zeros((SIZE, SIZE)), 1.0, 0.0
    for o in range(octaves):
        total += amp * _noise(cells << o, rng)
        norm += amp
        amp *= 0.5
    return total / norm


def _scratches(count, rng):
    """Kratzer als Rillen (0 bis 1): gerade Striche in beliebiger Richtung, 12 bis 110 Texel lang, an den Enden
    auslaufend, über den Kachelrand hinweg fortgesetzt."""
    y, x = np.mgrid[0:SIZE, 0:SIZE].astype(np.float64)
    out = np.zeros((SIZE, SIZE))
    for _ in range(count):
        cx, cy, angle = rng.random() * SIZE, rng.random() * SIZE, rng.random() * np.pi
        half, width, depth = 6 + 49 * rng.random() ** 2, 0.55 + 0.5 * rng.random(), 0.35 + 0.65 * rng.random()
        dx = (x - cx + SIZE / 2) % SIZE - SIZE / 2
        dy = (y - cy + SIZE / 2) % SIZE - SIZE / 2
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        out = np.maximum(out, depth * np.exp(-(across / width) ** 2) * np.clip(1 - (along / half) ** 2, 0, 1))
    return out


def _pits(count, rng):
    """Poren und Einschläge (0 bis 1): runde Dellen von 1 bis 3 Texeln Radius."""
    y, x = np.mgrid[0:SIZE, 0:SIZE].astype(np.float64)
    out = np.zeros((SIZE, SIZE))
    for _ in range(count):
        cx, cy, radius = rng.random() * SIZE, rng.random() * SIZE, 1.0 + 2.0 * rng.random() ** 3
        dx = (x - cx + SIZE / 2) % SIZE - SIZE / 2
        dy = (y - cy + SIZE / 2) % SIZE - SIZE / 2
        out = np.maximum(out, (0.5 + 0.5 * rng.random()) * np.exp(-(dx * dx + dy * dy) / (radius * radius)))
    return out


def pixels():
    """Die Textur als RGBA-Bytes (Zeile 0 unten wie in Blender)."""
    rng = np.random.default_rng(SEED)
    scratches, pits = _scratches(170, rng), _pits(420, rng)
    # Höhe: flache Beulen wie gewalztes Blech, darin Poren und Kratzer als Rillen
    height = 5.0 * _fbm(7, 3, rng) + 0.6 * _fbm(64, 2, rng) - 1.4 * scratches - 1.6 * pits
    slope = np.stack([np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1), np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)])
    slope = np.clip(slope / np.percentile(np.abs(slope), 99.7), -1, 1)
    stains = np.clip((_fbm(5, 4, rng) - 0.5) * 2.4, -1, 1)
    tone = np.clip(0.3 * stains + 0.8 * scratches - 0.9 * pits, -1, 1)
    grain = _fbm(90, 2, rng)
    grain = np.clip((grain - grain.mean()) / (4.0 * grain.std()), -0.5, 0.5) * 2
    out = np.stack([slope[0], slope[1], tone, grain], axis=2)
    return np.rint((0.5 + 0.5 * out) * 255).astype(np.uint8)


def write(target):
    """Schreibt die Textur als KTX2 `target` (über das PNG im Cache der Masken); liegt beides schon da, bleibt es
    stehen, damit gleichzeitige Builds einander die Datei nicht unter den Händen neu schreiben."""
    if os.path.exists(CACHE) and os.path.exists(target):
        return
    os.makedirs(weather.CACHE, exist_ok=True)
    weather.write_png(CACHE, pixels())
    subprocess.run([os.environ.get("CTT", "ctt"), CACHE, "-o", target, "-f", "rgba8unorm", "--input-color-space", "linear", "--mipmap",
                    "--zstd"], check=True)
