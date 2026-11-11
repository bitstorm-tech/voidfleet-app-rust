"""Iron Dreadnought (Schlachtschiff), Referenzschiff der Stilregeln (docs/stilregeln.md), mit Kleinteilen für den
dreckigen Look.

Primärformen: breiter, kantiger Keilrumpf mit Dorsaldeck und zwei Lagen Panzerschürzen, drei große Zwillingstürme in
Stahlblau (vorn, überhöht, seitlich), zwei schwere Belagerungskanonen am Bug mit magenta Mündung, ein Aufbau aus drei
Blöcken mit Fensterband, Kasematten mit Lichtband, Schildemitter am Deckrand, Kiel und vier Heckdüsen.
Detailzonen mit Zweck, dazwischen ruhige Flächen (Hauptdeck vor den Seitentürmen, Dorsaldeck zwischen den Türmen,
Schürzen, Flächen des Aufbaus):
- Bug: Bugpanzer-Kappe; Kanonen mit Blendenbalg, Kühlmantel, Laufschelle und zweiteiliger Mündungsbremse
- Türme: Barbettenring, Entfernungsmesser-Ohren, Turmluke, Blendenbälge, Mündungsringe (`parts.turret_details`)
- Flanke: zweite Plattenlage auf der Schürze, Kabelkanal mit Anschlusskasten, Kasematte mit Deckel, Luke und Lüfter,
  RCS-Blöcke an den Rumpfecken
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte und Querträger
- Achterdeck und Heck: Maschinenlüfter, Lagerkisten, Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
- Unterseite: Kinn mit Hangartor nach vorn, Spanten am V-Boden, Plattenlage unter der unteren Schürze
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`), seit der dreckige Look keine Konturringe mehr zeichnet: Fensterreihen aus Einzelfenstern,
Flak-Türmchen, Nadel- und Schüsselantennen, Geländer, Leiter, Rohrleitungen, Kühlrippen an den Düsen, Positionslichter.
"""

import palette
import vf
from parts import beam, bellows, engine, engine_details, rcs, turret, turret_details, vent
from vf import Ship, box, dish, grow, keel, loft, octagon, pipe, plate, post, rbox, rect, sec, segment, sphere, sweep, tube, wedge

# Rumpf: Querschnitte (y, Breite, Höhe) des Keilprofils, eingefügt um HULL_Z angehoben
HULL = ((-4.0, 0.6, 0.5), (-2.8, 1.8, 0.9), (-1.2, 3.6, 1.3), (0.6, 4.4, 1.4), (2.4, 3.8, 1.3), (3.4, 2.6, 1.0))
HULL_Z = 0.1
SKIRT = [(0.6, -3.0), (2.3, -1.0), (2.6, 1.0), (2.2, 2.9), (0.6, 2.9)]


