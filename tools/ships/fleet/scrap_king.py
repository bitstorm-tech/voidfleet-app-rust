"""Schrottkönig (Boss von Sektor 1), Clean Toon: der Hive-Rumpf, zusammengeflickt und geplündert.

Grundform wie die Hive (fleet/hive.py): Zwei-Kammer-Rumpf mit Taille und zwei Kragen, Brutplattform mit zehn Waben,
Sechseckturm, Sensorauge, Kiel, aber mit Schlachtschiff-Stahlblau statt Halvorsens Violett und ohne Gold.
Darauf geflickt: große aufgeschweißte Platten in Schrottgrau, Panzergrau, Dunkel und Rost, verdeckelte Waben aus Rost, offene glühen
orange. Statt der Wappenplatte sitzt auf dem Turm eine grobe Krone aus sechs schiefen Trümmerzacken. Backbord bleibt
ein Hangar-Sponson der Hive, steuerbord ist ein fremdes Modul angebaut: eine kantige Kasematte mit einem erbeuteten
Zwillingsturm. Unter dem Bug hängt eine schwere Beutekanone, achtern sitzt über den drei Düsen eine vierte, fremde
Düse auf einem Sockel. Farben aus palette.py: SCRAP_HULL und RUST des Scrap Raiders, die Lichter der Jäger.
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import engine, on_surface, ring_at, turret
from vf import Ship, box, grow, hoop, loft, ngon, plate, post, rbox, sec, smooth, superellipse, tube, wedge

HEX = math.radians(30)  # Phase eines Sechsecks mit Spitze oben bzw. vorn


def band(rings, y0, y1, grow_by, below=None):
    """Lage auf einem runden Rumpf von y0 bis y1, die seinen Querschnitten folgt: um `grow_by` aufgeweitet; mit `below`
    nur unterhalb dieser Höhe (darüber liegt die Lage im Rumpf), also eine Bauchschale statt eines Kragens."""
    ys = [y0] + [r[0].y for r in rings if y0 < r[0].y < y1] + [y1]
    sections = []
    for y in ys:
        ring = ring_at(rings, y)
        cx = sum(p.x for p in ring) / len(ring)
        cz = sum(p.z for p in ring) / len(ring)
        shape = []
        for p in ring:
            d = Vector((p.x - cx, p.z - cz))
            step = grow_by if below is None or p.z < below else -0.15
            d = d * ((d.length + step) / d.length)
            shape.append((cx + d.x, cz + d.y))
        sections.append(sec(y, shape))
    return loft(sections)


def convex_hull(points):
    """Konvexe Hülle (x, y) gegen den Uhrzeigersinn."""
    pts = sorted(set(points))

    def half(seq):
        out = []
        for p in seq:
            while len(out) >= 2 and (out[-1][0] - out[-2][0]) * (p[1] - out[-2][1]) - (out[-1][1] - out[-2][1]) * (p[0] - out[-2][0]) <= 0:
                out.pop()
            out.append(p)
        return out

    lower, upper = half(pts), half(reversed(pts))
    return lower[:-1] + upper[:-1]


def patch(w, l, cut=0.22):
    """Aufgeschweißtes Blech (w × l, 0,1 dick) mit einer schräg abgeschnittenen Ecke, damit es nicht wie eine Luke wirkt."""
    return plate([(-w / 2, -l / 2), (w / 2, -l / 2), (w / 2, l / 2 - cut), (w / 2 - cut, l / 2), (-w / 2, l / 2)], 0.1, "xy")


def crown(s, m, at, r):
    """Grobe Krone aus sechs schiefen Trümmerzacken im Kreis (Radius r) um `at`, nach außen gekippt, ungleich hoch."""
    x0, y0, z0 = at
    mats = ("rust", "steel_flat", "scrap", "rust", "dark", "steel_flat")
    for k, (h, lean) in enumerate(((1.3, 16), (0.85, 24), (1.15, 12), (0.75, 28), (1.1, 18), (0.9, 22))):
        a = 60 * k + 30
        shard = [(-0.2, 0.0), (0.2, 0.0), (0.08, h), (-0.1, h * 0.75)]
        x, y = x0 + r * math.cos(math.radians(a)), y0 + r * math.sin(math.radians(a))
        s.add(plate(shard, 0.16, "xz"), m[mats[k]], at=(x, y, z0), rot=(-lean, 0, a - 90))


def build():
    s = Ship("scrap_king")
    m = vf.materials(s.name, "battleship")
    fighter = palette.CLASSES["fighter"]
    m["scrap"] = vf.toon(f"{s.name}_scrap", palette.SCRAP_HULL, round=True)
    # Rost rund wie beim Scrap Raider: flache Bleche bleiben flach, bekommen aber nur das schmale Kantenlicht
    m["rust"] = vf.toon(f"{s.name}_rust", palette.RUST, round=True)
    m["steel_flat"] = vf.toon(f"{s.name}_steel_flat", palette.STEEL)
    m["glow"] = vf.light(f"{s.name}_glow", fighter["glow"])
    m["core"] = vf.light(f"{s.name}_core", fighter["core"])
    m["ember"] = vf.light(f"{s.name}_ember", fighter["glow"], glow=False)  # offene Waben, Buchten, Sensorauge
    # Zwei-Kammer-Rumpf wie die Hive: vordere Kammer, Taille, größere hintere Kammer
    body = lambda w, h: superellipse(w, h, 24, 3.0)  # noqa: E731
    rings = smooth([
        sec(-4.85, body(0.9, 0.6), dz=-0.15),
        sec(-4.35, body(2.0, 1.3), dz=-0.08),
        sec(-2.8, body(3.0, 1.9)),
        sec(-0.75, body(2.3, 1.5)),
        sec(1.3, body(3.7, 2.3), dz=0.05),
        sec(3.3, body(3.5, 2.2), dz=0.05),
        sec(4.45, body(2.5, 1.6)),
        sec(4.85, body(1.6, 1.05)),
    ], 2)
    part = loft(rings)
    tree = BVHTree.FromBMesh(part)
    s.add(part, m["hull_round"])
    for y in (-1.1, -0.35):
        s.add(band(rings, y - 0.12, y + 0.12, 0.07), m["accent_round"])
    # Bauchschale unter der vorderen Kammer als zweite Lage
    s.add(band(rings, -4.05, -1.55, 0.09, below=0.0), m["armor_round"])
    # Brutplattform auf der hinteren Kammer: zehn Waben, offene glühen orange, drei sind mit Rost verdeckelt
    pitch, rc = 0.92, (0.92 - 0.16) / math.sqrt(3)
    yc, ztop = 2.25, 1.3
    cells = [(0.0, yc + (k - 1.5) * pitch) for k in range(4)]
    cells += [(sx * pitch * math.sqrt(3) / 2, yc + (k - 1) * pitch) for sx in (1, -1) for k in range(3)]
    corners = [(x + rc * math.cos(a), y + rc * math.sin(a)) for x, y in cells for a in (math.radians(60 * i) for i in range(6))]
    deck = grow(convex_hull(corners), 0.16)
    s.add(plate(deck, 0.7, "xy"), m["dark"], at=(0, 0, ztop - 0.35))
    for i, (x, y) in enumerate(cells):
        if i in (2, 4, 6, 9):
            s.add(loft([sec(0.0, ngon(6, rc)), sec(0.12, ngon(6, rc * 0.9)), sec(0.22, ngon(6, rc * 0.5))]), m["rust"],
                  at=(x, y, ztop - 0.02), rot=(90, 0, 0))
        else:
            s.add(post(6, rc, rc, 0.0, 0.05), m["ember"], at=(x, y, ztop - 0.01))
    # Sechseckturm, darauf statt Mast und Wappen die Trümmerkrone
    ty = -2.1
    hit, _ = on_surface(tree, 0, ty)
    z = hit.z - 0.1
    s.add(post(6, 0.85, 0.81, 0.0, 0.62), m["hull"], at=(0, ty, z))
    z += 0.62
    s.add(post(6, 0.62, 0.58, 0.0, 0.56), m["armor"], at=(0, ty, z))
    s.add(hoop(6, 0.55, 0.62, 0.11), m["window"], at=(0, ty, z + 0.36))
    s.add(post(6, 0.66, 0.66, 0.0, 0.14), m["dark"], at=(0, ty, z + 0.56))
    crown(s, m, (0, ty, z + 0.62), 0.55)
    # Aufgeschweißte Flicken auf dem Rumpf (x, y, Breite, Länge, Material, Drehung)
    for x, y, w, l, mat, yaw in ((0.45, -3.75, 1.0, 0.9, "rust", 8), (-0.6, -3.2, 0.9, 1.2, "scrap", -12),
                                 (0.55, -2.75, 0.8, 0.6, "dark", 20), (0.95, -0.8, 0.8, 1.1, "scrap", 5),
                                 (-0.95, 0.15, 0.9, 1.3, "rust", -6), (1.05, 4.0, 1.0, 0.8, "armor", 14),
                                 (-0.9, 3.95, 0.9, 0.9, "rust", -10)):
        hit, rot = on_surface(tree, x, y)
        s.add(patch(w, l), m[mat], at=tuple(hit), rot=(rot[0], rot[1], yaw))
    # Sensorauge im Bug: dunkler Sechseckrahmen, Auge in Glut
    s.add(tube(6, 0.44, 0.44, -4.97, -4.7, dz=-0.15, phase=HEX), m["dark"])
    s.add(tube(6, 0.3, 0.3, -5.0, -4.9, dz=-0.15, phase=HEX), m["ember"])
    # Backbord: Hangar-Sponson der Hive mit drei Buchten
    sx, sz, rx, rz = 2.3, -0.32, 0.6, 0.75
    pod = [sec(-0.95, ngon(6, 0.26, 0.3, HEX)), sec(-0.35, ngon(6, rx, rz, HEX)), sec(3.5, ngon(6, rx, rz, HEX)),
           sec(4.15, ngon(6, 0.4, 0.5, HEX))]
    s.add(loft(pod), m["accent"], at=(-sx, 0, sz))
    face = sx + rx * math.cos(HEX)
    for y in (0.55, 1.6, 2.65):
        s.add(hoop(6, 0.2, 0.32, 0.12), m["dark"], at=(-face - 0.01, y, sz), rot=(0, 90, 0))
        s.add(tube(6, 0.22, 0.22, -0.06, 0.03, phase=HEX), m["ember"], at=(-face, y, sz), rot=(0, 0, 90))
    # Steuerbord: fremdes Modul, eine kantige Kasematte mit Lichtband und erbeutetem Zwillingsturm auf einem Sockel
    s.add(rbox(1.2, 3.6, 1.1, 0.08, 1), m["armor"], at=(2.15, 1.55, -0.25))
    s.add(box(0.04, 2.4, 0.09), m["glow"], at=(2.76, 1.55, -0.2))
    s.add(box(0.9, 0.9, 0.3), m["dark"], at=(2.15, 1.2, 0.45))
    turret(s, m, (2.15, 1.2, 0.6), r=0.46, barrel=1.3)
    # Beutekanone unter dem Bug: Gehäuse, Lauf mit Mündungsbremse
    s.add(rbox(0.8, 1.6, 0.7, 0.06, 1), m["dark"], at=(0.0, -2.9, -1.0))
    s.add(tube(16, 0.2, 0.18, -5.0, -3.6), m["steel"], at=(0, 0, -1.0))
    s.add(tube(16, 0.3, 0.3, -5.2, -4.85), m["dark_round"], at=(0, 0, -1.0))
    # Kiel unter dem Rumpf
    s.add(plate([(-2.2, -0.55), (3.6, -0.65), (3.35, -1.35), (-1.6, -1.2)], 0.18, "yz"), m["dark"])
    # Drei Düsen wie die Hive, darüber eine vierte, fremde Düse auf einem schiefen Sockel
    engine(s, m, 0.5, 4.5, 5.45, dz=0.05)
    engine(s, m, 0.34, 4.35, 5.15, dx=0.8, dz=-0.45, mirror=True)
    s.add(wedge(0.8, 1.2, 0.5), m["rust"], at=(-0.75, 4.0, 1.05), rot=(0, -8, 0))
    engine(s, m, 0.3, 4.3, 5.1, dx=-0.78, dz=1.15, n=16)
    return s.lift(0.25).finish()
