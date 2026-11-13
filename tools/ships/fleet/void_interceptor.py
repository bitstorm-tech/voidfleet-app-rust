"""Void Interceptor (Jäger), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste Flügel und Flossen, glatte
runde Teile, Kleinteile und Feinteile.

Primärformen: runder Rumpf, der vorn in eine lange Nadelnase ausläuft, darauf eine Glaskanzel. Klingenflügel mit
breitem Vorderkantenstreifen in Koralle, darunter zwei runde Triebwerksgondeln mit Lichtband, V-Leitwerk mit
Koralle-Spitzen, zwei Bugkanonen unter den Flügelwurzeln, dunkle Kielflosse.
Detailzonen (im Spiel steht er als Staffel zu je 0,42, dort zählen nur Umriss, Akzent und die großen Gliederungen),
dazwischen ruhig (Rumpfflanken, Flügel vor den Lüftern, Leitwerk):
- Bug: Stahlkappe um die Nadelspitze (macht die Nadel länger lesbar)
- Kanzel: Kanzelbügel quer über das Glas
- Bugkanonen: Blendenbalg an der Laufwurzel, zweiteilige Mündungsbremse (Stahlkern im Spalt)
- Flügel: Maschinenlüfter auf dem Flügel über jeder Gondel
- Gondeln: Bugkappe vorn, Gehäusesegment und Düsenkranz an der Düse (`parts.engine_details`)
- Unterseite: Bugfahrwerksklappen zwischen den Kanonen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Raketen an
Pylonen unter den Flügeln, Flügelspitzen-Schienen mit Positionslicht, Klappenspalt, Wartungsluken, zweiter Kanzelbügel,
Blatt- und Peitschenantenne, Kabelkanal an der Rumpfflanke, Kühlrippen an Kanonen und Gondeln, Sensor und Staurohr am
Bug.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; an einem Jäger sparsam und nichts unter 0,03): Steuerflächen
(Klappe und Querruder durch Spalte getrennt, Gelenke über dem Klappenspalt, Seitenruder mit Spalt und Gelenken), Rahmen
um die Wartungsluken, Avionikklappe an der Rumpfflanke unter der Kanzel, Bremsklappe mit Gelenkleiste zwischen den
Flossen, Sitzring der Bugkappe, Stellstreben an den Düsenglocken, Schellen um die Kanonengehäuse, Pendelstützen an den
Raketen. Dazu Beschriftung als flache Geometrie: ein Warnband neben jedem Maschinenlüfter und statt einer Kennnummer
(er steht zu fünft) die Staffelstreifen, zwei helle Streifen hinter dem Vorderkantenstreifen am Außenflügel und zwei an
jeder Flosse unter der Koralle-Spitze.
"""

import math

import palette
import vf
from mathutils import Vector
from parts import bellows, engine, engine_details, ring_at, stripes, vent
from vf import Ship, box, chamfer, flank, loft, plate, post, rect, sec, segment, smooth, sphere, superellipse, sweep, tip, tube

ROUND = 24  # Segmente der großen runden Teile (Rumpf, Gondeln, Düsen); mittlere (Kanzel, Kanonen) 16
ROOT, LE_TIP = (0.3, -1.25), (1.95, 0.75)  # Vorderkante des Flügels (x, y) von der Wurzel zur Spitze
FIN = [(0.95, -0.12), (2.2, -0.12), (2.5, 0.9), (2.12, 0.94)]  # Flosse des V-Leitwerks (y, z), vor dem Neigen
FIN_AT, FIN_LEAN = (0.22, 0, 0.14), 30


