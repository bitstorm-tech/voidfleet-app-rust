"""Gemeinsame Helfer der Schiffe: Baugruppen mit dem Materialsatz aus `vf.materials` (Turm, Düse, Wappenplatte,
Zierlinien), Kleinteile nach dem Vorbild der Iron Dreadnought (Turm- und Düsendetails, Lüftungsgitter, RCS-Blöcke,
Flak, Geländer, Fensterreihen) und Formen, die mehrere Schiffe teilen (Profile, Streben, Walzen, Facettenblöcke, Bänder
auf Lofts, Blendenbalg).

Ein Helfer kommt erst hierher, wenn ein zweites Schiff ihn braucht; bis dahin lebt er im Modul seines Schiffs
(fleet/<id>.py), damit parallel arbeitende Agents sich nicht in die Quere kommen. Wer hier etwas ändert, ändert alle
Schiffe, die es benutzen: vorher und nachher bauen und die Geometrie vergleichen.
"""

import math

import bmesh
from mathutils import Matrix, Vector

from vf import Ship, box, grow, loft, plate, post, rbox, rect, revolve, rounded, sec, smooth, sweep, tube

# ---------------------------------------------------------------- Baugruppen


def turret(s, m, at, r=0.5, barrel=1.35, yaw=0.0, mirror=False):
    """Zwillingsturm, grob: Barbette, gerundetes Gehäuse in der Akzentfarbe, Blende, zwei dicke Rohre mit Hülse und
    Mündung. `at` ist die Mitte der Barbette auf dem Deck, `r` der Turmradius; die Rohre zeigen nach -Y, gedreht um `yaw`."""
    x, y, z = at
    s.add(post(24, r * 1.06, r * 1.0, 0.0, 0.14), m["dark_round"], at=(x, y, z), mirror=mirror)
    g = Ship("turret")
    g.add(loft(smooth([sec(-0.95 * r, rounded(1.25 * r, 0.52 * r, 0.12 * r)), sec(-0.5 * r, rounded(1.75 * r, 0.84 * r, 0.16 * r)),
                       sec(0.6 * r, rounded(1.75 * r, 0.88 * r, 0.16 * r)), sec(1.05 * r, rounded(1.35 * r, 0.64 * r, 0.14 * r))], 1)), m["accent_round"])
    g.add(rbox(1.3 * r, 0.3 * r, 0.56 * r, 0.05 * r, 1), m["dark"], at=(0, -1.02 * r, 0.02 * r))
    rb = 0.19 * r
    for dx in (-0.38 * r, 0.38 * r):
        g.add(tube(16, rb * 1.5, rb * 1.4, -0.42 * barrel, -1.0 * r), m["dark_round"], at=(dx, 0, 0.04 * r))
        g.add(tube(16, rb, rb * 0.92, -barrel, -0.4 * barrel), m["steel"], at=(dx, 0, 0.04 * r))
        g.add(tube(16, rb * 1.32, rb * 1.32, -barrel - 0.06, -barrel + 0.16), m["dark_round"], at=(dx, 0, 0.04 * r))
    s.place(g, at=(x, y, z + 0.14 + 0.44 * r), rot=(0, 0, yaw), mirror=mirror)
    g.bm.free()


def engine(s, m, r, y0, y1, dx=0.0, dz=0.0, n=20, mirror=False):
    """Triebwerk ohne Rippen von y0 (vorn) bis y1 (Düsenende): dunkles Gehäuse, Düsenglocke mit Wandstärke, leuchtende
    Brennkammer (`core`) mit kurzer Fahne; `n` Segmente (kleine Düsen 10–16, sonst sprengen viele das Budget)."""
    length = y1 - y0
    ya, yb = y0 + 0.6 * length, y0 + 0.78 * length
    s.add(tube(n, r, r, y0, ya, dx, dz), m["dark_round"], mirror=mirror)
    # Die Glocke beginnt knapp im Gehäuse: deckungsgleiche Deckel verwirren Freestyle ("edge appears twice")
    bell = [(r * 0.97, ya - 0.04 * r), (r * 0.78, yb), (r * 0.74, yb + 0.05 * length), (r * 0.92, y1), (r * 0.8, y1), (r * 0.62, yb + 0.06 * length),
            (0.0, yb + 0.02 * length)]
    s.add(revolve(bell, n), m["steel"], at=(dx, 0, dz), mirror=mirror)
    s.add(revolve([(r * 0.62, yb + 0.03 * length), (r * 0.62, yb + 0.1 * length), (r * 0.52, y1 + 0.02 * length), (0.0, y1 + 0.22 * length)], n),
          m["core"], at=(dx, 0, dz), mirror=mirror)


