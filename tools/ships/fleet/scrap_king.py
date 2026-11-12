"""Schrottkönig (Boss von Sektor 1), Clean Toon in der detailreichen Fassung (docs/stilregeln.md): der Hive-Rumpf,
zusammengeflickt und geplündert.

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
"""

import math

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import axle, beam, bellows, engine, engine_details, flak, on_surface, rail, ring_at, turret, turret_details, vent, windows
from vf import Ship, box, dish, grow, hoop, loft, ngon, pipe, plate, post, rbox, sec, segment, smooth, sphere, superellipse, sweep, tube, wedge

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
        s.add(plate(shard, 0.17, "xz"), m[mats[k]], at=(x, y, z0), rot=(-lean, 0, a - 90))


def grapple(s, m):
    """Bergungsgreifer an der breitesten Stelle der vorderen Kammer, nach vorn gefaltet (gespiegelt): Schultergehäuse,
    Oberarm mit Hydraulikzylinder, Gelenke als Walzen, Unterarm nach innen, Zange mit zwei Backen neben dem Bug."""
    shoulder, elbow, wrist = Vector((1.8, -2.8, 0.0)), Vector((1.86, -3.92, -0.06)), Vector((1.3, -4.5, -0.08))
    s.add(rbox(0.5, 0.8, 0.5, 0.06, 1), m["dark"], at=(1.5, -2.8, 0.0), mirror=True)
    s.add(axle(0.25, 0.16, 16), m["armor_round"], at=shoulder, mirror=True)
    s.add(beam(shoulder, elbow, 0.24, 0.32), m["armor"], mirror=True)
    # Hydraulikzylinder auf dem Oberarm: Zylinder mit Kopf, der Neigung des Arms folgend
    top = Vector((0, 0, 0.25))
    s.add(segment(shoulder + top + Vector((0, -0.12, 0)), elbow.lerp(shoulder, 0.2) + top, 0.1, 12), m["steel"], mirror=True)
    s.add(segment(shoulder + top + Vector((0, -0.1, 0)), shoulder.lerp(elbow, 0.3) + top, 0.13, 12), m["dark_round"], mirror=True)
    s.add(axle(0.21, 0.36, 16), m["armor_round"], at=elbow, mirror=True)
    s.add(beam(elbow, wrist, 0.22, 0.28), m["armor"], mirror=True)
    s.add(axle(0.17, 0.3, 12), m["dark_round"], at=wrist, mirror=True)
    # Zwei Backen, die obere nach oben, die untere (um Y gewendet) nach unten geöffnet
    jaw = [(0.08, 0.06), (-0.16, 0.2), (-0.46, 0.18), (-0.64, 0.0), (-0.48, 0.02), (-0.26, 0.07), (0.08, -0.08)]
    for pitch, flip in ((-12, 0), (12, 180)):
        g = Ship("jaw")
        g.add(plate(jaw, 0.18, "yz"), m["steel_flat"], rot=(0, flip, 0))
        s.place(g, at=wrist, rot=(pitch, 0, 0), mirror=True)
        g.bm.free()


