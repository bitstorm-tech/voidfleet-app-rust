"""Horizon Cruiser (Kreuzer, im Spiel der Wachkreuzer) im Clean-Toon-Stil, detailreiche Fassung nach der Iron
Dreadnought.

Primärformen: langer, schlanker Facettenrumpf, der zum Bug flach wie ein Spatel ausläuft; darauf ein großer
Zwillingsturm in Kornblumenblau und ein Rückendeck mit dem Brückenturm achtern (drei Blöcke, Fensterband, Mast mit
Licht). Zwei runde Gondeln an gepfeilten Pylonen mit blauem Lichtband und Akzent-Einlauf, zwei schräge Heckflossen,
dunkler Kiel, vier Düsen.
Detailzonen mit Zweck, gröber und sparsamer als an der Dreadnought, weil das Schiff im Spiel als Paar zu 0,72 steht und
die Konturen dabei gleich breit bleiben; dazwischen ruhige Flächen (Bugdeck vor dem Turm, Rückendeck, Gondeln, Flossen):
- Bug: Bugpanzer-Kappe um die Spatelspitze
- Turm: größer als in der groben Fassung, mit `parts.turret_details`
- Rückendeck: Deckplatte vor der Brücke, Maschinenlüfter hinter ihr
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte
- Heck: Gehäusesegment und Düsenkranz an allen vier Düsen (`parts.engine_details`)
- Unterseite: Kinn unter dem Bug mit zweiflügeligem Bootstor nach vorn, aus dem der Kiel wächst; drei Spanten je Seite
  auf der unteren Fase vor den Pylonen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import vf
from parts import engine, engine_details, turret, turret_details, vent
from vf import Ship, box, loft, ngon, octagon, plate, post, sec, smooth, sphere, wedge

NX = 1.18  # Gondeln: Abstand zur Mittelachse
NZ = 0.02  # Gondeln: Höhe der Achse
# Rumpf: Querschnitte (y, Breite, Höhe, Versatz in z) des Profils `hull`, linear verbunden
HULL = ((-3.85, 0.5, 0.1, 0.0), (-3.0, 1.0, 0.32, 0.06), (-2.45, 1.2, 0.5, 0.05), (-1.5, 1.45, 0.72, 0.0), (-0.4, 1.56, 0.8, 0.0),
        (1.7, 1.56, 0.8, 0.0), (2.9, 1.3, 0.7, 0.0), (3.4, 1.0, 0.56, 0.0))
TURRET = {"at": (0, -2.05, 0.3), "r": 0.42, "barrel": 1.12}


def hull(w, h):
    """Rumpfprofil: flaches Deck, obere Fase, senkrechte Flanke, steile untere Fase (fängt in der Karte Licht),
    schmaler Boden."""
    return [(w / 2, -0.05 * h), (w / 2, 0.3 * h), (0.33 * w, h / 2), (-0.33 * w, h / 2), (-w / 2, 0.3 * h), (-w / 2, -0.05 * h),
            (-0.22 * w, -h / 2), (0.22 * w, -h / 2)]


def hull_at(y):
    """Breite, Höhe und Versatz in z des Rumpfprofils bei y."""
    for (y0, *a), (y1, *b) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            return tuple(p + t * (q - p) for p, q in zip(a, b))
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def build():
    s = Ship("horizon_cruiser")
    m = vf.materials(s.name, "cruiser")
    # Langer Facettenrumpf, vorn flach wie ein Spatel; Bugpanzer als Kappe um die Spitze, rundum 0,04 darüber
    s.add(loft([sec(y, hull(w, h), dz=dz) for y, w, h, dz in HULL]), m["hull"])
    cap = [(y, *hull_at(max(y, HULL[0][0]))) for y in (-3.92, -3.2)]
    s.add(loft([sec(y, hull(w + 0.08, h + 0.08), dz=dz) for y, w, h, dz in cap]), m["armor"])
    # Rückendeck vom Turm bis achtern, Deckplatte vor der Brücke, Maschinenlüfter dahinter
    s.add(loft([sec(-1.45, octagon(0.4, 0.2), dz=-0.05), sec(-0.9, octagon(0.82, 0.36)), sec(2.5, octagon(0.86, 0.38)),
                sec(3.05, octagon(0.5, 0.26), dz=-0.04)]), m["armor"], at=(0, 0, 0.47))
    s.add(plate([(-0.19, -0.75), (0.19, -0.75), (0.19, 0.7), (-0.19, 0.7)], 0.05, "xy"), m["hull"], at=(0, 0, 0.66))
    vent(s, m, m["armor"], (0, 2.66, 0.631), 0.42, 0.34, slats=2, rot=(-10, 0, 0))
    # Großer Zwillingsturm auf dem flachen Bug
    turret(s, m, **TURRET)
    turret_details(s, m, **TURRET)
    # Brückenturm achtern: drei Blöcke mit schräger Front, Fensterband, Brückenflügel, Mast mit Radarplatte und Licht
    z = 0.65
    for w, l, h, y in ((0.98, 1.3, 0.42, 1.55), (0.74, 0.95, 0.36, 1.62), (0.52, 0.62, 0.3, 1.6)):
        s.add(wedge(w, l, h), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.42, 0.04, 0.08), m["window"], at=(0, y - l / 2 + 0.1, z - 0.12))
    s.add(box(0.04, 0.34, 0.08), m["window"], at=(w / 2 + 0.01, y, z - 0.12), mirror=True)
    s.add(box(0.5, 0.26, 0.15), m["hull"], at=(0.41, 1.54, z - h + 0.075), mirror=True)
    s.add(post(10, 0.1, 0.07, z, z + 0.85), m["hull_round"], at=(0, 1.72, 0))
    s.add(box(0.46, 0.05, 0.2), m["armor"], at=(0, 1.63, z + 0.42), rot=(-15, 0, 0))
    s.add(sphere(0.09, 1), m["glow"], at=(0, 1.72, z + 0.9))
    # Gondeln an gepfeilten Pylonen: Einlauf in Akzentfarbe, Lichtband außen, Düse
    s.add(plate([(0.55, -0.4), (NX, 0.0), (NX, 1.9), (0.55, 1.75)], 0.18, "xy"), m["hull"], at=(0, 0, NZ), mirror=True)
    s.add(loft(smooth([sec(-0.95, ngon(24, 0.2)), sec(-0.55, ngon(24, 0.29)), sec(1.6, ngon(24, 0.3)), sec(2.5, ngon(24, 0.27))], 1)),
          m["hull_round"], at=(NX, 0, NZ), mirror=True)
    s.add(loft([sec(-1.2, ngon(24, 0.27)), sec(-0.95, ngon(24, 0.31)), sec(-0.62, ngon(24, 0.32))]), m["accent_round"], at=(NX, 0, NZ), mirror=True)
    s.add(loft([sec(-1.22, ngon(24, 0.17)), sec(-1.0, ngon(24, 0.17))]), m["dark_round"], at=(NX, 0, NZ), mirror=True)
    s.add(box(0.05, 1.8, 0.08), m["glow"], at=(NX + 0.3, 0.55, NZ + 0.04), mirror=True)
    engine(s, m, 0.27, 2.35, 3.2, dx=NX, dz=NZ, mirror=True)
    engine_details(s, m, 0.27, 2.35, 3.2, dx=NX, dz=NZ, mirror=True)
    # Heckflossen, schräg nach außen
    s.add(plate([(1.45, 0.0), (2.95, 0.0), (3.35, 0.9), (2.55, 0.94)], 0.1, "yz"), m["accent"], at=(0.4, 0, 0.3), rot=(0, 28, 0), mirror=True)
    # Unterseite: Kinn unter dem Bug mit zweiflügeligem Bootstor nach vorn, der Kiel wächst aus ihm
    s.add(wedge(0.56, 0.9, 0.26), m["hull"], at=(0, -2.45, -0.21), rot=(180, 0, 0))
    s.add(box(0.2, 0.06, 0.15), m["dark"], at=(0.13, -2.9, -0.25), mirror=True)
    s.add(plate([(-2.5, 0.0), (2.6, 0.0), (2.85, -0.56), (0.5, -0.6), (-1.6, -0.44)], 0.16, "yz"), m["dark"], at=(0, 0, -0.22))
    # Drei Spanten je Seite auf der unteren Fase vor den Pylonen
    for y in (-1.9, -1.45, -1.0):
        w, h, dz = hull_at(y)
        x0, z0, x1, z1 = 0.22 * w + 0.02, dz - h / 2, w / 2 - 0.02, dz - 0.05 * h
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.1, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    # Zwei Hauptdüsen, so weit nach achtern, dass ihr Gehäusesegment hinter dem Heck liegt
    engine(s, m, 0.24, 3.25, 4.0, dx=0.34, dz=-0.02, mirror=True)
    engine_details(s, m, 0.24, 3.25, 4.0, dx=0.34, dz=-0.02, mirror=True)
    return s.finish()
