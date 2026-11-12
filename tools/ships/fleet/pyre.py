"""Pyre (Flaggschiff von Admiralin Reyes, „die Glut“), detailreiche Fassung nach docs/stilregeln.md, mit Kleinteilen
für den dreckigen Look.

Schweres, breites Schlachtschiff, das heiß wirkt. Ein Keilrumpf mit hohem Rückendeck und goldener Zierlinie, davor der
Plasmawerfer als Bug: ein Gehäuse in Glutorange mit Fensterband, darauf ein dunkler Kühlmantel und ein Emitterkegel,
vor dem drei dunkle Klauen eine leuchtende Glutkugel halten. An beiden Flanken je ein langer, dunkler
Kühlkörper mit vier schrägen Glutschlitzen, auf dem Rückendeck zwei Reihen großer Kühlblöcke mit Glutkante.
Dahinter die Wappenplatte mit Reyes' Flamme, achtern die Brücke aus drei Keilen mit Fensterband und Mast, Panzerschürzen
in Glutorange, Kiel und drei schwere Düsen.
Detailzonen mit Zweck (Brennstoff, Plasma, Hitze), dazwischen ruhige Flächen (Rückendeck vor den Kühlblöcken und
zwischen ihnen, Flanken über den Kühlkörpern, Gehäuse des Werfers, Flächen der Brücke):
- Bug: Hitzeschild als große Scheibe vor dem Kühlmantel, aus der die Klauen ragen; Gelenke an den Klauenwurzeln,
  hitzefeste Klauenspitzen (`armor`) um die Glutkugel
- Brennstoff: zwei Satteltanks neben der Brücke, halb ins Rückendeck gesenkt, mit Spannbändern und Ventil; von jedem
  eine Zuleitung auf der Schulter nach vorn in den Plasmawerfer, mit Rohrschelle und Anschlussflansch
- Flanke: Kühlkörper mit Glutschlitzen, RCS-Blöcke an den Rumpfecken
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte und Querträger
- Heck: Maschinenlüfter hinter der Brücke, Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
- Unterseite: zweite Plattenlage unter den Schürzen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an der Brücke und an der Wand des
Rückendecks, Geländer und Leiter an der Brücke, Flak auf den Schultern und dem Brückendach, Peitschenantennen, Schüssel
und Gerätekasten, Rohre von den Tanks zur Brücke, Tankdeckel, Luke und Kisten vor der Brücke, Kühlrippen auf den
Kühlkörpern, am Kühlmantel des Werfers und an den Düsen, Positionslichter.
"""

from mathutils import Vector

import vf
from parts import axle, badge, engine, engine_details, flak, rail, rcs, sym, trim_line, vent, windows
from vf import Ship, box, dish, grow, keel, loft, ngon, octagon, pipe, plate, post, revolve, sec, segment, sphere, sweep, tube, wedge

# Rumpf: Querschnitte (y, Breite, Höhe) des Keilprofils um z = 0
HULL = ((-3.4, 2.0, 1.1), (-2.4, 3.4, 1.5), (2.2, 3.6, 1.6), (3.9, 2.8, 1.3), (4.5, 2.0, 1.0))
DECK = 1.15  # Bezugshöhe des Rückendecks; seine Oberkante liegt bei TOP
TOP = DECK + 0.1
PZ = 0.25  # Achse des Plasmawerfers
SKIRT = [(0.7, -2.3), (1.95, -2.0), (2.0, 3.0), (0.7, 3.6)]
CLAW = [(-4.2, 0.3), (-4.2, 0.82), (-5.2, 0.8), (-5.85, 0.46), (-5.7, 0.3)]
CLAW_TIP = [(-5.15, 0.28), (-5.15, 0.825), (-5.22, 0.822), (-5.88, 0.47), (-5.72, 0.28)]
TANK = (1.32, 1.9, 1.1)  # Mitte des Steuerbord-Tanks; Radius 0,3, von y 1,2 bis 2,6
# Wappen: Platte mit Spitze zum Bug, darauf eine Flamme, die zum Bug lodert
PLATE = sym([(0.0, -0.95), (0.62, -0.45), (0.62, 0.55), (0.0, 0.8)])
FLAME = sym([(0.0, -0.74), (0.13, -0.3), (0.3, -0.48), (0.38, -0.04), (0.34, 0.28), (0.18, 0.48), (0.0, 0.54)])