def edge_band(a, b, width, x0, x1, out=0.015):
    """Streifen (x, y) entlang der Vorderkante a→b eines Flügels, dessen Inneres bei größerem y liegt: senkrecht
    `width` breit, um `out` über die Kante hinaus, an den Enden bei x0 und x1 gerade abgeschnitten."""
    (ax, ay), (bx, by) = a, b
    slope = (by - ay) / (bx - ax)
    cos = (bx - ax) / ((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5

    def edge(x):
        return ay + (x - ax) * slope

    lo, hi = -out / cos, width / cos
    return [(x0, edge(x0) + lo), (x1, edge(x1) + lo), (x1, edge(x1) + hi), (x0, edge(x0) + hi)]


def around(ring, off):
    """Querschnitt um `off` von seiner Mitte weg nach außen versetzt (auch bei Lofts, die nicht auf der Achse liegen)."""
    c = sum(ring, Vector()) / len(ring)
    return [p + Vector((p.x - c.x, 0.0, p.z - c.z)).normalized() * off for p in ring]


def cap(rings, y1, off, nose):
    """Kappe über der Spitze eines Lofts (Ringe wie aus `smooth`, beginnend mit der Spitze `nose`) bis y1."""
    inner = [around(r, off) for r in rings if isinstance(r, list) and r[0].y < y1]
    return loft([nose] + inner + [around(ring_at(rings, y1), off)])


def band(rings, y0, y1, off):
    """Band um einen Loft (Ringe wie aus `smooth`) von y0 bis y1, um `off` nach außen versetzt."""
    inner = [around(r, off) for r in rings if isinstance(r, list) and y0 < r[0].y < y1]
    return loft([around(ring_at(rings, y0), off)] + inner + [around(ring_at(rings, y1), off)])


def skin(rings, y0, y1, a, b, off, sunk=0.01):
    """Blech auf einem Loft (Ringe wie aus `smooth`): die Haut von y0 bis y1 zwischen den Ringpunkten a und b, um `off`
    erhaben und um `sunk` versenkt (Klappen und Deckel, die der Rundung folgen)."""
    ys = [y0] + [r[0].y for r in rings if isinstance(r, list) and y0 < r[0].y < y1] + [y1]
    return loft([around(ring_at(rings, y), off)[a:b + 1] + around(ring_at(rings, y), -sunk)[a:b + 1][::-1] for y in ys])


def hinge(x):
    """y der Klappenachse bei x, 0,12 vor der Hinterkante des Flügels."""
    return 1.75 - (x - 0.3) * 0.1818 - 0.12


def details(s, m, hull, canopy, gx, gz):
    """Kleinteile (Modulkommentar); `hull` und `canopy` sind die Ringe der Lofts, (gx, gz) die Gondeln."""

    def top(y):
        """Höhe des Rumpfrückens bei y."""
        return max(p.z for p in ring_at(hull, y))

    def under(x):
        """Höhe der Flügelunterseite bei x (der Flügel steigt um 3° nach außen)."""
        return -0.07 + 0.0523 * x

    # ---------------------------------------------------------------- Raketen an Pylonen unter den Flügeln
    for x, y0 in ((1.5, 0.1), (1.78, 0.4)):
        z = under(x)
        s.add(chamfer(plate([(0.38, 0.04), (0.9, 0.04), (0.86, -0.04), (0.52, -0.04)], 0.06, "yz"), 0.015), m["dark"], at=(x, y0, z - 0.03),
              mirror=True)
        s.add(tube(8, 0.05, 0.05, y0, y0 + 1.0), m["armor_round"], at=(x, 0, z - 0.11), mirror=True)
        s.add(tube(8, 0.0, 0.05, y0 - 0.2, y0), m["accent_round"], at=(x, 0, z - 0.11), mirror=True)
        s.add(box(0.18, 0.1, 0.012), m["dark"], at=(x, y0 + 0.93, z - 0.11), mirror=True)
        s.add(box(0.012, 0.1, 0.18), m["dark"], at=(x, y0 + 0.93, z - 0.11), mirror=True)

    # ---------------------------------------------------------------- Flügel: Spitzenschiene mit Licht, Klappenspalt, Luke
    tip_z = -0.02 + 0.0523 * 1.95
    s.add(chamfer(box(0.07, 0.75, 0.07), 0.015), m["dark"], at=(1.95, 1.1, tip_z), mirror=True)
    s.add(sphere(0.04, 1), m["glow"], at=(1.95, 0.71, tip_z), mirror=True)

    gap = [(0.45, hinge(0.45) - 0.015), (1.85, hinge(1.85) - 0.015), (1.85, hinge(1.85) + 0.015), (0.45, hinge(0.45) + 0.015)]
    s.add(plate(gap, 0.115, "xy"), m["dark"], at=(0, 0, -0.02), rot=(0, -3, 0), mirror=True)
    s.add(box(0.26, 0.34, 0.03), m["armor"], at=(0.72, 0.1, 0.03 + 0.0523 * 0.72), rot=(0, -3, 0), mirror=True)

    # ---------------------------------------------------------------- Kanzel und Rücken: Bügel, Luke, Antennen, Hecklicht
    s.add(band(canopy, -1.06, -0.98, 0.02), m["hull_round"])
    s.add(box(0.22, 0.3, 0.03), m["armor"], at=(0, 0.95, top(0.95) + 0.004))
    s.add(box(0.025, 0.34, 0.2), m["dark"], at=(0, 0.35, top(0.35) + 0.08))
    s.add(post(6, 0.012, 0.005, top(1.5) - 0.04, top(1.5) + 0.45), m["dark_round"], at=(0, 1.5, 0))
    s.add(sphere(0.035, 1), m["glow"], at=(0, 2.42, top(2.42)))

    # ---------------------------------------------------------------- Kabelkanal an der Flanke, über der Flügelwurzel
    path = [p for p in flank(hull, 0.16, 0.02) if -0.6 <= p[1] <= 1.9]
    s.add(sweep(path, rect(0.05, 0.06)), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Kühlrippen an Kanonen und Gondeln
    for y in (-2.0, -2.12, -2.24):
        s.add(tube(16, 0.125, 0.125, y, y + 0.035), m["dark_round"], at=(0.42, 0, -0.2), mirror=True)
    for y in (1.35, 1.47, 1.59):
        s.add(tube(ROUND, 0.34, 0.34, y, y + 0.03, gx, gz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Bug: Sensor unter der Nase, Staurohr
    s.add(sphere(0.06, 1), m["glass"], at=(0, -2.6, -0.12))
    s.add(segment((0.08, -3.0, 0.03), (0.08, -3.55, 0.05), 0.01, 6), m["steel"])


def fine(s, m, hull, gx, gz):
    """Feinteile und Beschriftung (Modulkommentar); `hull` sind die Ringe des Rumpfs, (gx, gz) die Gondeln."""

    def top(x):
        """Höhe der Flügeloberseite bei x."""
        return 0.03 + 0.0523 * x

    def through(outline, proud, mat):
        """Platte mit dem Umriss (x, y) durch den Flügel, oben und unten um `proud` erhaben."""
        s.add(plate(outline, 0.1 + 2 * proud, "xy"), mat, at=(0, 0, -0.02), rot=(0, -3, 0), mirror=True)

    # ---------------------------------------------------------------- Flügel: Klappe und Querruder, Gelenke, Lukenrahmen
    # Der Klappenspalt (`details`) teilt die Hinterkante ab; drei Spalte quer dazu machen daraus Klappe (innen) und
    # Querruder (außen), sie stoßen an den Spalt und enden vor der Fase der Hinterkante. Je zwei Gelenke über dem Spalt
    for x in (0.45, 1.15, 1.85):
        a, b = x - 0.015, x + 0.015
        through([(a, hinge(a) + 0.015), (b, hinge(b) + 0.015), (b, hinge(b) + 0.095), (a, hinge(a) + 0.095)], 0.0075, m["dark"])
    for x in (0.64, 0.96, 1.34, 1.66):
        s.add(box(0.05, 0.13, 0.125), m["steel_flat"], at=(x, hinge(x), -0.02 + 0.0523 * x), rot=(0, -3, 0), mirror=True)
    s.add(box(0.31, 0.39, 0.012), m["dark"], at=(0.72, 0.1, top(0.72)), rot=(0, -3, 0), mirror=True)

    # ---------------------------------------------------------------- Beschriftung: Warnband am Lüfter, Staffelstreifen
    # Warnband innen neben dem Maschinenlüfter; die Staffelstreifen liegen parallel hinter dem Vorderkantenstreifen am
    # Außenflügel und an jeder Flosse unter der Koralle-Spitze (durch Flügel und Flosse, auf beiden Seiten zu sehen)
    stripes(s, m, (0.73, 1.05, top(0.73)), 0.06, 0.62, rot=(0, -3, 0), mirror=True)
    for d in (0.33, 0.43):
        through(edge_band(ROOT, LE_TIP, d + 0.05, 1.32, 1.72, out=-d), 0.006, m["paint"])
    le, te = (FIN[3][0] - FIN[0][0]) / (FIN[3][1] - FIN[0][1]), (FIN[2][0] - FIN[1][0]) / (FIN[2][1] - FIN[1][1])

    def rudder(z, dy=0.0):
        """Punkt (y, z) auf der Achse des Seitenruders in Höhe z der Flosse, 0,2 vor ihrer Hinterkante, um dy versetzt."""
        return FIN[1][0] + (z - FIN[1][1]) * te - 0.2 + dy, z

    def lead(z, dy=0.0):
        return FIN[0][0] + (z - FIN[0][1]) * le + dy, z

    for z in (0.36, 0.47):
        s.add(plate([lead(z, 0.07), rudder(z, -0.05), rudder(z + 0.05, -0.05), lead(z + 0.05, 0.07)], 0.092, "yz"), m["paint"], at=FIN_AT,
              rot=(0, FIN_LEAN, 0), mirror=True)

    # ---------------------------------------------------------------- Leitwerk: Seitenruder mit Spalt und zwei Gelenken
    s.add(plate([rudder(0.0, -0.015), rudder(0.0, 0.015), rudder(0.62, 0.015), rudder(0.62, -0.015)], 0.095, "yz"), m["dark"], at=FIN_AT,
          rot=(0, FIN_LEAN, 0), mirror=True)
    lean = math.radians(FIN_LEAN)
    for z in (0.26, 0.56):
        s.add(box(0.108, 0.11, 0.045), m["steel_flat"], at=(FIN_AT[0] + z * math.sin(lean), rudder(z)[0], FIN_AT[2] + z * math.cos(lean)),
              rot=(0, FIN_LEAN, 0), mirror=True)

    # ---------------------------------------------------------------- Rumpf: Avionikklappe, Bremsklappe, Sitzring der Bugkappe
    # Bleche, die der Rundung folgen: die Avionikklappe an der Flanke unter der Kanzel, die Bremsklappe auf dem Rücken
    # zwischen den Flossen mit ihrer Gelenkleiste vorn, der dunkle Sitzring am Fuß der Stahlkappe; Rahmen um die Rückenluke
    s.add(skin(hull, -1.95, -1.4, 1, 3, 0.012), m["armor_round"], mirror=True)
    s.add(skin(hull, 1.74, 2.2, ROUND // 4 - 1, ROUND // 4 + 1, 0.012), m["armor_round"])
    s.add(skin(hull, 1.69, 1.74, ROUND // 4 - 1, ROUND // 4 + 1, 0.022), m["dark_round"])
    s.add(band(hull, -2.8, -2.74, 0.035), m["dark_round"])
    s.add(box(0.27, 0.35, 0.012), m["dark"], at=(0, 0.95, max(p.z for p in ring_at(hull, 0.95)) + 0.004))

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    r, y0, y1 = 0.28, 1.85, 2.6
    for k in range(6):
        a = 2 * math.pi * (k + 0.5) / 6
        c, d = math.cos(a), math.sin(a)
        s.add(segment((gx + 1.1 * r * c, y0 + 0.6 * (y1 - y0), gz + 1.1 * r * d), (gx + 0.93 * r * c, y1 - 0.03, gz + 0.93 * r * d), 0.016, 6),
              m["steel"], mirror=True)

    # ---------------------------------------------------------------- Waffen: Schellen um die Kanonengehäuse, Pendelstützen
    for y in (-1.35, -0.75):
        s.add(tube(16, 0.168, 0.168, y, y + 0.07), m["armor_round"], at=(0.42, 0, -0.2), mirror=True)
    for x, y0 in ((1.5, 0.1), (1.78, 0.4)):
        for dy in (0.5, 0.85):
            s.add(box(0.13, 0.04, 0.04), m["dark"], at=(x, y0 + dy, -0.07 + 0.0523 * x - 0.068), mirror=True)


def build():
    s = Ship("void_interceptor")
    m = vf.materials(s.name, "fighter")
    # Vorderkantenstreifen: flache Platte, aber rundes Material ohne Glanzpunkt. Die Kanten bleiben hart (alle über 38°),
    # es greift nur das schmale Kantenlicht runder Teile; das breite der eckigen mischt waagrechte Flächen in der
    # Brettansicht zu 65 % in RIM_COLOR und macht Koralle fliederfarben, ein Glanzpunkt ließe die ganze Facette aufblitzen
    m["accent_plate"] = vf.toon(f"{s.name}_accent_plate", palette.CLASSES["fighter"]["accent"], round=True)
    N = ROUND

    # ---------------------------------------------------------------- Rumpf, Nadelnase mit Bugkappe, Kanzel
    hull = smooth([
        tip(-3.8, dz=-0.02),
        sec(-3.2, superellipse(0.15, 0.13, N), dz=-0.02),
        sec(-2.5, superellipse(0.34, 0.28, N), dz=-0.01),
        sec(-1.6, superellipse(0.7, 0.52, N)),
        sec(-0.6, superellipse(0.96, 0.66, N)),
        sec(0.6, superellipse(0.98, 0.68, N)),
        sec(1.6, superellipse(0.82, 0.58, N)),
        sec(2.3, superellipse(0.62, 0.46, N)),
        sec(2.5, superellipse(0.5, 0.38, N)),
    ], 2)
    s.add(loft(hull), m["hull_round"])
    s.add(cap(hull, -2.75, 0.025, tip(-3.84, dz=-0.02)), m["steel"])
    # Glaskanzel, unten überall im Rumpf, mit Bügel zwischen Frontscheibe und Haube
    canopy = smooth([
        sec(-2.2, superellipse(0.12, 0.06, 16), dz=0.17),
        sec(-1.7, superellipse(0.42, 0.3, 16), dz=0.27),
        sec(-1.0, superellipse(0.5, 0.36, 16), dz=0.3),
        sec(-0.35, superellipse(0.32, 0.16, 16), dz=0.32),
    ], 2)
    s.add(loft(canopy), m["glass"])
    s.add(band(canopy, -1.66, -1.56, 0.02), m["hull_round"])

    # ---------------------------------------------------------------- Klingenflügel mit Vorderkantenstreifen und Lüfter
    wing = [ROOT, LE_TIP, (1.95, 1.45), (0.3, 1.75)]
    s.add(chamfer(plate(wing, 0.1, "xy"), 0.025), m["hull"], at=(0, 0, -0.02), rot=(0, -3, 0), mirror=True)
    s.add(chamfer(plate(edge_band(ROOT, LE_TIP, 0.27, 0.25, 1.965), 0.12, "xy"), 0.02), m["accent_plate"], at=(0, 0, -0.02), rot=(0, -3, 0),
          mirror=True)
    gx, gz = 1.03, -0.27
    vent(s, m, m["hull"], (gx, 1.05, 0.03 + 0.0523 * gx), 0.44, 0.74, slats=2, rot=(0, -3, 0), mirror=True)

    # ---------------------------------------------------------------- Triebwerksgondeln unter den Flügeln
    nacelle = smooth([tip(-1.0), sec(-0.55, superellipse(0.5, 0.48, N)), sec(0.4, superellipse(0.62, 0.6, N)),
                      sec(1.9, superellipse(0.58, 0.56, N))], 2)
    s.add(loft(nacelle), m["hull_round"], at=(gx, 0, gz), mirror=True)
    s.add(cap(nacelle, -0.5, 0.02, tip(-1.03)), m["armor_round"], at=(gx, 0, gz), mirror=True)
    engine(s, m, 0.28, 1.85, 2.6, dx=gx, dz=gz, n=N, mirror=True)
    engine_details(s, m, 0.28, 1.85, 2.6, dx=gx, dz=gz, n=N, mirror=True)
    s.add(box(0.05, 1.0, 0.09), m["glow"], at=(gx + 0.3, 0.5, gz), mirror=True)

    # ---------------------------------------------------------------- V-Leitwerk, nach außen geneigt, mit Koralle-Spitzen
    s.add(chamfer(plate(FIN, 0.08, "yz"), 0.02), m["hull"], at=FIN_AT, rot=(0, FIN_LEAN, 0), mirror=True)
    s.add(chamfer(plate([(1.72, 0.6), (2.43, 0.6), (2.53, 0.92), (2.1, 0.97)], 0.1, "yz"), 0.02), m["accent"], at=FIN_AT, rot=(0, FIN_LEAN, 0),
          mirror=True)

    # ---------------------------------------------------------------- Unterseite: Kielflosse, Bugfahrwerksklappen
    s.add(chamfer(plate([(-0.3, 0.06), (1.9, 0.06), (1.75, -0.3), (0.35, -0.34)], 0.1, "yz"), 0.02), m["dark"], at=(0, 0, -0.26))
    s.add(box(0.15, 0.78, 0.07), m["dark"], at=(0.105, -1.72, -0.245), rot=(-6, -5, 0), mirror=True)

    # ---------------------------------------------------------------- Bugkanonen unter den Flügelwurzeln
    # Gehäuse, Blendenbalg, Lauf, zweiteilige Mündungsbremse mit Stahlkern im Spalt, Mündung mit Glut
    cx, cz = 0.42, -0.2
    s.add(tube(16, 0.15, 0.15, -0.3, -1.6), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(bellows(16, 0.135, -1.58, -1.8), m["armor_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(16, 0.095, 0.09, -1.6, -3.0), m["steel"], at=(cx, 0, cz), mirror=True)
    s.add(tube(16, 0.13, 0.13, -2.82, -2.9), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(16, 0.13, 0.13, -2.96, -3.04), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(12, 0.08, 0.08, -3.04, -3.08), m["glow"], at=(cx, 0, cz), mirror=True)
    details(s, m, hull, canopy, gx, gz)
    fine(s, m, hull, gx, gz)
    return s.finish()
