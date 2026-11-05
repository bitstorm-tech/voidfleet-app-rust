"""Pyre (Flaggschiff von Admiralin Reyes, „die Glut“), Clean Toon.

Schweres, breites Schlachtschiff, das heiß wirkt. Ein Keilrumpf mit hohem Rückendeck und goldener Zierlinie, davor der
Plasmawerfer als Bug: ein Gehäuse in Glutorange mit Fensterband, darauf ein dunkler Kühlmantel und ein Emitterkegel,
vor dem drei dunkle Klauen eine leuchtende Glutkugel halten. An beiden Flanken je ein langer, dunkler
Kühlkörper mit vier schrägen Glutschlitzen, auf dem Rückendeck zwei Reihen großer Kühlblöcke mit Glutkante.
Dahinter die Wappenplatte mit Reyes' Flamme, achtern die Brücke aus drei Keilen mit Fensterband und Mast, Panzerschürzen
in Glutorange, Kiel und drei schwere Düsen.
"""

import vf
from parts import badge, engine, sym, trim_line
from vf import Ship, box, keel, loft, octagon, plate, post, sec, sphere, tube, wedge

# Wappen: Platte mit Spitze zum Bug, darauf eine Flamme, die zum Bug lodert
PLATE = sym([(0.0, -0.95), (0.62, -0.45), (0.62, 0.55), (0.0, 0.8)])
FLAME = sym([(0.0, -0.74), (0.13, -0.3), (0.3, -0.48), (0.38, -0.04), (0.34, 0.28), (0.18, 0.48), (0.0, 0.54)])


def build():
    s = Ship("pyre")
    m = vf.materials(s.name, "battleship")

    # Breiter Keilrumpf, darauf das hohe Rückendeck; Zierlinie an der Rumpfkante
    hull = [(-3.4, 2.0, 1.1), (-2.4, 3.4, 1.5), (2.2, 3.6, 1.6), (3.9, 2.8, 1.3), (4.5, 2.0, 1.0)]
    s.add(loft([sec(y, keel(w, h)) for y, w, h in hull]), m["hull"])
    trim_line(s, m, [(w / 2 + 0.01, y, h / 2 + 0.005) for y, w, h in hull])
    deck = 1.15
    s.add(loft([sec(-3.0, octagon(1.4, 0.5), dz=deck - 0.35), sec(-2.3, octagon(2.2, 0.7), dz=deck - 0.25),
                sec(3.6, octagon(2.3, 0.7), dz=deck - 0.25), sec(4.2, octagon(1.6, 0.5), dz=deck - 0.3)]), m["hull"])
    top = deck + 0.1

    # Plasmawerfer als Bug: Gehäuse in Glutorange mit Fensterband, Kühlmantel, Emitterkegel und drei dunkle Klauen, die
    # eine Glutkugel halten (von der Seite zwei Klauen mit der Kugel dazwischen, von oben die Kugel um die obere Klaue)
    pz = 0.25
    s.add(loft([sec(-2.6, octagon(2.0, 1.5, 0.35), dz=pz), sec(-3.4, octagon(2.0, 1.5, 0.35), dz=pz),
                sec(-3.9, octagon(1.6, 1.25, 0.35), dz=pz)]), m["accent"])
    s.add(box(0.04, 0.7, 0.09), m["window"], at=(1.005, -3.0, pz + 0.3), mirror=True)
    s.add(tube(24, 0.7, 0.62, -4.4, -3.75, dz=pz), m["dark_round"])
    s.add(tube(20, 0.32, 0.14, -4.4, -4.95, dz=pz), m["steel"])
    claw = [(-4.2, 0.3), (-4.2, 0.82), (-5.2, 0.8), (-5.85, 0.46), (-5.7, 0.3)]
    for angle in (0, 120, 240):
        s.add(plate(claw, 0.3, "yz"), m["dark"], at=(0, 0, pz), rot=(0, angle, 0))
    s.add(sphere(0.4, 2), m["glow"], at=(0, -5.3, pz))

    # Kühlkörper an den Flanken: langer dunkler Block mit vier schrägen Glutschlitzen (Lichter ohne Glühen)
    s.add(plate([(1.55, -1.9), (2.05, -1.5), (2.05, 2.3), (1.55, 2.7)], 0.6, "xy"), m["dark"], at=(0, 0, 0.15), mirror=True)
    for y in (-1.1, -0.15, 0.8, 1.75):
        s.add(box(0.06, 0.18, 0.5), m["signal"], at=(2.06, y, 0.15), rot=(25, 0, 0), mirror=True)

    # Zwei Reihen Kühlblöcke auf dem Rückendeck mit Glutkante
    for y in (-1.4, -0.35, 0.7):
        s.add(wedge(0.42, 0.75, 0.55, front=0.5), m["dark"], at=(1.0, y, top + 0.2), mirror=True)
        s.add(box(0.3, 0.5, 0.06), m["signal"], at=(1.0, y + 0.1, top + 0.49), mirror=True)

    # Wappenplatte zwischen den Kühlblöcken
    badge(s, m, (0, -0.35, top - 0.05), PLATE, FLAME, dy=-0.04)

    # Panzerschürzen in Glutorange unter den Kühlkörpern
    s.add(plate([(0.7, -2.3), (1.95, -2.0), (2.0, 3.0), (0.7, 3.6)], 0.2, "xy"), m["accent"], at=(0, 0, -0.35), mirror=True)

    # Brücke achtern: drei Keile, Fensterband, Mast mit Leuchtfeuer
    z, by = top, 2.75
    for w, l, h in ((1.8, 1.5, 0.55), (1.35, 1.1, 0.45), (0.95, 0.8, 0.4)):
        s.add(wedge(w, l, h), m["hull"], at=(0, by, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, by - 0.4 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.49, by + 0.05, z - 0.17), mirror=True)
    s.add(post(10, 0.09, 0.06, z, z + 0.85), m["hull_round"], at=(0, by + 0.15, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, by + 0.15, z + 0.89))

    # Kiel und drei schwere Düsen
    s.add(plate([(-2.6, -0.4), (3.6, -0.5), (3.3, -1.0), (-1.6, -0.9)], 0.24, "yz"), m["dark"])
    s.add(box(2.6, 0.6, 1.2), m["dark"], at=(0, 4.35, 0.2))
    engine(s, m, 0.52, 4.4, 5.35, dz=0.25)
    engine(s, m, 0.4, 4.4, 5.15, dx=0.95, dz=0.15, mirror=True)
    return s.finish()
