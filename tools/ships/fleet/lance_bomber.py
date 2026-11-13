"""Lanzen-Bomber (Jäger, Torpedo-Staffel der Gegner), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Flügel, Flossen und Pylone, glatte runde Teile, Kleinteile und Feinteile.

Primärformen: schlanker, runder Rumpf mit kurzer Glaskanzel, gepfeilte Flügel mit Koralle-Spitzen, eine hohe
Seitenflosse und zwei Düsen. Kennzeichen ist die Lanze: ein langer Torpedo in Koralle, der an einem dunklen Pylon unter
dem Rumpf hängt und weit über den Bug hinausragt, mit Stahlspitze, leuchtendem Suchkopf, Lichtband und vier dunklen
Leitflossen. Auf dem Brett stehen fünf als Staffel zu je 0,42; der Umriss (Rumpf oben, Lanze darunter, vorn länger)
bleibt auch klein lesbar.
Detailzonen (alle an der Torpedolast und ihrer Aufhängung), dazwischen ruhig (Torpedokörper um das Lichtband, Rumpf,
Flügel, Flosse):
- Gefechtskopf: dunkler Zünderring am Übergang von der Stahlspitze zum Körper
- Aufhängung: zwei Halteschellen um den Torpedo am Fuß des Pylons, dazwischen ein Abwurfschloss
- Antrieb des Torpedos: Düsenring um die Brennkammer zwischen den Leitflossen
- Düsen des Bombers: Gehäusesegment und Düsenkranz (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie am Void Interceptor): Flügelspitzen-Schienen mit Positionslicht, Klappenspalt,
Wartungsluken, je eine kleine Rakete am Pylon unter den Flügeln, Kanzelbügel, Blattantenne, Staurohr am Bug,
Pendelstützen vom Pylon auf den Torpedo, Kühlrippen an den Düsen, Positionslicht an der Flossenspitze.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; an einem Jäger sparsam und nichts unter 0,03): Steuerflächen
(Klappe und Querruder durch einen Spalt getrennt, Gelenke über dem Klappenspalt, Seitenruder mit Spalt und Gelenken),
Rahmen um die Wartungsluken, Avionikklappen an den Rumpfflanken unter der Kanzel und Triebwerksklappen auf den
Schultern vor den Düsen, Stellstreben an den Düsenglocken, Pendelstützen an den Raketen; an der Lanze eine
Wartungsklappe in ihrer Farbe und die Nabelschnur vom Pylon zum Suchkopf. Dazu Beschriftung als flache Geometrie: ein
Warnband aus schrägen dunklen Balken um den Gefechtskopf und statt einer Kennnummer (er steht zu fünft) der
Staffelstreifen, ein breiter heller Streifen neben jeder Koralle-Spitze an Flügeln und Flosse.
"""

import math

import bmesh
from mathutils import Vector

import vf
from parts import beam, engine, engine_details, ring_at
from vf import Ship, box, chamfer, loft, pipe, plate, revolve, sec, segment, smooth, sphere, superellipse, tip, tube

TZ = -0.78  # Achse der Lanze
ROUND = 24  # Segmente der großen runden Teile (Rumpf, Lanze, Düsen); die Kanzel 16
FIN = [(0.7, 0.2), (2.15, 0.2), (2.45, 1.15), (1.95, 1.2)]  # Seitenflosse (y, z)


def around(ring, off):
    """Querschnitt um `off` von seiner Mitte weg nach außen versetzt."""
    c = sum(ring, Vector()) / len(ring)
    return [p + Vector((p.x - c.x, 0.0, p.z - c.z)).normalized() * off for p in ring]


def skin(rings, y0, y1, a, b, off, sunk=0.01):
    """Blech auf einem Loft (Ringe wie aus `smooth`): die Haut von y0 bis y1 zwischen den Ringpunkten a und b, um `off`
    erhaben und um `sunk` versenkt (Klappen, die der Rundung folgen)."""
    ys = [y0] + [r[0].y for r in rings if isinstance(r, list) and y0 < r[0].y < y1] + [y1]
    return loft([around(ring_at(rings, y), off)[a:b + 1] + around(ring_at(rings, y), -sunk)[a:b + 1][::-1] for y in ys])


def hinge(x):
    """y der Klappenachse bei x, 0,14 vor der Hinterkante des Flügels."""
    return 1.55 - (x - 0.3) * 0.1176 - 0.14