def turret_details(s, m, at, r=0.5, barrel=1.35, yaw=0.0, mirror=False):
    """Kleinteile zu einem `turret` mit denselben Argumenten: Barbettenring (`armor_round`) bis knapp unter das Deck,
    Entfernungsmesser-Ohren hinten an den Seiten mit dunkler Linse, Turmluke auf dem Dach, Blendenbälge an den
    Rohrwurzeln, je ein Mündungsring hinter der Mündung."""
    x, y, z = at
    s.add(post(24, r * 1.2, r * 1.2, -0.1, 0.05), m["armor_round"], at=(x, y, z), mirror=mirror)
    g = Ship("turret_details")
    for sx in (-1, 1):
        g.add(box(0.36 * r, 0.5 * r, 0.3 * r), m["accent"], at=(sx * 0.95 * r, 0.42 * r, 0.14 * r))
        g.add(box(0.22 * r, 0.06 * r, 0.16 * r), m["dark"], at=(sx * 1.0 * r, 0.16 * r, 0.14 * r))
    g.add(post(12, 0.2 * r, 0.2 * r, 0.36 * r, 0.5 * r), m["accent_round"], at=(-0.38 * r, 0.42 * r, 0))
    rb = 0.19 * r
    for dx in (-0.38 * r, 0.38 * r):
        g.add(bellows(12, rb * 1.55, -1.14 * r, -1.48 * r), m["armor_round"], at=(dx, 0, 0.04 * r))
        g.add(tube(12, rb * 1.4, rb * 1.4, -barrel + 0.22, -barrel + 0.28), m["dark_round"], at=(dx, 0, 0.04 * r))
    s.place(g, at=(x, y, z + 0.14 + 0.44 * r), rot=(0, 0, yaw), mirror=mirror)
    g.bm.free()


def engine_details(s, m, r, y0, y1, dx=0.0, dz=0.0, n=20, mirror=False):
    """Kleinteile zu einem `engine` mit denselben Argumenten: helles Gehäusesegment (`armor_round`) im vorderen
    Drittel, dunkler Düsenkranz über dem Übergang vom Gehäuse zur Glocke."""
    length = y1 - y0
    ya = y0 + 0.6 * length
    s.add(tube(n, r * 1.09, r * 1.09, y0 + 0.24 * length, y0 + 0.34 * length, dx, dz), m["armor_round"], mirror=mirror)
    s.add(tube(n, r * 1.16, r * 1.16, ya - 0.2 * r, ya + 0.1 * r, dx, dz), m["dark_round"], mirror=mirror)


def vent(s, m, mat, at, w, l, slats=3, rot=(0, 0, 0), mirror=False):
    """Lüftungsgitter: dunkle Vertiefung (w quer, l längs) in einem flachen Rahmen aus `mat` (dem Material der Fläche
    darunter) mit `slats` Stegen quer. Liegt mit der Unterseite auf `at` und steckt 0,03 darin (lokal +z nach außen):
    Boden 0,01, Stege 0,03, Rahmen 0,05 hoch. `rot` (Grad) legt es auf schräge Flächen und Flanken."""
    g = Ship("vent")
    bar = 0.07
    g.add(box(w - bar, l - bar, 0.04), m["dark"], at=(0, 0, -0.01))
    for sy in (-1, 1):
        g.add(box(w, bar, 0.08), mat, at=(0, sy * (l - bar) / 2, 0.01))
    for sx in (-1, 1):
        g.add(box(bar, l - 2 * bar, 0.08), mat, at=(sx * (w - bar) / 2, 0, 0.01))
    for k in range(slats):
        g.add(box(w - 2 * bar, 0.06, 0.06), mat, at=(0, -l / 2 + bar / 2 + (k + 1) * (l - bar) / (slats + 1), 0.0))
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def rcs(s, m, at, along, size=0.28):
    """RCS-Steuerdüsenblock an einer Rumpfecke, `at` auf Steuerbord, gespiegelt: flacher Kasten (`armor`) mit zwei
    dunklen Düsenöffnungen nach außen und einer längs (`along` 1 nach achtern, -1 nach vorn). Die Öffnungen liegen flach
    auf; abstehende Kleinstdüsen werden im Spiel mit ihren Konturen zu schwarzen Flecken."""
    x, y, z = at
    h = size / 2
    s.add(box(size, size, size * 0.6), m["armor"], at=at, mirror=True)
    for dy in (-0.07, 0.07):
        s.add(tube(8, 0.05, 0.05, -0.02, 0.02), m["dark_round"], at=(x + h, y + dy, z), rot=(0, 0, -90), mirror=True)
    s.add(tube(8, 0.05, 0.05, -0.02, 0.02), m["dark_round"], at=(x, y + along * h, z), mirror=True)


