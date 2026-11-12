"""Plasma Frigate (Kreuzer, im Spiel der Plasmakreuzer), detailreiche Fassung nach der Iron Dreadnought
(docs/stilregeln.md), mit Kleinteilen für den dreckigen Look.

Primärformen: gedrungener runder Rumpf, dessen Bug die Plasmakanone ist: dunkler Lagerkragen, Stahllauf mit drei breiten
violetten Leuchtringen, Mündung mit Plasmakern. Vier runde Gondeln an kurzen Trägern, je mit Nase in Kornblumenblau und
blauem Leuchtring dahinter. Auf dem Rücken ein erhöhtes Deck mit kleiner Brücke (Fensterband, Mast mit Licht) und
achtern der Reaktorwulst in Kornblumenblau mit violettem Band. Sechs Düsen (vier Gondeln, zwei am Heck), mit weniger
Segmenten als parts.engine.
Detailzonen mit Zweck, grob, weil das Schiff im Spiel als Paar zu 0,72 steht und die Konturen dabei gleich breit
bleiben; dazwischen ruhige Flächen (Rumpfflanken vor und hinter dem Tragring, Rückendeck, Reaktor, Gondeln):
- Bug: Panzerkappe um die Rumpfspitze, aus der die Kanone wächst
- Mittschiff: Tragring um den Rumpf, an dem die Träger der Gondeln hängen, und an jeder Gondel eine Schelle
- Rücken: Luke hinter der Brücke, Mast mit Radarplatte
- Heck: Gehäusesegment und Düsenkranz an allen sechs Düsen (`parts.engine_details`)
- Unterseite: Kinn mit der Kondensatorbank der Kanone und Lüfter nach vorn, aus dem der Kiel wächst; Tragring und
  Schellen laufen unten durch
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an der Brücke und an der Wand des
Rückendecks, Flak vor der Brücke, Geländer, Leiter, Querträger mit Nadelantennen am Mast, Schüssel und Gerätekasten auf
dem Brückendach, Leitungen vom Rückendeck in den Reaktor, Kisten, vier Leitungen am Lauf der Kanone unter den
Leuchtringen, Zieloptik auf dem Lagerkragen, Kühlrippen an allen sechs Düsen, Positionslichter an den oberen Gondeln.
"""

import math

import palette
import vf
from mathutils import Vector
from parts import engine, engine_details, flak, rail, ring_at, vent, windows
from vf import Ship, box, dish, loft, ngon, octagon, plate, post, revolve, sec, smooth, sphere, superellipse, sweep, tip, tube, wedge

GX = 1.27  # Gondeln: Abstand zur Mittelachse
GZ = (0.43, -0.43)  # Gondeln: Höhen der Achsen


