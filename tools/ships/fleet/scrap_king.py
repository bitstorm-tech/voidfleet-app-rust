"""Schrottkönig (Boss von Sektor 1), Fassung für die Nahaufnahme nach docs/stilregeln.md (gefaste Sekundärformen,
glatte runde Teile, Kleinteile und Feinteile): der Hive-Rumpf, zusammengeflickt und geplündert.

Primärformen wie die Hive (fleet/hive.py): Zwei-Kammer-Rumpf mit Taille und zwei Kragen, Brutplattform mit zehn Waben,
Sechseckturm, Sensorauge, Kiel, aber mit Schlachtschiff-Stahlblau statt Halvorsens Violett und ohne Gold. Offene Waben
glühen orange, vier sind mit Rost verdeckelt. Statt der Wappenplatte sitzt auf dem Turm eine grobe Krone aus sechs
schiefen Trümmerzacken. Backbord bleibt ein Hangar-Sponson der Hive, steuerbord ist ein fremdes Modul angebaut: eine
kantige Kasematte mit Lichtband und einem erbeuteten Zwillingsturm. Unter dem Bug hängt eine schwere Beutekanone, achtern
sitzt über den drei Düsen eine vierte, fremde Düse auf einem Sockel. Farben aus palette.py: SCRAP_HULL und RUST des
Scrap Raiders, die Lichter der Jäger.
Detailzonen mit Zweck:
- Bug: Bugkappe aus zwei aufgeschweißten Lagen um das Sensorauge; Bergungsgreifer an der breitesten Stelle der vorderen
  Kammer, nach vorn gefaltet (Schultergehäuse, Oberarm mit Hydraulikzylinder, Gelenkwalzen, Unterarm, Zange mit zwei
  Backen), mit denen er Wracks packt; Beutekanone mit Blendenbalg, Kühlmantel und zweiteiliger Mündungsbremse
- Flicken: aufgeschweißte Platten in zwei Lagen wie Schindeln, die dem Rumpf folgen (`shell`): beidseits der vorderen
  Kammer, steuerbord über dem Modul, backbord als Streifen über dem Sponson
- Turm: Kronreif aus Schrott, in dem die Trümmerzacken stecken
- Steuerbord-Modul: Turmdetails (`parts.turret_details`), Schrottdeckel mit zwei Schrottcontainern, notdürftig verlegte
  dicke Leitung aus der vorderen Kammer an der Taille entlang ins Modul, mit Halter und Schellen
- Backbord-Sponson: Hangarmaul nach vorn für die Plünderer-Staffeln (Rahmen, dunkler Schlund, Startlicht)
- Heck: Maschinenlüfter hinter der Brutplattform, Treibstoffleitung zur fremden Düse, Gehäusesegment und Düsenkranz an
  allen vier Düsen (`parts.engine_details`)
Ruhig bleiben der Rücken der vorderen Kammer, die Taille zwischen den Kragen, die Außenwand des Moduls, die Flanken des
Sponsons und die Brutplattform. Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`), seit der dreckige Look keine Konturringe mehr zeichnet: am Turm Einzelfenster in der unteren
Stufe, Geländer vorn am Absatz, zwei ungleiche Peitschenantennen und eine Schüssel achtern; am Steuerbord-Modul zwei
Flak, Fenster an Stirn und Außenwand, Geländer, Leiter, Luke, Fässer und ein lose verlegtes Kabel vom Turm an die Wand
der Brutplattform; Geländer an beiden Längskanten der Brutplattform; Flacheisen über den Schweißnähten der Kammern;
Kühlrippen an Beutekanone und Düsen, Staurohre am Sensorauge, Stützstreben am Sponson und an der fremden Düse,
Peitschenantenne am Hangarmaul, Positionslichter am Heck von Sponson und Modul.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme), hier keine ordentlichen Treppen und sauberen Warnbänder,
sondern was ein Schrottschiff zusammenhält: Heftnähte rund um jeden Flicken und grobe Schrauben in seinen Ecken,
Flanschschrauben in den Kragen, zwei ungleiche Leitungen über den Rücken vom Turm zur Brutplattform, eine schief
aufgeschweißte Bude mit Ofenrohr vor dem Turm, Stützstreben am Turm und hinter jeder Zacke der Krone, Steigeisen mit
Lücke, Ketten (zwischen zwei Zacken, über einem Container, vom Sponson zum Rumpf, an der Zange, an der fremden Düse),
verriegelte Wabendeckel, eine Kreuzstrebe an der Brutplattform, ein zugeschweißter Lukendeckel, zwei schiefe Bleche und
eine Leitung auf dem Sponson, Schrauben in den Rahmen von Buchten, Hangarmaul und Sensorauge, Achsmuttern und
Hydraulikschlauch am Greifer, Bänder um die Beutekanone, Stellstreben an den Düsenglocken (an der fremden fehlt eine).
Beschriftung nur als Rest: an der Außenwand des Moduls die Nummer des Vorbesitzers, durchgestrichen und eine Ecke unter
einem Rostblech, am Turm die Strichliste der Beute.
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import axle, beam, bellows, bolts, engine, engine_details, flak, number, on_surface, rail, ring_at, turret, turret_details, vent, windows
from vf import (Ship, box, chamfer, dish, grow, hoop, loft, ngon, pipe, plate, post, rbox, sec, segment, smooth, sphere, superellipse, sweep, tube,
                wedge)

HEX = math.radians(30)  # Phase eines Sechsecks mit Spitze oben bzw. vorn
ROUND = 40  # Segmente der großen runden Teile (Rumpf, Hauptdüse, Beutekanone; den Turm rundet `parts.turret`)
JET = 24  # Segmente der kleinen Düsen, Walzen und Zylinder
# Flicken: aufgeschweißte Platten in Lagen (Stützstellen für `shell`, Abstand vom Rumpf, Material, Punkte je Bogen). Die
# zweite Lage (Abstand 0,12) überlappt jeweils eine Kante der ersten wie eine Schindel und reicht darüber hinaus
PATCHES = (
    # Bugkappe um das Sensorauge, darüber ein Rostblech steuerbord
    ([(-4.72, 20, 160), (-4.4, 12, 168), (-4.02, 24, 150)], 0.06, "scrap", 10),
    ([(-4.58, 30, 96), (-4.15, 18, 84), (-3.85, 26, 70)], 0.12, "rust", 6),
    # vordere Kammer steuerbord: große Schrottplatte, über ihre obere Kante ein Panzerblech
    ([(-3.7, 14, 50), (-3.3, 12, 58), (-2.98, 18, 48)], 0.06, "scrap", 6),
    ([(-3.45, 40, 84), (-3.0, 44, 92), (-2.9, 52, 80)], 0.12, "armor_round", 6),
    # vordere Kammer backbord
    ([(-3.55, 122, 166), (-2.75, 130, 174)], 0.06, "rust", 6),
    ([(-3.2, 156, 196), (-2.55, 162, 190)], 0.12, "scrap", 6),
    # hintere Kammer steuerbord über dem Modul, backbord ein langer Streifen über dem Sponson
    ([(0.35, 14, 42), (1.2, 10, 48), (2.0, 18, 40)], 0.06, "scrap", 6),
    ([(1.0, 30, 58), (1.75, 34, 62), (2.1, 40, 56)], 0.12, "rust", 6),
    ([(0.7, 146, 166), (1.8, 140, 168), (2.9, 148, 164)], 0.06, "scrap", 6),
    ([(2.35, 132, 156), (2.85, 128, 160), (3.25, 136, 152)], 0.12, "rust", 6),
)


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


def radius(ring, c, d):
    """Abstand von der Mitte c (x, z) eines Querschnitts bis zu seinem Umriss in Richtung d (Einheitsvektor x, z)."""
    pts = [(p.x, p.z) for p in ring]
    best = 0.0
    for (ax, az), (bx, bz) in zip(pts, pts[1:] + pts[:1]):
        ex, ez = bx - ax, bz - az
        den = d[0] * ez - d[1] * ex
        if abs(den) < 1e-12:
            continue
        wx, wz = ax - c[0], az - c[1]
        t, u = (wx * ez - wz * ex) / den, (wx * d[1] - wz * d[0]) / den
        if t > 0 and -1e-9 <= u <= 1 + 1e-9:
            best = max(best, t)
    return best


def shell(rings, keys, lift=0.06, depth=0.12, k=6):
    """Aufgeschweißte Platte, die dem runden Rumpf folgt: an den Stützstellen keys = [(y, a0, a1), …] der Bogen von a0
    bis a1 (Grad um die Mitte des Querschnitts, 0 Steuerbord, 90 oben, 180 Backbord), dazwischen linear; außen `lift`
    über dem Rumpf, innen `depth` darin. Ungleiche Bögen geben schiefe Ränder, eine zweite Lage hat den doppelten `lift`."""
    stations = []
    for (y0, a0, b0), (y1, a1, b1) in zip(keys, keys[1:]):
        steps = max(1, math.ceil((y1 - y0) / 0.2))
        stations += [(y0 + (y1 - y0) * j / steps, a0 + (a1 - a0) * j / steps, b0 + (b1 - b0) * j / steps) for j in range(steps)]
    stations.append(keys[-1])
    sections = []
    for y, a0, a1 in stations:
        ring = ring_at(rings, y)
        c = (sum(p.x for p in ring) / len(ring), sum(p.z for p in ring) / len(ring))
        outer, inner = [], []
        for j in range(k + 1):
            a = math.radians(a0 + (a1 - a0) * j / k)
            d = (math.cos(a), math.sin(a))
            r = radius(ring, c, d)
            outer.append((c[0] + d[0] * (r + lift), c[1] + d[1] * (r + lift)))
            inner.append((c[0] + d[0] * (r - depth), c[1] + d[1] * (r - depth)))
        sections.append(sec(y, outer + inner[::-1]))
    return loft(sections)


def hex_frame(rx, rz, w, y0, y1):
    """Sechseckiger Rahmen um die Y-Achse (Spitze oben) von y0 bis y1: außen rx × rz, Rahmenbreite w."""
    outer, inner = ngon(6, rx, rz, HEX), ngon(6, rx - w, rz - w, HEX)
    # Der letzte Ring wiederholt den ersten; loft verschweißt die doppelten Punkte, der Rahmen ist also geschlossen
    return loft([sec(y0, inner), sec(y0, outer), sec(y1, outer), sec(y1, inner), sec(y0, inner)], cap_start=False, cap_end=False)


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


def crown(s, m, at, r):
    """Grobe Krone aus sechs schiefen Trümmerzacken im Kreis (Radius r) um `at`, nach außen gekippt, ungleich hoch, in
    einem Kronreif aus Schrott."""
    x0, y0, z0 = at
    s.add(hoop(6, r - 0.14, r + 0.16, 0.16), m["scrap"], at=(x0, y0, z0 + 0.06))
    mats = ("rust", "steel_flat", "scrap", "rust", "dark", "steel_flat")
    for k, (h, lean) in enumerate(((1.4, 16), (0.95, 24), (1.25, 12), (0.85, 28), (1.2, 18), (1.0, 22))):
        a = 60 * k + 30
        shard = [(-0.22, 0.0), (0.22, 0.0), (0.09, h), (-0.11, h * 0.75)]
        x, y = x0 + r * math.cos(math.radians(a)), y0 + r * math.sin(math.radians(a))
        s.add(chamfer(plate(shard, 0.17, "xz"), 0.02), m[mats[k]], at=(x, y, z0), rot=(-lean, 0, a - 90))


def grapple(s, m):
    """Bergungsgreifer an der breitesten Stelle der vorderen Kammer, nach vorn gefaltet (gespiegelt): Schultergehäuse,
    Oberarm mit Hydraulikzylinder, Gelenke als Walzen, Unterarm nach innen, Zange mit zwei Backen neben dem Bug."""
    shoulder, elbow, wrist = Vector((1.8, -2.8, 0.0)), Vector((1.86, -3.92, -0.06)), Vector((1.3, -4.5, -0.08))
    s.add(rbox(0.5, 0.8, 0.5, 0.06, 1), m["dark"], at=(1.5, -2.8, 0.0), mirror=True)
    s.add(axle(0.25, 0.16, JET), m["armor_round"], at=shoulder, mirror=True)
    s.add(chamfer(beam(shoulder, elbow, 0.24, 0.32), 0.03), m["armor"], mirror=True)
    # Hydraulikzylinder auf dem Oberarm: Zylinder mit Kopf, der Neigung des Arms folgend
    top = Vector((0, 0, 0.25))
    s.add(segment(shoulder + top + Vector((0, -0.12, 0)), elbow.lerp(shoulder, 0.2) + top, 0.1, JET), m["steel"], mirror=True)
    s.add(segment(shoulder + top + Vector((0, -0.1, 0)), shoulder.lerp(elbow, 0.3) + top, 0.13, JET), m["dark_round"], mirror=True)
    s.add(axle(0.21, 0.36, JET), m["armor_round"], at=elbow, mirror=True)
    s.add(chamfer(beam(elbow, wrist, 0.22, 0.28), 0.03), m["armor"], mirror=True)
    s.add(axle(0.17, 0.3, JET), m["dark_round"], at=wrist, mirror=True)
    # Zwei Backen, die obere nach oben, die untere (um Y gewendet) nach unten geöffnet
    jaw = [(0.08, 0.06), (-0.16, 0.2), (-0.46, 0.18), (-0.64, 0.0), (-0.48, 0.02), (-0.26, 0.07), (0.08, -0.08)]
    for pitch, flip in ((-12, 0), (12, 180)):
        g = Ship("jaw")
        g.add(chamfer(plate(jaw, 0.18, "yz"), 0.02), m["steel_flat"], rot=(0, flip, 0))
        s.place(g, at=wrist, rot=(pitch, 0, 0), mirror=True)
        g.bm.free()


def container(s, m, at, size, mat, yaw=0.0):
    """Schrottcontainer: Kasten aus `mat` mit dunklen Stirnrahmen, `at` ist die Mitte der Unterseite."""
    w, l, h = size
    g = Ship("container")
    g.add(chamfer(box(w, l, h), 0.02), mat, at=(0, 0, h / 2))
    for sy in (-1, 1):
        g.add(box(w + 0.05, 0.08, h + 0.05), m["dark"], at=(0, sy * (l / 2 - 0.03), h / 2))
    s.place(g, at=at, rot=(0, 0, yaw))
    g.bm.free()


def details(s, m, rings, tree, ty, ledge):
    """Kleinteile (Modulkommentar); `rings` und `tree` sind der Rumpf, `ty` die Lage des Turms, `ledge` die Höhe des
    Absatzes zwischen seinen beiden Stufen."""

    def on(x, y, up):
        """Punkt um `up` über dem Rumpf bei (x, y)."""
        hit, _ = on_surface(tree, x, y)
        return (x, y, hit.z + up)

    def corner(a, r=0.76):
        """Punkt auf dem Turmabsatz bei Winkel a (Grad, 0 Steuerbord, 90 achtern) im Abstand r von der Turmachse."""
        return Vector((r * math.cos(math.radians(a)), ty + r * math.sin(math.radians(a)), ledge))

    # ---------------------------------------------------------------- Turm: Fenster, Geländer vorn, Antennen und Schüssel achtern
    for a in range(30, 360, 60):  # je Wand der unteren Stufe drei Fenster
        n = Vector((math.cos(math.radians(a)), math.sin(math.radians(a)), 0.0))
        for o in (-0.2, 0.0, 0.2):
            s.add(box(0.07, 0.03, 0.045), m["window"], at=Vector((0, ty, ledge - 0.28)) + n * 0.721 + Vector((-n.y, n.x, 0.0)) * o,
                  rot=(0, 0, a - 90))
    rail(s, m, corner(240), corner(300), 4, mirror=False)
    rail(s, m, corner(300).lerp(corner(0), 0.1), corner(0), 3)
    s.add(post(6, 0.014, 0.006, ledge - 0.02, ledge + 1.15, 0.3, ty + 0.65), m["dark_round"])
    s.add(segment((-0.3, ty + 0.65, ledge - 0.02), (-0.44, ty + 0.92, ledge + 0.75), 0.012, 6), m["dark_round"])
    s.add(post(8, 0.03, 0.025, ledge - 0.02, ledge + 0.16, -0.57, ty + 0.33), m["dark_round"])
    s.add(dish(0.15, 0.055, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.57, ty + 0.33, ledge + 0.15), rot=(-40, 0, -150))

    # ---------------------------------------------------------------- Steuerbord-Modul: Flak, Fenster, Geländer, Leiter, Luke, Fässer
    roof, lid = 0.3, 0.37  # Dach des Moduls, Oberseite des Schrottdeckels
    flak(s, m, (2.55, -0.05, roof), 30, mirror=False)
    flak(s, m, (1.74, -0.05, roof), -25, mirror=False)
    windows(s, m, [(2.75, -0.02 + 0.12 * k, 0.1) for k in range(5)], mirror=False)
    windows(s, m, [(2.25 + 0.14 * k, -0.25, -0.05) for k in range(3)], out="-y")
    rail(s, m, (2.64, 0.2, roof), (2.64, 0.68, roof), 3, mirror=False)
    rail(s, m, (2.72, 1.8, lid), (2.72, 3.25, lid), 5, mirror=False)
    for dy in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.9), m["dark"], at=(2.757, 3.1 + dy, -0.25))
    for k in range(9):
        s.add(box(0.014, 0.1, 0.014), m["dark"], at=(2.757, 3.1, -0.6 + 0.1 * k))
    s.add(box(0.36, 0.32, 0.04), m["hull"], at=(2.2, 0.3, roof + 0.012))
    s.add(post(16, 0.09, 0.09, lid - 0.01, lid + 0.23, 1.95, 2.25), m["rust"])
    s.add(post(16, 0.09, 0.09, lid - 0.01, lid + 0.19, 1.92, 2.47), m["dark_round"])
    # Kabel vom Turmsockel über die Flicken hinweg an die Wand der Brutplattform, dort entlang und hinein
    cable = [(1.82, 1.5, 0.52), on(1.72, 1.56, 0.15), on(1.56, 1.66, 0.15), (1.43, 1.8, 1.15), (1.43, 2.9, 1.15), (1.3, 2.96, 1.15)]
    s.add(pipe(cable, 0.035, 6), m["dark_round"])
    for y in (2.1, 2.6):
        s.add(box(0.07, 0.06, 0.11), m["armor"], at=(1.425, y, 1.15))

    # ---------------------------------------------------------------- Brutplattform: Geländer an beiden Längskanten
    rail(s, m, (1.36, 1.4, 1.3), (1.36, 3.1, 1.3), 6)

    # ---------------------------------------------------------------- Flacheisen über den Schweißnähten der Kammern
    for y, a0, a1 in ((-1.95, 20, 58), (-1.75, 125, 168), (0.12, 16, 74), (3.55, 20, 56)):
        s.add(shell(rings, [(y, a0, a1), (y + 0.07, a0, a1)], lift=0.035, depth=0.05), m["steel_flat"])

    # ---------------------------------------------------------------- Bug: Kühlrippen an der Beutekanone, Staurohre am Sensorauge
    for y in (-4.44, -4.32, -4.2):
        s.add(tube(ROUND, 0.305, 0.305, y, y + 0.035), m["steel"], at=(0, 0, -1.0))
    s.add(segment((0.25, -4.9, 0.0), (0.3, -5.6, 0.06), 0.014, 6), m["steel"])
    s.add(segment((-0.25, -4.9, -0.3), (-0.31, -5.35, -0.36), 0.012, 6), m["steel"])

    # ---------------------------------------------------------------- Backbord-Sponson: Stützstreben zur Taille, Antenne, Hecklicht
    s.add(beam((-1.0, -0.2, 0.15), (-1.9, -0.35, -0.1), 0.09, 0.09), m["steel_flat"])
    s.add(beam((-1.0, 0.15, -0.3), (-1.9, 0.0, -0.5), 0.09, 0.09), m["steel_flat"])
    s.add(segment((-2.3, -0.64, 0.46), (-2.42, -0.95, 1.15), 0.012, 6), m["dark_round"])
    s.add(sphere(0.04, 1), m["glow"], at=(-2.3, 4.15, -0.32))
    s.add(sphere(0.04, 1), m["glow"], at=(2.72, 3.31, lid))

    # ---------------------------------------------------------------- Heck: Kühlrippen an den Düsen, Streben der fremden Düse
    for y in (4.85, 4.89, 4.93):
        s.add(tube(ROUND, 0.545, 0.545, y, y + 0.02, 0.0, 0.05), m["steel"])
    for y in (4.65, 4.7):
        s.add(tube(JET, 0.37, 0.37, y, y + 0.02, 0.8, -0.45), m["steel"], mirror=True)
    s.add(beam((-0.35, 4.55, 0.6), (-0.7, 4.75, 1.0), 0.07, 0.07), m["steel_flat"])
    s.add(beam((-1.0, 4.4, 0.5), (-0.92, 4.75, 0.98), 0.07, 0.07), m["steel_flat"])


def skin(tree, rings, y, a, lift=0.0):
    """Punkt auf dem Rumpf bei y in Richtung a (Grad um die Mitte des Querschnitts wie in `shell`), um `lift` nach außen,
    und die Drehung (`rot`), die ein Teil dort nach außen richtet."""
    ring = ring_at(rings, y)
    c = (sum(p.x for p in ring) / len(ring), sum(p.z for p in ring) / len(ring))
    d = (math.cos(math.radians(a)), math.sin(math.radians(a)))
    r = radius(ring, c, d)
    _, normal, _, _ = tree.find_nearest(Vector((c[0] + d[0] * r, y, c[1] + d[1] * r)))
    euler = Vector((0, 0, 1)).rotation_difference(normal).to_euler()
    return (c[0] + d[0] * (r + lift), y, c[1] + d[1] * (r + lift)), tuple(math.degrees(v) for v in euler)


def edge(keys, y, side):
    """Winkel der Längskante eines Flickens bei y (side 1 die untere, 2 die obere), linear zwischen den Stützstellen."""
    for k0, k1 in zip(keys, keys[1:]):
        if y <= k1[0]:
            return k0[side] + (y - k0[0]) / (k1[0] - k0[0]) * (k1[side] - k0[side])
    return keys[-1][side]


def welds(s, mat, rings, keys, lift):
    """Heftnähte rund um einen Flicken (Stützstellen wie für `shell`): kurze Schweißraupen mit Lücken über seinen
    Längskanten und über den Bögen vorn und achtern."""
    y0, y1 = keys[0][0], keys[-1][0]
    n = max(1, round((y1 - y0) / 0.24))
    for j in range(n):
        ya, yb = y0 + (y1 - y0) * (j + 0.2) / n, y0 + (y1 - y0) * (j + 0.75) / n
        for side in (1, 2):
            a, b = edge(keys, ya, side), edge(keys, yb, side)
            s.add(shell(rings, [(ya, a - 1.2, a + 1.2), (yb, b - 1.2, b + 1.2)], lift=lift + 0.014, depth=0.02, k=1), mat)
    for y, a0, a1 in (keys[0], keys[-1]):
        n = max(1, round((a1 - a0) / 11))
        for j in range(n):
            a, b = a0 + (a1 - a0) * (j + 0.2) / n, a0 + (a1 - a0) * (j + 0.75) / n
            s.add(shell(rings, [(y - 0.022, a, b), (y + 0.022, a, b)], lift=lift + 0.014, depth=0.02, k=2), mat)


def chain(s, mat, a, b, sag=0.0, link=0.055):
    """Kette von a nach b, in der Mitte um `sag` durchhängend: flache Glieder, abwechselnd hochkant und liegend."""
    a, b = Vector(a), Vector(b)
    n = max(2, round((b - a).length / link))
    pts = [a.lerp(b, j / n) - Vector((0, 0, 4 * sag * (j / n) * (1 - j / n))) for j in range(n + 1)]
    for j, (p, q) in enumerate(zip(pts, pts[1:])):
        d = (q - p) * 0.2
        s.add(beam(p - d, q + d, *((0.03, 0.011) if j % 2 else (0.011, 0.03))), mat)


def fine(s, m, rings, tree, ty, ledge):
    """Feinteile und Beschriftung (Modulkommentar); Argumente wie `details`."""
    cos, sin = (lambda a: math.cos(math.radians(a))), (lambda a: math.sin(math.radians(a)))

    def on(x, y, up):
        """Punkt um `up` über dem Rumpf bei (x, y)."""
        hit, _ = on_surface(tree, x, y)
        return (x, y, hit.z + up)

    # ---------------------------------------------------------------- Flicken: Heftnähte und grobe Schrauben
    # Jeder Flicken ist rundum geheftet und in den Ecken verschraubt
    for keys, lift, _, _ in PATCHES:
        welds(s, m["steel_flat"], rings, keys, lift)
        for (y, a0, a1), dy in ((keys[0], 0.08), (keys[-1], -0.08)):
            for a in (a0 + 5, a1 - 5):
                at, rot = skin(tree, rings, y + dy, a, lift)
                bolts(s, m["steel"], [at], rot, r=0.018)

    # ---------------------------------------------------------------- Kragen: Flanschschrauben
    for y in (-1.1, -0.35):
        for a in range(5, 180, 15):
            at, rot = skin(tree, rings, y, a, 0.07)
            bolts(s, m["steel"], [at], rot, r=0.018)

    # ---------------------------------------------------------------- Rücken: Leitungen vom Turm zur Brutplattform
    # Zwei ungleiche Leitungen, notdürftig über beide Kragen gelegt und dort mit einem Bügel gehalten
    base = ledge - 0.62
    for x, r, mat in ((0.1, 0.03, "steel"), (-0.06, 0.022, "dark_round")):
        run = [(x, ty + 0.7, base + 0.22), on(x, -1.3, r), on(x, -1.1, 0.07 + r), on(x, -0.72, r), on(x, -0.35, 0.07 + r), on(x, 0.02, r),
               (x, 0.32, on(x, 0.02, r)[2] + 0.08)]
        s.add(pipe(run, r, 6), m[mat])
    for y in (-1.1, -0.35):
        s.add(box(0.28, 0.06, 0.05), m["armor"], at=on(0.02, y, 0.1))

    # ---------------------------------------------------------------- Rücken vorn: schiefe Bude
    # Zwischen den Flicken vor dem Turm eine schief aufgeschweißte Bude: Kasten mit überstehendem Rostdach, Tür nach
    # achtern, Ofenrohr und einer Stütze an der tiefen Seite
    g = Ship("hut")
    g.add(chamfer(box(0.3, 0.4, 0.26), 0.02), m["armor"], at=(0, 0, 0.11))
    g.add(box(0.4, 0.5, 0.025), m["rust"], at=(0, 0.01, 0.25), rot=(0, 8, 0))
    g.add(box(0.11, 0.012, 0.17), m["dark"], at=(-0.04, 0.202, 0.095))
    g.add(post(6, 0.022, 0.018, 0.2, 0.47), m["steel"], at=(0.08, -0.1, 0), rot=(5, -7, 0))
    g.add(beam((-0.2, -0.05, -0.02), (-0.14, -0.05, 0.16), 0.03, 0.03), m["steel_flat"])
    s.place(g, at=on(-0.28, -3.38, -0.035), rot=(-3, -6, 12))
    g.bm.free()

    # ---------------------------------------------------------------- Turm: Stützstreben, Steigeisen, Flicken, Strichliste
    # Vier Streben vom Rumpf an die Ecken der unteren Stufe; achtern Steigeisen, eines fehlt, eines hängt schief; an der
    # Wand backbord vorn ein angeschraubtes Rostblech, steuerbord vorn die Strichliste der Beute
    for a in (0, 60):
        foot = on(1.3 * cos(a), ty + 1.3 * sin(a), -0.03)
        s.add(beam(foot, (0.82 * cos(a), ty + 0.82 * sin(a), base + 0.42), 0.07, 0.07), m["steel_flat"], mirror=True)
    for j, tilt in ((0, 0), (1, 0), (2, 0), (4, 0), (5, 16), (6, 0)):
        h = 0.1 + 0.07 * j
        s.add(box(0.09, 0.03, 0.012), m["steel_flat"], at=(0.1, ty + (0.85 - 0.04 * h / 0.62) * cos(30) + 0.008, base + h), rot=(0, tilt, 0))
    for a, parts in ((210, "patch"), (330, "tally")):
        g = Ship(parts)
        if parts == "patch":
            g.add(plate([(-0.2, -0.12), (0.17, -0.1), (0.19, 0.12), (-0.18, 0.1)], 0.03, "xz"), m["rust"])
            bolts(g, m["steel"], [(-0.16, 0.015, -0.08), (0.14, 0.015, -0.06), (0.15, 0.015, 0.08), (-0.14, 0.015, 0.06)], "y", r=0.016)
        else:
            for dx in (-0.2, -0.16, -0.12, -0.08, 0.06, 0.1, 0.14):
                g.add(box(0.016, 0.008, 0.1), m["paint"], at=(dx, 0.002, 0.0))
            g.add(box(0.2, 0.008, 0.016), m["paint"], at=(-0.14, 0.004, 0.0), rot=(0, 28, 0))
        s.place(g, at=(0.728 * cos(a), ty + 0.728 * sin(a), base + 0.16), rot=(0, 0, a - 90))
        g.bm.free()

    # ---------------------------------------------------------------- Krone: Schrauben im Reif, Stützen, Kette
    # Jede Zacke ist von innen abgestützt; zwischen zwei Zacken an Backbord hängt eine Kette
    z0 = ledge + 0.56 + 0.06
    bolts(s, m["steel"], [(0.6 * cos(a), ty + 0.6 * sin(a), z0 + 0.14) for a in range(0, 360, 60)], r=0.018)
    tips = []
    for k, (h, lean) in enumerate(((1.4, 16), (0.95, 24), (1.25, 12), (0.85, 28), (1.2, 18), (1.0, 22))):
        a = 60 * k + 30
        out = 0.55 + 0.5 * h * sin(lean) - 0.07
        s.add(beam((0.26 * cos(a), ty + 0.26 * sin(a), z0 + 0.12), (out * cos(a), ty + out * sin(a), z0 + 0.5 * h * cos(lean)), 0.035, 0.035),
              m["steel_flat"])
        out = 0.55 + 0.7 * h * sin(lean)
        tips.append((out * cos(a), ty + out * sin(a), z0 + 0.7 * h * cos(lean)))
    chain(s, m["steel_flat"], tips[2], tips[3], sag=0.14)

    # ---------------------------------------------------------------- Brutplattform: verriegelte Deckel, Kreuzstrebe
    # Schrauben um die vier Rostdeckel, zwei davon mit einem Riegel quer; an der Backbordwand eine Kreuzstrebe
    pitch, rc, yc, ztop = 0.92, (0.92 - 0.16) / math.sqrt(3), 2.25, 1.3
    lids = [(0.0, yc + 0.5 * pitch), (pitch * math.sqrt(3) / 2, yc - pitch), (pitch * math.sqrt(3) / 2, yc + pitch), (-pitch * math.sqrt(3) / 2, yc + pitch)]
    for k, (x, y) in enumerate(lids):
        bolts(s, m["steel"], [(x + (rc + 0.045) * cos(a), y + (rc + 0.045) * sin(a), ztop) for a in range(0, 360, 60)], r=0.016)
        if k % 2 == 0:
            s.add(box(1.9 * rc, 0.06, 0.03), m["steel_flat"], at=(x, y, ztop + 0.115), rot=(0, 0, 25 + 100 * k))
    s.add(beam((-1.405, 1.45, 1.03), (-1.405, 2.35, 1.26), 0.02, 0.05), m["steel_flat"])
    s.add(beam((-1.411, 1.45, 1.26), (-1.411, 2.35, 1.03), 0.02, 0.05), m["steel_flat"])

    # ---------------------------------------------------------------- Steuerbord-Modul: Beschriftung, Deckel, Ladung
    # An der Außenwand die Nummer des Vorbesitzers, durchgestrichen und eine Ecke unter einem Rostblech; die Luke auf dem Dach
    # mit einem Flacheisen zugeschweißt; der Schrottdeckel verschraubt, ein Container mit einer Kette verzurrt, der
    # andere mit einem Band; eine Strebe vom Rumpf aufs Dach
    roof, lid = 0.3, 0.37
    number(s, m["paint"], "58", (2.75, 1.5, 0.06), 0.24, (90, 0, 90))
    s.add(box(0.01, 0.56, 0.032), m["dark"], at=(2.758, 1.5, 0.06), rot=(15, 0, 0))
    s.add(plate([(-0.09, -0.07), (0.07, -0.08), (0.08, 0.06), (-0.08, 0.07)], 0.016, "yz"), m["rust"], at=(2.762, 1.33, -0.05))
    bolts(s, m["steel"], [(2.768, 1.33 + dy, -0.05 + dz) for dy, dz in ((-0.065, -0.045), (0.05, -0.055), (0.055, 0.035), (-0.055, 0.045))], "x",
          r=0.014)
    s.add(box(0.46, 0.05, 0.016), m["steel_flat"], at=(2.2, 0.3, roof + 0.036), rot=(0, 0, 35))
    bolts(s, m["steel"], [(2.2 + sx * 0.17, 0.3 + sx * 0.119, roof + 0.044) for sx in (-1, 1)], r=0.016)
    bolts(s, m["steel"], [(x, y, lid) for x, ys in ((1.6, (1.78, 2.8, 3.26)), (2.66, (1.76, 2.56, 3.27))) for y in ys], r=0.018)
    for a, b in (((2.63, 2.14, lid), (2.585, 2.14, lid + 0.415)), ((2.585, 2.14, lid + 0.415), (2.075, 2.14, lid + 0.415)),
                 ((2.075, 2.14, lid + 0.415), (2.03, 2.14, lid))):
        chain(s, m["steel_flat"], a, b)
    s.add(box(0.5, 0.05, 0.37), m["rust"], at=(2.3, 2.92, lid + 0.18), rot=(0, 0, 9))
    s.add(beam(on(1.15, 0.62, -0.03), (1.75, 0.62, roof + 0.02), 0.06, 0.06), m["steel_flat"])

    # ---------------------------------------------------------------- Backbord-Sponson: Flicken, Leitung, Buchten, Kette
    # Auf der oberen Außenfläche zwei schiefe Bleche, verschraubt; auf dem First eine Leitung, die vorn zum Rumpf
    # abbiegt, mit Schellen; Schrauben in den Rahmen der Buchten und des Hangarmauls; achtern eine Kette zum Rumpf
    sx, sz, rx, rz = 2.3, -0.32, 0.6, 0.75
    for y, mat, quad in ((1.15, "rust", [(-0.2, -0.42), (0.21, -0.38), (0.19, 0.4), (-0.21, 0.36)]),
                         (2.75, "scrap", [(-0.16, -0.26), (0.18, -0.29), (0.2, 0.24), (-0.14, 0.27)])):
        g = Ship("patch")
        g.add(plate(quad, 0.03, "xy"), m[mat])
        bolts(g, m["steel"], [(0.82 * qx, 0.88 * qy, 0.015) for qx, qy in quad], r=0.016)
        s.place(g, at=(-sx - 0.26 - 0.585 * 0.008, y, sz + 0.5625 + 0.811 * 0.008), rot=(0, -35.8, 0))
        g.bm.free()
    ridge = sz + rz - 0.005
    s.add(pipe([(-1.45, 0.02, 0.5), (-1.95, 0.1, 0.52), (-sx, 0.3, ridge), (-sx, 3.2, ridge), (-sx, 3.3, ridge - 0.08)], 0.03, 6), m["steel"])
    for y in (0.8, 1.9, 2.9):
        s.add(box(0.1, 0.05, 0.05), m["armor"], at=(-sx, y, ridge))
    face = sx + rx * cos(30)
    for y in (0.55, 1.6, 2.65):
        bolts(s, m["steel"], [(-face - 0.07, y + 0.26 * sin(a), sz - 0.26 * cos(a)) for a in range(0, 360, 60)], "-x", r=0.016)
    bolts(s, m["steel"], [(-sx + 0.57 * cos(a), -0.78, sz + 0.73 * sin(a)) for a in range(30, 360, 60)], "-y", r=0.018)
    chain(s, m["steel_flat"], (-1.5, 2.2, 0.85), (-sx + 0.04, 2.2, ridge), sag=0.07)

    # ---------------------------------------------------------------- Bug: Sensorauge, Greifer, Aufhängung der Beutekanone
    # Schrauben im Rahmen des Sensorauges; Achsmuttern und ein Hydraulikschlauch am Greifer, an der Zange backbord eine
    # Kette mit Haken; die Beutekanone hängt in zwei verschraubten Bändern
    bolts(s, m["steel"], [(0.36 * cos(a), -4.97, -0.15 + 0.36 * sin(a)) for a in range(0, 360, 60)], "-y", r=0.018)
    for at, r in (((1.88, -2.8, 0.0), 0.07), ((2.04, -3.92, -0.06), 0.06), ((1.45, -4.5, -0.08), 0.05)):
        bolts(s, m["steel_flat"], [at], "x", r=r, mirror=True)
    s.add(pipe([(1.62, -2.55, 0.24), (1.98, -3.0, 0.2), (2.0, -3.6, 0.12), (1.9, -3.86, 0.1)], 0.022, 6), m["dark_round"], mirror=True)
    chain(s, m["steel_flat"], (-1.3, -5.02, -0.1), (-1.3, -5.02, -0.52))
    s.add(box(0.03, 0.09, 0.1), m["steel_flat"], at=(-1.3, -5.0, -0.56))
    for y in (-3.4, -2.45):
        s.add(box(0.9, 0.08, 0.02), m["steel_flat"], at=(0, y, -1.355))
        s.add(box(0.02, 0.08, 0.5), m["steel_flat"], at=(0.44, y, -1.115), mirror=True)
        bolts(s, m["steel"], [(0.45, y, -1.25), (0.45, y, -1.05)], "x", r=0.016, mirror=True)

    # ---------------------------------------------------------------- Heck: Stellstreben, Schellen, Kette
    # Stellstreben an den Düsenglocken, an der fremden Düse fehlt eine; Schellen um ihre Treibstoffleitung, eine Kette
    # hält ihr Gehäuse an der Brutplattform
    for r, y0, y1, dx, dz, angles in ((0.5, 4.5, 5.45, 0.0, 0.05, range(22, 360, 45)), (0.34, 4.35, 5.15, 0.8, -0.45, range(0, 360, 60)),
                                      (0.34, 4.35, 5.15, -0.8, -0.45, range(0, 360, 60)), (0.3, 4.3, 5.1, -0.78, 1.15, range(0, 288, 72))):
        ya = y0 + 0.6 * (y1 - y0)
        for a in angles:
            s.add(segment((dx + 1.1 * r * cos(a), ya, dz + 1.1 * r * sin(a)), (dx + 0.93 * r * cos(a), y1 - 0.03, dz + 0.93 * r * sin(a)), 0.014, 6),
                  m["steel"])
    line = [Vector(p) for p in ((-1.25, 3.5, 0.8), (-1.42, 3.85, 1.0), (-1.3, 4.25, 1.17), (-1.0, 4.5, 1.2))]
    for a, b in zip(line[:2], line[1:3]):
        mid, d = (a + b) / 2, (b - a).normalized()
        s.add(sweep([mid - d * 0.04, mid + d * 0.04], ngon(16, 0.13)), m["armor_round"])
    chain(s, m["steel_flat"], (-0.62, 4.4, 1.42), (-0.3, 3.8, 1.3), sag=0.05)


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

    # ---------------------------------------------------------------- Rumpf: zwei Kammern wie die Hive, Kragen, Bauchschale
    body = lambda w, h: superellipse(w, h, ROUND, 3.0)  # noqa: E731
    rings = smooth([
        sec(-4.85, body(0.9, 0.6), dz=-0.15),
        sec(-4.35, body(2.0, 1.3), dz=-0.08),
        sec(-2.8, body(3.0, 1.9)),
        sec(-0.75, body(2.3, 1.5)),
        sec(1.3, body(3.7, 2.3), dz=0.05),
        sec(3.3, body(3.5, 2.2), dz=0.05),
        sec(4.45, body(2.5, 1.6)),
        sec(4.85, body(1.6, 1.05)),
    ], 3)
    part = loft(rings)
    tree = BVHTree.FromBMesh(part)
    s.add(part, m["hull_round"])
    for y in (-1.1, -0.35):
        s.add(band(rings, y - 0.12, y + 0.12, 0.07), m["accent_round"])
    s.add(band(rings, -4.05, -1.55, 0.09, below=0.0), m["armor_round"])

    # ---------------------------------------------------------------- Flicken: aufgeschweißte Platten in Lagen
    for keys, lift, mat, k in PATCHES:
        s.add(shell(rings, keys, lift=lift, k=k), m[mat])

    # ---------------------------------------------------------------- Brutplattform: zehn Waben, offene glühen orange, vier mit Rost verdeckelt
    pitch, rc = 0.92, (0.92 - 0.16) / math.sqrt(3)
    yc, ztop = 2.25, 1.3
    cells = [(0.0, yc + (k - 1.5) * pitch) for k in range(4)]
    cells += [(sx * pitch * math.sqrt(3) / 2, yc + (k - 1) * pitch) for sx in (1, -1) for k in range(3)]
    corners = [(x + rc * math.cos(a), y + rc * math.sin(a)) for x, y in cells for a in (math.radians(60 * i) for i in range(6))]
    deck = grow(convex_hull(corners), 0.16)
    s.add(chamfer(plate(deck, 0.7, "xy"), 0.025), m["dark"], at=(0, 0, ztop - 0.35))
    for i, (x, y) in enumerate(cells):
        if i in (2, 4, 6, 9):
            s.add(loft([sec(0.0, ngon(6, rc)), sec(0.12, ngon(6, rc * 0.9)), sec(0.22, ngon(6, rc * 0.5))]), m["rust"],
                  at=(x, y, ztop - 0.02), rot=(90, 0, 0))
        else:
            s.add(post(6, rc, rc, 0.0, 0.05), m["ember"], at=(x, y, ztop - 0.01))

    # ---------------------------------------------------------------- Sechseckturm, darauf statt Mast und Wappen die Trümmerkrone
    ty = -2.1
    hit, _ = on_surface(tree, 0, ty)
    z = hit.z - 0.1
    s.add(chamfer(post(6, 0.85, 0.81, 0.0, 0.62), 0.03), m["hull"], at=(0, ty, z))
    z += 0.62
    s.add(chamfer(post(6, 0.62, 0.58, 0.0, 0.56), 0.03), m["armor"], at=(0, ty, z))
    s.add(hoop(6, 0.55, 0.62, 0.11), m["window"], at=(0, ty, z + 0.36))
    s.add(chamfer(post(6, 0.66, 0.66, 0.0, 0.14), 0.02), m["dark"], at=(0, ty, z + 0.56))
    crown(s, m, (0, ty, z + 0.62), 0.55)
    ledge = z

    # ---------------------------------------------------------------- Bug: Sensorauge, Bergungsgreifer, Beutekanone
    s.add(chamfer(tube(6, 0.44, 0.44, -4.97, -4.7, dz=-0.15, phase=HEX), 0.03), m["dark"])
    s.add(tube(6, 0.3, 0.3, -5.0, -4.9, dz=-0.15, phase=HEX), m["ember"])
    grapple(s, m)
    cz = -1.0
    s.add(rbox(0.8, 1.6, 0.7, 0.06, 1), m["dark"], at=(0.0, -2.9, cz))
    s.add(bellows(ROUND, 0.32, -3.65, -3.92), m["armor_round"], at=(0, 0, cz))
    s.add(tube(ROUND, 0.21, 0.19, -4.95, -3.6), m["steel"], at=(0, 0, cz))
    s.add(tube(ROUND, 0.28, 0.28, -4.5, -4.08), m["dark_round"], at=(0, 0, cz))
    s.add(tube(ROUND, 0.17, 0.17, -5.2, -4.9), m["steel"], at=(0, 0, cz))
    s.add(tube(ROUND, 0.31, 0.31, -5.22, -5.08), m["dark_round"], at=(0, 0, cz))
    s.add(tube(ROUND, 0.31, 0.31, -4.98, -4.84), m["dark_round"], at=(0, 0, cz))

    # ---------------------------------------------------------------- Backbord: Hangar-Sponson der Hive mit drei Buchten und dem Hangarmaul
    sx, sz, rx, rz = 2.3, -0.32, 0.6, 0.75
    pod = [sec(-0.6, ngon(6, rx, rz, HEX)), sec(3.5, ngon(6, rx, rz, HEX)), sec(4.15, ngon(6, 0.4, 0.5, HEX))]
    s.add(chamfer(loft(pod), 0.04), m["accent"], at=(-sx, 0, sz))
    face = sx + rx * math.cos(HEX)
    for y in (0.55, 1.6, 2.65):
        s.add(hoop(6, 0.2, 0.32, 0.12), m["dark"], at=(-face - 0.01, y, sz), rot=(0, 90, 0))
        s.add(tube(6, 0.22, 0.22, -0.06, 0.03, phase=HEX), m["ember"], at=(-face, y, sz), rot=(0, 0, 90))
    s.add(hex_frame(rx + 0.07, rz + 0.08, 0.2, -0.78, -0.5), m["armor"], at=(-sx, 0, sz))
    s.add(loft([sec(-0.66, ngon(6, rx - 0.08, rz - 0.08, HEX)), sec(-0.56, ngon(6, rx - 0.08, rz - 0.08, HEX))]), m["dark"], at=(-sx, 0, sz))
    s.add(box(0.5, 0.06, 0.07), m["ember"], at=(-sx, -0.665, sz - 0.42))

    # ---------------------------------------------------------------- Steuerbord: fremdes Modul mit erbeutetem Zwillingsturm, Containern, Leitung
    s.add(rbox(1.2, 3.6, 1.1, 0.08, 1), m["armor"], at=(2.15, 1.55, -0.25))
    s.add(box(0.04, 2.4, 0.09), m["glow"], at=(2.76, 1.55, -0.2))
    s.add(chamfer(box(0.9, 0.9, 0.3), 0.03), m["dark"], at=(2.15, 1.2, 0.45))
    turret(s, m, (2.15, 1.2, 0.6), r=0.46, barrel=1.3)
    turret_details(s, m, (2.15, 1.2, 0.6), r=0.46, barrel=1.3)
    s.add(chamfer(box(1.26, 1.62, 0.07), 0.015), m["scrap"], at=(2.15, 2.52, 0.335))
    container(s, m, (2.33, 2.14, 0.37), (0.5, 0.62, 0.4), m["rust"])
    container(s, m, (2.3, 2.92, 0.37), (0.46, 0.52, 0.34), m["scrap"], yaw=9)
    # Leitung aus der vorderen Kammer an der Taille entlang in die Stirn des Moduls, unter den Rohren des Turms
    duct = [(1.12, -1.66, 0.2), (1.42, -1.48, 0.22), (1.5, -1.0, 0.18), (1.58, -0.6, 0.12), (1.76, -0.32, 0.04), (1.9, -0.1, 0.0)]
    s.add(sweep(duct, ngon(16, 0.11)), m["dark_round"])
    for a, b in ((duct[2], duct[3]), (duct[3], duct[4])):
        a, b = Vector(a), Vector(b)
        mid, d = (a + b) / 2, (b - a).normalized()
        s.add(sweep([mid - d * 0.05, mid + d * 0.05], ngon(16, 0.145)), m["armor_round"])
    s.add(beam((1.0, -0.8, 0.15), (1.52, -0.8, 0.15), 0.16, 0.18), m["dark"])

    # ---------------------------------------------------------------- Kiel unter dem Rumpf
    s.add(chamfer(plate([(-2.2, -0.55), (3.6, -0.65), (3.35, -1.35), (-1.6, -1.2)], 0.18, "yz"), 0.04), m["dark"])

    # ---------------------------------------------------------------- Heck: Maschinenlüfter, drei Düsen wie die Hive, darüber eine vierte, fremde Düse
    hit, rot = on_surface(tree, 0.78, 4.2)
    vent(s, m, m["hull"], tuple(hit), 0.44, 0.44, slats=2, rot=rot)
    engine(s, m, 0.5, 4.5, 5.45, dz=0.05, n=ROUND)
    engine_details(s, m, 0.5, 4.5, 5.45, dz=0.05, n=ROUND)
    engine(s, m, 0.34, 4.35, 5.15, dx=0.8, dz=-0.45, n=JET, mirror=True)
    engine_details(s, m, 0.34, 4.35, 5.15, dx=0.8, dz=-0.45, n=JET, mirror=True)
    s.add(chamfer(wedge(0.8, 1.2, 0.5), 0.03), m["rust"], at=(-0.75, 4.0, 1.05), rot=(0, -8, 0))
    engine(s, m, 0.3, 4.3, 5.1, dx=-0.78, dz=1.15, n=JET)
    engine_details(s, m, 0.3, 4.3, 5.1, dx=-0.78, dz=1.15, n=JET)
    # Treibstoffleitung der fremden Düse, außen an der Backbordflanke vorbei in ihr Gehäuse
    s.add(sweep([(-1.25, 3.5, 0.8), (-1.42, 3.85, 1.0), (-1.3, 4.25, 1.17), (-1.0, 4.5, 1.2)], ngon(16, 0.1)), m["dark_round"])
    details(s, m, rings, tree, ty, ledge)
    fine(s, m, rings, tree, ty, ledge)
    return s.lift(0.25).finish()
