"""Siegel (Flaggschiff von Kommandantin Vey, Boss von Sektor 3), Clean Toon: der Mirage-Rumpf mit eigener Prägung.

Rumpf, Rückengrat mit Nähten, Diamantflügel, Unterseite und Heck wie die Mirage (fleet/mirage.py), in Veys Karmin statt
Türkis. Anders als die Mirage: keine Projektorflossen in der Schiffsmitte, sondern zwei hohe, senkrechte Heckflossen
mit Karminplatte und leuchtender Oberkante; Klingen in Karmin an den Flügelspitzen statt der Flügelplatten; zwei
Canards am Bug; ein zweistufiger Kommandoturm mit umlaufendem Fensterband und Mast statt der flachen Kabine; dahinter
das runde Siegel als Wappen: Goldrand, dunkle Scheibe, ein Schlüsselloch in Karmin, die Rundung zum Bug.

Vey steht nicht in palette.ADMIRALS/FLAGSHIPS: Ihre Farben stehen vorerst als Konstanten hier, der Flaggschiff-
Materialsatz (Akzent, Lichter, `signal`, `trim`) entsteht in `flagship_materials`.
"""

import math

import palette
import vf
from parts import block, engine, stealth, trim_line
from vf import Ship, grow, loft, ngon, plate, post, sec, sphere, sweep, tip, tube

# Rumpfstationen (y, Breite an der Kimm, Höhe des Grats, Tiefe des Bodens)
STATIONS = [(-3.4, 0.9, 0.18, 0.12), (-1.8, 1.8, 0.32, 0.22), (0.2, 2.2, 0.38, 0.26), (2.4, 2.0, 0.36, 0.26),
            (3.6, 1.6, 0.3, 0.22), (4.15, 1.4, 0.26, 0.2)]
TOP = 0.36  # Anteil des flachen Grats an der Breite
LOW = 0.74  # Anteil des flachen Bodens an der Breite

VEY = "#ff4f6e"  # UI-Farbe von Kommandantin Vey (Karmin), Vorschlag für palette.ADMIRALS
VEY_ACCENT = palette.tones(palette.oklch(0.58, 0.13, 14))  # Akzentfläche, gedämpft wie die anderen Admiralsakzente


def keyhole(cy=-0.14, r=0.24, neck=0.09, foot=0.2, end=0.42, n=16):
    """Schlüsselloch (x, y): Rundung um (0, cy) zum Bug, Fuß nach achtern bis y = end."""
    a0 = math.asin(neck / r)
    arc = [(r * math.sin(a), cy + r * math.cos(a)) for a in (a0 + (2 * math.pi - 2 * a0) * k / n for k in range(n + 1))]
    return [(foot, end)] + arc + [(-foot, end)]


def flagship_materials(m, name):
    """Flaggschiff-Satz in Veys Farben über den Klassensatz legen (wie vf.materials für palette.FLAGSHIPS)."""
    m["accent"] = vf.toon(f"{name}_accent", VEY_ACCENT)
    m["accent_round"] = vf.toon(f"{name}_accent_round", VEY_ACCENT, round=True, glint=vf.GLINT)
    m["glow"] = vf.light(f"{name}_glow", VEY)
    m["core"] = vf.light(f"{name}_core", palette.pale(VEY))
    m["signal"] = vf.light(f"{name}_signal", VEY, glow=False)
    m["trim"] = vf.toon(f"{name}_trim", palette.TRIM)


def joint(y, w, up, z=0.08):
    """Punkt der Zierlinie auf der Fuge zwischen Rumpfflanke und Flügeloberseite (Höhe z) an der Station y."""
    return (w / 2 - (w / 2 - w * TOP / 2) * z / up + 0.01, y, z)


def seam(s, mat, points, w=0.06, h=0.05, mirror=True):
    """Naht oder Lichtband entlang einer Punktfolge (x, y, z)."""
    s.add(sweep(points, vf.rect(w, h)), mat, mirror=mirror)