def flak(s, m, at, yaw, mirror=True):
    """Zwillings-Flak, `at` auf Steuerbord und gespiegelt: Sockel, Gehäuse in der Akzentfarbe, Blende, zwei dünne Rohre
    nach -Y, um `yaw` gedreht (positiv: nach außen)."""
    x, y, z = at
    s.add(post(8, 0.1, 0.09, -0.03, 0.05), m["dark_round"], at=at, mirror=mirror)
    g = Ship("flak")
    g.add(box(0.17, 0.2, 0.11), m["accent"], at=(0, 0.02, 0.055))
    g.add(box(0.19, 0.05, 0.07), m["dark"], at=(0, -0.09, 0.05))
    for dx in (-0.04, 0.04):
        g.add(tube(6, 0.02, 0.016, -0.42, -0.08, dx, 0.06), m["steel"])
    s.place(g, at=(x, y, z + 0.05), rot=(0, 0, yaw), mirror=mirror)
    g.bm.free()


def rail(s, m, a, b, posts, mirror=True, h=0.09):
    """Geländer von a nach b (Punkte auf der Fläche): Handlauf in Höhe h auf `posts` Pfosten."""
    (ax, ay, az), (bx, by, bz) = a, b
    s.add(beam((ax, ay, az + h), (bx, by, bz + h), 0.014, 0.014), m["dark"], mirror=mirror)
    for k in range(posts):
        t = k / (posts - 1)
        s.add(box(0.014, 0.014, h + 0.01), m["dark"], at=(ax + t * (bx - ax), ay + t * (by - ay), az + t * (bz - az) + h / 2 - 0.005),
              mirror=mirror)


def windows(s, m, points, out="x", mirror=None):
    """Fensterreihe aus Einzelfenstern (0,07 breit, 0,045 hoch, `window`), je eines an jedem Punkt (x, y, z) auf einer
    senkrechten Wand. `out` ist die Seite, nach der die Wand schaut: "x" eine Flanke (gespiegelt auf beide Seiten),
    "y" eine Rückwand, "-y" eine Stirnwand (beide ungespiegelt, außer `mirror` sagt es anders)."""
    side = out == "x"
    size = (0.03, 0.07, 0.045) if side else (0.07, 0.03, 0.045)
    dx, dy = (0.004, 0.0) if side else (0.0, 0.004 if out == "y" else -0.004)
    for x, y, z in points:
        s.add(box(*size), m["window"], at=(x + dx, y + dy, z), mirror=side if mirror is None else mirror)


def badge(s, m, at, outline, symbol, rim=0.08, dy=0.0):
    """Wappenplatte eines Flaggschiffs, liegend; `at` ist die Mitte ihrer Unterseite: Goldrand (`trim`, um `rim` breiter),
    dunkle Platte mit dem Umriss `outline`, darauf das Zeichen `symbol` in `signal`, um `dy` längs versetzt."""
    x, y, z = at
    s.add(plate(grow(outline, rim), 0.06, "xy"), m["trim"], at=(x, y, z + 0.03))
    s.add(plate(outline, 0.1, "xy"), m["dark"], at=(x, y, z + 0.05))
    s.add(plate(symbol, 0.06, "xy"), m["signal"], at=(x, y + dy, z + 0.12))


def trim_loop(s, m, outline, z, w=0.07, h=0.07, d=0.02):
    """Zierlinie (`trim`) rund um ein Deck (Umriss x, y auf Höhe z), knapp außen auf der Kante."""
    pts = [(x, y, z) for x, y in grow(outline, d)]
    s.add(sweep(pts + pts[:2], rect(w, h)), m["trim"])