def container(s, m, at, size, mat, yaw=0.0):
    """Schrottcontainer: Kasten aus `mat` mit dunklen Stirnrahmen, `at` ist die Mitte der Unterseite."""
    w, l, h = size
    g = Ship("container")
    g.add(box(w, l, h), mat, at=(0, 0, h / 2))
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
    s.add(post(8, 0.09, 0.09, lid - 0.01, lid + 0.23, 1.95, 2.25), m["rust"])
    s.add(post(8, 0.09, 0.09, lid - 0.01, lid + 0.19, 1.92, 2.47), m["dark_round"])
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
        s.add(tube(12, 0.305, 0.305, y, y + 0.035), m["steel"], at=(0, 0, -1.0))
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
        s.add(tube(16, 0.545, 0.545, y, y + 0.02, 0.0, 0.05), m["steel"])
    for y in (4.65, 4.7):
        s.add(tube(12, 0.37, 0.37, y, y + 0.02, 0.8, -0.45), m["steel"], mirror=True)
    s.add(beam((-0.35, 4.55, 0.6), (-0.7, 4.75, 1.0), 0.07, 0.07), m["steel_flat"])
    s.add(beam((-1.0, 4.4, 0.5), (-0.92, 4.75, 0.98), 0.07, 0.07), m["steel_flat"])


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
    s.add(band(rings, -4.05, -1.55, 0.09, below=0.0), m["armor_round"])

    # ---------------------------------------------------------------- Flicken: aufgeschweißte Platten in Lagen
    # Die zweite Lage überlappt jeweils eine Kante der ersten wie eine Schindel und reicht darüber hinaus
    # Bugkappe um das Sensorauge, darüber ein Rostblech steuerbord
    s.add(shell(rings, [(-4.72, 20, 160), (-4.4, 12, 168), (-4.02, 24, 150)], k=10), m["scrap"])
    s.add(shell(rings, [(-4.58, 30, 96), (-4.15, 18, 84), (-3.85, 26, 70)], lift=0.12), m["rust"])
    # vordere Kammer steuerbord: große Schrottplatte, über ihre obere Kante ein Panzerblech
    s.add(shell(rings, [(-3.7, 14, 50), (-3.3, 12, 58), (-2.98, 18, 48)]), m["scrap"])
    s.add(shell(rings, [(-3.45, 40, 84), (-3.0, 44, 92), (-2.9, 52, 80)], lift=0.12), m["armor_round"])
    # vordere Kammer backbord
    s.add(shell(rings, [(-3.55, 122, 166), (-2.75, 130, 174)]), m["rust"])
    s.add(shell(rings, [(-3.2, 156, 196), (-2.55, 162, 190)], lift=0.12), m["scrap"])
    # hintere Kammer steuerbord über dem Modul, backbord ein langer Streifen über dem Sponson
    s.add(shell(rings, [(0.35, 14, 42), (1.2, 10, 48), (2.0, 18, 40)]), m["scrap"])
    s.add(shell(rings, [(1.0, 30, 58), (1.75, 34, 62), (2.1, 40, 56)], lift=0.12), m["rust"])
    s.add(shell(rings, [(0.7, 146, 166), (1.8, 140, 168), (2.9, 148, 164)]), m["scrap"])
    s.add(shell(rings, [(2.35, 132, 156), (2.85, 128, 160), (3.25, 136, 152)], lift=0.12), m["rust"])

    # ---------------------------------------------------------------- Brutplattform: zehn Waben, offene glühen orange, vier mit Rost verdeckelt
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

    # ---------------------------------------------------------------- Sechseckturm, darauf statt Mast und Wappen die Trümmerkrone
    ty = -2.1
    hit, _ = on_surface(tree, 0, ty)
    z = hit.z - 0.1
    s.add(post(6, 0.85, 0.81, 0.0, 0.62), m["hull"], at=(0, ty, z))
    z += 0.62
    s.add(post(6, 0.62, 0.58, 0.0, 0.56), m["armor"], at=(0, ty, z))
    s.add(hoop(6, 0.55, 0.62, 0.11), m["window"], at=(0, ty, z + 0.36))
    s.add(post(6, 0.66, 0.66, 0.0, 0.14), m["dark"], at=(0, ty, z + 0.56))
    crown(s, m, (0, ty, z + 0.62), 0.55)
    ledge = z

    # ---------------------------------------------------------------- Bug: Sensorauge, Bergungsgreifer, Beutekanone
    s.add(tube(6, 0.44, 0.44, -4.97, -4.7, dz=-0.15, phase=HEX), m["dark"])
    s.add(tube(6, 0.3, 0.3, -5.0, -4.9, dz=-0.15, phase=HEX), m["ember"])
    grapple(s, m)
    cz = -1.0
    s.add(rbox(0.8, 1.6, 0.7, 0.06, 1), m["dark"], at=(0.0, -2.9, cz))
    s.add(bellows(18, 0.32, -3.65, -3.92), m["armor_round"], at=(0, 0, cz))
    s.add(tube(18, 0.21, 0.19, -4.95, -3.6), m["steel"], at=(0, 0, cz))
    s.add(tube(18, 0.28, 0.28, -4.5, -4.08), m["dark_round"], at=(0, 0, cz))
    s.add(tube(18, 0.17, 0.17, -5.2, -4.9), m["steel"], at=(0, 0, cz))
    s.add(tube(18, 0.31, 0.31, -5.22, -5.08), m["dark_round"], at=(0, 0, cz))
    s.add(tube(18, 0.31, 0.31, -4.98, -4.84), m["dark_round"], at=(0, 0, cz))

    # ---------------------------------------------------------------- Backbord: Hangar-Sponson der Hive mit drei Buchten und dem Hangarmaul
    sx, sz, rx, rz = 2.3, -0.32, 0.6, 0.75
    pod = [sec(-0.6, ngon(6, rx, rz, HEX)), sec(3.5, ngon(6, rx, rz, HEX)), sec(4.15, ngon(6, 0.4, 0.5, HEX))]
    s.add(loft(pod), m["accent"], at=(-sx, 0, sz))
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
    s.add(box(0.9, 0.9, 0.3), m["dark"], at=(2.15, 1.2, 0.45))
    turret(s, m, (2.15, 1.2, 0.6), r=0.46, barrel=1.3)
    turret_details(s, m, (2.15, 1.2, 0.6), r=0.46, barrel=1.3)
    s.add(box(1.26, 1.62, 0.07), m["scrap"], at=(2.15, 2.52, 0.335))
    container(s, m, (2.33, 2.14, 0.37), (0.5, 0.62, 0.4), m["rust"])
    container(s, m, (2.3, 2.92, 0.37), (0.46, 0.52, 0.34), m["scrap"], yaw=9)
    # Leitung aus der vorderen Kammer an der Taille entlang in die Stirn des Moduls, unter den Rohren des Turms
    duct = [(1.12, -1.66, 0.2), (1.42, -1.48, 0.22), (1.5, -1.0, 0.18), (1.58, -0.6, 0.12), (1.76, -0.32, 0.04), (1.9, -0.1, 0.0)]
    s.add(sweep(duct, ngon(12, 0.11)), m["dark_round"])
    for a, b in ((duct[2], duct[3]), (duct[3], duct[4])):
        a, b = Vector(a), Vector(b)
        mid, d = (a + b) / 2, (b - a).normalized()
        s.add(sweep([mid - d * 0.05, mid + d * 0.05], ngon(12, 0.145)), m["armor_round"])
    s.add(beam((1.0, -0.8, 0.15), (1.52, -0.8, 0.15), 0.16, 0.18), m["dark"])

    # ---------------------------------------------------------------- Kiel unter dem Rumpf
    s.add(plate([(-2.2, -0.55), (3.6, -0.65), (3.35, -1.35), (-1.6, -1.2)], 0.18, "yz"), m["dark"])

    # ---------------------------------------------------------------- Heck: Maschinenlüfter, drei Düsen wie die Hive, darüber eine vierte, fremde Düse
    hit, rot = on_surface(tree, 0.78, 4.2)
    vent(s, m, m["hull"], tuple(hit), 0.44, 0.44, slats=2, rot=rot)
    engine(s, m, 0.5, 4.5, 5.45, dz=0.05)
    engine_details(s, m, 0.5, 4.5, 5.45, dz=0.05)
    engine(s, m, 0.34, 4.35, 5.15, dx=0.8, dz=-0.45, mirror=True)
    engine_details(s, m, 0.34, 4.35, 5.15, dx=0.8, dz=-0.45, mirror=True)
    s.add(wedge(0.8, 1.2, 0.5), m["rust"], at=(-0.75, 4.0, 1.05), rot=(0, -8, 0))
    engine(s, m, 0.3, 4.3, 5.1, dx=-0.78, dz=1.15, n=16)
    engine_details(s, m, 0.3, 4.3, 5.1, dx=-0.78, dz=1.15, n=16)
    # Treibstoffleitung der fremden Düse, außen an der Backbordflanke vorbei in ihr Gehäuse
    s.add(sweep([(-1.25, 3.5, 0.8), (-1.42, 3.85, 1.0), (-1.3, 4.25, 1.17), (-1.0, 4.5, 1.2)], ngon(12, 0.1)), m["dark_round"])
    details(s, m, rings, tree, ty, ledge)
    return s.lift(0.25).finish()