def build():
    s = Ship("seal")
    m = vf.materials(s.name, "fighter")
    flagship_materials(m, s.name)
    m["phantom"] = vf.toon(f"{s.name}_phantom", palette.PHANTOM)

    # Rumpf wie die Mirage: flacher Facettenpfeil, Kimmkante bei z = 0; goldene Zierlinie auf der Kimm, im Flügelbereich auf der Fuge
    s.add(loft([tip(-4.65, 0, 0.02)] + [sec(y, stealth(w, up, down, TOP, LOW)) for y, w, up, down in STATIONS]), m["phantom"])
    trim_line(s, m, [(0.0, -4.66, 0.02), (0.46, -3.4, 0.0), (0.66, -2.75, 0.0)] + [joint(y, w, up) for y, w, up, _ in STATIONS[1:4]]
              + [(0.84, 3.05, 0.0)] + [(w / 2 + 0.01, y, 0.0) for y, w, _, _ in STATIONS[4:]])
    # Rückengrat: dunklere, facettierte Lage auf dem Grat, Nähte an ihrem Fuß
    spine = [(-3.9, 0.0, 0.12)] + [(y, w * TOP * 0.95, up) for y, w, up, _ in STATIONS[:5]]
    rings = [sec(y, [(hw / 2, up - 0.04), (hw * 0.3, up + 0.16), (-hw * 0.3, up + 0.16), (-hw / 2, up - 0.04)]) for y, hw, up in spine[1:]]
    s.add(loft([tip(spine[0][0], 0, spine[0][2])] + rings), m["dark"])
    seam(s, m["signal"], [(0.0, -3.92, 0.14)] + [(hw / 2 + 0.02, y, up + 0.01) for y, hw, up in spine[1:]])

    # Diamantflügel: Facettenplatte mit gefasten Kanten, Klingen in Karmin an den Spitzen, Naht auf der Vorderkante
    wing = [(0.7, -2.7), (2.25, 0.95), (0.8, 2.95), (0.65, 2.95), (0.6, -2.7)]
    s.add(block(wing, [(-0.09, 0.08), (-0.04, 0.0), (0.03, 0.0), (0.08, 0.1)]), m["phantom"], mirror=True)
    blade = [(1.62, -0.55), (2.31, 0.95), (1.75, 1.75), (1.45, 0.8)]
    s.add(block(blade, [(-0.12, 0.03), (-0.06, 0.0), (0.06, 0.0), (0.12, 0.03)]), m["accent"], mirror=True)
    # Canards am Bug
    s.add(block([(0.3, -3.75), (1.1, -3.25), (1.1, -3.02), (0.3, -3.08)], [(-0.05, 0.02), (0.05, 0.0)]), m["accent"],
          at=(0, 0, 0.02), mirror=True)
    seam(s, m["signal"], [(0.72, -2.72, 0.0), (2.28, 0.95, 0.0)], 0.07, 0.08)

    # Zwei hohe, senkrechte Heckflossen: dunkler Rahmen, Karminplatte, leuchtende Oberkante, Naht an der Vorderkante
    fin = [(2.45, -0.1), (3.95, -0.1), (4.25, 1.35), (3.45, 1.4)]
    g = Ship("seal_fin")
    g.add(block(fin, [(-0.07, 0.05), (-0.03, 0.0), (0.03, 0.0), (0.07, 0.05)], "yz"), m["phantom"])
    g.add(block([(3.0, 0.3), (3.88, 0.3), (4.08, 1.12), (3.5, 1.16)], [(-0.085, 0.0), (0.085, 0.0)], "yz"), m["accent"])
    g.add(sweep([(0.0, 3.47, 1.4), (0.0, 4.25, 1.35)], vf.rect(0.1, 0.08)), m["glow"])
    g.add(sweep([(0.0, 2.55, 0.0), (0.0, 3.45, 1.38)], vf.rect(0.16, 0.06)), m["signal"])
    for side in (1, -1):
        s.place(g, at=(side * 0.62, 0, 0.18), rot=(0, side * 8, 0))
    g.bm.free()

    # Kommandoturm: Facettenkabine auf dem Rückengrat, darauf ein zweiter, schmalerer Stock mit umlaufendem Fensterband,
    # Mast mit Licht
    cabin = [(0.1, -1.75), (0.45, -0.95), (0.48, 0.4), (0.32, 0.75), (-0.32, 0.75), (-0.48, 0.4), (-0.45, -0.95), (-0.1, -1.75)]
    s.add(block(cabin, [(0.25, 0.0), (0.8, 0.0), (0.92, 0.12)]), m["hull"])
    tower = [(0.12, -0.75), (0.3, -0.35), (0.3, 0.5), (-0.3, 0.5), (-0.3, -0.35), (-0.12, -0.75)]
    s.add(block(tower, [(0.85, 0.0), (1.32, 0.0), (1.42, 0.1)]), m["hull"])
    band = [(x, y, 1.2) for x, y in grow(tower, 0.012)]
    s.add(sweep(band + band[:2], vf.rect(0.04, 0.1)), m["window"])
    s.add(post(10, 0.09, 0.06, 1.4, 2.0), m["hull_round"], at=(0, 0.2, 0))
    s.add(sphere(0.09, 1), m["glow"], at=(0, 0.2, 2.05))

    # Siegel hinter dem Turm: runde Platte mit Goldrand, darin ein Schlüsselloch in Karmin, die Rundung zum Bug
    y0 = 1.65
    disk = ngon(16, 0.62)
    s.add(block(grow(disk, 0.09), [(0.22, 0.0), (0.6, 0.0)]), m["trim"], at=(0, y0, 0))
    s.add(block(disk, [(0.25, 0.0), (0.62, 0.0), (0.67, 0.04)]), m["dark"], at=(0, y0, 0))
    s.add(plate(keyhole(), 0.06, "xy"), m["signal"], at=(0, y0, 0.68))

    # Unterseite: Kiel mit Karminplatte (im Hero im Licht), schräge Bauchflossen unter den Flügelwurzeln, Nähte auf den
    # Bodenkanten, kurze Bugkanonen in Gehäusen unter den Flügelwurzeln
    s.add(plate([(-1.6, 0.0), (3.3, 0.0), (3.0, -0.42), (-0.6, -0.38)], 0.16, "yz"), m["dark"], at=(0, 0, -0.2))
    s.add(plate([(-0.7, -0.1), (2.75, -0.1), (2.6, -0.32), (-0.25, -0.3)], 0.2, "yz"), m["accent"], at=(0, 0, -0.2))
    belly = [(-0.7, 0.0), (1.3, 0.0), (1.1, -0.36), (0.0, -0.4)]
    for side in (1, -1):
        s.add(block(belly, [(-0.06, 0.04), (0.06, 0.04)], "yz"), m["phantom"], at=(side * 1.25, 0, -0.06), rot=(0, side * -18, 0))
    seam(s, m["signal"], [(w * LOW / 2 + 0.01, y, -down - 0.005) for y, w, _, down in STATIONS[:5]])
    s.add(block([(0.62, -2.4), (0.98, -2.0), (0.98, -0.6), (0.62, -0.4)], [(-0.3, 0.06), (-0.22, 0.0), (-0.05, 0.0)]), m["dark"], mirror=True)
    s.add(tube(16, 0.1, 0.1, -2.95, -2.2), m["steel"], at=(0.8, 0, -0.17), mirror=True)
    s.add(tube(16, 0.13, 0.13, -3.1, -2.85), m["dark_round"], at=(0.8, 0, -0.17), mirror=True)

    # Heck: flache Düsenverkleidung, zwei Triebwerke
    s.add(block([(0.95, 2.9), (0.95, 3.9), (-0.95, 3.9), (-0.95, 2.9)], [(-0.3, 0.1), (-0.2, 0.0), (0.22, 0.0), (0.3, 0.1)]), m["dark"])
    engine(s, m, 0.3, 3.6, 4.6, dx=0.55, dz=-0.02, mirror=True)
    return s.finish()