def details(s, m, canopy):
    """Kleinteile (Modulkommentar), vor dem Skalieren auf Jägerlänge; `canopy` sind die Ringe der Kanzel."""

    def wing(x, dz=0.0):
        """Höhe der Flügelmitte bei x (der Flügel steigt um 4° nach außen), um dz versetzt."""
        return 0.0698 * x + dz

    # ---------------------------------------------------------------- Flügel: Spitzenschiene mit Licht, Klappenspalt, Luke
    s.add(chamfer(box(0.07, 0.72, 0.07), 0.015), m["dark"], at=(2.07, 1.1, wing(2.07)), mirror=True)
    s.add(sphere(0.04, 1), m["glow"], at=(2.07, 0.73, wing(2.07)), mirror=True)
    gap = [(0.45, hinge(0.45) - 0.015), (1.42, hinge(1.42) - 0.015), (1.42, hinge(1.42) + 0.015), (0.45, hinge(0.45) + 0.015)]
    s.add(plate(gap, 0.13, "xy"), m["dark"], rot=(0, -4, 0), mirror=True)
    s.add(box(0.3, 0.34, 0.03), m["armor"], at=(0.85, 0.75, wing(0.85, 0.06)), rot=(0, -4, 0), mirror=True)

    # ---------------------------------------------------------------- Rakete am Pylon unter jedem Flügel
    x, z = 1.15, wing(1.15, -0.06)
    s.add(chamfer(plate([(0.58, 0.04), (1.02, 0.04), (0.98, -0.04), (0.7, -0.04)], 0.06, "yz"), 0.015), m["dark"], at=(x, 0, z - 0.03),
          mirror=True)
    s.add(tube(8, 0.045, 0.045, 0.3, 1.15), m["armor_round"], at=(x, 0, z - 0.105), mirror=True)
    s.add(tube(8, 0.0, 0.045, 0.12, 0.3), m["accent_round"], at=(x, 0, z - 0.105), mirror=True)
    s.add(box(0.16, 0.09, 0.012), m["dark"], at=(x, 1.09, z - 0.105), mirror=True)
    s.add(box(0.012, 0.09, 0.16), m["dark"], at=(x, 1.09, z - 0.105), mirror=True)

    # ---------------------------------------------------------------- Rumpf: Kanzelbügel, Blattantenne, Staurohr am Bug
    s.add(loft([ring_at(canopy, -1.13, 0.015), ring_at(canopy, -1.06, 0.015)]), m["hull_round"])
    s.add(box(0.025, 0.3, 0.2), m["dark"], at=(0, 0.2, 0.42))
    s.add(tube(6, 0.008, 0.014, -2.95, -2.5, 0.0, 0.02), m["steel"])

    # ---------------------------------------------------------------- Aufhängung: Pendelstützen vom Pylon auf den Torpedo
    for y in (-0.95, 0.45):
        s.add(beam((0.1, y, -0.4), (0.25, y, TZ + 0.17), 0.035, 0.035), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Kühlrippen an den Düsen, Licht an der Flossenspitze
    for t in (0.38, 0.44, 0.5):
        y = 1.9 + 0.9 * t
        s.add(tube(ROUND, 0.308, 0.308, y, y + 0.025, 0.3, 0.0), m["steel"], mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(0, 2.5, 1.22))


def fine(s, m, hull, lance):
    """Feinteile und Beschriftung (Modulkommentar), vor dem Skalieren auf Jägerlänge; `hull` und `lance` sind die Ringe
    von Rumpf und Torpedo (dieser um TZ abgesenkt)."""

    def through(outline, proud, mat):
        """Platte mit dem Umriss (x, y) durch den Flügel, oben und unten um `proud` erhaben."""
        s.add(plate(outline, 0.12 + 2 * proud, "xy"), mat, rot=(0, -4, 0), mirror=True)

    def rudder(z, dy=0.0):
        """Punkt (y, z) auf der Achse des Seitenruders in Höhe z, 0,22 vor der Hinterkante der Flosse, um dy versetzt."""
        return FIN[1][0] + (z - FIN[1][1]) * (FIN[2][0] - FIN[1][0]) / (FIN[2][1] - FIN[1][1]) - 0.22 + dy, z

    def lead(z, dy=0.0):
        return FIN[0][0] + (z - FIN[0][1]) * (FIN[3][0] - FIN[0][0]) / (FIN[3][1] - FIN[0][1]) + dy, z

    # ---------------------------------------------------------------- Flügel: Klappe und Querruder, Gelenke, Lukenrahmen
    # Der Klappenspalt (`details`) teilt die Hinterkante ab und endet außen an der Koralle-Spitze; zwei Spalte quer dazu
    # machen daraus Klappe (innen) und Querruder (außen), sie stoßen an den Spalt und enden vor der Fase der Hinterkante
    for x in (0.45, 0.95):
        a, b = x - 0.015, x + 0.015
        through([(a, hinge(a) + 0.015), (b, hinge(b) + 0.015), (b, hinge(b) + 0.11), (a, hinge(a) + 0.11)], 0.005, m["dark"])
    for x in (0.6, 0.8, 1.08, 1.24):
        s.add(box(0.05, 0.13, 0.14), m["steel_flat"], at=(x, hinge(x), 0.0698 * x), rot=(0, -4, 0), mirror=True)
    s.add(box(0.35, 0.39, 0.012), m["dark"], at=(0.85, 0.75, 0.0698 * 0.85 + 0.06), rot=(0, -4, 0), mirror=True)

    # ---------------------------------------------------------------- Staffelstreifen neben den Koralle-Spitzen
    # Durch Flügel und Flosse, also von beiden Seiten zu sehen: am Flügel innen neben der Spitze von der Vorderkante bis
    # vor den Klappenspalt, an der Flosse unter der Spitze von der Vorderkante bis vor das Seitenruder
    through([(1.31, 0.39), (1.38, 0.445), (1.38, hinge(1.38) - 0.05), (1.31, hinge(1.31) - 0.05)], 0.006, m["paint"])
    s.add(plate([lead(0.7, 0.07), rudder(0.7, -0.05), rudder(0.78, -0.05), lead(0.78, 0.07)], 0.132, "yz"), m["paint"])

    # ---------------------------------------------------------------- Seitenflosse: Ruder mit Spalt und zwei Gelenken
    s.add(plate([rudder(0.25, -0.015), rudder(0.25, 0.015), rudder(0.87, 0.015), rudder(0.87, -0.015)], 0.13, "yz"), m["dark"])
    for z in (0.42, 0.62):
        s.add(box(0.145, 0.11, 0.045), m["steel_flat"], at=(0, rudder(z)[0], z))

    # ---------------------------------------------------------------- Rumpf: Avionik- und Triebwerksklappen
    # Bleche, die der Rundung folgen: an den Flanken unter der Kanzel und auf den Schultern vor den Düsen
    s.add(skin(hull, -1.75, -1.25, 1, 3, 0.012), m["armor_round"], mirror=True)
    s.add(skin(hull, 1.05, 1.55, 2, 4, 0.012), m["armor_round"], mirror=True)

    # ---------------------------------------------------------------- Lanze: Warnband, Wartungsklappe, Nabelschnur
    # Schräge dunkle Balken um den Gefechtskopf hinter dem Zünderring (jeder zweite Abschnitt des Umfangs, über die
    # Breite des Bands um einen Abschnitt verdreht); die Klappe in der Farbe der Lanze, damit die Akzentfläche auf dem
    # Brett ruhig bleibt; die Nabelschnur führt vorn aus dem Pylon auf den Rücken des Torpedos
    y0, y1 = -2.98, -2.86
    a, b = around(ring_at(lance, y0), 0.008), around(ring_at(lance, y1), 0.008)
    ai, bi = around(ring_at(lance, y0), -0.01), around(ring_at(lance, y1), -0.01)
    for k in range(0, ROUND, 2):
        i, j = (k + 1) % ROUND, (k + 2) % ROUND
        s.add(loft([[a[k], a[i], ai[i], ai[k]], [b[i], b[j], bi[j], bi[i]]]), m["dark"], at=(0, 0, TZ))
    s.add(skin(lance, -2.3, -1.9, 2, 4, 0.012), m["accent_round"], at=(0, 0, TZ), mirror=True)
    s.add(pipe([(0.06, -1.22, -0.38), (0.1, -1.6, -0.36), (0.12, -1.85, TZ + 0.3)], 0.03, 6), m["dark_round"])

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    r, e0, e1 = 0.27, 1.9, 2.8
    for k in range(6):
        c, d = math.cos(2 * math.pi * (k + 0.5) / 6), math.sin(2 * math.pi * (k + 0.5) / 6)
        s.add(segment((0.3 + 1.1 * r * c, e0 + 0.6 * (e1 - e0), 1.1 * r * d), (0.3 + 0.93 * r * c, e1 - 0.03, 0.93 * r * d), 0.017, 6),
              m["steel"], mirror=True)

    # ---------------------------------------------------------------- Raketen: Pendelstützen am Pylon
    for y in (0.65, 0.95):
        s.add(box(0.12, 0.04, 0.04), m["dark"], at=(1.15, y, 0.0698 * 1.15 - 0.06 - 0.065), mirror=True)


def build():
    s = Ship("lance_bomber")
    m = vf.materials(s.name, "fighter")
    N = ROUND
    # Schlanker runder Rumpf
    hull = smooth([
        tip(-2.55, dz=0.02),
        sec(-2.2, superellipse(0.34, 0.3, N), dz=0.02),
        sec(-1.4, superellipse(0.78, 0.6, N)),
        sec(0.2, superellipse(0.96, 0.7, N)),
        sec(1.6, superellipse(0.86, 0.62, N)),
        sec(2.3, superellipse(0.7, 0.5, N)),
    ], 2)
    s.add(loft(hull), m["hull_round"])
    # Kurze Glaskanzel
    canopy = smooth([
        sec(-1.85, superellipse(0.12, 0.06, 16), dz=0.16),
        sec(-1.4, superellipse(0.42, 0.3, 16), dz=0.26),
        sec(-0.8, superellipse(0.46, 0.32, 16), dz=0.3),
        sec(-0.3, superellipse(0.3, 0.14, 16), dz=0.32),
    ], 2)
    s.add(loft(canopy), m["glass"])
    # Gepfeilte Flügel mit Koralle-Spitzen
    s.add(chamfer(plate([(0.3, -0.5), (2.0, 0.85), (2.0, 1.35), (0.3, 1.55)], 0.12, "xy"), 0.03), m["hull"], rot=(0, -4, 0), mirror=True)
    s.add(chamfer(plate([(1.45, 0.38), (2.05, 0.83), (2.05, 1.38), (1.45, 1.42)], 0.16, "xy"), 0.03), m["accent"], rot=(0, -4, 0), mirror=True)
    # Hohe Seitenflosse mit Koralle-Spitze
    s.add(chamfer(plate(FIN, 0.12, "yz"), 0.03), m["hull"])
    s.add(chamfer(plate([(1.6, 0.85), (2.36, 0.85), (2.5, 1.22), (1.9, 1.26)], 0.16, "yz"), 0.03), m["accent"])
    # Zwei Düsen am Heck mit Gehäusesegment und Düsenkranz
    engine(s, m, 0.27, 1.9, 2.8, dx=0.3, dz=0.0, n=N, mirror=True)
    engine_details(s, m, 0.27, 1.9, 2.8, dx=0.3, dz=0.0, n=N, mirror=True)

    # ---------------------------------------------------------------- Lanze
    # Dunkler Pylon, Torpedokörper in Koralle, Stahlspitze mit Suchkopf und Zünderring, Lichtband, Leitflossen
    s.add(chamfer(plate([(-1.5, -0.12), (0.6, -0.12), (0.4, TZ + 0.1), (-1.1, TZ + 0.1)], 0.22, "yz"), 0.04), m["dark"])
    lance = smooth([
        sec(-3.3, superellipse(0.5, 0.5, N)),
        sec(-2.7, superellipse(0.62, 0.62, N)),
        sec(0.5, superellipse(0.62, 0.62, N)),
        sec(1.0, superellipse(0.44, 0.44, N)),
    ], 1)
    s.add(loft(lance), m["accent_round"], at=(0, 0, TZ))
    s.add(revolve([(0.26, -3.2), (0.26, -3.4), (0.12, -4.15), (0.0, -4.3)], N), m["steel"], at=(0, 0, TZ))
    s.add(tube(12, 0.09, 0.0, -4.26, -4.4), m["glow"], at=(0, 0, TZ))
    s.add(tube(N, 0.29, 0.305, -3.33, -3.13), m["dark_round"], at=(0, 0, TZ))
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(0.315, -1.6, TZ))
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(-0.315, -1.6, TZ))
    for rot in (45, -45, 135, -135):
        s.add(chamfer(plate([(0.4, 0.0), (1.15, 0.0), (1.35, 0.55), (0.95, 0.55)], 0.1, "yz"), 0.02), m["dark"], at=(0, 0, TZ), rot=(0, rot, 0))
    s.add(tube(N, 0.24, 0.1, 0.95, 1.25), m["core"], at=(0, 0, TZ))
    s.add(tube(N, 0.25, 0.27, 0.9, 1.1), m["dark_round"], at=(0, 0, TZ))
    # Aufhängung: zwei Halteschellen am Fuß des Pylons, dazwischen das Abwurfschloss
    for y in (-0.6, 0.3):
        s.add(tube(N, 0.335, 0.335, y - 0.08, y + 0.08), m["dark_round"], at=(0, 0, TZ))
    s.add(chamfer(box(0.36, 0.5, 0.2), 0.03), m["armor"], at=(0, -0.15, TZ + 0.34))

    details(s, m, canopy)
    fine(s, m, hull, lance)
    bmesh.ops.scale(s.bm, vec=(0.92, 0.92, 0.92), verts=s.bm.verts)  # auf Jägerlänge
    return s.finish()
