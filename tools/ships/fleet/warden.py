"""Wächter (Boss von Sektor 2): unbewegliche automatische Verteidigungsstation, Schlachtschiff-Palette, Clean Toon,
detailreiche Fassung nach der Iron Dreadnought (STYLE.md).

Kein Schiff, also keine Düsen: ein runder Stationskern mit Kragen, um ihn ein waagrechter Schildring aus vier Segmenten
auf dunklen Streben, mit Lichtband und stahlblauen Emitterblöcken. Vorn, durch die Lücke im Ring, das eine riesige
Hauptgeschütz in Nasenrichtung: Gehäuse in Stahlblau, Kühlmantel, langer Lauf, Mündungsbremse mit kaltem Kern. Oben eine
Deckplatte mit dem Sensorkopf, achtern der Energieblock, unten eine Bodenplatte mit Ankerdorn.
Detailzonen mit Zweck (eine Station ohne Besatzung, also Technik statt Brücke), dazwischen ruhige Flächen (Kernkuppel
vor und hinter der Deckplatte, Flanken des Geschützgehäuses, Energieblock):
- Hauptgeschütz: Lafettenring am Kern, Ladebänke an den Gehäuseflanken über der Ringebene mit Deckel, Ladeanzeige und
  drei Zellen, Lüfter auf dem Gehäuse, Blendenbalg vor dem Kühlmantel, Laufschelle, zweiteilige Mündungsbremse
  (Stahlkern im Spalt)
- Schildring: Deckel oben und unten auf jedem Segment (das Lichtband liegt auf dem oberen), Emitterlinsen nach außen
- Flanken: Andockarme in den seitlichen Ringlücken mit Flansch, Stutzen und Luke; Kabelkanäle vom Energieverteiler
  achtern unter dem Kragen durch bis in den Lafettenring
- Sensorkopf: zweite Lage auf der Deckplatte, Drehkranz, Trommel mit umlaufendem Sensorband (Licht ohne Glühen),
  Kappe, Mast mit Licht
- Energie und Kühlung achtern: zwei Kühlblöcke mit Lüftern auf dem Energieblock, Energieverteiler an seinen Seiten
- Unterseite: Bodenplatte mit zweiter Lage, Ankerflansch und Ankerdorn, Plattenlage und Lafettenkiel unter dem
  Geschützgehäuse
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import math

import palette
import vf
from parts import beam, bellows, ring_at, vent
from vf import Ship, box, flank, grow, loft, octagon, plate, post, rect, sec, smooth, sphere, superellipse, sweep, tube

ZC = 0.5  # Höhe der Kernmitte und des Rings
RING = (2.55, 3.0)  # Innen- und Außenradius des Rings
SEGMENTS = (45, 135, 225, 315)  # Mitten der Ringsegmente (Grad, 0 = Steuerbord); Lücken vorn, achtern und seitlich
SPAN = 40  # Bogen je Segment in Grad
CABLE_Z = 1.03  # Höhe der Kabelkanäle an den Kernflanken


def arc(r_in, r_out, a0, a1, n=8):
    """Kreisringsektor (x, y) von a0 bis a1 Grad."""
    angles = [math.radians(a0 + (a1 - a0) * k / n) for k in range(n + 1)]
    outer = [(r_out * math.cos(a), r_out * math.sin(a)) for a in angles]
    inner = [(r_in * math.cos(a), r_in * math.sin(a)) for a in reversed(angles)]
    return outer + inner


def polar(r, a, z=0.0):
    """Punkt um die Hochachse: Radius r, Winkel a in Grad (0 = Steuerbord, 90 = achtern), Höhe z."""
    t = math.radians(a)
    return (r * math.cos(t), r * math.sin(t), z)


def flank_at(rings, y, z, dx=0.03):
    """Punkt auf der Steuerbordflanke des Kerns bei (y, z), um dx nach außen versetzt."""
    return flank([ring_at(rings, y)], z, dx)[0]


def build():
    s = Ship("warden")
    m = vf.materials(s.name, "battleship")
    # Das Sensorband läuft ganz um die Trommel: großes Licht ohne Glühen (STYLE.md, Große Lichter)
    m["sensor"] = vf.light(f"{s.name}_sensor", palette.WINDOW, glow=False)

    # ---------------------------------------------------------------- Kern und Kragen
    body = lambda w, h: superellipse(w, h, 24, 2.4)  # noqa: E731
    core = smooth([
        sec(-2.3, body(1.6, 1.4), dz=ZC),
        sec(-1.6, body(3.0, 2.5), dz=ZC),
        sec(0.0, body(3.5, 2.9), dz=ZC),
        sec(1.6, body(3.0, 2.5), dz=ZC),
        sec(2.3, body(1.6, 1.4), dz=ZC),
    ], 2)
    s.add(loft(core), m["hull_round"])
    s.add(loft([sec(-0.22, body(3.8, 3.2), dz=ZC), sec(0.22, body(3.8, 3.2), dz=ZC)]), m["armor_round"])

    # ---------------------------------------------------------------- Schildring: Segmente mit Deckeln, Lichtband,
    # Streben, Emitter mit Linse
    for a in SEGMENTS:
        s.add(plate(arc(*RING, a - SPAN / 2, a + SPAN / 2), 0.42, "xy"), m["armor"], at=(0, 0, ZC))
        lid = arc(RING[0] + 0.09, RING[1] - 0.09, a - SPAN / 2 + 1.9, a + SPAN / 2 - 1.9)
        for dz in (0.225, -0.225):
            s.add(plate(lid, 0.07, "xy"), m["hull"], at=(0, 0, ZC + dz))
        band = [polar(2.72, a + t, ZC + 0.28) for t in range(-18, 19, 6)]
        s.add(sweep(band, rect(0.08, 0.06)), m["glow"])
        s.add(beam(polar(1.2, a, ZC), polar(2.7, a, ZC), 0.34, 0.3), m["dark"])
        s.add(box(0.5, 0.42, 0.6), m["accent"], at=polar(3.05, a, ZC), rot=(0, 0, a + 90))
        s.add(tube(16, 0.15, 0.15, -0.02, 0.03), m["dark_round"], at=polar(3.26, a, ZC), rot=(0, 0, a - 90))

    # ---------------------------------------------------------------- Hauptgeschütz durch die vordere Lücke
    s.add(loft([sec(-1.95, octagon(2.15, 1.95, 0.35), dz=ZC), sec(-2.3, octagon(2.15, 1.95, 0.35), dz=ZC)]), m["dark"])
    s.add(loft([sec(-1.2, octagon(1.9, 1.7, 0.35), dz=ZC), sec(-3.2, octagon(1.9, 1.7, 0.35), dz=ZC),
                sec(-3.75, octagon(1.4, 1.3, 0.35), dz=ZC)]), m["accent"])
    s.add(plate([(-0.62, -2.45), (0.62, -2.45), (0.62, -3.15), (-0.62, -3.15)], 0.07, "xy"), m["armor"], at=(0, 0, ZC - 0.86))
    # Lafettenkiel: stützt das Gehäuse gegen den Kernboden
    s.add(plate([(-1.2, -0.62), (-2.35, -0.3), (-2.35, -0.5), (-1.45, -0.92)], 0.3, "yz"), m["dark"])
    vent(s, m, m["accent"], (0, -2.8, ZC + 0.82), 0.7, 0.62, slats=2)
    # Ladebänke an den Gehäuseflanken über der Ringebene: Block mit Deckel, Ladeanzeige und drei Zellen
    bank_y = -2.825
    s.add(box(0.34, 0.85, 0.44), m["armor"], at=(1.05, bank_y, 1.14), mirror=True)
    s.add(box(0.4, 0.91, 0.05), m["hull"], at=(1.06, bank_y, 1.385), mirror=True)
    s.add(box(0.04, 0.6, 0.07), m["glow"], at=(1.23, bank_y, 1.25), mirror=True)
    for dy in (-0.275, 0.0, 0.275):
        s.add(box(0.04, 0.2, 0.17), m["dark"], at=(1.23, bank_y + dy, 1.04), mirror=True)
    # Kühlmantel, Blendenbalg, Lauf mit Laufschelle, zweiteilige Mündungsbremse mit Stahlkern, kalter Kern
    s.add(tube(24, 0.72, 0.72, -4.75, -3.6, dz=ZC), m["dark_round"])
    s.add(bellows(20, 0.62, -4.72, -5.02), m["armor_round"], at=(0, 0, ZC))
    s.add(tube(20, 0.5, 0.44, -6.72, -4.65, dz=ZC), m["steel"])
    s.add(tube(20, 0.56, 0.56, -5.67, -5.55, dz=ZC), m["armor_round"])
    for y0, y1 in ((-6.32, -6.1), (-6.75, -6.5)):
        s.add(loft([sec(y1, octagon(1.3, 1.3, 0.4), dz=ZC), sec(y0, octagon(1.3, 1.3, 0.4), dz=ZC)]), m["dark"])
    s.add(tube(16, 0.3, 0.3, -6.8, -6.7, dz=ZC), m["glow"])

    # ---------------------------------------------------------------- Andockarme in den seitlichen Ringlücken
    s.add(beam((1.6, 0, ZC), (2.45, 0, ZC), 0.3, 0.3), m["dark"], mirror=True)
    s.add(tube(20, 0.4, 0.4, -0.08, 0.08), m["armor_round"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)
    s.add(tube(20, 0.28, 0.28, -0.12, 0.22), m["dark_round"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)
    s.add(tube(16, 0.17, 0.17, 0.2, 0.24), m["armor_round"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)

    # ---------------------------------------------------------------- Kabelkanäle vom Verteiler bis in den Lafettenring
    path = [flank_at(core, y, CABLE_Z) for y in (-2.0, -1.6, -1.2, -0.6, 0.0, 0.6, 1.2, 1.6, 1.9)]
    s.add(sweep(path, rect(0.1, 0.1)), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Sensorkopf: Deckplatte mit zweiter Lage,
    # Drehkranz, Trommel mit Sensorband, Kappe, Mast mit Licht
    deck = octagon(2.3, 2.7)
    s.add(plate(deck, 0.32, "xy"), m["armor"], at=(0, 0, ZC + 1.36))
    s.add(plate(grow(deck, -0.14), 0.07, "xy"), m["hull"], at=(0, 0, ZC + 1.525))
    top = ZC + 1.72
    s.add(post(24, 1.08, 1.08, top - 0.2, top - 0.1), m["armor_round"])
    s.add(post(24, 0.95, 0.8, top - 0.2, top + 0.35), m["dark_round"])
    s.add(post(24, 0.9, 0.88, top + 0.12, top + 0.2), m["sensor"])
    s.add(post(20, 0.7, 0.35, top + 0.35, top + 0.6), m["hull_round"])
    s.add(post(10, 0.09, 0.06, top + 0.6, top + 1.5), m["hull_round"])
    s.add(sphere(0.09, 1), m["glow"], at=(0, 0, top + 1.55))

    # ---------------------------------------------------------------- Energieblock achtern mit Kühlblöcken und
    # Energieverteilern
    s.add(loft([sec(1.9, octagon(1.6, 1.2), dz=ZC), sec(3.3, octagon(1.6, 1.2), dz=ZC), sec(3.65, octagon(1.1, 0.8), dz=ZC)]), m["dark"])
    s.add(box(0.54, 1.0, 0.5), m["armor"], at=(0.35, 2.7, 1.25), mirror=True)
    vent(s, m, m["armor"], (0.35, 2.7, 1.5), 0.4, 0.8, slats=2, mirror=True)
    s.add(box(0.36, 0.4, 0.34), m["armor"], at=(1.08, 1.98, CABLE_Z + 0.03), mirror=True)

    # ---------------------------------------------------------------- Unterseite: Bodenplatte, Ankerflansch, Ankerdorn
    s.add(plate(octagon(2.0, 2.4), 0.3, "xy"), m["armor"], at=(0, 0, -0.97))
    s.add(plate(grow(octagon(2.0, 2.4), -0.13), 0.07, "xy"), m["hull"], at=(0, 0, -1.135))
    s.add(post(24, 0.62, 0.62, -1.26, -1.15), m["armor_round"])
    s.add(post(16, 0.5, 0.0, -1.24, -1.39), m["dark_round"])
    return s.finish()
