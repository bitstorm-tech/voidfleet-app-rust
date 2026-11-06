"""Plasma Frigate (Kreuzer, im Spiel der Plasmakreuzer) im Clean-Toon-Stil, detailreiche Fassung nach der Iron
Dreadnought.

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
"""

import palette
import vf
from mathutils import Vector
from parts import engine, engine_details, ring_at, vent
from vf import Ship, box, loft, octagon, plate, post, revolve, sec, smooth, sphere, superellipse, tip, tube, wedge

GX = 1.27  # Gondeln: Abstand zur Mittelachse
GZ = (0.43, -0.43)  # Gondeln: Höhen der Achsen


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
    return s.finish()