def hull_at(y):
    """Rumpf bei y: halbe Breite, Höhe des Decks, Unterkante der Flanke, Kiel."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            w, h = w0 + t * (w1 - w0), h0 + t * (h1 - h0)
            return w / 2, HULL_Z + h / 2, HULL_Z + 0.1 * h, HULL_Z - h / 2
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def bottom_z(x, y):
    """Höhe des V-Bodens bei (x, y), x ≥ 0."""
    hw, _, low, keel_z = hull_at(y)
    return keel_z + (low - keel_z) * x / hw


def flak(s, m, at, yaw):
    """Zwillings-Flak, `at` auf Steuerbord und gespiegelt: Sockel, Gehäuse in der Akzentfarbe, Blende, zwei dünne Rohre
    nach -Y, um `yaw` gedreht (positiv: nach außen)."""
    x, y, z = at
    s.add(post(8, 0.1, 0.09, -0.03, 0.05), m["dark_round"], at=at, mirror=True)
    g = Ship("flak")
    g.add(box(0.17, 0.2, 0.11), m["accent"], at=(0, 0.02, 0.055))
    g.add(box(0.19, 0.05, 0.07), m["dark"], at=(0, -0.09, 0.05))
    for dx in (-0.04, 0.04):
        g.add(tube(6, 0.02, 0.016, -0.42, -0.08, dx, 0.06), m["steel"])
    s.place(g, at=(x, y, z + 0.05), rot=(0, 0, yaw), mirror=True)
    g.bm.free()


def rail(s, m, a, b, posts, mirror=True, h=0.09):
    """Geländer von a nach b (Punkte auf der Fläche): Handlauf in Höhe h auf `posts` Pfosten."""
    (ax, ay, az), (bx, by, bz) = a, b
    s.add(beam((ax, ay, az + h), (bx, by, bz + h), 0.014, 0.014), m["dark"], mirror=mirror)
    for k in range(posts):
        t = k / (posts - 1)
        s.add(box(0.014, 0.014, h + 0.01), m["dark"], at=(ax + t * (bx - ax), ay + t * (by - ay), az + t * (bz - az) + h / 2 - 0.005),
              mirror=mirror)


def details(s, m, z):
    """Kleinteile (Modulkommentar); `z` ist das Dach des Aufbaus."""
    roof1, roof2 = 1.765, 2.215  # Dächer der beiden unteren Blöcke des Aufbaus

    # ---------------------------------------------------------------- Fensterreihen: Aufbau und Wand des Dorsaldecks
    def side_windows(x, ys, wz):
        for y in ys:
            s.add(box(0.03, 0.07, 0.045), m["window"], at=(x + 0.004, y, wz), mirror=True)

    def rear_windows(xs, y, wz):
        for x in xs:
            s.add(box(0.07, 0.03, 0.045), m["window"], at=(x, y + 0.004, wz))

    row = [1.86 + 0.12 * k for k in range(8)]
    side_windows(0.85, row, 1.6)
    side_windows(0.85, row[1::2], 1.42)
    rear_windows([-0.6 + 0.2 * k for k in range(7)], 2.75, 1.6)
    side_windows(0.625, [1.96 + 0.12 * k for k in range(6)], 2.03)
    rear_windows([-0.4 + 0.2 * k for k in range(5)], 2.6, 2.03)
    for y in [-0.3 + 0.12 * k for k in range(6)] + [1.6 + 0.12 * k for k in range(8)]:
        s.add(box(0.03, 0.07, 0.045), m["window"], at=(0.9 + 0.2 * (y + 2.2) / 4.8 + 0.004, y, 0.99), mirror=True)

    # ---------------------------------------------------------------- Flak: Hauptdeck und Dach des Aufbaus
    for (x, y), yaw in (((1.6, -0.85), 30), ((1.85, -0.1), 40), ((1.85, 1.75), 140)):
        flak(s, m, (x, y, hull_at(y)[1]), yaw)
    flak(s, m, (0.74, 2.05, roof1), 90)

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.66, z + 1.2), m["dark_round"], at=(0.4, 2.15, 0), mirror=True)
    s.add(segment((0.55, 2.55, roof2 - 0.02), (0.62, 3.05, 2.95), 0.012, 6), m["dark_round"], mirror=True)
    s.add(segment((0.16, -3.9, 0.3), (0.16, -4.5, 0.34), 0.014, 6), m["steel"], mirror=True)
    # Schüssel backbord, Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.74, 2.5, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.74, 2.5, roof1 + 0.13), rot=(-40, 0, 150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(0.74, 2.5, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.5), m["dark_round"], at=(0.74, 2.56, 0))

    # ---------------------------------------------------------------- Geländer und Leiter am Aufbau
    rail(s, m, (0.83, 2.27, roof1), (0.83, 2.73, roof1), 3)
    rail(s, m, (-0.83, 2.73, roof1), (0.83, 2.73, roof1), 5, mirror=False)
    rail(s, m, (0.61, 2.1, roof2), (0.61, 2.58, roof2), 3)
    rail(s, m, (0.47, 1.79, z - 0.255), (0.99, 1.79, z - 0.255), 3)
    rail(s, m, (0.99, 1.79, z - 0.255), (0.99, 2.05, z - 0.255), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.5), m["dark"], at=(0.75 + dx, 2.757, 1.515))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.75, 2.757, 1.33 + 0.08 * k))

    # ---------------------------------------------------------------- Rohrleitungen und Kisten auf dem Dorsaldeck
    for x, (ex, ey) in ((0.62, (0.35, 0.05)), (0.7, (0.42, 0.0))):
        s.add(pipe([(x, 1.3, 1.29), (x, 0.5, 1.29), (ex, ey, 1.29)], 0.028, 6), m["steel"], mirror=True)
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(0.3, 1.12, 1.335))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(-0.34, 1.11, 1.345))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(-0.34, 1.1, 1.475))
    # Luken auf dem Hauptdeck vor den Seitentürmen
    s.add(box(0.3, 0.36, 0.04), m["armor"], at=(1.25, -0.45, hull_at(-0.45)[1] + 0.01), mirror=True)

    # ---------------------------------------------------------------- Kühlrippen an den Düsen
    for dz in (0.45, -0.25):
        for t in (0.37, 0.42, 0.47):
            y = 3.3 + 0.7 * t
            s.add(tube(12, 0.365, 0.365, y, y + 0.025, 0.6, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter
    s.add(sphere(0.03, 1), m["glow"], at=(1.0, 1.92, z - 0.225), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(2.52, 1.0, 0.43), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(0, -4.05, 0.41))


def build():
    s = Ship("iron_dreadnought")
    m = vf.materials(s.name, "battleship")
    m["shield"] = vf.light(f"{s.name}_shield", palette.SHIELD)

    # ---------------------------------------------------------------- Rumpf und Panzerlagen
    s.add(loft([sec(y, keel(w, h)) for y, w, h in HULL]), m["hull"], at=(0, 0, HULL_Z))
    # Bugpanzer: Kappe um die Bugspitze
    s.add(loft([sec(-4.06, keel(0.72, 0.6)), sec(-3.35, keel(1.36, 0.84))]), m["armor"], at=(0, 0, HULL_Z))
    # Panzerschürzen in zwei Lagen, beide mit einer zweiten Platte (Materialwechsel, damit die Stufe als Ton liest)
    s.add(plate(SKIRT, 0.24, "xy"), m["armor"], at=(0, 0, 0.3), mirror=True)
    s.add(plate(grow(SKIRT, -0.12), 0.07, "xy"), m["hull"], at=(0, 0, 0.445), mirror=True)
    lower = [(0.8, -2.2), (2.5, -0.4), (2.5, 1.6), (0.8, 2.4)]
    s.add(plate(lower, 0.22, "xy"), m["hull"], at=(0, 0, -0.2), mirror=True)
    s.add(plate(grow(lower, -0.14), 0.07, "xy"), m["armor"], at=(0, 0, -0.33), mirror=True)
    # Dorsaldeck und Deckplatte
    s.add(loft([sec(-3.0, octagon(0.6, 0.3)), sec(-2.2, octagon(1.8, 0.5)), sec(2.6, octagon(2.2, 0.55)), sec(3.2, octagon(1.4, 0.4))]),
          m["hull"], at=(0, 0, 0.95))
    s.add(plate([(-0.85, -2.1), (0.85, -2.1), (1.05, 2.4), (-1.05, 2.4)], 0.05, "xy"), m["hull"], at=(0, 0, 1.24))
    s.add(plate(octagon(0.42, 0.34, 0.4), 0.06, "xy"), m["armor"], at=(0, 0.78, 1.27))

    # ---------------------------------------------------------------- Türme: vorn, überhöht auf Barbette, seitlich
    turret(s, m, (0, -1.85, 1.265))
    turret_details(s, m, (0, -1.85, 1.265))
    s.add(post(24, 0.56, 0.54, 1.265, 1.6), m["hull_round"], at=(0, -0.3, 0))
    turret(s, m, (0, -0.3, 1.6))
    turret_details(s, m, (0, -0.3, 1.6))
    turret(s, m, (1.45, 0.95, 0.86), r=0.46, barrel=1.2, mirror=True)
    turret_details(s, m, (1.45, 0.95, 0.86), r=0.46, barrel=1.2, mirror=True)

    # ---------------------------------------------------------------- Aufbau: drei Blöcke, Brückenflügel, Mast
    z = 1.265
    for w, l, h, y in ((1.7, 1.5, 0.5, 2.0), (1.25, 1.1, 0.45, 2.05), (0.92, 0.75, 0.4, 2.0)):
        s.add(wedge(w, l, h), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, 2.0 - 0.75 / 2 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.465, 2.05, z - 0.17), mirror=True)
    # Brückenflügel: offene Nocken auf Höhe des Brückendecks, unter den seitlichen Fenstern
    s.add(box(0.65, 0.28, 0.16), m["hull"], at=(0.675, 1.92, z - 0.335), mirror=True)
    # Mast mit Querträger, Radarplatte und Licht
    s.add(post(10, 0.09, 0.06, z, z + 0.95), m["hull_round"], at=(0, 2.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, 2.15, z + 0.66))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, 2.06, z + 0.36), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, 2.15, z + 0.99))

    # ---------------------------------------------------------------- Kasematten mit Lichtband, Deckel, Luke, Lüfter
    s.add(box(0.7, 1.6, 0.5), m["armor"], at=(2.1, 0.6, 0.35), mirror=True)
    s.add(box(0.78, 1.68, 0.06), m["hull"], at=(2.1, 0.6, 0.62), mirror=True)
    s.add(box(0.04, 1.2, 0.08), m["glow"], at=(2.46, 0.6, 0.42), mirror=True)
    s.add(box(0.36, 0.04, 0.28), m["hull"], at=(2.1, -0.21, 0.34), mirror=True)
    vent(s, m, m["hull"], (2.2, 0.95, 0.65), 0.36, 0.62, slats=2, mirror=True)

    # ---------------------------------------------------------------- Kabelkanäle an den Flanken
    for ys, box_y in (((-1.95, -1.2, -0.1), -0.9), ((1.3, 2.4, 2.7), 2.7)):
        pts = []
        for y in ys:
            hw, deck, _, _ = hull_at(y)
            pts.append((hw + 0.03, y, deck - 0.18))
        s.add(sweep(pts, rect(0.1, 0.1)), m["dark"], mirror=True)
        hw, deck, _, _ = hull_at(box_y)
        s.add(box(0.18, 0.3, 0.22), m["armor"], at=(hw + 0.04, box_y, deck - 0.18), mirror=True)

    # ---------------------------------------------------------------- Belagerungskanonen am Bug
    cx, cz = 0.52, 0.36
    s.add(rbox(0.62, 1.0, 0.62, 0.06, 1), m["dark"], at=(cx, -2.35, cz), mirror=True)
    s.add(bellows(18, 0.34, -2.83, -3.05), m["armor_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.24, 0.21, -4.6, -2.7), m["steel"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.3, 0.3, -3.55, -3.03), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.27, 0.27, -4.02, -3.9), m["armor_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.2, 0.2, -4.8, -4.55), m["steel"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.31, 0.31, -4.9, -4.76), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.31, 0.31, -4.66, -4.5), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(18, 0.15, 0.15, -4.94, -4.88), m["shield"], at=(cx, 0, cz), mirror=True)

    # ---------------------------------------------------------------- Schildemitter (magenta)
    s.add(box(0.08, 1.6, 0.1), m["shield"], at=(1.0, -1.3, 1.02), mirror=True)
    for y in (-2.25, 0.05):
        s.add(post(16, 0.17, 0.15, 0.0, 0.12), m["dark_round"], at=(0.98, y, 1.06), mirror=True)
        s.add(sphere(0.13, 2), m["shield"], at=(0.98, y, 1.2), mirror=True)
    s.add(box(0.5, 0.06, 0.18), m["shield"], at=(0, -3.72, 0.5))

    # ---------------------------------------------------------------- Achterdeck: Maschinenlüfter, Lagerkisten
    vent(s, m, m["hull"], (0, 2.98, 1.17), 0.9, 0.34, slats=2, rot=(-7, 0, 0))
    vent(s, m, m["hull"], (1.45, 2.2, hull_at(2.2)[1]), 0.5, 0.72, mirror=True)
    s.add(box(0.26, 0.32, 0.2), m["armor"], at=(1.05, 2.78, hull_at(2.78)[1] + 0.08), mirror=True)

    # ---------------------------------------------------------------- RCS an den Rumpfecken
    hw, deck, _, _ = hull_at(-1.35)
    rcs(s, m, (hw - 0.06, -1.35, deck + 0.03), -1)
    hw, deck, _, _ = hull_at(3.1)
    rcs(s, m, (hw - 0.04, 3.1, deck + 0.03), 1)

    # ---------------------------------------------------------------- Unterseite: Kiel, Spanten, Hangartor
    s.add(plate([(-2.0, 0.0), (2.8, 0.0), (2.9, -0.4), (-1.0, -0.35)], 0.12, "yz"), m["dark"], at=(0, 0, -0.5))
    for y in (0.0, 0.55, 1.1, 1.65):
        x0, x1 = 0.04, 0.78
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        rib = [(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)]
        s.add(plate(rib, 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    s.add(wedge(1.0, 1.4, 0.42), m["hull"], at=(0, -2.05, -0.4), rot=(180, 0, 0))
    s.add(box(0.34, 0.06, 0.26), m["dark"], at=(0.2, -2.75, -0.42), mirror=True)

    # ---------------------------------------------------------------- Vier Heckdüsen mit Gehäusesegment und Düsenkranz
    for dz in (0.45, -0.25):
        engine(s, m, 0.32, 3.3, 4.0, dx=0.6, dz=dz, mirror=True)
        engine_details(s, m, 0.32, 3.3, 4.0, dx=0.6, dz=dz, mirror=True)
    details(s, m, z)
    return s.finish()
