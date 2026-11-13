"""Siegel (Flaggschiff von Kommandantin Vey, Boss von Sektor 3), Fassung für die Nahaufnahme nach docs/stilregeln.md,
als Tarnkappenschiff zurückhaltend: der Mirage-Rumpf mit eigener Prägung, glatte runde Teile, bündige Feinteile.

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

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme): Das Siegel bleibt glatt, die Facetten sind sein Kennzeichen.
Gefast sind deshalb nur der Goldrand des Siegels und der Kiel, die Facettenblöcke behalten ihre scharfen Kanten; rund
sind Siegel, Düsen und Mast. Die Feinteile liegen flach auf und zeigen, dass das Schiff gefertigt ist: Senkschrauben an
den Wartungsklappen, um die Waffenschächte, die Sensorfenster und im Goldrand des Siegels, bündige Klappen an den
Flossenwurzeln, eine Tür an der Kabine und die Betankungsklappe auf dem Rückengrat, Drehzapfen der Canards,
Sägezahnnähte quer über die Bugfacetten und die Flügel, eine Naht um die Radarnase und entlang der Flügelvorderkante.
Beschriftung nur als wenige Marken: Warnbänder vor den Waffenschächten und neben den Düsen, eine Leitlinie zur
Betankungsklappe. Eine Kennnummer trägt das Tarnkappenschiff nicht, sein Zeichen ist das Siegel.
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import TURN, block, bolts, engine, engine_details, on_surface, spaced, stealth, stripes, trim_line, windows
from vf import Ship, box, chamfer, grow, loft, ngon, plate, post, sec, segment, sphere, sweep, tip, tube

# Rumpfstationen (y, Breite an der Kimm, Höhe des Grats, Tiefe des Bodens)
STATIONS = [(-3.4, 0.9, 0.18, 0.12), (-1.8, 1.8, 0.32, 0.22), (0.2, 2.2, 0.38, 0.26), (2.4, 2.0, 0.36, 0.26),
            (3.6, 1.6, 0.3, 0.22), (4.15, 1.4, 0.26, 0.2)]
TOP = 0.36  # Anteil des flachen Grats an der Breite
LOW = 0.74  # Anteil des flachen Bodens an der Breite
WING = [(0.7, -2.7), (2.25, 0.95), (0.8, 2.95), (0.65, 2.95), (0.6, -2.7)]  # Diamantflügel (x, y), Oberseite auf z = 0,08
BAY = [(1.05, -1.0), (1.2, -1.0), (1.33, -0.3), (1.3, 0.7), (1.13, 0.7)]  # Waffenschacht an der Flügelwurzel
SENSOR = [(-0.05, -0.3), (0.07, -0.26), (0.07, 0.3), (-0.05, 0.26)]  # Sensorfenster auf der Bugfacette
SEAM = vf.rect(0.014, 0.012)  # Querschnitt der feinen Nähte: zur Hälfte in der Fläche


def keyhole(cy=-0.14, r=0.24, neck=0.09, foot=0.2, end=0.42, n=24):
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


def screw(g, m, x, y, z, out="z"):
    """Senkschraube auf einer Fläche in Höhe z, die nach `out` schaut: 0,004 über der Fläche."""
    g.add(post(6, 0.009, 0.009, -0.006, 0.004), m["steel_flat"], at=(x, y, z), rot=TURN[out])


def panel(s, m, at, w, l, rot=(0, 0, 0), mirror=True):
    """Bündige Wartungsklappe (lokal +z nach außen, w quer, l längs): dunkle Platte, 0,015 über der Fläche, mit
    Senkschrauben in den Ecken und an den Längsseiten."""
    g = Ship("panel")
    g.add(box(w, l, 0.03), m["dark"])
    for sx in (-1, 1):
        for sy in (-1, 0, 1):
            screw(g, m, sx * (w / 2 - 0.022), sy * (l / 2 - 0.022), 0.015)
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def zigzag(a, b, teeth, depth):
    """Punkte (x, y) einer Sägezahnlinie von a nach b: `teeth` Zähne, um `depth` nach rechts der Laufrichtung."""
    (ax, ay), (bx, by) = a, b
    length = math.hypot(bx - ax, by - ay)
    nx, ny = (by - ay) / length * depth, -(bx - ax) / length * depth
    n = 2 * teeth
    return [(ax + (bx - ax) * k / n + nx * (k % 2), ay + (by - ay) * k / n + ny * (k % 2)) for k in range(n + 1)]


def fine(s, m, tree):
    """Feinteile und Beschriftung (Modulkommentar), alle flach aufliegend; `tree` ist der BVH-Baum des Rumpfs."""
    # ---------------------------------------------------------------- Bug: Naht um die Radarnase, Sägezahnnaht,
    # Schrauben um die Sensorfenster, Drehzapfen der Canards
    s.add(sweep([on_surface(tree, x, -3.95)[0] for x in (0.24, 0.095, -0.095, -0.24)], SEAM), m["dark"])
    s.add(sweep([on_surface(tree, x, y)[0] for x, y in zigzag((0.38, -1.93), (0.76, -1.93), 4, 0.09)], SEAM), m["dark"], mirror=True)
    hit, rot = on_surface(tree, 0.42, -2.45)
    g = Ship("sensor")
    for x, y in ((-0.072, -0.3), (-0.072, -0.02), (-0.072, 0.26), (0.092, -0.26), (0.092, 0.02), (0.092, 0.3)):
        screw(g, m, x, y, 0.0)
    s.place(g, at=tuple(hit), rot=rot, mirror=True)
    g.bm.free()
    s.add(post(12, 0.07, 0.07, 0.06, 0.078), m["dark"], at=(0.62, -3.3, 0), mirror=True)
    screw(s, m, 0.62, -3.3, 0.078)
    screw(s, m, -0.62, -3.3, 0.078)

    # ---------------------------------------------------------------- Rückengrat: Betankungsklappe mit Leitlinie
    # (das Rückengrat steigt vor der Kabine um 5°)
    panel(s, m, (0, -2.3, 0.436), 0.2, 0.3, rot=(5, 0, 0), mirror=False)
    s.add(box(0.016, 0.3, 0.008), m["paint"], at=(0, -2.72, 0.3995 + 0.001), rot=(5, 0, 0))

    # ---------------------------------------------------------------- Kabine: bündige Tür hinter den Fenstern
    panel(s, m, (0.4756, 0.2, 0.56), 0.3, 0.16, rot=(0, 90, 0))

    # ---------------------------------------------------------------- Flügel: Schrauben um den Waffenschacht, Nähte
    # Senkschrauben auf dem Rahmen des Schachts, eine Naht hinter der Vorderkante bis zur Klinge, eine Sägezahnnaht
    # quer zwischen Schacht und Paneelstufe, drei Antriebsverkleidungen am Klappenspalt
    for x, y in spaced(grow(BAY, 0.03), 0.22):
        screw(s, m, x, y, 0.13)
        screw(s, m, -x, y, 0.13)
    s.add(sweep([(0.7 + 1.55 * (y + 2.7) / 3.65 - 0.13, y, 0.08) for y in (-2.2, -0.75)], SEAM), m["dark"], mirror=True)
    s.add(sweep([(x, y, 0.08) for x, y in zigzag((0.99, 0.92), (1.43, 0.92), 5, 0.08)], SEAM), m["dark"], mirror=True)
    for t in (0.2, 0.5, 0.8):
        s.add(box(0.05, 0.14, 0.02), m["dark"], at=(1.53 - 0.675 * t, 1.6 + 0.934 * t, 0.085), rot=(0, 0, -54.1), mirror=True)

    # ---------------------------------------------------------------- Siegel: Schrauben im Goldrand
    bolts(s, m["steel"], [(0.705 * math.cos(math.radians(a)), 1.65 + 0.705 * math.sin(math.radians(a)), 0.6) for a in range(0, 360, 15)],
          r=0.011)

    # ---------------------------------------------------------------- Beschriftung: Warnbänder vor den Waffenschächten
    # und neben den Düsen
    stripes(s, m, (1.09, -1.16, 0.08), 0.04, 0.16, rot=(0, 0, 90), mirror=True)
    stripes(s, m, (0.78, 3.62, 0.3), 0.05, 0.3, mirror=True)


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
    panel(s, m, tuple(hit), 0.2, 0.36, rot)
    panel(s, m, (0.92, -1.6, 0.08), 0.14, 0.3)

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
        s.add(tube(24, 0.342, 0.342, 3.6 + t, 3.625 + t, 0.55, -0.02), m["steel"], mirror=True)


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
    s.add(plate(SENSOR, 0.04, "xy"), m["sensor"], at=tuple(hit), rot=rot, mirror=True)

    # ---------------------------------------------------------------- Flügel: Facettenplatte, Klingen, Naht, Waffenschacht, Paneelstufe
    s.add(block(WING, [(-0.09, 0.08), (-0.04, 0.0), (0.03, 0.0), (0.08, 0.1)]), m["phantom"], mirror=True)
    blade = [(1.62, -0.55), (2.31, 0.95), (1.75, 1.75), (1.45, 0.8)]
    s.add(block(blade, [(-0.12, 0.03), (-0.06, 0.0), (0.06, 0.0), (0.12, 0.03)]), m["accent"], mirror=True)
    seam(s, m["signal"], [(0.72, -2.72, 0.0), (2.28, 0.95, 0.0)], 0.07, 0.08)
    bay(s, m, BAY, -0.15, 0.08)
    s.add(block([(1.08, 1.0), (1.38, 1.0), (1.5, 1.45), (1.08, 2.15)], [(0.06, 0.0), (0.13, 0.03)]), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Zwei hohe, senkrechte Heckflossen: dunkler Rahmen, Karminplatte, leuchtende Oberkante, Naht an der Vorderkante
    fin = [(2.45, -0.1), (3.95, -0.1), (4.25, 1.35), (3.45, 1.4)]
    g = Ship("seal_fin")
    g.add(block(fin, [(-0.07, 0.05), (-0.03, 0.0), (0.03, 0.0), (0.07, 0.05)], "yz"), m["phantom"])
    g.add(block([(3.0, 0.3), (3.88, 0.3), (4.08, 1.12), (3.5, 1.16)], [(-0.085, 0.0), (0.085, 0.0)], "yz"), m["accent"])
    g.add(sweep([(0.0, 3.47, 1.4), (0.0, 4.25, 1.35)], vf.rect(0.1, 0.08)), m["glow"])
    g.add(sweep([(0.0, 2.55, 0.0), (0.0, 3.45, 1.38)], vf.rect(0.16, 0.06)), m["signal"])
    # Feinteile: bündige Klappe über dem Ruderantrieb an beiden Seiten der Flossenwurzel, unter der Karminplatte
    for sx, out in ((1, "x"), (-1, "-x")):
        g.add(box(0.03, 0.5, 0.1), m["dark"], at=(sx * 0.07, 3.4, 0.21))
        for y in (3.19, 3.4, 3.61):
            for z in (0.185, 0.235):
                screw(g, m, sx * 0.085, y, z, out)
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
    s.add(post(16, 0.09, 0.06, 1.4, 2.0), m["hull_round"], at=(0, 0.2, 0))
    s.add(sphere(0.09, 2), m["glow"], at=(0, 0.2, 2.05))

    # ---------------------------------------------------------------- Siegel hinter dem Turm: runde Platte mit Goldrand, darin ein Schlüsselloch in Karmin, die Rundung zum Bug
    y0 = 1.65
    disk = ngon(40, 0.66)
    s.add(chamfer(block(grow(disk, 0.09), [(0.22, 0.0), (0.6, 0.0)]), 0.02), m["trim"], at=(0, y0, 0))
    s.add(block(disk, [(0.25, 0.0), (0.62, 0.0), (0.67, 0.04)]), m["dark"], at=(0, y0, 0))
    s.add(plate(keyhole(cy=-0.15, r=0.255, neck=0.095, foot=0.21, end=0.45), 0.06, "xy"), m["signal"], at=(0, y0, 0.68))

    # ---------------------------------------------------------------- Unterseite: Kiel mit Karminplatte (im Hero im Licht), schräge Bauchflossen unter den Flügelwurzeln, Nähte auf den Bodenkanten, Bugkanonen in flachen Gondeln
    s.add(chamfer(plate([(-1.6, 0.0), (3.3, 0.0), (3.0, -0.42), (-0.6, -0.38)], 0.16, "yz"), 0.03), m["dark"], at=(0, 0, -0.2))
    s.add(chamfer(plate([(-0.7, -0.1), (2.75, -0.1), (2.6, -0.32), (-0.25, -0.3)], 0.2, "yz"), 0.03), m["accent"], at=(0, 0, -0.2))
    belly = [(-0.7, 0.0), (1.3, 0.0), (1.1, -0.36), (0.0, -0.4)]
    for side in (1, -1):
        s.add(block(belly, [(-0.06, 0.04), (0.06, 0.04)], "yz"), m["phantom"], at=(side * 1.25, 0, -0.06), rot=(0, side * -18, 0))
    seam(s, m["signal"], [(w * LOW / 2 + 0.01, y, -down - 0.005) for y, w, _, down in STATIONS[:5]])
    pod = [(0.66, -2.78), (0.94, -2.78), (0.98, -2.3), (0.98, -0.6), (0.62, -0.4), (0.62, -2.3)]
    s.add(block(pod, [(-0.3, 0.06), (-0.22, 0.0), (-0.05, 0.0)]), m["dark"], mirror=True)
    s.add(tube(24, 0.12, 0.12, -2.82, -2.7), m["dark_round"], at=(0.8, 0, -0.17), mirror=True)

    # ---------------------------------------------------------------- Heck: flache Düsenverkleidung, zwei Triebwerke
    s.add(block([(0.95, 2.9), (0.95, 3.9), (-0.95, 3.9), (-0.95, 2.9)], [(-0.3, 0.1), (-0.2, 0.0), (0.22, 0.0), (0.3, 0.1)]), m["dark"])
    engine(s, m, 0.3, 3.6, 4.6, dx=0.55, dz=-0.02, n=24, mirror=True)
    engine_details(s, m, 0.3, 3.6, 4.6, dx=0.55, dz=-0.02, n=24, mirror=True)
    details(s, m, tree)
    fine(s, m, tree)
    return s.finish()
