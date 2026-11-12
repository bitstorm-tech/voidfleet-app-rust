"""Lanzen-Bomber (Jäger, Torpedo-Staffel der Gegner), detailreiche Fassung nach der Iron Dreadnought
(docs/stilregeln.md), mit Kleinteilen für den dreckigen Look.

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
"""

import bmesh

import vf
from parts import beam, engine, engine_details, ring_at
from vf import Ship, box, loft, plate, revolve, sec, smooth, sphere, superellipse, tip, tube

TZ = -0.78  # Achse der Lanze


def details(s, m, canopy):
    """Kleinteile (Modulkommentar), vor dem Skalieren auf Jägerlänge; `canopy` sind die Ringe der Kanzel."""

    def wing(x, dz=0.0):
        """Höhe der Flügelmitte bei x (der Flügel steigt um 4° nach außen), um dz versetzt."""
        return 0.0698 * x + dz

    def hinge(x):
        return 1.55 - (x - 0.3) * 0.1176 - 0.14  # 0,14 vor der Hinterkante

    # ---------------------------------------------------------------- Flügel: Spitzenschiene mit Licht, Klappenspalt, Luke
    s.add(box(0.07, 0.72, 0.07), m["dark"], at=(2.07, 1.1, wing(2.07)), mirror=True)
    s.add(sphere(0.04, 1), m["glow"], at=(2.07, 0.73, wing(2.07)), mirror=True)
    gap = [(0.45, hinge(0.45) - 0.015), (1.42, hinge(1.42) - 0.015), (1.42, hinge(1.42) + 0.015), (0.45, hinge(0.45) + 0.015)]
    s.add(plate(gap, 0.13, "xy"), m["dark"], rot=(0, -4, 0), mirror=True)
    s.add(box(0.3, 0.34, 0.03), m["armor"], at=(0.85, 0.75, wing(0.85, 0.06)), rot=(0, -4, 0), mirror=True)

    # ---------------------------------------------------------------- Rakete am Pylon unter jedem Flügel
    x, z = 1.15, wing(1.15, -0.06)
    s.add(box(0.05, 0.4, 0.07), m["dark"], at=(x, 0.8, z - 0.03), mirror=True)
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
        s.add(tube(10, 0.308, 0.308, y, y + 0.025, 0.3, 0.0), m["steel"], mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(0, 2.5, 1.22))


def build():
    s = Ship("lance_bomber")
    m = vf.materials(s.name, "fighter")
    N = 20
    # Schlanker runder Rumpf
    s.add(loft(smooth([
        tip(-2.55, dz=0.02),
        sec(-2.2, superellipse(0.34, 0.3, N), dz=0.02),
        sec(-1.4, superellipse(0.78, 0.6, N)),
        sec(0.2, superellipse(0.96, 0.7, N)),
        sec(1.6, superellipse(0.86, 0.62, N)),
        sec(2.3, superellipse(0.7, 0.5, N)),
    ], 2)), m["hull_round"])
    # Kurze Glaskanzel
    canopy = smooth([
        sec(-1.85, superellipse(0.12, 0.06, 16), dz=0.16),
        sec(-1.4, superellipse(0.42, 0.3, 16), dz=0.26),
        sec(-0.8, superellipse(0.46, 0.32, 16), dz=0.3),
        sec(-0.3, superellipse(0.3, 0.14, 16), dz=0.32),
    ], 2)
    s.add(loft(canopy), m["glass"])
    # Gepfeilte Flügel mit Koralle-Spitzen
    s.add(plate([(0.3, -0.5), (2.0, 0.85), (2.0, 1.35), (0.3, 1.55)], 0.12, "xy"), m["hull"], at=(0, 0, 0.0), rot=(0, -4, 0), mirror=True)
    s.add(plate([(1.45, 0.38), (2.05, 0.83), (2.05, 1.38), (1.45, 1.42)], 0.16, "xy"), m["accent"], at=(0, 0, 0.0), rot=(0, -4, 0), mirror=True)
    # Hohe Seitenflosse mit Koralle-Spitze
    s.add(plate([(0.7, 0.2), (2.15, 0.2), (2.45, 1.15), (1.95, 1.2)], 0.12, "yz"), m["hull"])
    s.add(plate([(1.6, 0.85), (2.36, 0.85), (2.5, 1.22), (1.9, 1.26)], 0.16, "yz"), m["accent"])
    # Zwei Düsen am Heck mit Gehäusesegment und Düsenkranz
    engine(s, m, 0.27, 1.9, 2.8, dx=0.3, dz=0.0, n=16, mirror=True)
    engine_details(s, m, 0.27, 1.9, 2.8, dx=0.3, dz=0.0, n=16, mirror=True)

    # ---------------------------------------------------------------- Lanze
    # Dunkler Pylon, Torpedokörper in Koralle, Stahlspitze mit Suchkopf und Zünderring, Lichtband, Leitflossen
    s.add(plate([(-1.5, -0.12), (0.6, -0.12), (0.4, TZ + 0.1), (-1.1, TZ + 0.1)], 0.22, "yz"), m["dark"])
    s.add(loft(smooth([
        sec(-3.3, superellipse(0.5, 0.5, N)),
        sec(-2.7, superellipse(0.62, 0.62, N)),
        sec(0.5, superellipse(0.62, 0.62, N)),
        sec(1.0, superellipse(0.44, 0.44, N)),
    ], 1)), m["accent_round"], at=(0, 0, TZ))
    s.add(revolve([(0.26, -3.2), (0.26, -3.4), (0.12, -4.15), (0.0, -4.3)], N), m["steel"], at=(0, 0, TZ))
    s.add(tube(12, 0.09, 0.0, -4.26, -4.4), m["glow"], at=(0, 0, TZ))
    s.add(tube(N, 0.29, 0.305, -3.33, -3.13), m["dark_round"], at=(0, 0, TZ))
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(0.315, -1.6, TZ))
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(-0.315, -1.6, TZ))
    for rot in (45, -45, 135, -135):
        s.add(plate([(0.4, 0.0), (1.15, 0.0), (1.35, 0.55), (0.95, 0.55)], 0.1, "yz"), m["dark"], at=(0, 0, TZ), rot=(0, rot, 0))
    s.add(tube(16, 0.24, 0.1, 0.95, 1.25), m["core"], at=(0, 0, TZ))
    s.add(tube(N, 0.25, 0.27, 0.9, 1.1), m["dark_round"], at=(0, 0, TZ))
    # Aufhängung: zwei Halteschellen am Fuß des Pylons, dazwischen das Abwurfschloss
    for y in (-0.6, 0.3):
        s.add(tube(N, 0.335, 0.335, y - 0.08, y + 0.08), m["dark_round"], at=(0, 0, TZ))
    s.add(box(0.36, 0.5, 0.2), m["armor"], at=(0, -0.15, TZ + 0.34))

    details(s, m, canopy)
    bmesh.ops.scale(s.bm, vec=(0.92, 0.92, 0.92), verts=s.bm.verts)  # auf Jägerlänge
    return s.finish()