def details(s, m):
    """Kleinteile (Modulkommentar)."""
    deck, roof = 0.68, 0.99  # Rückendeck und Dach der Brücke

    # ---------------------------------------------------------------- Fensterreihen: Brücke, Wand des Rückendecks
    windows(s, m, [(0.35, -0.75 + 0.12 * k, 0.84) for k in range(4)])
    windows(s, m, [(-0.15 + 0.15 * k, -0.25, 0.84) for k in range(3)], out="y")
    windows(s, m, [(0.46, -1.3 + 0.12 * k, 0.585) for k in range(5)])

    # ---------------------------------------------------------------- Flak vor der Brücke
    flak(s, m, (0.18, -1.4, deck), 15)

    # ---------------------------------------------------------------- Brücke: Geländer, Leiter, Antennen, Schüssel, Kasten
    rail(s, m, (0.335, -0.8, roof), (0.335, -0.265, roof), 3)
    rail(s, m, (-0.335, -0.265, roof), (0.335, -0.265, roof), 4, mirror=False)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.32), m["dark"], at=(0.27 + dx, -0.243, 0.835))
    for k in range(4):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.27, -0.243, 0.72 + 0.08 * k))
    s.add(box(0.44, 0.04, 0.04), m["hull"], at=(0, -0.5, 1.34))
    s.add(post(6, 0.012, 0.006, 1.32, 1.72), m["dark_round"], at=(0.2, -0.5, 0), mirror=True)
    s.add(post(8, 0.03, 0.025, roof - 0.02, roof + 0.12), m["dark_round"], at=(-0.23, -0.38, 0))
    s.add(dish(0.1, 0.04, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.23, -0.38, roof + 0.11), rot=(-40, 0, 150))
    s.add(box(0.12, 0.16, 0.1), m["armor"], at=(0.22, -0.4, roof + 0.045))

    # ---------------------------------------------------------------- Rückendeck: Leitungen in den Reaktor, Kisten
    for x in (-0.22, -0.29):
        s.add(sweep([(x, -0.3, deck + 0.02), (x, 0.8, deck + 0.02), (x, 1.02, deck + 0.12)], ngon(6, 0.025)), m["steel"])
    s.add(box(0.18, 0.22, 0.13), m["armor"], at=(0.2, 0.66, deck + 0.06))
    s.add(box(0.14, 0.16, 0.1), m["dark"], at=(0.24, 0.47, deck + 0.045))

    # ---------------------------------------------------------------- Kanone: Leitungen am Lauf, Zieloptik auf dem Kragen
    for a in (45, 135, 225, 315):
        dx, dz = 0.315 * math.cos(math.radians(a)), 0.315 * math.sin(math.radians(a))
        s.add(tube(6, 0.022, 0.022, -3.97, -2.84, dx, dz), m["dark_round"])
    s.add(box(0.12, 0.2, 0.08), m["armor"], at=(0, -2.72, 0.46))
    s.add(box(0.07, 0.02, 0.045), m["dark"], at=(0, -2.825, 0.465))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen (Gondeln und Heck)
    for r, y0, dx, heights in ((0.26, 1.65, GX, GZ), (0.3, 2.4, 0.42, (-0.1,))):
        for dz in heights:
            for t in (0.37, 0.44):
                y = y0 + 0.65 * t
                s.add(tube(10, r * 1.14, r * 1.14, y, y + 0.025, dx, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter an den oberen Gondeln
    s.add(sphere(0.03, 1), m["glow"], at=(GX + 0.34, 0.25, GZ[0]), mirror=True)


def build():
    s = Ship("plasma_frigate")
    m = vf.materials(s.name, "cruiser")
    m["plasma"] = vf.light(f"{s.name}_plasma", palette.PLASMA)
    N = 20
    # Gedrungener runder Rumpf; Panzerkappe um die Spitze und Tragring als zweite Lage, 0,04 über dem Rumpf
    rings = smooth([
        sec(-2.6, superellipse(0.96, 0.84, N, 3.2)),
        sec(-2.1, superellipse(1.42, 1.04, N, 3.2)),
        sec(-0.8, superellipse(1.74, 1.14, N, 3.2)),
        sec(1.0, superellipse(1.74, 1.14, N, 3.2)),
        sec(2.15, superellipse(1.4, 0.96, N, 3.2)),
        sec(2.5, superellipse(1.16, 0.8, N, 3.2)),
    ], 2)
    s.add(loft(rings), m["hull_round"])
    front = [Vector((p.x, -2.62, p.z)) for p in ring_at(rings, -2.599, 0.04)]
    s.add(loft([front, ring_at(rings, -2.4, 0.04), ring_at(rings, -2.25, 0.04)]), m["armor_round"])
    s.add(loft([ring_at(rings, y, 0.04) for y in (-0.25, 0.25, 0.75)]), m["armor_round"])
    # Erhöhtes Rückendeck als zweite Lage, darauf die Brücke und eine Luke
    s.add(loft([sec(-2.05, octagon(0.62, 0.26), dz=0.46), sec(-1.5, octagon(0.92, 0.32), dz=0.52), sec(0.85, octagon(0.92, 0.32), dz=0.52)]),
          m["armor"])
    s.add(plate(octagon(0.32, 0.28, 0.4), 0.05, "xy"), m["hull"], at=(0, 0.25, 0.685))
    # Unterseite: Kinn mit der Kondensatorbank der Kanone, Lüfter nach vorn; der dunkle Kiel wächst aus ihm
    s.add(wedge(0.6, 0.9, 0.28), m["hull"], at=(0, -1.7, -0.58), rot=(180, 0, 0))
    vent(s, m, m["hull"], (0, -2.15, -0.61), 0.42, 0.18, slats=1, rot=(90, 0, 0))
    s.add(plate([(-1.9, 0.05), (2.1, 0.05), (1.9, -0.26), (-1.5, -0.3)], 0.16, "yz"), m["dark"], at=(0, 0, -0.5))
    # Plasmakanone als Bug: Lagerkragen, Lauf mit drei Leuchtringen, Mündung mit Plasmakern
    s.add(tube(24, 0.47, 0.43, -2.35, -2.85), m["dark_round"])
    # Wenig Überstand und breite Lücken, damit die Ringe auch in der Kartenansicht (30° zur Laufachse) getrennt bleiben
    s.add(tube(20, 0.31, 0.29, -2.8, -4.0), m["steel"])
    for y in (-2.96, -3.32, -3.68):
        s.add(tube(20, 0.39, 0.39, y, y - 0.16), m["plasma"])
    s.add(revolve([(0.3, -3.96), (0.42, -4.06), (0.42, -4.16), (0.25, -4.16), (0.0, -4.14)], 20), m["dark_round"])
    s.add(tube(20, 0.24, 0.24, -4.15, -4.19), m["plasma"])
    # Vier Gondeln an kurzen Trägern, Schelle am Träger, Leuchtring hinter der Nase, Düse am Ende
    for gz in GZ:
        s.add(loft([sec(-1.2, superellipse(0.56, 0.56, 14)), sec(-0.95, superellipse(0.62, 0.62, 14)), sec(1.35, superellipse(0.62, 0.62, 14)),
                    sec(1.7, superellipse(0.54, 0.54, 14))]), m["hull_round"], at=(GX, 0, gz), mirror=True)
        s.add(loft([sec(-0.2, superellipse(0.68, 0.68, 14)), sec(0.7, superellipse(0.68, 0.68, 14))]), m["armor_round"], at=(GX, 0, gz), mirror=True)
        s.add(loft(smooth([tip(-1.68), sec(-1.38, superellipse(0.46, 0.46, 14)), sec(-1.12, superellipse(0.56, 0.56, 14))], 1)), m["accent_round"],
              at=(GX, 0, gz), mirror=True)
        s.add(tube(14, 0.35, 0.35, -1.08, -1.24), m["glow"], at=(GX, 0, gz), mirror=True)
        s.add(box(0.56, 0.8, 0.16), m["dark"], at=(GX - 0.38, 0.25, gz), mirror=True)
        engine(s, m, 0.26, 1.65, 2.3, dx=GX, dz=gz, n=14, mirror=True)
        engine_details(s, m, 0.26, 1.65, 2.3, dx=GX, dz=gz, n=14, mirror=True)
    # Reaktorwulst achtern auf dem Rücken, mit violettem Band
    s.add(revolve([(0.0, 0.75), (0.4, 0.95), (0.56, 1.35), (0.56, 1.95), (0.44, 2.35), (0.0, 2.5)], 20), m["accent_round"], at=(0, 0, 0.5))
    s.add(tube(20, 0.6, 0.6, 1.55, 1.75), m["plasma"], at=(0, 0, 0.5))
    # Brücke mit Fensterband, Mast mit Radarplatte und Licht
    s.add(wedge(0.7, 0.9, 0.34), m["hull"], at=(0, -0.7, 0.82))
    s.add(box(0.5, 0.12, 0.05), m["window"], at=(0, -1.0, 0.915), rot=(24.8, 0, 0))
    s.add(post(10, 0.1, 0.09, 0.95, 1.45), m["hull_round"], at=(0, -0.5, 0))
    s.add(box(0.36, 0.05, 0.16), m["armor"], at=(0, -0.59, 1.22), rot=(-15, 0, 0))
    s.add(sphere(0.11, 1), m["glow"], at=(0, -0.5, 1.5))
    # Zwei Heckdüsen, so weit nach achtern, dass ihr Gehäusesegment hinter dem Heck liegt
    engine(s, m, 0.3, 2.4, 3.05, dx=0.42, dz=-0.1, n=16, mirror=True)
    engine_details(s, m, 0.3, 2.4, 3.05, dx=0.42, dz=-0.1, n=16, mirror=True)
    details(s, m)
    return s.finish()