def trim_line(s, m, points, w=0.07, h=0.07, mirror=True):
    """Zierlinie (`trim`) entlang einer Punktfolge (x, y, z), gespiegelt auf beide Flanken."""
    s.add(sweep(points, rect(w, h)), m["trim"], mirror=mirror)


def on_surface(tree, x, y, z=10.0):
    """Punkt und Neigung (Grad, für `rot`) der Oberfläche unter (x, y), per Strahl von oben auf `tree` (BVHTree)."""
    hit, normal, _, _ = tree.ray_cast(Vector((x, y, z)), Vector((0, 0, -1)))
    if hit is None:
        return None, None
    euler = Vector((0, 0, 1)).rotation_difference(normal).to_euler()
    return hit, tuple(math.degrees(a) for a in euler)


# ---------------------------------------------------------------- Formen


def sym(half):
    """Symmetrischer Umriss (x, y) aus der Steuerbordhälfte, von vorn nach achtern; Punkte auf x = 0 nur einmal."""
    return half + [(-x, y) for x, y in reversed(half) if x > 1e-6]


def stealth(w, up, down, top=0.36, low=0.6):
    """Tarnkappen-Querschnitt (x, z): scharfe Kimmkante bei z = 0, schmaler flacher Grat oben (Anteil `top` der Breite)
    auf Höhe `up`, flacher Boden (Anteil `low`) in Tiefe `down`. Wie vf.facet, nur mit Höhen statt Anteilen."""
    return [(w / 2, 0.0), (w * top / 2, up), (-w * top / 2, up), (-w / 2, 0.0), (-w * low / 2, -down), (w * low / 2, -down)]


def block(outline, levels, plane="xy"):
    """Facettenblock aus einem konvexen Umriss auf mehreren Lagen [(Lage, Einzug)]: 'xy' liegt (Umriss x, y, Lagen
    in z), 'yz' steht längs (Umriss y, z, Lagen in x). Ein Einzug an der ersten oder letzten Lage fast die Kante."""
    rings = []
    for level, d in levels:
        pts = grow(outline, -d) if d else outline
        rings.append([Vector((a, b, level)) if plane == "xy" else Vector((level, a, b)) for a, b in pts])
    return loft(rings)


def beam(a, b, w, h):
    """Kantige Strebe oder Armglied von a nach b, Querschnitt w (quer) × h."""
    a, b = Vector(a), Vector(b)
    d = b - a
    bm = loft([sec(0.0, rect(w, h)), sec(d.length, rect(w, h))])
    rot = Vector((0, 1, 0)).rotation_difference(d.normalized()).to_matrix().to_4x4()
    bmesh.ops.transform(bm, matrix=Matrix.Translation(a) @ rot, verts=bm.verts)
    return bm


def axle(r, length, n=12):
    """Walze entlang X um den Ursprung (Achse, Gelenk)."""
    bm = tube(n, r, r, -length / 2, length / 2)
    bmesh.ops.rotate(bm, verts=bm.verts, matrix=Matrix.Rotation(-math.pi / 2, 4, "Z"))
    return bm


def bellows(n, r, y0, y1, folds=2):
    """Blendenbalg um ein Rohr (Achse Y) von y0 nach y1: `folds` Falten zwischen r und 0,86 r."""
    return revolve([(r if k % 2 == 0 else r * 0.86, y0 + (y1 - y0) * k / (2 * folds)) for k in range(2 * folds + 1)], n)


def ring_at(rings, y, offset=0.0):
    """Querschnitt eines Lofts (Ringe wie aus `smooth`) bei Tiefe y, radial um `offset` von der Längsachse weg versetzt:
    Bänder, die dem Rumpf genau folgen (`loft([ring_at(r, y0, 0.04), ring_at(r, y1, 0.04)])`)."""
    rings = [r for r in rings if isinstance(r, list)]
    for a, b in zip(rings, rings[1:]):
        if a[0].y <= y <= b[0].y:
            t = (y - a[0].y) / (b[0].y - a[0].y)
            return [Vector((p.x, y, p.z)) + Vector((p.x, 0.0, p.z)).normalized() * offset for p in (p.lerp(q, t) for p, q in zip(a, b))]
    raise ValueError(f"y = {y} liegt außerhalb des Lofts")
