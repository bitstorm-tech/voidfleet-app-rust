"""Siegel (Flaggschiff von Kommandantin Vey, Boss von Sektor 3) in der detailreichen Fassung (docs/stilregeln.md), mit
Kleinteilen für den dreckigen Look: der Mirage-Rumpf mit eigener Prägung.

Rumpf, Rückengrat mit Nähten, Diamantflügel, Unterseite und Heck wie die Mirage (fleet/mirage.py), in Veys Karmin statt
Türkis. Anders als die Mirage: keine Projektorflossen in der Schiffsmitte, sondern zwei hohe, senkrechte Heckflossen
mit Karminplatte und leuchtender Oberkante; Klingen in Karmin an den Flügelspitzen statt der Flügelplatten; zwei
Canards am Bug; ein zweistufiger Kommandoturm mit umlaufendem Fensterband und Mast statt der flachen Kabine; dahinter
das runde Siegel als Wappen: Goldrand, dunkle Scheibe, ein Schlüsselloch in Karmin, die Rundung zum Bug.
Details nach Tarnkappen-Logik: flach aufliegend, sparsam, ohne abstehende Teile, in Detailzonen mit Zweck; ruhig bleiben
Rückengrat, Turm, Flossen und die Flügelspitzen mit den Klingen:
- Bug: zwei flache Sensorfenster auf den oberen Bugfacetten
- Flügel: je ein eingelassener Waffenschacht an der Flügelwurzel (Rahmen, dunkle Vertiefung, zwei bündige Klappen),
  dahinter eine flache Paneelstufe
- Unterseite: Bugkanonen in flachen Gondeln unter den Flügelwurzeln, die Mündung bündig in der Stirn
- Heck: Gehäusesegment und Düsenkranz an beiden Düsen (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`), weiter nach Tarnkappen-Logik, also fast alles bündig: Einzelfenster an der Kabine unter dem
Turm, Wartungsluken auf Rumpffacetten und Flügelwurzeln, Klappenspalt an der Flügelhinterkante, Fahrwerksklappen und
Kühlschlitze der Kanonengondeln an der Unterseite, Kühlrippen an den Düsen. Abstehen dürfen nur Antennen (Peitschen
auf dem Turm, Blattantenne auf der Kabine, Staurohr am Bug) und die Positionslichter an den Flügelspitzen.
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import block, engine, engine_details, on_surface, stealth, trim_line, windows
from vf import Ship, box, grow, loft, ngon, plate, post, sec, segment, sphere, sweep, tip, tube

# Rumpfstationen (y, Breite an der Kimm, Höhe des Grats, Tiefe des Bodens)
STATIONS = [(-3.4, 0.9, 0.18, 0.12), (-1.8, 1.8, 0.32, 0.22), (0.2, 2.2, 0.38, 0.26), (2.4, 2.0, 0.36, 0.26),
            (3.6, 1.6, 0.3, 0.22), (4.15, 1.4, 0.26, 0.2)]
TOP = 0.36  # Anteil des flachen Grats an der Breite
LOW = 0.74  # Anteil des flachen Bodens an der Breite
WING = [(0.7, -2.7), (2.25, 0.95), (0.8, 2.95), (0.65, 2.95), (0.6, -2.7)]  # Diamantflügel (x, y), Oberseite auf z = 0,08


def keyhole(cy=-0.14, r=0.24, neck=0.09, foot=0.2, end=0.42, n=16):
    """Schlüsselloch (x, y): Rundung um (0, cy) zum Bug, Fuß nach achtern bis y = end."""
    a0 = math.asin(neck / r)
    arc = [(r * math.sin(a), cy + r * math.cos(a)) for a in (a0 + (2 * math.pi - 2 * a0) * k / n for k in range(n + 1))]
    return [(foot, end)] + arc + [(-foot, end)]


def joint(y, w, up, z=0.08):
    """Punkt der Zierlinie auf der Fuge zwischen Rumpfflanke und Flügeloberseite (Höhe z) an der Station y."""
    return (w / 2 - (w / 2 - w * TOP / 2) * z / up + 0.01, y, z)


def seam(s, mat, points, w=0.06, h=0.05, mirror=True):
    """Naht oder Lichtband entlang einer Punktfolge (x, y, z)."""
    s.add(sweep(points, vf.rect(w, h)), mat, mirror=mirror)


def frame(outline, width, z0, z1):
    """Flacher Rahmen um einen konvexen Umriss (x, y): innen der Umriss, außen um `width` größer, von z0 bis z1."""
    outer = grow(outline, width)
    # Der letzte Ring wiederholt den ersten; loft verschweißt die doppelten Punkte, der Rahmen ist also geschlossen
    rings = [[Vector((x, y, z)) for x, y in pts] for pts, z in ((outline, z0), (outer, z0), (outer, z1), (outline, z1), (outline, z0))]
    return loft(rings, cap_start=False, cap_end=False)


def clip(outline, y, keep_front):
    """Teil eines konvexen Umrisses (x, y) vor (keep_front) bzw. hinter der Linie y."""
    out = []
    for (ax, ay), (bx, by) in zip(outline, outline[1:] + outline[:1]):
        a_in, b_in = (ay <= y) == keep_front, (by <= y) == keep_front
        if a_in:
            out.append((ax, ay))
        if a_in != b_in:
            t = (y - ay) / (by - ay)
            out.append((ax + t * (bx - ax), y))
    return out


def bay(s, m, outline, split, z):
    """Eingelassener Waffenschacht auf einer waagrechten Fläche in Höhe z (gespiegelt): dunkle Vertiefung im Umriss in
    einem flachen Rahmen aus Rumpfton, darin zwei bündige dunkle Klappen, die sich bei y = split teilen."""
    s.add(block(outline, [(z - 0.03, 0.0), (z + 0.02, 0.0)]), m["dark"], mirror=True)
    s.add(frame(outline, 0.06, z - 0.02, z + 0.05), m["phantom"], mirror=True)
    for front in (True, False):
        s.add(block(grow(clip(outline, split, front), -0.035), [(z - 0.02, 0.0), (z + 0.035, 0.0)]), m["dark"], mirror=True)


def details(s, m, tree):
    """Kleinteile (Modulkommentar); `tree` ist der BVH-Baum des Rumpfs."""
    # ---------------------------------------------------------------- Kommandoturm: Einzelfenster an der Kabine,
    # Peitschenantennen neben dem Mast, Blattantenne auf dem Kabinendach
    windows(s, m, [(0.45 + 0.03 * (y + 0.95) / 1.35, y, 0.62) for y in (-0.55 + 0.14 * j for j in range(5))])
    s.add(post(6, 0.012, 0.005, 1.4, 1.95), m["dark_round"], at=(0.14, 0.32, 0), mirror=True)
    s.add(box(0.02, 0.22, 0.14), m["dark"], at=(0, -1.15, 0.98))

    # ---------------------------------------------------------------- Bug: Staurohr
    s.add(segment((0, -4.6, 0.02), (0, -4.95, 0.02), 0.012, 6), m["steel"])

    # ---------------------------------------------------------------- Wartungsluken, bündig: Rumpffacette neben der
    # Kabine, Flügelwurzel vorn
    hit, rot = on_surface(tree, 0.67, -0.3)
    s.add(box(0.2, 0.36, 0.03), m["dark"], at=tuple(hit), rot=rot, mirror=True)
    s.add(box(0.14, 0.3, 0.03), m["dark"], at=(0.92, -1.6, 0.08), mirror=True)

    # ---------------------------------------------------------------- Flügel: Klappenspalt vor der Hinterkante,
    # Positionslicht an der Spitze
    (ax, ay), (bx, by), (nx, ny) = (1.53, 1.6), (0.855, 2.534), (-0.0122, -0.0088)  # Spalt a→b, halbe Breite quer dazu
    gap = [(ax + nx, ay + ny), (bx + nx, by + ny), (bx - nx, by - ny), (ax - nx, ay - ny)]
    s.add(plate(gap, 0.03, "xy"), m["dark"], at=(0, 0, 0.075), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(2.31, 0.95, 0.0), mirror=True)

    # ---------------------------------------------------------------- Unterseite: Fahrwerksklappen (hell, damit sie in
    # der Karte nicht im dunklen Boden versinken), Kühlschlitze unter den Kanonengondeln
    for y in (0.8, 1.8):
        s.add(box(0.3, 0.55, 0.03), m["hull"], at=(0.42, y, -0.265), mirror=True)
    s.add(box(0.24, 0.5, 0.03), m["hull"], at=(0, -2.4, -0.1875), rot=(-3.58, 0, 0))
    for y in (-1.9, -1.75, -1.6):
        s.add(box(0.2, 0.03, 0.02), m["hull"], at=(0.8, y, -0.305), mirror=True)

    # ---------------------------------------------------------------- Kühlrippen an den Düsen
    for t in (0.41, 0.455, 0.5):
        s.add(tube(12, 0.342, 0.342, 3.6 + t, 3.625 + t, 0.55, -0.02), m["steel"], mirror=True)


def build():
    s = Ship("seal")
    m = vf.materials(s.name, "fighter")
    m["phantom"] = vf.toon(f"{s.name}_phantom", palette.PHANTOM)
    m["sensor"] = vf.toon(f"{s.name}_sensor", palette.GLASS)  # Sensorfenster: Glas ohne Glanzpunkt, eckig wie die Facetten

    # ---------------------------------------------------------------- Rumpf, Zierlinie, Rückengrat
    # Rumpf wie die Mirage: flacher Facettenpfeil, Kimmkante bei z = 0; goldene Zierlinie auf der Kimm, im Flügelbereich auf der Fuge
    part = loft([tip(-4.65, 0, 0.02)] + [sec(y, stealth(w, up, down, TOP, LOW)) for y, w, up, down in STATIONS])
    tree = BVHTree.FromBMesh(part)
    s.add(part, m["phantom"])
    trim_line(s, m, [(0.0, -4.66, 0.02), (0.46, -3.4, 0.0), (0.66, -2.75, 0.0)] + [joint(y, w, up) for y, w, up, _ in STATIONS[1:4]]
              + [(0.84, 3.05, 0.0)] + [(w / 2 + 0.01, y, 0.0) for y, w, _, _ in STATIONS[4:]])
    # Rückengrat: dunklere, facettierte Lage auf dem Grat, Nähte an ihrem Fuß
    spine = [(-3.9, 0.0, 0.12)] + [(y, w * TOP * 0.95, up) for y, w, up, _ in STATIONS[:5]]
    rings = [sec(y, [(hw / 2, up - 0.04), (hw * 0.3, up + 0.16), (-hw * 0.3, up + 0.16), (-hw / 2, up - 0.04)]) for y, hw, up in spine[1:]]
    s.add(loft([tip(spine[0][0], 0, spine[0][2])] + rings), m["dark"])
    seam(s, m["signal"], [(0.0, -3.92, 0.14)] + [(hw / 2 + 0.02, y, up + 0.01) for y, hw, up in spine[1:]])

    # ---------------------------------------------------------------- Bug: Canards, Sensorfenster auf den oberen Bugfacetten
    s.add(block([(0.3, -3.75), (1.1, -3.25), (1.1, -3.02), (0.3, -3.08)], [(-0.05, 0.02), (0.05, 0.0)]), m["accent"],
          at=(0, 0, 0.02), mirror=True)
    hit, rot = on_surface(tree, 0.42, -2.45)
    s.add(plate([(-0.05, -0.3), (0.07, -0.26), (0.07, 0.3), (-0.05, 0.26)], 0.04, "xy"), m["sensor"], at=tuple(hit), rot=rot, mirror=True)

    # ---------------------------------------------------------------- Flügel: Facettenplatte, Klingen, Naht, Waffenschacht, Paneelstufe
    s.add(block(WING, [(-0.09, 0.08), (-0.04, 0.0), (0.03, 0.0), (0.08, 0.1)]), m["phantom"], mirror=True)
    blade = [(1.62, -0.55), (2.31, 0.95), (1.75, 1.75), (1.45, 0.8)]
    s.add(block(blade, [(-0.12, 0.03), (-0.06, 0.0), (0.06, 0.0), (0.12, 0.03)]), m["accent"], mirror=True)
    seam(s, m["signal"], [(0.72, -2.72, 0.0), (2.28, 0.95, 0.0)], 0.07, 0.08)
    bay(s, m, [(1.05, -1.0), (1.2, -1.0), (1.33, -0.3), (1.3, 0.7), (1.13, 0.7)], -0.15, 0.08)
    s.add(block([(1.08, 1.0), (1.38, 1.0), (1.5, 1.45), (1.08, 2.15)], [(0.06, 0.0), (0.13, 0.03)]), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Zwei hohe, senkrechte Heckflossen: dunkler Rahmen, Karminplatte, leuchtende Oberkante, Naht an der Vorderkante
    fin = [(2.45, -0.1), (3.95, -0.1), (4.25, 1.35), (3.45, 1.4)]
    g = Ship("seal_fin")
    g.add(block(fin, [(-0.07, 0.05), (-0.03, 0.0), (0.03, 0.0), (0.07, 0.05)], "yz"), m["phantom"])
    g.add(block([(3.0, 0.3), (3.88, 0.3), (4.08, 1.12), (3.5, 1.16)], [(-0.085, 0.0), (0.085, 0.0)], "yz"), m["accent"])
    g.add(sweep([(0.0, 3.47, 1.4), (0.0, 4.25, 1.35)], vf.rect(0.1, 0.08)), m["glow"])
    g.add(sweep([(0.0, 2.55, 0.0), (0.0, 3.45, 1.38)], vf.rect(0.16, 0.06)), m["signal"])
    for side in (1, -1):
        s.place(g, at=(side * 0.62, 0, 0.18), rot=(0, side * 8, 0))
    g.bm.free()

    # ---------------------------------------------------------------- Kommandoturm: Facettenkabine auf dem Rückengrat, darauf ein zweiter, schmalerer Stock mit umlaufendem Fensterband, Mast mit Licht
    cabin = [(0.1, -1.75), (0.45, -0.95), (0.48, 0.4), (0.32, 0.75), (-0.32, 0.75), (-0.48, 0.4), (-0.45, -0.95), (-0.1, -1.75)]
    s.add(block(cabin, [(0.25, 0.0), (0.8, 0.0), (0.92, 0.12)]), m["hull"])
    tower = [(0.12, -0.75), (0.3, -0.35), (0.3, 0.5), (-0.3, 0.5), (-0.3, -0.35), (-0.12, -0.75)]
    s.add(block(tower, [(0.85, 0.0), (1.32, 0.0), (1.42, 0.1)]), m["hull"])
    band = [(x, y, 1.2) for x, y in grow(tower, 0.012)]
    s.add(sweep(band + band[:2], vf.rect(0.04, 0.1)), m["window"])
    s.add(post(10, 0.09, 0.06, 1.4, 2.0), m["hull_round"], at=(0, 0.2, 0))
    s.add(sphere(0.09, 1), m["glow"], at=(0, 0.2, 2.05))

    # ---------------------------------------------------------------- Siegel hinter dem Turm: runde Platte mit Goldrand, darin ein Schlüsselloch in Karmin, die Rundung zum Bug
    y0 = 1.65
    disk = ngon(16, 0.66)
    s.add(block(grow(disk, 0.09), [(0.22, 0.0), (0.6, 0.0)]), m["trim"], at=(0, y0, 0))
    s.add(block(disk, [(0.25, 0.0), (0.62, 0.0), (0.67, 0.04)]), m["dark"], at=(0, y0, 0))
    s.add(plate(keyhole(cy=-0.15, r=0.255, neck=0.095, foot=0.21, end=0.45), 0.06, "xy"), m["signal"], at=(0, y0, 0.68))

    # ---------------------------------------------------------------- Unterseite: Kiel mit Karminplatte (im Hero im Licht), schräge Bauchflossen unter den Flügelwurzeln, Nähte auf den Bodenkanten, Bugkanonen in flachen Gondeln
    s.add(plate([(-1.6, 0.0), (3.3, 0.0), (3.0, -0.42), (-0.6, -0.38)], 0.16, "yz"), m["dark"], at=(0, 0, -0.2))
    s.add(plate([(-0.7, -0.1), (2.75, -0.1), (2.6, -0.32), (-0.25, -0.3)], 0.2, "yz"), m["accent"], at=(0, 0, -0.2))
    belly = [(-0.7, 0.0), (1.3, 0.0), (1.1, -0.36), (0.0, -0.4)]
    for side in (1, -1):
        s.add(block(belly, [(-0.06, 0.04), (0.06, 0.04)], "yz"), m["phantom"], at=(side * 1.25, 0, -0.06), rot=(0, side * -18, 0))
    seam(s, m["signal"], [(w * LOW / 2 + 0.01, y, -down - 0.005) for y, w, _, down in STATIONS[:5]])
    pod = [(0.66, -2.78), (0.94, -2.78), (0.98, -2.3), (0.98, -0.6), (0.62, -0.4), (0.62, -2.3)]
    s.add(block(pod, [(-0.3, 0.06), (-0.22, 0.0), (-0.05, 0.0)]), m["dark"], mirror=True)
    s.add(tube(16, 0.12, 0.12, -2.82, -2.7), m["dark_round"], at=(0.8, 0, -0.17), mirror=True)

    # ---------------------------------------------------------------- Heck: flache Düsenverkleidung, zwei Triebwerke
    s.add(block([(0.95, 2.9), (0.95, 3.9), (-0.95, 3.9), (-0.95, 2.9)], [(-0.3, 0.1), (-0.2, 0.0), (0.22, 0.0), (0.3, 0.1)]), m["dark"])
    engine(s, m, 0.3, 3.6, 4.6, dx=0.55, dz=-0.02, mirror=True)
    engine_details(s, m, 0.3, 3.6, 4.6, dx=0.55, dz=-0.02, mirror=True)
    details(s, m, tree)
    return s.finish()