def hull_at(y):
    """Rumpf bei y: halbe Breite und Höhe der Oberkante (die Schulter neben dem Rückendeck)."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            return (w0 + t * (w1 - w0)) / 2, (h0 + t * (h1 - h0)) / 2
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def claw(m):
    """Eine Klaue des Werfers als Baugruppe um die Werferachse (oben liegend): dunkle Platte, Gelenk an der Wurzel vor dem
    Hitzeschild, hitzefeste Spitze."""
    g = Ship("claw")
    g.add(plate(CLAW, 0.3, "yz"), m["dark"])
    g.add(axle(0.13, 0.42), m["dark_round"], at=(0, -4.6, 0.6))
    g.add(plate(CLAW_TIP, 0.34, "yz"), m["armor"])
    return g


def details(s, m, z, by):
    """Kleinteile (Modulkommentar); `z` ist das Dach der Brücke, `by` ihre Mitte."""
    roof1, roof2, wing = TOP + 0.55, TOP + 1.0, z - 0.255  # Dächer der beiden unteren Keile, Oberseite der Brückenflügel
    tx, ty, tz = TANK

    # ---------------------------------------------------------------- Fensterreihen: Brücke und Wand des Rückendecks
    row = [by - 0.14 + 0.12 * k for k in range(8)]
    windows(s, m, [(0.9, y, roof1 - 0.215) for y in row])
    windows(s, m, [(0.9, y, roof1 - 0.395) for y in row[1::2]])
    windows(s, m, [(-0.6 + 0.2 * k, by + 0.75, roof1 - 0.215) for k in range(7)], out="y")
    windows(s, m, [(0.675, by - 0.09 + 0.12 * k, roof2 - 0.185) for k in range(6)])
    windows(s, m, [(-0.4 + 0.2 * k, by + 0.55, roof2 - 0.185) for k in range(5)], out="y")
    # Wand des Rückendecks über der Zuleitung: vorn und neben der Brücke
    windows(s, m, [(1.1 + 0.05 * (y + 2.3) / 5.9, y, 1.03) for y in [-2.0 + 0.12 * k for k in range(5)] + [2.9 + 0.12 * k for k in range(6)]])

    # ---------------------------------------------------------------- Flak: Schultern vorn und achtern, Dach des unteren Keils
    for (x, y), yaw in (((1.6, -1.6), 30), ((1.36, 3.15), 140)):
        flak(s, m, (x, y, hull_at(y)[1]), yaw)
    flak(s, m, (0.79, by + 0.22, roof1), 90)

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.58, z + 1.1), m["dark_round"], at=(0.4, by + 0.15, 0), mirror=True)
    s.add(segment((0.58, by + 0.5, roof2 - 0.02), (0.66, by + 1.0, roof2 + 0.7), 0.012, 6), m["dark_round"], mirror=True)
    # Schüssel backbord (zeigt nach achtern außen), Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.79, by + 0.55, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.79, by + 0.55, roof1 + 0.13), rot=(-40, 0, -150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(0.79, by + 0.58, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.5), m["dark_round"], at=(0.79, by + 0.64, 0))

    # ---------------------------------------------------------------- Geländer und Leiter an der Brücke
    rail(s, m, (0.88, by + 0.4, roof1), (0.88, by + 0.73, roof1), 3)
    rail(s, m, (-0.88, by + 0.73, roof1), (0.88, by + 0.73, roof1), 5, mirror=False)
    rail(s, m, (0.655, by + 0.1, roof2), (0.655, by + 0.53, roof2), 3)
    rail(s, m, (0.45, by - 0.15, wing), (0.99, by - 0.15, wing), 3)
    rail(s, m, (0.99, by - 0.15, wing), (0.99, by + 0.09, wing), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.5), m["dark"], at=(-0.75 + dx, by + 0.757, TOP + 0.25))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(-0.75, by + 0.757, TOP + 0.065 + 0.08 * k))

    # ---------------------------------------------------------------- Brennstoff: Rohre von den Tanks in die Brücke, Tankdeckel
    for x, y in ((0.62, 1.47), (0.7, 1.56)):  # zwischen Tankboden und vorderem Spannband
        s.add(pipe([(1.1, y, TOP + 0.028), (x, y, TOP + 0.028), (x, by - 0.63, TOP + 0.028)], 0.028, 6), m["steel"], mirror=True)
    s.add(post(8, 0.07, 0.07, tz + 0.27, tz + 0.34), m["dark_round"], at=(tx, ty, 0), mirror=True)
    # Luke und Lagerkisten vor der Brücke
    s.add(box(0.36, 0.3, 0.04), m["armor"], at=(0, 1.5, TOP + 0.01))
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(-0.3, by - 0.86, TOP + 0.07))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(0.32, by - 0.87, TOP + 0.08))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(0.32, by - 0.87, TOP + 0.21))

    # ---------------------------------------------------------------- Kühlrippen: Kühlkörper, Kühlmantel des Werfers, Düsen
    for y0 in (-0.62, 0.32, 1.28):  # je drei zwischen den Glutschlitzen
        for dy in (-0.1, 0.0, 0.1):
            s.add(box(0.26, 0.03, 0.07), m["armor"], at=(1.92, y0 + dy, 0.48), mirror=True)
    for y in (-4.24, -4.12, -4.0):  # ohne Glanz: die Ringflächen schauen in `hero` zur Kamera
        r = 0.7 - 0.08 * (y + 4.4) / 0.65 + 0.035
        s.add(tube(20, r, r, y, y + 0.04, dz=PZ), m["armor_round"])
    for r, ys, dx, dz, mirror in ((0.52, (4.742, 4.78, 4.818), 0.0, 0.25, False), (0.4, (4.675, 4.705, 4.735), 0.95, 0.15, True)):
        for y in ys:
            s.add(tube(12, r * 1.14, r * 1.14, y, y + 0.02, dx, dz), m["steel"], mirror=mirror)

    # ---------------------------------------------------------------- Positionslichter: Brückenflügel, Kühlkörper achtern, Heck
    s.add(sphere(0.03, 1), m["glow"], at=(1.0, by - 0.03, z - 0.225), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(2.04, 2.28, 0.46), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(0, 4.64, 0.81))


def build():
    s = Ship("pyre")
    m = vf.materials(s.name, "battleship")

    # ---------------------------------------------------------------- Rumpf, Rückendeck, Zierlinie
    s.add(loft([sec(y, keel(w, h)) for y, w, h in HULL]), m["hull"])
    trim_line(s, m, [(w / 2 + 0.01, y, h / 2 + 0.005) for y, w, h in HULL])
    s.add(loft([sec(-3.0, octagon(1.4, 0.5), dz=DECK - 0.35), sec(-2.3, octagon(2.2, 0.7), dz=DECK - 0.25),
                sec(3.6, octagon(2.3, 0.7), dz=DECK - 0.25), sec(4.2, octagon(1.6, 0.5), dz=DECK - 0.3)]), m["hull"])

    # ---------------------------------------------------------------- Plasmawerfer als Bug
    # Gehäuse in Glutorange mit Fensterband, Kühlmantel, Emitterkegel
    s.add(loft([sec(-2.6, octagon(2.0, 1.5, 0.35), dz=PZ), sec(-3.4, octagon(2.0, 1.5, 0.35), dz=PZ),
                sec(-3.9, octagon(1.6, 1.25, 0.35), dz=PZ)]), m["accent"])
    s.add(box(0.04, 0.7, 0.09), m["window"], at=(1.005, -3.0, PZ + 0.3), mirror=True)
    s.add(tube(24, 0.7, 0.62, -4.4, -3.75, dz=PZ), m["dark_round"])
    s.add(tube(20, 0.32, 0.14, -4.4, -4.95, dz=PZ), m["steel"])
    # Hitzeschild: große Scheibe vor dem Kühlmantel, ohne Glanz (eine glänzende Stirnfläche würde in `hero` weiß)
    s.add(revolve([(0.0, -4.52), (0.88, -4.52), (0.95, -4.46), (0.95, -4.4), (0.8, -4.33), (0.0, -4.33)], 32), m["armor_round"], at=(0, 0, PZ))
    # Drei Klauen halten die Glutkugel (von der Seite zwei Klauen mit der Kugel dazwischen, von oben die Kugel um die
    # obere Klaue)
    for angle in (0, 120, 240):
        g = claw(m)
        s.place(g, at=(0, 0, PZ), rot=(0, angle, 0))
        g.bm.free()
    s.add(sphere(0.4, 2), m["glow"], at=(0, -5.3, PZ))

    # ---------------------------------------------------------------- Kühlkörper an den Flanken mit Glutschlitzen (Lichter ohne Glühen)
    s.add(plate([(1.55, -1.9), (2.05, -1.5), (2.05, 2.3), (1.55, 2.7)], 0.6, "xy"), m["dark"], at=(0, 0, 0.15), mirror=True)
    for y in (-1.1, -0.15, 0.8, 1.75):
        s.add(box(0.06, 0.18, 0.5), m["signal"], at=(2.06, y, 0.15), rot=(25, 0, 0), mirror=True)

    # ---------------------------------------------------------------- Rückendeck: zwei Reihen Kühlblöcke mit Glutkante, Wappen
    for y in (-1.4, -0.35, 0.7):
        s.add(wedge(0.42, 0.75, 0.55, front=0.5), m["dark"], at=(1.0, y, TOP + 0.2), mirror=True)
        s.add(box(0.3, 0.5, 0.06), m["signal"], at=(1.0, y + 0.1, TOP + 0.49), mirror=True)
    badge(s, m, (0, -0.35, TOP - 0.05), PLATE, FLAME, dy=-0.04)

    # ---------------------------------------------------------------- Brennstoff: Satteltanks und Zuleitungen zum Werfer
    tx, ty, tz = TANK
    s.add(revolve([(0.0, -0.7), (0.17, -0.67), (0.26, -0.6), (0.3, -0.47), (0.3, 0.47), (0.26, 0.6), (0.17, 0.67), (0.0, 0.7)], 24),
          m["armor_round"], at=TANK, mirror=True)
    for dy in (-0.25, 0.25):
        s.add(tube(24, 0.33, 0.33, ty + dy - 0.05, ty + dy + 0.05, dx=tx, dz=tz), m["dark_round"], mirror=True)
    # Die Zuleitung liegt auf der Schulter zwischen Rückendeck und Zierlinie und tritt seitlich in das Werfergehäuse
    line = [(1.36, 1.35, 0.87), (1.36, -1.95, 0.835), (1.18, -2.6, 0.79), (0.9, -3.0, 0.74)]
    s.add(sweep(line, ngon(12, 0.1)), m["dark_round"], mirror=True)
    # Ventil vor dem Tank, Rohrschelle am Rückendeck, Anschlussflansch am Werfer
    s.add(tube(16, 0.15, 0.15, 1.1, 1.26, dx=1.36, dz=0.87), m["armor_round"], mirror=True)
    s.add(box(0.38, 0.12, 0.24), m["armor"], at=(1.31, -0.4, 0.88), mirror=True)
    a, b = Vector(line[2]), Vector(line[3])
    s.add(segment(a.lerp(b, 0.55), a.lerp(b, 0.75), 0.14, 16), m["armor_round"], mirror=True)

    # ---------------------------------------------------------------- Panzerschürzen in Glutorange, darunter eine zweite Lage
    s.add(plate(SKIRT, 0.2, "xy"), m["accent"], at=(0, 0, -0.35), mirror=True)
    s.add(plate(grow(SKIRT, -0.14), 0.06, "xy"), m["armor"], at=(0, 0, -0.47), mirror=True)

    # ---------------------------------------------------------------- RCS an den Rumpfecken
    for y, along in ((-2.25, -1), (3.65, 1)):
        hw, top = hull_at(y)
        rcs(s, m, (hw - 0.06, y, top + 0.03), along)

    # ---------------------------------------------------------------- Brücke achtern: drei Keile, Brückenflügel, Mast
    z, by = TOP, 2.75
    for w, l, h in ((1.8, 1.5, 0.55), (1.35, 1.1, 0.45), (0.95, 0.8, 0.4)):
        s.add(wedge(w, l, h), m["hull"], at=(0, by, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, by - 0.4 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.49, by + 0.05, z - 0.17), mirror=True)
    s.add(box(0.62, 0.28, 0.16), m["hull"], at=(0.7, by - 0.03, z - 0.335), mirror=True)
    s.add(post(10, 0.09, 0.06, z, z + 0.85), m["hull_round"], at=(0, by + 0.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, by + 0.15, z + 0.58))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, by + 0.06, z + 0.32), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, by + 0.15, z + 0.89))

    # ---------------------------------------------------------------- Achterdeck: Maschinenlüfter auf der Schräge hinter der Brücke
    # (das Rückendeck fällt ab y 3,6 um 14°, bei y 3,88 liegt es 0,07 unter TOP)
    vent(s, m, m["hull"], (0, 3.88, TOP - 0.07), 0.9, 0.34, slats=2, rot=(-14, 0, 0))

    # ---------------------------------------------------------------- Kiel und drei schwere Düsen
    s.add(plate([(-2.6, -0.4), (3.6, -0.5), (3.3, -1.0), (-1.6, -0.9)], 0.24, "yz"), m["dark"])
    s.add(box(2.6, 0.6, 1.2), m["dark"], at=(0, 4.35, 0.2))
    for r, y1, dx, dz, mirror in ((0.52, 5.35, 0.0, 0.25, False), (0.4, 5.15, 0.95, 0.15, True)):
        engine(s, m, r, 4.4, y1, dx=dx, dz=dz, mirror=mirror)
        engine_details(s, m, r, 4.4, y1, dx=dx, dz=dz, mirror=mirror)
    details(s, m, z, by)
    return s.finish()
