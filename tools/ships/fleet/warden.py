"""Wächter (Boss von Sektor 2): unbewegliche automatische Verteidigungsstation, Schlachtschiff-Palette, Clean Toon.

Kein Schiff, also keine Düsen: ein runder Stationskern mit Kragen, um ihn ein waagrechter Ring aus vier Segmenten auf
dunklen Streben, mit Lichtband und stahlblauen Emitterblöcken. Vorn, durch die Lücke im Ring, das eine riesige
Hauptgeschütz in Nasenrichtung: Gehäuse in Stahlblau mit Fensterband, Kühlmantel, langer Lauf, Mündungsbremse mit
kaltem Kern. Oben eine Deckplatte mit Sensorkuppel mit umlaufendem Fensterband und Mast, achtern ein Energieblock, unten ein Ankerdorn.
"""

import math

import vf
from parts import beam
from vf import Ship, box, loft, octagon, plate, post, sec, smooth, sphere, superellipse, sweep, tube

ZC = 0.5  # Höhe der Kernmitte und des Rings
RING = (2.55, 3.0)  # Innen- und Außenradius des Rings
SEGMENTS = (45, 135, 225, 315)  # Mitten der Ringsegmente (Grad, 0 = Steuerbord); Lücken vorn, achtern und seitlich
SPAN = 40  # Bogen je Segment in Grad


def arc(r_in, r_out, a0, a1, n=8):
    """Kreisringsektor (x, y) von a0 bis a1 Grad."""
    angles = [math.radians(a0 + (a1 - a0) * k / n) for k in range(n + 1)]
    outer = [(r_out * math.cos(a), r_out * math.sin(a)) for a in angles]
    inner = [(r_in * math.cos(a), r_in * math.sin(a)) for a in reversed(angles)]
    return outer + inner


def build():
    s = Ship("warden")
    m = vf.materials(s.name, "battleship")

    # Runder Stationskern mit Kragen in der Mitte
    body = lambda w, h: superellipse(w, h, 24, 2.4)  # noqa: E731
    s.add(loft(smooth([
        sec(-2.3, body(1.6, 1.4), dz=ZC),
        sec(-1.6, body(3.0, 2.5), dz=ZC),
        sec(0.0, body(3.5, 2.9), dz=ZC),
        sec(1.6, body(3.0, 2.5), dz=ZC),
        sec(2.3, body(1.6, 1.4), dz=ZC),
    ], 2)), m["hull_round"])
    s.add(loft([sec(-0.22, body(3.8, 3.2), dz=ZC), sec(0.22, body(3.8, 3.2), dz=ZC)]), m["armor_round"])

    # Ring aus vier Segmenten auf Streben, mit Lichtband oben und Emitterblöcken außen
    for a in SEGMENTS:
        s.add(plate(arc(*RING, a - SPAN / 2, a + SPAN / 2), 0.42, "xy"), m["armor"], at=(0, 0, ZC))
        r = math.radians(a)
        mid = sum(RING) / 2
        band = [(mid * math.cos(math.radians(a + t)), mid * math.sin(math.radians(a + t)), ZC + 0.22) for t in range(-18, 19, 6)]
        s.add(sweep(band, vf.rect(0.08, 0.06)), m["glow"])
        s.add(beam((1.2 * math.cos(r), 1.2 * math.sin(r), ZC), (2.7 * math.cos(r), 2.7 * math.sin(r), ZC), 0.34, 0.3), m["dark"])
        s.add(box(0.5, 0.42, 0.6), m["accent"], at=(3.05 * math.cos(r), 3.05 * math.sin(r), ZC), rot=(0, 0, a + 90))

    # Hauptgeschütz durch die vordere Lücke: Gehäuse, Fensterband, Kühlmantel, Lauf, Mündungsbremse mit kaltem Kern
    s.add(loft([sec(-1.2, octagon(1.9, 1.7, 0.35), dz=ZC), sec(-3.2, octagon(1.9, 1.7, 0.35), dz=ZC),
                sec(-3.75, octagon(1.4, 1.3, 0.35), dz=ZC)]), m["accent"])
    s.add(box(0.04, 1.2, 0.09), m["window"], at=(0.955, -2.5, ZC + 0.4), mirror=True)
    s.add(tube(24, 0.72, 0.72, -4.75, -3.6, dz=ZC), m["dark_round"])
    s.add(tube(20, 0.5, 0.45, -6.2, -4.65, dz=ZC), m["steel"])
    s.add(loft([sec(-6.75, octagon(1.3, 1.3, 0.4), dz=ZC), sec(-6.1, octagon(1.3, 1.3, 0.4), dz=ZC)]), m["dark"])
    s.add(tube(16, 0.3, 0.3, -6.8, -6.7, dz=ZC), m["glow"])

    # Deckplatte als Bruch der Kernwölbung, darauf die Sensorkuppel mit Fensterband, Mast mit Licht
    s.add(plate(octagon(2.3, 2.7), 0.32, "xy"), m["armor"], at=(0, 0, ZC + 1.36))
    top = ZC + 1.72
    s.add(post(20, 0.95, 0.8, top - 0.2, top + 0.35), m["dark_round"])
    s.add(post(20, 0.82, 0.82, top + 0.12, top + 0.22), m["window"])
    s.add(post(20, 0.7, 0.35, top + 0.35, top + 0.6), m["hull_round"])
    s.add(post(10, 0.09, 0.06, top + 0.6, top + 1.5), m["hull_round"])
    s.add(sphere(0.09, 1), m["glow"], at=(0, 0, top + 1.55))

    # Energieblock achtern, Ankerdorn unten
    s.add(loft([sec(1.9, octagon(1.6, 1.2), dz=ZC), sec(3.3, octagon(1.6, 1.2), dz=ZC), sec(3.65, octagon(1.1, 0.8), dz=ZC)]), m["dark"])
    s.add(post(16, 0.75, 0.0, ZC - 1.2, ZC - 1.75), m["dark_round"])
    return s.finish()
